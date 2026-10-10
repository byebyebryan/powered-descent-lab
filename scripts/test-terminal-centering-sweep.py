"""No-flight checks for revised admission; old criteria and native sources stay frozen."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("centering_sweep", Path(__file__).with_name("run-terminal-centering-sweep.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class SweepAdmissionTests(unittest.TestCase):
    def test_contract_is_same_population_candidate_and_bounds(self):
        value = {**runner.fixed_contract(), **{k: "a" * 64 for k in runner.VARIABLE_HASHES}}
        self.assertEqual(value["maximum_measured_attempts"], 1002)
        self.assertEqual(value["repeat_indices"], [142, 974])
        self.assertEqual(value["candidate_executable_sha256"], runner.NATIVE_SHA)
        self.assertEqual(value["retries"], 0)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "plan.json"
            runner.study.write_new(path, runner.study.encoded(value))
            self.assertEqual(runner.contract(path), value)
            for i, (key, bad) in enumerate((("primary_cases", 999), ("correction_cap", 48),
                    ("publication", True), ("candidate_executable_sha256", "b" * 64),
                    ("gate_panel_receipt_sha256", "b" * 64), ("admission_runner_sha256", "invalid"))):
                target = Path(directory) / f"bad-{i}.json"
                runner.study.write_new(target, runner.study.encoded({**value, key: bad}))
                with self.assertRaises(ValueError):
                    runner.contract(target)

    def control(self):
        return {"updates": [{"phase": "ballistic"}, {"phase": "maintained_terminal"}],
                "ordinary_flight": {"actions": [{"physics_step": 0, "command": 1}, {"physics_step": 2, "command": 2}]},
                "refreshes": [{"decision": "maintained_landing_entry"}], "handoffs": []}

    def test_only_post_entry_commands_may_change(self):
        old = self.control()
        new = copy.deepcopy(old)
        new["ordinary_flight"]["actions"][1]["command"] = 3
        runner.control_prefix(new, old)
        new["ordinary_flight"]["actions"][0]["command"] = 4
        with self.assertRaises(ValueError):
            runner.control_prefix(new, old)

    def test_missing_shifted_entry_or_waypoint_is_rejected(self):
        for key, bad in (("updates", [{"phase": "ballistic"}] * 2), ("handoffs", [{}]),
                         ("refreshes", [{"decision": "changed"}])):
            new = self.control()
            new[key] = bad
            with self.assertRaises(ValueError):
                runner.control_prefix(new, self.control())
        new = self.control()
        new["ordinary_flight"]["actions"][1]["physics_step"] = 4
        with self.assertRaises(ValueError):
            runner.control_prefix(new, self.control())

    def test_new_runner_does_not_change_old_contract(self):
        self.assertNotEqual(runner.PROTOCOL, runner.prior.PROTOCOL)
        self.assertEqual(runner.prior.fixed_contract()["experiment_plan"], runner.prior.PROTOCOL)
        self.assertEqual(runner.fixed_contract()["candidate_id"], runner.sweep.CENTERING_CANDIDATE)
        self.assertEqual(runner.fixed_contract()["admission_id"], "terminal-centering-outcome-preservation-v1")


if __name__ == "__main__":
    unittest.main()
