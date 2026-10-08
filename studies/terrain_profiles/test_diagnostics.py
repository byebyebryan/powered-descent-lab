"""Diagnostic contracts and synthetic records; no measured native flight runs."""

import argparse
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import diagnostics as d


def scenario():
    return {"initial_state": {"position_m": {"x": 0}},
            "world": {"landing_pads": [{"id": "target", "center_x_m": 1200, "surface_y_m": 10}]},
            "mission": {"goal": {"target_pad_id": "target"}}, "sim": {"physics_hz": 120}}


def state(x=0, tick=0):
    return {"physics_step": tick, "sim_time_s": tick / 120,
            "position_m": {"x": x, "y": 100}, "velocity_mps": {"x": 80, "y": -10}, "fuel_kg": 5000}


def cycle(x=0, tick=0):
    return {"current_state": state(x, tick), "audit": {"passed": True},
            "nominal_attempt_status_counts": {"nominal_proposal": 1},
            "nominal_rejection_reason_counts": {}, "local_search": None, "conflict_state": None}


def flight():
    return {"cycles": [cycle()], "input_identity": "synthetic",
            "policy": {"policy_id": "piecewise_local_clearing_v2_policy_3"},
            "reason": "synthetic", "initial_nominal_terrain_blocked": False,
            "absolute_deadline_physics_step": 9600,
            "planning_stop": "landed", "physical_outcome": "landed_on_target", "mission_outcome": "success",
            "correction_count": 0, "integrity_passed": True, "final_source_replay_passed": True,
            "timings": {"planning_s": 1, "execution_s": 2, "replay_s": 3}}


def compact(f):
    keys = ("planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
            "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings")
    return {"schema_id": "waypoint_v2_flight_summary_v1", "input_identity": f["input_identity"], "policy": f["policy"],
            "result": {k: f[k] for k in keys}}


def write(root, relative, value):
    d.study.write_new(root / relative, d.study.encoded(value))


