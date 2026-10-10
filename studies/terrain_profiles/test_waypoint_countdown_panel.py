import unittest

import waypoint_countdown_panel as panel
import waypoint_landing_panel as landing


class CountdownPanelTests(unittest.TestCase):
    def test_focused_inventory_includes_new_success_and_two_repeats(self):
        primary, repeats = panel.layout()
        self.assertEqual(primary[:6], ["random-715", "random-084", "random-142", "random-268", "random-349", "random-006"])
        self.assertEqual(len(primary), 9)
        self.assertEqual(repeats, ["random-715", "random-084"])

    def test_fixed_allowance_and_explicit_previous_candidate(self):
        self.assertEqual(2 * sum(map(len, panel.layout())) + sum(map(len, panel.layout(True)))
                         + sum(map(len, panel.EARLY_SPEC.layout())), 48)
        self.assertEqual(panel.SPEC.control_mode, "landing-duration")
        self.assertEqual(panel.MODES["landing-countdown"], "ballistic_feedback_v9_landing_countdown")
        self.assertNotEqual(panel.NATIVE, landing.NATIVE)
        self.assertEqual(len(landing.MODES), 2)


if __name__ == "__main__":
    unittest.main()
