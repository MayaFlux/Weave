import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = ROOT / "templates" / "run-report.sh"
APPLICATION = """#!/bin/bash
echo "cwd=$(pwd -P)"
echo "config=${2:-INVALID}"
echo "environment=${WEAVE_REPORT_VALUE:-unset}"
echo "stdout: diagnostic output"
echo "stderr: crash details" >&2
if [ "${WEAVE_REPORT_NO_LOG:-0}" != 1 ]; then
    mkdir -p data/log
    printf 'fresh application report' >data/log/mayaflux_report.log
fi
exit "${WEAVE_REPORT_EXIT:-0}"
"""


def write_executable(path, body):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)
    path.chmod(0o755)


class RunReportScriptTests(unittest.TestCase):
    def setUp(self):
        self._temporary = tempfile.TemporaryDirectory()
        self.root = Path(self._temporary.name).resolve()
        self.project = self.root / "User's Project [test] & spaces"
        self.install = self.root / "Maya Flux [install]"
        self.config = self.install / "share/MayaFlux/report/mayaflux.report.json"
        self.config.parent.mkdir(parents=True)
        self.config.write_text('{"journal":{}}\n')
        script = TEMPLATE.read_text().replace("@PROJECT_NAME@", "ReportFixture")
        write_executable(self.project / "run-report.sh", script)
        write_executable(self.project / "build/ReportFixture", APPLICATION)

    def tearDown(self):
        self._temporary.cleanup()

    def run_script(self, *arguments, environment=None, inherit_root=True):
        variables = {"PATH": os.environ["PATH"], "HOME": str(self.root)}
        if inherit_root:
            variables["MAYAFLUX_ROOT"] = str(self.install)
        variables.update(environment or {})
        result = subprocess.run(
            ["/bin/bash", str(self.project / "run-report.sh"), *arguments],
            cwd=self.root, env=variables, capture_output=True, text=True, timeout=30,
        )
        return result.returncode, result.stdout + result.stderr

    def read(self, relative):
        return (self.project / relative).read_text()

    def previous(self, relative):
        path = self.project / relative
        return [item.read_text() for item in path.parent.glob(path.name + ".previous-*")]

    def test_runs_application_with_configuration_and_captures_both_channels(self):
        status, output = self.run_script(environment={"WEAVE_REPORT_VALUE": "terminal value"})
        capture = self.read("mayaflux_run.txt")

        self.assertEqual(status, 0, output)
        self.assertIn(f"cwd={self.project}", capture)
        self.assertIn(f"config={self.config}", capture)
        self.assertIn("environment=terminal value", capture)
        self.assertIn("stdout: diagnostic output", capture)
        self.assertIn("stderr: crash details", capture)
        self.assertEqual(self.read("data/log/mayaflux_report.log"), "fresh application report")
        self.assertIn(str(self.project / "mayaflux_run.txt"), output)

    def test_application_failure_is_returned_and_previous_logs_are_preserved(self):
        (self.project / "mayaflux_run.txt").write_text("previous console")
        (self.project / "data/log").mkdir(parents=True)
        (self.project / "data/log/mayaflux_report.log").write_text("previous report")

        status, output = self.run_script(environment={"WEAVE_REPORT_EXIT": "37"})

        self.assertEqual(status, 37, output)
        self.assertIn("Application exit code: 37", output)
        self.assertEqual(self.previous("mayaflux_run.txt"), ["previous console"])
        self.assertEqual(self.previous("data/log/mayaflux_report.log"), ["previous report"])
        self.assertEqual(self.read("data/log/mayaflux_report.log"), "fresh application report")

    def test_early_crash_explains_missing_application_report(self):
        status, output = self.run_script(environment={"WEAVE_REPORT_EXIT": "23", "WEAVE_REPORT_NO_LOG": "1"})

        self.assertEqual(status, 23, output)
        self.assertIn("Application report was not created", output)
        self.assertIn("stderr: crash details", self.read("mayaflux_run.txt"))

    def test_unresolved_project_name_falls_back_to_cmake_cache(self):
        write_executable(self.project / "run-report.sh", TEMPLATE.read_text())
        (self.project / "build/CMakeCache.txt").write_text("CMAKE_PROJECT_NAME:STATIC=ReportFixture\n")

        self.assertEqual(self.run_script()[0], 0)

        (self.project / "build/CMakeCache.txt").unlink()
        status, output = self.run_script()
        self.assertEqual(status, 1)
        self.assertIn("Build the project first", output)

    def test_invalid_inputs_fail_clearly_without_rotating_logs(self):
        (self.project / "mayaflux_run.txt").write_text("previous console")
        cases = (
            (["--unsupported"], "Unknown option"),
            (["--executable"], "requires a value"),
            (["--configuration", "Fast"], "Choose Debug, Release"),
            (["--project-name", "../outside"], "project name is unsafe"),
            (["--project-name", "bad name"], "project name is unsafe"),
            (["--executable", "missing"], "missing or is not executable"),
            (["--executable", "build"], "missing or is not executable"),
            (["--mayaflux-root", str(self.root / "absent")], "installation folder does not exist"),
            (["--mayaflux-root", str(self.root)], "report configuration is missing"),
            (["--project-root", str(self.root / "absent")], "project folder does not exist"),
        )
        for arguments, message in cases:
            with self.subTest(arguments=arguments):
                status, output = self.run_script(*arguments)
                self.assertEqual(status, 1, output)
                self.assertIn(message, output)
        self.assertEqual(self.read("mayaflux_run.txt"), "previous console")
        self.assertFalse((self.project / "data/log/mayaflux_report.log").exists())

    def test_malformed_report_configuration_is_rejected(self):
        (self.project / "mayaflux_run.txt").write_text("previous console")
        for content in ("{broken", "[1]", "", '"text"'):
            with self.subTest(content=content):
                self.config.write_text(content)
                status, output = self.run_script()
                self.assertEqual(status, 1, output)
                self.assertIn("not a readable JSON object", output)
        self.assertEqual(self.read("mayaflux_run.txt"), "previous console")

    @unittest.skipIf(
        any(Path(prefix, "share/MayaFlux/report/mayaflux.report.json").is_file() for prefix in ("/usr", "/usr/local")),
        "MayaFlux is installed in a system prefix",
    )
    def test_missing_mayaflux_root_is_recoverable(self):
        status, output = self.run_script(inherit_root=False)

        self.assertEqual(status, 1, output)
        self.assertIn("MayaFlux was not found", output)
        self.assertIn("MAYAFLUX_ROOT", output)

    def test_multiple_configurations_require_selection(self):
        write_executable(self.project / "build/Release/ReportFixture", APPLICATION)

        status, output = self.run_script()
        self.assertEqual(status, 1)
        self.assertIn("multiple project executables", output)
        (self.project / "build/ReportFixture").unlink()
        self.assertEqual(self.run_script()[0], 0)
        self.assertEqual(self.run_script("--configuration", "Release")[0], 0)
        self.assertEqual(self.run_script("--configuration", "Debug")[0], 1)

    def test_explicit_executable_supports_custom_layout_with_spaces(self):
        (self.project / "build/ReportFixture").unlink()
        write_executable(self.project / "custom output/Renamed", APPLICATION)

        status, output = self.run_script("--executable", "custom output/Renamed")

        self.assertEqual(status, 0, output)
        self.assertIn(f"config={self.config}", self.read("mayaflux_run.txt"))

    def test_unstartable_executable_reports_error_without_rotating_logs(self):
        (self.project / "mayaflux_run.txt").write_text("previous console")
        write_executable(self.project / "build/Broken", "\x00\x01\x02\x03")

        status, output = self.run_script("--executable", "build/Broken")

        self.assertEqual(status, 1, output)
        self.assertIn("[ERROR]", output)
        self.assertIn("could not be started", output)
        self.assertEqual(self.read("mayaflux_run.txt"), "previous console")


if __name__ == "__main__":
    unittest.main()
