import json
import unittest
from pathlib import Path

from linux.lib.community_manifest import ManifestError, parse_manifest


VECTORS = Path(__file__).resolve().parents[1] / "vectors" / "community"


class CommunityManifestVectorTests(unittest.TestCase):
    def test_corpus_is_present(self):
        names = [path.name for path in VECTORS.glob("*.json")]

        self.assertTrue(any(name.startswith("valid-") for name in names), VECTORS)
        self.assertTrue(any(name.startswith("invalid-") for name in names), VECTORS)
        for name in names:
            self.assertTrue(name.startswith(("valid-", "invalid-")), name)

    def test_every_vector_is_accepted_or_rejected_as_named(self):
        for path in sorted(VECTORS.glob("*.json")):
            with self.subTest(vector=path.name):
                source = path.read_text(encoding="utf-8")
                name = json.loads(source)["name"]

                if path.name.startswith("valid-"):
                    parse_manifest(source, expected_name=name)
                else:
                    with self.assertRaises(ManifestError):
                        parse_manifest(source, expected_name=name)


if __name__ == "__main__":
    unittest.main()
