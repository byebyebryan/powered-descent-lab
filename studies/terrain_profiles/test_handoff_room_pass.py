import copy
import math
import unittest

import handoff_room_pass as room
import intervention_pass as timing


class HandoffRoomTests(unittest.TestCase):
    def test_diagnostic_uses_actual_query_state_and_current_scenario_shape(self):
        state = {"physics_step": 200, "position_m": {"x": 0.0},
                 "velocity_mps": {"x": 20.0}, "attitude_rad": 0.0, "fuel_kg": 0.0}
        turn = math.ceil((math.pi / 3) * 60) / 60
        needed = 20 * turn + 400 / (2 * math.sqrt(300))
        estimate = {"available_acceleration_mps2": 20.0,
                    "horizontal_braking_acceleration_mps2": math.sqrt(300),
                    "turn_time_s": turn, "required_distance_m": needed,
                    "remaining_room_m": 100 - needed}
        row = {"row_id": "primary_00", "entry_physics_step": 100,
               "eligible_handoff": {"state": state, "braking_room": estimate,
                                    "actual_fuel_burn_to_handoff_kg": 1.0}}
        search = {"entries": [{"physics_step": 100, "entry_id": "primary"}],
                  "row_count": 42, "accepted_row_count": 1,
                  "row_diagnostics": [row] + [{"row_id": f"primary_{i:02}"} for i in range(1, 42)],
                  "selected": {"row_id": row["row_id"], "handoff_state": state}}
        scenario = {"world": {"gravity_mps2": 10.0, "landing_pads": [{"id": "target", "center_x_m": 100.0}]},
                    "mission": {"goal": {"target_pad_id": "target"}}, "sim": {"physics_hz": 120},
                    "vehicle": {"max_thrust_n": 200.0 / 0.925, "dry_mass_kg": 10.0, "max_rotation_rate_radps": 1.0}}
        self.assertEqual(room.check_local(search, scenario, False)["original_room_m"], 100 - needed)
        search["selected"]["handoff_state"] = dict(state, physics_step=202)
        with self.assertRaisesRegex(ValueError, "selection"):
            room.check_local(search, scenario, False)

    def test_frozen_allowance_and_phase_populations(self):
        plan = room.contract()
        self.assertEqual(plan["maximum_measured_attempts"], 8 + 17 + 44 + 100 + 2)
        self.assertEqual(len(timing.phase_indices(plan, "diagnostics")), 8)
        self.assertEqual(len(timing.phase_indices(plan, "focus")), 17)
        self.assertEqual(len(timing.phase_indices(plan, "challenge")), 100)
        self.assertEqual(plan["repeat_indices"], [28, 50])
        self.assertEqual(timing.phase_indices(timing.contract(), "diagnostics"), timing.contract()["focus_indices"])

    def test_choice_retains_original_rank_within_nonnegative_subset(self):
        def row(name, tick, handoff_tick, value):
            return {"row_id": name, "entry_physics_step": tick, "eligible_handoff": {
                "state": {"physics_step": handoff_tick}, "actual_fuel_burn_to_handoff_kg": 1.0,
                "braking_room": None if value is None else {"remaining_room_m": value}}}
        original = row("original", 100, 200, -1)
        first = row("first", 100, 220, 0)
        second = row("second", 80, 180, 200)
        search = {"row_diagnostics": [second, original, first], "accepted_row_count": 3}
        before = copy.deepcopy(search)
        self.assertEqual(room.local_choice(search), (original, first))
        self.assertEqual(search, before)
        for value in (0, 1, None):
            original["eligible_handoff"]["braking_room"] = None if value is None else {"remaining_room_m": value}
            self.assertEqual(room.local_choice(search), (original, original))
        original["eligible_handoff"]["braking_room"] = {"remaining_room_m": -1}
        first["eligible_handoff"]["braking_room"] = {"remaining_room_m": -2}
        second["eligible_handoff"]["braking_room"] = {"remaining_room_m": -3}
        self.assertEqual(room.local_choice(search), (original, original))
        with self.assertRaisesRegex(ValueError, "inventory"):
            room.local_choice(dict(search, accepted_row_count=2))


if __name__ == "__main__":
    unittest.main()
