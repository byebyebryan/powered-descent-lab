import unittest

import waypoint_landing_panel as panel
import waypoint_local_panel as local


class LandingPanelTests(unittest.TestCase):
    def test_focused_original_failures_and_successes(self):
        primary, repeats = panel.layout()
        self.assertEqual(primary[:5], ["random-715", "random-084", "random-142", "random-349", "random-006"])
        self.assertEqual(len(primary), 8)
        self.assertEqual(repeats, ["random-715"])

    def test_closed_allowance_preserves_denominators(self):
        self.assertEqual(sum(map(len, panel.layout(True))), 21)
        self.assertEqual(sum(map(len, panel.early_layout())), 5)
        self.assertEqual(2 * sum(map(len, panel.layout())) + sum(map(len, panel.layout(True)))
                         + sum(map(len, panel.early_layout())), 44)
        with self.assertRaises(ValueError):
            panel.early_layout(True)

    def test_explicit_identities_keep_prior_experiment_unchanged(self):
        self.assertEqual(panel.SPEC.control_mode, "local-height-early-target")
        self.assertEqual(panel.MODES["landing-duration"], "ballistic_feedback_v8_landing_duration")
        self.assertNotEqual(panel.NATIVE, local.NATIVE)
        self.assertEqual(len(local.MODES), 4)


if __name__ == "__main__":
    unittest.main()
