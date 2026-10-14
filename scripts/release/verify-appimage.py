#!/usr/bin/env python3
"""Check final, trusted VibeHub AppImages outside their build directory.

Requires Linux, desktop-file-utils, xvfb, xauth, xdotool and ImageMagick.
The --appdir mode is also usable on macOS for structural regression tests.
"""
import argparse
import configparser
import hashlib
import json
import os
from pathlib import Path
import signal
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


def validate_appdir(directory):
    root = Path(directory).resolve()
    entry = require_file(root, root / "AppRun")
    if not os.access(entry, os.X_OK):
        raise ValueError("AppRun is not executable")
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
        Path(env["XDG_RUNTIME_DIR"]).mkdir(mode=0o700)
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
                time.sleep(3)
                if process.poll() is not None:
                    raise ValueError("AppImage exited after opening its window")
                subprocess.run(["import", "-window", window, str(output / "startup.png")], check=True, timeout=10)
                print("PASS: visible VibeHub window in clean English locale")
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
            with (destination / "extract.log").open("w") as log:
                subprocess.run([str(image), "--appimage-extract"], cwd=temporary,
                               stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
            appdir = Path(temporary) / "squashfs-root"
            report.update(validate_appdir(appdir))
            subprocess.run(["desktop-file-validate", str(appdir / report["desktop"])], check=True, timeout=10)
        report["structure"] = "passed"
        if args.smoke:
            subprocess.run(["xvfb-run", "--auto-servernum", sys.executable, str(Path(__file__).resolve()),
                            "--smoke-child", str(image), "--output", str(destination)], check=True, timeout=70)
            report["startup"] = "passed (visible window; not full UI acceptance)"
        (destination / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, configparser.Error, subprocess.SubprocessError) as error:
        sys.exit(f"AppImage verification failed: {error}")
