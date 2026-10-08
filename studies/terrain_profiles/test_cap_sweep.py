"""Pure cap-sweep admission, population, paired comparison and failure tests."""

import copy
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import cap_sweep as s
from test_cap_probe import cap_prefix


class CapSweepTests(unittest.TestCase):
    def manifest(self):
        return {"sentinel_cases": [{"case_id": f"control-{i}", "scenario_path": f"scenarios/control-{i}.json"} for i in range(3)],
                "random_cases": [{"case_id": f"random-{i:03}", "scenario_path": f"scenarios/random-{i:03}.json",
                                  "seed": i, "geometry": {"recipe_id": "test"}} for i in range(1000)]}

    def test_contract_is_fixed_and_uses_existing_probe_not_production_policy(self):
        plan = s.contract()
        self.assertEqual(plan["maximum_corrections"], 24)
        self.assertEqual(s.p.policy(24)["policy_id"], "piecewise_local_clearing_v2_policy_3_cap_probe_24")
        for key, value in (("maximum_corrections", 6), ("maximum_measured_attempts", 1011), ("repeat_indices", [0]), ("workers", 8)):
            with patch("cap_sweep.d.read", return_value=dict(plan, **{key: value})), self.assertRaises(ValueError):
                s.contract()

    def test_population_and_waves_have_no_retry_or_missing_case(self):
        plan = s.contract()
        rows = s.attempts(plan, self.manifest())
        self.assertEqual(len(rows), 1010)
        self.assertEqual(sum(r["cohort"] == "random" for r in rows), 1000)
        self.assertEqual([r["case_id"] for r in rows[-2:]], ["random-030", "random-280"])
        self.assertEqual(len({r["attempt_id"] for r in rows}), 1010)
        waves = s.survey.waves_for(plan)
        self.assertEqual([i for wave in waves for i in wave], list(range(1010)))
        self.assertTrue(all(len(w) <= 4 for w in waves))
        with self.assertRaises(ValueError):
            s.attempts(plan, {"sentinel_cases": [], "random_cases": []})

    def test_paired_summary_keeps_controls_repeats_and_unattempted_out(self):
        def row(cid, old, new, count, cohort="random", status="recorded"):
            return {"case_id": cid, "cohort": cohort, "status": status, "geometry": {"recipe_id": "test"},
                    "result": {"verified_landing": new == "landed", "planning_stop": new, "correction_count": count, "nominal_class": "blocked"},
                    "paired": {"baseline_result": {"verified_landing": old == "landed", "planning_stop": old}, "endpoint": {"physics_step": 100}}}
        rows = [row("old-success", "landed", "landed", 2), row("new-success", "correction_limit", "landed", 7),
                row("later-stop", "correction_limit", "no_clearing", 8), row("control", "landed", "landed", 0, "sentinel"),
                row("repeat-new", "correction_limit", "landed", 7, "repeat"), row("pending", "landed", "landed", 99, status="not_attempted")]
        result = s.paired_summary(rows)
        self.assertEqual(result["baseline_landings"], 1)
        self.assertEqual(result["preserved_landings"], 1)
        self.assertEqual(result["new_landings"], ["new-success"])
        self.assertEqual(result["blocked_landings"], 2)
        self.assertEqual(result["old_cap_outcomes"], {"landed": 1, "no_clearing": 1})
        self.assertEqual(result["maximum_actual_corrections"], 8)
        self.assertEqual(result["per_recipe"]["test"], {"count": 3, "baseline_landings": 1, "landings": 2})
        self.assertEqual(s.paired_summary([])["maximum_actual_corrections"], 0)

    def test_bound_read_rejects_content_drift_and_escaping_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = s.study.encoded({"synthetic": True})
            s.study.write_new(root / "part.json", data)
            self.assertEqual(s.bound_read(root, "part.json", {"part.json": s.study.digest(data)}), {"synthetic": True})
            with self.assertRaises(ValueError):
                s.bound_read(root, "part.json", {"part.json": "changed"})
            with self.assertRaises(ValueError):
                s.bound_read(root, "../part.json", {})

    def capsule(self, root, changed=None):
        actual, old = cap_prefix()
        for f in (actual, old):
            f.update(initial_nominal_terrain_blocked=True, integrity_passed=True, final_source_replay_passed=True,
                     physical_outcome="flying", mission_outcome="in_progress", reason=None,
                     timings={"planning_s": 1, "execution_s": 2, "replay_s": 3})
        actual["planning_stop"] = "no_clearing"
        scenario = {"synthetic": "same input"}
        previous = root / "old"
        out = root / "runs/random-030"
        data = s.study.encoded(old)
        s.study.write_new(previous / "runs/random-030/flight.json", data)
        old_inventory = {"runs/random-030/flight.json": s.study.digest(data)}
        scenario_data = s.study.encoded(scenario)
        s.study.write_new(root / "scenarios/random-030.json", scenario_data)
        inputs = {"scenarios/random-030.json": s.study.digest(scenario_data)}
        printed = {k: actual[k] for k in ("planning_stop", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
                   "final_source_replay_passed", "physical_outcome", "mission_outcome", "reason", "timings")}
        printed.update(output_dir=str(out.resolve()), policy_version=3)
        payload = {"flight": actual, "scenario": scenario, "printed": printed}
        if changed:
            changed(payload)
        s.study.write_new(out / "flight.json", s.study.encoded(payload["flight"]))
        s.study.write_new(out / "scenario.json", s.study.encoded(payload["scenario"]))
        s.study.write_new(out / "summary.json", b"{}")
        s.study.write_new(root / "logs/random-030.stdout", s.study.encoded(payload["printed"]))
        result = {"verified_landing": False}
        row = {"case_id": "random-030", "attempt_id": "random-030", "cohort": "random", "status": "recorded",
               "output_dir": "runs/random-030", "exit_code": 0, "scenario_path": "scenarios/random-030.json", "result": result}
        return previous, old_inventory, inputs, result, row

    def test_record_prefix_input_and_stdout_binding(self):
        changes = [None, lambda x: x["flight"]["ordinary_flight"]["actions"][0].update(command="different"),
                   lambda x: x.update(scenario={"synthetic": "different"}),
                   lambda x: x["printed"].update(output_dir="another-root"),
                   lambda x: x["printed"].update(physical_outcome="crashed")]
        for changed in changes:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                baseline, old, inputs, result, row = self.capsule(root, changed)
                with patch("cap_sweep.p.validate_native", return_value=result), \
                        patch("cap_sweep.subprocess.run", side_effect=AssertionError("execution forbidden")):
                    if changed:
                        with self.assertRaises(ValueError):
                            s.check_record(root, row, baseline, old, inputs)
                    else:
                        checked = s.check_record(root, row, baseline, old, inputs)
                        self.assertEqual(checked["comparison"]["corrections"], 1)
                        self.assertEqual(checked["baseline_result"]["planning_stop"], "correction_limit")

    def test_timeout_retains_log_and_never_retries_or_claims_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = s.attempts(s.contract(), self.manifest())[3]
            with patch("cap_sweep.subprocess.run", side_effect=subprocess.TimeoutExpired("synthetic", 1, output=b"partial")) as run:
                actual = s.attempt(root, root / "not-executed", row, 1, root, {}, {})
            self.assertEqual(run.call_count, 1)
            self.assertEqual(actual["status"], "runner_timeout")
            self.assertIsNone(actual["result"])
            self.assertIsNone(actual["paired"])
            self.assertEqual((root / "logs/random-000.stdout").read_bytes(), b"partial")

    def test_native_cap_admission_and_old_prefix_helpers_remain_strict(self):
        self.assertEqual(s.p.CAPS, [12, 24])
        self.assertEqual(s.p.contract()["maximum_measured_attempts"], 12)
        with self.assertRaises(ValueError):
            s.p.policy(25)
        actual, old = cap_prefix()
        s.p.compare_prefix(actual, old)
        changed = copy.deepcopy(actual)
        changed["absolute_deadline_physics_step"] += 1
        with self.assertRaises(ValueError):
            s.p.compare_prefix(changed, old)


if __name__ == "__main__":
    unittest.main()
