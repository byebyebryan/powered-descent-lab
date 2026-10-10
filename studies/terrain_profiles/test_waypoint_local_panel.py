import unittest

import waypoint_local_panel as panel
import waypoint_entry_panel as entry


class LocalPanelTests(unittest.TestCase):
    def test_frozen_focused_inventory_and_repeat(self):
        primary, repeats = panel.layout()
        self.assertEqual(primary[:7], ["random-715", "random-349", "random-006", "random-807",
                                      "random-928", "random-139", "random-268"])
        self.assertEqual(len(primary), 10)
        self.assertEqual(repeats, ["random-715"])

    def test_preservation_uses_existing_worlds_with_separate_repeats(self):
        primary, repeats = panel.layout(True)
        self.assertEqual(len(primary), 19)
        self.assertEqual(len(set(primary)), 19)
        self.assertEqual(repeats, ["random-715", "random-349"])
        self.assertEqual(4 * sum(map(len, panel.layout())) + sum(map(len, panel.layout(True))), 65)

    def test_explicit_v6_control_and_new_modes_do_not_change_previous_contract(self):
        self.assertEqual(list(panel.MODES), ["combined", "local-height", "early-target", "local-height-early-target"])
        self.assertEqual(panel.MODES["combined"], "ballistic_feedback_v6_waypoint_combined")
        self.assertEqual(entry.SPEC.control_mode, "ridge")
        self.assertNotEqual(panel.SPEC.native, entry.SPEC.native)


if __name__ == "__main__":
    unittest.main()
