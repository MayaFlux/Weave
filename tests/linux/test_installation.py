import asyncio
import importlib.util
import os
import stat
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import AsyncMock, Mock, call, patch


def load_installation_mode():
    gi = types.ModuleType("gi")
    gi.require_version = lambda *args: None
    repository = types.ModuleType("gi.repository")
    repository.Gtk = types.SimpleNamespace(ApplicationWindow=object)
    repository.GLib = types.SimpleNamespace(idle_add=Mock())
    path = Path(__file__).resolve().parents[2] / "linux/lib/modes/installation.py"
    spec = importlib.util.spec_from_file_location("linux_installation_mode_test", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, {"gi": gi, "gi.repository": repository}):
        spec.loader.exec_module(module)
    return module


installation_mode = load_installation_mode()


class InstallationTests(unittest.TestCase):
    def setUp(self):
        installation_mode.GLib.idle_add.reset_mock(side_effect=True)
        installation_mode.GLib.idle_add.side_effect = lambda callback, *args: callback(*args)
        self.step = installation_mode.DependenciesStep()
        self.step._password = "temporary-password"
        self.step._pw_entry = Mock()
        self.step._install_btn = Mock()
        self.step.status = Mock()
        self.step._log = Mock()

    def test_distro_detection_checks_supported_package_managers(self):
        for available, expected in (
            ({"pacman", "dnf", "apt-get"}, "arch"),
            ({"dnf", "apt-get"}, "fedora"),
            ({"apt-get"}, "ubuntu"),
            (set(), "unknown"),
        ):
            with self.subTest(available=available):
                with patch.object(
                    installation_mode.shutil,
                    "which",
                    side_effect=lambda name: f"/bin/{name}" if name in available else None,
                ):
                    self.assertEqual(self.step._detect_distro(), expected)

    def test_conflicting_package_depends_on_distro_and_release(self):
        expected = {
            ("arch", "stable"): "mayaflux-dev-bin",
            ("arch", "dev"): "mayaflux",
            ("fedora", "stable"): "mayaflux-dev",
            ("fedora", "dev"): "mayaflux",
            ("ubuntu", "stable"): "mayaflux-edge",
            ("ubuntu", "dev"): "mayaflux",
        }
        for (distro, release), conflict in expected.items():
            with self.subTest(distro=distro, release=release):
                self.step.release_type = release
                self.assertEqual(self.step._conflicting_pkg(distro), conflict)

    def test_arch_uses_available_aur_helper(self):
        for available, helper in (({"yay", "paru"}, "yay"), ({"paru"}, "paru")):
            with self.subTest(available=available):
                with patch.object(
                    installation_mode.shutil,
                    "which",
                    side_effect=lambda name: f"/bin/{name}" if name in available else None,
                ):
                    self.assertEqual(
                        self.step._build_commands("arch"),
                        [[helper, "-S", "--noconfirm", "mayaflux"]],
                    )

    def test_arch_without_helper_prepares_clone_and_build(self):
        self.step.release_type = "dev"
        with patch.object(installation_mode.shutil, "which", return_value=None):
            with patch.object(installation_mode.tempfile, "mkdtemp", return_value="/tmp/aur-build"):
                self.assertEqual(
                    self.step._build_commands("arch"),
                    [
                        [
                            "git",
                            "clone",
                            "https://aur.archlinux.org/mayaflux-dev-bin.git",
                            "/tmp/aur-build",
                        ],
                        ["bash", "-c", "cd /tmp/aur-build && makepkg -si --noconfirm"],
                    ],
                )

    def test_ubuntu_and_fedora_commands_select_release_package(self):
        for distro, release, expected in (
            ("ubuntu", "stable", ["sudo", "apt-get", "install", "-y", "mayaflux"]),
            ("ubuntu", "dev", ["sudo", "apt-get", "install", "-y", "mayaflux-edge"]),
            ("fedora", "stable", ["sudo", "dnf", "install", "-y", "mayaflux"]),
            ("fedora", "dev", ["sudo", "dnf", "install", "-y", "mayaflux-dev"]),
        ):
            with self.subTest(distro=distro, release=release):
                self.step.release_type = release
                self.assertEqual(self.step._build_commands(distro)[-1], expected)

    def test_package_presence_queries_are_read_only(self):
        for distro, output, installed in (
            ("arch", "", True),
            ("fedora", "", True),
            ("ubuntu", "install ok installed", True),
            ("ubuntu", "deinstall ok config-files", False),
        ):
            with self.subTest(distro=distro, output=output):
                result = types.SimpleNamespace(returncode=0, stdout=output)
                with patch.object(installation_mode.subprocess, "run", return_value=result) as run:
                    self.assertEqual(self.step._is_pkg_installed(distro, "mayaflux"), installed)
                    self.assertIn(run.call_args.args[0][0], ("pacman", "rpm", "dpkg-query"))

    def test_unsupported_distro_stops_before_process_launch(self):
        with patch.object(self.step, "_detect_distro", return_value="unknown"):
            with patch.object(self.step, "_run_pty", new_callable=AsyncMock) as runner:
                self.assertFalse(asyncio.run(self.step._install()))
                runner.assert_not_awaited()

        self.assertEqual(self.step._password, "")
        self.step.status.set_text.assert_called_with("ERROR: Unsupported distribution.")

    def test_install_runs_commands_in_order_and_clears_password(self):
        with patch.object(self.step, "_detect_distro", return_value="ubuntu"):
            with patch.object(self.step, "_is_pkg_installed", return_value=False):
                with patch.object(self.step, "_run_pty", new_callable=AsyncMock, return_value=True) as runner:
                    self.assertTrue(asyncio.run(self.step._install()))

        self.assertEqual(
            runner.await_args_list,
            [call(command) for command in self.step._build_commands("ubuntu")],
        )
        self.assertEqual(self.step._password, "")
        self.step._pw_entry.set_text.assert_called_with("")
        self.step.status.set_text.assert_called_with("MayaFlux installed successfully.")

    def test_failed_install_command_stops_sequence_and_clears_password(self):
        with patch.object(self.step, "_detect_distro", return_value="ubuntu"):
            with patch.object(self.step, "_is_pkg_installed", return_value=False):
                with patch.object(self.step, "_run_pty", new_callable=AsyncMock, return_value=False) as runner:
                    self.assertFalse(asyncio.run(self.step._install()))

        self.assertEqual(runner.await_count, 1)
        self.assertEqual(self.step._password, "")
        self.step.status.set_text.assert_called_with("ERROR: Installation failed.")

    def test_declined_conflict_does_not_remove_or_install_packages(self):
        with patch.object(self.step, "_detect_distro", return_value="ubuntu"):
            with patch.object(self.step, "_is_pkg_installed", return_value=True):
                with patch.object(self.step, "_show_conflict_dialog", side_effect=lambda _a, _b, event: event.set()):
                    with patch.object(self.step, "_run_pty", new_callable=AsyncMock) as runner:
                        with patch.object(
                            asyncio.BaseEventLoop,
                            "run_in_executor",
                            lambda loop, executor, func: asyncio.sleep(0),
                        ):
                            self.assertFalse(asyncio.run(self.step._install()))
                            runner.assert_not_awaited()

        self.assertEqual(self.step._password, "")
        self.step._install_btn.set_sensitive.assert_called_with(True)

    def test_approved_conflict_removes_old_package_before_installing(self):
        self.step._conflict_confirmed = True
        self.step.release_type = "dev"
        with patch.object(self.step, "_detect_distro", return_value="ubuntu"):
            with patch.object(self.step, "_is_pkg_installed", return_value=True):
                with patch.object(self.step, "_show_conflict_dialog", side_effect=lambda _a, _b, event: event.set()):
                    with patch.object(self.step, "_run_pty", new_callable=AsyncMock, return_value=True) as runner:
                        with patch.object(
                            asyncio.BaseEventLoop,
                            "run_in_executor",
                            lambda loop, executor, func: asyncio.sleep(0),
                        ):
                            self.assertTrue(asyncio.run(self.step._install()))

        self.assertEqual(
            runner.await_args_list,
            [
                call(["sudo", "apt-get", "remove", "-y", "mayaflux"]),
                *[call(command) for command in self.step._build_commands("ubuntu")],
            ],
        )
        self.assertEqual(self.step._password, "")

    def test_completion_deploys_cli_inside_user_home(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "bundle"
            home = Path(temporary) / "home"
            scripts = root / "lib" / "scripts"
            templates = root / "lib" / "templates"
            scripts.mkdir(parents=True)
            templates.mkdir(parents=True)
            home.mkdir()
            (scripts / "create_project.sh").write_text("#!/bin/bash\n")
            (templates / "CMakeLists.txt").write_text("project(Test)\n")
            completion = installation_mode.CompletionStep()
            completion._status = Mock()
            done = Mock()

            with patch.dict(os.environ, {"WEAVE_ROOT": str(root)}):
                with patch.object(installation_mode.Path, "home", return_value=home):
                    completion._deploy_cli(done)

            launcher = home / ".local" / "bin" / "weave"
            self.assertEqual(launcher.read_text(), "#!/bin/bash\n")
            self.assertTrue(launcher.stat().st_mode & stat.S_IXUSR)
            self.assertEqual(
                (home / ".local" / "share" / "weave" / "templates" / "CMakeLists.txt").read_text(),
                "project(Test)\n",
            )
            done.assert_called_once_with(True)


if __name__ == "__main__":
    unittest.main()
