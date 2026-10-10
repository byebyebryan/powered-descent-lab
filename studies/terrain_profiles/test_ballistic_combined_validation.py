import copy
import unittest

import ballistic_combined_validation as validation
import ballistic_feedback_sweep as sweep
import test_ballistic_recovery_consistency_panel as recovery_tests


class CombinedValidationTests(unittest.TestCase):
    def test_frozen_attempt_allowance_and_repeats(self):
        counts = [sum(map(len, validation.layout(stage))) for stage in validation.STAGES]
        self.assertEqual(counts, [112, 1008, 1005, 1005])
        self.assertEqual(sum(counts), 3130)
        self.assertTrue(set(validation.REPEATS).issubset(validation.FOCUS))
        self.assertEqual(validation.layout("heldout-acquisition"), validation.layout("heldout-combined"))

    def test_fresh_seeds_are_deterministic_and_disjoint(self):
        seeds = validation.seeds()
        self.assertEqual(len(set(seeds)), 1000)
        self.assertEqual(seeds, validation.seeds())
        self.assertFalse(set(seeds) & validation.excluded())
        self.assertEqual(validation.input_plan()["master_seed"], 2026101001)
        self.assertEqual(len(validation.input_plan()["recipes"]), 4)

    def evidence(self):
        attempt, feedback = recovery_tests.RecoveryConsistencyTests().evidence()
        feedback["candidate_id"] = sweep.COMBINED_SAFETY_CANDIDATES["acquisition-terminal-safety"]
        attempt["acquisition_and_terminal_safety"] = {
            "open_destination_acquisition": True, "terminal_safety_fallback": True,
            "fallback_choices": ["upright_coast", "upright_support"], "query_refresh_ticks": 24,
            "physical_guards_changed": False, "standalone_coast_terminal_changed": False}
        return attempt, feedback

    def test_combined_flags_require_explicit_new_identity(self):
        attempt, feedback = self.evidence()
        sweep.validate_phase_transition(attempt, feedback)
        for identity in sweep.SAFETY_CANDIDATES.values():
            changed = copy.deepcopy(feedback)
            changed["candidate_id"] = identity
            with self.assertRaises(ValueError):
                sweep.validate_phase_transition(attempt, changed)
        for key in ("open_destination_acquisition", "terminal_safety_fallback"):
            changed = copy.deepcopy(attempt)
            changed["acquisition_and_terminal_safety"][key] = False
            with self.assertRaises(ValueError):
                sweep.validate_phase_transition(changed, feedback)

    def test_combined_preserves_common_recovery_horizon(self):
        attempt, feedback = self.evidence()
        feedback["refreshes"][0]["recovery_common_query"]["commands"][0]["prediction_ticks"] = 24
        with self.assertRaises(ValueError):
            sweep.validate_phase_transition(attempt, feedback)

    def test_terminal_prefix_comparison_is_exact(self):
        prior = {"updates": [{"physics_step": 12, "phase": "maintained_terminal"}],
                 "ordinary_flight": {"actions": [{"physics_step": 0, "command": 1},
                                                   {"physics_step": 12, "command": 2}]}}
        current = copy.deepcopy(prior)
        current["ordinary_flight"]["actions"][1]["command"] = 3
        validation.preterminal_equal(current, prior)
        current["ordinary_flight"]["actions"][0]["command"] = 3
        with self.assertRaises(ValueError):
            validation.preterminal_equal(current, prior)
        current = copy.deepcopy(prior)
        current["updates"][0]["physics_step"] = 14
        with self.assertRaises(ValueError):
            validation.preterminal_equal(current, prior)

    def test_never_terminal_flight_cannot_change(self):
        prior = {"updates": [], "ordinary_flight": {"actions": [], "fuel": 1}}
        current = copy.deepcopy(prior)
        validation.preterminal_equal(current, prior)
        current["ordinary_flight"]["fuel"] = 0
        with self.assertRaises(ValueError):
            validation.preterminal_equal(current, prior)

    def rows(self, paired=False):
        rows = []
        for i in range(2):
            result = {"verified_landing": True, "handoffs": i, "stop_group": "physical_terminal",
                      "physical_outcome": "landed", "initial_candidate_arc": "clear"}
            baseline = dict(result, verified_landing=bool(i)) if paired else None
            rows.append({"case_id": f"random-{i:03}", "cohort": "random", "status": "recorded",
                         "result": result, "baseline_result": baseline,
                         "geometry": {"recipe_id": "mountains_4x"}})
        return rows

    def test_unpaired_fresh_metrics_do_not_invent_prior_failures(self):
        result = validation.metrics(self.rows())
        self.assertIsNone(result["baseline_landings"])
        self.assertNotIn("gained_landings", result)
        self.assertEqual(result["verified_landings"], 2)
        self.assertEqual(result["primary_count"], 2)

    def test_fresh_comparison_uses_genuine_ballistic_results(self):
        result = validation.metrics(self.rows(True))
        self.assertEqual(result["baseline_landings"], 1)
        self.assertEqual(result["gained_landings"], ["random-000"])
        self.assertEqual(result["lost_landings"], [])


if __name__ == "__main__":
    unittest.main()
