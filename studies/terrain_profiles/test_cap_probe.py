"""Pure/synthetic cap-probe checks: no builds or measured flight attempts."""

import copy
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import cap_probe as p
from test_diagnostics import flight, compact


def cap_prefix():
    boundary = {"physics_step": 10, "position_m": {"x": 100}, "fuel_kg": 5000}
    earlier = {"current_state": {"physics_step": 0}, "decision": "correct", "conflict_state": None,
               "conflict_incoming_contact": None, "local_search": {"selected": "same"}, "audit": {"passed": False}}
    at_cap = {"current_state": boundary, "decision": "correction_limit", "conflict_state": None,
              "conflict_incoming_contact": None, "local_search": None, "audit": {"passed": False},
              "nominal_updates": [{"physics_step": 10, "command": "same"}]}
    f = {"planning_stop": "correction_limit", "correction_count": 1, "policy": {"maximum_corrections": 1},
         "cycles": [earlier, at_cap], "segments": [{"kind": "local_correction", "end_physics_step": 10}],
         "absolute_deadline_physics_step": 100, "ordinary_flight": {"final_state": boundary,
         "actions": [{"physics_step": 0, "command": "same"}], "events": [{"physics_step": 0}],
         "samples": [{"physics_step": 0}, {"physics_step": 10}]}}
    a = copy.deepcopy(f)
    a["cycles"][1].update(decision="correct", local_search={"selected": "new"}, conflict_state={"physics_step": 20})
    a["cycles"].append({"current_state": {"physics_step": 30}})
    a["segments"].append({"kind": "local_correction", "end_physics_step": 30})
    a["ordinary_flight"]["actions"].append({"physics_step": 10, "command": "new"})
    a["ordinary_flight"]["events"].append({"physics_step": 10})
    a["ordinary_flight"]["samples"].append({"physics_step": 20})
    return a, f


