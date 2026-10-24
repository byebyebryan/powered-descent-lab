"""Create-only allocated timing study. Every native/proof invocation is charged."""
import argparse
import json
import traceback
from pathlib import Path

from timing_study import (Budget, FIRST, FUEL, assert_fuel_selection, capture,
    capture_roles, choice, compact, native, read, sha, summary, write_new)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("allocation", type=Path)
    args = parser.parse_args()
    a = read(args.allocation)
    if sha(native.__file__) != a["native_sha256"]:
        raise RuntimeError("native library differs from allocation")
    for item in a["cases"]:
        if sha(item["path"]) != item["sha256"]:
            raise RuntimeError("case bytes differ")
        case = read(item["path"])
        if sha(case["scenario_path"]) != case["scenario_sha256"]:
            raise RuntimeError("scenario bytes differ")
    root = Path(a["output"])
    root.mkdir()
    b = Budget(root, a)
    for key, value in a.get("carried_counts", {}).items():
        b.counts[key] = b.counts.get(key, 0)+value
    write_new(root/"runtime.json", {"native_path": str(Path(native.__file__)),
        "native_sha256": sha(native.__file__), "compiled_sources": json.loads(native.source_identity_json()),
        "allocation_sha256": sha(args.allocation)})
    rows, captures = [], []
    try:
        for item in a["cases"]:
            case = read(item["path"])
            for mode in item["modes"]:
                result = summary(b, case, mode)
                info = result["info"]
                if a.get("retained_root"):
                    old = read(Path(a["retained_root"])/"confirmation/summaries"/(case["id"]+"-native_pair_v2.json"))["info"]
                    if mode == FIRST and old != info:
                        raise RuntimeError("default selector changed retained execution")
                    if mode == FUEL and old["arrival"] != info["arrival"]:
                        raise RuntimeError("arrival prefix changed under timing selection")
                if mode == FUEL:
                    assert_fuel_selection(info)
                row = {"case": case["id"], "case_path": item["path"], "population": case["population"],
                    "state_id": case["state_id"], "choice": choice(case), "mode": mode,
                    "result": compact(info), "measured_cost": result["measured_cost"]}
                rows.append(row)
                write_new(root/"summaries"/(case["id"]+"-"+mode+".json"), result)
                print(json.dumps({"case": case["id"], "mode": mode, **row["result"], "work": b.total()}), flush=True)
        selected = capture_roles(a, rows)
        write_new(root/"capture-release.json", {"rule": "fixed plus per-state eligible minimum and all failures" if a.get("capture_best_and_failures") else "fixed",
            "roles": selected, "captures": len(selected)})
        lookup = {(row["case"], row["mode"]): row for row in rows}
        for case_id, mode in selected:
            row = lookup[(case_id, mode)]
            case = read(row["case_path"])
            info = read(root/"summaries"/(case_id+"-"+mode+".json"))["info"]
            retained = str(Path(a["retained_root"])/"confirmation/captures"/(case_id+"-native_pair_v2")) if a.get("retained_root") else None
            captures.append(capture(b, case, row["case_path"], mode, root/"captures"/(case_id+"-"+mode), info, args.allocation, retained))
            print(json.dumps({"proof": case_id, "mode": mode, "work": b.total()}), flush=True)
        write_new(root/"result.json", {"rows": rows, "captures": captures, "resources": b.receipt()})
    except BaseException as error:
        write_new(root/"fault.json", {"error": repr(error), "traceback": traceback.format_exc(),
            "rows": rows, "captures": captures, "counts": b.counts})
        raise


if __name__ == "__main__":
    main()
