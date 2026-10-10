import copy
import unittest

import ballistic_acquisition_safety_panel as panel
import ballistic_feedback_sweep as sweep
import test_ballistic_recovery_consistency_panel as recovery_tests


class AcquisitionSafetyTests(unittest.TestCase):
    def test_fixed_independent_layouts_and_reference(self):
        self.assertEqual(len(panel.MISSING_AIM), 23)
        self.assertEqual(len(panel.WAYPOINT_MISS), 12)
        self.assertEqual(len(panel.TERMINAL), 23)
        self.assertEqual(len(panel.FOCUS), len(set(panel.FOCUS)))
        self.assertTrue(set(panel.REPEATS).issubset(panel.FOCUS))
        self.assertEqual(panel.STAGES["acquisition-full"], "acquisition-gate")
        self.assertEqual(panel.STAGES["terminal-full"], "terminal-safety-fallback")
        self.assertEqual([len(x) for x in panel.layout("acquisition-full")], [1000, 5])
        self.assertEqual(panel.layout("acquisition-full"), panel.layout("terminal-full"))
        self.assertEqual(set(panel.STAGES.values()), set(panel.MODES))

    def evidence(self, mode):
        attempt, feedback = recovery_tests.RecoveryConsistencyTests().evidence()
        feedback["candidate_id"] = sweep.SAFETY_CANDIDATES[mode]
        attempt["acquisition_and_terminal_safety"] = {
            "open_destination_acquisition": mode == "acquisition-gate",
            "terminal_safety_fallback": mode == "terminal-safety-fallback",
            "fallback_choices": ["upright_coast", "upright_support"],
            "query_refresh_ticks": 24, "physical_guards_changed": False,
            "standalone_coast_terminal_changed": False}
        return attempt, feedback

    def test_flags_cannot_enable_both_changes_or_relax_guards(self):
        for mode in sweep.SAFETY_CANDIDATES:
            attempt, feedback = self.evidence(mode)
            sweep.validate_phase_transition(attempt, feedback)
            for key in ("open_destination_acquisition", "terminal_safety_fallback", "physical_guards_changed"):
                bad = copy.deepcopy(attempt)
                bad["acquisition_and_terminal_safety"][key] = not bad["acquisition_and_terminal_safety"][key]
                with self.assertRaises(ValueError):
                    sweep.validate_phase_transition(bad, feedback)

    def test_new_candidates_retain_common_recovery_horizon(self):
        for mode in sweep.SAFETY_CANDIDATES:
            attempt, feedback = self.evidence(mode)
            feedback["refreshes"][0]["recovery_common_query"]["commands"][0]["prediction_ticks"] = 24
            with self.assertRaises(ValueError):
                sweep.validate_phase_transition(attempt, feedback)

    def test_terminal_fallback_is_identity_and_command_bound(self):
        attempt, feedback = self.evidence("terminal-safety-fallback")
        frame = {"command": {"throttle_frac": 0, "target_attitude_rad": 0},
                 "status": "terminal reserve safety fallback: upright_coast",
                 "metrics": {"guidance.terminal_safety_fallback": "upright_coast",
                             "guidance.terminal_safety_original_throttle": 0.2,
                             "guidance.terminal_safety_original_attitude": 0.15}}
        feedback["controller_updates"] = [{"frame": frame}]
        sweep.validate_safety_options(attempt, feedback)
        for command in ({"throttle_frac": 1, "target_attitude_rad": 0},
                        {"throttle_frac": 0, "target_attitude_rad": 0.1}):
            bad = copy.deepcopy(feedback)
            bad["controller_updates"][0]["frame"]["command"] = command
            with self.assertRaises(ValueError):
                sweep.validate_safety_options(attempt, bad)
        attempt, other = self.evidence("acquisition-gate")
        other["controller_updates"] = feedback["controller_updates"]
        with self.assertRaises(ValueError):
            sweep.validate_safety_options(attempt, other)

    def test_positive_room_goal_change_requires_native_admission(self):
        origin = {"physics_step": 100, "held_command": {"throttle_frac": 0}}
        goal = {"destination": True, "revision": 2, "position_m": {"x": 1200, "y": 5}}
        arc = {"target_m": goal["position_m"]}
        correction = {"burn_end_physics_step": 200}
        query = {"rejection": None, "acquisition": {"rejection": None}}
        preview = {"decision": "finite_destination_query", "origin": origin,
                   "desired_arc": arc, "correction": correction, "finite_destination_query": query}
        selected = {"decision": "destination_reacquired_before_waypoint", "origin": origin,
                    "goal": goal, "previous_goal": {"destination": False, "revision": 1},
                    "desired_arc": arc, "correction": correction,
                    "waypoint_braking_room": {"remaining_room_m": 100, "required_distance_m": 50}}
        feedback = {"candidate_id": sweep.SAFETY_CANDIDATES["acquisition-gate"],
                    "refreshes": [preview, selected], "handoffs": []}
        sweep.validate_local_waypoints(feedback)
        for mutation in ("no_query", "wrong_arc", "rejected"):
            bad = copy.deepcopy(feedback)
            if mutation == "no_query":
                bad["refreshes"].pop(0)
            elif mutation == "wrong_arc":
                bad["refreshes"][0]["desired_arc"] = {}
            else:
                bad["refreshes"][0]["finite_destination_query"]["rejection"] = "powered_short_guard"
            with self.assertRaises(ValueError):
                sweep.validate_local_waypoints(bad)
        feedback["candidate_id"] = sweep.RECOVERY_CANDIDATES["recovery-consistency"]
        with self.assertRaises(ValueError):
            sweep.validate_local_waypoints(feedback)


if __name__ == "__main__":
    unittest.main()
