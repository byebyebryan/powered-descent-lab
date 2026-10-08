import copy
import unittest

import fallback_pass as fallback


class FallbackPassTests(unittest.TestCase):
    def test_frozen_population_and_allowance(self):
        plan = fallback.contract()
        self.assertEqual(len(plan["focus_indices"]), 17)
        self.assertIn(81, plan["focus_indices"])
        self.assertEqual(plan["maximum_measured_attempts"], 17 + 44 + 100 + 2)
        self.assertEqual(plan["repeat_indices"], [35, 50])

    def test_query_metadata_is_removed_only_from_comparison_copy(self):
        flight = {"cycles": [{"local_search": {"row_diagnostics": [1], "selected": {"identity": "bound"}}}],
                  "timings": {"planning_s": 1, "execution_s": 2, "replay_s": 3}, "segments": [1]}
        before = copy.deepcopy(flight)
        result = fallback.without_query_diagnostics(flight)
        self.assertEqual(flight, before)
        self.assertEqual(result["timings"], flight["timings"])
        self.assertEqual(result["segments"], flight["segments"])
        self.assertEqual(result["cycles"][0]["local_search"], {"selected": {"identity": "bound"}})

    def test_preservation_rejects_changed_success_and_allows_failed_route_to_improve(self):
        old = {"planning_stop": "landed", "cycles": [], "segments": [1]}
        fallback.preservation(dict(old, timings={"planning_s": 10}), old)
        with self.assertRaisesRegex(ValueError, "previously landed"):
            fallback.preservation(dict(old, segments=[2]), old)
        fallback.preservation(old, dict(old, planning_stop="no_clearing"))


if __name__ == "__main__":
    unittest.main()
