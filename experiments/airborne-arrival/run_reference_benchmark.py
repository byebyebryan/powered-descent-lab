"""Isolated sequential timing: first invocation, two warmups, seven repeats."""
import argparse
import os
import platform
import traceback
from pathlib import Path

from reference_study import Budget, MODES, compact, native, read, sha, summary, write_new


def main():
    p = argparse.ArgumentParser()
    p.add_argument("allocation", type=Path)
    args = p.parse_args()
    a = read(args.allocation)
    assert a["native_sha256"] == sha(native.__file__)
    root = Path(a["output"])
    root.mkdir()
    budget = Budget(root, a)
    write_new(root/"host.json", {"platform": platform.platform(), "python": platform.python_version(),
        "cpu_count": os.cpu_count(), "affinity": sorted(os.sched_getaffinity(0)),
        "load_start": os.getloadavg(), "allocation_sha256": sha(args.allocation),
        "native_sha256": sha(native.__file__), "protocol": "1 first invocation, 2 warmups, 7 measured; rotated mode order; sequential worker"})
    rows = []
    try:
        for index, item in enumerate(a["cases"]):
            assert sha(item["path"]) == item["sha256"]
            case = read(item["path"])
            assert sha(case["scenario_path"]) == case["scenario_sha256"]
            expected = {}
            for repeat in range(10):
                shift = (index+repeat) % len(MODES)
                for mode in MODES[shift:]+MODES[:shift]:
                    result = summary(budget, case, mode, measured=True)
                    if mode in expected:
                        assert expected[mode] == result["info"], "measurement execution changed"
                    else:
                        expected[mode] = result["info"]
                    row = {"case": case["id"], "mode": mode, "repeat": repeat,
                        "role": "first_invocation" if repeat == 0 else "warmup" if repeat < 3 else "measured",
                        "measurements": result["measurements"], "measured_cost": result["measured_cost"],
                        "result": compact(result["info"])}
                    rows.append(row)
                    write_new(root/"records"/(case["id"]+"-"+mode+f"-{repeat}.json"), row)
            print(case["id"], flush=True)
        write_new(root/"result.json", {"rows": rows, "resources": budget.receipt(), "load_end": os.getloadavg()})
    except BaseException as error:
        write_new(root/"fault.json", {"error": repr(error), "traceback": traceback.format_exc(), "rows": rows, "counts": budget.counts})
        raise


if __name__ == "__main__":
    main()
