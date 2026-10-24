"""Frozen allocation runner; every output is create-only."""
import argparse
import json
import traceback
from pathlib import Path
from actuation_study import Budget, capture, compact, finding, native, read, sha, summary, write_new


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("allocation", type=Path)
    args = parser.parse_args()
    allocation = read(args.allocation)
    if sha(native.__file__) != allocation["native_sha256"]:
        raise RuntimeError("native library differs from allocation")
    for item in allocation["cases"]:
        if sha(item["path"]) != item["sha256"]:
            raise RuntimeError("case input drift")
        c = read(item["path"])
        if sha(c["scenario_path"]) != c["scenario_sha256"]:
            raise RuntimeError("scenario input drift")
    root = Path(allocation["output"])
    root.mkdir()
    budget = Budget(root, allocation)
    for key,value in allocation.get("carried_counts",{}).items():
        budget.counts[key]=budget.counts.get(key,0)+value
    rows, captures = [], []
    write_new(root / "runtime.json", {"native_path": str(Path(native.__file__)), "native_sha256": sha(native.__file__),
        "compiled_sources": json.loads(native.source_identity_json()), "allocation_sha256": sha(args.allocation)})
    try:
        for item in allocation["cases"]:
            case = read(item["path"])
            for mode in item["modes"]:
                result = summary(budget, case, mode)
                row = {"case": case["id"], "case_path": item["path"], "mode": mode,
                       "population": case["population"], "result": compact(result["info"])}
                if "legacy_summary" in case:
                    baseline = read(case["legacy_summary"])["info"]
                    row["legacy"] = compact(baseline)
                    if mode == "legacy_v1" or case["choice"] == "direct":
                        if row["result"] != row["legacy"]:
                            raise RuntimeError("retained legacy/direct preservation failed")
                rows.append(row)
                write_new(root / "summaries" / (case["id"] + "-" + mode + ".json"), result)
                print(json.dumps({"case": case["id"], "mode": mode, **row["result"], "work": budget.total()}), flush=True)
        selected = [tuple(x) for x in allocation["captures"]]
        if len(selected)>allocation["max_captures"]:
            raise RuntimeError("capture allocation exceeded")
        lookup = {(r["case"], r["mode"]): r for r in rows}
        for key in selected:
            row = lookup[key]
            case = read(row["case_path"])
            info = read(root / "summaries" / (row["case"] + "-" + row["mode"] + ".json"))["info"]
            captures.append(capture(budget, case, Path(row["case_path"]), row["mode"],
                root / "captures" / (row["case"] + "-" + row["mode"]), info, args.allocation))
            print(json.dumps({"proof": row["case"], "mode": row["mode"], "work": budget.total()}), flush=True)
        write_new(root / "result.json", {"rows": rows, "captures": captures, "findings": finding(rows), "resources": budget.receipt()})
    except BaseException as error:
        write_new(root / "fault.json", {"error": repr(error), "traceback": traceback.format_exc(),
            "rows": rows, "captures": captures, "counts": budget.counts})
        raise


if __name__ == "__main__":
    main()
