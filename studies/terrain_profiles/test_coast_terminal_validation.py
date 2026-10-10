import copy
import json
import tempfile
import unittest
from pathlib import Path

import ballistic_feedback_sweep as sweep
import coast_terminal_validation as validation
import study
import waypoint_clearance_panel as previous


class CoastValidationTests(unittest.TestCase):
    def rows(self):
        primary, repeats = validation.layout(True)
        return [{"case_id": name, "attempt_id": name, "status": "recorded",
                 "cohort": "random" if name.startswith("random-") else "control",
                 "previous_result": {"verified_landing": True},
                 "result": {"verified_landing": True, "physical_outcome": "landed_on_target", "handoffs": 1}}
                for name in primary] + [
                    {"case_id": name, "attempt_id": "repeat-" + name, "status": "recorded", "cohort": "repeat",
                     "result": {"verified_landing": True, "physical_outcome": "landed_on_target", "handoffs": 1}}
                    for name in repeats]

    def test_full_inventory_is_fixed_and_old_focus_stays_separate(self):
        primary, repeats = validation.layout(True)
        self.assertEqual(primary[:16], [f"random-{i:03}" for i in previous.INDICES])
        self.assertEqual(primary[16:], previous.CONTROLS)
        self.assertEqual(repeats, ["random-084", "random-715"])
        self.assertEqual(len(primary) + len(repeats), 21)
        with self.assertRaises(ValueError):
            validation.layout()

    def test_sweep_gate_rejects_loss_bad_subject_and_abnormal_outcome(self):
        rows = self.rows()
        self.assertTrue(validation.gate(rows)["sweep_gate_passed"])
        for name, change in [("random-349", {"verified_landing": False, "physical_outcome": "flying"}),
                             ("random-084", {"handoffs": 2}),
                             ("random-715", {"physical_outcome": "crashed"})]:
            invalid = copy.deepcopy(rows)
            next(r for r in invalid if r["case_id"] == name)["result"].update(change)
            self.assertFalse(validation.gate(invalid)["sweep_gate_passed"])
        with self.assertRaises(ValueError):
            validation.gate(rows[1:])

    def test_identity_cannot_rebuild_or_swap_candidate(self):
        source = {"executable_sha256": validation.NATIVE_SHA, "rust_source_tree_sha256": validation.RUST_SHA,
                  "renderer_sha256": validation.RENDERER_SHA}
        validation.require_identity(source)
        for key in source:
            with self.assertRaises(ValueError):
                validation.require_identity({**source, key: "different"})

    def test_conditional_sweep_contract_cannot_change_identity_or_allowance(self):
        plan = sweep.contract(sweep.COAST_PLAN)
        self.assertEqual(plan["candidate_id"], sweep.COAST_CANDIDATE)
        self.assertEqual(plan["waypoint_experiment"], "coast-terminal")
        self.assertEqual(plan["repeat_indices"], [84, 715])
        self.assertEqual(plan["maximum_measured_attempts"], 1002)
        for key, value in [("candidate_id", sweep.TERRAIN_CANDIDATE), ("repeat_indices", [349]),
                           ("primary_cases", 999), ("correction_cap", 48), ("retries", 1), ("publication", True)]:
            with tempfile.TemporaryDirectory() as temp:
                path = Path(temp) / "plan.json"
                study.write_new(path, study.encoded({**plan, key: value}))
                with self.assertRaises(ValueError):
                    sweep.contract(path)

    def test_portable_gate_binds_all_three_documents_and_rejects_failed_panel(self):
        source = {"executable_sha256": validation.NATIVE_SHA, "rust_source_tree_sha256": validation.RUST_SHA,
                  "renderer_sha256": validation.RENDERER_SHA}
        manifest = study.encoded({"source": source})
        report = {"schema": "pd-lab.coast-terminal-preservation-panel.v1", "stopped_reason": None,
                  "source_after": source, "rows": self.rows()}
        results = study.encoded(report)
        receipt = study.encoded({"files": {"ballistic-feedback-sweep.json": study.digest(results),
                                           "manifest.json": study.digest(manifest)}})
        plan = {"gate_panel_receipt_sha256": study.digest(receipt),
                "gate_panel_results_sha256": study.digest(results), "gate_panel_manifest_sha256": study.digest(manifest)}
        validation.verify_gate_data(plan, receipt, results, manifest)
        for key in plan:
            with self.assertRaises(ValueError):
                validation.verify_gate_data({**plan, key: "different"}, receipt, results, manifest)
        report["rows"][0]["result"]["verified_landing"] = False
        failed = study.encoded(report)
        bad_receipt = json.loads(receipt)
        bad_receipt["files"]["ballistic-feedback-sweep.json"] = study.digest(failed)
        bad_receipt = study.encoded(bad_receipt)
        with self.assertRaises(ValueError):
            validation.verify_gate_data({**plan, "gate_panel_receipt_sha256": study.digest(bad_receipt),
                                         "gate_panel_results_sha256": study.digest(failed)},
                                        bad_receipt, failed, manifest)


if __name__ == "__main__":
    unittest.main()
