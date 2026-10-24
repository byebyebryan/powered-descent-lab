import json
import tempfile
import time
import unittest
from pathlib import Path
from composition_study import Budget, capture, encoded, native, read, sha, summary, write_new

ROOT = Path(__file__).resolve().parents[2]


class CompositionTests(unittest.TestCase):
    def test_witness_and_guidance_select_from_first_actual_resolution_with_complete_proof(self):
        with tempfile.TemporaryDirectory(prefix="pd-composition-test-") as directory:
            root = Path(directory)
            s = read(ROOT / "fixtures/scenarios/flat_terminal_descent.json")
            s["sim"].update(max_time_s=90.0, sample_hz=120)
            s["initial_state"].update(position_m={"x": 0, "y": 800}, velocity_mps={"x": 0, "y": -30}, attitude_rad=0)
            write_new(root / "scenario.json", s)
            recipe = {"physics_ticks": 720, "prepare_ticks": 0, "thrust_accel_mps2": {"x": 0, "y": 9.81}}
            g = json.loads(native.generate_witness(encoded(s), encoded(recipe)))
            self.assertTrue(g["admitted"])
            case = {"id": "software-composition", "population": "software", "scenario_path": str(root / "scenario.json"),
                "scenario_sha256": sha(root / "scenario.json"), "recipe": recipe, "program": {"arrival": g["request"], "land": True}}
            write_new(root / "case.json", case)
            a = {"max_steps": 20_000_000, "max_bytes": 1024**3, "deadline": time.time() + 60}
            write_new(root / "allocation.json", a)
            b = Budget(root, a)
            for mode in ["witness", "native_pair_v2"]:
                result = summary(b, case, mode)
                info = result["info"]
                self.assertEqual(info["arrival"]["status"], "reached")
                self.assertLess(info["arrival"]["resolution"]["physics_step"], 720)
                self.assertEqual(info["landing_search"]["selection_observation"], info["arrival"]["resolution"])
                proof = capture(b, case, root / "case.json", mode, root / mode, info, root / "allocation.json")
                self.assertTrue(proof["physical_replay"])
                self.assertTrue(proof["verified_target_landing"])
                self.assertGreater(info["forecast_steps"], 0)


if __name__ == "__main__":
    unittest.main()
