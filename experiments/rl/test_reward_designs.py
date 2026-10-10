import json
import unittest

import numpy as np
import pd_native

from pd_env import ProceduralLandingEnv as Original
from pd_env_v2 import ProceduralLandingEnv as Braking
from pd_env_v3 import ProceduralLandingEnv as Aligned, potential_reward


class RewardDesignContracts(unittest.TestCase):
    def test_discounted_shaping_has_no_failed_endpoint_payoff(self):
        for potentials in [[-1.5, -0.1], [-1.5, -3, -2, -0.02], [-1.5]+[-0.5]*900]:
            total = sum(0.999**index*potential_reward(previous, following, index == len(potentials)-2)
                        for index, (previous, following) in enumerate(zip(potentials, potentials[1:])))
            self.assertAlmostEqual(total, 150.0, places=8)

    def test_reward_revision_preserves_observations_actions_clocks_and_physics(self):
        mission = json.loads(pd_native.generate_mission("layered", 901, 7))["mission"]
        original = Original(fixed_missions=[mission], captured=True, agent_id="reward-test")
        braking = Braking(fixed_missions=[mission], captured=True, agent_id="reward-test")
        aligned = Aligned(fixed_missions=[mission], captured=True, agent_id="reward-test")
        a, _ = original.reset()
        b, _ = braking.reset()
        c, _ = aligned.reset()
        np.testing.assert_array_equal(a, b)
        np.testing.assert_array_equal(a, c)
        rng = np.random.default_rng(902)
        for _ in range(1000):
            action = rng.uniform(-1, 1, 2)
            a = original.step(action)
            b = braking.step(action)
            c = aligned.step(action)
            np.testing.assert_array_equal(a[0], b[0])
            np.testing.assert_array_equal(original.raw, braking.raw)
            self.assertEqual(a[2:4], b[2:4])
            np.testing.assert_array_equal(a[0], c[0])
            np.testing.assert_array_equal(original.raw, aligned.raw)
            self.assertEqual(a[2:4], c[2:4])
            if a[2]:
                break
        self.assertTrue(a[2])
        old = json.loads(original.episode.finish_json())
        new = json.loads(braking.episode.finish_json())
        self.assertEqual(old, new)
        self.assertEqual(old, json.loads(aligned.episode.finish_json()))

    def test_potential_distinguishes_contact_velocity_and_attitude(self):
        mission = json.loads(pd_native.generate_mission("value-noise", 903, 0))["mission"]
        env = Braking(fixed_missions=[mission])
        env.reset()
        stable = env.raw.copy()
        stable[11], stable[12], stable[16] = 0, env.base_offset, 0
        fast = stable.copy()
        fast[4], fast[5] = 75, -35
        tilted = stable.copy()
        tilted[6], tilted[7] = 0.3, 1.0
        self.assertLess(env._potential(fast), env._potential(stable))
        self.assertLess(env._potential(tilted), env._potential(stable))
        mirrored = tilted.copy()
        mirrored[[4, 6, 7]] *= -1
        self.assertEqual(env._potential(tilted), env._potential(mirrored))


if __name__ == "__main__":
    unittest.main()
