import json
import tempfile
import unittest
from pathlib import Path

import numpy as np
import pd_native
from gymnasium.utils.env_checker import check_env

from pd_env import ProceduralLandingEnv, native_module_paths


def mission():
    return json.loads(pd_native.generate_mission("layered", 501201, 0))["mission"]


class EnvironmentContracts(unittest.TestCase):
    def test_gym_contract_and_observations_exclude_metadata(self):
        original = mission()
        altered = json.loads(json.dumps(original))
        altered["scenario"].update(id="unrelated", seed=999, tags=["hidden"], metadata={"hidden": "99"})
        left = ProceduralLandingEnv(fixed_missions=[original])
        right = ProceduralLandingEnv(fixed_missions=[altered])
        a, _ = left.reset(seed=17)
        b, _ = right.reset(seed=17)
        np.testing.assert_array_equal(a, b)
        self.assertTrue(left.observation_space.contains(a))
        check_env(left, skip_render_check=True)

    def test_captured_and_summary_physics_match_and_replay(self):
        fixed = mission()
        summary = ProceduralLandingEnv(fixed_missions=[fixed])
        captured = ProceduralLandingEnv(fixed_missions=[fixed], captured=True)
        summary.reset()
        captured.reset()
        with tempfile.TemporaryDirectory(prefix="pd-rl-test-") as temp:
            input_path = Path(temp)/"mission.json"
            input_path.write_text(json.dumps(fixed))
            evidence = pd_native.evidence_snapshot([__file__], [str(input_path), *map(str,native_module_paths())])
            done = False
            for _ in range(1000):
                action = np.array([0.0, 0.0], dtype=np.float32)
                a = summary.step(action)
                b = captured.step(action)
                np.testing.assert_array_equal(a[0], b[0])
                self.assertEqual(a[1:4], b[1:4])
                np.testing.assert_array_equal(summary.raw, captured.raw)
                done = a[2]
                if done:
                    break
            self.assertTrue(done)
            self.assertFalse(a[3])
            with self.assertRaises(RuntimeError):
                summary.step(action)
            descriptor = json.dumps({"schema_version": 1, "implementation": captured.agent_id,
                                     "configuration": {"test": True}, "checkpoint_sha256": None})
            result = json.loads(captured.episode.save_capture(descriptor, evidence, evidence,
                                str(Path(temp)/"capture"), 0))
            self.assertTrue(result["integrity_passed"])
            self.assertTrue(result["replay"]["complete_episode_verified"])
            verify = json.loads(pd_native.verify_capture(str(Path(temp)/"capture")))
            self.assertTrue(verify["replay"]["complete_episode_verified"])
            self.assertIn("Trajectory", (Path(temp)/"capture/report.html").read_text())
            source = Path(temp)/"capture/evaluation.json"
            source.write_text(source.read_text()+" ")
            with self.assertRaises(ValueError):
                pd_native.verify_capture(str(Path(temp)/"capture"))

    def test_explicit_reset_finite_actions_and_summary_retention(self):
        env = ProceduralLandingEnv(fixed_missions=[mission()])
        env.reset()
        with self.assertRaises(ValueError):
            env.step([float("nan"), 0])
        self.assertEqual(env.episode.retained_record_counts(), (0, 0, 0, 0))
        native = pd_native.NativeEpisode(json.dumps(mission()["scenario"]))
        _, done, advanced, _, _ = native.step(float("nan"), 0)
        self.assertTrue(done)
        self.assertEqual(advanced, 0)
        result = json.loads(native.finish_json())
        self.assertIsNone(result["capture"])
        self.assertEqual(result["summary"]["stop"], "execution_invalid")
        with self.assertRaises(ValueError):
            native.step(1, 0)


if __name__ == "__main__":
    unittest.main()
