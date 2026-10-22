import unittest

from run_stopping import jobs, summarize


class SelectionTests(unittest.TestCase):
    def rows(self, count, wins):
        result = []
        for job in jobs(count):
            success = job["arc"] < wins.get((job["seed"], job["rule"]), 0)
            result.append({**job, "native_success": success})
        return result

    def test_repeat_count_does_not_overweight_sampling_and_ties_prefer_g(self):
        wins = {(seed, rule): 2 for seed in [281101, 281102, 281103] for rule in ["G", "M", "S"]}
        result = summarize(self.rows(4, wins), 4)
        self.assertEqual(result["aggregate_rates"], {"G": .5, "M": .5, "S": .5})
        self.assertEqual(result["winner"], "G")
        self.assertFalse(result["nomination_passed"])

    def test_aggregate_improvement_cannot_hide_primary_regression(self):
        wins = {(seed, rule): 1 for seed in [281101, 281102, 281103] for rule in ["G", "M", "S"]}
        wins[(281101, "M")], wins[(281102, "M")], wins[(281103, "M")] = 4, 4, 0
        result = summarize(self.rows(4, wins), 4)
        self.assertEqual(result["winner"], "M")
        self.assertFalse(result["nomination_passed"])

    def test_nomination_requires_complete_inventory(self):
        wins = {(seed, "M"): 4 for seed in [281101, 281102, 281103]}
        rows = self.rows(4, wins)
        self.assertTrue(summarize(rows, 4)["nomination_passed"])
        with self.assertRaises(RuntimeError):
            summarize(rows[1:], 4)


if __name__ == "__main__":
    unittest.main()
