"""Create-only allocated composition runner. No optimization or input filtering."""
import argparse
import json
import traceback
from pathlib import Path
from composition_study import Budget, capture, compact, native, read, sha, summary, write_new
from actuation_study import finding


def main():
    p = argparse.ArgumentParser()
    p.add_argument("allocation", type=Path)
    args = p.parse_args()
    a = read(args.allocation)
    if sha(native.__file__) != a["native_sha256"]:
        raise RuntimeError("native library differs from allocation")
    for item in a["cases"]:
        if sha(item["path"]) != item["sha256"]:
            raise RuntimeError("case drift")
        case = read(item["path"])
        if sha(case["scenario_path"]) != case["scenario_sha256"]:
            raise RuntimeError("scenario drift")
    root = Path(a["output"])
    root.mkdir()
    b = Budget(root, a)
    for k, v in a.get("carried_counts", {}).items():
        b.counts[k] = b.counts.get(k, 0) + v
    rows, captures = [], []
    write_new(root / "runtime.json", {"native_path": str(Path(native.__file__)), "native_sha256": sha(native.__file__),
        "compiled_sources": json.loads(native.source_identity_json()), "allocation_sha256": sha(args.allocation)})
    try:
        for item in a["cases"]:
            case = read(item["path"])
            for mode in item["modes"]:
                result = summary(b, case, mode)
                row = {"case": case["id"], "case_path": item["path"], "population": case["population"],
                    "mode": mode, "result": compact(result["info"])}
                if "retained_summary" in case:
                    original = read(case["retained_summary"])
                    if sha(case["retained_summary"]) != case["retained_summary_sha256"]:
                        raise RuntimeError("retained Summary drift")
                    row["retained"] = compact(original["info"])
                    if row["result"] != row["retained"]:
                        raise RuntimeError("retained execution changed without an authorized guidance profile")
                rows.append(row)
                write_new(root / "summaries" / (case["id"] + "-" + mode + ".json"), result)
                print(json.dumps({"case": case["id"], "mode": mode, **row["result"], "work": b.total()}), flush=True)
        selected = [tuple(x) for x in a["captures"]]
        if len(selected) > a["max_captures"]:
            raise RuntimeError("capture allocation exceeded")
        lookup = {(r["case"], r["mode"]): r for r in rows}
        for key in selected:
            row = lookup[key]
            case = read(row["case_path"])
            info = read(root / "summaries" / (row["case"] + "-" + row["mode"] + ".json"))["info"]
            captures.append(capture(b, case, Path(row["case_path"]), row["mode"], root / "captures" / (row["case"] + "-" + row["mode"]), info, args.allocation))
            print(json.dumps({"proof": row["case"], "mode": row["mode"], "work": b.total()}), flush=True)
        write_new(root / "result.json", {"rows": rows, "captures": captures, "findings": finding(rows), "resources": b.receipt()})
    except BaseException as e:
        write_new(root / "fault.json", {"error": repr(e), "traceback": traceback.format_exc(), "rows": rows, "captures": captures, "counts": b.counts})
        raise


if __name__ == "__main__":
    main()
