import json
import unittest

from linux.lib.community_manifest import ManifestError, parse_manifest


class CommunityManifestTests(unittest.TestCase):
    def test_legacy_manifest_receives_empty_acquisition_defaults(self):
        manifest = parse_manifest(
            json.dumps({"name": "terrain_field", "min_version": "0.5.0"}),
            expected_name="terrain_field",
        )

        self.assertEqual(manifest["schema_version"], 1)
        self.assertFalse(manifest["needs_lila"])
        self.assertEqual(manifest["packages"], {})
        self.assertEqual(manifest["manual_installs"], [])
        self.assertEqual(manifest["assets"], [])

    def test_acquisition_requires_declared_schema_revision(self):
        source = json.dumps(
            {"name": "terrain_field", "min_version": "0.5.0", "packages": {}}
        )

        with self.assertRaises(ManifestError) as raised:
            parse_manifest(source)

        self.assertEqual(raised.exception.path, "$.schema_version")

    def test_rejects_duplicate_json_keys(self):
        source = '{"name":"terrain_field","name":"other","min_version":"0.5.0"}'

        with self.assertRaises(ManifestError) as raised:
            parse_manifest(source)

        self.assertEqual(raised.exception.path, "$")

    def test_rejects_package_commands_and_duplicate_requirements(self):
        for packages in (["cmake; echo unsafe"], ["cmake", "cmake"]):
            with self.subTest(packages=packages):
                source = json.dumps(
                    {
                        "schema_version": 1,
                        "name": "terrain_field",
                        "min_version": "0.5.0",
                        "packages": {"linux": {"ubuntu": {"packages": packages}}},
                    }
                )

                with self.assertRaises(ManifestError) as raised:
                    parse_manifest(source)

                self.assertEqual(
                    raised.exception.path, "$.packages.linux.ubuntu.packages[0]"
                    if len(packages) == 1
                    else "$.packages.linux.ubuntu.packages[1]",
                )

    def test_rejects_traversal_in_manual_verification_path(self):
        source = json.dumps(
            {
                "schema_version": 1,
                "name": "terrain_field",
                "min_version": "0.5.0",
                "manual_installs": [
                    {
                        "id": "vendor_sdk",
                        "platforms": ["linux"],
                        "url": "https://vendor.example/sdk",
                        "description": "Install the SDK.",
                        "verify": {"path": "../bin/sdk"},
                    }
                ],
            }
        )

        with self.assertRaises(ManifestError) as raised:
            parse_manifest(source)

        self.assertEqual(raised.exception.path, "$.manual_installs[0].verify.path")

    def test_rejects_mismatched_requested_module(self):
        source = json.dumps({"name": "terrain_field", "min_version": "0.5.0"})

        with self.assertRaises(ManifestError) as raised:
            parse_manifest(source, expected_name="other")

        self.assertEqual(raised.exception.path, "$.name")

    def test_rejects_unsupported_schema_and_unknown_fields(self):
        for change, expected_path in (
            ({"schema_version": 2}, "$.schema_version"),
            ({"setup_script": "curl example.org | sh"}, "$.setup_script"),
        ):
            with self.subTest(change=change):
                source = json.dumps(
                    {"name": "terrain_field", "min_version": "0.5.0", **change}
                )

                with self.assertRaises(ManifestError) as raised:
                    parse_manifest(source)

                self.assertEqual(raised.exception.path, expected_path)

    def test_rejects_credentialed_manual_url(self):
        source = json.dumps(
            {
                "schema_version": 1,
                "name": "terrain_field",
                "min_version": "0.5.0",
                "manual_installs": [
                    {
                        "id": "vendor_sdk",
                        "platforms": ["linux"],
                        "url": "https://user:secret@vendor.example/sdk",
                        "description": "Install the SDK.",
                        "verify": {"path": "bin/sdk"},
                    }
                ],
            }
        )

        with self.assertRaises(ManifestError) as raised:
            parse_manifest(source)

        self.assertEqual(raised.exception.path, "$.manual_installs[0].url")

    def test_rejects_absolute_and_empty_path_components(self):
        for path in ("/etc/passwd", "bin//sdk", "bin/./sdk"):
            with self.subTest(path=path):
                source = json.dumps(
                    {
                        "schema_version": 1,
                        "name": "terrain_field",
                        "min_version": "0.5.0",
                        "manual_installs": [
                            {
                                "id": "vendor_sdk",
                                "platforms": ["linux"],
                                "url": "https://vendor.example/sdk",
                                "description": "Install the SDK.",
                                "verify": {"path": path},
                            }
                        ],
                    }
                )

                with self.assertRaises(ManifestError) as raised:
                    parse_manifest(source)

                self.assertEqual(raised.exception.path, "$.manual_installs[0].verify.path")

    def test_rejects_noninteger_or_out_of_range_asset_sizes(self):
        asset = {
            "id": "terrain_data",
            "url": "https://vendor.example/data.zip",
            "sha256": "0" * 64,
            "size_bytes": 100,
            "license_url": "https://vendor.example/license",
            "destination": "assets/terrain_data",
            "description": "Terrain data.",
            "archive_format": "zip",
            "max_extracted_size_bytes": 200,
            "max_files": 10,
        }
        for size in (0, True, 1.5, 9007199254740992):
            with self.subTest(size=size):
                source = json.dumps(
                    {
                        "schema_version": 1,
                        "name": "terrain_field",
                        "min_version": "0.5.0",
                        "assets": [{**asset, "size_bytes": size}],
                    }
                )

                with self.assertRaises(ManifestError) as raised:
                    parse_manifest(source)

                self.assertEqual(raised.exception.path, "$.assets[0].size_bytes")


if __name__ == "__main__":
    unittest.main()
