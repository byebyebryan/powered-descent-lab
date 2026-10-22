import math
import unittest

from stopping_policy import Stopper, uniform


class StoppingTests(unittest.TestCase):
    def test_random_stream_has_golden_coordinates_and_is_order_independent(self):
        expected = {(0, 0): "0x1.719fd254369b8p-4", (0, 12): "0x1.fe43c06a4cf99p-1",
                    (7, 960): "0x1.d0a398fc23782p-2"}
        for coordinates in reversed(list(expected)):
            self.assertEqual(uniform("golden", "a" * 64, *coordinates).hex(), expected[coordinates])
        self.assertNotEqual(uniform("golden", "a" * 64, 0, 0),
                            uniform("golden", "a" * 64, 1, 0))
        for args in [("x", "a" * 63, 0, 0), ("x", "a" * 64, 0, 1),
                     ("x", "a" * 64, -1, 0)]:
            with self.assertRaises(ValueError):
                uniform(*args)

    def test_sampling_has_expected_frequency_without_global_rng(self):
        n = 10000
        draws = [uniform("frequency", "b" * 64, r, 0) for r in range(n)]
        self.assertTrue(all(0 < x < 1 for x in draws))
        self.assertLess(abs(sum(x < .25 for x in draws) / n - .25), .015)

    def test_median_retains_never_mass_and_resets_per_episode(self):
        median = Stopper("M", "a" * 64, "unused")
        self.assertEqual([median.advance(.1, 12*k) for k in range(7)], [0]*6 + [1])
        with self.assertRaises(ValueError):
            median.advance(.1, 84)
        never = Stopper("M", "a" * 64, "unused")
        for k in range(81):
            self.assertEqual(never.advance(.001, 12*k), 0)
        self.assertTrue(never.closed)
        self.assertAlmostEqual(never.trace[-1]["survival_after"], .999**81)
        self.assertEqual(Stopper("M", "a" * 64, "unused").log_survival, 0)

    def test_prior_median_and_zero_one_hazards(self):
        median = Stopper("M", "a" * 64, "unused")
        actions = [median.advance(1/(82-k), 12*k) for k in range(41)]
        self.assertEqual(actions, [0]*40 + [1])
        for rule in ["G", "M", "S"]:
            zero = Stopper(rule, "a" * 64, "unused")
            self.assertEqual(zero.advance(0, 0, 0), 0)
            self.assertEqual(zero.advance(1, 12, 1), 1)
            self.assertEqual(zero.trace[-1]["survival_after"], 0)
            self.assertEqual(zero.trace[-1]["cumulative_stopping_probability"], 1)

    def test_g_preserves_original_action_and_does_not_rethreshold_hazard(self):
        greedy = Stopper("G", "a" * 64, "unused")
        self.assertEqual(greedy.advance(.5, 0, 0), 0)
        self.assertEqual(greedy.advance(.5, 12, 1), 1)
        for hazard in [math.nan, math.inf, -.1, 1.1]:
            with self.assertRaises(ValueError):
                Stopper("S", "a" * 64, "unused").advance(hazard, 0)
        with self.assertRaises(ValueError):
            Stopper("M", "a" * 64, "unused").advance(.1, 12)


if __name__ == "__main__":
    unittest.main()
