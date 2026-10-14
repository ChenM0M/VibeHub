"""Regression cases for AppImage catalog failures; no third-party dependencies."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("verify_appimage", Path(__file__).with_name("verify-appimage.py"))
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


class AppImageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
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