class CapProbeTests(unittest.TestCase):
    def test_fixed_caps_populations_and_allowance(self):
        plan = p.contract()
        self.assertEqual(plan["caps"], [12, 24])
        self.assertEqual(sum(len(p.stage_attempts(c)) for c in plan["caps"]), 12)
        self.assertEqual([r["case_id"] for r in p.stage_attempts(12)[:2]], ["random-000", "random-253"])
        self.assertEqual(sum(r["cohort"] == "repeat" for r in p.stage_attempts(12)), 2)
        for key, value in (("caps", [12, 48]), ("maximum_measured_attempts", 13), ("primary_case_ids", p.PRIMARIES[::-1])):
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "plan.json"
                p.study.write_new(path, p.study.encoded(dict(plan, **{key: value})))
                with self.assertRaises(ValueError):
                    p.contract(path)

    def test_transformation_changes_only_cap_and_explicit_probe_identity(self):
        original = b'''pub const WAYPOINT_V2_POLICY_REVISION_3_ID: &str = "piecewise_local_clearing_v2_policy_3";
    pub fn revision_3() -> Self {
        Self {
            policy_id: WAYPOINT_V2_POLICY_REVISION_3_ID.into(),
            maximum_corrections: 6,
        }
    }
historical_policy_maximum_corrections: 6,
'''
        # Synthetic source shape, never a rewritten tracked or captured source.
        with patch("cap_probe.study.digest", return_value=p.POLICY_SHA256):
            for cap in p.CAPS:
                changed = p.transformed_policy(original, cap)
                restored = changed.replace(p.policy(cap)["policy_id"].encode(), b"piecewise_local_clearing_v2_policy_3").replace(
                    f"maximum_corrections: {cap},".encode(), b"maximum_corrections: 6,")
                self.assertEqual(restored, original)
                self.assertIn(b"historical_policy_maximum_corrections: 6", changed)
            with self.assertRaises(ValueError):
                p.transformed_policy(original, 48)
        with self.assertRaises(ValueError):
            p.transformed_policy(original, 12)

    def test_second_stage_only_admitted_by_subject_cap_stop(self):
        row = {"case_id": "random-030", "cohort": "primary", "status": "recorded", "result": {"planning_stop": "correction_limit"}}
        self.assertTrue(p.stage_two_needed([row]))
        for stop in ("landed", "no_nominal", "no_clearing", "deadline"):
            self.assertFalse(p.stage_two_needed([dict(row, result={"planning_stop": stop})]))
        self.assertFalse(p.stage_two_needed([dict(row, cohort="repeat")]))
        self.assertFalse(p.stage_two_needed([dict(row, case_id="random-253")]))

    def test_exact_prefix_includes_state_commands_samples_and_next_nominal(self):
        a, b = cap_prefix()
        self.assertEqual(p.compare_prefix(a, b), {"kind": "exact_executed_cap_prefix", "corrections": 1, "through_physics_step": 10})
        for edit in (
                lambda f: f["cycles"][0].update(local_search={"selected": "different"}),
                lambda f: f["cycles"][1]["current_state"].update(fuel_kg=1),
                lambda f: f["cycles"][1].update(nominal_updates=[]),
                lambda f: f["segments"][0].update(end_physics_step=9),
                lambda f: f["ordinary_flight"]["actions"][0].update(command="different"),
                lambda f: f["ordinary_flight"]["samples"][0].update(physics_step=1),
                lambda f: f.update(absolute_deadline_physics_step=200)):
            changed = copy.deepcopy(a)
            edit(changed)
            with self.assertRaises(ValueError):
                p.compare_prefix(changed, b)

    def test_complete_controls_ignore_only_declared_root_identities_and_timings(self):
        b = dict(flight(), segments=[], ordinary_flight={"final_state": {"physics_step": 100}})
        a = copy.deepcopy(b)
        a.update(policy=p.policy(12), input_identity="new-explicit-policy-bound-id")
        a["timings"]["planning_s"] = 99
        p.compare_prefix(a, b)
        a["correction_count"] = 1
        with self.assertRaises(ValueError):
            p.compare_prefix(a, b)
        a = copy.deepcopy(b)
        a["timings"]["extra"] = 0
        with self.assertRaises(ValueError):
            p.compare_prefix(a, b)

    def valid_probe(self):
        f = dict(flight(), policy=p.policy(12), segments=[],
                 ordinary_flight={"final_state": {"physics_step": 100, "physical_outcome": "landed_on_target", "mission_outcome": "success"}})
        return f

    def test_probe_identity_full_tuple_inventory_and_native_summary_binding(self):
        f = self.valid_probe()
        self.assertTrue(p.validate_native(f, compact(f), 12)["verified_landing"])
        for edit in (
                lambda x: x.update(policy={"policy_id": "piecewise_local_clearing_v2_policy_3", "maximum_corrections": 12}),
                lambda x: x.update(integrity_passed=False),
                lambda x: x.update(final_source_replay_passed=False),
                lambda x: x.update(correction_count=13),
                lambda x: x["ordinary_flight"]["final_state"].update(physical_outcome="flying"),
                lambda x: x["ordinary_flight"]["final_state"].update(physics_step=10000)):
            changed = copy.deepcopy(f)
            edit(changed)
            with self.assertRaises(ValueError):
                p.validate_native(changed, compact(changed), 12)
        c = compact(f)
        c["result"]["mission_outcome"] = "in_progress"
        with self.assertRaises(ValueError):
            p.validate_native(f, c, 12)

    def test_honest_finite_stops_not_reclassified_as_crashes_or_landings(self):
        f = self.valid_probe()
        f.update(planning_stop="no_nominal", physical_outcome="flying", mission_outcome="in_progress")
        f["ordinary_flight"]["final_state"].update(physical_outcome="flying", mission_outcome="in_progress")
        self.assertFalse(p.validate_native(f, compact(f), 12)["verified_landing"])
        for stop in ("no_clearing", "nominal_rejected", "deadline"):
            changed = dict(f, planning_stop=stop)
            p.validate_native(changed, compact(changed), 12)
        with self.assertRaisesRegex(ValueError, "not actually binding"):
            changed = dict(f, planning_stop="correction_limit")
            p.validate_native(changed, compact(changed), 12)
        f.update(physical_outcome="crashed", mission_outcome="failed_crash")
        f["ordinary_flight"]["final_state"].update(physical_outcome="crashed", mission_outcome="failed_crash")
        with self.assertRaises(ValueError):
            p.validate_native(f, compact(f), 12)

    def test_attempt_timeout_is_recorded_without_retry_or_native_claim(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = p.stage_attempts(12)[2]
            with patch("cap_probe.subprocess.run", side_effect=subprocess.TimeoutExpired("synthetic", 1, output=b"partial")) as run:
                actual = p.attempt(root, 12, row, 1)
            self.assertEqual(run.call_count, 1)
            self.assertEqual(actual["status"], "runner_timeout")
            self.assertIsNone(actual["result"])
            self.assertEqual((root / "variants/cap-12/logs/random-030.stdout").read_bytes(), b"partial")

    def test_variant_state_and_create_only_source_changes_are_detectable(self):
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp)
            p.study.write_new(stage / "source/part.rs", b"synthetic source")
            p.study.write_new(stage / "bin/pd-eval", b"synthetic never-executed binary")
            seal = p.variant_state(stage, ["part.rs"])
            self.assertEqual(seal["files"], {"part.rs": p.study.digest(b"synthetic source")})
            self.assertEqual(seal["executable_sha256"], p.study.digest(b"synthetic never-executed binary"))
            with self.assertRaises(FileExistsError):
                p.study.write_new(stage / "source/part.rs", b"replacement")

    def test_saved_verification_checks_source_stage_admission_and_never_runs(self):
        # Synthetic capsule wiring; native records are separately exercised above.
        def capsule(root, mutate=None):
            plan = p.contract()
            original = b"synthetic original policy"
            source = {"files": {p.POLICY_PATH: p.study.digest(original),
                                str(p.PLAN.relative_to(p.survey.REPO)): p.study.digest(p.PLAN.read_bytes())},
                      "production_executable_sha256": plan["production_executable_sha256"]}
            p.study.write_new(root / "inputs/source" / p.POLICY_PATH, original)
            p.study.write_new(root / "inputs/source" / p.PLAN.relative_to(p.survey.REPO), p.PLAN.read_bytes())
            p.study.write_new(root / "plan.json", p.PLAN.read_bytes())
            old_plan = {"cases": []}
            old_files = {}
            for cid in p.PRIMARIES:
                data = p.study.encoded({"synthetic": cid})
                old_plan["cases"].append({"case_id": cid, "scenario_sha256": p.study.digest(data)})
                p.study.write_new(root / "scenarios" / (cid + ".json"), data)
                for name in ("flight.json", "scenario.json"):
                    relative = f"runs/{cid}/{name}"
                    p.study.write_new(root / "baseline" / relative, data)
                    old_files[relative] = p.study.digest(data)
            old_data = p.study.encoded(old_plan)
            old_files["plan.json"] = p.study.digest(old_data)
            p.study.write_new(root / "baseline/plan.json", old_data)
            p.study.write_new(root / "baseline/manifest.json", b"{}")
            p.study.write_new(root / "baseline/receipt.json", p.study.encoded({"files": old_files}))
            plan["baseline_manifest_sha256"] = p.study.digest(b"{}")
            plan["baseline_receipt_sha256"] = p.study.digest((root / "baseline/receipt.json").read_bytes())
            stage = root / "variants/cap-12"
            p.study.write_new(stage / "source" / p.POLICY_PATH, b"synthetic transformed")
            p.study.write_new(stage / "source" / p.PLAN.relative_to(p.survey.REPO), p.PLAN.read_bytes())
            p.study.write_new(stage / "bin/pd-eval", b"synthetic never executed")
            seal = p.variant_state(stage, source["files"])
            p.study.write_new(stage / "source-seal.json", p.study.encoded(seal))
            for cid in p.PRIMARIES:
                p.study.write_new(stage / "preflight" / (cid + ".json"), p.study.encoded(
                    {"supported": True, "rejection": None, "reason": None, "simulation_created": False}))
            frozen = p.stage_attempts(12)
            rows = [dict(r, status="recorded", result={"planning_stop": "landed"}, diagnostic={}) for r in frozen]
            manifest = {"schema": "pd-lab.correction-cap-probe-inputs.v1", "main_source_before": source, "protected": {}}
            report = {"schema": "pd-lab.correction-cap-probe-results.v1", "main_source_after": copy.deepcopy(source),
                      "protected_after": {}, "stages": [{"cap": 12, "rows": rows}], "stopped_reason": None, "measured_attempts": 6}
            if mutate:
                mutate(manifest, report)
            for row in rows:
                p.study.write_new(stage / "ledger" / (row["attempt_id"] + ".json"), p.study.encoded(row))
            p.study.write_new(stage / "run-start.json", p.study.encoded(frozen))
            report["manifest_sha256"] = p.study.digest(p.study.encoded(manifest))
            p.study.write_new(root / "manifest.json", p.study.encoded(manifest))
            p.study.write_new(root / "cap-probe.json", p.study.encoded(report))
            p.study.write_new(root / "receipt.json", p.study.encoded({"files": p.survey.inventory(root)}))
            return plan

        mutations = [None,
                     lambda m, r: r.update(measured_attempts=7),
                     lambda m, r: r["main_source_after"].update(production_executable_sha256="changed"),
                     lambda m, r: r["stages"][0]["rows"][2].update(result={"planning_stop": "correction_limit"}),
                     lambda m, r: r["stages"][0]["rows"].reverse()]
        for mutate in mutations:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                plan = capsule(root, mutate)
                before = p.survey.inventory(root)
                with patch("cap_probe.contract", return_value=plan), \
                        patch("cap_probe.transformed_policy", return_value=b"synthetic transformed"), \
                        patch("cap_probe.check_record", return_value={}), \
                        patch("cap_probe.subprocess.run", side_effect=AssertionError("execution forbidden")), \
                        patch("cap_probe.study.write_new", side_effect=AssertionError("write forbidden")):
                    if mutate:
                        with self.assertRaises(ValueError):
                            p.verify(root)
                    else:
                        self.assertEqual(p.verify(root)["measured_attempts"], 6)
                self.assertEqual(before, p.survey.inventory(root))


if __name__ == "__main__":
    unittest.main()
