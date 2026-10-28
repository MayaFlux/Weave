import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from linux.lib import community_update
from linux.lib.community_update import (
    UpdateError,
    fetch_registry,
    installed_version,
    parse_names,
    parse_version,
    register,
)


class CommunityUpdateTests(unittest.TestCase):
    def test_parse_names_preserves_order_and_removes_duplicates(self):
        self.assertEqual(
            parse_names([" terrain_field, noise ", "noise,sky"]),
            ["terrain_field", "noise", "sky"],
        )

    def test_parse_names_rejects_empty_and_invalid_names(self):
        for values in ([], ["terrain_field,"], ["../terrain"]):
            with self.subTest(values=values), self.assertRaises(UpdateError):
                parse_names(values)

    def test_parse_version_requires_three_canonical_components(self):
        self.assertEqual(parse_version("2.10.0", "module"), (2, 10, 0))
        for version in ("2.10", "02.10.0", "2.10.0-rc1"):
            with self.subTest(version=version), self.assertRaises(UpdateError):
                parse_version(version, "module")

    def test_installed_version_reads_explicit_prefix(self):
        with tempfile.TemporaryDirectory() as temporary:
            prefix = Path(temporary)
            version_file = prefix / "lib" / "cmake" / "MayaFlux" / "MayaFluxConfigVersion.cmake"
            version_file.parent.mkdir(parents=True)
            version_file.write_text('set(PACKAGE_VERSION "0.6.2")\n', encoding="utf-8")

            self.assertEqual(installed_version(prefix), ("0.6.2", (0, 6, 2)))

    def test_register_keeps_existing_modules_and_adds_only_new_names(self):
        with tempfile.TemporaryDirectory() as temporary:
            registry = Path(temporary) / "community.cmake"
            registry.write_text("terrain_field", encoding="utf-8")

            self.assertEqual(register(registry, ["terrain_field", "sky"]), ["sky"])
            self.assertEqual(registry.read_text(encoding="utf-8"), "terrain_field\nsky\n")
            self.assertEqual(register(registry, ["sky"]), [])
            self.assertEqual(registry.read_text(encoding="utf-8"), "terrain_field\nsky\n")

    def test_fetch_registry_rejects_malformed_or_unsafe_entries(self):
        valid = {
            "name": "terrain_field",
            "repo": "https://example.org/terrain_field.git",
            "min_version": "0.5.0",
        }
        registries = (
            {"name": "terrain_field"},
            [valid, valid],
            [{**valid, "repo": "http://example.org/terrain_field.git"}],
            [{**valid, "name": "../terrain_field"}],
            [{**valid, "min_version": "0.5"}],
        )
        for registry in registries:
            with self.subTest(registry=registry):
                response = io.BytesIO(json.dumps(registry).encode("utf-8"))
                with patch.object(community_update, "urlopen", return_value=response):
                    with self.assertRaises(UpdateError):
                        fetch_registry()

    def test_invalid_project_is_rejected_before_registry_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            with patch.object(community_update, "fetch_registry") as fetch:
                with self.assertRaises(UpdateError):
                    community_update.update(temporary, ["terrain_field"], None, lambda _: None)
                fetch.assert_not_called()

    def test_failed_clone_does_not_register_module(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "CMakeLists.txt").write_text("project(Test)\n", encoding="utf-8")

            def fail_clone(repo, destination, report):
                destination.mkdir()
                raise UpdateError("Clone failed")

            with patch.object(
                community_update,
                "fetch_registry",
                return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")},
            ), patch.object(
                community_update, "installed_version", return_value=("0.5.0", (0, 5, 0))
            ), patch.object(community_update, "clone_module", side_effect=fail_clone):
                with self.assertRaises(UpdateError) as raised:
                    community_update.update(project, ["terrain_field"], None, lambda _: None)

            self.assertIn("Clone failed", str(raised.exception))
            self.assertFalse((project / "community.cmake").exists())
            self.assertFalse((project / "community" / "terrain_field").exists())

    def test_failed_clone_removes_dangling_module_link(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "CMakeLists.txt").write_text("project(Test)\n")

            def fail_clone(repo, destination, report):
                destination.symlink_to(project / "missing-module")
                raise UpdateError("Clone failed")

            with patch.object(community_update, "fetch_registry", return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")}):
                with patch.object(community_update, "installed_version", return_value=("0.5.0", (0, 5, 0))):
                    with patch.object(community_update, "clone_module", side_effect=fail_clone):
                        with self.assertRaises(UpdateError):
                            community_update.update(project, ["terrain_field"], None, lambda _: None)

            self.assertFalse((project / "community/terrain_field").is_symlink())

    def test_failed_clone_can_be_retried_without_touching_existing_module(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "CMakeLists.txt").write_text("project(Test)\n")
            existing = project / "community" / "existing"
            existing.mkdir(parents=True)
            (existing / "keep.txt").write_text("user content")
            attempts = 0

            def clone(repo, destination, report):
                nonlocal attempts
                attempts += 1
                destination.mkdir()
                if attempts == 1:
                    (destination / "partial.txt").write_text("partial")
                    raise UpdateError("Clone failed")
                (destination / "src").mkdir()
                (destination / "terrain_field.cmake").write_text("cmake")
                (destination / "community.json").write_text(
                    json.dumps({"name": "terrain_field", "min_version": "0.5.0"})
                )

            with patch.object(community_update, "fetch_registry", return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")}):
                with patch.object(community_update, "installed_version", return_value=("0.6.0", (0, 6, 0))):
                    with patch.object(community_update, "clone_module", side_effect=clone):
                        with self.assertRaises(UpdateError):
                            community_update.update(project, ["terrain_field"], None, lambda _: None)
                        self.assertFalse((project / "community" / "terrain_field").exists())
                        self.assertEqual((existing / "keep.txt").read_text(), "user content")
                        community_update.update(project, ["terrain_field"], None, lambda _: None)

            self.assertEqual((project / "community.cmake").read_text(), "terrain_field\n")
            self.assertEqual(attempts, 2)

    def test_failed_registration_preserves_original_and_removes_temporary_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            registration = Path(temporary) / "community.cmake"
            registration.write_text("existing\n")
            with patch.object(community_update.os, "replace", side_effect=OSError("write failed")):
                with self.assertRaises(OSError):
                    register(registration, ["terrain_field"])
            self.assertEqual(registration.read_text(), "existing\n")
            self.assertEqual(list(Path(temporary).iterdir()), [registration])
            self.assertEqual(register(registration, ["terrain_field"]), ["terrain_field"])
            self.assertEqual(registration.read_text(), "existing\nterrain_field\n")

    def test_linked_registration_is_rejected_before_registry_access(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            project = root / "project"
            project.mkdir()
            (project / "CMakeLists.txt").write_text("project(Test)\n")
            outside = root / "outside.txt"
            outside.write_text("user content")
            (project / "community.cmake").symlink_to(outside)
            with patch.object(community_update, "fetch_registry") as fetch:
                with self.assertRaises(UpdateError):
                    community_update.update(project, ["terrain_field"], None, lambda _: None)
                fetch.assert_not_called()
            self.assertEqual(outside.read_text(), "user content")


if __name__ == "__main__":
    unittest.main()
