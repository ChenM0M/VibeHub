"""Regression cases for AppImage catalog failures; no third-party dependencies."""
import importlib.util
import hashlib
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location("verify_appimage", Path(__file__).with_name("verify-appimage.py"))
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


prepare_spec = importlib.util.spec_from_file_location("prepare_appimage", Path(__file__).with_name("prepare-appimage.py"))
preparer = importlib.util.module_from_spec(prepare_spec)
prepare_spec.loader.exec_module(preparer)


class LauncherPreparationTests(unittest.TestCase):
    def test_cached_restricted_launcher_is_repaired_without_changing_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "AppRun-x86_64"
            data = b"trusted launcher fixture"
            path.write_bytes(data)
            path.chmod(0o770)
            with mock.patch.object(preparer, "SHA256", hashlib.sha256(data).hexdigest()):
                preparer.prepare(path)
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
            self.assertEqual(path.read_bytes(), data)

    def test_clean_cache_gets_verified_executable_launcher(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "tauri/AppRun-x86_64"
            data = b"trusted launcher fixture"
            with mock.patch.object(preparer, "SHA256", hashlib.sha256(data).hexdigest()):
                preparer.prepare(path, download=lambda: data)
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)

    def test_unexpected_download_is_rejected_before_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "AppRun-x86_64"
            with self.assertRaisesRegex(ValueError, "digest"):
                preparer.prepare(path, download=lambda: b"unexpected")
            self.assertFalse(path.exists())


class AppImageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.root.chmod(0o755)
        (self.root / "AppRun").write_text("#!/bin/sh\nexit 0\n")
        (self.root / "AppRun").chmod(0o755)
        (self.root / "VibeHub.png").write_bytes(validator.PNG_SIGNATURE)
        (self.root / ".DirIcon").symlink_to("VibeHub.png")
        (self.root / "usr/share/applications").mkdir(parents=True)
        (self.root / "usr/share/applications/VibeHub.desktop").write_text(
            "[Desktop Entry]\nType=Application\nName=VibeHub\nExec=vibehub\nIcon=VibeHub\nCategories=Development;\n")
        (self.root / "VibeHub.desktop").symlink_to("usr/share/applications/VibeHub.desktop")

    def test_relative_links_pass(self):
        self.assertEqual(validator.validate_appdir(self.root)["dir_icon_target"], "VibeHub.png")

    def test_absolute_icon_rejected_even_if_build_target_exists(self):
        (self.root / ".DirIcon").unlink()
        (self.root / ".DirIcon").symlink_to(self.root / "VibeHub.png")
        with self.assertRaisesRegex(ValueError, "Absolute symlink"):
            validator.validate_appdir(self.root)

    def test_dangling_icon_rejected(self):
        (self.root / "VibeHub.png").unlink()
        with self.assertRaisesRegex(ValueError, "dangling"):
            validator.validate_appdir(self.root)

    def test_absolute_desktop_rejected(self):
        (self.root / "VibeHub.desktop").unlink()
        (self.root / "VibeHub.desktop").symlink_to(self.root / "usr/share/applications/VibeHub.desktop")
        with self.assertRaisesRegex(ValueError, "Absolute symlink"):
            validator.validate_appdir(self.root)

    def test_link_escape_rejected(self):
        (self.root / ".DirIcon").unlink()
        (self.root / ".DirIcon").symlink_to("../outside.png")
        with self.assertRaisesRegex(ValueError, "escapes AppDir"):
            validator.validate_appdir(self.root)

    def test_non_executable_entry_rejected(self):
        (self.root / "AppRun").chmod(0o644)
        with self.assertRaisesRegex(ValueError, "not executable"):
            validator.validate_appdir(self.root)

    def test_owner_only_wrapped_launcher_rejected(self):
        wrapped = self.root / "AppRun.wrapped"
        wrapped.write_text("#!/bin/sh\nexit 0\n")
        wrapped.chmod(0o770)
        with self.assertRaisesRegex(ValueError, "all users: AppRun.wrapped"):
            validator.validate_appdir(self.root)

    def test_unreadable_resource_rejected(self):
        (self.root / "VibeHub.png").chmod(0o640)
        with self.assertRaisesRegex(ValueError, "all users: VibeHub.png"):
            validator.validate_appdir(self.root)

    def test_untraversable_directory_rejected(self):
        (self.root / "usr").chmod(0o750)
        with self.assertRaisesRegex(ValueError, "all users: usr"):
            validator.validate_appdir(self.root)

    def test_multiple_desktops_rejected(self):
        (self.root / "other.desktop").write_text("")
        with self.assertRaisesRegex(ValueError, "exactly one"):
            validator.validate_appdir(self.root)

    def test_missing_categories_rejected(self):
        desktop = self.root / "usr/share/applications/VibeHub.desktop"
        desktop.write_text(desktop.read_text().replace("Categories=Development;\n", ""))
        with self.assertRaisesRegex(ValueError, "Categories"):
            validator.validate_appdir(self.root)


if __name__ == "__main__":
    unittest.main()
