"""Shared Python/Rust conformance cases; no measured flights."""

import copy
import json
import unittest

import sentinel_comparison as comparison


def edited(value, edits):
    value = copy.deepcopy(value)
    for edit in edits:
        parts = edit["path"].split("/")[1:]
        parent = value
        for part in parts[:-1]:
            parent = parent[int(part)] if isinstance(parent, list) else parent[part]
        key = int(parts[-1]) if isinstance(parent, list) else parts[-1]
        if edit.get("remove"):
            del parent[key]
        else:
            parent[key] = edit["value"]
    return value


class ComparisonTests(unittest.TestCase):
    def setUp(self):
        self.corpus = json.loads(comparison.CONTRACT_PATH.with_name("sentinel_comparison_cases.json").read_bytes())

    def test_shared_contract_cases(self):
        for case in self.corpus["cases"]:
            with self.subTest(case["name"]):
                baseline = edited(self.corpus["baseline"], case.get("baseline_edits", []))
                actual = edited(baseline, case.get("edits", []))
                contract = None if case.get("exact") else edited(comparison.current_contract(), case.get("contract_edits", []))
                if case["accepted"]:
                    exceptions = comparison.compare(actual, baseline, contract)
                    self.assertEqual(len(exceptions), case["exceptions"])
                    for exception in exceptions:
                        self.assertEqual(exception["delta_m"], exception["actual"] - exception["baseline"])
                        self.assertLessEqual(abs(exception["delta_m"]), 1e-12)
                else:
                    with self.assertRaises(ValueError):
                        comparison.compare(actual, baseline, contract)

    def test_nonfinite_numbers_are_rejected_even_when_both_sides_match(self):
        for path in ("/cycles/0/audit/clearance_scan/minimum_airborne/clearance_m", "/timings/planning_s"):
            for value in (float("nan"), float("inf"), -float("inf")):
                with self.subTest(path=path, value=value):
                    flight = edited(self.corpus["baseline"], [{"path": path, "value": value}])
                    with self.assertRaises(ValueError):
                        comparison.compare(flight, flight, comparison.current_contract())

    def test_exception_order_paths_and_values_are_deterministic(self):
        case = next(c for c in self.corpus["cases"] if c["name"] == "multiple-exceptions")
        actual = edited(self.corpus["baseline"], case["edits"])
        result = comparison.compare(actual, self.corpus["baseline"], comparison.current_contract())
        self.assertEqual(result, comparison.compare(actual, self.corpus["baseline"], comparison.current_contract()))
        self.assertEqual([e["path"] for e in result], [
            "/cycles/0/audit/clearance_scan/first_violation/clearance_m",
            "/cycles/0/audit/clearance_scan/minimum_airborne/clearance_m",
        ])


if __name__ == "__main__":
    unittest.main()
