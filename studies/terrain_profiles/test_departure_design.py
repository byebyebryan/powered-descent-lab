import math
import random
import unittest

import departure_design as design


class DepartureDesignTests(unittest.TestCase):
    def test_travel_time_and_unsupported_inputs(self):
        for distance, vx, ax in [(30, 0.3, 6.66), (4, 2, 0), (0, 1, 2)]:
            t = design.travel_time(distance, vx, ax)
            self.assertAlmostEqual(vx * t + 0.5 * ax * t * t, distance)
        for args in [(1, 0, 1), (1, 1, -1), (math.nan, 1, 1)]:
            with self.assertRaises(ValueError):
                design.travel_time(*args)

    def test_heightfield_knots_and_domain(self):
        points = [(-5, 0), (0, 10), (5, 0)]
        self.assertEqual(design.height_max(points, -2, 2), 10)
        self.assertEqual(design.height_max(points, -4, -2), 6)
        with self.assertRaises(ValueError):
            design.height_max(points, -6, 0)

    def test_flat_coast_interior_minimum(self):
        reserve, end = design.segment_reserve([(-100, 0), (100, 0)], (0, 20, 1, -5), 0, 10, 1, 2)
        self.assertAlmostEqual(reserve, 16.75)
        self.assertEqual(end, (1, 20, 1, 5))

    def test_narrow_spike_inside_body_footprint_is_not_missed(self):
        points = [(-100, 0), (0, 0), (0.01, 19), (0.02, 0), (100, 0)]
        reserve, _ = design.segment_reserve(points, (-2, 20, 1, 0), 0, 0, 4, 1)
        self.assertAlmostEqual(reserve, 0)

    def test_analytical_minimum_matches_dense_square_envelope_samples(self):
        rng = random.Random(8192)
        points = [(-20 + i * 4, rng.uniform(-5, 5)) for i in range(31)]
        for _ in range(12):
            state = (0, rng.uniform(5, 20), rng.uniform(1, 4), rng.uniform(-8, 8))
            ax, ay, radius, duration = rng.uniform(0, 3), rng.uniform(-10, 10), 2, 2
            minimum, _ = design.segment_reserve(points, state, ax, ay, duration, radius)
            sampled = []
            for i in range(2001):
                t = i * duration / 2000
                x = state[0] + state[2] * t + 0.5 * ax * t * t
                y = state[1] + state[3] * t + 0.5 * ay * t * t
                sampled.append(y - radius - design.height_max(points, x - radius, x + radius))
            self.assertLessEqual(minimum, min(sampled) + 1e-10)
            self.assertLess(min(sampled) - minimum, 0.03)

    def test_idle_tail_inversion_uses_discrete_gravity(self):
        g, t, x, y, vx, vy = 9.81, 8, 2, 70, 0.3, 36
        row = {"row_id": "fallback_row_27", "entry_physics_step": 100,
               "stop_reason": "finite_trace_bound", "stop_state": {
                   "physics_step": 2020, "position_m": {"x": x + vx * t, "y": y + vy * t - 0.5 * g * t * (t + 1 / 120)},
                   "velocity_mps": {"x": vx, "y": vy - g * t}, "held_command": {"throttle_frac": 0}}}
        for a, b in zip(design.upright_boundary(row, g), (x, y, vx, vy)):
            self.assertAlmostEqual(a, b)
        row["stop_reason"] = "physical_trace: reserve"
        self.assertIsNone(design.upright_boundary(row, g))

    def test_forward_screen_checks_certificate_beyond_progress(self):
        points = [(-100, 0), (15, 0), (25, 100), (100, 100)]
        screen = design.forward_screen(points, (0, 40, 1, 5), 10, 1, 13.32, 9.81, math.pi / 2)
        self.assertGreater(screen["estimated_certificate_x_m"], 25)
        self.assertLess(screen["minimum_estimated_reserve_m"], 5)

    def test_forward_screen_does_not_ignore_turning_drift(self):
        points = [(-100, 0), (200, 0)]
        fast = design.forward_screen(points, (0, 100, 10, 10), 30, 1, 13.32, 9.81, math.pi / 2)
        slow = design.forward_screen(points, (0, 100, 10, 10), 30, 1, 13.32, 9.81, math.pi / 4)
        self.assertNotEqual(fast["estimated_certificate_x_m"], slow["estimated_certificate_x_m"])
        for vy, rate in [(-1, 1), (1, 0), (math.nan, 1)]:
            with self.assertRaises(ValueError):
                design.forward_screen(points, (0, 100, 1, vy), 30, 1, 13.32, 9.81, rate)

    def test_delayed_lift_respects_total_power_not_stage_budget(self):
        points = [(-100, 0), (15, 0), (25, 100), (200, 100)]
        row = design.delayed_screen(points, (0, 40, 1, 5), 10, 1, 13.32, 9.81, math.pi / 2)
        self.assertIsNotNone(row)
        self.assertLessEqual(row["total_estimated_power_s"], 16)
        self.assertGreater(row["extra_estimated_lift_s"], 0)


if __name__ == "__main__":
    unittest.main()
