"""Survey unit tests are synthetic and do not spend the measured flight budget."""

import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess

import survey


class SurveyTests(unittest.TestCase):
    def setUp(self):
        self.plan, self.refinement, self.base = survey.load_plan()

    def test_seed_selection_is_unique_reproducible_and_excludes_inspected_seeds(self):
        seeds = survey.seeds_for(self.plan)
        self.assertEqual(seeds, survey.seeds_for(self.plan))
        self.assertEqual(len(set(seeds)), 100)
        self.assertFalse(set(seeds) & set(self.base["seeds"]))
        self.assertTrue(all(0 <= s <= self.plan["seed_max"] for s in seeds))

    def test_frozen_allowance_cannot_silently_expand(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "plan.json"
            plan = dict(self.plan, maximum_measured_attempts=109)
            survey.study.write_new(path, survey.study.encoded(plan))
            with self.assertRaises(ValueError):
                survey.load_plan(path)

    def test_preparation_keeps_native_string_metadata_and_only_local_pad_changes(self):
        profile = {"variant": "refined", "seed": 987,
                   "points_m": [[x, x * 0.01] for x in range(-160, 1361, 4)]}
        value, geometry = survey.prepared(profile, self.plan, self.refinement, self.base)
        template = json.loads((survey.HERE / "scenario_template.json").read_bytes())
        self.assertTrue(all(isinstance(v, str) for v in value["metadata"].values()))
        for key in ("vehicle", "sim", "mission"):
            self.assertEqual(value[key], template[key])
        self.assertEqual(geometry["terrain_point_count"], 389)
        self.assertTrue(geometry["outside_patches_unchanged"])
        self.assertEqual(value["initial_state"]["position_m"], {"x": 0, "y": 5})

    def flight(self):
        return {"cycles": [], "initial_nominal_terrain_blocked": False, "planning_stop": "no_nominal",
                "physical_outcome": "flying", "mission_outcome": "in_progress", "correction_count": 0,
                "integrity_passed": True, "final_source_replay_passed": True,
                "timings": {"planning_s": 1, "execution_s": 2, "replay_s": 3}}

    def test_no_nominal_does_not_count_as_a_clear_route(self):
        projected = survey.projection(self.flight())
        self.assertEqual(projected["nominal_class"], "not_established")
        self.assertFalse(projected["verified_landing"])

    def test_clear_blocked_and_rejected_audits_are_distinct(self):
        flight = self.flight()
        flight["cycles"] = [{"audit": {"passed": True}}]
        self.assertEqual(survey.projection(flight)["nominal_class"], "clear")
        flight["cycles"][0]["audit"]["passed"] = False
        self.assertEqual(survey.projection(flight)["nominal_class"], "rejected")
        flight["initial_nominal_terrain_blocked"] = True
        self.assertEqual(survey.projection(flight)["nominal_class"], "blocked")

    def test_landing_requires_the_complete_tuple(self):
        flight = dict(self.flight(), planning_stop="landed", physical_outcome="landed_on_target", mission_outcome="success")
        self.assertTrue(survey.projection(flight)["verified_landing"])
        for key in ("integrity_passed", "final_source_replay_passed"):
            self.assertFalse(survey.projection(dict(flight, **{key: False}))["verified_landing"])

    def test_repeat_excludes_only_declared_flight_wall_timings(self):
        first = self.flight()
        second = copy.deepcopy(first)
        second["timings"]["planning_s"] = 55
        self.assertEqual(survey.non_timing(first), survey.non_timing(second))
        second["correction_count"] = 1
        self.assertNotEqual(survey.non_timing(first), survey.non_timing(second))
        second["timings"]["fuel"] = 1
        with self.assertRaises(ValueError):
            survey.non_timing(second)

    def test_totals_keep_misses_unattempted_and_preservation_cohorts(self):
        rows = [{"cohort": "random", "status": "recorded", "result": survey.projection(self.flight())},
                {"cohort": "random", "status": "not_attempted"},
                {"cohort": "sentinel", "status": "recorded"}, {"cohort": "repeat", "status": "recorded"}]
        total = survey.summary_for(rows)
        self.assertEqual((total["primary_count"], total["recorded_count"], total["verified_landings"]), (2, 1, 0))
        self.assertEqual(total["nominal_classes"], {"not_established": 1})
        self.assertEqual(total["dispositions"], {"not_attempted": 1, "recorded": 1})

    def test_hash_checker_rejects_changed_files_and_path_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            survey.study.write_new(root / "a.json", b"{}")
            survey.check_inventory(root, {"a.json": survey.study.digest(b"{}")})
            with self.assertRaises(ValueError):
                survey.check_inventory(root, {"a.json": "forged"})
            with self.assertRaises(ValueError):
                survey.check_inventory(root, {"../outside": "forged"})

    def test_runner_timeout_retains_partial_logs_without_claiming_a_flight(self):
        row = {"attempt_id": "random-000", "case_id": "random-000", "cohort": "random",
               "scenario_path": "scenarios/random-000.json"}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch("survey.subprocess.run", side_effect=subprocess.TimeoutExpired("synthetic", 1, output=b"partial")):
                result = survey.attempt(Path("synthetic-never-executed"), root, row, 1)
            self.assertEqual(result["status"], "runner_timeout")
            self.assertIsNone(result["result"])
            self.assertEqual((root / "logs/random-000.stdout").read_bytes(), b"partial")

    def test_spawn_error_is_accounted_and_existing_logs_are_create_only(self):
        row = {"attempt_id": "random-000", "case_id": "random-000", "cohort": "random",
               "scenario_path": "scenarios/random-000.json"}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch("survey.subprocess.run", side_effect=OSError("synthetic spawn error")):
                result = survey.attempt(Path("synthetic-never-executed"), root, row, 1)
                self.assertEqual(result["status"], "runner_error")
                with self.assertRaises(FileExistsError):
                    survey.attempt(Path("synthetic-never-executed"), root, row, 1)

    def test_sentinel_runner_records_exceptions_and_repeat_runner_remains_exact(self):
        from test_sentinel_comparison import edited
        corpus = json.loads((survey.HERE / "sentinel_comparison_cases.json").read_bytes())
        baseline = dict(corpus["baseline"], reason="synthetic", planning_stop="landed",
                        physical_outcome="landed_on_target", mission_outcome="success",
                        correction_count=0, initial_nominal_terrain_blocked=False,
                        input_identity="synthetic", policy={"policy_id":"piecewise_local_clearing_v2_policy_3"})
        case = next(c for c in corpus["cases"] if c["name"] == "observed-clearance-roundoff")
        actual = edited(baseline, case["edits"])
        keys = ("planning_stop", "reason", "correction_count", "initial_nominal_terrain_blocked", "integrity_passed",
                "physical_outcome", "mission_outcome", "final_source_replay_passed", "timings")
        compact = {"schema_id":"waypoint_v2_flight_summary_v1", "input_identity":"synthetic", "policy":actual["policy"],
                   "result":{k:actual[k] for k in keys}}
        for cohort in ("sentinel", "repeat"):
            with self.subTest(cohort=cohort), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                attempt_id = "control" if cohort == "sentinel" else "repeat-000"
                reference = "controls/control.json" if cohort == "sentinel" else "runs/control/flight.json"
                survey.study.write_new(root / reference, survey.study.encoded(baseline))
                survey.study.write_new(root / f"runs/{attempt_id}/flight.json", survey.study.encoded(actual))
                survey.study.write_new(root / f"runs/{attempt_id}/summary.json", survey.study.encoded(compact))
                row = {"attempt_id":attempt_id, "case_id":"control", "cohort":cohort, "scenario_path":"synthetic.json"}
                result = subprocess.CompletedProcess("synthetic", 0, stdout=survey.study.encoded(actual), stderr=b"")
                with patch("survey.subprocess.run", return_value=result):
                    saved = survey.attempt(Path("synthetic-never-executed"), root, row, 1, survey.comparison.current_contract())
                self.assertEqual(saved["status"], "recorded" if cohort == "sentinel" else "evidence_error")
                if cohort == "sentinel":
                    self.assertEqual(len(saved["comparison_exceptions"]), 1)
                    self.assertTrue(saved["result"]["verified_landing"])

    def test_manifest_contract_is_source_bound_and_legacy_does_not_upgrade(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertIsNone(survey.input_comparison_contract(root, {}))
            contract = survey.comparison.current_contract()
            data = survey.comparison.CONTRACT_PATH.read_bytes()
            survey.study.write_new(root / "inputs/tools/sentinel_comparison.json", data)
            path = "studies/terrain_profiles/sentinel_comparison.json"
            manifest = {"sentinel_comparison":contract, "source":{"files":{path:survey.study.digest(data)}}}
            self.assertEqual(survey.input_comparison_contract(root, manifest), contract)
            for changed in (None, dict(contract, absolute_tolerance_m=1e-6)):
                with self.assertRaises(ValueError):
                    survey.input_comparison_contract(root, dict(manifest, sentinel_comparison=changed))
            manifest["source"]["files"][path] = "forged"
            with self.assertRaises(ValueError):
                survey.input_comparison_contract(root, manifest)

    def test_v2_manifest_is_separately_bound_and_uses_read_only_native_comparison(self):
        contract = survey.comparison.latest_contract()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = survey.comparison.LATEST_CONTRACT_PATH.read_bytes()
            survey.study.write_new(root / "inputs/tools/sentinel_comparison_v2.json", data)
            path = "studies/terrain_profiles/sentinel_comparison_v2.json"
            manifest = {"sentinel_comparison":contract, "source":{"files":{path:survey.study.digest(data)}}}
            self.assertEqual(survey.input_comparison_contract(root, manifest), contract)
            with patch("sentinel_comparison.subprocess.run", return_value=subprocess.CompletedProcess("synthetic", 0, stdout=b"[]", stderr=b"")) as run:
                self.assertEqual(survey.comparison.compare_files(root / "actual.json", root / "baseline.json", contract, Path("synthetic-never-executed")), [])
                command = run.call_args.args[0]
                self.assertEqual(command[1], "compare-terrain-flights")
                self.assertNotIn("waypoint-v2-flight", command)
            with patch("sentinel_comparison.subprocess.run", return_value=subprocess.CompletedProcess("synthetic", 1, stdout=b"", stderr=b"forged hash")):
                with self.assertRaisesRegex(ValueError, "forged hash"):
                    survey.comparison.compare_files(root / "actual.json", root / "baseline.json", contract)


if __name__ == "__main__":
    unittest.main()
