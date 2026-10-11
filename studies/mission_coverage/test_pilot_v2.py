import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import run_pilot_v2 as pilot


class PilotDeclarationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "declaration"
        shutil.copytree(pilot.DECLARATION.parent, self.root)
        self.path = self.root / "pilot.json"
        self.cfg = json.loads(self.path.read_text())

    def tearDown(self):
        self.temp.cleanup()

    def altered(self, cfg):
        self.path.write_text(json.dumps(cfg))
        return pilot.load_declaration(self.path)

    def test_frozen_declaration_retains_counts_specs_and_zero_campaign_budget(self):
        cfg, files, requests, spec = pilot.load_declaration(self.path)
        self.assertEqual(sum(len(r["indices"]) for r in requests), 20000)
        self.assertEqual([r["seed"] for r in requests], [20261011031, 20261011032])
        self.assertEqual(len(files), 6)
        self.assertEqual(len(spec["coverage"]["joints"]), 5)
        self.assertEqual(spec["reference_qa"]["regular_index_offsets"], [0, 1])
        self.assertFalse(cfg["held_out_generation"])

    def test_holdout_training_and_collection_role_changes_fail_before_a_run(self):
        for field, value in [("held_out_generation", True), ("flight_training_budget", 1), ("benchmark_freeze", True)]:
            cfg = copy.deepcopy(self.cfg)
            cfg[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.altered(cfg)
        cfg = copy.deepcopy(self.cfg)
        cfg["collections"][0]["role"] = "held_out"
        with self.assertRaises(ValueError):
            self.altered(cfg)

    def test_changed_recipe_bytes_and_traversal_cannot_be_silently_substituted(self):
        recipe = self.root / "layered-recipe.json"
        recipe.write_text(recipe.read_text() + " ")
        with self.assertRaisesRegex(ValueError, "identity changed"):
            pilot.load_declaration(self.path)
        cfg = copy.deepcopy(self.cfg)
        cfg["file_sha256"] = {"../outside.json": "a" * 64}
        with self.assertRaisesRegex(ValueError, "basename"):
            self.altered(cfg)

    def test_duplicate_ids_boolean_seeds_and_oversized_schedules_fail(self):
        for field, value in [("seed", True), ("index_count", 20001), ("index_start", 2**64 - 1)]:
            cfg = copy.deepcopy(self.cfg)
            cfg["collections"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.altered(cfg)
        cfg = copy.deepcopy(self.cfg)
        cfg["collections"][1]["id"] = cfg["collections"][0]["id"]
        with self.assertRaises(ValueError):
            self.altered(cfg)

    def test_review_and_qa_cannot_hide_exposure_or_change_bounded_browsing(self):
        cfg = copy.deepcopy(self.cfg)
        cfg["reference_qa"]["flags_are_not_benchmark_acceptance"] = False
        with self.assertRaises(ValueError):
            self.altered(cfg)
        cfg = copy.deepcopy(self.cfg)
        cfg["preview_review"]["open_bands_at_once"] = 30
        with self.assertRaises(ValueError):
            self.altered(cfg)

    def test_existing_and_dangling_output_paths_fail_before_build_or_generation(self):
        existing = Path(self.temp.name) / "existing"
        existing.mkdir()
        dangling = Path(self.temp.name) / "dangling"
        dangling.symlink_to(Path(self.temp.name) / "absent-target", target_is_directory=True)
        for output in [existing, dangling]:
            with self.subTest(output=output), patch.object(pilot, "committed_declaration"), patch.object(pilot, "source_binding", return_value={}), patch.object(pilot, "run_stage") as stage:
                with self.assertRaises(FileExistsError):
                    pilot.run_pilot(self.path, output)
                stage.assert_not_called()
        self.assertFalse((Path(self.temp.name) / "absent-target").exists())


if __name__ == "__main__":
    unittest.main()
