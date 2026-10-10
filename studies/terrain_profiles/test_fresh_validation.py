import copy
import unittest

import fresh_validation as fresh
import survey


class FreshValidationTests(unittest.TestCase):
    def test_population_is_new_and_repeatable(self):
        plan = fresh.contract()
        selected = fresh.seeds(plan)
        self.assertEqual(len(selected), 100)
        self.assertEqual(len(set(selected)), 100)
        self.assertFalse(set(selected) & fresh.excluded_seeds())
        self.assertEqual(selected, fresh.seeds(plan))
        self.assertEqual(len(fresh.excluded_seeds()), 1230)

    def test_attempts_have_separate_denominators(self):
        plan = fresh.contract()
        cases = [dict(case_id=cid, cohort="sentinel") for cid in plan["sentinels"]]
        cases += [dict(case_id=f"random-{i:03}", cohort="random") for i in range(100)]
        rows = fresh.attempts(cases, plan)
        self.assertEqual(len(rows), 108)
        self.assertEqual([r["case_id"] for r in rows[103:]], [f"random-{i:03}" for i in plan["repeat_indices"]])
        self.assertEqual(sum(r["cohort"] == "random" for r in rows), 100)
        self.assertEqual(survey.summary_for(rows)["primary_count"], 100)

    def test_preservation_has_no_numerical_or_proof_exceptions(self):
        original = {"policy": {"policy_id": "retained"}, "input_identity": "bound",
                    "timings": {"planning_s": 1.0, "execution_s": 0.0, "replay_s": 0.0},
                    "ordinary_flight": {"final_state": {"fuel_kg": 42.0}},
                    "proof": "same"}
        candidate = copy.deepcopy(original)
        candidate["timings"]["planning_s"] = 2.0
        fresh.compare(candidate, original)
        candidate["proof"] = "changed"
        with self.assertRaises(ValueError):
            fresh.compare(candidate, original)
        candidate = copy.deepcopy(original)
        candidate["ordinary_flight"]["final_state"]["fuel_kg"] += 1e-12
        with self.assertRaises(ValueError):
            fresh.compare(candidate, original)


if __name__ == "__main__":
    unittest.main()
