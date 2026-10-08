"""Pure collection-contract checks: no native flights or retained-data dependency."""

import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import early_exit_sweep as s


def population():
    cases = [{"case_id": f"random-{i:03}", "scenario_path": f"scenarios/random-{i:03}.json", "seed": i,
              "geometry": {"recipe_id": "synthetic"}} for i in range(1000)]
    return {"random_cases": cases, "sentinel_cases": [{"case_id": f"control-{i}", "scenario_path": f"scenarios/control-{i}.json"} for i in range(3)]}


class SweepTests(unittest.TestCase):
    def test_scenario_values_ignore_only_serialization_not_numeric_or_structural_changes(self):
        self.assertTrue(s.same_scenario(b'{"x":1.0,"g":0.1}', b'{ "g": 0.1, "x": 1 }'))
        self.assertFalse(s.same_scenario(b'{"g":0.10000000000000002,"x":1}', b'{"g":0.1,"x":1}'))
        self.assertFalse(s.same_scenario(b'{"g":0.1,"x":1,"extra":0}', b'{"g":0.1,"x":1}'))

    def test_frozen_population_and_repeats_have_no_extra_retry(self):
        plan = s.contract()
        rows = s.cap_sweep.attempts(plan, population())
        self.assertEqual(len(rows), 1010)
        self.assertEqual([r["attempt_id"] for r in rows[-7:]], ["repeat-000", "repeat-250", "repeat-500", "repeat-750", "repeat-001", "repeat-030", "repeat-280"])
        self.assertEqual(len({r["attempt_id"] for r in rows}), 1010)
        self.assertEqual([i for wave in s.survey.waves_for(plan) for i in wave], list(range(1010)))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "changed.json"
            s.study.write_new(path, s.study.encoded(dict(plan, seed_count=1001)))
            with self.assertRaises(ValueError):
                s.contract(path)

    def test_summary_keeps_losses_and_gains_not_just_net_percentage(self):
        rows = []
        for cid, old, new in (("lost", True, False), ("new", False, True), ("kept", True, True)):
            rows.append({"case_id": cid, "cohort": "random", "status": "recorded", "geometry": {"recipe_id": "synthetic"},
                         "result": {"verified_landing": new, "nominal_class": "blocked", "correction_count": 1},
                         "paired": {"baseline_result": {"verified_landing": old}, "comparison": {"early_dispositions": ["committed"] if new else ["terrain_blocked"]}}})
        summary = s.paired_summary(rows)
        self.assertEqual(summary["baseline_landings"], 2)
        self.assertEqual(summary["preserved_landings"], 1)
        self.assertEqual(summary["lost_landings"], ["lost"])
        self.assertEqual(summary["new_landings"], ["new"])
        self.assertFalse(s.contract()["finite_regressions_stop_collection"])

    def test_unchanged_comparison_cannot_drop_ordinary_commands_or_states(self):
        previous = {"policy": {}, "input_identity": "old", "timings": {k: 1 for k in ("planning_s", "execution_s", "replay_s")},
                    "cycles": [{"local_search": None}], "ordinary_flight": {"actions": [{"physics_step": 0}], "final_state": {"fuel": 42}}}
        actual = copy.deepcopy(previous)
        actual["input_identity"] = "new"
        self.assertIn("complete", s.compare_flight(actual, previous)["kind"])
        for key in ("actions", "final_state"):
            changed = copy.deepcopy(actual)
            changed["ordinary_flight"][key] = None
            with self.assertRaises(ValueError):
                s.compare_flight(changed, previous)

    def test_saved_partial_verification_runs_no_process_and_is_independent_of_live_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = copy.deepcopy(s.contract())
            s.study.write_new(root / "plan.json", s.PLAN.read_bytes())
            plan_path = str(s.PLAN.relative_to(s.survey.REPO))
            source = {"production_executable_sha256": plan["production_executable_sha256"], "files": {plan_path: s.PLAN_SHA256}}
            s.study.write_new(root / "inputs/operator_source" / plan_path, s.PLAN.read_bytes())
            binary = b"synthetic never-executed binary"
            plan["candidate_executable_sha256"] = s.study.digest(binary)
            s.study.write_new(root / "candidate/bin/pd-eval", binary)
            seal = {"files": {}, "executable_sha256": plan["candidate_executable_sha256"]}
            seal_bytes = s.study.encoded(seal)
            s.study.write_new(root / "candidate/source-seal.json", seal_bytes)
            for name, data, key in (("receipt.json", s.study.encoded({"files": {"variant/source-seal.json": s.study.digest(seal_bytes)}}), "candidate_receipt_sha256"),
                                    ("manifest.json", b"{}", "candidate_manifest_sha256")):
                s.study.write_new(root / "candidate" / name, data)
                plan[key] = s.study.digest(data)
            manifest = {"schema": "pd-lab.early-exit-sweep-inputs.v1", "operator_source": source, "protected": {}, "candidate_source_seal_sha256": s.study.digest(seal_bytes)}
            s.study.write_new(root / "manifest.json", s.study.encoded(manifest))
            pop = population()
            rows = s.cap_sweep.attempts(plan, pop)
            s.study.write_new(root / "run-start.json", s.study.encoded(rows))
            inventory = {}
            for row in rows:
                path = row["scenario_path"]
                if path not in inventory:
                    s.study.write_new(root / path, b"{}")
                    inventory[path] = s.study.digest(b"{}")
            report = {"schema": "pd-lab.early-exit-sweep-results.v1", "source_after": source, "protected_after": {},
                      "manifest_sha256": s.study.digest(s.study.encoded(manifest)), "rows": rows, "measured_attempts": 0,
                      "summary": s.survey.summary_for(rows), "paired_summary": s.paired_summary(rows), "stopped_reason": "synthetic before missions"}
            s.study.write_new(root / "early-exit-sweep.json", s.study.encoded(report))
            s.study.write_new(root / "receipt.json", s.study.encoded({"files": s.survey.inventory(root)}))
            with patch.object(s, "contract", return_value=plan), patch.object(s, "baseline_inputs", return_value=(root, pop, inventory)), patch.object(s.subprocess, "run", side_effect=AssertionError("saved verification executed")):
                self.assertEqual(s.verify(root)["measured_attempts"], 0)


if __name__ == "__main__":
    unittest.main()
