"""Pure contract/comparison tests; never start a diagnostic mission."""

import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import early_exit as p


class EarlyExitTests(unittest.TestCase):
    def test_frozen_nine_attempt_order_and_no_extra_retry(self):
        plan = p.contract()
        rows = p.attempts(plan)
        self.assertEqual(len(rows), 9)
        self.assertEqual([r["case_id"] for r in rows[:7]], plan["primary_order"])
        self.assertEqual([r["attempt_id"] for r in rows[7:]], ["repeat-random-967", "repeat-random-983"])
        with tempfile.TemporaryDirectory() as tmp:
            changed = dict(plan, maximum_measured_missions=10)
            path = Path(tmp) / "changed.json"
            p.study.write_new(path, p.study.encoded(changed))
            with self.assertRaises(ValueError):
                p.contract(path)

    def test_only_declared_generated_policy_transformation(self):
        data = (p.survey.REPO / p.cap.POLICY_PATH).read_bytes()
        transformed = p.transform(data, p.contract())
        self.assertIn(b'"piecewise_local_clearing_v2_policy_3_early_exit_probe_24";', transformed)
        self.assertIn(b"maximum_corrections: 24", transformed)
        self.assertNotEqual(data, transformed)
        with self.assertRaises(ValueError):
            p.transform(data + b"\n", p.contract())

    def test_fallback_comparison_omits_only_root_ids_wall_times_and_optional_query(self):
        previous = {"policy": {"old": True}, "input_identity": "old", "timings": {"planning_s": 1.0, "execution_s": 1.0, "replay_s": 1.0},
                    "cycles": [{"local_search": {"selected": {"fuel": 42.0}}}], "segments": [{"end": 12}], "ordinary_flight": {"actions": [1, 2]}}
        actual = copy.deepcopy(previous)
        actual["policy"] = {"new": True}
        actual["input_identity"] = "new"
        actual["timings"]["planning_s"] = 2.0
        actual["cycles"][0]["local_search"]["early_exit"] = {"disposition": "terrain_blocked"}
        p.unchanged(actual, previous)
        for obj in ("segments", "ordinary_flight"):
            changed = copy.deepcopy(actual)
            changed[obj] = None
            with self.assertRaises(ValueError):
                p.unchanged(changed, previous)
        actual["cycles"][0]["local_search"]["selected"]["fuel"] -= 1.0
        with self.assertRaises(ValueError):
            p.unchanged(actual, previous)

    def test_saved_verifier_does_not_spawn_native_processes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            p.study.write_new(root / "plan.json", p.PLAN.read_bytes())
            # A missing receipt is rejected without invoking any native tool.
            with patch.object(p.subprocess, "run", side_effect=AssertionError("read-only verify executed")):
                with self.assertRaises(OSError):
                    p.verify(root)

    def test_complete_saved_verifier_accepts_partial_capsule_without_live_source_or_processes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = copy.deepcopy(p.contract())
            p.study.write_new(root / "plan.json", p.PLAN.read_bytes())
            files = {}
            for path in (str(p.PLAN.relative_to(p.survey.REPO)), p.cap.POLICY_PATH):
                data = (p.survey.REPO / path).read_bytes()
                files[path] = p.study.digest(data)
                p.study.write_new(root / "inputs/source" / path, data)
                p.study.write_new(root / "variant/source" / path, p.transform(data, plan) if path == p.cap.POLICY_PATH else data)
            p.study.write_new(root / "variant/bin/pd-eval", b"synthetic never-executed binary")
            seal = p.cap.variant_state(root / "variant", files)
            p.study.write_new(root / "variant/source-seal.json", p.study.encoded(seal))
            refs = {}
            for name, key in (("manifest.json", "handoff_probe_manifest_sha256"), ("receipt.json", "handoff_probe_receipt_sha256")):
                data = p.study.encoded({"files": {}})
                p.study.write_new(root / "references" / name, data)
                refs[name] = plan[key] = p.study.digest(data)
            source = {"files": files, "production_executable_sha256": plan["production_executable_sha256"]}
            manifest = {"source": source, "protected": {}, "references": refs}
            p.study.write_new(root / "manifest.json", p.study.encoded(manifest))
            report = {"schema": "pd-lab.terrain-early-exit-results.v1", "manifest_sha256": p.study.digest(p.study.encoded(manifest)),
                      "source_after": source, "protected_after": {}, "variant_after": seal, "rows": p.attempts(plan),
                      "measured_missions": 0, "stopped_reason": "synthetic stop before measurement"}
            p.study.write_new(root / "early-exit.json", p.study.encoded(report))
            p.study.write_new(root / "receipt.json", p.study.encoded({"files": p.survey.inventory(root)}))
            with patch.object(p, "contract", return_value=plan), patch.object(p.subprocess, "run", side_effect=AssertionError("saved verifier executed")):
                self.assertEqual(p.verify(root)["measured_missions"], 0)


if __name__ == "__main__":
    unittest.main()
