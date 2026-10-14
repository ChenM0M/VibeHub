#!/usr/bin/env python3
"""Check final, trusted VibeHub AppImages outside their build directory.

Requires Linux, sudo, squashfs-tools, desktop-file-utils, Xvfb, xdotool, ImageMagick and Tesseract.
The --appdir mode is also usable on macOS for structural regression tests.
"""
import argparse
import configparser
import hashlib
import json
import os
import re
from pathlib import Path
import signal
import shutil
import subprocess
import sys
import tempfile
import time


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def require_file(root, path):
    """Reject dangling, absolute and escaping links, including link chains."""
    relative = path.relative_to(root)
    current = root
    pending = list(relative.parts)
    followed = 0
    while pending:
        current = current / pending.pop(0)
        if current.is_symlink():
            followed += 1
            if followed > 40:
                raise ValueError(f"Symlink loop: {relative}")
            target = Path(os.readlink(current))
            if target.is_absolute():
                raise ValueError(f"Absolute symlink: {relative} -> {target}")
            destination = Path(os.path.abspath(current.parent / target))
            if not destination.is_relative_to(root):
                raise ValueError(f"Symlink escapes AppDir: {relative} -> {target}")
            pending = list(destination.relative_to(root).parts) + pending
            current = root
    if not current.is_file():
        raise ValueError(f"Missing file or dangling symlink: {relative}")
    return current


def validate_stored_permissions(listing):
    """Check permissions actually stored in SquashFS, before extraction changes them."""
    count = 0
    for line in listing.splitlines():
        parts = line.split(None, 5)
        if len(parts) != 6 or not re.fullmatch(r"[d\-l][rwxstST-]{9}", parts[0]):
            continue
        mode, name = parts[0], parts[5]
        count += 1
        if mode.startswith("l"):
            continue
        readable = all(mode[i] == "r" for i in (1, 4, 7))
        executable = [mode[i] in "xst" for i in (3, 6, 9)]
        needs_execute = mode.startswith("d") or any(executable)
        if not readable or (needs_execute and not all(executable)):
            raise ValueError(f"Stored AppImage permissions restrict other users: {name} ({mode})")
    if count == 0:
        raise ValueError("No stored SquashFS permission entries were parsed")
    return count


def validate_appdir(directory):
    root = Path(directory).resolve()
    entry = require_file(root, root / "AppRun")
    if entry.stat().st_mode & 0o111 != 0o111:
        raise ValueError("AppRun is not executable by all users")
    # Ownership changes on mounted AppImages (e.g. firejail uses root:root).
    # os.access() as the builder would silently accept a 0770 launcher.
    for path in [root, *root.rglob("*")]:
        if path.is_symlink():
            continue
        mode = path.stat().st_mode
        required = 0o555 if path.is_dir() or mode & 0o111 else 0o444
        if mode & required != required:
            raise ValueError(f"AppDir path is not readable/executable by all users: {path.relative_to(root)} ({mode & 0o777:o})")
    icon = require_file(root, root / ".DirIcon")
    if icon.read_bytes()[:8] != PNG_SIGNATURE:
        raise ValueError(".DirIcon must resolve to a PNG")
    desktops = list(root.glob("*.desktop"))
    if len(desktops) != 1:
        raise ValueError("AppDir must contain exactly one root .desktop file")
    desktop = require_file(root, desktops[0])
    config = configparser.ConfigParser(interpolation=None, strict=True)
    config.optionxform = str
    config.read(desktop, encoding="utf-8")
    section = config["Desktop Entry"]
    for key in ("Name", "Exec", "Icon", "Categories"):
        if not section.get(key, "").strip():
            raise ValueError(f"Missing or empty desktop key: {key}")
    if section.get("Type") != "Application":
        raise ValueError("Desktop Type must be Application")
    icon_name = section["Icon"]
    if Path(icon_name).name != icon_name or icon_name in (".", ".."):
        raise ValueError("Desktop Icon must be a root icon name")
    matches = [root / f"{icon_name}{ext}" for ext in (".png", ".svg", ".xpm")]
    matches = [p for p in matches if p.exists() or p.is_symlink()]
    if not matches:
        raise ValueError(f"Missing root icon for Icon={icon_name}")
    for match in matches:
        require_file(root, match)
    return {"desktop": desktops[0].name, "icon": icon_name,
            "dir_icon_target": os.readlink(root / ".DirIcon") if (root / ".DirIcon").is_symlink() else "PNG file"}


