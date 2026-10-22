"""Prospective staged runner. New output destinations only; no default promotion."""
from __future__ import annotations

import argparse
import collections
import datetime
import importlib.metadata
import json
import math
import shutil
import signal
import sys
import time
from pathlib import Path

import numpy as np
import torch
from stable_baselines3 import PPO
from stable_baselines3.common.vec_env import DummyVecEnv
from stable_baselines3.common.callbacks import BaseCallback

from coast_ignite import (ArcSampler, CoastEnv, NATIVE_ARTIFACT, capture_verified, encoded, model_for,
                          native, run_episode, sha, write_new)


class Budget:
    artifact_root = None
    instances = []
    interruptible = False
    def __init__(self, root, seconds, steps, max_bytes, outer_deadline):
        self.root = Path(root)
        self.started = time.monotonic()
        self.deadline = min(time.time() + seconds, outer_deadline)
        self.max_steps, self.max_bytes = steps, max_bytes
        self.counts = collections.Counter()
        self.instances.append(self)
        if self.interruptible:
            signal.setitimer(signal.ITIMER_REAL, max(0.001, self.deadline - time.time()))
        write_new(self.root / "allocation.json", {"seconds": seconds, "native_steps": steps,
                  "max_bytes": max_bytes, "outer_deadline": outer_deadline,
                  "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat()})

    def count(self, key, value):
        self.counts[key] += value

    def remaining_steps(self):
        return max(0, self.max_steps - sum(n for k, n in self.counts.items()
                                          if k.endswith("_native_steps")))

    def check(self):
        steps = sum(n for k, n in self.counts.items() if k.endswith("_native_steps"))
        if time.time() >= self.deadline or steps > self.max_steps:
            raise RuntimeError("phase wall/native-step budget exhausted")
        if shutil.disk_usage(self.root).free < 50 * 2**30:
            raise RuntimeError("free-space boundary reached")
        # Check storage periodically, not on every 10 Hz policy step.
        if time.monotonic() - getattr(self, "last_bytes_check", 0) > 5:
            self.last_bytes_check = time.monotonic()
            size = sum(p.stat().st_size for p in self.root.rglob("*") if p.is_file())
            if size > self.max_bytes:
                raise RuntimeError("phase byte budget exhausted")
            if self.artifact_root:
                total = sum(p.stat().st_size for p in self.artifact_root.rglob("*") if p.is_file())
                if total > 32 * 2**30:
                    raise RuntimeError("outer artifact budget exhausted")

    def receipt(self, **fields):
        return {"counts": dict(self.counts), "wall_seconds": time.monotonic() - self.started,
                "bytes": sum(p.stat().st_size for p in self.root.rglob("*") if p.is_file()), **fields}


def prepared(root, anchor, seed, cohort, count, budget, seen):
    sampler = ArcSampler(root, anchor, seed, cohort, budget, seen, requested_arcs=count)
    tasks = [sampler.next() for _ in range(count)]
    sampler.log.close()
    return tasks


def evaluated(tasks, rule, budget, model=None):
    rows = []
    for task, path in tasks:
        episode, _ = run_episode(task, rule, budget, model)
        result = json.loads(episode.finish_json())
        rows.append({"task": str(path), "native_success": result["info"]["native_success"],
                     "physics_steps": result["info"]["physics_steps"]})
    return rows


def preflight(root, anchor, outer_deadline):
    root.mkdir()
    seen = set()
    pilot = Budget(root / "pilot", 300, 1_500_000, 2**30, outer_deadline)
    tasks = prepared(root / "development", anchor, 281031, "development", 12, pilot, seen)
    write_new(root / "pilot" / "schedule.json", {"original_arcs": 8, "slots": 64,
              "tasks": [str(path) for _, path in tasks[:8]],
              "schedules": ["0s", "1s", "2s", "3s", "4s", "5s", "nominal witness", "never"]})
    pilot_rows = []
    for i, (task, path) in enumerate(tasks[:8]):
        audit = json.loads(path.with_name(path.stem + "-audit.json").read_text())
        for j, tick in enumerate([0, 120, 240, 360, 480, 600, audit["nominal_coast_ticks"], None]):
            rule = {"kind": "never"} if tick is None else {"kind": "delay", "seconds": tick / 120}
            row = capture_verified(task, path, rule, root / "pilot" / f"case-{i}-{j}", pilot)
            pilot_rows.append(row)
    write_new(root / "pilot" / "result.json", pilot.receipt(rows=pilot_rows, complete=True))
    # Cost evidence is retained before later limits are released.
    timing = Budget(root / "timing", 600, 5_000_000, 256 * 2**20, outer_deadline)
    write_new(root / "timing" / "schedule.json", {"tasks": [str(p) for _, p in tasks],
              "matched_slots": 984, "sampled_prior_slots": 12, "decision_ticks": list(range(0, 961, 12))})
    mapping = []
    for task, path in tasks:
        rows = []
        for slot in range(82):
            rule = {"kind": "delay", "seconds": slot / 10} if slot < 81 else {"kind": "never"}
            ep, _ = run_episode(task, rule, timing)
            info = json.loads(ep.finish_json())["info"]
            rows.append({"requested_slot": slot, "success": info["native_success"],
                         "actual_ignition_tick": next((t for t, a in info["decisions"] if a), None),
                         "stop": info["stop"], "physics_steps": info["physics_steps"],
                         "admission": "accepted" if (info["feedback"] or {}).get("accepted") else "no-plan-or-passive"})
        audit = json.loads(path.with_name(path.stem + "-audit.json").read_text())
        mapping.append({"task": str(path), "witness_slot": audit["nominal_coast_ticks"] // 12,
                        "successful_slots": [r["requested_slot"] for r in rows if r["success"]], "rows": rows})
    witness_pass = sum(m["rows"][m["witness_slot"]]["success"] for m in mapping)
    reachable = sum(bool(m["successful_slots"]) for m in mapping)
    prior_rows = []
    prior_rng = np.random.default_rng(281030)
    for task, path in tasks:
        ep, _ = run_episode(task, {"kind": "prior"}, timing, rng=prior_rng)
        prior_rows.append({"task": str(path), **json.loads(ep.finish_json())["info"]})
    write_new(root / "timing" / "result.json", timing.receipt(mapping=mapping,
              prior_baseline=prior_rows, nominal_witness_successes=witness_pass,
              reachable_arcs=reachable, complete=True))
    if reachable < 2 or witness_pass != 12:
        return {"gate": "timing", "passed": False, "reason": "nominal native witnesses or reward coverage failed",
                "nominal_witness_successes": witness_pass, "reachable_arcs": reachable}
    readiness = Budget(root / "readiness", 600, 2_000_000, 256 * 2**20, outer_deadline)
    readiness_tasks = prepared(root / "readiness-inputs", anchor, 281032, "readiness", 256, readiness, seen)
    rng = np.random.default_rng(281033)
    positive = []
    stop_histogram = collections.Counter()
    for task, path in readiness_tasks:
        ep, _ = run_episode(task, {"kind": "prior"}, readiness, rng=rng)
        info = json.loads(ep.finish_json())["info"]
        stop_histogram[str(next((t for t, a in info["decisions"] if a), None))] += 1
        if info["native_success"]:
            positive.append(str(path))
    write_new(root / "readiness" / "sampling.json", readiness.receipt(positive_tasks=positive,
              stop_histogram=dict(stop_histogram), complete=True))
    if len(set(positive)) < 2:
        return {"gate": "reward-access", "passed": False, "reason": "fewer than two positive distinct arcs"}
    # The first complete update uses a new, separately declared training stream.
    sampler = ArcSampler(root / "update-inputs", anchor, 281034, "update-smoke", readiness, seen, requested_arcs=2052)
    with (root / "readiness" / "update-episodes.jsonl").open("x") as records:
        vec = DummyVecEnv([lambda: CoastEnv(sampler, readiness, records) for _ in range(4)])
        model = model_for(vec, 281035)
        before = model.policy.parameters_to_vector().copy()
        optimizer_steps = []
        hook = model.policy.optimizer.register_step_post_hook(lambda *_: optimizer_steps.append(1))
        model.learn(total_timesteps=2048)
        hook.remove()
        after = model.policy.parameters_to_vector()
        finite = bool(np.isfinite(after).all())
        changed = bool(np.any(before != after))
        sample = np.stack([native.CoastEpisode(encoded(t[0]["scenario"]), t[0]["reference_end"]).observation() for t in readiness_tasks[:64]]).astype(np.float32)
        with torch.no_grad():
            probs = model.policy.get_distribution(torch.as_tensor(sample)).distribution.probs[:, 1].numpy()
        noncollapsed = bool(np.all((probs > 1e-6) & (probs < 1 - 1e-6)))
        write_new(root / "readiness" / "update.json", readiness.receipt(finite=finite, changed=changed,
                  noncollapsed=noncollapsed, optimizer_steps=len(optimizer_steps),
                  probability_range=[float(probs.min()), float(probs.max())], decisions=model.num_timesteps))
        sampler.log.close()
    return {"gate": "readiness", "passed": finite and changed and noncollapsed,
            "positive_arcs": len(positive), "nominal_witness_successes": witness_pass}


def all_physical_identities(root):
    return {json.loads(p.read_text())["physical_sha256"] for p in root.rglob("arc-*.json")
            if not p.stem.endswith("-audit")}


class TrainingAudit(BaseCallback):
    def __init__(self, budget):
        super().__init__()
        self.budget = budget

    def _on_step(self):
        self.budget.check()
        return True


def binomial_interval(successes, total, alpha=0.05):
    """Exact equal-tail Clopper-Pearson bounds, without an extra dependency."""
    def cdf(k, p):
        if p <= 0:
            return 1.0
        if p >= 1:
            return float(k == total)
        logs = [math.lgamma(total + 1) - math.lgamma(i + 1) - math.lgamma(total - i + 1)
                + i * math.log(p) + (total - i) * math.log1p(-p) for i in range(k + 1)]
        peak = max(logs)
        return min(1.0, math.exp(peak) * sum(math.exp(x - peak) for x in logs))
    def solve(k, target):
        low, high = 0.0, 1.0
        for _ in range(64):
            mid = (low + high) / 2
            if cdf(k, mid) > target:
                low = mid
            else:
                high = mid
        return (low + high) / 2
    return [0.0 if successes == 0 else solve(successes - 1, 1 - alpha / 2),
            1.0 if successes == total else solve(successes, alpha / 2)]


def checkpoint_metrics(model):
    # SB3 creates its logger at the first learn(), after checkpoint zero.
    logger = getattr(model, "_logger", None)
    return {k: float(v) for k, v in (logger.name_to_value.items() if logger else [])
            if k.startswith("train/") and np.isscalar(v)}


def learn_and_confirm(root, anchor, outer_deadline, prior_exposure=None, fixed_validation=None,
                      carry=None):
    root.mkdir()
    carry = carry or {}
    seen = all_physical_identities(prior_exposure or root.parent / "preflight")
    prior = carry.get("validation", {})
    validation_budget = Budget(root / "validation", 600 - prior.get("wall_seconds", 0),
                               6_000_000 - prior.get("native_steps", 0), 2**30, outer_deadline)
    validation = fixed_validation if fixed_validation is not None else prepared(
        root / "validation-inputs", anchor, 281040, "validation", 64, validation_budget, seen)
    write_new(root / "validation" / "input-bindings.json", [{"path": str(p), "sha256": sha(p)} for _, p in validation])
    baselines = [{"kind": "immediate"}]
    baselines += [{"kind": "delay", "seconds": i / 2} for i in range(1, 17)]
    baselines += [{"kind": "ttc", "seconds": s} for s in [0.25, 0.5, 1, 1.5, 2, 3, 4, 6]]
    baseline_scores = []
    for rule in baselines:
        rows = evaluated(validation, rule, validation_budget)
        baseline_scores.append({"rule": rule, "successes": sum(r["native_success"] for r in rows), "rows": rows})
    selected_baseline = max(baseline_scores, key=lambda r: r["successes"])
    write_new(root / "validation" / "baselines.json", validation_budget.receipt(
              scores=baseline_scores, selected=selected_baseline["rule"]))
    seed_results = []
    for seed in [281101, 281102, 281103]:
        prior = carry.get(f"seed-{seed}", {})
        training = Budget(root / f"seed-{seed}", 1800 - prior.get("wall_seconds", 0),
                          50_000_000 - prior.get("native_steps", 0), 2**30, outer_deadline)
        sampler = ArcSampler(root / f"training-inputs-{seed}", anchor, seed, f"train-{seed}", training, seen, requested_arcs=131076)
        curves = []
        with (root / f"seed-{seed}" / "episodes.jsonl").open("x") as records:
            vec = DummyVecEnv([lambda: CoastEnv(sampler, training, records) for _ in range(4)])
            model = model_for(vec, seed)
            if prior.get("initial_weights_sha256"):
                import hashlib
                if hashlib.sha256(model.policy.parameters_to_vector().tobytes()).hexdigest() != prior["initial_weights_sha256"]:
                    raise RuntimeError("continuation changed the original zero-update policy")
            steps = []
            hook = model.policy.optimizer.register_step_post_hook(lambda *_: steps.append(1))
            for checkpoint in [0, 32768, 65536, 98304, 131072]:
                if checkpoint:
                    model.learn(total_timesteps=32768, reset_num_timesteps=False,
                                callback=TrainingAudit(training))
                if model.num_timesteps != checkpoint:
                    raise RuntimeError("checkpoint decision accounting differs")
                path = root / f"seed-{seed}" / f"checkpoint-{checkpoint}.zip"
                if path.exists():
                    raise RuntimeError("checkpoint already exists")
                model.save(path)
                loaded = PPO.load(path, device="cpu")
                score = evaluated(validation, {"kind": "model"}, training, loaded)
                curve = {"checkpoint": checkpoint, "path": str(path), "sha256": sha(path),
                         "successes": sum(r["native_success"] for r in score), "rows": score,
                         "optimizer_steps": len(steps), "ppo_epochs": model._n_updates,
                         "train_metrics": checkpoint_metrics(model)}
                if not np.isfinite(model.policy.parameters_to_vector()).all():
                    raise RuntimeError("nonfinite trained model")
                curves.append(curve)
                write_new(root / f"seed-{seed}" / f"validation-{checkpoint}.json", training.receipt(**curve))
                print(encoded({"seed": seed, "checkpoint": checkpoint, "validation_successes": curve["successes"]}), flush=True)
            hook.remove()
            sampler.log.close()
            selected = max(curves, key=lambda c: c["successes"])
            seed_results.append({"seed": seed, "selected": selected, "curve": curves})
            partial = [{"task": str(env.task_path), "native_prefix": json.loads(env.episode.status_json())}
                       for env in vec.envs]
            write_new(root / f"seed-{seed}" / "result.json", training.receipt(
                      selected=selected, curve=curves, unfinished_rollout_episodes=partial))
    primary = max(seed_results, key=lambda r: (r["selected"]["successes"], -r["selected"]["checkpoint"], -r["seed"]))
    selected = primary["selected"]
    freeze = {"primary_seed": primary["seed"], "selected_model": selected,
              "baseline": selected_baseline["rule"], "source": json.loads(native.source_identity_json()),
              "native_library_sha256": sha(NATIVE_ARTIFACT), "training_seeds": seed_results}
    write_new(root / "selection-freeze.json", freeze)
    proof_budget = Budget(root / "validation-proof", 600, 8_000_000, 2 * 2**30, outer_deadline)
    report_rows = []
    for item in seed_results + [{"seed": "heuristic"}]:
        is_model = "selected" in item
        rule = {"kind": "model"} if is_model else selected_baseline["rule"]
        model_path = Path(item["selected"]["path"]) if is_model else None
        model = PPO.load(model_path, device="cpu") if is_model else None
        count = 0
        for i, (task, path) in enumerate(validation):
            capture = root / "validation-proof" / f"{item['seed']}-{i}"
            row = capture_verified(task, path, rule, capture, proof_budget, model, model_path)
            count += row["verified_target_landing"]
            report_rows.append({"capture": str(capture), "population": "Validation", "label": str(item["seed"])})
        expected = item["selected"]["successes"] if is_model else selected_baseline["successes"]
        if count != expected:
            raise RuntimeError("verified validation differs from package selection")
    write_new(root / "validation-proof" / "result.json", proof_budget.receipt(complete=True))
    fresh = Budget(root / "confirmation", 3000, 60_000_000, 24 * 2**30, outer_deadline)
    fresh_tasks = prepared(root / "fresh-inputs", anchor, 281200, "fresh-confirmation", 600, fresh, seen)
    model_path = Path(selected["path"])
    model = PPO.load(model_path, device="cpu")
    if sha(model_path) != selected["sha256"]:
        raise RuntimeError("frozen checkpoint changed")
    packages = [("RL", {"kind": "model"}), ("Heuristic", selected_baseline["rule"]),
                ("Immediate", {"kind": "immediate"}), ("Never", {"kind": "never"})]
    # Persist the complete schedule before the first fresh flight.
    write_new(root / "confirmation" / "schedule.json", {"inputs": [str(p) for _, p in fresh_tasks],
              "packages": packages, "slots": 2400, "selection_freeze_sha256": sha(root / "selection-freeze.json")})
    outcomes = []
    with (root / "confirmation" / "results.jsonl").open("x") as stream:
        for i, (task, path) in enumerate(fresh_tasks):
            comparison = {}
            for label, rule in packages:
                capture = root / "confirmation" / f"case-{i}-{label.lower()}"
                row = capture_verified(task, path, rule, capture, fresh,
                                       model if label == "RL" else None,
                                       model_path if label == "RL" else None)
                comparison[label] = row["verified_target_landing"]
                stream.write(encoded({"mission": i, "package": label, **row}) + "\n")
                stream.flush()
                report_rows.append({"capture": str(capture), "population": "Fresh confirmation", "label": label})
            outcomes.append(comparison)
            if i % 50 == 0:
                print(encoded({"fresh_completed": i + 1}), flush=True)
    w = sum(o["RL"] and not o["Heuristic"] for o in outcomes)
    l = sum(o["Heuristic"] and not o["RL"] for o in outcomes)
    p = sum(math.comb(w + l, k) for k in range(w, w + l + 1)) / 2**(w + l) if w + l else 1.0
    counts = {name: sum(o[name] for o in outcomes) for name, _ in packages}
    intervals = {name: binomial_interval(count, 600) for name, count in counts.items()}
    result = fresh.receipt(complete=True, primary_seed=primary["seed"], counts=counts,
                          intervals=intervals, rl_only_wins=w, heuristic_only_wins=l,
                          lift=(w - l) / 600, one_sided_p=p,
                          demonstrated_advantage=w > l and p <= 0.05)
    write_new(root / "confirmation" / "result.json", result)
    summary = f"600 fresh shared arcs; verified landings {counts}. RL-only wins {w}, heuristic-only wins {l}; paired one-sided p={p:.6g}. This is one validation-selected package, not general controller qualification."
    write_new(root / "report-spec.json", {"title": "Coast and ignition: frozen hybrid evaluation", "summary": summary, "rows": report_rows})
    native.render_report(encoded(json.loads((root / "report-spec.json").read_text())), str(root / "report"))
    return result


def closed_zero_update_release(prior, source):
    """Explicitly resume only this retained preflight after its logging fault."""
    release = json.loads((prior / "preflight-result.json").read_text())
    fault = json.loads((prior / "fault.json").read_text())
    if not release["passed"] or fault["type"] != "AttributeError" or "_logger" not in fault["reason"]:
        raise RuntimeError("continuation requires the closed checkpoint-zero logging fault")
    if (prior / "learning/fresh-inputs").exists():
        raise RuntimeError("fresh inputs were already released")
    checkpoints = list(prior.glob("learning/seed-*/checkpoint-*.zip"))
    if len(checkpoints) != 1 or checkpoints[0].name != "checkpoint-0.zip":
        raise RuntimeError("continuation is limited to zero training updates")
    initial = PPO.load(checkpoints[0], device="cpu")
    if initial.num_timesteps != 0 or initial._n_updates != 0 or initial.policy.optimizer.state:
        raise RuntimeError("prior training optimizer or decisions already advanced")
    if any(p.stat().st_size for p in prior.glob("learning/seed-*/episodes.jsonl")):
        raise RuntimeError("prior training has completed episodes")
    before = release["source"]["source_files"]
    old_root = Path(next(f["path"] for f in before if f["path"].endswith("/Cargo.lock"))).parent
    new_root = Path(__file__).resolve().parents[2]
    old = {str(Path(f["path"]).relative_to(old_root)): f["sha256"] for f in before}
    new = {str(Path(f["path"]).relative_to(new_root)): f["sha256"] for f in source["source_files"]}
    changed = {p for p in old.keys() | new.keys() if old.get(p) != new.get(p)}
    if changed != {"experiments/coast-ignite/run_loop.py"}:
        raise RuntimeError(f"continuation changed sources beyond runner bookkeeping: {changed}")
    if any(sha(f["path"]) != f["sha256"] for f in before):
        raise RuntimeError("retained preflight sources changed")
    paths = [prior / f"learning/validation-inputs/arc-{i}.json" for i in range(64)]
    validation = [(json.loads(p.read_text()), p) for p in paths]
    carry = {}
    for resource in fault["resources"]:
        name = Path(resource["phase"]).name
        if name == "validation" or name.startswith("seed-"):
            carry[name] = {"wall_seconds": resource["wall_seconds"],
                           "native_steps": sum(n for k, n in resource["counts"].items()
                                               if k.endswith("_native_steps"))}
    import hashlib
    carry["seed-281101"]["initial_weights_sha256"] = hashlib.sha256(initial.policy.parameters_to_vector().tobytes()).hexdigest()
    return validation, {"preflight_result_sha256": sha(prior / "preflight-result.json"),
                        "fault_sha256": sha(prior / "fault.json"), "changed_sources": sorted(changed),
                        "prior_training_decisions": 0, "prior_training_optimizer_updates": 0,
                        "carried_resources": carry}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--anchor", type=Path, default=Path("fixtures/scenarios/flat_terminal_descent.json"))
    parser.add_argument("--outer-deadline", type=float, required=True)
    parser.add_argument("--continue-from", type=Path)
    args = parser.parse_args()
    Budget.artifact_root = args.output.parent
    Budget.interruptible = True
    def alarm(_signum, _frame):
        raise TimeoutError("declared phase wall limit reached")
    signal.signal(signal.SIGALRM, alarm)
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    if args.output.exists():
        raise RuntimeError("create-only output already exists")
    # Bind/import checks precede all measured native execution.
    source = json.loads(native.source_identity_json())
    versions = {name: importlib.metadata.version(name) for name in
                ["stable-baselines3", "torch", "gymnasium", "numpy", "maturin"]}
    anchor = json.loads(args.anchor.read_text())
    try:
        args.output.mkdir()
        if args.continue_from:
            validation, release = closed_zero_update_release(args.continue_from, source)
            write_new(args.output / "continuation-release.json", {**release, "source": source,
                      "versions": versions, "native_library_sha256": sha(NATIVE_ARTIFACT)})
            result = learn_and_confirm(args.output / "learning", anchor, args.outer_deadline,
                                       prior_exposure=args.continue_from, fixed_validation=validation,
                                       carry=release["carried_resources"])
            print(encoded(result), flush=True)
            return
        result = preflight(args.output / "preflight", anchor, args.outer_deadline)
        write_new(args.output / "preflight-result.json", {**result, "source": source,
                  "versions": versions, "native_library_sha256": sha(NATIVE_ARTIFACT)})
        print(encoded(result), flush=True)
        if result["passed"]:
            result = learn_and_confirm(args.output / "learning", anchor, args.outer_deadline)
            print(encoded(result), flush=True)
        else:
            rows = json.loads((args.output / "preflight/pilot/result.json").read_text())["rows"]
            spec = {"title": "Coast and ignition: bounded preflight result", "summary": encoded(result),
                    "rows": [{"capture": row["capture"], "population": "Cost pilot",
                              "label": encoded(row["rule"])} for row in rows]}
            write_new(args.output / "report-spec.json", spec)
            native.render_report(encoded(spec), str(args.output / "report"))
    except BaseException as exc:
        write_new(args.output / "fault.json", {"type": type(exc).__name__, "reason": str(exc),
                  "resources": [{"phase": str(b.root), **b.receipt()} for b in Budget.instances]})
        raise


if __name__ == "__main__":
    main()
