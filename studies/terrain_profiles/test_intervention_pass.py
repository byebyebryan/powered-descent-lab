import copy
import unittest

import intervention_pass as timing


class TimingPassTests(unittest.TestCase):
    def test_frozen_allowance_is_16_plus_16_plus_100_and_two_repeats(self):
        plan = timing.contract()
        self.assertEqual(len(plan["focus_indices"]), 16)
        self.assertEqual(plan["repeat_indices"], [35, 50])
        self.assertEqual(len(set(plan["focus_indices"])), 16)
        self.assertTrue(set(plan["successful_focus_controls"]) <= set(plan["focus_indices"]))

    def test_motion_excludes_only_additive_diagnostics_and_wall_timings(self):
        original = {"timings": {"planning_s": 1}, "cycles": [{"local_search": {"selected": {"identity": "fixed"}}}], "segments": [1]}
        instrumented = copy.deepcopy(original)
        instrumented["timings"]["planning_s"] = 2
        instrumented["cycles"][0]["local_search"]["row_diagnostics"] = [{"stop_reason": "reserve"}]
        self.assertEqual(timing.motion(original), timing.motion(instrumented))
        self.assertIn("row_diagnostics", instrumented["cycles"][0]["local_search"])
        instrumented["cycles"][0]["local_search"]["selected"]["identity"] = "changed"
        self.assertNotEqual(timing.motion(original), timing.motion(instrumented))


if __name__ == "__main__":
    unittest.main()
