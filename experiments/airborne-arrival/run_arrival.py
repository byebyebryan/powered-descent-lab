"""Create-only frozen-allocation runner. Run from a retained source epoch."""
import argparse
import itertools
import json
import traceback
from pathlib import Path

from arrival import Budget, capture, compact, execute_summary, findings, native, program, read, sha, write_new


def main():
    p = argparse.ArgumentParser()
    p.add_argument("allocation", type=Path)
    args = p.parse_args()
    a = read(args.allocation)
    root = Path(a["output"])
    root.mkdir()
    budget = Budget(root, a)
    rows, proofs = [], []
    write_new(root / "runtime.json", {"native": str(Path(native.__file__)), "native_sha256": sha(Path(native.__file__)),
              "compiled_sources": json.loads(native.source_identity_json()), "allocation_sha256": sha(args.allocation)})
    try:
        for item in a["cases"]:
            s, path = read(item["path"]), Path(item["path"])
            for choice in item["choices"]:
                result = execute_summary(budget, s, choice)
                row = {"case": s["id"], "input": str(path), "choice": choice, "result": compact(result["info"])}
                rows.append(row)
                write_new(root / "summaries" / f'{s["id"]}-{choice}.json', result)
                print(json.dumps({"case": s["id"], "choice": choice, **row["result"], "work_total": budget.total()}), flush=True)
        f = findings(rows)
        write_new(root / "summary.json", {"rows": rows, "findings": f, "resources": budget.receipt()})
        selected = set(tuple(x) for x in a["fixed_captures"])
        if a.get("capture_best"):
            selected.update((case, choice) for case, choice in f["best_tested"].items() if choice)
        if len(selected) > a["max_captures"]:
            raise RuntimeError("capture allocation exceeded")
        lookup = {(r["case"], r["choice"]): r for r in rows}
        for case, choice in sorted(selected):
            r = lookup[(case, choice)]
            expected = read(root / "summaries" / f"{case}-{choice}.json")["info"]
            proof = capture(budget, read(r["input"]), Path(r["input"]), choice,
                            root / "captures" / f"{case}-{choice}", expected, args.allocation)
            proofs.append(proof)
            print(json.dumps({"proof": case, "choice": choice, "verified": proof["verified_target_landing"], "work_total": budget.total()}), flush=True)
        write_new(root / "result.json", {"rows": rows, "findings": f, "captures": proofs, "resources": budget.receipt()})
    except BaseException as e:
        write_new(root / "fault.json", {"error": repr(e), "traceback": traceback.format_exc(), "counts": budget.counts,
                                      "completed_rows": len(rows), "completed_proofs": len(proofs)})
        raise


if __name__ == "__main__":
    main()
