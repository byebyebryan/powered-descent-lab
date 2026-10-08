"""Source-frozen shadow/preference pass over unchanged saved terrain inputs."""

import argparse
import json
import math
from pathlib import Path

import fallback_pass as fallback
import intervention_pass as timing
import study
import survey

PLAN = timing.HERE / "handoff_room_plan.json"


def contract():
    plan = timing.read(PLAN)
    expected = dict(fallback.contract(), schema="pd-lab.handoff-room-pass.v1",
                    diagnostic_indices=[8, 28, 29, 60, 66, 94, 90, 64],
                    focus_indices=[0, 8, 28, 29, 60, 66, 77, 80, 90, 94, 97, 34, 50, 54, 23, 43, 64],
                    successful_focus_controls=[34, 50, 54, 23, 43, 64],
                    repeat_indices=[28, 50], maximum_measured_attempts=171,
                    motion_baseline_capture="outputs/eval/planner_v2_random_terrain/capture-fallback-20261007-challenge",
                    motion_baseline_manifest_sha256="f45cbfea2ec13c9b0d890508c555340d5440984e0feb411be2b391c8efd439d4",
                    motion_baseline_receipt_sha256="7f7c162eda55bf3b292daa800ddd8ab9007cae6fe35e0fce1cc5d1fd4757c535",
                    benchmark_baseline_capture="outputs/eval/planner_v2_lab_suite/capture-fallback-20261007-benchmark",
                    benchmark_baseline_summary_sha256="118f09ce7d5cbd555c1890d0155fe465602ea42fe2bbc147ecc3b816edd95551")
    if plan != expected:
        raise ValueError("changed bounded handoff-room contract")
    return plan


def baseline():
    plan = contract()
    root = timing.motion_baseline(plan)
    for name, key in (("manifest.json", "motion_baseline_manifest_sha256"),
                      ("receipt.json", "motion_baseline_receipt_sha256")):
        if study.digest((root / name).read_bytes()) != plan[key]:
            raise ValueError("changed current fallback baseline")
    fallback.verify(root)
    return root


def local_choice(search):
    accepted = [r for r in search["row_diagnostics"] if r.get("eligible_handoff") is not None]
    if len(accepted) != search["accepted_row_count"]:
        raise ValueError("eligible handoff inventory differs")
    if not accepted:
        if search["selected"] is not None:
            raise ValueError("selected row lacks accepted handoff")
        return None, None

    def rank(row):
        handoff = row["eligible_handoff"]
        return (-row["entry_physics_step"], handoff["state"]["physics_step"],
                handoff["actual_fuel_burn_to_handoff_kg"], row["row_id"])

    original = min(accepted, key=rank)
    positive = [r for r in accepted if r["eligible_handoff"]["braking_room"] is not None
                and r["eligible_handoff"]["braking_room"]["remaining_room_m"] >= 0]
    estimate = original["eligible_handoff"]["braking_room"]
    preferred = min(positive, key=rank) if positive and estimate is not None and estimate["remaining_room_m"] < 0 else original
    return original, preferred


