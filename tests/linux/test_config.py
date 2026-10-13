import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from linux.lib.config import WeaveConfig


class WeaveConfigTests(unittest.TestCase):
    def test_explicit_root_resolves_paths_and_environment(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            config = {
                "paths": {"scripts": "${WEAVE_ROOT}/scripts"},
                "executables": {"create": "${WEAVE_ROOT}/scripts/create.sh"},
                "environment_variables": {
                    "WEAVE_ROOT": "${WEAVE_ROOT}",
                    "PYTHONPATH": "${WEAVE_ROOT}/lib:${PYTHONPATH}",
                },
            }
            (root / "weave-config.json").write_text(json.dumps(config), encoding="utf-8")

            with patch.dict(os.environ, {"WEAVE_ROOT": str(root), "PYTHONPATH": "existing"}):
                loaded = WeaveConfig()
                self.assertEqual(loaded.root, root)
                self.assertEqual(loaded.scripts_dir, root / "scripts")
                self.assertEqual(loaded.get_script("create"), root / "scripts" / "create.sh")
                self.assertEqual(loaded.get("missing", "fallback"), "fallback")
                self.assertEqual(
                    loaded.get_env_vars(),
                    {"WEAVE_ROOT": str(root), "PYTHONPATH": f"{root}/lib:existing"},
                )


if __name__ == "__main__":
    unittest.main()
