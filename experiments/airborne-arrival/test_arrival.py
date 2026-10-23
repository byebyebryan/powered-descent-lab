import copy
import json
import tempfile
import time
import unittest
from pathlib import Path

from arrival import Budget, capture, compact, encoded, execute_summary, findings, native, program, read, scenario, write_new

ROOT = Path(__file__).resolve().parents[2]


class ArrivalTests(unittest.TestCase):
    def setUp(self):
        self.anchor = read(ROOT / "fixtures/scenarios/flat_terminal_descent.json")

    def test_input_geometry_and_goal_bank_are_deterministic_and_do_not_mutate_input(self):
        original = copy.deepcopy(self.anchor)
        s = scenario(self.anchor, 0, 60, "mid", 75, -1)
        self.assertEqual(self.anchor, original)
        n = int(s["metadata"]["nominal_ballistic_ticks"])
        p, v = s["initial_state"]["position_m"], s["initial_state"]["velocity_mps"]
        self.assertAlmostEqual(p["x"] + n / 120 * v["x"], -75)
        self.assertAlmostEqual(p["y"] + n / 120 * v["y"] - 9.81 * n * (n + 1) / (2 * 120**2), 5)
        before = copy.deepcopy(s)
        a = program(s, "p35-v70")
        self.assertEqual(a, program(s, "p35-v70"))
        self.assertEqual(s, before)
        self.assertEqual(a["arrival"]["deadline_physics_step"], 3600)
        self.assertIsNone(program(s, "direct")["arrival"])

    def test_headroom_excludes_successful_landing_after_failed_arrival(self):
        rows = [{"case": "a", "choice": "direct", "result": {"success": True, "arrival_status": "direct", "fuel_used_kg": 20}},
                {"case": "a", "choice": "p35-v70", "result": {"success": True, "arrival_status": "plane_miss", "fuel_used_kg": 10}}]
        f = findings(rows)
        self.assertEqual(f["best_tested"]["a"], "direct")
        self.assertEqual(f["fuel_improvements"], [])
        self.assertEqual(f["arrival_landed_without_fulfillment"], 1)

    def test_full_native_proof_reconstructs_controller_and_input_goal_rule(self):
        with tempfile.TemporaryDirectory(prefix="pd-arrival-test-") as folder:
            root = Path(folder)
            a = {"max_steps": 10_000_000, "max_bytes": 1024**3, "deadline": time.time() + 60}
            write_new(root / "allocation.json", a)
            s = self.anchor
            s["initial_state"]["position_m"] = {"x": 0, "y": 600}
            s["initial_state"]["velocity_mps"] = {"x": 0, "y": -10}
            write_new(root / "input.json", s)
            b = Budget(root, a)
            result = execute_summary(b, s, "direct")
            self.assertTrue(result["info"]["native_success"])
            proof = capture(b, s, root / "input.json", "direct", root / "capture",
                            result["info"], root / "allocation.json")
            self.assertTrue(proof["verified_target_landing"])
            self.assertTrue(proof["summary_equality"])
            self.assertEqual(b.counts["reconstructions"], 2)
            evaluation = read(root / "capture/evaluation.json")
            self.assertTrue(evaluation["integrity_passed"])
            self.assertEqual(compact(result["info"])["arrival_status"], "direct")
            ep = native.ArrivalEpisode(encoded(s), encoded(program(s, "direct")), True,
                                       input_files=[str(root / "input.json")])
            ep.execute(10_000_000)
            (root / "input.json").write_text("{}")
            with self.assertRaises(ValueError):
                ep.save_capture(str(root / "drift-capture"))


if __name__ == "__main__":
    unittest.main()
