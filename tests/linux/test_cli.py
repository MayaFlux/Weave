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
            self.assertFalse((project / ".vscode").exists())

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
