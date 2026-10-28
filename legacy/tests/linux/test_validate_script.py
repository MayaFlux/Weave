import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT.parent / "templates" / "validate.sh"
PROJECT_FILES = (
    "CMakeLists.txt", "CMakePresets.json", "cmake/mayaflux.cmake",
    "cmake/shaders.cmake", "cmake/build_community.cmake", "src/main.cpp",
    "src/user_project.hpp", "community.cmake", ".gitignore",
)


def executable(path, body):
    path.write_text("#!/bin/sh\n" + body)
    path.chmod(0o755)


def fixture(root, manager):
    project = root / "User Project"
    for relative in PROJECT_FILES:
        file = project / relative
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text("project content\n")
    (project / "CMakePresets.json").write_text(json.dumps({
        "version": 6,
        "configurePresets": [{"name": "debug"}, {"name": "release"}],
        "buildPresets": [
            {"name": "debug", "configurePreset": "debug"},
            {"name": "release", "configurePreset": "release"},
        ],
    }))
    (project / "community.cmake").write_text("")
    (project / "data" / "shaders").mkdir(parents=True)
    prefix = root / "Maya Flux"
    config = prefix / "lib" / "cmake" / "MayaFlux" / "MayaFluxConfig.cmake"
    config.parent.mkdir(parents=True)
    config.write_text("package content\n")
    binary = root / "bin"
    binary.mkdir()
    (binary / "python3").symlink_to(sys.executable)
    for command in ("find", "grep"):
        (binary / command).symlink_to(shutil.which(command))
    if manager == "pacman":
        executable(binary / "pacman", 'case "$1" in -Q) [ "${PACKAGE_PRESENT:-1}" = 1 ] ;; -Dk) [ "${DEPS_OK:-1}" = 1 ] ;; esac\n')
    elif manager == "dnf":
        executable(binary / "rpm", '[ "${PACKAGE_PRESENT:-1}" = 1 ]\n')
        executable(binary / "dnf", '[ "${DEPS_OK:-1}" = 1 ]\n')
    else:
        executable(binary / "dpkg-query", '[ "${PACKAGE_PRESENT:-1}" = 1 ] || exit 1\nprintf "install ok installed"\n')
        executable(binary / "apt-get", '[ "${DEPS_OK:-1}" = 1 ]\n')
    return project, prefix, binary


def run_validator(project, prefix, binary, **settings):
    environment = {
        "PATH": str(binary),
        "MAYAFLUX_ROOT": str(prefix),
        **settings,
    }
    return subprocess.run(
        ["/bin/bash", str(SCRIPT), str(project)],
        env=environment,
        capture_output=True,
        text=True,
        timeout=10,
    )


class ValidateScriptTests(unittest.TestCase):
    def test_supported_packages_and_dependencies_report_healthy_without_writes(self):
        for manager in ("pacman", "dnf", "apt"):
            with self.subTest(manager=manager), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                project, prefix, binary = fixture(root, manager)
                before = {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()}
                result = run_validator(project, prefix, binary)
                after = {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()}
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn("No problems found", result.stdout)
                self.assertEqual(after, before)

    def test_missing_package_and_broken_dependencies_fail_for_each_manager(self):
        for manager in ("pacman", "dnf", "apt"):
            for setting in ({"PACKAGE_PRESENT": "0"}, {"DEPS_OK": "0"}):
                with self.subTest(manager=manager, setting=setting), tempfile.TemporaryDirectory() as temporary:
                    project, prefix, binary = fixture(Path(temporary), manager)
                    result = run_validator(project, prefix, binary, **setting)
                    self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                    self.assertIn("[FIX]", result.stdout)

    def test_missing_installation_file_and_project_file_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project, prefix, binary = fixture(root, "pacman")
            (prefix / "lib" / "cmake" / "MayaFlux" / "MayaFluxConfig.cmake").unlink()
            (project / "cmake" / "shaders.cmake").unlink()
            result = run_validator(project, prefix, binary)
            self.assertEqual(result.returncode, 1)
            self.assertIn("MayaFluxConfig.cmake is missing", result.stdout)
            self.assertIn("Missing project file: cmake/shaders.cmake", result.stdout)

    def test_malformed_or_incomplete_project_presets_fail(self):
        for presets in ("{broken", '{"version": 6, "configurePresets": [], "buildPresets": []}'):
            with self.subTest(presets=presets), tempfile.TemporaryDirectory() as temporary:
                project, prefix, binary = fixture(Path(temporary), "pacman")
                (project / "CMakePresets.json").write_text(presets)
                result = run_validator(project, prefix, binary)
                self.assertEqual(result.returncode, 1)
                self.assertIn("Project presets", result.stdout)

    def test_unfinished_template_substitution_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            project, prefix, binary = fixture(Path(temporary), "pacman")
            (project / "CMakeLists.txt").write_text("project(@PROJECT_NAME@)\n")
            result = run_validator(project, prefix, binary)
            self.assertEqual(result.returncode, 1)
            self.assertIn("template substitution is unfinished", result.stdout)

    def test_missing_or_unsafe_community_module_fails(self):
        for entry in ("missing_module", "../escape"):
            with self.subTest(entry=entry), tempfile.TemporaryDirectory() as temporary:
                project, prefix, binary = fixture(Path(temporary), "pacman")
                (project / "community.cmake").write_text(entry + "\n")
                result = run_validator(project, prefix, binary)
                self.assertEqual(result.returncode, 1)
                self.assertIn("community module", result.stdout.lower())

    def test_pending_shader_output_is_advisory_and_read_only(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project, prefix, binary = fixture(root, "pacman")
            shader = project / "data/shaders/blur.comp"
            shader.write_text("shader source\n")
            result = run_validator(project, prefix, binary)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("shader outputs are missing or older", result.stdout)
            self.assertFalse((project / "data/shaders/blur.comp.spv").exists())


if __name__ == "__main__":
    unittest.main()
