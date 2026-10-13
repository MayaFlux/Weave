import importlib.util
import os
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import Mock, patch


def load_project_mode():
    gi = types.ModuleType("gi")
    gi.require_version = lambda *args: None
    repository = types.ModuleType("gi.repository")
    repository.Gtk = types.SimpleNamespace(ApplicationWindow=object)
    repository.Gio = types.SimpleNamespace()
    repository.GLib = types.SimpleNamespace(timeout_add=Mock())
    path = Path(__file__).resolve().parents[2] / "linux/lib/modes/project.py"
    spec = importlib.util.spec_from_file_location("linux_project_mode_test", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"gi": gi, "gi.repository": repository}):
        spec.loader.exec_module(module)
    return module


project_mode = load_project_mode()


def project_view(name, location):
    button = Mock()
    button.get_label.return_value = "Create Project"
    lila = Mock()
    lila.get_active.return_value = False
    shaders = Mock()
    shaders.get_active.return_value = False
    return types.SimpleNamespace(
        name_entry=Mock(get_text=Mock(return_value=name)),
        loc_entry=Mock(get_text=Mock(return_value=location)),
        create_btn=button,
        lila_check=lila,
        shader_sources_check=shaders,
        script_dir=Path("/weave/scripts"),
        template_dir=Path("/weave/templates"),
        _show_error=Mock(),
        _create_project_async=Mock(),
        _on_creation_complete=Mock(),
    )


