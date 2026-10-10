import copy
import unittest

import ballistic_feedback_sweep as sweep
import terminal_braking_panel as panel


class TerminalBrakingTests(unittest.TestCase):
    def rows(self, full=False):
        primary, repeats = panel.previous.layout(True) if full else panel.focus_layout()
        return [{"case_id": name, "attempt_id": name, "status": "recorded",
                 "cohort": "random" if name.startswith("random-") else "control",
                 "previous_result": {"verified_landing": True},
                 "result": {"verified_landing": True, "physical_outcome": "landed_on_target"}}
                for name in primary] + [
                    {"case_id": name, "attempt_id": "repeat-" + name, "status": "recorded", "cohort": "repeat",
                     "result": {"verified_landing": True, "physical_outcome": "landed_on_target"}}
                    for name in repeats]

    def test_allowance_and_identities(self):
        self.assertEqual(panel.focus_layout(), (["random-142", "random-974"], ["random-142", "random-974"]))
        self.assertEqual(sum(map(len, panel.focus_layout())) + sum(map(len, panel.previous.layout(True))), 25)
        with self.assertRaises(ValueError):
            panel.focus_layout(True)
        self.assertNotEqual(panel.NATIVE, panel.previous.focused.NATIVE)
        self.assertEqual(panel.FOCUS_SPEC.control_mode, "coast-terminal")
        self.assertIn(sweep.BRAKING_CANDIDATE, sweep.LANDING_FAMILY.values())

    def test_gates_reject_failure_loss_and_abnormal_outcome(self):
        for full in (False, True):
            rows = self.rows(full)
            self.assertTrue(panel.gate(rows, full)["passed"])
            lost = copy.deepcopy(rows)
            lost[0]["result"].update(verified_landing=False, physical_outcome="flying")
            self.assertFalse(panel.gate(lost, full)["passed"])
            bad = copy.deepcopy(rows)
            bad[-1]["result"]["physical_outcome"] = "crashed"
            self.assertFalse(panel.gate(bad, full)["passed"])
            with self.assertRaises(ValueError):
                panel.gate(rows[:-1], full)

    def test_guard_metadata_cannot_relax_existing_boundaries(self):
        metadata = {"enabled": True, "rule": "existing_braking_envelope_activates_touchdown_rescue",
                    "nominal_upward_acceleration_insufficient": True, "standalone_coast_terminal_changed": False,
                    "ordinary_default_changed": False, "short_command_guard_changed": False}
        sweep.validate_landing_braking_guard({"landing_braking_guard": metadata}, sweep.BRAKING_CANDIDATE)
        for key in ("standalone_coast_terminal_changed", "ordinary_default_changed", "short_command_guard_changed"):
            with self.assertRaises(ValueError):
                sweep.validate_landing_braking_guard({"landing_braking_guard": {**metadata, key: True}}, sweep.BRAKING_CANDIDATE)
        with self.assertRaises(ValueError):
            sweep.validate_landing_braking_guard({"landing_braking_guard": metadata}, sweep.COAST_CANDIDATE)
        sweep.validate_landing_braking_guard({}, sweep.COAST_CANDIDATE)
        with self.assertRaises(ValueError):
            sweep.validate_landing_braking_guard({}, sweep.BRAKING_CANDIDATE)


if __name__ == "__main__":
    unittest.main()