def check_local(search, scenario, preference):
    entries = search["entries"]
    if (len(entries) > 8 or len({e["physics_step"] for e in entries}) != len(entries)
            or search["row_count"] != len(entries) * 42
            or len(search["row_diagnostics"]) != search["row_count"]):
        raise ValueError("changed bounded search inventory")
    primary = [r for r in search["row_diagnostics"] if not r["row_id"].startswith("fallback_")]
    if any(e["entry_id"].startswith("fallback_") for e in entries) and any(r.get("eligible_handoff") for r in primary):
        raise ValueError("room estimate improperly opened fallback")
    original, preferred = local_choice(search)
    if original is None:
        return None
    chosen = preferred if preference else original
    if search["selected"]["row_id"] != chosen["row_id"] or search["selected"]["handoff_state"] != chosen["eligible_handoff"]["state"]:
        raise ValueError("selection differs from sealed room preference")
    target = next(p for p in scenario["world"]["landing_pads"] if p["id"] == scenario["mission"]["goal"]["target_pad_id"])
    for row in search["row_diagnostics"]:
        handoff = row.get("eligible_handoff")
        if handoff is None or handoff["braking_room"] is None:
            continue
        state, estimate = handoff["state"], handoff["braking_room"]
        a = 0.925 * scenario["vehicle"]["max_thrust_n"] / (scenario["vehicle"]["dry_mass_kg"] + state["fuel_kg"])
        g = scenario["world"]["gravity_mps2"]
        brake = math.sqrt((a - g) * (a + g))
        angle = ((-math.acos(g / a) - state["attitude_rad"] + math.pi) % math.tau) - math.pi
        held = 2 / scenario["sim"]["physics_hz"]
        turn = math.ceil(abs(angle) / scenario["vehicle"]["max_rotation_rate_radps"] / held) * held
        distance = state["velocity_mps"]["x"] * turn + state["velocity_mps"]["x"] ** 2 / (2 * brake)
        expected = [a, brake, turn, distance, target["center_x_m"] - state["position_m"]["x"] - distance]
        actual = [estimate[k] for k in ("available_acceleration_mps2", "horizontal_braking_acceleration_mps2",
                                       "turn_time_s", "required_distance_m", "remaining_room_m")]
        if any(not math.isfinite(v) or abs(v - w) > 1e-9 for v, w in zip(actual, expected)):
            raise ValueError("actual handoff estimate differs from scalar model")
    return {"original_row": original["row_id"], "preferred_row": preferred["row_id"],
            "original_room_m": original["eligible_handoff"]["braking_room"]["remaining_room_m"]
            if original["eligible_handoff"]["braking_room"] else None,
            "preferred_room_m": preferred["eligible_handoff"]["braking_room"]["remaining_room_m"]
            if preferred["eligible_handoff"]["braking_room"] else None}


def verify(root):
    plan = contract()
    report = timing.verify(root, plan, PLAN)
    old = baseline()
    phase = timing.read(root / "manifest.json")["pass_phase"]
    if report["stopped_reason"] is not None:
        raise ValueError("incomplete handoff-room capture")
    choices = []
    for row in report["rows"]:
        if row["status"] != "recorded":
            raise ValueError("unrecorded evaluation")
        output = root / row["output_dir"]
        flight = timing.read(output / "flight.json")
        previous = timing.read(old / "runs" / row["case_id"] / "flight.json")
        fallback.preservation(flight, previous)
        if flight["planning_stop"] not in ("landed", "no_clearing", "no_nominal", "correction_limit"):
            raise ValueError("unexpected crash/fuel/integrity stop")
        total = 0
        for cycle in flight["cycles"]:
            if cycle.get("local_search") is None:
                continue
            total += cycle["local_search"]["row_count"]
            choice = check_local(cycle["local_search"], timing.read(output / "scenario.json"), phase != "diagnostics")
            if choice:
                choices.append(dict(choice, attempt_id=row["attempt_id"], cycle_index=cycle["cycle_index"]))
        if total > 2016:
            raise ValueError("local work bound exceeded")
    return report, choices


def focus_gate(root):
    report, choices = verify(root)
    if timing.read(root / "manifest.json")["pass_phase"] != "focus":
        raise ValueError("not a focus capture")
    controls = {f"random-{i:03}" for i in contract()["successful_focus_controls"]}
    if {r["case_id"] for r in report["rows"] if r["result"]["verified_landing"]} & controls != controls:
        raise ValueError("successful focus control regressed")
    old = baseline()
    gains = [r["case_id"] for r in report["rows"] if r["result"]["verified_landing"]
             and timing.read(old / "runs" / r["case_id"] / "flight.json")["planning_stop"] != "landed"]
    if not gains:
        raise ValueError("focus has no new verified landing")
    return report, gains, choices


