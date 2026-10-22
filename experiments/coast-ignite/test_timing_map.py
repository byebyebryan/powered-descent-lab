import collections
import copy
import json
import tempfile
import unittest
from pathlib import Path

from coast_ignite import encoded, native
from render_timing_map import capture_groups, map_widget
from run_timing_map import execute
from timing_analysis import SLOTS, components, fixed_action, map_summary, metric


class TimingTests(unittest.TestCase):
    def test_full_integer_domain_and_never_preserve_last_wait(self):
        self.assertEqual(len(SLOTS), 82)
        for slot in SLOTS:
            actions = [(tick, fixed_action(slot, tick)) for tick in range(0, 961, 12)]
            self.assertEqual([t for t, action in actions if action], [] if slot is None else [slot * 12])
        for bad in [-1, 81, 1.5, True]:
            with self.assertRaises(ValueError):
                fixed_action(bad, 0)
        with self.assertRaises(ValueError):
            fixed_action(1, 2)

    def example_map(self):
        rows = []
        for slot in SLOTS:
            success = slot in [10, 11, 12, 30, 31]
            info = {"decisions": [[0, 0], [12 * slot, 1]] if slot is not None else [[0, 0]], "native_success": success}
            fuel = {10: 100, 11: 99, 12: 101, 30: 105, 31: 106}.get(slot, 0)
            rows.append({"slot": slot, "native_success": success, "physical_sha256": "same",
                         "info": info, "metrics": {"admitted": slot is not None,
                         "fuel_used_kg": fuel}})
        case = {"id": "case", "cohort": "development", "arc": 0, "physical_sha256": "same",
                "policies": {"M": {"info": rows[10]["info"]}, "G": {"info": rows[20]["info"]}}}
        return case, rows

    def test_failed_low_fuel_cannot_win_and_disconnected_windows_survive(self):
        case, rows = self.example_map()
        result = map_summary(case, rows)
        self.assertEqual(result["best_slot"], 11)
        self.assertEqual(result["best_fuel_kg"], 99)
        self.assertEqual(result["successful_powered_components"], components([10, 11, 12, 30, 31]))
        self.assertEqual(len(result["successful_powered_components"]), 2)
        self.assertTrue(result["best_neighbors"]["two_sided_success"])
        self.assertEqual(result["policies"]["M"]["fuel_regret_kg"], 1)
        self.assertIsNone(result["policies"]["G"]["fuel_regret_kg"])

    def test_incomplete_duplicate_and_changed_policy_are_faults(self):
        case, rows = self.example_map()
        for broken in [rows[:-1], rows[:-1] + [rows[0]]]:
            with self.assertRaises(ValueError):
                map_summary(case, broken)
        case["policies"]["M"]["info"] = {"decisions": [[0, 1]]}
        with self.assertRaises(ValueError):
            map_summary(case, rows)

    def test_neighbor_at_boundary_is_missing_and_passive_success_retained(self):
        case, rows = self.example_map()
        rows[0]["native_success"] = True
        rows[0]["metrics"]["fuel_used_kg"] = 1
        self.assertIsNone(map_summary(case, rows)["best_neighbors"]["left"])
        rows[-1]["native_success"] = True
        rows[-1]["metrics"]["admitted"] = False
        result = map_summary(case, rows)
        self.assertTrue(result["has_successful_point"])
        self.assertIsNone(result["best_slot"])
        self.assertEqual(result["successful_powered_schedules"], 6)

    def test_rich_capture_roles_do_not_duplicate_payloads(self):
        captures = [{"capture": "/root/" + name, "id": name, "cohort": cohort}
                    for name, cohort in [("a", "development"), ("b", "diagnostic"), ("c", "diagnostic")]]
        roles = [{"id": "a", "capture_key": "a", "role": "best"},
                 {"id": "a", "capture_key": "a", "role": "pilot"},
                 {"id": "b", "capture_key": "b", "role": "best"}]
        groups = capture_groups(captures, roles)
        self.assertEqual([g[0] for g in groups], ["report", "report-diagnostics", "report-probes"])
        self.assertEqual(sum(len(g[2]) for g in groups), 3)

    def test_widget_escapes_json_script_boundaries(self):
        markup = map_widget([{"id": "</script><script>bad()", "arc": 0}])
        self.assertNotIn('"id":"</script>', markup)
        self.assertIn('\\u003c/script>', markup)

    def test_native_contact_censors_later_request_and_preserves_coast(self):
        project = Path(__file__).resolve().parents[2]
        anchor = json.loads((project / "fixtures/scenarios/flat_terminal_descent.json").read_text())
        proposal = next(p for k in range(256) if (p := json.loads(native.generate_proposal(
            encoded(anchor), 81131, "timing-software-only", k)))["admitted"])
        task = {k: copy.deepcopy(proposal[k]) for k in ["scenario", "reference_end", "physical_sha256"]}
        task["scenario"]["initial_state"] = {"position_m": {"x": 0., "y": 5.01},
            "velocity_mps": {"x": 0., "y": -2.}, "attitude_rad": 0., "angular_rate_radps": 0.}
        task["reference_end"] = 1200
        class Budget:
            def __init__(self, root): self.root, self.counts = root, collections.Counter()
            def check(self): pass
            def count(self, key, value): self.counts[key] += value
            def remaining_steps(self): return 100000
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            case = {"id": "software", "cohort": "software", "arc": 0, "task_path": str(root / "input.json")}
            allocation = {"models": {}, "randomization_key": "software-only"}
            a = execute(case, task, allocation, Budget(root), slot=60)
            b = execute(case, task, allocation, Budget(root), slot=None)
        self.assertEqual(a["info"], b["info"])
        self.assertIsNone(a["metrics"]["ignition_tick"])
        self.assertLess(a["metrics"]["stop_tick"], 12)
        self.assertEqual(a["metrics"]["fuel_used_kg"], 0)
        self.assertEqual(a["info"]["decisions"], [[0, 0]])
        changed = copy.deepcopy(a["diagnostics"])
        changed["activation"] = {"fuel_kg": task["scenario"]["vehicle"]["initial_fuel_kg"] - 1}
        with self.assertRaises(ValueError):
            metric(task, a["info"], changed)


if __name__ == "__main__":
    unittest.main()
