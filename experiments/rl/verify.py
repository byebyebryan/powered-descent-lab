"""Independent saved-command verification; never imports or loads a PPO policy."""
import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path

import pd_native


def verify_campaign(directory):
    directory = Path(directory)
    summary_path = directory/"summary.json"
    summary = json.loads(summary_path.read_text())
    projection = json.loads((directory/"report-input.json").read_text())
    if not summary["integrity_passed"] or summary["evidence_before"] != summary["evidence_after"]:
        raise ValueError("campaign evidence changed during execution")
    schedule, attempts, rows = summary["request"]["attempt_indices"], summary["attempts"], projection["attempts"]
    if len(schedule) != len(attempts) or len(rows) != len(attempts):
        raise ValueError("campaign lost scheduled attempts")
    manifests = [item for item in summary["evidence_before"]["input_files"]
                 if Path(item["path"]).name == "manifest.json" and item["sha256"] == summary["request"]["manifest_sha256"]]
    if len(manifests) != 1:
        raise ValueError("campaign lacks its uniquely bound collection manifest")
    manifest_path = Path(manifests[0]["path"])
    if hashlib.sha256(manifest_path.read_bytes()).hexdigest() != manifests[0]["sha256"]:
        raise ValueError("original collection manifest changed")
    manifest = json.loads(manifest_path.read_text())
    entries = {entry["index"]: entry for entry in manifest["entries"]}
    captures, landings, unsupported = 0, 0, 0
    timeout_endpoints, incoming_speeds = [], []
    for index, (mission_index, attempt, row) in enumerate(zip(schedule, attempts, rows)):
        if attempt["attempt_index"] != index or attempt["mission_index"] != mission_index or row["mission_index"] != mission_index:
            raise ValueError("saved schedule differs from request")
        entry = entries[mission_index]
        if row["physical_sha256"] != entry["physical_sha256"]:
            raise ValueError("projected physical input differs from scheduled collection input")
        if hashlib.sha256((manifest_path.parent/entry["file"]).read_bytes()).hexdigest() != entry["sha256"]:
            raise ValueError("original resolved input bytes changed")
        disposition = attempt["disposition"]
        if disposition["kind"] == "unsupported":
            if row["disposition"] != "unsupported" or row["verified_target_landing"]:
                raise ValueError("unsupported input has a projected execution claim")
            unsupported += 1
            continue
        if disposition["kind"] != "executed":
            raise ValueError("final campaign contains a preparation/execution/capture failure")
        capture = directory/disposition["capture"]
        verified = json.loads(pd_native.verify_capture(str(capture)))
        evaluation = json.loads((capture/"evaluation.json").read_text())
        if evaluation["physical_sha256"] != row["physical_sha256"] or evaluation["execution"]["descriptor"] != summary["request"]["agent"]:
            raise ValueError("capture differs from projected physical input or declared approach")
        expected = {"integrity_passed": disposition["integrity_passed"],
                    "physical_replay_verified": disposition["physical_replay_verified"],
                    "complete_replay_verified": disposition["complete_replay_verified"],
                    "verified_target_landing": disposition["verified_target_landing"]}
        actual = {"integrity_passed": verified["integrity_passed"],
                  "physical_replay_verified": verified["replay"]["physical_prefix_verified"],
                  "complete_replay_verified": verified["replay"]["complete_episode_verified"],
                  "verified_target_landing": verified["verified_target_landing"]}
        if actual != expected or any(row[key] != value for key, value in actual.items()):
            raise ValueError("independent command replay differs from saved campaign/report")
        if not actual["integrity_passed"] or not actual["physical_replay_verified"] or not actual["complete_replay_verified"]:
            raise ValueError("final capture lacks exact integrity/replay proof")
        captures += 1
        landings += int(actual["verified_target_landing"])
        episode, scenario = evaluation["execution"]["episode"], evaluation["scenario"]
        state = episode["final_state"]
        goal = scenario["mission"]["goal"]
        if state["physical_outcome"] == "timed_out" and goal["kind"] == "landing_on_pad":
            pad = next(pad for pad in scenario["world"]["landing_pads"] if pad["id"] == goal["target_pad_id"])
            vertical = state["position_m"]["y"]-pad["surface_y_m"]-scenario["vehicle"]["geometry"]["touchdown_base_offset_m"]
            timeout_endpoints.append((abs(state["position_m"]["x"]-pad["center_x_m"]), vertical,
                                      math.hypot(state["velocity_mps"]["x"], state["velocity_mps"]["y"])))
        if episode["incoming_contact"] is not None:
            velocity = episode["incoming_contact"]["state"]["velocity_mps"]
            incoming_speeds.append(math.hypot(velocity["x"], velocity["y"]))
        if captures % 128 == 0:
            print(json.dumps({"campaign": str(directory), "replayed": captures}), flush=True)
    return {"directory": str(directory), "scheduled": len(schedule), "replayed": captures,
            "unsupported": unsupported, "verified_target_landings": landings,
            "summary_sha256": hashlib.sha256(summary_path.read_bytes()).hexdigest(),
            "checkpoint_loaded": False,
            "timeout_endpoints": {"count": len(timeout_endpoints),
                "median_horizontal_offset_m": statistics.median(item[0] for item in timeout_endpoints) if timeout_endpoints else None,
                "median_vertical_above_upright_target_m": statistics.median(item[1] for item in timeout_endpoints) if timeout_endpoints else None,
                "median_speed_mps": statistics.median(item[2] for item in timeout_endpoints) if timeout_endpoints else None},
            "incoming_contacts": {"count": len(incoming_speeds),
                "median_speed_mps": statistics.median(incoming_speeds) if incoming_speeds else None}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--campaign", required=True, type=Path, action="append")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    reports = [verify_campaign(path) for path in args.campaign]
    with args.output.open("x") as stream:
        json.dump({"schema_version": 1, "campaigns": reports, "all_passed": True}, stream, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
