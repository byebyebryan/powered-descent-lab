"""Hand-calculated geometry checks, independent of any retained flight evidence."""

import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import audit


def scenario(points, source=(0, 0), target=(1000, 0), width=20):
    return {"world": {"terrain": {"kind": "heightfield", "points_m": [{"x": x, "y": y} for x, y in points]},
                      "landing_pads": [{"id": "source", "center_x_m": source[0], "surface_y_m": source[1], "width_m": width},
                                       {"id": "target", "center_x_m": target[0], "surface_y_m": target[1], "width_m": width}]},
            "mission": {"goal": {"target_pad_id": "target"}}, "initial_state": {"position_m": {"x": source[0]}}}


class GeometryAudit(unittest.TestCase):
    def test_triangular_barrier_has_exact_interpolated_width(self):
        value = audit.measure(scenario([(0, 0), (500, 200), (1000, 0)]))
        self.assertEqual(value["relief_m"], 200)
        self.assertEqual(value["barrier_m"], 200)
        self.assertEqual(value["longest_barrier_100m_width_m"], 500)
        self.assertEqual(value["longest_barrier_50m_width_m"], 750)
        self.assertEqual(value["p95_slope"], .4)

    def test_uphill_relief_does_not_imply_an_intervening_barrier(self):
        value = audit.measure(scenario([(-100, -40), (500, 200), (1100, 440)], target=(1000, 400)))
        self.assertEqual(value["relief_m"], 400)
        self.assertEqual(value["signed_pad_rise_m"], 400)
        self.assertEqual(value["barrier_m"], 0)
        self.assertEqual(value["longest_barrier_100m_width_m"], 0)

    def test_disconnected_barriers_do_not_merge_across_a_valley(self):
        points = [(0, 0), (200, 200), (400, 0), (600, 200), (800, 0), (1000, 0)]
        value = audit.measure(scenario(points))
        self.assertEqual(value["longest_barrier_100m_width_m"], 200)

    def test_plateau_threshold_and_flat_terrain(self):
        self.assertEqual(audit.longest_above([(0, 0), (100, 100), (300, 100), (400, 0)], 100), 200)
        value = audit.measure(scenario([(0, 0), (1000, 0)]))
        self.assertEqual(value["relief_m"], 0)
        self.assertEqual(value["p95_slope"], 0)
        self.assertFalse(value["max_slope_at_pad_edge"])

    def test_translation_reflection_and_collinear_subdivision_preserve_geometry(self):
        original = scenario([(0, 0), (500, 200), (1000, 0)])
        expected = audit.measure(original)
        translated = copy.deepcopy(original)
        for point in translated["world"]["terrain"]["points_m"]:
            point["x"] += 71
            point["y"] -= 250
        for pad in translated["world"]["landing_pads"]:
            pad["center_x_m"] += 71
            pad["surface_y_m"] -= 250
        translated["initial_state"]["position_m"]["x"] += 71
        self.assertEqual(audit.measure(translated), expected)
        reflected = scenario([(-1000, 0), (-500, 200), (0, 0)], source=(0, 0), target=(-1000, 0))
        self.assertEqual(audit.measure(reflected), expected)
        subdivided = scenario([(0, 0), (250, 100), (500, 200), (750, 100), (1000, 0)])
        self.assertEqual(audit.measure(subdivided), expected)

    def test_slope_quantile_is_weighted_by_distance_not_vertex_count(self):
        weighted = [(0, 960)] + [(10, 1)] * 40
        self.assertEqual(audit.weighted_quantile(weighted, .95), 0)

    def test_summary_includes_integer_physical_inputs(self):
        row = {"index": "mission", "group": "test", "max_slope_at_pad_edge": False,
               **audit.measure(scenario([(0, 0), (1000, 0)]))}
        self.assertEqual(audit.summarize([row])["distance_m"]["p50"], 1000)
        self.assertEqual(audit.summarize([row])["target_width_m"]["p50"], 20)

    def test_pad_edge_spike_is_reported_separately(self):
        value = audit.measure(scenario([(0, 0), (10, 0), (11, 10), (100, 10), (990, 0), (1000, 0)]))
        self.assertTrue(value["max_slope_at_pad_edge"])
        self.assertEqual(value["max_slope"], 10)
        self.assertLess(value["max_slope_without_pad_edges"], .1)

    def test_tied_slope_location_is_independent_of_travel_direction(self):
        points = [(0, 0), (10, 0), (20, 10), (30, 20), (50, 40), (990, 0), (1000, 0)]
        forward = audit.measure(scenario(points))
        mirrored = scenario(sorted((-x, y) for x, y in points), source=(0, 0), target=(-1000, 0))
        self.assertTrue(forward["max_slope_at_pad_edge"])
        self.assertEqual(audit.measure(mirrored), forward)

    def test_invalid_and_out_of_domain_terrain_fail(self):
        for points in [[(0, 0), (0, 1), (1000, 0)], [(0, 0), (1000, float("nan"))], [(100, 0), (1000, 0)]]:
            with self.assertRaises(ValueError):
                audit.measure(scenario(points))

    def test_bound_reader_rejects_changed_bytes_and_path_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = json.dumps({"ok": True}).encode()
            (root / "input.json").write_bytes(raw)
            expected = hashlib.sha256(raw).hexdigest()
            self.assertEqual(audit.read_bound_file(root, "input.json", expected)[0], {"ok": True})
            (root / "input.json").write_bytes(raw + b" ")
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                audit.read_bound_file(root, "input.json", expected)
            with self.assertRaises(ValueError):
                audit.read_bound_file(root, "../outside.json")


if __name__ == "__main__":
    unittest.main()
