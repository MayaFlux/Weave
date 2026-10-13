import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


def appimage_path():
    configured = os.environ.get("WEAVE_APPIMAGE")
    if configured:
        return Path(configured).resolve()
    project = (ROOT / "linux" / "pyproject.toml").read_text()
    version = re.search(r'^version\s*=\s*"([^"]+)"', project, re.MULTILINE)
    if version is None:
        raise AssertionError("Linux package version is missing")
    return ROOT / "build" / "linux" / f"Weave-{version.group(1)}-x86_64.AppImage"


def is_within(path, root):
    return os.path.commonpath((str(Path(path).resolve()), str(root.resolve()))) == str(
        root.resolve()
    )


class AppImageIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        image = appimage_path()
        if not image.is_file():
            raise AssertionError(f"Built AppImage is missing: {image}")
        cls._temporary = tempfile.TemporaryDirectory(prefix="weave-appimage-test-")
        cls.workdir = Path(cls._temporary.name)
        try:
            extracted = subprocess.run(
                [str(image), "--appimage-extract"],
                cwd=cls.workdir,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
                timeout=120,
            )
            if extracted.returncode != 0:
                raise AssertionError(
                    f"AppImage extraction failed ({extracted.returncode}): "
                    f"{extracted.stderr}"
                )
            cls.appdir = cls.workdir / "squashfs-root"
            if not cls.appdir.is_dir():
                raise AssertionError("AppImage extraction produced no AppDir")
        except BaseException:
            cls._temporary.cleanup()
            raise

    @classmethod
    def tearDownClass(cls):
        cls._temporary.cleanup()

    def runtime_environment(self):
        root = self.appdir
        python_lib = root / "usr" / "lib" / "python3.11"
        return {
            "HOME": str(self.workdir),
            "PATH": "/usr/bin:/bin",
            "PYTHONHOME": str(root / "usr"),
            "PYTHONPATH": ":".join(
                (str(python_lib), str(python_lib / "lib-dynload"),
                 str(python_lib / "site-packages"), str(root / "lib"))
            ),
            "PYTHONNOUSERSITE": "1",
            "GI_TYPELIB_PATH": str(root / "usr" / "lib" / "girepository-1.0"),
            "LD_LIBRARY_PATH": str(root / "usr" / "lib"),
            "XDG_DATA_DIRS": str(root / "usr" / "share"),
            "GSETTINGS_SCHEMA_DIR": str(root / "usr" / "share" / "glib-2.0" / "schemas"),
            "GDK_BACKEND": "wayland",
            "WEAVE_ROOT": str(root),
            "WEAVE_TEMPLATE_DIR": str(root / "lib" / "templates"),
        }

    def run_packaged_cli(self, *arguments, environment=None):
        variables = self.runtime_environment()
        if environment:
            variables.update(environment)
        return subprocess.run(
            [
                "/bin/bash",
                str(self.appdir / "lib" / "scripts" / "create_project.sh"),
                *map(str, arguments),
            ],
            cwd=self.workdir,
            env=variables,
            capture_output=True,
            text=True,
            timeout=30,
        )

    def test_required_application_payload_is_in_image(self):
        required = (
            "AppRun",
            "Weave.desktop",
            "weave-config.json",
            "lib/main.py",
            "lib/modes/installation.py",
            "lib/modes/project.py",
            "lib/modes/community.py",
            "lib/modes/update.py",
            "lib/scripts/create_project.sh",
            "lib/templates/CMakeLists.txt",
            "lib/templates/main.cpp",
            "lib/templates/community",
            "usr/bin/python3",
            "usr/lib/girepository-1.0/Gtk-4.0.typelib",
            "usr/lib/girepository-1.0/Adw-1.typelib",
            "usr/share/glib-2.0/schemas/gschemas.compiled",
        )
        for relative in required:
            with self.subTest(path=relative):
                self.assertTrue((self.appdir / relative).exists(), relative)

    def test_bundled_python_imports_application_and_bundled_gtk(self):
        probe = r'''
import gi
import json
import os
import runpy
import sys
from pathlib import Path

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw
runpy.run_path(
    str(Path(os.environ["WEAVE_ROOT"]) / "lib" / "main.py"),
    run_name="weave_appimage_probe",
)
from lib.config import get_config

maps = Path("/proc/self/maps").read_text().splitlines()
libraries = {}
for name in ("libgtk-4.so", "libadwaita-1.so"):
    libraries[name] = sorted({
        line.split()[-1] for line in maps
        if name in line and line.split()[-1].startswith("/")
    })
print(json.dumps({
    "prefix": sys.prefix,
    "gi": gi.__file__,
    "libraries": libraries,
    "scripts": str(get_config().scripts_dir),
    "templates": str(get_config().templates_dir),
    "gtk_version": Gtk.get_major_version(),
    "adw_version": Adw.get_major_version(),
}))
'''
        result = subprocess.run(
            [str(self.appdir / "usr" / "bin" / "python3"), "-c", probe],
            cwd=self.workdir,
            env=self.runtime_environment(),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        details = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertEqual(Path(details["prefix"]).resolve(), self.appdir / "usr")
        self.assertTrue(is_within(details["gi"], self.appdir), details["gi"])
        self.assertEqual(details["gtk_version"], 4)
        self.assertEqual(Path(details["scripts"]), self.appdir / "lib" / "scripts")
        self.assertEqual(Path(details["templates"]), self.appdir / "lib" / "templates")
        for name, paths in details["libraries"].items():
            with self.subTest(library=name):
                self.assertTrue(paths, f"{name} was not loaded")
                self.assertTrue(
                    all(is_within(path, self.appdir) for path in paths), paths
                )

    def test_packaged_cli_creates_project_with_packaged_templates(self):
        destination = self.workdir / "projects"
        result = self.run_packaged_cli(
            "new", "AudioGarden", destination, "--no-vscode"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        project = destination / "AudioGarden"
        self.assertIn("project(AudioGarden", (project / "CMakeLists.txt").read_text())
        self.assertTrue((project / "src" / "main.cpp").is_file())
        self.assertTrue((project / "cmake" / "mayaflux.cmake").is_file())

    def test_packaged_cli_creates_community_module_with_packaged_templates(self):
        self.assertIsNotNone(shutil.which("git"), "git is required by the CLI")
        destination = self.workdir / "modules"
        result = self.run_packaged_cli("community", "terrain_field", destination)
        self.assertEqual(result.returncode, 0, result.stderr)
        module = destination / "terrain_field"
        self.assertTrue((module / ".git").is_dir())
        self.assertTrue((module / "terrain_field.cmake").is_file())
        self.assertTrue((module / "test" / "CMakeLists.txt").is_file())
        self.assertEqual(
            json.loads((module / "community.json").read_text())["name"],
            "terrain_field",
        )

    def test_packaged_cli_updates_project_from_local_registry_fixture(self):
        project = self.workdir / "updated-project"
        project.mkdir()
        (project / "CMakeLists.txt").write_text("project(AudioGarden)\n")
        module = self.workdir / "module-fixture"
        (module / "src").mkdir(parents=True)
        (module / "terrain_field.cmake").write_text("")
        registry = self.workdir / "registry.json"
        registry.write_text(
            json.dumps(
                [
                    {
                        "name": "terrain_field",
                        "repo": "https://example.invalid/terrain_field.git",
                        "min_version": "0.5.0",
                    }
                ]
            )
        )
        prefix = self.workdir / "mayaflux" / "lib" / "cmake" / "MayaFlux"
        prefix.mkdir(parents=True)
        (prefix / "MayaFluxConfigVersion.cmake").write_text(
            'set(PACKAGE_VERSION "0.6.0")\n'
        )
        commands = self.workdir / "commands"
        commands.mkdir()
        curl = commands / "curl"
        curl.write_text('#!/bin/sh\ncat "$WEAVE_REGISTRY_FIXTURE"\n')
        curl.chmod(0o755)
        git = commands / "git"
        git.write_text(
            '#!/bin/sh\n'
            '[ "$1" = "clone" ] || exit 1\n'
            'cp -R "$WEAVE_MODULE_FIXTURE" "$4"\n'
        )
        git.chmod(0o755)
        environment = {
            "PATH": f"{commands}:/usr/bin:/bin",
            "MAYAFLUX_ROOT": str(self.workdir / "mayaflux"),
            "WEAVE_REGISTRY_FIXTURE": str(registry),
            "WEAVE_MODULE_FIXTURE": str(module),
        }
        result = self.run_packaged_cli(
            "update", project, "terrain_field", environment=environment
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((project / "community" / "terrain_field" / "src").is_dir())
        self.assertEqual((project / "community.cmake").read_text(), "terrain_field\n")

    def test_packaged_completion_deploys_working_cli(self):
        home = self.workdir / "install-home"
        home.mkdir()
        environment = self.runtime_environment()
        environment["HOME"] = str(home)
        deploy = subprocess.run(
            [
                str(self.appdir / "usr" / "bin" / "python3"),
                "-c",
                "import os, sys\n"
                "sys.path.insert(0, os.environ['WEAVE_ROOT'])\n"
                "from lib.modes.installation import CompletionStep\n"
                "class Status:\n"
                "    def set_text(self, value):\n"
                "        pass\n"
                "step = CompletionStep()\n"
                "step._status = Status()\n"
                "step._deploy_cli(lambda success: None)\n",
            ],
            cwd=self.workdir,
            env=environment,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(deploy.returncode, 0, deploy.stderr)
        launcher = home / ".local" / "bin" / "weave"
        self.assertTrue(launcher.is_file())
        self.assertTrue(os.access(launcher, os.X_OK))
        self.assertTrue(
            (home / ".local" / "share" / "weave" / "templates" / "CMakeLists.txt").is_file()
        )
        environment.pop("WEAVE_TEMPLATE_DIR")
        project_root = self.workdir / "installed-cli-projects"
        created = subprocess.run(
            [str(launcher), "new", "AudioGarden", str(project_root), "--no-vscode"],
            cwd=self.workdir,
            env=environment,
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(created.returncode, 0, created.stderr)
        self.assertTrue((project_root / "AudioGarden" / "CMakeLists.txt").is_file())

    def test_packaged_installer_pty_reports_command_status(self):
        probe = r'''
import asyncio
import json
import os
import sys

sys.path.insert(0, os.environ["WEAVE_ROOT"])
from lib.modes.installation import DependenciesStep

step = DependenciesStep()
step._password = "fixture-password"
step._log = lambda line: None
accepted = asyncio.run(step._run_pty([
    "/bin/sh", "-c",
    'read confirmation; read password; '
    '[ "$confirmation" = y ] && [ "$password" = fixture-password ]',
]))
rejected = asyncio.run(step._run_pty(["/bin/sh", "-c", "exit 7"]))
print(json.dumps({"accepted": accepted, "rejected": rejected}))
'''
        result = subprocess.run(
            [str(self.appdir / "usr" / "bin" / "python3"), "-c", probe],
            cwd=self.workdir,
            env=self.runtime_environment(),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        details = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertEqual(details, {"accepted": True, "rejected": False})


if __name__ == "__main__":
    unittest.main()
