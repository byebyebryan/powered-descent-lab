import copy
import unittest

import coast_terminal_panel as panel
import ballistic_feedback_sweep as sweep


class CoastTerminalPanelTests(unittest.TestCase):
    def test_inventory_and_budget_remain_small_and_separate(self):
        primary, repeats = panel.layout()
        self.assertEqual(primary[:6], ["random-715", "random-084", "random-142", "random-268", "random-349", "random-006"])
        self.assertEqual(len(primary), 9)
        self.assertEqual(repeats, ["random-715", "random-084"])
        self.assertEqual(2 * (len(primary) + len(repeats)), 22)
        with self.assertRaises(ValueError):
            panel.layout(True)
        self.assertEqual(panel.SPEC.control_mode, "landing-countdown")
        self.assertEqual(panel.SPEC.panel_mode, "coast-terminal")

    def test_metadata_cannot_disable_terrain_or_borrow_saved_clocks(self):
        feedback = {"candidate_id": sweep.COAST_CANDIDATE, "refreshes": [], "updates": []}
        sweep.validate_coast_terminal({"coast_terminal": sweep.COAST_METADATA}, feedback)
        for key in ["configured_terminal_terrain_enabled", "saved_clock_used", "complete_landing_suffix_required"]:
            invalid = copy.deepcopy(sweep.COAST_METADATA)
            invalid[key] = not invalid[key]
            with self.assertRaises(ValueError):
                sweep.validate_coast_terminal({"coast_terminal": invalid}, feedback)
        with self.assertRaises(ValueError):
            sweep.validate_coast_terminal({"coast_terminal": sweep.COAST_METADATA},
                                          {**feedback, "candidate_id": sweep.COUNTDOWN_CANDIDATES["landing-countdown"]})

    def test_orphaned_coast_commands_are_not_bound_proof(self):
        feedback = {"candidate_id": sweep.COAST_CANDIDATE, "refreshes": [],
                    "updates": [{"physics_step": 24, "phase": "coast_through_to_terminal"}]}
        with self.assertRaises(ValueError):
            sweep.validate_coast_terminal({"coast_terminal": sweep.COAST_METADATA}, feedback)


if __name__ == "__main__":
    unittest.main()
