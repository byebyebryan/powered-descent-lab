import json
import tempfile
import time
import unittest
from pathlib import Path
from actuation_study import Budget, capture, encoded, native, read, sha, summary, write_new

ROOT = Path(__file__).resolve().parents[2]


class ActuationTests(unittest.TestCase):
    def fixture(self, folder):
        s = read(ROOT / "fixtures/scenarios/flat_terminal_descent.json")
        s["sim"].update(max_time_s=90.0, sample_hz=120)
        s["initial_state"].update(position_m={"x": 0, "y": 800}, velocity_mps={"x": 0, "y": -30}, attitude_rad=0)
        path = folder / "scenario.json"
        write_new(path, s)
        r = {"physics_ticks": 720, "prepare_ticks": 0, "thrust_accel_mps2": {"x": 0, "y": 9.81}}
        g = json.loads(native.generate_witness(encoded(s), encoded(r)))
        self.assertTrue(g["admitted"])
        c = {"id": "software-witness", "population": "software", "scenario_path": str(path),
             "scenario_sha256": sha(path), "recipe": r, "program": {"arrival": g["request"], "land": False}}
        return c

    def test_native_bench_checks_minimum_and_fuel_regimes(self):
        s = read(ROOT / "fixtures/scenarios/flat_terminal_descent.json")
        result = json.loads(native.actuation_bench(encoded(s)))
        self.assertTrue(result["passed"])
        self.assertEqual(result["native_steps"], 2 * len(result["cases"]))
        self.assertIn("fuel_limited", {r["allocation"]["regime"] for r in result["cases"]})
        self.assertIn("gap_off", {r["allocation"]["regime"] for r in result["cases"]})

    def test_independent_witness_and_corrected_capture_have_full_proof(self):
        with tempfile.TemporaryDirectory(prefix="pd-actuation-test-") as directory:
            folder = Path(directory)
            case = self.fixture(folder)
            write_new(folder / "case.json", case)
            a = {"max_steps": 20_000_000, "max_bytes": 1024**3, "deadline": time.time() + 60}
            write_new(folder / "allocation.json", a)
            b = Budget(folder, a)
            for mode in ["witness", "native_pair_v2"]:
                result = summary(b, case, mode)
                if mode == "witness":
                    self.assertEqual(result["info"]["arrival"]["status"], "reached")
                proof = capture(b, case, folder / "case.json", mode, folder / mode,
                                result["info"], folder / "allocation.json")
                self.assertTrue(proof["physical_replay"])
                self.assertTrue(proof["summary_equality"])


if __name__ == "__main__":
    unittest.main()
