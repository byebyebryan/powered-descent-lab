import functools
import http.server
import importlib.util
import pathlib
import subprocess
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("report_server", pathlib.Path(__file__).with_name("report_server.py"))
server = importlib.util.module_from_spec(spec)
spec.loader.exec_module(server)


class ReportServerTests(unittest.TestCase):
    def test_entrypoint_fingerprint_excludes_navigation_missions_and_symlinks(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = pathlib.Path(temporary)
            outputs = repo / "outputs"
            page = outputs / "eval/pack/capture/index.html"
            page.parent.mkdir(parents=True)
            page.write_text("batch")
            first = server.fingerprint(repo, outputs)
            for name in ("index.html", "reports/library/index.html", "eval/pack/capture/runs/case/report.html",
                         "eval/pack/capture/details/report.html", "eval/pack/capture/previews/index.html",
                         "eval/pack/a b/report.html", "eval/pack/日本語/report.html"):
                path = outputs / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("not a source")
            (outputs / "eval/pack/escape").symlink_to(repo, target_is_directory=True)
            self.assertEqual(first, server.fingerprint(repo, outputs))
            page.write_text("new batch")
            self.assertNotEqual(first, server.fingerprint(repo, outputs))

    def test_fingerprint_does_not_open_detail_or_preview_directories(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = pathlib.Path(temporary)
            outputs = repo / "outputs"
            capture = outputs / "reports/eval/procedural/edition"
            capture.mkdir(parents=True)
            (capture / "report.html").write_text("batch")
            for name in ("details", "previews"):
                (capture / name).mkdir()
            scandir = server.os.scandir
            def scan(path):
                self.assertNotIn(pathlib.Path(path).name, ("details", "previews"))
                return scandir(path)
            with patch.object(server.os, "scandir", side_effect=scan):
                signature = server.fingerprint(repo, outputs)
            self.assertEqual(len(signature), 1)

    def test_navigation_refresh_is_cached_retries_failure_and_never_runs_flights(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = pathlib.Path(temporary)
            refresh = server.NavigationRefresh(repo, repo / "outputs", repo / "pd-eval", interval=0)
            with patch.object(server.subprocess, "run") as run:
                run.return_value = subprocess.CompletedProcess([], 0, "", "")
                self.assertTrue(refresh.refresh())
                self.assertTrue(refresh.refresh())
                self.assertEqual(run.call_count, 1)
                self.assertEqual(run.call_args.args[0], [str(repo / "pd-eval"), "refresh-reports", "--home-only"])
                page = repo / "outputs/eval/pack/new/index.html"
                page.parent.mkdir(parents=True)
                page.write_text("new")
                run.side_effect = subprocess.CalledProcessError(1, refresh.command)
                self.assertFalse(refresh.refresh())
                run.side_effect = None
                self.assertTrue(refresh.refresh())
                self.assertEqual(run.call_count, 3)

    def test_http_navigation_detects_a_new_capture_and_static_details_do_not_refresh(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = pathlib.Path(temporary)
            outputs = repo / "outputs"
            outputs.mkdir()
            (outputs / "index.html").write_text("old")
            refresh = server.NavigationRefresh(repo, outputs, repo / "pd-eval", interval=0)
            def render(*args, **kwargs):
                pages = server.fingerprint(repo, outputs)
                (outputs / "index.html").write_text(str(pages))
                return subprocess.CompletedProcess([], 0, "", "")
            handler = functools.partial(server.ReportHandler, directory=str(outputs), refresh=refresh)
            with patch.object(server.subprocess, "run", side_effect=render) as run:
                with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as test_http:
                    thread = threading.Thread(target=test_http.serve_forever)
                    thread.start()
                    try:
                        base = f"http://127.0.0.1:{test_http.server_port}"
                        with urllib.request.urlopen(base + "/") as response:
                            self.assertEqual(response.headers["Cache-Control"], "no-store")
                            self.assertNotIn("new-capture", response.read().decode())
                        page = outputs / "eval/pack/new-capture/index.html"
                        page.parent.mkdir(parents=True)
                        page.write_text("saved batch")
                        request = urllib.request.Request(base + "/", headers={"If-Modified-Since": "Wed, 01 Jan 2099 00:00:00 GMT"})
                        with urllib.request.urlopen(request) as response:
                            self.assertEqual(response.status, 200)
                            self.assertIn("new-capture", response.read().decode())
                        self.assertEqual(run.call_count, 2)
                        with urllib.request.urlopen(base + "/eval/pack/new-capture/index.html") as response:
                            self.assertEqual(response.read(), b"saved batch")
                        self.assertEqual(run.call_count, 2)
                        run.side_effect = subprocess.CalledProcessError(1, refresh.command)
                        page.write_text("changed batch")
                        with self.assertRaises(urllib.error.HTTPError) as error:
                            urllib.request.urlopen(base + "/")
                        self.assertEqual(error.exception.code, 503)
                        error.exception.close()
                    finally:
                        test_http.shutdown()
                        thread.join()


if __name__ == "__main__":
    unittest.main()
