import copy
import unittest

import ballistic_finite_correction_panel as panel
import ballistic_feedback_sweep as sweep


class FiniteCorrectionTests(unittest.TestCase):
    def test_fixed_worlds_and_separate_repeats(self):
        self.assertEqual([len(x) for x in panel.layout("probe")], [17, 3])
        self.assertEqual([len(x) for x in panel.layout("focus")], [26, 3])
        self.assertEqual([len(x) for x in panel.layout("full")], [1000, 5])
        self.assertEqual(len(set(panel.FOCUS)), 26)
        losses = {142, 236, 262, 357, 392, 565, 570, 715, 814, 862, 971}
        self.assertTrue(losses.issubset(panel.FOCUS))

    def evidence(self, mode="finite-correction"):
        metadata = {"queries_recorded": True, "admission_enabled": mode == "finite-correction",
                    "powered_prefix_plant_checked": True, "optional_refresh_ticks": 24,
                    "initial_nominal_changed": False, "waypoint_ranking_changed": False,
                    "physical_guards_changed": False}
        origin = {"physics_step": 100}
        correction = {"burn_end_physics_step": 200}
        arc = {"target_m": {"x": 1200, "y": 5}}
        acquisition = {"origin": origin, "checked_powered_ticks": 100,
                       "cutoff": {"physics_step": 200}, "coast_miss_m": 0.2,
                       "conflict": None, "domain_stop": None, "coast_domain_error": None,
                       "rejection": None}
        query = {"desired_arc": arc, "correction": correction, "acquisition": acquisition,
                 "rejection": None}
        refresh = {"origin": origin, "desired_arc": arc, "correction": correction,
                   "decision": "finite_destination_query", "goal": {"destination": True},
                   "previous_goal": {"destination": False}, "finite_destination_query": query}
        return {"finite_correction": metadata}, {"candidate_id": sweep.FINITE_CANDIDATES[mode],
                                                "refreshes": [refresh]}

    def test_query_binding_and_complete_acceptance(self):
        for mode in sweep.FINITE_CANDIDATES:
            attempt, feedback = self.evidence(mode)
            sweep.validate_finite_correction(attempt, feedback)
        for field, value in [("checked_powered_ticks", 98), ("coast_miss_m", None),
                             ("conflict", {}), ("domain_stop", {})]:
            attempt, feedback = self.evidence()
            query = feedback["refreshes"][0]["finite_destination_query"]
            query["acquisition"][field] = value
            with self.assertRaises(ValueError):
                sweep.validate_finite_correction(attempt, feedback)
        attempt, feedback = self.evidence()
        feedback["refreshes"][0]["finite_destination_query"]["desired_arc"] = copy.deepcopy({})
        with self.assertRaises(ValueError):
            sweep.validate_finite_correction(attempt, feedback)

    def test_old_identity_cannot_smuggle_finite_admission(self):
        attempt, feedback = self.evidence()
        feedback["candidate_id"] = sweep.MECHANICS_CANDIDATES["exit-consistency"]
        with self.assertRaises(ValueError):
            sweep.validate_finite_correction(attempt, feedback)
        attempt, feedback = self.evidence()
        attempt["finite_correction"]["physical_guards_changed"] = True
        with self.assertRaises(ValueError):
            sweep.validate_finite_correction(attempt, feedback)

    def test_finite_identity_retains_inherited_contracts(self):
        for candidate in sweep.FINITE_CANDIDATES.values():
            with self.assertRaises(ValueError):
                sweep.validate_terminal_coordination({"terminal_coordination": {"enabled": False}},
                                                     {"candidate_id": candidate})

    def test_powered_obstruction_requires_a_same_state_native_query_binding(self):
        origin = {"physics_step": 100}
        conflict = {"cause": "short_command_reserve", "state": {"physics_step": 300}}
        goal = {"destination": True, "revision": 1}
        obstruction = {"decision": "ballistic_obstruction", "origin": origin, "goal": goal,
                       "predicted_conflict": conflict, "finite_destination_query": {
                           "acquisition": {"origin": origin, "conflict": conflict,
                                           "rejection": "powered_short_guard"}}}
        selected = {"decision": "waypoint_selected", "origin": origin,
                    "goal": {"destination": False, "revision": 2}, "replan_trigger": conflict}
        feedback = {"candidate_id": sweep.FINITE_CANDIDATES["finite-correction"],
                    "refreshes": [obstruction, selected], "updates": [], "handoffs": [],
                    "handoff_goal_revisions": []}
        sweep.validate_terrain_corrections(feedback)
        bad = copy.deepcopy(feedback)
        bad["refreshes"][0]["finite_destination_query"]["acquisition"]["origin"] = {"physics_step": 98}
        with self.assertRaises(ValueError):
            sweep.validate_terrain_corrections(bad)
        bad = copy.deepcopy(feedback)
        bad["candidate_id"] = sweep.MECHANICS_CANDIDATES["exit-consistency"]
        with self.assertRaises(ValueError):
            sweep.validate_terrain_corrections(bad)
        bad = copy.deepcopy(feedback)
        bad["refreshes"] = [selected]
        with self.assertRaises(ValueError):
            sweep.validate_terrain_corrections(bad)


if __name__ == "__main__":
    unittest.main()
