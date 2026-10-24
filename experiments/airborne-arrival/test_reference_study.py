import json
import tempfile
import time
import unittest
from pathlib import Path

from reference_inputs import scenarios, PATTERNS
from reference_study import (Budget, FIRST, FUEL, EXTENDED, compact, summary, capture,
    read, sha, write_new)

ROOT = Path(__file__).resolve().parents[2]


class ReferenceTests(unittest.TestCase):
    def test_population_is_independent_reproducible_and_boundary_is_declared(self):
        anchor = read(ROOT/"fixtures/scenarios/flat_terminal_descent.json")
        primary = list(scenarios(anchor, "primary", test_seed=12345))
        self.assertEqual(primary, list(scenarios(anchor, "primary", test_seed=12345)))
        self.assertEqual(len(primary), 300)
        self.assertEqual(len({s["id"] for s in primary}), 300)
        counts = {}
        for s in primary:
            key = (s["metadata"]["family"], s["metadata"]["sink_band"])
            counts[key] = counts.get(key, 0)+1
        self.assertEqual(len(counts), 12)
        self.assertGreater(len(set(counts.values())), 1)  # No balanced per-cell quota.
        boundary = list(scenarios(anchor, "boundary", test_seed=12346))
        self.assertEqual(len(boundary), 60)
        self.assertEqual(len(PATTERNS), 5)
        self.assertEqual({s["initial_state"]["position_m"]["y"] for s in boundary}, {725.,875.})
        self.assertEqual({s["metadata"]["endpoint_pattern"] for s in boundary}, set(map(str, range(5))))

    def test_extended_native_proof_and_measurement_are_separate_from_decisions(self):
        with tempfile.TemporaryDirectory(prefix="pd-reference-test-") as directory:
            root = Path(directory)
            scenario = read(ROOT/"fixtures/scenarios/flat_terminal_descent.json")
            scenario["sim"].update(max_time_s=90., sample_hz=120)
            scenario["initial_state"].update(position_m={"x":0,"y":800}, velocity_mps={"x":0,"y":-30}, attitude_rad=0)
            write_new(root/"scenario.json", scenario)
            case = {"id":"software-reference", "state_id":"software", "population":"software", "choice":"direct",
                "scenario_path":str(root/"scenario.json"), "scenario_sha256":sha(root/"scenario.json")}
            write_new(root/"case.json", case)
            allocation = {"max_steps":30_000_000,"max_bytes":1024**3,"deadline":time.time()+90}
            write_new(root/"allocation.json", allocation)
            budget = Budget(root, allocation)
            results = {}
            for mode in [FIRST, FUEL, EXTENDED]:
                plain, measured = summary(budget, case, mode), summary(budget, case, mode, measured=True)
                self.assertEqual(plain["info"], measured["info"])
                self.assertEqual(plain["summary"], measured["summary"])
                self.assertIsNone(plain["measurements"])
                self.assertEqual(len(measured["measurements"]["selection_wall_ns"]), 1)
                self.assertGreater(measured["measurements"]["selection_wall_ns"][0], 0)
                self.assertGreater(len(measured["measurements"]["ordinary_frame_wall_ns"]), 10)
                results[mode] = plain
            self.assertLessEqual(compact(results[EXTENDED]["info"])["fuel_used_kg"], compact(results[FUEL]["info"])["fuel_used_kg"])
            proof = capture(budget, case, root/"case.json", EXTENDED, root/"capture", results[EXTENDED]["info"], root/"allocation.json")
            self.assertTrue(proof["verified_target_landing"])
            self.assertTrue(proof["physical_replay"])


if __name__ == "__main__":
    unittest.main()
