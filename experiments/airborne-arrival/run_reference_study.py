"""Create-only fixed schedule. Preserve physical misses; stop software faults."""
import argparse
import json
import traceback
from pathlib import Path

from reference_study import Budget, compact, native, read, sha, summary, capture, write_new


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("allocation", type=Path)
    args = parser.parse_args()
    a = read(args.allocation)
    assert sha(native.__file__) == a["native_sha256"]
    for item in a["cases"]:
        assert sha(item["path"]) == item["sha256"]
        case = read(item["path"])
        assert sha(case["scenario_path"]) == case["scenario_sha256"]
    root = Path(a["output"])
    root.mkdir()
    budget = Budget(root, a)
    write_new(root/"runtime.json", {"native_path": str(Path(native.__file__)),
        "native_sha256": sha(native.__file__), "compiled_sources": json.loads(native.source_identity_json()),
        "allocation_sha256": sha(args.allocation)})
    rows, captures = [], []
    try:
        for item in a["cases"]:
            case = read(item["path"])
            for mode in item["modes"]:
                result = summary(budget, case, mode)
                if item.get("expected_root"):
                    expected = read(Path(item["expected_root"])/(case["id"]+"-"+mode+".json"))["info"]
                    assert expected == result["info"], "retained profile changed"
                row = {"case": case["id"], "case_path": item["path"], "population": case["population"],
                    "state_id": case.get("state_id", case["id"]), "mode": mode,
                    "result": compact(result["info"]), "measured_cost": result["measured_cost"]}
                rows.append(row)
                write_new(root/"summaries"/(case["id"]+"-"+mode+".json"), result)
                if mode in item.get("capture_modes", []):
                    if len(captures) >= a["max_captures"]:
                        raise RuntimeError("capture allocation exhausted")
                    proof = capture(budget, case, item["path"], mode, root/"captures"/(case["id"]+"-"+mode),
                        result["info"], args.allocation, item.get("retained_capture"))
                    captures.append(proof)
                print(json.dumps({"case": case["id"], "mode": mode, "success": row["result"]["success"],
                    "fuel_kg": row["result"]["fuel_used_kg"], "steps": budget.total()}), flush=True)
        write_new(root/"result.json", {"rows": rows, "captures": captures, "resources": budget.receipt()})
    except BaseException as error:
        write_new(root/"fault.json", {"error": repr(error), "traceback": traceback.format_exc(),
            "rows": rows, "captures": captures, "counts": budget.counts})
        raise


if __name__ == "__main__":
    main()
