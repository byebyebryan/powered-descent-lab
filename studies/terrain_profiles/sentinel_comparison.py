"""Cross-source diagnostic comparison only; never a flight safety tolerance."""

import copy
import json
import math
from pathlib import Path
import subprocess

CONTRACT_PATH = Path(__file__).with_suffix(".json")
LATEST_CONTRACT_PATH = CONTRACT_PATH.with_name("sentinel_comparison_v2.json")


def current_contract():
    return json.loads(CONTRACT_PATH.read_bytes())

def latest_contract():
    return json.loads(LATEST_CONTRACT_PATH.read_bytes())


def contract_filename(contract):
    return LATEST_CONTRACT_PATH.name if contract == latest_contract() else CONTRACT_PATH.name


def compare_files(actual, baseline, contract=None, binary=None):
    validate_contract(contract)
    if contract != latest_contract():
        return compare(json.loads(actual.read_bytes()), json.loads(baseline.read_bytes()), contract)
    # One native authority for typed serde hashes, shared with report verification.
    binary = binary or Path(__file__).resolve().parents[2] / "target/release/pd-eval"
    result = subprocess.run([str(binary), "compare-terrain-flights", "--actual", str(actual),
                             "--baseline", str(baseline), "--contract-id", contract["id"]],
                            capture_output=True, timeout=60)
    if result.returncode != 0:
        raise ValueError("native flight preservation: " + result.stderr.decode(errors="replace").strip())
    exceptions = json.loads(result.stdout)
    if not isinstance(exceptions, list):
        raise ValueError("native comparison did not return an exception list")
    return exceptions


def validate_contract(contract):
    # Absent is the historical exact contract, not an automatic upgrade.
    if contract is not None and contract not in (current_contract(), latest_contract()):
        raise ValueError("unknown sentinel comparison contract")


def finite_number(value):
    return type(value) in (int, float) and math.isfinite(value)


def non_timing(flight):
    value = copy.deepcopy(flight)
    timings = value.pop("timings", None)
    if (not isinstance(timings, dict) or set(timings) != {"planning_s", "execution_s", "replay_s"}
            or not all(finite_number(v) and v >= 0 for v in timings.values())):
        raise ValueError("unexpected flight timing exclusions")
    return value


def compare(actual, baseline, contract=None):
    """Return an audited exception list, or fail on any unpermitted difference."""
    validate_contract(contract)
    if contract == latest_contract():
        raise ValueError("v2 requires native file comparison for authenticated proposal hashes")
    actual, baseline = non_timing(actual), non_timing(baseline)
    exceptions = []

    def walk(a, b, path, parent_a=None, parent_b=None):
        if isinstance(a, dict) and isinstance(b, dict):
            if a.keys() != b.keys():
                raise ValueError(f"flight object keys differ at {path}")
            for key in sorted(a):
                walk(a[key], b[key], f"{path}/{key}", a, b)
        elif isinstance(a, list) and isinstance(b, list):
            if len(a) != len(b):
                raise ValueError(f"flight array length differs at {path}")
            for index, (left, right) in enumerate(zip(a, b)):
                walk(left, right, f"{path}/{index}")
        elif type(a) in (int, float) and type(b) in (int, float):
            if not finite_number(a) or not finite_number(b):
                raise ValueError(f"non-finite flight number at {path}")
            if a == b:
                return
            parts = path.split("/")
            pattern = "/".join(parts[:2] + ["*"] + parts[3:]) if len(parts) > 2 and parts[2].isdigit() else path
            if contract is None or pattern not in contract["allowed_paths"]:
                raise ValueError(f"flight value differs at {path}")
            required_a, required_b = parent_a.get("required_clearance_m"), parent_b.get("required_clearance_m")
            delta = a - b
            if (not finite_number(required_a) or not finite_number(required_b) or required_a != required_b
                    or abs(delta) > contract["absolute_tolerance_m"]
                    or (a >= required_a) != (b >= required_b)
                    or (a > 0) != (b > 0) or (a < 0) != (b < 0)):
                raise ValueError(f"clearance exception exceeds bound or crosses a threshold at {path}")
            exceptions.append({"path": path, "baseline": b, "actual": a, "delta_m": delta})
        elif type(a) is not type(b) or a != b:
            raise ValueError(f"flight value differs at {path}")

    walk(actual, baseline, "")
    return exceptions
