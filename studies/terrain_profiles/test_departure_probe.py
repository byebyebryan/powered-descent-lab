import copy
import unittest

import cap_probe as cap
import departure_probe as probe
import survey


class DepartureProbeTests(unittest.TestCase):
    def test_fixed_input_and_work_allowance(self):
        plan = probe.contract()
        rows = probe.attempts(plan)
        self.assertEqual(len(rows), 13)
        self.assertEqual([r["case_id"] for r in rows[10:]], ["random-327", "random-024", "random-030"])
        self.assertEqual(plan["maximum_extra_rows"], 8 * 7)
        self.assertFalse(plan["publication"])

    def test_transformation_only_changes_explicit_policy_and_cap(self):
        plan = probe.contract()
        original = (survey.REPO / cap.POLICY_PATH).read_bytes()
        transformed = probe.transform(original, plan)
        self.assertIn(b"maximum_corrections: 6", original)
        definition = b"policy_id: WAYPOINT_V2_POLICY_REVISION_3_ID.into(),\n            maximum_corrections: 24,"
        self.assertIn(definition, transformed)
        self.assertNotEqual(transformed, original)
        with self.assertRaises(ValueError):
            probe.transform(transformed, plan)

    def test_non_subject_comparison_keeps_proofs_and_numerics(self):
        original = {"policy": {"policy_id": "baseline"}, "input_identity": "baseline",
                    "timings": {"planning_s": 1.0, "execution_s": 0.0, "replay_s": 0.0},
                    "ordinary_flight": {"fuel": 42.0}, "proof": True}
        actual = copy.deepcopy(original)
        actual.update(policy={"policy_id": "experiment"}, input_identity="experiment")
        probe.compare(actual, original)
        actual["proof"] = False
        with self.assertRaises(ValueError):
            probe.compare(actual, original)

    def test_subject_only_extends_an_exhausted_source_search(self):
        old_local = {"accepted_row_count": 0, "entries": [{"admitted": True}],
                     "row_diagnostics": [{"row_id": "old"}], "row_count": 1, "boundary_count": 360}
        original = {"cycles": [{"decision": "no_clearing", "local_search": old_local,
                                "audit": {"clearance_scan": {"first_violation": {"phase": "source_bridge"}}}}],
                    "absolute_deadline_physics_step": 10800, "ordinary_flight": {"fuel": 42.0}, "segments": []}
        actual = copy.deepcopy(original)
        local = actual["cycles"][0]["local_search"]
        local.update(row_count=8, boundary_count=2880, selected=None)
        local["row_diagnostics"] += [{"row_id": f"departure_row_{i}"} for i in range(7)]
        self.assertFalse(probe.compare(actual, original, True)["executed_handoff"])
        local["row_diagnostics"][0]["row_id"] = "changed"
        with self.assertRaises(ValueError):
            probe.compare(actual, original, True)


if __name__ == "__main__":
    unittest.main()
