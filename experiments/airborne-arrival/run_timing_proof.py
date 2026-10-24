"""Complete a declared saved Summary roster's proof without selecting new inputs."""
import argparse
import json
import traceback
from pathlib import Path

from timing_study import Budget, FUEL, capture, native, read, sha, write_new


def main():
    p = argparse.ArgumentParser();p.add_argument("allocation", type=Path);args = p.parse_args()
    a = read(args.allocation)
    assert sha(native.__file__) == a["native_sha256"]
    assert sha(__file__) == a["runner_sha256"]
    root = Path(a["output"]);root.mkdir();budget = Budget(root, a)
    write_new(root/"runtime.json", {"native_path": str(Path(native.__file__)), "native_sha256": sha(native.__file__),
        "compiled_sources": json.loads(native.source_identity_json()), "runner_path": str(Path(__file__).resolve()),
        "runner_sha256": sha(__file__), "allocation_sha256": sha(args.allocation)})
    captures = []
    try:
        assert len(a["roles"]) <= a["max_captures"]
        for role in a["roles"]:
            assert sha(role["case_path"]) == role["case_sha256"]
            assert sha(role["summary_path"]) == role["summary_sha256"]
            case = read(role["case_path"])
            assert sha(case["scenario_path"]) == case["scenario_sha256"]
            expected = read(role["summary_path"])["info"]
            retained = Path(a["retained_root"])/"confirmation/captures"/(case["id"]+"-native_pair_v2")
            proof = capture(budget, case, role["case_path"], FUEL, root/"captures"/(case["id"]+"-"+FUEL), expected, args.allocation, str(retained))
            captures.append({**proof, "case_path": role["case_path"], "summary_path": role["summary_path"]})
            print(json.dumps({"proof": case["id"], "work": budget.total()}), flush=True)
        assert sha(__file__) == a["runner_sha256"]
        write_new(root/"result.json", {"captures": captures, "resources": budget.receipt(),
            "scope": "Complete proof of every previously uncaptured new-selector Summary; no new population or policy selection"})
    except BaseException as error:
        write_new(root/"fault.json", {"error": repr(error), "traceback": traceback.format_exc(),
            "captures": captures, "counts": budget.counts})
        raise


if __name__ == "__main__":
    main()
