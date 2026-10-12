import json
from pathlib import Path
import tempfile
import unittest

from prepare import declare, inventory, reconcile, stress_selection, write_new


def manifest(role="development", identities=("a",), seed=1):
    entries = [{"index": i, "physical_sha256": identity}
               for i, identity in enumerate(identities)]
    return {"schema_version": 2, "request": {"role": role, "seed": seed,
            "recipe": {}, "population": {"kind": "stress", "panels": [
                {"id": "one", "quota": 1}, {"id": "two", "quota": 1}]}},
            "entries": entries, "counts": {"accepted": len(entries)},
            "receipts": [{"proposal": e, "challenge": {"qualifies": True},
                          "matching_stress_panels": ["one", "two"]} for e in entries]}


class PreparationTests(unittest.TestCase):
    def test_inventory_retains_excluded_identities_logs_and_usage(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            m = manifest(identities=("admitted", "excluded"))
            m["entries"] = m["entries"][:1]
            write_new(root/"collection"/"manifest.json", m)
            write_new(root/"collection"/"usage-inspection.json", {"receipt_id": "inspection"})
            (root/"worker-00.jsonl").write_text(json.dumps({
                "kind": "proposal", "seed": 5, "physical_sha256": "training"})+"\n")
            result = inventory(root)
            self.assertEqual(result["physical_sha256"], ["admitted", "excluded", "training"])
            self.assertEqual(result["reserved_seeds"], [1, 5])
            self.assertEqual(len(result["collections"][0]["usage_files"]), 1)

    def test_stress_support_counts_unique_qualified_memberships_before_quota(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            a, b = manifest(identities=("a", "b")), manifest(identities=("c",))
            a["receipts"].append(a["receipts"][0])
            nonqualifying = dict(a["receipts"][0], proposal={"physical_sha256": "bad"},
                                 challenge={"qualifies": False})
            a["receipts"].append(nonqualifying)
            write_new(root/"a"/"manifest.json", a); write_new(root/"b"/"manifest.json", b)
            result = stress_selection([root/"a", root/"b"])
            self.assertEqual(result["candidates"][0]["matching_counts"], {"one": 2, "two": 2})
            self.assertEqual(result["selected"], str(root/"a"))
            self.assertEqual(stress_selection([root/"b", root/"b"])["selected"], str(root/"b"))

    def test_freeze_detects_prior_and_cross_role_collision_without_editing(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_new(root/"val"/"manifest.json", manifest("validation", ("prior", "shared")))
            write_new(root/"final"/"manifest.json", manifest("held_out", ("shared",)))
            original = (root/"final"/"manifest.json").read_bytes()
            result = reconcile({"physical_sha256": ["prior"]}, [root/"val", root/"final"])
            self.assertFalse(result["passed"])
            self.assertEqual(len(result["collisions"]), 2)
            self.assertEqual((root/"final"/"manifest.json").read_bytes(), original)

    def test_declaration_rejects_exposed_namespace_and_create_only_outputs(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_new(root/"prior.json", {"errors": [], "reserved_seeds": [20261012045]})
            write_new(root/"template.json", {"recipe": {"backend": "layered_perlin", "terrain": {}}})
            with self.assertRaisesRegex(ValueError, "collides"):
                declare(root/"prior.json", root/"template.json", root/"declaration")
            path = root/"immutable.json"; write_new(path, {"original": True})
            with self.assertRaises(FileExistsError):
                write_new(path, {"rewritten": True})
            self.assertEqual(json.loads(path.read_text()), {"original": True})


if __name__ == "__main__":
    unittest.main()
