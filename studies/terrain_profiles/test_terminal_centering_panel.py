import copy
import tempfile
import unittest
from pathlib import Path

import ballistic_feedback_sweep as sweep
import study
import terminal_centering_panel as panel


class CenteringPanelTests(unittest.TestCase):
    def rows(self, full=False):
        primary, repeats = panel.previous.layout(True) if full else panel.braking.focus_layout()
        return [{"case_id": name, "attempt_id": name, "status": "recorded",
                 "cohort": "random" if name.startswith("random-") else "control",
                 "previous_result": {"verified_landing": name != "random-142"},
                 "result": {"verified_landing": True, "physical_outcome": "landed_on_target", "handoffs": 1}}
                for name in primary] + [
                    {"case_id": name, "attempt_id": "repeat-" + name, "status": "recorded", "cohort": "repeat",
                     "result": {"verified_landing": True, "physical_outcome": "landed_on_target", "handoffs": 1}}
                    for name in repeats]

    def test_fixed_allowance_and_failed_subject_gate(self):
        fixed = panel.fixed_contract()
        self.assertEqual(sum(map(len, panel.braking.focus_layout()))
                         + sum(map(len, panel.previous.layout(True))) + fixed["maximum_measured_attempts"], 1027)
        self.assertEqual(fixed["repeat_indices"], [142, 974])
        self.assertNotEqual(panel.NATIVE, panel.braking.NATIVE)
        for full in (False, True):
            rows = self.rows(full)
            self.assertTrue(panel.gate(rows, full)["passed"])
            bad = copy.deepcopy(rows)
            next(r for r in bad if r["attempt_id"] == "random-142")["result"].update(
                verified_landing=False, physical_outcome="flying")
            self.assertFalse(panel.gate(bad, full)["passed"])
        bad = self.rows(True)
        next(r for r in bad if r["attempt_id"] == "random-084")["result"]["handoffs"] = 2
        self.assertFalse(panel.gate(bad, True)["passed"])

    def test_contract_preserves_worlds_bounds_and_opt_in_mode(self):
        plan = {**panel.fixed_contract(), **{name: "a" * 64 for name in panel.SHA_FIELDS}}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "plan.json"
            study.write_new(path, study.encoded(plan))
            self.assertEqual(panel.sweep_contract(path), plan)
            for i, (key, value) in enumerate((
                    ("candidate_id", sweep.COAST_CANDIDATE), ("previous_capture", "elsewhere"),
                    ("primary_cases", 999), ("repeat_indices", [142]), ("correction_cap", 48),
                    ("publication", True), ("retries", 1), ("candidate_rust_source_sha256", "not-a-digest"))):
                bad_path = Path(directory) / f"bad-{i}.json"
                study.write_new(bad_path, study.encoded({**plan, key: value}))
                with self.assertRaises(ValueError):
                    panel.sweep_contract(bad_path)

    def admission(self):
        source = {"executable_sha256": "a" * 64, "rust_source_tree_sha256": "b" * 64,
                  "renderer_sha256": "c" * 64, "files": {"pd-report/src/planning_cycles.js": "d" * 64}}
        plan = {**panel.fixed_contract(), "candidate_executable_sha256": "a" * 64,
                "candidate_rust_source_sha256": "b" * 64, "candidate_renderer_sha256": "c" * 64,
                "candidate_report_script_sha256": "d" * 64, "experiment_plan_sha256": "e" * 64}
        values = {}
        for kind in ("focus", "panel"):
            identity = "terminal-centering-preservation" if kind == "panel" else "terminal-centering-focus"
            report = {"schema": f"pd-lab.{identity}-panel.v1", "stopped_reason": None,
                      "source_after": source, "rows": self.rows(kind == "panel")}
            results, manifest = study.encoded(report), study.encoded({"source": source})
            receipt = study.encoded({"files": {"ballistic-feedback-sweep.json": study.digest(results),
                "manifest.json": study.digest(manifest), "experiment-plan.md": "e" * 64}})
            values[kind] = [receipt, results, manifest]
            for data, name in zip(values[kind], ("receipt", "results", "manifest")):
                plan[f"gate_{kind}_{name}_sha256"] = study.digest(data)
        return plan, values, source

    def test_portable_admission_binds_both_stages_and_candidate(self):
        plan, values, source = self.admission()
        self.assertEqual(panel.verify_gate_data(plan, values), source)
        for key in panel.SHA_FIELDS:
            with self.assertRaises(ValueError):
                panel.verify_gate_data({**plan, key: "f" * 64}, values)

    def test_centering_metadata_keeps_authority_and_guards(self):
        metadata = {"enabled": True, "rule": "rotated_hull_and_feet_rescue_pad_interval",
                    "reuse_outside_pad_lateral_target": True, "vertical_authority_cap_unchanged": True,
                    "standalone_coast_terminal_changed": False, "ordinary_default_changed": False,
                    "short_command_guard_changed": False}
        sweep.validate_landing_body_centering({"landing_body_centering": metadata}, sweep.CENTERING_CANDIDATE)
        for key in ("vertical_authority_cap_unchanged", "ordinary_default_changed", "short_command_guard_changed"):
            with self.assertRaises(ValueError):
                sweep.validate_landing_body_centering({"landing_body_centering": {**metadata, key: not metadata[key]}}, sweep.CENTERING_CANDIDATE)
        with self.assertRaises(ValueError):
            sweep.validate_landing_body_centering({"landing_body_centering": metadata}, sweep.BRAKING_CANDIDATE)
        sweep.validate_landing_body_centering({}, sweep.BRAKING_CANDIDATE)


if __name__ == "__main__":
    unittest.main()