class ProjectCreationTests(unittest.TestCase):
    def setUp(self):
        project_mode.GLib.timeout_add.reset_mock(side_effect=True)

    def test_empty_name_is_rejected_before_scheduling_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("  ", temporary)
            project_mode.ProjectCreationMode._on_create(view, None)

        view._show_error.assert_called_once_with("Project name cannot be empty")
        project_mode.GLib.timeout_add.assert_not_called()
        view.create_btn.set_sensitive.assert_not_called()

    def test_missing_location_is_rejected(self):
        view = project_view("MyProject", "/missing/weave/project/location")
        project_mode.ProjectCreationMode._on_create(view, None)

        view._show_error.assert_called_once()
        self.assertIn("Directory does not exist", view._show_error.call_args.args[0])
        project_mode.GLib.timeout_add.assert_not_called()

    def test_existing_project_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as temporary:
            (Path(temporary) / "MyProject").mkdir()
            view = project_view("MyProject", temporary)
            project_mode.ProjectCreationMode._on_create(view, None)

        self.assertIn("Project already exists", view._show_error.call_args.args[0])
        project_mode.GLib.timeout_add.assert_not_called()

    def test_valid_request_schedules_creation_with_captured_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view(" MyProject ", temporary)
            project_mode.GLib.timeout_add.side_effect = lambda delay, callback: callback()
            project_mode.ProjectCreationMode._on_create(view, None)

        view.create_btn.set_sensitive.assert_called_once_with(False)
        view._create_project_async.assert_called_once_with(
            "MyProject", temporary, "Create Project"
        )
        view._show_error.assert_not_called()

    def test_generator_receives_argument_vector_and_template_environment(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("MyProject", temporary)
            view.lila_check.get_active.return_value = True
            failed = subprocess.CompletedProcess([], 1, stdout="", stderr="Template missing")

            with patch.object(project_mode.subprocess, "run", return_value=failed) as run:
                project_mode.ProjectCreationMode._create_project_async(
                    view, "MyProject", temporary, "Create Project"
                )

        args, kwargs = run.call_args
        self.assertEqual(
            args[0],
            ["/weave/scripts/create_project.sh", "new", "MyProject", temporary, "--with-lila"],
        )
        self.assertEqual(kwargs["env"]["WEAVE_TEMPLATE_DIR"], "/weave/templates")
        self.assertEqual(kwargs["timeout"], 30)
        self.assertNotIn("shell", kwargs)
        view._show_error.assert_called_once_with("Failed to create project:\nTemplate missing")
        view.create_btn.set_sensitive.assert_called_once_with(True)

    def test_success_reports_created_project_location(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("MyProject", temporary)
            project = Path(temporary) / "MyProject"
            project.mkdir()
            (project / "validate.sh").write_text("#!/bin/bash\n")
            completed = subprocess.CompletedProcess([], 0, stdout="Created", stderr="")
            dialog = Mock()

            with patch.object(project_mode.subprocess, "run", return_value=completed) as run:
                with patch.object(project_mode.Gtk, "AlertDialog", return_value=dialog, create=True) as alert:
                    project_mode.ProjectCreationMode._create_project_async(
                        view, "MyProject", temporary, "Create Project"
                    )

        self.assertEqual(
            run.call_args.args[0],
            ["/weave/scripts/create_project.sh", "new", "MyProject", temporary],
        )
        self.assertIn(str(Path(temporary) / "MyProject"), alert.call_args.kwargs["detail"])
        self.assertNotIn("cmake", alert.call_args.kwargs["detail"])
        self.assertNotIn("validate.sh", alert.call_args.kwargs["detail"])
        dialog.choose.assert_called_once()
        view._show_error.assert_not_called()

    def test_shader_copy_choice_reaches_generator_as_an_argument(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("MyProject", temporary)
            view.shader_sources_check.get_active.return_value = True
            failed = subprocess.CompletedProcess([], 1, stdout="", stderr="Sources missing")
            with patch.object(project_mode.subprocess, "run", return_value=failed) as run:
                project_mode.ProjectCreationMode._create_project_async(
                    view, "MyProject", temporary, "Create Project"
                )

        self.assertEqual(
            run.call_args.args[0],
            ["/weave/scripts/create_project.sh", "new", "MyProject", temporary, "--copy-shader-sources"],
        )

    def test_gui_creates_project_with_validator_and_requested_shader_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prefix = root / "MayaFlux"
            shader = prefix / "share/MayaFlux/src/shaders/effects/blur.comp"
            shader.parent.mkdir(parents=True)
            shader.write_text("shader source\n")
            view = project_view("MyProject", temporary)
            view.script_dir = Path(__file__).resolve().parents[2] / "linux/scripts"
            view.template_dir = Path(__file__).resolve().parents[2] / "templates"
            view.shader_sources_check.get_active.return_value = True
            dialog = Mock()

            with patch.dict(os.environ, {"MAYAFLUX_ROOT": str(prefix)}):
                with patch.object(project_mode.Gtk, "AlertDialog", return_value=dialog, create=True):
                    project_mode.ProjectCreationMode._create_project_async(
                        view, "MyProject", temporary, "Create Project"
                    )

            project = root / "MyProject"
            self.assertTrue((project / "validate.sh").is_file())
            self.assertEqual(
                (project / "data/shaders/effects/blur.comp").read_text(),
                "shader source\n",
            )
            self.assertTrue((project / "CMakePresets.json").is_file())
            dialog.choose.assert_called_once()
            view._show_error.assert_not_called()

    def test_missing_deployed_validator_prevents_success_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("MyProject", temporary)
            completed = subprocess.CompletedProcess([], 0, stdout="Created", stderr="")
            with patch.object(project_mode.subprocess, "run", return_value=completed):
                with patch.object(project_mode.Gtk, "AlertDialog", create=True) as alert:
                    project_mode.ProjectCreationMode._create_project_async(
                        view, "MyProject", temporary, "Create Project"
                    )

        alert.assert_not_called()
        self.assertIn("validate.sh was not deployed", view._show_error.call_args.args[0])

    def test_generator_timeout_restores_create_button(self):
        with tempfile.TemporaryDirectory() as temporary:
            view = project_view("MyProject", temporary)
            view.lila_check.get_active.return_value = False

            with patch.object(
                project_mode.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired("create_project.sh", 30),
            ):
                project_mode.ProjectCreationMode._create_project_async(
                    view, "MyProject", temporary, "Create Project"
                )

        view._show_error.assert_called_once_with("Project creation timed out")
        view.create_btn.set_sensitive.assert_called_once_with(True)
        view.create_btn.set_label.assert_called_once_with("Create Project")


if __name__ == "__main__":
    unittest.main()
