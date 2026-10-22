"""Scheduled failure accounting independently of optimizer quality."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import numpy as np

from evaluate import campaign, execute_attempt, resolve_accounted


class EvaluationContracts(unittest.TestCase):
    def test_bad_native_input_keeps_order_and_valid_neighbors(self):
        def native_read(root, indices, usage_id, exposure):
            self.assertEqual(exposure, "evaluation")
            if 2 in indices:
                raise ValueError("resolved mission bytes changed")
            return json.dumps([{"index": index} for index in indices])

        with patch("evaluate.pd_native.load_collection_missions", side_effect=native_read):
            resolved = resolve_accounted(Path("unused"), [{"index": i} for i in range(4)], "test")
        self.assertEqual([item[0] for item in resolved], [{"index": 0}, {"index": 1}, None, {"index": 3}])
        self.assertIn("bytes changed", resolved[2][1])

    def test_policy_and_capture_failures_remain_distinct(self):
        class Episode:
            def save_capture(self, *args):
                raise OSError("capture volume unavailable")

        class Environment:
            def __init__(self, **kwargs):
                self.episode, self.raw = Episode(), np.zeros(17)

            def reset(self):
                return np.zeros(64), {}

            def step(self, action):
                return np.zeros(64), 0, True, False, {}

        class Policy:
            def predict(self, *args, **kwargs):
                return np.zeros(2), None

        class BrokenPolicy:
            def predict(self, *args, **kwargs):
                raise ValueError("policy emitted an invalid action")

        with patch("evaluate.pd_native.evidence_snapshot", return_value="{}"):
            arguments = (Environment, {}, {"implementation": "test"}, Path("unused"), "attempt", [], [], False)
            broken = execute_attempt(BrokenPolicy(), *arguments)
            capture = execute_attempt(Policy(), *arguments)
        self.assertEqual(broken[0]["kind"], "execution_failed")
        self.assertEqual(capture[0]["kind"], "capture_failed")
        self.assertIsNone(broken[1])
        self.assertIsNone(capture[1])

    def test_invalid_count_does_not_create_campaign(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/"manifest.json").write_text(json.dumps({"entries": [{"index": 0}]}))
            for count in [0, -1, 2]:
                with self.assertRaises(ValueError):
                    campaign(root, root/"unused.zip", root/"campaign", root/"receipt", count)
                self.assertFalse((root/"campaign").exists())


if __name__ == "__main__":
    unittest.main()
