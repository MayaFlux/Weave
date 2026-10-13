import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from linux.lib import community_update
from linux.lib.community_update import UpdateError


def existing_project(root, manifest):
    (root / "CMakeLists.txt").write_text("project(Test)\n", encoding="utf-8")
    module = root / "community" / "terrain_field"
    (module / "src").mkdir(parents=True)
    (module / "terrain_field.cmake").write_text("", encoding="utf-8")
    (module / "community.json").write_text(json.dumps(manifest), encoding="utf-8")


class ProjectUpdateTests(unittest.TestCase):
    def test_valid_existing_module_is_registered_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            existing_project(project, {"name": "terrain_field", "min_version": "0.5.0"})
            logs = []

            with patch.object(
                community_update,
                "fetch_registry",
                return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")},
            ), patch.object(
                community_update, "installed_version", return_value=("0.5.0", (0, 5, 0))
            ), patch.object(community_update, "clone_module") as clone:
                self.assertEqual(
                    community_update.update(project, ["terrain_field"], None, logs.append),
                    ["terrain_field"],
                )
                self.assertEqual(
                    community_update.update(project, ["terrain_field"], None, logs.append),
                    ["terrain_field"],
                )
                clone.assert_not_called()

            self.assertEqual(
                (project / "community.cmake").read_text(encoding="utf-8"),
                "terrain_field\n",
            )
            self.assertIn("terrain_field was already registered", logs)

    def test_manifest_name_mismatch_cannot_modify_registration(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            existing_project(project, {"name": "other", "min_version": "0.5.0"})

            with patch.object(
                community_update,
                "fetch_registry",
                return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")},
            ), patch.object(
                community_update, "installed_version", return_value=("0.5.0", (0, 5, 0))
            ):
                with self.assertRaises(UpdateError) as raised:
                    community_update.update(project, ["terrain_field"], None, lambda _: None)

            self.assertIn("Invalid manifest for terrain_field", str(raised.exception))
            self.assertFalse((project / "community.cmake").exists())

    def test_incompatible_version_cannot_modify_registration(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            existing_project(project, {"name": "terrain_field", "min_version": "0.7.0"})

            with patch.object(
                community_update,
                "fetch_registry",
                return_value={"terrain_field": ("https://example.org/terrain.git", "0.5.0")},
            ), patch.object(
                community_update, "installed_version", return_value=("0.6.0", (0, 6, 0))
            ):
                with self.assertRaises(UpdateError) as raised:
                    community_update.update(project, ["terrain_field"], None, lambda _: None)

            self.assertIn("requires MayaFlux >= 0.7.0", str(raised.exception))
            self.assertFalse((project / "community.cmake").exists())

    def test_unknown_module_cannot_modify_registration(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary)
            (project / "CMakeLists.txt").write_text("project(Test)\n", encoding="utf-8")

            with patch.object(community_update, "fetch_registry", return_value={}), patch.object(
                community_update, "installed_version", return_value=("0.6.0", (0, 6, 0))
            ), patch.object(community_update, "clone_module") as clone:
                with self.assertRaises(UpdateError) as raised:
                    community_update.update(project, ["unknown"], None, lambda _: None)
                clone.assert_not_called()

            self.assertIn("not in the community registry", str(raised.exception))
            self.assertFalse((project / "community.cmake").exists())


if __name__ == "__main__":
    unittest.main()
