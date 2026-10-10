import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("continuation", Path(__file__).with_name("continue-terminal-centering-sweep.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class DomainExitTests(unittest.TestCase):
    def test_only_observed_domain_exit_is_contained(self):
        row = {"attempt_id": "test", "status": "runner_error", "exit_code": 1, "result": None}
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for i, (text, expected) in enumerate((
                ("Error: exact body-envelope terrain query failed: terrain query x=1360.0987 lies outside domain [-160, 1360]\n", True),
                ("Error: exact body-envelope terrain query failed: terrain query x=-160.1 lies outside domain [-160, 1360]\n", True),
                ("Error: exact body-envelope terrain query failed: terrain query x=1200 lies outside domain [-160, 1360]\n", False),
                ("Error: replay failed\n", False), ("Error: nonfinite short prediction\n", False))):
                item = {**row, "attempt_id": str(i)}
                runner.study.write_new(root / "logs" / f"{i}.stderr", text.encode())
                self.assertEqual(runner.domain_error(root, item), expected)
                if expected:
                    for key, value in (("exit_code", 0), ("result", {}), ("status", "recorded")):
                        self.assertFalse(runner.domain_error(root, {**item, key: value}))

    def test_prefix_is_not_retried_or_overwritten(self):
        self.assertNotEqual(runner.PARTIAL, runner.ROOT)
        self.assertIn("receipt.json", runner.SKIP)
        self.assertIn("ballistic-feedback-sweep.json", runner.SKIP)
        self.assertNotIn("plan.json", runner.SKIP)
        self.assertNotIn("candidate/source-seal.json", runner.SKIP)


if __name__ == "__main__":
    unittest.main()
