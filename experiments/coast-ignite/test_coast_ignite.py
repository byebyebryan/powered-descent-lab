import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch
from stable_baselines3.common.buffers import RolloutBuffer

from coast_ignite import CoastEnv, capture_verified, encoded, model_for, native, write_new
from run_loop import Budget, binomial_interval


class ShapeOnlyEnv:
    observation_space = CoastEnv.observation_space
    action_space = CoastEnv.action_space


class Tests(unittest.TestCase):
    def test_hazard_forward_evaluate_predict_and_save_load_agree(self):
        # Construct only network software: no native mission or transition.
        import gymnasium as gym

        class Empty(gym.Env):
            observation_space = CoastEnv.observation_space
            action_space = CoastEnv.action_space

        model = model_for(Empty(), 44)
        obs = np.zeros((81, 10), dtype=np.float32)
        obs[:, -1] = np.arange(81, 0, -1) / 81
        tensor = torch.as_tensor(obs)
        with torch.no_grad():
            dist = model.policy.get_distribution(tensor)
            p = dist.distribution.probs[:, 1].numpy()
            np.testing.assert_allclose(p, 1 / np.arange(82, 1, -1), rtol=2e-6)
            actions, _, logp = model.policy(tensor)
            _, evaluated_logp, _ = model.policy.evaluate_actions(tensor, actions)
            torch.testing.assert_close(logp, evaluated_logp)
            greedy, _ = model.predict(obs, deterministic=True)
            self.assertTrue(np.all(greedy == 0))
        survival = 1.0
        masses = []
        for hazard in p:
            masses.append(survival * float(hazard))
            survival *= 1 - float(hazard)
        np.testing.assert_allclose(masses + [survival], np.full(82, 1 / 82), rtol=3e-6)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "model.zip"
            with torch.no_grad():
                model.policy.action_net.bias[1] = 4.0
            model.save(path)
            loaded = type(model).load(path, device="cpu")
            np.testing.assert_array_equal(model.predict(obs, deterministic=True)[0],
                                          loaded.predict(obs, deterministic=True)[0])

    def test_returns_respect_task_end_and_bootstrap_buffer_cutoff(self):
        buffer = RolloutBuffer(5, CoastEnv.observation_space, CoastEnv.action_space,
                               gamma=1.0, gae_lambda=1.0, n_envs=1)
        values = [0.2, 0.3, 0.8, 0.7, 0.9]
        for i, reward in enumerate([0, 0, 1, 0, 0]):
            buffer.add(np.zeros((1, 10)), np.zeros((1, 1)), np.array([reward]),
                       np.array([i in [0, 3]]), torch.tensor([values[i]]), torch.tensor([0.0]))
        buffer.compute_returns_and_advantage(torch.tensor([0.4]), np.array([False]))
        np.testing.assert_allclose(buffer.returns[:, 0], [1, 1, 1, 0.4, 0.4], atol=1e-6)

    def test_exact_intervals_handle_edges_and_symmetry(self):
        lo, hi = binomial_interval(0, 600)
        self.assertEqual(lo, 0)
        self.assertAlmostEqual(hi, 1 - 0.025**(1 / 600))
        lo, hi = binomial_interval(600, 600)
        self.assertEqual(hi, 1)
        self.assertAlmostEqual(lo, 0.025**(1 / 600))
        lo, hi = binomial_interval(300, 600)
        self.assertAlmostEqual(lo, 1 - hi)

    def test_saved_model_native_control_and_physical_proofs_and_input_drift(self):
        import gymnasium as gym

        class Empty(gym.Env):
            observation_space = CoastEnv.observation_space
            action_space = CoastEnv.action_space

        anchor = Path(__file__).resolve().parents[2] / "fixtures/scenarios/flat_terminal_descent.json"
        proposal = next(p for i in range(256) if (p := __import__("json").loads(
            native.generate_proposal(anchor.read_text(), 991, "proof-test", i)))["admitted"])
        task = {"scenario": proposal["scenario"], "reference_end": proposal["reference_end"],
                "physical_sha256": proposal["physical_sha256"]}
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[2] / "target") as folder:
            folder = Path(folder)
            task_path = folder / "task.json"
            write_new(task_path, task)
            model = model_for(Empty(), 45)
            with torch.no_grad():
                model.policy.action_net.bias[1] = 8.0
            model_path = folder / "model.zip"
            model.save(model_path)
            budget = Budget(folder / "budget", 60, 100_000, 64 * 2**20, __import__("time").time() + 60)
            proof = capture_verified(task, task_path, {"kind": "model"}, folder / "capture",
                                     budget, model, model_path)
            self.assertTrue(proof["controller_reconstruction"])
            self.assertTrue(proof["planner_reconstruction"])
            native.render_report(encoded({"title": "Software proof fixture", "summary": "No campaign claim",
                                 "rows": [{"capture": str(folder / "capture"), "population": "Software fixture", "label": "Saved model"}]}), str(folder / "report"))
            self.assertTrue((folder / "report/navigation.json").is_file())
            html = (folder / "report/flight-0.html").read_text()
            self.assertIn("../capture/evaluation.json", html)
            self.assertIn("Back to mission tree", html)
            episode = native.CoastEpisode(encoded(task["scenario"]), task["reference_end"], True,
                                          None, [str(task_path)])
            episode.step(0)
            task_path.write_text("changed after reset")
            while not episode.step(0)[2]:
                pass
            with self.assertRaisesRegex(ValueError, "integrity"):
                episode.save_capture(str(folder / "drift"))


if __name__ == "__main__":
    torch.set_num_threads(1)
    unittest.main()
