"""Analysis must gate landing counts and preserve the scheduled denominator."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from analyze import campaign_audit, verified_row


class AnalysisContracts(unittest.TestCase):
    def test_partial_tuple_is_never_a_landing(self):
        row = {"disposition": "executed", "verified_target_landing": True,
               "execution_valid": True, "integrity_passed": True,
               "physical_replay_verified": True, "complete_replay_verified": True,
               "stop": "mission_terminal", "physical_outcome": "landed_on_target", "mission_outcome": "success"}
        self.assertTrue(verified_row(row))
        for key in ["execution_valid", "integrity_passed", "physical_replay_verified", "complete_replay_verified"]:
            incomplete = copy.deepcopy(row)
            incomplete[key] = False
            self.assertFalse(verified_row(incomplete))

    def test_report_cannot_remove_unsupported_attempt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            summary = {"request": {"agent": {}, "attempt_indices": [0]}, "integrity_passed": True,
                       "attempts": [{"mission_index": 0, "disposition": {"kind": "unsupported"}}]}
            page = {"agent": {}, "integrity_passed": True, "attempts": []}
            (root/"summary.json").write_text(json.dumps(summary))
            (root/"report-input.json").write_text(json.dumps(page))
            with self.assertRaisesRegex(ValueError, "denominator"):
                campaign_audit(root)


if __name__ == "__main__":
    unittest.main()
