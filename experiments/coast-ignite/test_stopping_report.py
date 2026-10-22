import json
import tempfile
import time
import unittest
from pathlib import Path

from coast_ignite import capture_verified, encoded, native, write_new
from run_loop import Budget
from render_stopping import split_reports, label_for, scroll_summary_tables


class ReportTests(unittest.TestCase):
    def test_scroll_wrappers_do_not_rewrite_maintained_review_tables(self):
        raw = '<table><tr><td>Score</td></tr></table><table class="paired-table"><tr></tr></table>'
        result = scroll_summary_tables(raw)
        self.assertEqual(result.count('<div class="table-wrap stopping-table-wrap">'),1)
        self.assertIn('<table class="paired-table"><tr></tr></table>',result)

    def test_populations_keep_dense_approaches_and_primary_view_first(self):
        rows = []
        for arc in range(3):
            for seed,rule,role in [(281101,"M","candidate"),(281102,"M","candidate"),
                                   (281103,"M","candidate"),(281103,"G","baseline"),(0,"delay","heuristic")]:
                rows.append({"arc":arc,"seed":seed,"rule":rule,"role":role,"replicate":0,"population":"Fresh confirmation"})
        rows.append({"arc":0,"seed":281103,"rule":"G","replicate":0,"population":"Preservation"})
        reports = split_reports(rows)
        self.assertEqual([name for name,_,_ in reports],["report","report-preservation"])
        labels = [label_for(r,"Fresh confirmation") for r in reports[0][2][:5]]
        self.assertEqual(labels[:2],["Median · primary","Greedy · primary"])
        for i in range(0,15,5):
            self.assertEqual([label_for(r,"Fresh confirmation") for r in reports[0][2][i:i+5]],labels)

    def test_grouping_and_camera_preserve_complete_rich_payload(self):
        project = Path(__file__).resolve().parents[2]
        anchor = json.loads((project/"fixtures/scenarios/flat_terminal_descent.json").read_text())
        proposal = next(p for k in range(256) if (p := json.loads(native.generate_proposal(
            encoded(anchor), 81131, "stopping-report-software", k)))["admitted"])
        task = {k:proposal[k] for k in ["scenario","reference_end","physical_sha256"]}
        with tempfile.TemporaryDirectory(dir=project/"target") as temp:
            root = Path(temp)
            path = root/"task.json";write_new(path,task)
            budget = Budget(root/"budget", 60, 1_000_000, 128*2**20, time.time()+60)
            capture_verified(task,path,{"kind":"immediate"},root/"case",budget)
            rows = [{"capture":str(root/"case"),"population":"Software fixture",
                     "label":label} for label in ["Saved capture A","Saved capture B"]]
            spec = {"title":"Report software check","summary":"Same saved capture, two presentation slots.","rows":rows}
            native.render_report(encoded(spec),str(root/"raw"))
            native.render_report(encoded({**spec,"grouped":True,"focus_flat":True}),str(root/"focused"))
            raw = json.loads((root/"raw/report-input.json").read_text())
            focused = json.loads((root/"focused/report-input.json").read_text())
            self.assertEqual(len(raw["missions"]),2)
            self.assertEqual(len(focused["missions"]),1)
            self.assertEqual(len(focused["missions"][0]["attempts"]),2)
            self.assertIn('2 complete rich flight views',(root/"focused/report.html").read_text())
            def payload(folder):
                text = (root/folder/"flight-0.html").read_text();marker="const reportData = "
                return json.JSONDecoder().raw_decode(text[text.index(marker)+len(marker):])[0]
            self.assertEqual(payload("raw"),payload("focused"))
            self.assertIn('xaxis: axisStyle({ title: "", range:',(root/"focused/flight-0.html").read_text())


if __name__ == "__main__":
    unittest.main()
