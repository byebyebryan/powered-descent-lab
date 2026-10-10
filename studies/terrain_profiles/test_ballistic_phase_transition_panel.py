import copy
import unittest

import ballistic_feedback_sweep as sweep
import ballistic_phase_transition_panel as panel


class PhaseTransitionTests(unittest.TestCase):
    def test_frozen_denominators_and_separate_repeats(self):
        self.assertEqual(len(set(panel.FOCUS)), 29)
        for stage in panel.STAGES:
            self.assertEqual([len(x) for x in panel.layout(stage)], [1000, 5] if stage == "full" else [29, 3])
        self.assertEqual(set(panel.STAGES.values()), set(sweep.PHASE_CANDIDATES))

    def evidence(self):
        origin = {"physics_step": 100}
        query = {"origin": origin, "kind": "actual_idle_rotation", "prediction_ticks": 120,
                 "checked_ticks": 120, "end_state": {"physics_step": 220},
                 "conflict": None, "domain_stop": None, "rejection": None, "first_frame": None}
        feedback = {"candidate_id": sweep.PHASE_CANDIDATES["transition-probe"],
                    "refreshes": [{"origin": origin, "transition_query": query}],
                    "updates": [], "ordinary_flight": {"actions": []}}
        attempt = {"phase_transition": {"queries_recorded": True, "actual_coast_settling": False,
                   "configured_terminal_takeover": False, "pad_reserve_command_adapter": False,
                   "recovery_diagnostic_only": True, "terminal_prefix_ticks": 240,
                   "query_refresh_ticks": 24, "ordinary_default_changed": False, "physical_guards_changed": False}}
        return attempt, feedback

    def test_query_proof_rejects_origin_clock_and_false_acceptance(self):
        attempt, feedback = self.evidence()
        sweep.validate_phase_transition(attempt, feedback)
        for key, value in (("origin", {"physics_step": 98}), ("checked_ticks", 119),
                           ("end_state", None), ("prediction_ticks", 241)):
            bad = copy.deepcopy(feedback)
            bad["refreshes"][0]["transition_query"][key] = value
            with self.assertRaises(ValueError):
                sweep.validate_phase_transition(attempt, bad)

    def test_old_identity_cannot_smuggle_new_behavior(self):
        attempt, feedback = self.evidence()
        feedback["candidate_id"] = sweep.FINITE_CANDIDATES["finite-correction"]
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, feedback)

    def test_common_horizon_cannot_use_choice_specific_short_window(self):
        attempt, feedback = self.evidence()
        origin = feedback["refreshes"][0]["origin"]
        queued = copy.deepcopy(feedback["refreshes"][0]["transition_query"])
        queued["kind"] = "queued_turn_burn_coast"
        common = {"origin": origin, "requested_command": {"throttle_frac": 0, "target_attitude_rad": 0},
                  "prediction_ticks": 120, "queued_program": queued,
                  "commands": [{"choice": "upright_lift", "command": {"throttle_frac": 1, "target_attitude_rad": 0},
                                "prediction_ticks": 120, "checked": True, "conflict": None}]}
        feedback["refreshes"][0]["recovery_common_query"] = common
        sweep.validate_phase_transition(attempt, feedback)
        common["commands"][0]["prediction_ticks"] = 24
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, feedback)


if __name__ == "__main__":
    unittest.main()
