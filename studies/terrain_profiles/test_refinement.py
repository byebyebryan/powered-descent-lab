"""Synthetic/unit checks; only explicit captures establish source/preflight evidence."""

import argparse
import copy
import io
import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import refinement as ref
import study


class RefinementTests(unittest.TestCase):
    def setUp(self):
        self.plan, self.base = ref.plans(ref.HERE / "refinement_plan.json")
        self.template = json.loads((ref.HERE / "scenario_template.json").read_bytes())

    def test_plan_freezes_one_change_and_local_pad_geometry(self):
        for key, value in (("ridge_slice_offset", 0.5), ("landmark_radius_m", 40), ("flight_runs", 1), ("control_plan_sha256", "bad")):
            plan = dict(self.plan, **{key: value})
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "plan.json"
                study.write_new(path, study.encoded(plan))
                with self.assertRaises(ValueError):
                    ref.plans(path)
        plan = copy.deepcopy(self.plan)
        plan["pad_preparation"]["transition_m"] = 100
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "plan.json"
            study.write_new(path, study.encoded(plan))
            with self.assertRaises(ValueError):
                ref.plans(path)

    def test_slice_wrapper_changes_only_second_coordinate(self):
        class Noise:
            def noise2(self, x, y):
                return (x, y)
        noise = Noise()
        self.assertEqual(ref.RidgeSlice(noise, 0).noise2(2.5, 67), noise.noise2(2.5, 67))
        self.assertEqual(ref.RidgeSlice(noise, 0.37).noise2(2.5, 67), (2.5, 67.37))

    def test_ridge_wrapper_leaves_other_channels_unchanged(self):
        class Noise:
            def noise2(self, x, y):
                return math.sin(x + y) * 0.5
        class Generator:
            structure_octaves = 3
            structure_frequency = 1 / 600
            structure_persistence = 0.35
            structure_lacunarity = 2
            ridge_mix = 0.45
            structure_amplitude = 140
            _structure_noise = Noise()
            _ridge_noise = Noise()
            def _warped_x(self, x):
                return x + math.sin(x * 0.001) * 25
        before, after = Generator(), Generator()
        after._ridge_noise = ref.RidgeSlice(after._ridge_noise, 0.37)
        zero = Generator()
        zero._ridge_noise = ref.RidgeSlice(zero._ridge_noise, 0)
        for x in (-160, 137, 463, 1063, 1360):
            self.assertEqual(study.weighted_structure(before, x, 0.65), study.weighted_structure(zero, x, 0.65))
            self.assertEqual(before._warped_x(x), after._warped_x(x))
            self.assertEqual(before._structure_noise.noise2(x, 23), after._structure_noise.noise2(x, 23))
        self.assertNotEqual(study.weighted_structure(before, 463, 0.65), study.weighted_structure(after, 463, 0.65))

    def test_peak_descriptor_keeps_narrow_peaks_and_plateau_start(self):
        profile = {"points_m": [[0, 0], [1, 4], [2, 4], [3, 0], [4, 1], [5, 0]]}
        self.assertEqual(ref.peaks(profile, self.base), [1, 4])
        profile["points_m"] = [[x, x] for x in range(6)]
        self.assertEqual(ref.peaks(profile, self.base), [])

    def test_comparison_plot_keeps_endpoint_label_inside_viewport(self):
        before = {"variant": "refined", "seed": 0, "points_m": [[x, x * 0.01] for x in range(-160, 1361)]}
        after = dict(before)
        plot = ref.comparison_chart(before, after, self.base, (-50, 50), 525, 250)
        self.assertIn('x="513.00" y="235" text-anchor="end">1200</text>', plot)
        self.assertIn("Original refined control", plot)
        self.assertEqual(before["variant"], "refined")

    def test_authoritative_query_is_strict_and_piecewise_linear(self):
        points = [[0, 1], [4, 5], [8, 1]]
        self.assertEqual(ref.height_at(points, 2), 3)
        self.assertEqual(ref.height_at(points, 8), 1)
        for x in (-1, 9, float("nan")):
            with self.assertRaises(ValueError):
                ref.height_at(points, x)

    def test_pad_compiler_has_exact_shelves_and_preserves_outside_vertices(self):
        raw = [[x, math.sin(x * 0.01) * 40 + x * 0.03] for x in range(-160, 1361, 4)]
        old = copy.deepcopy(raw)
        points, patches = ref.prepare_points(raw, self.plan["pad_preparation"])
        self.assertEqual(raw, old)
        self.assertEqual(len(points), len(raw) + 8)
        self.assertEqual([p["edited_interval_m"] for p in patches], [[-42, 42], [1158, 1242]])
        for p in patches:
            left, right = p["shelf_interval_m"]
            self.assertEqual(ref.height_at(points, left), p["surface_y_m"])
            self.assertEqual(ref.height_at(points, right), p["surface_y_m"])
            self.assertTrue(all(y == p["surface_y_m"] for x, y in points if left <= x <= right))
            self.assertEqual(p["surface_y_m"], ref.height_at(raw, p["center_x_m"]))
        by_x = dict(points)
        for x, y in raw:
            if not any(p["edited_interval_m"][0] < x < p["edited_interval_m"][1] for p in patches):
                self.assertEqual(by_x[x], y)
        for x in (43, 250.5, 1157, 1243, 1359):
            self.assertAlmostEqual(ref.height_at(raw, x), ref.height_at(points, x), places=12)

    def test_patch_overlap_or_domain_overrun_is_rejected(self):
        raw = [[x, x * 0.1] for x in range(-160, 1361, 4)]
        pad = dict(self.plan["pad_preparation"], target_center_x_m=40)
        with self.assertRaises(ValueError):
            ref.prepare_points(raw, pad)
        with self.assertRaises(ValueError):
            ref.prepare_points([[0, 0], [1200, 0]], self.plan["pad_preparation"])

    def test_scenario_keeps_vehicle_gravity_clocks_mission_and_uses_primary_surface(self):
        # A narrow reference spike at x=1 is not secretly queried by the game.
        fine = [[x, 500.0 if x == 1 else x * 0.02] for x in range(-160, 1361)]
        original = copy.deepcopy(self.template)
        value, stats = ref.scenario({"variant": "refined", "seed": 42, "points_m": fine}, self.plan, self.base, self.template, b"plan")
        self.assertEqual(self.template, original)
        self.assertEqual(value["vehicle"], self.template["vehicle"])
        self.assertEqual(value["sim"], self.template["sim"])
        self.assertEqual(value["mission"], self.template["mission"])
        self.assertEqual(value["world"]["gravity_mps2"], self.template["world"]["gravity_mps2"])
        self.assertEqual(value["initial_state"]["position_m"], {"x": 0, "y": 5})
        self.assertEqual(value["world"]["landing_pads"][0]["surface_y_m"], 0)
        self.assertTrue(stats["outside_patches_unchanged"])

    def preflight(self):
        return {"schema_id": "planner_v2_cli_flight_v1", "status": "preflight_only", "supported": True,
                "input_identity": "fnv1a64:synthetic_unit_only", "policy": {"policy_id": "piecewise_local_clearing_v2_policy_3", "maximum_corrections": 6},
                "planning_stop": None, "reason": None, "correction_count": 0, "integrity_passed": None,
                "final_source_replay_passed": None, "physical_outcome": None, "mission_outcome": None, "output_dir": None}

    def test_preflight_guard_rejects_flights_missing_fields_or_unsupported(self):
        ref.validate_preflight(self.preflight())
        for key, value in (("status", "completed"), ("supported", False), ("physical_outcome", "landed_on_target"), ("output_dir", "out"), ("policy", {})):
            with self.assertRaises(ValueError):
                ref.validate_preflight(dict(self.preflight(), **{key: value}))
        value = self.preflight()
        del value["mission_outcome"]
        with self.assertRaises(ValueError):
            ref.validate_preflight(value)

    def synthetic_capture(self, root):
        controls, profiles = [], []
        control_hashes = {}
        for variant in ref.VARIANTS:
            for seed in sorted(self.base["seeds"]):
                before = {"variant": variant, "seed": seed, "points_m": [[x, seed + x * 0.01] for x in range(-160, 1361)]}
                after = dict(before, points_m=[[x, seed + x * 0.02] for x in range(-160, 1361)])
                controls.append(before)
                profiles.append(after)
                for folder, p in (("controls", before), ("profiles", after)):
                    value = {"variant": variant, "seed": seed, "profile_only": True,
                             "points_m": study.subset(p["points_m"], 4), "reference_points_m": p["points_m"]}
                    if folder == "profiles":
                        value["ridge_slice_offset"] = 0.37
                    data = study.encoded(value)
                    study.write_new(root / folder / ref.name(p), data)
                    if folder == "controls":
                        control_hashes["profiles/" + ref.name(p)] = study.digest(data)
        receipt_data = study.encoded({"files": control_hashes})
        plan = dict(self.plan, control_receipt_sha256=study.digest(receipt_data))
        plan_data = study.encoded(plan)
        study.write_new(root / "plan.json", plan_data)
        study.write_new(root / "control_receipt.json", receipt_data)
        for relative in ref.SOURCES:
            study.write_new(root / "inputs/study" / relative, (ref.HERE / relative).read_bytes())
        for relative in self.base["pylander"]["files"]:
            study.write_new(root / "inputs/pylander" / relative, b"synthetic; reference loader mocked")
        stats = ref.summary(profiles, controls, plan, self.base, plan_data)
        study.write_new(root / "summary.json", study.encoded(stats))
        study.write_new(root / "overview.svg", ref.sheet(profiles, controls, self.base))
        study.write_new(root / "index.html", ref.gallery(profiles, controls, stats, self.base))
        ref.write_receipt(root, {"schema": plan["schema"], "flight_runs": 0, "profile_only": True,
                                "source_sha256": {p: study.digest((ref.HERE / p).read_bytes()) for p in ref.SOURCES}})
        return plan_data

    def rehash(self, root, filename):
        receipt = json.loads((root / "receipt.json").read_bytes())
        receipt["files"][filename] = study.digest((root / filename).read_bytes())
        (root / "receipt.json").write_bytes(study.encoded(receipt))

    def test_verifier_rejects_rehashed_metrics_and_extra_files(self):
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            root = Path(directory)
            self.synthetic_capture(root)
            expected_plan = root.parent / (root.name + "-plan.json")
            try:
                study.write_new(expected_plan, (root / "plan.json").read_bytes())
                ref.verify(root, expected_plan)
                stats = json.loads((root / "summary.json").read_bytes())
                stats["landmark_alignment"][0]["after"] = 6
                (root / "summary.json").write_bytes(study.encoded(stats))
                self.rehash(root, "summary.json")
                with self.assertRaisesRegex(ValueError, "derived comparison"):
                    ref.verify(root, expected_plan)
                study.write_new(root / "extra.json", b"{}")
                with self.assertRaisesRegex(ValueError, "inventory"):
                    ref.verify(root, expected_plan)
            finally:
                expected_plan.unlink(missing_ok=True)

    def test_controls_cannot_be_forged_by_rehashing_the_new_receipt(self):
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            root = Path(directory)
            self.synthetic_capture(root)
            filename = "controls/refined-seed-0.json"
            value = json.loads((root / filename).read_bytes())
            value["reference_points_m"][0][1] += 1
            (root / filename).write_bytes(study.encoded(value))
            self.rehash(root, filename)
            with self.assertRaisesRegex(ValueError, "frozen control"):
                ref.verify(root, root / "plan.json")

    def test_prepared_verifier_recomputes_pad_geometry_after_rehash(self):
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            parent = Path(directory)
            capture_root = parent / "capture"
            capture_root.mkdir()
            self.synthetic_capture(capture_root)
            prepared = parent / "prepared"
            binary = parent / "synthetic-bin"
            study.write_new(binary, b"unit-only; never executed")
            args = argparse.Namespace(capture=capture_root, plan=capture_root / "plan.json", output=prepared, preflight_bin=binary)
            with patch("refinement.STUDY_OUTPUTS", parent), patch("refinement.checked_preflight", return_value=self.preflight()), patch("sys.stdout", new_callable=io.StringIO):
                ref.prepare(args)
            ref.verify_prepared(prepared, capture_root, args.plan)
            filename = "scenarios/refined-seed-0.json"
            value = json.loads((prepared / filename).read_bytes())
            value["world"]["terrain"]["points_m"][1]["y"] += 1
            (prepared / filename).write_bytes(study.encoded(value))
            self.rehash(prepared, filename)
            with self.assertRaisesRegex(ValueError, "fixed pad compiler"):
                ref.verify_prepared(prepared, capture_root, args.plan)


if __name__ == "__main__":
    unittest.main()
