"""Pure/synthetic diagnostic checks. Never launch a measured probe or flight."""

import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import handoff_probe as p


def result(row, disposition="no_nominal"):
    spec = row["spec"]
    feasible = disposition != "no_nominal"
    return {"schema": "pd-lab.terrain-handoff-probe.v1", "request": spec,
            "evidence_kind": "counterfactual_query", "executed_mission_landing_claimed": False,
            "prefix_source_replay_passed": True, "probe_state": {"physics_step": spec["physics_step"],
                "sim_time_s": spec["physics_step"] / 120}, "absolute_deadline_physics_step": 9600,
            "disposition": disposition, "original_result_matched": spec["arm"] == "original",
            "production_boundary_admitted": spec["arm"] != "early",
            "nominal_proposal_identity": "synthetic" if feasible else None,
            "audit_passed": disposition == "nominal_clear" if feasible else None,
            "first_terrain_violation": {"physics_step": 2} if disposition == "nominal_terrain_blocked" else None}


class HandoffProbeTests(unittest.TestCase):
    def test_fixed_nineteen_queries_and_only_two_declared_repeats(self):
        plan = p.contract()
        rows = p.attempts(plan)
        self.assertEqual(len(rows), 19)
        self.assertEqual([r["spec"]["arm"] for r in rows[:7]], ["original"] * 7)
        self.assertEqual([r["attempt_id"] for r in rows[-2:]], ["repeat-random-967-early", "repeat-random-983-early"])
        self.assertEqual(len({r["attempt_id"] for r in rows}), 19)
        for changes in ({"maximum_probes": 20}, {"repeats": plan["repeats"][::-1]},
                        {"cases": plan["cases"][::-1]}, {"baseline_manifest_sha256": "0" * 64}):
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "plan.json"
                p.study.write_new(path, p.study.encoded(dict(plan, **changes)))
                with self.assertRaises(ValueError):
                    p.contract(path)

    def test_query_classification_does_not_claim_executed_landings(self):
        rows = p.attempts(p.contract())
        for row in rows:
            disposition = "nominal_clear" if row["role"] == "control" else "no_nominal"
            self.assertEqual(p.validate_probe(result(row, disposition), row)["disposition"], disposition)
        early = rows[7]
        for disposition in p.DISPOSITIONS:
            p.validate_probe(result(early, disposition), early)
        for changes in ({"executed_mission_landing_claimed": True}, {"production_boundary_admitted": True},
                        {"prefix_source_replay_passed": False}, {"absolute_deadline_physics_step": 9602},
                        {"audit_passed": False, "disposition": "nominal_clear"},
                        {"first_terrain_violation": {"physics_step": 5}}):
            with self.assertRaises(ValueError):
                p.validate_probe(dict(result(early), **changes), early)
        with self.assertRaises(ValueError):
            p.validate_probe(result(rows[0], "nominal_clear"), rows[0])

    def test_saved_partial_verification_authenticates_source_binary_inputs_and_never_runs(self):
        def capsule(root, mutate=None):
            plan = copy.deepcopy(p.contract())
            plan_bytes = p.study.encoded(plan)
            source = {"files": {str(p.PLAN.relative_to(p.survey.REPO)): p.study.digest(plan_bytes)},
                      "executable_sha256": p.study.digest(b"synthetic never-executed binary")}
            p.study.write_new(root / "inputs/source" / p.PLAN.relative_to(p.survey.REPO), plan_bytes)
            p.study.write_new(root / "inputs/bin/pd-eval", b"synthetic never-executed binary")
            for case in plan["cases"]:
                for name, key in (("scenario.json", "scenario_sha256"), ("flight.json", "flight_sha256")):
                    data = p.study.encoded({"synthetic": case["case_id"], "kind": name})
                    case[key] = p.study.digest(data)
                    p.study.write_new(root / "inputs" / case["case_id"] / name, data)
            for name, key in (("manifest.json", "baseline_manifest_sha256"), ("receipt.json", "baseline_receipt_sha256")):
                p.study.write_new(root / "baseline" / name, b"{}")
                plan[key] = p.study.digest(b"{}")
            # Tests use a synthetic contract instead of rewriting any real source/input.
            test_plan = root / "synthetic-plan.json"
            p.study.write_new(test_plan, plan_bytes)
            p.study.write_new(root / "plan.json", plan_bytes)
            rows = p.attempts(plan)
            p.study.write_new(root / "run-start.json", p.study.encoded(rows))
            for row in rows:
                p.study.write_new(root / "specs" / (row["attempt_id"] + ".json"), p.study.encoded(row["spec"]))
            manifest = {"schema": "pd-lab.terrain-handoff-probe-inputs.v1", "source": source, "protected": {}}
            report = {"schema": "pd-lab.terrain-handoff-probe-results.v1", "manifest_sha256": p.study.digest(p.study.encoded(manifest)),
                      "source_after": copy.deepcopy(source), "protected_after": {}, "rows": rows,
                      "stopped_reason": "synthetic stop before any query", "measured_probes": 0, "summary": p.summary(rows)}
            if mutate:
                mutate(manifest, report)
            p.study.write_new(root / "manifest.json", p.study.encoded(manifest))
            p.study.write_new(root / "handoff-probes.json", p.study.encoded(report))
            p.study.write_new(root / "receipt.json", p.study.encoded({"files": p.survey.inventory(root)}))
            return plan, test_plan

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan, path = capsule(root)
            # PLAN.relative_to(REPO) remains the source-bound real path, while
            # read_bytes is supplied from the synthetic fixture for this check.
            with patch.object(p, "contract", return_value=plan), patch.object(Path, "read_bytes", autospec=True) as read:
                real_read = Path.open
                def bytes_for(path_arg):
                    if path_arg == p.PLAN:
                        with real_read(path, "rb") as stream:
                            return stream.read()
                    with real_read(path_arg, "rb") as stream:
                        return stream.read()
                read.side_effect = bytes_for
                with patch.object(p.subprocess, "run", side_effect=AssertionError("saved verify must never execute")):
                    self.assertEqual(p.verify(root)["measured_probes"], 0)
        for mutation in (lambda m, r: r["source_after"].update(executable_sha256="changed"),
                         lambda m, r: r["protected_after"].update(report="changed"),
                         lambda m, r: r.update(measured_probes=20),
                         lambda m, r: r["rows"][0].update(status="invented")):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                plan, path = capsule(root, mutation)
                with patch.object(p, "contract", return_value=plan), patch.object(Path, "read_bytes", autospec=True) as read:
                    real_read = Path.open
                    def bytes_for(path_arg):
                        with real_read(path if path_arg == p.PLAN else path_arg, "rb") as stream:
                            return stream.read()
                    read.side_effect = bytes_for
                    with self.assertRaises(ValueError):
                        p.verify(root)


if __name__ == "__main__":
    unittest.main()
