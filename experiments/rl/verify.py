"""Independent saved-command verification; never imports or loads a PPO policy."""
import argparse
import hashlib
import json
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
    captures, landings, unsupported = 0, 0, 0
    for index, (mission_index, attempt, row) in enumerate(zip(schedule, attempts, rows)):
        if attempt["attempt_index"] != index or attempt["mission_index"] != mission_index or row["mission_index"] != mission_index:
            raise ValueError("saved schedule differs from request")
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
        if captures % 128 == 0:
            print(json.dumps({"campaign": str(directory), "replayed": captures}), flush=True)
    return {"directory": str(directory), "scheduled": len(schedule), "replayed": captures,
            "unsupported": unsupported, "verified_target_landings": landings,
            "summary_sha256": hashlib.sha256(summary_path.read_bytes()).hexdigest(),
            "checkpoint_loaded": False}


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
