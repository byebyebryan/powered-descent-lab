"""Output-free fixed diagnostic authorization tests; no flights or captures."""
import copy
import unittest
from unittest.mock import patch

import ballistic_exit_diagnostic as diagnostic


class ExitDiagnosticTests(unittest.TestCase):
    def source(self):
        value = {k: v for k, v in diagnostic.IDENTITY.items() if k != "report_script_sha256"}
        value["files"] = {"pd-report/src/planning_cycles.js": diagnostic.IDENTITY["report_script_sha256"]}
        return value

    def focus(self):
        winners = set(diagnostic.panel.PRESERVED + diagnostic.panel.DIRECT)
        rows = [{"case_id": f"random-{i:03}", "cohort": "random",
                 "previous_result": {"verified_landing": i in winners},
                 "result": {"verified_landing": i in (35, 47, 94) or (i in winners and i not in (142, 715))}}
                for i in diagnostic.panel.FOCUS]
        return {"rows": rows, "stopped_reason": None, "source_after": self.source()}

    def test_fixed_denominator_and_repeats(self):
        primary, repeats = diagnostic.layout()
        self.assertEqual(primary, [f"random-{i:03}" for i in range(1000)])
        self.assertEqual(repeats, ["random-047", "random-142", "random-715", "random-044", "random-020"])
        with self.assertRaises(ValueError):
            diagnostic.layout(True)

    def test_diagnostic_keeps_the_failed_admission(self):
        diagnostic.check_frozen(self.source(), self.focus())
        self.assertFalse(diagnostic.METADATA["panel_admission"]["passed"])
        changed = self.focus()
        changed["stopped_reason"] = "evidence_error"
        with self.assertRaises(ValueError):
            diagnostic.check_frozen(self.source(), changed)

    def test_collector_may_change_but_flight_identity_may_not(self):
        changed = self.source()
        changed["files"]["collector.py"] = "new"
        diagnostic.check_frozen(changed, self.focus())
        for key in diagnostic.IDENTITY:
            changed = copy.deepcopy(self.source())
            if key == "report_script_sha256":
                changed["files"]["pd-report/src/planning_cycles.js"] = "changed"
            else:
                changed[key] = "changed"
            with self.assertRaises(ValueError):
                diagnostic.check_frozen(changed, self.focus())

    def test_landing_regression_does_not_stop_measurement(self):
        rows = [{"attempt_id": str(i), "status": "not_attempted"} for i in range(6)]
        def stopped(root, row, plan):
            return dict(row, status="recorded", result={"verified_landing": False})
        with patch.object(diagnostic.panel.sweep, "attempt", side_effect=stopped):
            measured = diagnostic.measure(None, rows, {"campaign_wall_limit_s": 7200, "workers": 4})
        self.assertEqual([r["attempt_id"] for r in measured], [str(i) for i in range(6)])
        self.assertTrue(all(r["status"] == "recorded" for r in measured))

    def test_time_ceiling_preserves_unattempted_denominator(self):
        rows = [{"attempt_id": str(i), "status": "not_attempted"} for i in range(6)]
        with patch.object(diagnostic.panel.sweep, "attempt", side_effect=lambda root, row, plan: dict(row, status="recorded")):
            measured = diagnostic.measure(None, rows, {"campaign_wall_limit_s": 0, "workers": 4})
        self.assertEqual(len(measured), 6)
        self.assertEqual(sum(r["status"] == "recorded" for r in measured), 4)


if __name__ == "__main__":
    unittest.main()
