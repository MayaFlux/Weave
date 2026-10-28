import importlib.util
import json
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[2]


def load_community_mode():
    gi = types.ModuleType("gi")
    gi.require_version = lambda *args: None
    repository = types.ModuleType("gi.repository")
    repository.Gtk = types.SimpleNamespace(ApplicationWindow=object)
    repository.Gio = types.SimpleNamespace()
    repository.GLib = types.SimpleNamespace(timeout_add=Mock())
    path = ROOT / "linux" / "lib" / "modes" / "community.py"
    spec = importlib.util.spec_from_file_location("linux_community_mode_test", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"gi": gi, "gi.repository": repository}):
        with patch.object(sys, "path", [str(ROOT / "linux"), *sys.path]):
            spec.loader.exec_module(module)
    return module


community_mode = load_community_mode()


def community_view():
    return types.SimpleNamespace(
        template_dir=ROOT.parent / "templates",
        script_dir=ROOT / "linux" / "scripts",
        create_btn=Mock(),
        _show_error=Mock(),
        _on_done=Mock(),
    )


class CommunityCreationTests(unittest.TestCase):
    def test_gui_creates_module_with_requested_manifest_and_cmake(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = community_view()
            dialog = Mock()
            with patch.object(community_mode.Gtk, "AlertDialog", return_value=dialog, create=True):
                community_mode.CommunityModuleMode._create_async(
                    view, "terrain_field", temporary, "Terrain", "0.8.1", True
                )
            module = Path(temporary) / "terrain_field"
            manifest = json.loads((module / "community.json").read_text())
            cmake = (module / "terrain_field.cmake").read_text()
            self.assertEqual(manifest["description"], "Terrain")
            self.assertEqual(manifest["min_version"], "0.8.1")
            self.assertTrue(manifest["needs_lila"])
            self.assertIn('set(MF_MIN_VERSION "0.8.1")', cmake)
            self.assertIn("set(MF_NEEDS_LILA ON)", cmake)
            self.assertTrue((module / "CMakePresets.json").is_file())
            self.assertTrue((module / ".git").is_dir())
            dialog.choose.assert_called_once()
            view._show_error.assert_not_called()

    def test_missing_template_keeps_gui_usable_without_creating_module(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            view = community_view()
            view.template_dir = root / "missing-templates"
            community_mode.CommunityModuleMode._create_async(
                view, "terrain_field", temporary, "Terrain", "0.8.1", False
            )
            self.assertFalse((root / "terrain_field").exists())
            view._show_error.assert_called_once()
            view.create_btn.set_sensitive.assert_called_with(True)
            view.create_btn.set_label.assert_called_with("Create Module")


if __name__ == "__main__":
    unittest.main()
