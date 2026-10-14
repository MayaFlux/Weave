import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "linux" / "scripts" / "create_project.sh"
TEMPLATES = ROOT / "templates"


def run_cli(*args, root, environment=None):
    variables = os.environ.copy()
    variables.update(
        {
            "HOME": str(root),
            "WEAVE_TEMPLATE_DIR": str(TEMPLATES),
            "MAYAFLUX_ROOT": str(root / "mayaflux"),
        }
    )
    if environment:
        variables.update(environment)
    return subprocess.run(
        ["/bin/bash", str(SCRIPT), *map(str, args)],
        cwd=root,
        env=variables,
        capture_output=True,
        text=True,
        timeout=30,
    )


class LinuxCliTests(unittest.TestCase):
    def test_new_creates_project_from_templates(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = run_cli("new", "AudioGarden", root, "--no-vscode", root=root)

            self.assertEqual(result.returncode, 0, result.stderr)
            project = root / "AudioGarden"
            self.assertIn("project(AudioGarden", (project / "CMakeLists.txt").read_text())
            self.assertTrue((project / "src" / "main.cpp").is_file())
            self.assertTrue((project / "cmake" / "mayaflux.cmake").is_file())
            self.assertTrue((project / "community.cmake").is_file())
            self.assertEqual((project / "validate.sh").read_text(), (TEMPLATES / "validate.sh").read_text())
            self.assertTrue(os.access(project / "validate.sh", os.X_OK))
            report = (project / "run-report.sh").read_text()
            self.assertEqual(report, (TEMPLATES / "run-report.sh").read_text().replace("@PROJECT_NAME@", "AudioGarden"))
            self.assertIn("project_name='AudioGarden'", report)
            self.assertTrue(os.access(project / "run-report.sh", os.X_OK))
            self.assertIn("./run-report.sh", (project / "README.md").read_text())
            presets = json.loads((project / "CMakePresets.json").read_text())
            self.assertEqual({entry["name"] for entry in presets["configurePresets"]}, {"debug", "release"})
            self.assertIn("cmake --preset release", (project / "README.md").read_text())
            self.assertFalse((project / ".vscode").exists())

    def test_shader_copy_preserves_nested_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "mayaflux" / "share" / "MayaFlux" / "src" / "shaders"
            (source / "nested").mkdir(parents=True)
            (source / "include").mkdir()
            (source / "nested" / "edit.comp").write_text("editable source")
            (source / "include" / "common.glsl").write_text("shared include")
            (source / "nested" / "edit.comp.spv").write_text("binary")

            result = run_cli("new", "AudioGarden", root, "--copy-shader-sources", root=root)

            self.assertEqual(result.returncode, 0, result.stderr)
            shaders = root / "AudioGarden" / "data" / "shaders"
            self.assertEqual((shaders / "nested" / "edit.comp").read_text(), "editable source")
            self.assertEqual((shaders / "include" / "common.glsl").read_text(), "shared include")
            self.assertFalse((shaders / "nested" / "edit.comp.spv").exists())

    def test_missing_or_empty_shader_sources_fail_before_project_creation(self):
        for empty in (False, True):
            with self.subTest(empty=empty), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                if empty:
                    (root / "mayaflux" / "share" / "MayaFlux" / "src" / "shaders").mkdir(parents=True)
                result = run_cli("new", "AudioGarden", root, "--copy-shader-sources", root=root)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root / "AudioGarden").exists())

    def test_linked_shader_sources_fail_before_project_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "mayaflux" / "share" / "MayaFlux" / "src" / "shaders"
            source.mkdir(parents=True)
            (source / "linked").symlink_to(root, target_is_directory=True)
            result = run_cli("new", "AudioGarden", root, "--copy-shader-sources", root=root)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / "AudioGarden").exists())

    def test_invalid_project_name_and_missing_parent_fail_without_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            invalid = run_cli("new", "bad/name", root, root=root)
            missing = run_cli("new", "GoodName", root / "missing", root=root)
            self.assertNotEqual(invalid.returncode, 0)
            self.assertNotEqual(missing.returncode, 0)
            self.assertFalse((root / "bad").exists())
            self.assertFalse((root / "missing").exists())

    def test_existing_project_and_linked_parent_are_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            existing = root / "AudioGarden"
            existing.mkdir()
            (existing / "keep.txt").write_text("user content")
            linked = root / "linked"
            linked.symlink_to(root, target_is_directory=True)
            first = run_cli("new", "AudioGarden", root, root=root)
            second = run_cli("new", "OtherProject", linked, root=root)
            self.assertNotEqual(first.returncode, 0)
            self.assertNotEqual(second.returncode, 0)
            self.assertEqual((existing / "keep.txt").read_text(), "user content")
            self.assertFalse((root / "OtherProject").exists())

    def test_failed_template_copy_removes_only_new_project_and_allows_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            other = root / "OtherProject"
            other.mkdir()
            (other / "keep.txt").write_text("user content")
            fake_bin = root / "fake-bin"
            fake_bin.mkdir()
            fake_cp = fake_bin / "cp"
            fake_cp.write_text("#!/bin/sh\nexit 7\n")
            fake_cp.chmod(0o755)
            failed = run_cli("new", "AudioGarden", root, root=root, environment={"PATH": f"{fake_bin}:{os.environ['PATH']}"})
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse((root / "AudioGarden").exists(), failed.stderr)
            self.assertEqual((other / "keep.txt").read_text(), "user content")
            retry = run_cli("new", "AudioGarden", root, root=root)
            self.assertEqual(retry.returncode, 0, retry.stderr)

    def test_missing_required_project_template_fails_before_project_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            templates = root / "templates"
            shutil.copytree(TEMPLATES, templates)
            (templates / ".gitignore").unlink()
            result = run_cli("new", "AudioGarden", root, root=root, environment={"WEAVE_TEMPLATE_DIR": str(templates)})
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / "AudioGarden").exists())

    @unittest.skipUnless(shutil.which("git"), "git is required for module scaffolding")
    def test_missing_community_template_fails_before_module_creation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            templates = root / "templates"
            shutil.copytree(TEMPLATES, templates)
            (templates / "community" / "CMakePresets.json").unlink()
            result = run_cli("community", "terrain_field", root, root=root, environment={"WEAVE_TEMPLATE_DIR": str(templates)})
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / "terrain_field").exists())

    @unittest.skipUnless(shutil.which("git"), "git is required for module scaffolding")
    def test_failed_community_generation_preserves_sibling_and_allows_retry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sibling = root / "OtherModule"
            sibling.mkdir()
            (sibling / "keep.txt").write_text("user content")
            fake_bin = root / "fake-bin"
            fake_bin.mkdir()
            fake_cp = fake_bin / "cp"
            fake_cp.write_text("#!/bin/sh\nexit 7\n")
            fake_cp.chmod(0o755)
            failed = run_cli("community", "terrain_field", root, root=root, environment={"PATH": f"{fake_bin}:{os.environ['PATH']}"})
            self.assertNotEqual(failed.returncode, 0)
            self.assertFalse((root / "terrain_field").exists(), failed.stderr)
            self.assertEqual((sibling / "keep.txt").read_text(), "user content")
            retry = run_cli("community", "terrain_field", root, root=root)
            self.assertEqual(retry.returncode, 0, retry.stderr)

    def test_new_rejects_unknown_option_without_creating_project(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = run_cli("new", "AudioGarden", root, "--unexpected", root=root)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("Unknown option", result.stderr)
            self.assertFalse((root / "AudioGarden").exists())

    @unittest.skipUnless(shutil.which("git"), "git is required for module scaffolding")
    def test_community_creates_module_and_initializes_repository(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = run_cli("community", "terrain_field", root, root=root)

            self.assertEqual(result.returncode, 0, result.stderr)
            module = root / "terrain_field"
            self.assertTrue((module / "terrain_field.cmake").is_file())
            self.assertTrue((module / "src").is_dir())
            self.assertTrue((module / "test" / "CMakeLists.txt").is_file())
            self.assertTrue((module / ".git").exists())
            self.assertEqual(
                json.loads((module / "community.json").read_text())["name"],
                "terrain_field",
            )

    def test_community_rejects_invalid_name_without_creating_module(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = run_cli("community", "../escape", root, root=root)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("snake_case", result.stderr)
            self.assertFalse((root / "escape").exists())

    def test_update_clones_and_registers_module_once_without_network(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "AudioGarden"
            project.mkdir()
            (project / "CMakeLists.txt").write_text("project(AudioGarden)\n")
            fixture = root / "module_fixture"
            (fixture / "src").mkdir(parents=True)
            (fixture / "terrain_field.cmake").write_text("")
            prefix = root / "mayaflux" / "lib" / "cmake" / "MayaFlux"
            prefix.mkdir(parents=True)
            (prefix / "MayaFluxConfigVersion.cmake").write_text(
                'set(PACKAGE_VERSION "0.6.0")\n'
            )

            registry = root / "registry.json"
            registry.write_text(
                json.dumps(
                    [
                        {
                            "name": "terrain_field",
                            "repo": "https://example.org/terrain_field.git",
                            "min_version": "0.5.0",
                        }
                    ]
                )
            )
            bin_dir = root / "bin"
            bin_dir.mkdir()
            for command in ("python3", "grep", "sed", "mkdir", "touch"):
                executable = shutil.which(command)
                if executable is None:
                    self.skipTest(f"{command} is required for the CLI")
                (bin_dir / command).symlink_to(executable)
            curl = bin_dir / "curl"
            curl.write_text(
                '#!/usr/bin/env python3\n'
                'import os, pathlib, sys\n'
                'sys.stdout.write(pathlib.Path(os.environ["REGISTRY_FIXTURE"]).read_text())\n'
            )
            curl.chmod(0o755)
            git = bin_dir / "git"
            git.write_text(
                '#!/usr/bin/env python3\n'
                'import os, pathlib, shutil, sys\n'
                'if sys.argv[1] != "clone":\n'
                '    sys.exit(1)\n'
                'shutil.copytree(os.environ["MODULE_FIXTURE"], sys.argv[-1])\n'
                'pathlib.Path(os.environ["GIT_LOG"]).open("a").write("clone\\n")\n'
            )
            git.chmod(0o755)
            clone_log = root / "git.log"
            environment = {
                "PATH": str(bin_dir),
                "REGISTRY_FIXTURE": str(registry),
                "MODULE_FIXTURE": str(fixture),
                "GIT_LOG": str(clone_log),
            }

            first = run_cli("update", project, "terrain_field", root=root, environment=environment)
            second = run_cli("update", project, "terrain_field", root=root, environment=environment)

            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertTrue((project / "community" / "terrain_field" / "src").is_dir())
            self.assertEqual(clone_log.read_text(), "clone\n")
            self.assertEqual(
                (project / "community.cmake").read_text(), "terrain_field\n"
            )

    def test_update_rejects_missing_module_name_before_registry_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "AudioGarden"
            project.mkdir()
            (project / "CMakeLists.txt").write_text("project(AudioGarden)\n")

            result = run_cli("update", project, root=root)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("At least one module name required", result.stderr)
            self.assertFalse((project / "community.cmake").exists())


if __name__ == "__main__":
    unittest.main()
