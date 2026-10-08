"""Tracked pure challenge-contract checks; no campaigns or external checkout."""

import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import challenge
import survey


class FakeGenerator:
    def __init__(self, seed, **parameters):
        self.seed = seed
        self.__dict__.update(parameters)
        self._ridge_noise = object()

    def __call__(self, x):
        return self.macro_amplitude * (self.seed % 7 + x * self.macro_frequency)


class ChallengeTests(unittest.TestCase):
    def setUp(self):
        self.pilot, self.refinement, self.base = survey.load_plan(challenge.HERE / "challenge_calibration_plan.json")
        self.main, _, _ = survey.load_plan(challenge.HERE / "challenge_plan.json")
        self.validation, _, _ = survey.load_plan(challenge.HERE / "challenge_validation_1k_plan.json")

    def test_populations_are_distinct_reproducible_and_all_accounted(self):
        sanity, _, _ = survey.load_plan()
        sets = [set(survey.seeds_for(p)) for p in (sanity, self.pilot, self.main)]
        self.assertEqual([len(s) for s in sets], [100, 24, 100])
        self.assertFalse(sets[0] & sets[1] or sets[0] & sets[2] or sets[1] & sets[2])
        self.assertEqual(challenge.seeds(self.main), challenge.seeds(self.main))
        for plan in (sanity, self.pilot, self.main):
            waves = survey.waves_for(plan)
            self.assertEqual([i for wave in waves for i in wave], list(range(plan["maximum_measured_attempts"])))
            self.assertTrue(all(1 <= len(wave) <= 4 for wave in waves))
        self.assertEqual(self.main["repeat_indices"], [0, 25, 50, 75, 1])

    def test_global_recipe_changes_only_physical_amplitudes_and_dimensions(self):
        base = self.base["refined"]
        for descriptor in self.main["recipes"]:
            recipe = challenge.recipe(self.base, descriptor)
            self.assertEqual(recipe["structure_octaves"], base["structure_octaves"])
            self.assertEqual(recipe["structure_amplitude"], base["structure_amplitude"] * descriptor["vertical_scale"])
            self.assertEqual(recipe["structure_frequency"], base["structure_frequency"] / descriptor["horizontal_scale"])
            self.assertEqual(recipe["feature_cell_size"], base["feature_cell_size"] * descriptor["horizontal_scale"])
            for key in ("base_height", "structure_lacunarity", "structure_persistence", "ridge_mix", "feature_density"):
                self.assertEqual(recipe[key], base[key])

    def test_sampling_is_order_independent_and_never_filters_seeds(self):
        with patch("challenge.study.load_reference", return_value=FakeGenerator):
            first = challenge.sample(self.pilot, self.base, self.refinement, Path("synthetic"))
            reverse = challenge.sample(self.pilot, self.base, self.refinement, Path("synthetic"), True)
        self.assertEqual(first, reverse)
        self.assertEqual(len(first), 24)
        self.assertEqual({p["seed"] for p in first}, set(challenge.seeds(self.pilot)))
        self.assertEqual({v:sum(p["variant"] == v for p in first) for v in {p["variant"] for p in first}},
                         {r["id"]:6 for r in self.pilot["recipes"]})

    def test_preparation_keeps_vehicle_clocks_mission_and_only_local_pad_edits(self):
        profile = {"variant":self.main["recipes"][0]["id"], "seed":987,
                   "points_m":[[x, x * 0.15] for x in range(-160, 1361, 4)]}
        scenario, geometry = challenge.prepared(profile, self.main, self.refinement, self.base)
        template = json.loads((challenge.HERE / "scenario_template.json").read_bytes())
        for key in ("vehicle", "sim", "mission"):
            self.assertEqual(scenario[key], template[key])
        self.assertTrue(geometry["outside_patches_unchanged"])
        self.assertEqual(geometry["recipe_id"], profile["variant"])
        self.assertEqual(geometry["maximum_height_above_endpoint_chord_m"], 0)
        self.assertTrue(all(type(v) is str for v in scenario["metadata"].values()))

    def test_changed_population_or_unbounded_recipes_are_rejected(self):
        for key, value in (("seed_count",101), ("workers",5), ("master_seed",123), ("repeat_indices",[0]*5)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                challenge.validate(dict(self.main, **{key:value}), self.refinement, self.base)
        for key,value in (("vertical_scale",32), ("horizontal_scale",0.1)):
            changed = copy.deepcopy(self.main)
            changed["recipes"][0][key] = value
            with self.assertRaises(ValueError):
                challenge.validate(changed, self.refinement, self.base)

    def test_validation_1k_is_fresh_balanced_bounded_and_recipe_exact(self):
        sanity, _, _ = survey.load_plan()
        old = set(self.base["seeds"])
        for plan in (sanity, self.pilot, self.main):
            old.update(survey.seeds_for(plan))
        fresh = survey.seeds_for(self.validation)
        self.assertEqual(len(fresh), 1000)
        self.assertEqual(len(set(fresh)), 1000)
        self.assertFalse(set(fresh) & old)
        self.assertEqual(fresh, survey.seeds_for(self.validation))
        self.assertEqual(self.validation["recipes"], self.main["recipes"])
        self.assertEqual(self.validation["repeat_indices"], [0, 250, 500, 750, 1])
        waves = survey.waves_for(self.validation)
        self.assertEqual([i for wave in waves for i in wave], list(range(1008)))
        self.assertTrue(all(1 <= len(wave) <= 4 for wave in waves))
        with patch("challenge.study.load_reference", return_value=FakeGenerator):
            profiles = challenge.sample(self.validation, self.base, self.refinement, Path("synthetic"))
            reverse = challenge.sample(self.validation, self.base, self.refinement, Path("synthetic"), True)
        self.assertEqual(profiles, reverse)
        self.assertEqual({p["seed"] for p in profiles}, set(fresh))
        self.assertEqual({r["id"]:sum(p["variant"] == r["id"] for p in profiles)
                          for r in self.main["recipes"]}, {r["id"]:250 for r in self.main["recipes"]})
        for key, value in (("seed_count",1001), ("master_seed",2026100704), ("maximum_measured_attempts",1009)):
            with self.subTest(key=key), self.assertRaises(ValueError):
                challenge.validate(dict(self.validation, **{key:value}), self.refinement, self.base)
        changed = copy.deepcopy(self.validation)
        changed["recipes"][0]["vertical_scale"] = 5
        with self.assertRaises(ValueError):
            challenge.validate(changed, self.refinement, self.base)

    def test_challenge_source_paths_are_explicit_and_unknown_phases_rejected(self):
        for plan in (self.pilot, self.main, self.validation):
            self.assertEqual(survey.load_plan(challenge.HERE / challenge.plan_filename(plan["phase"]))[0], plan)
        with self.assertRaises(ValueError):
            challenge.plan_filename("unknown")


if __name__ == "__main__":
    unittest.main()
