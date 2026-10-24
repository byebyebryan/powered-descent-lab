import json
import tempfile
import time
import unittest
from pathlib import Path

from timing_study import (Budget, FIRST, FUEL, assert_fuel_selection, capture,
    compact, encoded, native, read, sha, summary, write_new)

ROOT = Path(__file__).resolve().parents[2]


class TimingTests(unittest.TestCase):
    def test_complete_bank_has_native_execution_proof_and_default_is_preserved(self):
        with tempfile.TemporaryDirectory(prefix="pd-timing-test-") as directory:
            root = Path(directory)
            scenario = read(ROOT/"fixtures/scenarios/flat_terminal_descent.json")
            scenario["sim"].update(max_time_s=90., sample_hz=120)
            scenario["initial_state"].update(position_m={"x": 0, "y": 800},
                velocity_mps={"x": 0, "y": -30}, attitude_rad=0)
            write_new(root/"scenario.json", scenario)
            case = {"id": "software-timing", "state_id": "software", "population": "software",
                "choice": "direct", "scenario_path": str(root/"scenario.json"), "scenario_sha256": sha(root/"scenario.json")}
            write_new(root/"case.json", case)
            allocation = {"max_steps": 30_000_000, "max_bytes": 1024**3, "deadline": time.time()+60}
            write_new(root/"allocation.json", allocation)
            budget = Budget(root, allocation)
            old = native.ArrivalEpisode(encoded(scenario), encoded({"arrival": None, "land": True}), actuation_profile="native_pair_v2")
            old.execute(budget.remaining())
            previous = json.loads(old.finish_json())
            first, fuel = summary(budget, case, FIRST), summary(budget, case, FUEL)
            self.assertEqual(first["info"], previous["info"])
            self.assertLessEqual(compact(fuel["info"])["fuel_used_kg"], compact(first["info"])["fuel_used_kg"])
            self.assertEqual(len(fuel["info"]["landing_search"]["candidates"]), 63)
            assert_fuel_selection(fuel["info"])
            proof = capture(budget, case, root/"case.json", FUEL, root/"capture", fuel["info"], root/"allocation.json")
            self.assertTrue(proof["verified_target_landing"])
            self.assertTrue(proof["physical_replay"])
            with self.assertRaises(Exception):
                native.ArrivalEpisode(encoded(scenario), encoded({"arrival": None, "land": True}), timing_selection="invented")


if __name__ == "__main__":
    unittest.main()
