#!/usr/bin/env python3
"""Prepare Tauri's official x86_64 launcher with portable permissions.

Tauri CLI 2.11.4 writes newly downloaded tools as 0770, then copies AppRun
into the final bundle. Pre-populate the exact official launcher as 0755 so
linuxdeploy's AppRun.wrapped remains executable for non-owner users.
"""
import hashlib
import os
from pathlib import Path
import platform
import sys
import urllib.request

URL = "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64"
SHA256 = "f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f"


def prepare(path, download=None):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        data = path.read_bytes()
    elif download is not None:
        data = download()
    else:
        with urllib.request.urlopen(URL, timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise ValueError("Unexpected official AppRun digest; refusing to package an unverified launcher")
    if not path.exists():
        path.write_bytes(data)
    path.chmod(0o755)
    return {"launcher": str(path), "sha256": SHA256, "mode": "0755"}


if __name__ == "__main__":
    if sys.platform != "linux" or platform.machine() != "x86_64":
        sys.exit("This launcher is pinned for the Linux x86_64 release target")
    import json
    cache = Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "tauri"
    print(json.dumps(prepare(cache / "AppRun-x86_64")))
