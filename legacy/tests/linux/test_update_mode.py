import importlib.util
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[2]


def load_update_mode():
    gi = types.ModuleType("gi")
    gi.require_version = lambda *args: None
    repository = types.ModuleType("gi.repository")
    repository.Gtk = types.SimpleNamespace(ApplicationWindow=object)
    repository.Gio = types.SimpleNamespace()
    repository.GLib = types.SimpleNamespace(idle_add=Mock())
    path = ROOT / "linux" / "lib" / "modes" / "update.py"
    spec = importlib.util.spec_from_file_location("lib.modes.update", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"gi": gi, "gi.repository": repository}):
        with patch.object(sys, "path", [str(ROOT / "linux"), *sys.path]):
            spec.loader.exec_module(module)
    return module


update_mode = load_update_mode()


class UpdateModeTests(unittest.TestCase):
    def test_network_failure_keeps_inputs_and_allows_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "CMakeLists.txt").write_text("project(Test)\n")
            view = types.SimpleNamespace(
                proj_entry=Mock(get_text=Mock(return_value=str(project))),
                name_entry=Mock(get_text=Mock(return_value="terrain_field")),
                add_btn=Mock(),
                log_buffer=Mock(),
                _append_log=Mock(),
                _on_done=Mock(),
                _show_error=Mock(),
            )
            view._on_error = lambda message: update_mode.UpdateProjectMode._on_error(view, message)
            update_mode.GLib.idle_add.side_effect = lambda callback, *args: callback(*args)
            thread = lambda target, daemon: types.SimpleNamespace(start=target)
            with patch.object(update_mode.threading, "Thread", side_effect=thread):
                with patch.object(update_mode, "update", side_effect=[update_mode.UpdateError("network unavailable"), ["terrain_field"]]) as update:
                    update_mode.UpdateProjectMode._on_add(view, None)
                    view.add_btn.set_sensitive.assert_called_with(True)
                    view.add_btn.set_label.assert_called_with("Add Module")
                    view._append_log.assert_called_with("Error: network unavailable")
                    self.assertEqual(view.proj_entry.get_text(), str(project))
                    self.assertEqual(view.name_entry.get_text(), "terrain_field")
                    update_mode.UpdateProjectMode._on_add(view, None)
                    self.assertEqual(update.call_count, 2)
                    view._on_done.assert_called_once()


if __name__ == "__main__":
    unittest.main()
