import unittest

import waypoint_entry_panel as panel


class EntryPanelTests(unittest.TestCase):
    def test_focused_denominator_and_external_repeat_are_fixed(self):
        primary, repeats = panel.layout()
        self.assertEqual(primary[:4], ["random-349", "random-006", "random-807", "random-928"])
        self.assertEqual(len(primary), 7)
        self.assertEqual(repeats, ["random-349"])

    def test_combined_preservation_denominator_is_separate(self):
        primary, repeats = panel.layout(True)
        self.assertEqual(len(primary), 19)
        self.assertEqual(len(set(primary)), 19)
        self.assertEqual(repeats, ["random-349", "random-807"])

    def test_ablations_have_distinct_identities_and_keep_ridge_control(self):
        self.assertEqual(list(panel.MODES), ["ridge", "effort", "recovery", "combined"])
        self.assertEqual(len(set(panel.MODES.values())), 4)
        self.assertEqual(panel.MODES["ridge"], "ballistic_feedback_v5_ridge_waypoint")


if __name__ == "__main__":
    unittest.main()