def check_benchmark(args):
    plan = contract()
    capture, output, binary = args.capture.resolve(), args.output.resolve(), args.binary.resolve()
    old = survey.REPO / plan["benchmark_baseline_capture"]
    if study.digest((old / "summary.json").read_bytes()) != plan["benchmark_baseline_summary_sha256"]:
        raise ValueError("changed current benchmark baseline")
    source = survey.source_state(binary)
    checks = [fallback.native_check(binary, root) for root in (old, capture)]
    previous, actual = (timing.read(root / "summary.json") for root in (old, capture))
    if (actual["input_identity"] != previous["input_identity"]
            or actual["provenance"]["source_before"]["executable_sha256"] != source["executable_sha256"]
            or actual["provenance"]["source_before"] != actual["provenance"]["source_after"]
            or actual["provenance"]["unchanged_during_capture"] is not True):
        raise ValueError("benchmark input/source differs")
    rows = []
    for row in previous["cases"]:
        if row["planning_stop"] != "landed":
            continue
        paths = [root / "runs" / row["case_id"] / "flight.json" for root in (old, capture)]
        if timing.motion(timing.read(paths[0])) != timing.motion(timing.read(paths[1])):
            raise ValueError("changed previously landed benchmark flight: " + row["case_id"])
        rows.append({"case_id": row["case_id"], "source_sha256": [study.digest(p.read_bytes()) for p in paths]})
    if len(rows) != 38 or survey.source_state(binary) != source:
        raise ValueError("benchmark preservation count/source differs")
    survey.reserve(output)
    audit = {"schema": "pd-lab.handoff-room-benchmark-preservation.v1", "source": source,
             "capture": str(capture), "baseline": str(old), "native_checks": checks, "rows": rows,
             "summary_sha256": study.digest((capture / "summary.json").read_bytes())}
    study.write_new(output / "preservation.json", study.encoded(audit))
    study.write_new(output / "receipt.json", study.encoded({"files": survey.inventory(output)}))
    print(json.dumps({"benchmark_preservation": str(output), "landed_cases_preserved": len(rows)}))


def run(args):
    baseline()
    if args.phase == "focus":
        if args.shadow_capture is None:
            raise ValueError("focus requires frozen shadow gate")
        shadow, choices = verify(args.shadow_capture)
        if timing.read(args.shadow_capture / "manifest.json")["pass_phase"] != "diagnostics":
            raise ValueError("not a shadow capture")
        switches = {c["attempt_id"] for c in choices if c["original_row"] != c["preferred_row"]}
        if switches != {f"random-{i:03}" for i in (8, 28, 29, 60, 66, 94)} or len(shadow["rows"]) != 8:
            raise ValueError("actual shadow state does not support proposed preference")
    if args.phase == "challenge":
        if args.focus_capture is None or args.benchmark_audit is None:
            raise ValueError("challenge requires focus and benchmark gates")
        focus, _, _ = focus_gate(args.focus_capture)
        source = survey.source_state(args.binary.resolve())
        audit = timing.read(args.benchmark_audit / "preservation.json")
        survey.check_inventory(args.benchmark_audit, timing.read(args.benchmark_audit / "receipt.json")["files"])
        if (audit["schema"] != "pd-lab.handoff-room-benchmark-preservation.v1"
                or audit["source"] != source or focus["source_before"] != source
                or audit["summary_sha256"] != study.digest((Path(audit["capture"]) / "summary.json").read_bytes())
                or len(audit["rows"]) != 38):
            raise ValueError("conditional source-frozen gate did not pass")
    timing.run(args, contract(), PLAN)
    report, choices = verify(args.output.resolve())
    if args.phase == "focus":
        _, gains, _ = focus_gate(args.output.resolve())
        print(json.dumps({"focus_gate": "passed", "new_verified_landings": gains}))
    else:
        print(json.dumps({"phase_gate": "passed", "summary": report["summary"], "choices": choices}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    command = sub.add_parser("run")
    command.add_argument("--phase", choices=("diagnostics", "focus", "challenge"), required=True)
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    command.add_argument("--shadow-capture", type=Path)
    command.add_argument("--focus-capture", type=Path)
    command.add_argument("--benchmark-audit", type=Path)
    command = sub.add_parser("verify")
    command.add_argument("capture", type=Path)
    command = sub.add_parser("check-benchmark")
    command.add_argument("--binary", type=Path, required=True)
    command.add_argument("--capture", type=Path, required=True)
    command.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    elif args.command == "check-benchmark":
        check_benchmark(args)
    else:
        report, choices = verify(args.capture)
        print(json.dumps({"verified": str(args.capture), "summary": report["summary"], "choices": choices, "fresh_flights": 0}))
