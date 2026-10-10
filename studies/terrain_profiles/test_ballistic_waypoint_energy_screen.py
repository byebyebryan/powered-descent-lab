"""Pure mathematical tests, independent of retained captures and native flights."""
import unittest

import ballistic_waypoint_energy_screen as screen


class EnergyScreenTests(unittest.TestCase):
    def inputs(self):
        state = {"position_m": {"x": 0, "y": 100}, "velocity_mps": {"x": 60, "y": 30},
                 "physics_step": 0, "fuel_kg": 6300, "attitude_rad": 0}
        scenario = {"vehicle": {"dry_mass_kg": 7200, "max_thrust_n": 240000,
                                "max_rotation_rate_radps": 1.5707963267948966,
                                "max_fuel_burn_kgps": 49.5, "min_throttle_frac": .25},
                    "sim": {"physics_hz": 120, "max_time_s": 90},
                    "world": {"gravity_mps2": 9.81, "landing_pads": [{"id": "pad_main", "center_x_m": 1200}]}}
        return state, {"x": 1100, "y": 120}, scenario

    def test_bounded_profiles_preserve_exact_geometric_endpoint(self):
        state, goal, scenario = self.inputs()
        candidates = screen.fits(state, goal, scenario)
        self.assertTrue(candidates)
        self.assertLessEqual(len(candidates), 32)
        for candidate in candidates:
            self.assertEqual(candidate["ticks"] % 2, 0)
            self.assertAlmostEqual(candidate["arrival_position"][0], goal["x"], places=8)
            self.assertAlmostEqual(candidate["arrival_position"][1], goal["y"], places=8)
        self.assertEqual(candidates, sorted(candidates, key=lambda c: (c["effort"], c["speed"], c["ticks"])))

    def test_room_is_a_heuristic_with_unsupported_fallback(self):
        self.assertIsNone(screen.room(100, 20, 9, 9.81, 1, 1 / 120))
        self.assertGreater(screen.room(100, 0, 20, 9.81, 1, 1 / 120), 0)
        self.assertLess(screen.room(10, 80, 20, 9.81, 1, 1 / 120), 0)

    def test_backward_empty_fuel_or_passed_goal_is_not_screened(self):
        for field, value in (("fuel_kg", 0), ("velocity_mps", {"x": 0, "y": 30}),
                             ("position_m", {"x": 1200, "y": 100})):
            state, goal, scenario = self.inputs()
            state[field] = value
            with self.assertRaises(ValueError):
                screen.fits(state, goal, scenario)


if __name__ == "__main__":
    unittest.main()
