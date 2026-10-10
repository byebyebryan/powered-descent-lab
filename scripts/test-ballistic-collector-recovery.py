"""Output-free tests of same-clock handoff/recovery ordering in the reader."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "studies/terrain_profiles"))
import ballistic_feedback_sweep as sweep
from test_ballistic_feedback_sweep import terrain_feedback


class RecoveryOrderingTests(unittest.TestCase):
    def test_previous_handoff_can_precede_next_goal_recovery_at_the_same_tick(self):
        value = terrain_feedback()
        previous = {"number": 1, "revision": 1, "destination": False, "position_m": {"x": 10, "y": 100}}
        current = dict(previous, number=2, revision=2, position_m={"x": 20, "y": 120})
        old = dict(value["refreshes"][0], decision="waypoint_selected", goal=previous,
                   replan_trigger={"cause": "ideal_arc_reserve"})
        new = dict(old, goal=current, origin={"physics_step": 100})
        for refresh in value["refreshes"][1:]:
            refresh["goal"] = copy.deepcopy(current)
        value["refreshes"] = [old, new] + value["refreshes"][1:]
        value["handoffs"] = [{"physics_step": 100}]
        value["handoff_goal_revisions"] = [1]
        self.assertEqual(sweep.projection(value, sweep.TERRAIN_CANDIDATE)["handoffs"], 1)
        value["handoffs"][0]["physics_step"] = 150
        with self.assertRaises(ValueError):
            sweep.projection(value, sweep.TERRAIN_CANDIDATE)

    def test_handoff_of_the_recovering_goal_at_start_is_rejected(self):
        value = terrain_feedback()
        value["handoffs"] = [{"physics_step": 100}]
        value["handoff_goal_revisions"] = [0]
        with self.assertRaises(ValueError):
            sweep.validate_terrain_corrections(value)


if __name__ == "__main__":
    unittest.main()