def smoke(image, output):
    """Run only in a fresh Xvfb display and a disposable application home."""
    with tempfile.TemporaryDirectory(prefix="vibehub-smoke-") as home:
        env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=f"{home}/config",
                   XDG_DATA_HOME=f"{home}/data", XDG_CACHE_HOME=f"{home}/cache",
                   XDG_STATE_HOME=f"{home}/state", XDG_RUNTIME_DIR=f"{home}/runtime",
                   LANG="en_US.UTF-8", LC_ALL="en_US.UTF-8", LANGUAGE="en_US:en",
                   APPIMAGE_EXTRACT_AND_RUN="1")
        if image.name == "AppRun":
            env["APPDIR"] = str(image.parent)
        Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700)
        (output / "startup-identity.json").write_text(json.dumps({
            "uid": os.getuid(), "launcher_uid": image.stat().st_uid,
            "launcher_mode": oct(image.stat().st_mode & 0o777),
        }, indent=2) + "\n")
        with (output / "startup.log").open("w") as log:
            process = subprocess.Popen([str(image)], cwd=home, env=env,
                                       stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                deadline = time.monotonic() + 40
                window = None
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise ValueError(f"AppImage exited before opening a window: {process.returncode}")
                    found = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", "^VibeHub$"],
                                           capture_output=True, text=True, timeout=5)
                    if found.returncode == 0 and found.stdout.strip():
                        window = found.stdout.splitlines()[0]
                        break
                    time.sleep(0.5)
                if window is None:
                    raise ValueError("No visible VibeHub window within 40 seconds")
                # WebKit may map a white window before React has rendered.
                # Wait for actual English project UI, not just a window title.
                deadline = time.monotonic() + 40
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise ValueError("AppImage exited after opening its window")
                    subprocess.run(["import", "-window", window, str(output / "startup.png")],
                                   check=True, timeout=10)
                    ocr = subprocess.run(["tesseract", str(output / "startup.png"), "stdout", "-l", "eng"],
                                         check=True, capture_output=True, text=True, timeout=10)
                    (output / "startup.ocr.txt").write_text(ocr.stdout)
                    rendered = ocr.stdout.lower()
                    if "projects" in rendered and "search projects" in rendered:
                        break
                    time.sleep(2)
                else:
                    raise ValueError("Visible window did not render the English project UI within 40 seconds")
                print("PASS: rendered English project UI in clean locale")
            finally:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)


def smoke_as_non_owner(appdir, output):
    """Exercise final extracted bytes as nobody, while every app file is root-owned."""
    import pwd
    account = pwd.getpwnam("nobody")
    temporary = appdir.parent
    temporary.chmod(0o755)
    harness = temporary / "verify-appimage.py"
    shutil.copyfile(__file__, harness)
    evidence = temporary / "non-owner-evidence"
    evidence.mkdir()
    original_owner = f"{os.getuid()}:{os.getgid()}"
    subprocess.run(["sudo", "-n", "chown", "-hR", "0:0", str(appdir)], check=True)
    try:
        subprocess.run(["sudo", "-n", "chown", f"{account.pw_uid}:{account.pw_gid}", str(evidence)], check=True)
        subprocess.run(["sudo", "-n", "-u", "nobody", "--", "xvfb-run", "--auto-servernum",
                        sys.executable, str(harness), "--smoke-child", str(appdir / "AppRun"),
                        "--output", str(evidence)], check=True, timeout=120)
    finally:
        subprocess.run(["sudo", "-n", "chown", "-hR", original_owner, str(appdir), str(evidence)], check=True)
        shutil.copytree(evidence, output, dirs_exist_ok=True)
    identity = json.loads((output / "startup-identity.json").read_text())
    if identity["uid"] != account.pw_uid or identity["launcher_uid"] != 0:
        raise ValueError("Non-owner smoke did not run as nobody against a root-owned launcher")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("images", nargs="*")
    parser.add_argument("--appdir", type=Path)
    parser.add_argument("--search-root", action="append", default=[])
    parser.add_argument("--output", type=Path, default=Path("appimage-evidence"))
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--smoke-child", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.appdir:
        print(json.dumps(validate_appdir(args.appdir), indent=2))
        return
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if args.smoke_child:
        smoke(args.smoke_child.resolve(), output)
        return
    images = [Path(p).resolve() for p in args.images]
    for search in args.search_root:
        images.extend(p.resolve() for p in Path(search).glob("**/bundle/appimage/*.AppImage"))
    images = sorted(set(images))
    if not images:
        raise ValueError("No AppImage produced; a .deb alone does not satisfy this gate")
    for image in images:
        destination = output / image.name
        destination.mkdir(parents=True, exist_ok=True)
        digest = hashlib.sha256()
        with image.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
        report = {"artifact": image.name, "sha256": digest.hexdigest()}
        # Extract away from target/: absolute build-path symlinks must never pass.
        with tempfile.TemporaryDirectory(prefix="vibehub-appimage-") as temporary:
            offset = subprocess.run([str(image), "--appimage-offset"], check=True,
                                    capture_output=True, text=True, timeout=10).stdout.strip()
            if not offset.isdigit():
                raise ValueError("AppImage runtime did not return a numeric SquashFS offset")
            listing = subprocess.run(["unsquashfs", "-lln", "-o", offset, str(image)],
                                     check=True, capture_output=True, text=True, timeout=60).stdout
            (destination / "stored-permissions.log").write_text(listing)
            report["stored_permission_entries"] = validate_stored_permissions(listing)
            appdir = Path(temporary) / "squashfs-root"
            # The runtime's --appimage-extract creates directories as 0700.
            # unsquashfs preserves the stored modes needed for this acceptance test.
            with (destination / "extract.log").open("w") as log:
                subprocess.run(["unsquashfs", "-no-progress", "-no-xattrs", "-d", str(appdir),
                                "-o", offset, str(image)], stdout=log, stderr=subprocess.STDOUT,
                               check=True, timeout=120)
            report.update(validate_appdir(appdir))
            subprocess.run(["desktop-file-validate", str(appdir / report["desktop"])], check=True, timeout=10)
            if args.smoke:
                smoke_as_non_owner(appdir, destination / "non-owner")
                report["non_owner_startup"] = "passed (nobody running root-owned AppDir; rendered English UI)"
        report["structure"] = "passed"
        if args.smoke:
            subprocess.run(["xvfb-run", "--auto-servernum", sys.executable, str(Path(__file__).resolve()),
                            "--smoke-child", str(image), "--output", str(destination)], check=True, timeout=120)
            report["startup"] = "passed (rendered English project UI; not full UI acceptance)"
        (destination / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, configparser.Error, subprocess.SubprocessError) as error:
        sys.exit(f"AppImage verification failed: {error}")
