import asyncio
import importlib.util
import os
import stat
import sys
import tempfile
import time
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

    def test_conflicting_packages_depend_on_distro_and_release(self):
        expected = {
            ("arch", "stable"): ("mayaflux-dev-bin",),
            ("arch", "dev"): ("mayaflux", "mayaflux-bin"),
            ("fedora", "stable"): ("mayaflux-dev",),
            ("fedora", "dev"): ("mayaflux",),
            ("ubuntu", "stable"): ("mayaflux-edge",),
            ("ubuntu", "dev"): ("mayaflux",),
        }
        for (distro, release), conflicts in expected.items():
            with self.subTest(distro=distro, release=release):
                self.step.release_type = release
                self.assertEqual(self.step._conflicting_pkgs(distro), conflicts)

    def test_arch_clones_fresh_package_even_with_aur_helper(self):
        for release, available, pkg in (
            ("stable", {"yay", "paru"}, "mayaflux"),
            ("dev", {"paru"}, "mayaflux-dev-bin"),
        ):
            with self.subTest(release=release):
                self.step.release_type = release
                with patch.object(
                    installation_mode.shutil,
                    "which",
                    side_effect=lambda name: f"/bin/{name}" if name in available else None,
                ):
                    self.assertEqual(
                        self.step._build_commands("arch", "/tmp/aur-build"),
                        [
                            ["git", "clone", f"https://aur.archlinux.org/{pkg}.git", "/tmp/aur-build"],
                            ["makepkg", "--dir", "/tmp/aur-build", "-si", "--noconfirm"],
                        ],
                    )

    def test_arch_removes_temporary_checkout_after_install_or_failure(self):
        self.step.release_type = "dev"
        for build_succeeds in (True, False):
            with self.subTest(build_succeeds=build_succeeds):
                checkout = None

                async def run_command(command):
                    nonlocal checkout
                    if command[0] == "git":
                        checkout = Path(command[-1])
                        self.assertTrue(checkout.is_dir())
                    else:
                        self.assertEqual(command[:3], ["makepkg", "--dir", str(checkout)])
                        self.assertTrue(checkout.is_dir())
                    return command[0] == "git" or build_succeeds

                with patch.object(self.step, "_detect_distro", return_value="arch"):
                    with patch.object(self.step, "_is_pkg_installed", return_value=False):
                        with patch.object(self.step, "_run_pty", side_effect=run_command):
                            self.assertEqual(asyncio.run(self.step._install()), build_succeeds)

                self.assertIsNotNone(checkout)
                self.assertFalse(checkout.exists())

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
            ("arch", "mayaflux\n", True),
            ("fedora", "", True),
            ("ubuntu", "install ok installed", True),
            ("ubuntu", "deinstall ok config-files", False),
        ):
            with self.subTest(distro=distro, output=output):
                result = types.SimpleNamespace(returncode=0, stdout=output)
                with patch.object(installation_mode.subprocess, "run", return_value=result) as run:
                    self.assertEqual(self.step._is_pkg_installed(distro, "mayaflux"), installed)
                    self.assertIn(run.call_args.args[0][0], ("pacman", "rpm", "dpkg-query"))

    def test_arch_provider_does_not_count_as_exact_package(self):
        for actual in ("mayaflux-dev-bin", "mayaflux-bin"):
            with self.subTest(actual=actual):
                result = types.SimpleNamespace(returncode=0, stdout=f"{actual}\n")
                with patch.object(installation_mode.subprocess, "run", return_value=result):
                    self.assertFalse(self.step._is_pkg_installed("arch", "mayaflux"))
                    self.assertTrue(self.step._is_pkg_installed("arch", actual))

    def test_reinstalling_arch_stable_bin_keeps_stable_variant(self):
        self.step.release_type = "stable"
        result = types.SimpleNamespace(returncode=0, stdout="mayaflux-bin\n")
        with patch.object(self.step, "_detect_distro", return_value="arch"):
            with patch.object(installation_mode.subprocess, "run", return_value=result):
                with patch.object(self.step, "_show_conflict_dialog", side_effect=AssertionError("Unexpected conflict dialog")) as dialog:
                    with patch.object(self.step, "_run_pty", new_callable=AsyncMock, return_value=True) as runner:
                        self.assertTrue(asyncio.run(self.step._install()))
        dialog.assert_not_called()
        self.assertEqual(
            runner.await_args_list[0].args[0][:3],
            ["git", "clone", "https://aur.archlinux.org/mayaflux-bin.git"],
        )

    def test_reinstalling_arch_dev_does_not_offer_to_remove_stable(self):
        self.step.release_type = "dev"
        result = types.SimpleNamespace(returncode=0, stdout="mayaflux-dev-bin\n")
        with patch.object(self.step, "_detect_distro", return_value="arch"):
            with patch.object(installation_mode.subprocess, "run", return_value=result):
                with patch.object(self.step, "_show_conflict_dialog", side_effect=AssertionError("Unexpected conflict dialog")) as dialog:
                    with patch.object(self.step, "_run_pty", new_callable=AsyncMock, return_value=True) as runner:
                        self.assertTrue(asyncio.run(self.step._install()))
        dialog.assert_not_called()
        self.assertEqual(runner.await_count, 2)

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
        self.step._install_btn.set_sensitive.assert_called_with(True)
        self.step._pw_entry.set_sensitive.assert_called_with(True)

    def test_stalled_package_transfer_ends_and_reports_retry(self):
        command = [
            sys.executable,
            "-c",
            "import sys,time\n"
            "for _ in range(20):\n"
            " sys.stdout.write('  0  0  0  0  0  0\\r')\n"
            " sys.stdout.flush()\n"
            " time.sleep(0.05)\n",
        ]
        with patch.object(installation_mode.DependenciesStep, "_STALL_TIMEOUT", 0.2, create=True):
            started = time.monotonic()
            result = asyncio.run(self.step._run_pty(command))
        self.assertFalse(result)
        self.assertLess(time.monotonic() - started, 0.8)
        self.assertTrue(any("no download progress" in call.args[0].lower() for call in self.step._log.call_args_list))

    def test_package_transfer_with_new_bytes_continues(self):
        command = [
            sys.executable,
            "-c",
            "import sys,time\n"
            "for amount in range(1, 7):\n"
            " sys.stdout.write(f'  0  0  0  {amount}K  0  0\\r')\n"
            " sys.stdout.flush()\n"
            " time.sleep(0.08)\n",
        ]
        with patch.object(installation_mode.DependenciesStep, "_STALL_TIMEOUT", 0.2):
            self.assertTrue(asyncio.run(self.step._run_pty(command)))

    def test_stalled_aria_transfer_ends_despite_status_messages(self):
        command = [
            sys.executable,
            "-c",
            "import sys,time\n"
            "for index in range(20):\n"
            " sys.stdout.write('[#2089b0 1.0MiB/84.0MiB(1%) CN:4 DL:0B ETA:0s]\\r')\n"
            " if index % 4 == 0: sys.stdout.write('Download Progress Summary\\n')\n"
            " sys.stdout.flush()\n"
            " time.sleep(0.05)\n",
        ]
        with patch.object(installation_mode.DependenciesStep, "_STALL_TIMEOUT", 0.2):
            started = time.monotonic()
            result = asyncio.run(self.step._run_pty(command))
        self.assertFalse(result)
        self.assertLess(time.monotonic() - started, 0.8)
        self.assertTrue(any("no download progress" in call.args[0].lower() for call in self.step._log.call_args_list))

    def test_aria_unknown_size_stall_logs_once_and_times_out(self):
        command = [
            sys.executable,
            "-c",
            "import sys,time\n"
            "for _ in range(20):\n"
            " sys.stdout.write('[#bcac73 0B/0B CN:1 DL:0B]\\r')\n"
            " sys.stdout.flush()\n"
            " time.sleep(0.05)\n",
        ]
        with patch.object(installation_mode.DependenciesStep, "_STALL_TIMEOUT", 0.2):
            started = time.monotonic()
            result = asyncio.run(self.step._run_pty(command))
        self.assertFalse(result)
        self.assertLess(time.monotonic() - started, 0.8)
        messages = [entry.args[0] for entry in self.step._log.call_args_list]
        self.assertEqual(messages.count("[#bcac73 0B/0B CN:1 DL:0B]"), 1)
        self.assertTrue(any("no download progress" in message.lower() for message in messages))

    def test_aria_transfer_with_new_bytes_continues(self):
        command = [
            sys.executable,
            "-c",
            "import sys,time\n"
            "for amount in range(1, 7):\n"
            " sys.stdout.write(f'[#2089b0 {amount}.0MiB/84.0MiB(1%) CN:4 DL:1MiB ETA:1m]\\r')\n"
            " sys.stdout.flush()\n"
            " time.sleep(0.08)\n",
        ]
        with patch.object(installation_mode.DependenciesStep, "_STALL_TIMEOUT", 0.2):
            self.assertTrue(asyncio.run(self.step._run_pty(command)))

    def test_pty_has_columns_for_aria_console_readout(self):
        command = [sys.executable, "-c", "import os; print(os.get_terminal_size(1).columns)"]
        self.assertTrue(asyncio.run(self.step._run_pty(command)))
        self.assertIn(call("80"), self.step._log.call_args_list)

    def test_aria_readout_does_not_log_blank_repaint_lines(self):
        command = [
            sys.executable,
            "-c",
            "import sys; sys.stdout.write('\\r   \\r\\x1b[35m[#2089b0 1.0MiB/8.0MiB(12%) CN:1]\\x1b[0m\\r')",
        ]
        self.assertTrue(asyncio.run(self.step._run_pty(command)))
        messages = [entry.args[0] for entry in self.step._log.call_args_list]
        self.assertTrue(any("1.0MiB/8.0MiB" in message for message in messages))
        self.assertTrue(all(message.strip() for message in messages))

    def test_failed_install_keeps_next_step_disabled(self):
        view = types.SimpleNamespace(next_btn=Mock())
        installation_mode.InstallationMode._on_step_done(view, False)
        view.next_btn.set_sensitive.assert_called_once_with(False)

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
