import unittest
from unittest.mock import patch

import ballistic_mechanics_panel as panel
import ballistic_feedback_sweep as sweep


class MechanicsTests(unittest.TestCase):
    def rows(self, gain=None, loss=None):
        winners = set(panel.PRESERVED + panel.DIRECT)
        rows = []
        for i in panel.FOCUS:
            rows.append({"case_id": f"random-{i:03}", "cohort": "random",
                         "previous_result": {"verified_landing": i in winners},
                         "result": {"verified_landing": i == gain or (i in winners and i != loss)}})
        return {"rows": rows, "stopped_reason": None}

    def test_fixed_denominators_and_separate_repeats(self):
        self.assertEqual(len(panel.FOCUS), 45)
        self.assertEqual([len(part) for part in panel.layout()], [45, 3])
        self.assertEqual([len(part) for part in panel.layout(True)], [1000, 5])
        self.assertEqual(len(panel.PRESERVED + panel.DIRECT), 25)
        self.assertEqual(len(set(panel.FOCUS)), 45)

    def test_each_mode_requires_its_subject_gain_and_no_losses(self):
        self.assertFalse(panel.admission(self.rows(), "exit-consistency")["passed"])
        self.assertTrue(panel.admission(self.rows(gain=47), "exit-consistency")["passed"])
        self.assertFalse(panel.admission(self.rows(gain=44), "exit-consistency")["passed"])
        self.assertFalse(panel.admission(self.rows(gain=47, loss=349), "exit-consistency")["passed"])
        incomplete = self.rows(gain=47)
        incomplete["stopped_reason"] = "evidence_error"
        self.assertFalse(panel.admission(incomplete, "exit-consistency")["passed"])

    def test_combination_cannot_bypass_a_failed_mechanism(self):
        with patch.object(panel.entry, "source", return_value={}), patch.object(
                panel, "require_admission", side_effect=ValueError("failed")), patch.object(panel.entry, "run") as run:
            with self.assertRaises(ValueError):
                panel.run("mechanics-combined")
            run.assert_not_called()

    def test_only_combined_may_request_full_sweep(self):
        with self.assertRaises(ValueError):
            panel.run("exit-consistency", full=True)

    def test_mechanics_identity_cannot_drop_inherited_terminal_behavior(self):
        feedback = {"candidate_id": sweep.MECHANICS_CANDIDATES["exit-consistency"], "stop": "no_ballistic_aim"}
        with self.assertRaises(ValueError):
            sweep.validate_terminal_coordination({"terminal_coordination": {"enabled": False}}, feedback)

    def test_readers_recognize_all_inherited_mechanics_contracts(self):
        attempt = {
            "landing_countdown": {"enabled": True, "rule": "retain_first_selected_ballistic_fallback_arrival",
                                  "admissions_per_landing": 1, "remaining_time_floor_s": 0.5,
                                  "revalidate_each_update": True, "release_on_infeasible_or_expired": True,
                                  "target_convention_changed": False, "ordinary_default_changed": False},
            "landing_braking_guard": {"enabled": True, "rule": "existing_braking_envelope_activates_touchdown_rescue",
                                      "nominal_upward_acceleration_insufficient": True, "standalone_coast_terminal_changed": False,
                                      "ordinary_default_changed": False, "short_command_guard_changed": False},
            "landing_body_centering": {"enabled": True, "rule": "rotated_hull_and_feet_rescue_pad_interval",
                                       "reuse_outside_pad_lateral_target": True, "vertical_authority_cap_unchanged": True,
                                       "standalone_coast_terminal_changed": False, "ordinary_default_changed": False,
                                       "short_command_guard_changed": False},
            "coast_terminal": sweep.COAST_METADATA,
        }
        for candidate in sweep.MECHANICS_CANDIDATES.values():
            sweep.validate_landing_countdown(attempt, candidate)
            sweep.validate_landing_braking_guard(attempt, candidate)
            sweep.validate_landing_body_centering(attempt, candidate)
            sweep.validate_coast_terminal(attempt, {"candidate_id": candidate, "refreshes": [], "updates": []})


if __name__ == "__main__":
    unittest.main()
