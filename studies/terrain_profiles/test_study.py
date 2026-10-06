import copy
import json
import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import study


class ProfileTests(unittest.TestCase):
    def setUp(self):
        self.plan = json.loads((study.HERE / "plan.json").read_bytes())

    def test_frozen_plan_and_aligned_halos(self):
        study.validate_plan(self.plan)
        bad = copy.deepcopy(self.plan)
        bad["seeds"][1] = bad["seeds"][0]
        with self.assertRaises(ValueError):
            study.validate_plan(bad)
        bad = copy.deepcopy(self.plan)
        bad["domain_m"][0] = -159
        with self.assertRaises(ValueError):
            study.validate_plan(bad)

    def test_linear_profile_has_no_sampling_loss_or_turns(self):
        points = [[x, 10 + x * 0.5] for x in range(1201)]
        error = study.interpolation_error(points, study.subset(points, 4))
        self.assertEqual(error, {"max_m": 0.0, "rms_m": 0.0})
        self.assertEqual(study.significant_turns(points, 8, 40), [])

    def test_sampling_detects_lost_feature_and_rejects_bad_domain(self):
        fine = [[x, 1.0 if x == 1 else 0.0] for x in range(9)]
        self.assertEqual(study.interpolation_error(fine, study.subset(fine, 4))["max_m"], 1.0)
        with self.assertRaises(ValueError):
            study.interpolation_error(fine, [[0, 0], [4, 0]])

    def test_turn_descriptor_ignores_small_and_narrow_wiggles(self):
        tiny = [[x, 2 * math.sin(x * 0.2)] for x in range(1201)]
        self.assertEqual(study.significant_turns(tiny, 8, 40), [])
        narrow = [[x, 100 if x == 100 else 0] for x in range(1201)]
        self.assertEqual(study.significant_turns(narrow, 8, 40), [])
        hill = [[x, 100 - abs(x - 100)] for x in range(201)]
        self.assertEqual(study.significant_turns(hill, 8, 40), [{"kind": "peak", "x_m": 100, "y_m": 100}])
        valley = [[x, abs(x - 100)] for x in range(201)]
        self.assertEqual(study.significant_turns(valley, 8, 40), [{"kind": "valley", "x_m": 100, "y_m": 0}])

    def test_percentile_and_metrics(self):
        self.assertEqual(study.percentile([0, 2, 4], 0.5), 2)
        points = [[x, 7 + 0.25 * x] for x in range(-160, 1361)]
        stat = study.describe({"variant": "refined", "seed": 0, "points_m": points}, self.plan)
        self.assertEqual(stat["relief_m"], 300)
        self.assertEqual(stat["endpoint_rise_m"], 300)
        self.assertEqual(stat["max_abs_slope"], 0.25)
        self.assertEqual(stat["start_height_m"], 7)

    def test_create_only_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "profile.json"
            study.write_new(path, b"first")
            with self.assertRaises(FileExistsError):
                study.write_new(path, b"second")
            self.assertEqual(path.read_bytes(), b"first")

    def test_reference_rejects_changed_source_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            study.write_new(root / "game/core/noise.py", b"raise RuntimeError('must not execute')")
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                study.load_reference(root, self.plan)

    def test_display_translation_preserves_raw_data_and_uses_shared_scale(self):
        p = {"variant": "refined", "seed": 0, "points_m": [[x, x / 10 + 200] for x in range(-160, 1361)]}
        original = copy.deepcopy(p)
        svg = study.chart([p], self.plan, (-50, 200))
        self.assertIn("shared scale", svg)
        self.assertIn("horizontal distance (m)", svg)
        self.assertEqual(p, original)
        self.assertEqual(study.shared_extent([p], self.plan), (-50, 200))

    def test_weighting_zero_matches_reference_structure_algebra(self):
        class Noise:
            def noise2(self, x, y):
                return math.sin(x + y) * 0.5
        class Generator:
            structure_octaves = 3
            structure_frequency = 0.002
            structure_persistence = 0.35
            structure_lacunarity = 2
            ridge_mix = 0.45
            structure_amplitude = 140
            _structure_noise = Noise()
            _ridge_noise = Noise()
            def _warped_x(self, x):
                return x
        g = Generator()
        x, amp, freq = 17, 1.0, g.structure_frequency
        normal = ridged = norm = 0.0
        for _ in range(g.structure_octaves):
            normal += g._structure_noise.noise2(x * freq, 23) * amp
            r = (1 - abs(g._ridge_noise.noise2(x * freq, 67))) ** 2
            ridged += (2 * r - 1) * amp
            norm += amp
            amp *= g.structure_persistence
            freq *= g.structure_lacunarity
        expected = (normal * (1 - g.ridge_mix) + ridged * g.ridge_mix) / norm * g.structure_amplitude
        self.assertAlmostEqual(study.weighted_structure(g, x, 0), expected)

    def synthetic_capture(self, output):
        plan_data = (study.HERE / "plan.json").read_bytes()
        study.write_new(output / "plan.json", plan_data)
        profiles = []
        for variant in study.VARIANTS:
            for seed in sorted(self.plan["seeds"]):
                points = [[x, x / 10 + seed] for x in range(-160, 1361)]
                profiles.append({"variant": variant, "seed": seed, "points_m": points})
                study.write_new(output / "profiles" / f"{variant}-seed-{seed}.json", study.encoded({
                    "profile_only": True, "variant": variant, "seed": seed,
                    "points_m": study.subset(points, 4), "reference_points_m": points,
                }))
        summary = {"schema": self.plan["schema"], "profile_only": True, "flight_runs": 0,
                   "plan_sha256": study.digest(plan_data), "profile_count": 18,
                   "repeat_sha256": study.digest(study.encoded(profiles)), "repeat_passed": True,
                   "profiles": [study.describe(p, self.plan) for p in profiles]}
        study.write_new(output / "summary.json", study.encoded(summary))
        study.write_new(output / "index.html", study.gallery(profiles, summary, self.plan))
        study.write_new(output / "overview.svg", study.overview_svg(profiles, self.plan))
        study.write_new(output / "refined.svg", study.overview_svg(profiles, self.plan, True))
        for name in study.SOURCES:
            study.write_new(output / "inputs" / "study" / name, (study.HERE / name).read_bytes())
        for name in self.plan["pylander"]["files"]:
            study.write_new(output / "inputs" / "pylander" / name, b"synthetic fixture; loader mocked")
        receipt = {"schema": self.plan["schema"], "profile_only": True, "flight_runs": 0,
                   "source_sha256": {name: study.digest((study.HERE / name).read_bytes()) for name in study.SOURCES},
                   "files": {str(p.relative_to(output)): study.digest(p.read_bytes()) for p in output.rglob("*") if p.is_file()}}
        study.write_new(output / "receipt.json", study.encoded(receipt))

    def test_saved_verifier_recomputes_metrics_and_rejects_tampering(self):
        # Synthetic arrays test the artifact verifier, not Pylander acceptance.
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            output = Path(directory)
            self.synthetic_capture(output)
            study.verify(output, study.HERE / "plan.json")
            summary_path = output / "summary.json"
            summary = json.loads(summary_path.read_bytes())
            summary["profiles"][0]["relief_m"] += 1
            summary_path.write_bytes(study.encoded(summary))
            with self.assertRaisesRegex(ValueError, "artifact mismatch"):
                study.verify(output, study.HERE / "plan.json")
            # Rehashing a forged derived result still fails raw-data derivation.
            receipt_path = output / "receipt.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["files"]["summary.json"] = study.digest(summary_path.read_bytes())
            receipt_path.write_bytes(study.encoded(receipt))
            with self.assertRaisesRegex(ValueError, "derived summary"):
                study.verify(output, study.HERE / "plan.json")

    def test_saved_verifier_checks_independent_plan_and_inventory(self):
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            output = Path(directory)
            self.synthetic_capture(output)
            other_plan = output.parent / (output.name + "-other-plan.json")
            try:
                study.write_new(other_plan, b"{}")
                with self.assertRaisesRegex(ValueError, "independently supplied plan"):
                    study.verify(output, other_plan)
            finally:
                other_plan.unlink(missing_ok=True)
            study.write_new(output / "extra.json", b"{}")
            with self.assertRaisesRegex(ValueError, "inventory"):
                study.verify(output, study.HERE / "plan.json")

    def test_saved_verifier_cannot_omit_generation_sources(self):
        with tempfile.TemporaryDirectory() as directory, patch("study.load_reference"):
            output = Path(directory)
            self.synthetic_capture(output)
            path = output / "receipt.json"
            receipt = json.loads(path.read_bytes())
            receipt["source_sha256"] = {}
            path.write_bytes(study.encoded(receipt))
            with self.assertRaisesRegex(ValueError, "source inventory"):
                study.verify(output, study.HERE / "plan.json")


if __name__ == "__main__":
    unittest.main()
