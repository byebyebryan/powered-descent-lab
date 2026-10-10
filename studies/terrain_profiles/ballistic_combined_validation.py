"""Frozen original preservation plus seed-disjoint paired combined validation."""
import argparse
from collections import Counter
import json
import math
from pathlib import Path
import subprocess
import sys
import time

import ballistic_acquisition_safety_panel as acquisition
import ballistic_exit_diagnostic as scheduling
import ballistic_feedback_sweep as sweep
import challenge
import diagnostics as d
import fresh_validation as fresh
import refinement as ref
import ridge_waypoint_panel as ridge
import study
import survey
import waypoint_entry_panel as entry

PREVIOUS = acquisition.capture("acquisition-full")
SEAL = ("c673075623673007dfd839cccf45a734991a1a5e68fb9fa2e2f838ebce2fd973",
        "dc75abe4c82975afd1a2ed2c4a7870eddbc51bf4cf6b2facd11d247477babea9")
NATIVE = survey.REPO / "target/ballistic-combined-validation-20261010/release/pd-eval"
RENDERER = NATIVE.parent / "examples/ballistic_feedback_batch_report"
PROTOCOL = "docs/ballistic_combined_validation_plan.md"
FOCUS = sorted(set(acquisition.FOCUS + [283, 327, 861]))
REPEATS = acquisition.REPEATS + [283, 327, 861]
FRESH_REPEATS = [0, 250, 500, 750, 999]
MODES = {"acquisition-gate": sweep.SAFETY_CANDIDATES["acquisition-gate"],
         **sweep.COMBINED_SAFETY_CANDIDATES}
STAGES = {"control": "acquisition-gate", "original-combined": "acquisition-terminal-safety",
          "heldout-acquisition": "acquisition-gate", "heldout-combined": "acquisition-terminal-safety"}


def capture(stage):
    return survey.OUTPUTS / f"capture-ballistic-combined-validation-{stage}-20261010-v1"


def excluded():
    return fresh.excluded_seeds() | set(fresh.seeds(fresh.contract()))


def input_plan():
    return {"schema": "pd-lab.ballistic-combined-heldout-inputs.v1", "phase": "held_out",
            "master_seed": 2026101001, "seed_count": 1000, "cases_per_recipe": 250,
            "recipes": d.read(survey.HERE / "challenge_validation_1k_plan.json")["recipes"]}


def seeds():
    return challenge.draw(input_plan()["master_seed"], 1000, excluded())


