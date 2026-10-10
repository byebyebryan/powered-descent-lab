import copy
import unittest

import ballistic_feedback_sweep as sweep
import ballistic_recovery_consistency_panel as panel
import test_ballistic_phase_transition_panel as transition_tests


class RecoveryConsistencyTests(unittest.TestCase):
    def test_frozen_layout_and_reference(self):
        self.assertEqual(len(panel.FOCUS), 48)
        self.assertEqual(len(set(panel.FOCUS)), 48)
        self.assertTrue(set(panel.SUCCESS + panel.RECOVERY).issubset(panel.FOCUS))
        self.assertEqual(set(panel.STAGES.values()), set(sweep.RECOVERY_CANDIDATES))
        self.assertEqual(sum(sum(map(len, panel.layout(s))) for s in panel.STAGES), 1107)
        self.assertEqual([len(x) for x in panel.layout("full")], [1000, 5])

    def evidence(self):
        attempt, feedback = transition_tests.PhaseTransitionTests().evidence()
        feedback["candidate_id"] = sweep.RECOVERY_CANDIDATES["recovery-consistency"]
        attempt["phase_transition"].update(actual_coast_settling=True,
                                         configured_terminal_takeover=True,
                                         pad_reserve_command_adapter=True,
                                         recovery_diagnostic_only=False)
        attempt["queued_recovery"] = {"enabled": True, "bounded_comparison": True, "goals_unchanged": True}
        origin = feedback["refreshes"][0]["origin"]
        queued = copy.deepcopy(feedback["refreshes"][0]["transition_query"])
        queued["kind"] = "bounded_queued_turn_burn_coast"
        feedback["refreshes"][0]["recovery_common_query"] = {
            "origin": origin, "requested_command": {"throttle_frac": 0, "target_attitude_rad": 0},
            "prediction_ticks": 120, "queued_program": queued,
            "commands": [{"choice": "upright_lift", "command": {"throttle_frac": 1, "target_attitude_rad": 0},
                          "prediction_ticks": 120, "checked": True, "conflict": None}]}
        return attempt, feedback

    def test_bounded_warning_rejects_hidden_extra_horizon(self):
        attempt, feedback = self.evidence()
        sweep.validate_phase_transition(attempt, feedback)
        queued = feedback["refreshes"][0]["recovery_common_query"]["queued_program"]
        queued["rejection"] = "native_guard"
        queued["conflict"] = {"state": {"physics_step": 221}, "cause": "short_command_reserve"}
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, feedback)
        queued["conflict"]["state"]["physics_step"] = 220
        sweep.validate_phase_transition(attempt, feedback)

    def test_old_kind_and_diagnostic_only_cannot_hide_active_recovery(self):
        attempt, feedback = self.evidence()
        bad = copy.deepcopy(feedback)
        bad["refreshes"][0]["recovery_common_query"]["queued_program"]["kind"] = "queued_turn_burn_coast"
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, bad)
        attempt["phase_transition"]["recovery_diagnostic_only"] = True
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, feedback)


if __name__ == "__main__":
    unittest.main()
