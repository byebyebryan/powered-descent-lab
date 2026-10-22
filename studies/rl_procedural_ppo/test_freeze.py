"""Selection freezes require complete allocations and unchanged checkpoints."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from freeze import development_selection


class FreezeContracts(unittest.TestCase):
    def fixture(self, root):
        for seed in [801, 802, 803]:
            directory = root/f"training-v1-seed-{seed}"
            directory.mkdir()
            checkpoint = directory/"checkpoint-final.zip"
            checkpoint.write_bytes(b"test-checkpoint")
            best = {"checkpoint": str(checkpoint), "checkpoint_sha256": hashlib.sha256(checkpoint.read_bytes()).hexdigest()}
            values = {
                "request.json": {"seed": seed, "requested_transitions": 8192, "source_identities": {}},
                "selection.json": {"source_identity_unchanged": True, "actual_transitions": 8192, "best": best},
                "development-final.json": {"scheduled": 128, "attempts": [
                    {"physical_sha256": str(index), "mission_outcome": "failed_timeout", "target_distance_m": index}
                    for index in range(128)]},
            }
            for name, value in values.items():
                (directory/name).write_text(json.dumps(value))

    def test_incomplete_allocation_cannot_enter_design_ranking(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            self.assertEqual(development_selection(root, "v1")["development_attempts"], 384)
            path = root/"training-v1-seed-802/selection.json"
            value = json.loads(path.read_text())
            value["actual_transitions"] = 8191
            path.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "has not completed"):
                development_selection(root, "v1")

    def test_changed_checkpoint_cannot_be_frozen(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.fixture(root)
            (root/"training-v1-seed-803/checkpoint-final.zip").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "bytes changed"):
                development_selection(root, "v1")


if __name__ == "__main__":
    unittest.main()