class DiagnosticTests(unittest.TestCase):
    def test_fixed_selection_is_balanced_and_four_recipes_are_present(self):
        plan = d.load_contract()
        d.check_inputs(plan)
        self.assertEqual(len(plan["cases"]), 10)
        self.assertEqual({c["recipe_id"] for c in plan["cases"]},
                         {"mountains_4x", "mountains_8x", "broad_massifs_8x", "successive_ridges_8x"})
        self.assertEqual(sum(c["role"] == "failure" for c in plan["cases"]), 6)
        for c in plan["cases"]:
            d.check_family(c, c["baseline_result"], c["baseline_observations"])

    def test_attempts_keep_three_repeats_out_of_primary_denominator(self):
        rows = d.attempts(d.load_contract())
        self.assertEqual(len(rows), 13)
        self.assertEqual([r["case_id"] for r in rows[10:]], d.REPEATS)
        self.assertEqual(len({r["attempt_id"] for r in rows}), 13)
        self.assertEqual(d.survey.summary_for(rows)["primary_count"], 10)

    def test_contract_rejects_bound_selection_repeat_and_baseline_mutation(self):
        plan = d.load_contract()
        changed = []
        for key in ("maximum_measured_attempts", "case_wall_limit_s", "campaign_wall_limit_s", "workers", "policy_version"):
            changed.append(dict(plan, **{key: plan[key] + 1}))
        changed.append(dict(plan, repeat_case_ids=d.REPEATS[::-1]))
        changed.append(dict(plan, cases=plan["cases"][::-1]))
        changed.append(dict(plan, baseline=dict(plan["baseline"], receipt_sha256="0" * 64)))
        changed.append(dict(plan, cases=[dict(plan["cases"][0], scenario_path="../outside"), *plan["cases"][1:]]))
        for value in changed:
            with self.subTest(value=value.get("workers")), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "plan.json"
                d.study.write_new(path, d.study.encoded(value))
                with self.assertRaises(ValueError):
                    d.load_contract(path)

    def test_fixture_hash_and_seed_labels_cannot_change(self):
        plan = d.load_contract()
        for key, value in (("scenario_sha256", "0" * 64), ("seed", 1)):
            altered = copy.deepcopy(plan)
            altered["cases"][0][key] = value
            with self.assertRaises(ValueError):
                d.check_inputs(altered)

    def test_fingerprint_excludes_only_three_finite_wall_timings(self):
        f = flight()
        changed = copy.deepcopy(f)
        changed["timings"]["planning_s"] = 99
        self.assertEqual(d.fingerprint(f), d.fingerprint(changed))
        for key in ("integrity_passed", "correction_count", "absolute_deadline_physics_step"):
            changed = copy.deepcopy(f)
            changed[key] = 999
            self.assertNotEqual(d.fingerprint(f), d.fingerprint(changed))
        changed = copy.deepcopy(f)
        changed["cycles"][0]["nominal_rejection_reason_counts"]["extra diagnostic"] = 1
        self.assertNotEqual(d.fingerprint(f), d.fingerprint(changed))
        changed["timings"]["query_s"] = 1
        with self.assertRaises(ValueError):
            d.fingerprint(changed)

    def test_handoff_observations_use_h_not_certificate_end(self):
        f = flight()
        h, certificate = state(200, 120), state(400, 360)
        f["cycles"][0]["local_search"] = {
            "selected": {"row_id": "chosen", "handoff_state": h, "continuation_end_state": certificate},
            "row_diagnostics": [{"row_id": "chosen", "eligible_handoff": {"braking_room": {"remaining_room_m": 7}}}]}
        f["cycles"].append(cycle(200, 120))
        f["cycles"][1]["conflict_state"] = state(250, 240)
        observed = d.observations(f, scenario())
        self.assertEqual(observed["handoffs"][0]["x_m"], 200)
        self.assertEqual(observed["handoffs"][0]["height_above_target_m"], 90)
        self.assertEqual(observed["handoffs"][0]["next_conflict_after_h_s"], 1)
        self.assertEqual(observed["handoffs"][0]["braking_room_m"], 7)
        self.assertEqual(observed["remaining_deadline_s"], 79)

    def test_supported_progress_counts_all_continuation_statuses(self):
        f = flight()
        f["cycles"][0]["local_search"] = {
            "selected": None, "entries": [1], "row_diagnostics": [{"physically_propagated": True}],
            "accepted_row_count": 0, "row_stop_reason_counts": {"physical_trace": 1},
            "boundary_status_counts": {"insufficient_progress": 10, "handoff_unsupported": 3,
                                       "unsafe_continuation": 2, "continuation_unsupported": 4, "eligible": 1}}
        observed = d.observations(f, scenario())
        self.assertEqual(observed["last_local_supported_progress_count"], 7)
        self.assertEqual(observed["last_local_propagated_row_count"], 1)

    def test_early_exit_observations_use_actual_end_and_do_not_reuse_witness_room(self):
        h, early = state(200, 120), state(160, 100)
        early["velocity_mps"] = {"x": 70, "y": 15}
        for disposition, expected, room in (("committed", early, None), ("terrain_blocked", h, 7)):
            with self.subTest(disposition=disposition):
                f = flight()
                f["cycles"][0]["local_search"] = {
                    "selected": {"row_id": "chosen", "handoff_state": h},
                    "early_exit": {"disposition": disposition, "query_state": early},
                    "row_diagnostics": [{"row_id": "chosen", "eligible_handoff": {
                        "braking_room": {"remaining_room_m": 7}}}]}
                f["cycles"].append(cycle(expected["position_m"]["x"], expected["physics_step"]))
                f["cycles"][1]["conflict_state"] = state(250, 240)
                before = copy.deepcopy(f)
                observed = d.observations(f, scenario())["handoffs"][0]
                self.assertEqual(observed["physics_step"], expected["physics_step"])
                self.assertEqual(observed["x_m"], expected["position_m"]["x"])
                self.assertEqual(observed["vx_mps"], expected["velocity_mps"]["x"])
                self.assertEqual(observed["vy_mps"], expected["velocity_mps"]["y"])
                self.assertEqual(observed["next_conflict_after_h_s"],
                                 f["cycles"][1]["conflict_state"]["sim_time_s"] - expected["sim_time_s"])
                self.assertEqual(observed["braking_room_m"], room)
                self.assertEqual(f, before)

    def test_family_guards_reject_misclassification_without_proving_impossibility(self):
        plan = d.load_contract()
        cases = {c["case_id"]: c for c in plan["cases"]}
        for cid, key, value in (("random-327", "first_conflict_x_m", 50),
                                ("random-791", "last_local_supported_progress_count", 1),
                                ("random-024", "last_nominal_physical_attempt_count", 1),
                                ("random-030", "final_cycle_x_m", 900)):
            c = cases[cid]
            with self.assertRaises(ValueError):
                d.check_family(c, c["baseline_result"], dict(c["baseline_observations"], **{key: value}))
        # Six corrections can land; do not define every long sequence as failure.
        c = cases["random-253"]
        d.check_family(c, c["baseline_result"], c["baseline_observations"])

    def test_success_requires_every_landing_tuple_field(self):
        c = next(c for c in d.load_contract()["cases"] if c["case_id"] == "random-000")
        for key in ("integrity_passed", "final_source_replay_passed", "mission_outcome", "physical_outcome", "planning_stop"):
            f = flight()
            f[key] = False if key.endswith("passed") else "not_landed"
            with self.assertRaises(ValueError):
                d.check_family(c, d.survey.projection(f), c["baseline_observations"])

    def test_safe_paths_reject_parent_absolute_and_symlink_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "escape").symlink_to(root.parent, target_is_directory=True)
            for path in ("../outside", "/tmp/outside", "escape/outside"):
                with self.assertRaises(ValueError):
                    d.safe_file(root, path)

    def record(self, root):
        f, s = flight(), scenario()
        c = {"case_id": "random-000", "family": "direct", "role": "comparison", "scenario_path": "scenario.json",
             "baseline_non_timing_sha256": d.fingerprint(f), "baseline_result": d.survey.projection(f),
             "baseline_observations": d.observations(f, s)}
        row = dict(c, attempt_id="random-000", cohort="random", output_dir="runs/random-000",
                   result=d.survey.projection(f), exit_code=0)
        for path, value in (("scenario.json", s), ("runs/random-000/scenario.json", s),
                            ("runs/random-000/flight.json", f), ("runs/random-000/summary.json", compact(f)),
                            ("logs/random-000.stdout", f)):
            write(root, path, value)
        return row, c

    def test_complete_saved_record_and_output_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row, c = self.record(root)
            self.assertTrue(d.check_record(root, row, c)["baseline_exact_non_timing"])
            for edited in (dict(row, output_dir="runs/other"), dict(row, exit_code=1),
                           dict(row, result=dict(row["result"], verified_landing=False))):
                with self.assertRaises(ValueError):
                    d.check_record(root, edited, c)

    def test_candidate_can_differ_but_not_regress_comparison_or_forge_repeat(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row, c = self.record(root)
            c["baseline_non_timing_sha256"] = "0" * 64
            with self.assertRaises(ValueError):
                d.check_record(root, row, c)
            self.assertFalse(d.check_record(root, row, c, candidate=True)["baseline_exact_non_timing"])
            # Source records stay untouched; manipulate expectations, not files.
            failure = dict(c, role="failure", family="acquisition")
            d.check_record(root, row, failure, candidate=True)
            with patch("diagnostics.survey.validate_record", return_value=dict(row["result"], verified_landing=False)):
                edited = dict(row, result=dict(row["result"], verified_landing=False))
                with self.assertRaisesRegex(ValueError, "regressed"):
                    d.check_record(root, edited, c, candidate=True)
            repeated = dict(row, cohort="repeat")
            with patch("diagnostics.survey.comparison.compare", side_effect=ValueError("repeat differs")):
                with self.assertRaisesRegex(ValueError, "repeat differs"):
                    d.check_record(root, repeated, c, candidate=True)

    def test_wrong_baseline_binary_rejected_before_output_or_flights(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = argparse.Namespace(binary=Path("synthetic-never-executed"), output=Path(tmp) / "capture", candidate=False)
            with patch("diagnostics.source_state", return_value={"executable_sha256": "wrong"}), \
                    patch("diagnostics.survey.protected_state", return_value={}), \
                    patch("diagnostics.survey.attempt") as attempt:
                with self.assertRaisesRegex(ValueError, "baseline executable"):
                    d.run(args)
                attempt.assert_not_called()
                self.assertFalse(args.output.exists())

    def test_reservation_is_create_only_and_outside_root_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch("diagnostics.survey.OUTPUTS", root):
                d.survey.reserve(root / "capture")
                with self.assertRaises(FileExistsError):
                    d.survey.reserve(root / "capture")
                with self.assertRaises(ValueError):
                    d.survey.reserve(root.parent / "not-in-allowance")

    def synthetic_capture(self, root, mutate=None):
        """Exercise capture wiring only; check_record is mocked, not simulated."""
        plan = d.load_contract()
        files = {str(d.PLAN.relative_to(d.survey.REPO)): d.study.digest(d.PLAN.read_bytes())}
        files.update({c["scenario_path"]: c["scenario_sha256"] for c in plan["cases"]})
        source = {"git_commit": "synthetic", "executable_sha256": plan["baseline"]["executable_sha256"], "files": files}
        manifest = {"schema": "pd-lab.terrain-diagnostics-inputs.v1", "mode": "baseline",
                    "source": source, "protected": {}, "cases": plan["cases"]}
        rows = [dict(r, status="recorded", result=r["baseline_result"], exit_code=0,
                     output_dir="runs/" + r["attempt_id"]) for r in d.attempts(plan)]
        report = {"schema": "pd-lab.terrain-diagnostics-results.v1", "source_before": source,
                  "source_after": copy.deepcopy(source), "protected_after": {}, "stopped_reason": None,
                  "rows": rows, "observations": {r["attempt_id"]: {} for r in rows},
                  "summary": d.survey.summary_for(rows)}
        if mutate:
            mutate(manifest, report)
        report["manifest_sha256"] = d.study.digest(d.study.encoded(manifest))
        for relative in files:
            d.study.write_new(root / "inputs/source" / relative, (d.survey.REPO / relative).read_bytes())
        d.study.write_new(root / "plan.json", d.PLAN.read_bytes())
        for c in plan["cases"]:
            d.study.write_new(root / c["scenario_path"], (d.survey.REPO / c["scenario_path"]).read_bytes())
            write(root, "preflight/" + c["case_id"] + ".json",
                  {"supported": True, "rejection": None, "reason": None, "simulation_created": False})
        for row in rows:
            write(root, "ledger/" + row["attempt_id"] + ".json", row)
        write(root, "manifest.json", manifest)
        write(root, "run-start.json", {"attempts": d.attempts(plan)})
        write(root, "diagnostics.json", report)
        write(root, "receipt.json", {"files": d.survey.inventory(root)})

    def test_saved_verifier_checks_receipt_source_identity_order_and_projection(self):
        mutations = [
            lambda m, r: m.update(mode="unknown"),
            lambda m, r: r["source_after"].update(executable_sha256="wrong"),
            lambda m, r: r["source_after"]["files"].update({"additional.rs": "wrong"}),
            lambda m, r: r["rows"].reverse(),
            lambda m, r: r["rows"][0].update(status="not_attempted"),
            lambda m, r: r["observations"].pop("random-327"),
            lambda m, r: r["summary"].update(verified_landings=10),
            lambda m, r: r.update(protected_after={"accepted": "changed"}),
        ]
        with patch("diagnostics.check_record", return_value={}):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                self.synthetic_capture(root)
                self.assertEqual(len(d.verify(root)["rows"]), 13)
                write(root, "unreceipted.json", {})
                with self.assertRaisesRegex(ValueError, "inventory"):
                    d.verify(root)
            for mutate in mutations:
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    self.synthetic_capture(root, mutate)
                    with self.assertRaises(ValueError):
                        d.verify(root)

    def test_saved_verification_never_executes_or_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.synthetic_capture(root)
            before = d.survey.inventory(root)
            with patch("diagnostics.check_record", return_value={}), \
                    patch("diagnostics.survey.subprocess.run", side_effect=AssertionError("execution forbidden")), \
                    patch("diagnostics.study.write_new", side_effect=AssertionError("write forbidden")):
                d.verify(root)
            self.assertEqual(before, d.survey.inventory(root))

    def test_collector_failure_stops_without_retry_and_retains_partial_receipt(self):
        plan = d.load_contract()
        files = {str(d.PLAN.relative_to(d.survey.REPO)): d.study.digest(d.PLAN.read_bytes())}
        files.update({c["scenario_path"]: c["scenario_sha256"] for c in plan["cases"]})
        source = {"executable_sha256": plan["baseline"]["executable_sha256"], "files": files}

        def failed(binary, root, row, limit):
            return dict(row, status="runner_error", result=None, failure="synthetic spawn failure",
                        exit_code=1, output_dir="runs/" + row["attempt_id"])

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            args = argparse.Namespace(binary=root / "never-executed", output=root / "capture", candidate=False)
            with patch("diagnostics.source_state", return_value=source), \
                    patch("diagnostics.survey.protected_state", return_value={}), \
                    patch("diagnostics.survey.OUTPUTS", root), \
                    patch("diagnostics.survey.preflight", return_value={"supported": True, "rejection": None, "reason": None, "simulation_created": False}), \
                    patch("diagnostics.survey.attempt", side_effect=failed) as attempt, \
                    patch("builtins.print"):
                with self.assertRaisesRegex(RuntimeError, "stopped; evidence retained"):
                    d.run(args)
            self.assertEqual(attempt.call_count, 1)
            report = d.verify(args.output)
            self.assertEqual(len(report["rows"]), 13)
            self.assertEqual(report["summary"]["recorded_count"], 0)
            self.assertEqual(report["rows"][0]["status"], "runner_error")
            self.assertTrue(all(r["status"] == "not_attempted" for r in report["rows"][1:]))
            self.assertTrue((args.output / "receipt.json").exists())


if __name__ == "__main__":
    unittest.main()
