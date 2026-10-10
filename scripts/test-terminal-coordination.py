"""Pure admission/prefix/diagnostic checks; no saved outputs or measured flights."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("coordination", Path(__file__).with_name("run-terminal-coordination.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class CoordinationTests(unittest.TestCase):
    def test_frozen_cohort_keeps_unverified_losses_in_denominator(self):
        old, current = [], []
        for i in range(1000):
            name = f"random-{i:03}"
            old.append({"case_id": name, "cohort": "random", "result": {"verified_landing": i >= 75}})
            result = None if 100 <= i < 120 else {"verified_landing": i < 75 or i >= 120}
            current.append({"case_id": name, "cohort": "random", "result": result})
        gains, losses = runner.cohorts({"rows": old}, {"rows": current})
        self.assertEqual((len(gains), len(losses)), (75, 45))
        self.assertIn("random-119", losses)
        current.pop()
        with self.assertRaises(ValueError):
            runner.cohorts({"rows": old}, {"rows": current})

    def test_admission_requires_gains_controls_upright_and_recovery(self):
        names = ["gain", "loss", "random-559", "random-757", "random-888"] + runner.controls.CONTROLS
        rows = [{"case_id": name, "cohort": "random", "result": {"verified_landing": True}} for name in names]
        self.assertTrue(runner.admission(rows, ["gain"], ["loss"])["passed"])
        for name in names:
            changed = copy.deepcopy(rows)
            next(r for r in changed if r["case_id"] == name)["result"] = None
            self.assertFalse(runner.admission(changed, ["gain"], ["loss"])["passed"])

    def test_terminal_commands_may_change_but_prefix_may_not(self):
        flight = {"ordinary_flight": {"actions": [{"physics_step": 0, "command": "prefix"},
                                                   {"physics_step": 2, "command": "terminal"}]},
                  "updates": [{"physics_step": 0, "command": "prefix"},
                              {"physics_step": 2, "command": "terminal"}], "handoffs": [],
                  "refreshes": [{"decision": "maintained_landing_entry", "origin": {"physics_step": 2}}]}
        changed = copy.deepcopy(flight)
        changed["ordinary_flight"]["actions"][1]["command"] = "new terminal"
        changed["updates"][1]["command"] = "new terminal"
        runner.prefix_equal(changed, flight)
        changed["updates"][0]["command"] = "new prefix"
        with self.assertRaises(ValueError):
            runner.prefix_equal(changed, flight)

    def test_diagnostic_authority_does_not_pass_failed_conditional_gate(self):
        names = ["gain", "loss", "random-559", "random-757", "random-888"] + runner.controls.CONTROLS
        rows = [{"case_id": name, "cohort": "random", "result": {"verified_landing": name != "gain"}}
                for name in names]
        seal = {"executable_sha256": "binary", "rust_source_tree_sha256": "rust",
                "renderer_sha256": "renderer", "files": {"pd-report/src/planning_cycles.js": "js"}}
        admitted = {"rows": rows, "stopped_reason": None, "source_after": seal}
        with self.assertRaisesRegex(ValueError, "conditional sweep is closed"):
            runner.check_sweep_admission("sweep", admitted, ["gain"], ["loss"], seal)
        changed_collector = copy.deepcopy(seal)
        changed_collector["files"][runner.RUNNER] = "new collector"
        verdict = runner.check_sweep_admission("diagnostic-sweep", admitted, ["gain"], ["loss"], changed_collector)
        self.assertFalse(verdict["passed"])
        self.assertEqual(verdict["lost_gains"], ["gain"])
        for key in ("executable_sha256", "rust_source_tree_sha256", "renderer_sha256"):
            drift = copy.deepcopy(changed_collector)
            drift[key] = "changed"
            with self.assertRaisesRegex(ValueError, "frozen flight candidate changed"):
                runner.check_sweep_admission("diagnostic-sweep", admitted, ["gain"], ["loss"], drift)
        drift = copy.deepcopy(changed_collector)
        drift["files"]["pd-report/src/planning_cycles.js"] = "changed"
        with self.assertRaises(ValueError):
            runner.check_sweep_admission("diagnostic-sweep", admitted, ["gain"], ["loss"], drift)

    def test_domain_diagnostic_cannot_claim_executed_contact(self):
        metadata = {"enabled": True,
                    "early_braking": "preserve_nominal_lateral_acceleration_with_lift_constraint",
                    "touchdown": "body_contained_low_energy_upright_settlement",
                    "ordinary_default_changed": False, "standalone_coast_terminal_changed": False,
                    "physical_guards_changed": False}
        actual = {"physics_step": 10, "physical_outcome": "flying"}
        diagnostic = {"query": "short_command_prediction", "actual_state": actual,
                      "query_origin": actual, "query_state": {"physics_step": 12}, "requested_command": {},
                      "error": {"DomainOverrun": {"x_m": 101.0, "domain_min_x_m": 0.0, "domain_max_x_m": 100.0}}}
        feedback = {"candidate_id": runner.sweep.COORDINATION_CANDIDATE,
                    "stop": "prediction_terrain_domain", "final_state": actual, "terrain_domain_stop": diagnostic}
        runner.sweep.validate_terminal_coordination({"terminal_coordination": metadata}, feedback)
        changed = copy.deepcopy(feedback)
        changed["final_state"]["physical_outcome"] = "crashed"
        with self.assertRaises(ValueError):
            runner.sweep.validate_terminal_coordination({"terminal_coordination": metadata}, changed)


if __name__ == "__main__":
    unittest.main()