def profiles(reference, reverse=False):
    plan = input_plan()
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    cls = study.load_reference(reference, base)
    xs = list(range(base["domain_m"][0], base["domain_m"][1] + 1, base["spacing_m"]))
    assignments = list(enumerate(seeds()))
    if reverse:
        assignments.reverse()
        xs.reverse()
    result = {}
    for index, seed in assignments:
        recipe = plan["recipes"][index // 250]
        generator = study.make_refined(cls, seed, challenge.recipe(base, recipe), base["weighted_strength"])
        generator._ridge_noise = ref.RidgeSlice(generator._ridge_noise, refinement["ridge_slice_offset"])
        points = sorted([x, generator(x + base["source_offset_m"])] for x in xs)
        if any(not math.isfinite(y) for _, y in points):
            raise ValueError("nonfinite fresh terrain")
        result[seed] = {"variant": recipe["id"], "seed": seed, "points_m": points}
    return [result[seed] for seed in seeds()]


def source():
    value = entry.source(spec("control"))
    value["files"][PROTOCOL] = study.digest((survey.REPO / PROTOCOL).read_bytes())
    return value


def protected():
    value = acquisition.protected()
    for root in (PREVIOUS, acquisition.capture("terminal-full")):
        for name in ("receipt.json", "ballistic-feedback-sweep.json", "index.html"):
            value[str((root / name).relative_to(survey.REPO))] = study.digest((root / name).read_bytes())
    return value


def previous_verify(root):
    if tuple(study.digest((root / name).read_bytes()) for name in
             ("receipt.json", "ballistic-feedback-sweep.json")) != SEAL:
        raise ValueError("changed acquisition-only reference")
    if survey.inventory(root) != d.read(root / "receipt.json")["files"]:
        raise ValueError("acquisition-only inventory changed")


def layout(stage):
    indices = FOCUS if stage == "control" else range(1000)
    repeat_indices = FRESH_REPEATS if stage.startswith("heldout-") else REPEATS
    return ([f"random-{i:03}" for i in indices], [f"random-{i:03}" for i in repeat_indices])


def spec(stage):
    return entry.PanelSpec(PREVIOUS, NATIVE, RENDERER, MODES, "acquisition-gate",
                           STAGES[stage], lambda _: layout(stage), PROTOCOL,
                           previous_verify, "ballistic-combined-validation")


def prepare(reference):
    root = capture("inputs")
    seal, safe = source(), protected()
    survey.reserve(root)
    study.write_new(root / "seeds.json", study.encoded(seeds()))
    study.write_new(root / "input-plan.json", study.encoded(input_plan()))
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    for name in base["pylander"]["files"]:
        data = (reference / name).read_bytes()
        if study.digest(data) != base["pylander"]["files"][name]:
            raise ValueError("Pylander reference changed")
        study.write_new(root / "reference" / name, data)
    sampled = profiles(root / "reference")
    repeated = subprocess.run([sys.executable, "-B", str(survey.HERE / "ballistic_combined_validation.py"),
                               "sample", "--reference", str(root / "reference"), "--reverse"],
                              capture_output=True, check=True, timeout=1200)
    if study.encoded(sampled) != study.encoded(json.loads(repeated.stdout)):
        raise ValueError("fresh-process reversed generation differs")
    rows = []
    for i, profile in enumerate(sampled):
        case = f"random-{i:03}"
        scenario, geometry = challenge.prepared(profile, input_plan(), refinement, base)
        data = study.encoded(scenario)
        path = f"scenarios/{case}.json"
        study.write_new(root / path, data)
        study.write_new(root / "raw" / (case + ".json"), study.encoded(profile))
        rows.append({"case_id": case, "attempt_id": case, "cohort": "random", "seed": profile["seed"],
                     "scenario_path": path, "scenario_sha256": study.digest(data), "geometry": geometry,
                     "baseline_result": None, "status": "not_attempted"})
    if source() != seal or protected() != safe:
        raise ValueError("source/protected state changed during generation")
    study.write_new(root / "manifest.json", study.encoded({"rows": rows, "source": seal, "protected": safe,
                    "generation_repeat_sha256": study.digest(study.encoded(sampled))}))
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    verify_inputs(root)
    return {"prepared": len(rows), "seed_disjoint": True}


def verify_inputs(root):
    if survey.inventory(root) != d.read(root / "receipt.json")["files"]:
        raise ValueError("fresh input inventory changed")
    manifest = d.read(root / "manifest.json")
    rows = manifest["rows"]
    if (d.read(root / "input-plan.json") != input_plan() or d.read(root / "seeds.json") != seeds()
            or len(rows) != 1000 or [r["seed"] for r in rows] != seeds()
            or [r["case_id"] for r in rows] != [f"random-{i:03}" for i in range(1000)]):
        raise ValueError("fresh input layout changed")
    refinement, base = ref.plans(survey.HERE / "refinement_plan.json")
    for row in rows:
        profile = d.read(root / "raw" / (row["case_id"] + ".json"))
        scenario, geometry = challenge.prepared(profile, input_plan(), refinement, base)
        if (row["baseline_result"] is not None or row["geometry"] != geometry
                or row["seed"] != profile["seed"]
                or study.digest(study.encoded(scenario)) != row["scenario_sha256"]
                or study.digest((root / row["scenario_path"]).read_bytes()) != row["scenario_sha256"]):
            raise ValueError("fresh profile/scenario identity changed")
    return manifest


def metrics(rows):
    if rows[0]["baseline_result"] is not None and "nominal_class" in rows[0]["baseline_result"]:
        return ridge.metrics(rows)
    primary = [r for r in rows if r["cohort"] == "random"]
    recorded = [r for r in primary if r["status"] == "recorded"]
    wins = [r for r in recorded if r["result"]["verified_landing"]]
    result = {"primary_count": len(primary), "recorded_count": len(recorded), "verified_landings": len(wins),
            "baseline_landings": None, "zero_h_landings": sum(r["result"]["handoffs"] == 0 for r in wins),
            "waypoint_landings": sum(r["result"]["handoffs"] > 0 for r in wins),
            "planning_stops": dict(Counter(r["result"]["stop_group"] for r in recorded)),
            "physical_outcomes": dict(Counter(r["result"]["physical_outcome"] for r in recorded)),
            "repeats_recorded": sum(r["cohort"] == "repeat" and r["status"] == "recorded" for r in rows),
            "per_recipe": {recipe: {"count": sum(r["geometry"]["recipe_id"] == recipe for r in primary),
                                   "landings": sum(r["geometry"]["recipe_id"] == recipe for r in wins)}
                           for recipe in sorted({r["geometry"]["recipe_id"] for r in primary})}}
    if rows[0]["baseline_result"] is not None:
        result.update(baseline_landings=sum(r["baseline_result"]["verified_landing"] for r in primary),
                      gained_landings=[r["case_id"] for r in wins if not r["baseline_result"]["verified_landing"]],
                      lost_landings=[r["case_id"] for r in recorded
                                     if r["baseline_result"]["verified_landing"] and not r["result"]["verified_landing"]])
    return result


def context(stage):
    return {"population": "Fresh seed-disjoint procedural 1k; unchanged four recipes." if stage.startswith("heldout-")
            else "Original procedural worlds; development reuse, not held-out generalization.",
            "comparison": "No prior same-world outcome; acquisition-only establishes the fresh reference."
            if stage == "heldout-acquisition" else "Compared with acquisition-only on these exact worlds.",
            "title": f"Ballistic feedback — {stage} diagnostic"}


def preterminal_equal(current, prior):
    def first(feedback):
        return next((u["physics_step"] for u in feedback["updates"] if u["phase"] == "maintained_terminal"), None)
    step = first(prior)
    if first(current) != step:
        raise ValueError("combination changed terminal entry clock")
    def prefix(feedback):
        return [a for a in feedback["ordinary_flight"]["actions"] if step is None or a["physics_step"] < step]
    if prefix(current) != prefix(prior):
        raise ValueError("combination changed preterminal commands")
    if step is None and current["ordinary_flight"] != prior["ordinary_flight"]:
        raise ValueError("combination changed never-terminal flight")


def verify(stage):
    root = capture(stage)
    if survey.inventory(root) != d.read(root / "receipt.json")["files"]:
        raise ValueError("stage inventory changed")
    manifest, plan, report = [d.read(root / name) for name in
                             ("manifest.json", "plan.json", "ballistic-feedback-sweep.json")]
    primary, repeats = layout(stage)
    expected = primary + ["repeat-" + name for name in repeats]
    if (plan["stage"] != stage or plan["waypoint_experiment"] != STAGES[stage]
            or plan["candidate_id"] != MODES[STAGES[stage]] or plan["workers"] != 4
            or plan["correction_cap"] != 24 or plan["case_wall_limit_s"] != 60
            or plan["campaign_wall_limit_s"] != 7200 or plan["retries"] != 0
            or plan["tuning"] or plan["publication"] or plan["maximum_measured_attempts"] != len(expected)
            or [r["attempt_id"] for r in report["rows"]] != expected
            or report["source_after"] != manifest["source"] or report["protected_after"] != manifest["protected"]
            or report["stopped_reason"] is not None or report["diagnostic_context"] != context(stage)
            or report["summary"] != metrics(report["rows"])
            or study.digest((root / "experiment-plan.md").read_bytes()) != plan["experiment_plan_sha256"]):
        raise ValueError("combined validation contract/source/summary changed")
    if stage.startswith("heldout-"):
        if study.digest((root / "input-receipt.json").read_bytes()) != plan["input_receipt_sha256"]:
            raise ValueError("wrong frozen fresh-input receipt")
        input_files = d.read(root / "input-receipt.json")["files"]
        if study.digest((root / "input-manifest.json").read_bytes()) != input_files["manifest.json"]:
            raise ValueError("fresh input manifest changed")
        input_rows = {r["case_id"]: r for r in d.read(root / "input-manifest.json")["rows"]}
    seal = manifest["source"]
    if (plan["candidate_executable_sha256"] != seal["executable_sha256"]
            or plan["candidate_rust_source_sha256"] != seal["rust_source_tree_sha256"]
            or plan["candidate_report_script_sha256"] != seal["files"]["pd-report/src/planning_cycles.js"]):
        raise ValueError("plan/source identity changed")
    for name, key in (("pd-eval", "executable_sha256"), ("batch-report", "renderer_sha256")):
        if study.digest((root / "candidate/bin" / name).read_bytes()) != seal[key]:
            raise ValueError("saved native/renderer changed")
    for name, digest in seal["files"].items():
        if study.digest(d.safe_file(root / "candidate/source", name).read_bytes()) != digest:
            raise ValueError("saved source changed")
    old_files = None
    if stage != "heldout-acquisition":
        receipt = (root / "previous-receipt.json").read_bytes()
        old_data = (root / "previous-results.json").read_bytes()
        old_files = json.loads(receipt)["files"]
        if (study.digest(receipt) != plan["previous_receipt_sha256"]
                or study.digest(old_data) != plan["previous_results_sha256"]
                or old_files["ballistic-feedback-sweep.json"] != study.digest(old_data)):
            raise ValueError("paired reference changed")
        old = {r["case_id"]: r for r in json.loads(old_data)["rows"] if r["cohort"] == "random"}
        if stage != "heldout-combined" and tuple(plan[k] for k in
                ("previous_receipt_sha256", "previous_results_sha256")) != SEAL:
            raise ValueError("wrong original reference")
    else:
        inputs = json.loads((root / "input-manifest.json").read_bytes())
        files = json.loads((root / "input-receipt.json").read_bytes())["files"]
        if files["manifest.json"] != study.digest((root / "input-manifest.json").read_bytes()):
            raise ValueError("input manifest receipt changed")
        old = {r["case_id"]: r for r in inputs["rows"]}
    if len(report["rows"]) != len(manifest["rows"]):
        raise ValueError("attempt inventory length changed")
    for row, frozen in zip(report["rows"], manifest["rows"]):
        if (any(row[k] != v for k, v in frozen.items() if k != "status") or row["status"] != "recorded"
                or sweep.check_record(root, row, plan) != row["result"]):
            raise ValueError("frozen attempt/native proof changed")
        prior_row = old[row["case_id"]]
        if any(row[k] != prior_row[k] for k in ("seed", "geometry", "scenario_path", "scenario_sha256")):
            raise ValueError("paired input identity changed")
        current = d.read(root / row["output_dir"] / "feedback.json")
        if current["candidate_id"] != plan["candidate_id"]:
            raise ValueError("candidate identity changed")
        if stage.startswith("heldout-") and any(row[k] != input_rows[row["case_id"]][k]
                for k in ("seed", "geometry", "scenario_path", "scenario_sha256")):
            raise ValueError("fresh scenario differs from input freeze")
        if old_files is not None:
            previous = root / "previous-feedback" / (row["case_id"] + ".json")
            if (study.digest(previous.read_bytes()) != old_files[f'runs/{row["case_id"]}/feedback.json']
                    or row["previous_result"] != prior_row["result"]
                    or row["scenario_sha256"] != old_files[row["scenario_path"]]):
                raise ValueError("paired complete feedback/result changed")
            prior = d.read(previous)
            if stage == "heldout-combined" and row["baseline_result"] != prior_row["result"]:
                raise ValueError("fresh paired baseline changed")
            if stage == "control" and current != prior:
                raise ValueError("exact acquisition-only control changed")
            if STAGES[stage] == "acquisition-terminal-safety":
                preterminal_equal(current, prior)
                initial = lambda f: next((r["desired_arc"] for r in f["refreshes"] if r["desired_arc"]), None)
                if initial(current) != initial(prior):
                    raise ValueError("combination changed initial arc")
        elif row["baseline_result"] is not None:
            raise ValueError("fresh acquisition invented a baseline")
        if row["cohort"] == "repeat" and current != d.read(root / "runs" / row["case_id"] / "feedback.json"):
            raise ValueError("complete external repeat changed")
    if stage != "control" and seal != d.read(capture("control") / "manifest.json")["source"]:
        raise ValueError("source differs from exact control")
    return report["summary"]


def run(stage):
    if stage == "control":
        previous_verify(PREVIOUS)
    else:
        verify("control")
    if source() != d.read(capture("inputs") / "manifest.json")["source"]:
        raise ValueError("source differs from fresh-input freeze")
    if stage == "heldout-combined":
        verify("heldout-acquisition")
    previous = capture("heldout-acquisition") if stage == "heldout-combined" else PREVIOUS
    if stage == "heldout-acquisition":
        old = verify_inputs(capture("inputs"))["rows"]
        previous = None
    else:
        old = d.read(previous / "ballistic-feedback-sweep.json")["rows"]
    lookup = {r["case_id"]: r for r in old if r["cohort"] != "repeat"}
    primary, repeats = layout(stage)
    rows = []
    for name in primary:
        prior = lookup[name]
        row = {k: v for k, v in prior.items() if k not in ("output_dir", "exit_code", "failure", "result", "wall_s")}
        row.update(attempt_id=name, cohort="random", status="not_attempted")
        if previous is not None:
            row["previous_result"] = prior["result"]
        if stage == "heldout-combined":
            row["baseline_result"] = prior["result"]
        rows.append(row)
    rows += [dict(next(r for r in rows if r["case_id"] == name), attempt_id="repeat-" + name, cohort="repeat") for name in repeats]
    seal, safe = source(), protected()
    protocol = (survey.REPO / PROTOCOL).read_bytes()
    plan = {"schema": "pd-lab.ballistic-combined-validation-plan.v1", "stage": stage,
            "waypoint_experiment": STAGES[stage], "candidate_id": MODES[STAGES[stage]],
            "correction_cap": 24, "workers": 4, "campaign_wall_limit_s": 7200, "case_wall_limit_s": 60,
            "candidate_executable_sha256": seal["executable_sha256"], "candidate_rust_source_sha256": seal["rust_source_tree_sha256"],
            "candidate_report_script_sha256": seal["files"]["pd-report/src/planning_cycles.js"],
            "maximum_measured_attempts": len(rows), "retries": 0, "tuning": False, "publication": False,
            "experiment_plan_sha256": study.digest(protocol)}
    if previous is not None:
        plan.update(previous_receipt_sha256=study.digest((previous / "receipt.json").read_bytes()),
                    previous_results_sha256=study.digest((previous / "ballistic-feedback-sweep.json").read_bytes()))
    if stage.startswith("heldout-"):
        plan["input_receipt_sha256"] = study.digest((capture("inputs") / "receipt.json").read_bytes())
    root = capture(stage)
    survey.reserve(root)
    for name, value in (("plan.json", plan), ("manifest.json", {"rows": rows, "source": seal, "protected": safe})):
        study.write_new(root / name, study.encoded(value))
    study.write_new(root / "experiment-plan.md", protocol)
    if previous is not None:
        for name in ("receipt.json", "ballistic-feedback-sweep.json"):
            study.write_new(root / ("previous-receipt.json" if name == "receipt.json" else "previous-results.json"), (previous / name).read_bytes())
    if stage.startswith("heldout-"):
        for name in ("manifest.json", "receipt.json"):
            study.write_new(root / ("input-" + name), (capture("inputs") / name).read_bytes())
    for name, digest in seal["files"].items():
        data = (survey.REPO / name).read_bytes()
        if study.digest(data) != digest:
            raise ValueError("source changed during snapshot")
        study.write_new(root / "candidate/source" / name, data)
    for name, path in (("pd-eval", NATIVE), ("batch-report", RENDERER)):
        study.write_new(root / "candidate/bin" / name, path.read_bytes())
        (root / "candidate/bin" / name).chmod(0o755)
    for row in rows[:len(primary)]:
        origin = capture("inputs") if previous is None else previous
        study.write_new(root / row["scenario_path"], (origin / row["scenario_path"]).read_bytes())
        if previous is not None:
            study.write_new(root / "previous-feedback" / (row["case_id"] + ".json"),
                            (previous / "runs" / row["case_id"] / "feedback.json").read_bytes())
    started = time.monotonic()
    actual = scheduling.measure(root, rows, plan)
    after, safe_after = source(), protected()
    report = {"schema": "pd-lab.ballistic-combined-validation.v1", "rows": actual,
              "source_after": after, "protected_after": safe_after,
              "stopped_reason": None if all(r["status"] == "recorded" for r in actual)
              and seal == after and safe == safe_after else "evidence_or_collection_error",
              "collection_wall_s": time.monotonic() - started,
              "summary": metrics(actual), "diagnostic_context": context(stage)}
    study.write_new(root / "ballistic-feedback-sweep.json", study.encoded(report))
    subprocess.run([str(root / "candidate/bin/batch-report"), str(root)], capture_output=True, check=True, timeout=60)
    study.write_new(root / "receipt.json", study.encoded({"files": survey.inventory(root)}))
    return verify(stage)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "sample", "run", "verify"])
    parser.add_argument("stage", choices=STAGES, nargs="?")
    parser.add_argument("--reference", type=Path,
                        default=survey.REPO.parent / "pylander")
    parser.add_argument("--reverse", action="store_true")
    args = parser.parse_args()
    if args.action in ("run", "verify") and args.stage is None:
        parser.error("run/verify requires a stage")
    result = (prepare(args.reference) if args.action == "prepare" else profiles(args.reference, args.reverse)
              if args.action == "sample" else run(args.stage) if args.action == "run" else verify(args.stage))
    print(json.dumps(result, indent=2))
