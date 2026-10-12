"""Serve saved outputs; refresh navigation only when its report sources change."""
import argparse
import functools
import http.server
import os
import pathlib
import subprocess
import sys
import threading
import time
import urllib.parse

SCOPES = ("reports/eval", "reports/setups", "reports/runs", "reports/replays", "eval", "research", "procedural-challenge")
SKIP = {"latest", "cache", "runs", "cases", "details", "previews", "candidate", "source", "inputs", "baseline", "bin"}


def fingerprint(repo, outputs):
    """Bounded entrypoint stats, not evidence reads or recursive mission scans."""
    rows = []

    def record(path, stat=None):
        if stat is None:
            stat = path.stat(follow_symlinks=False)
        rows.append((str(path), stat.st_mtime_ns, stat.st_size))

    def scan(directory, depth):
        if directory.is_symlink() or not directory.is_dir():
            return
        # DirEntry caches file types, avoiding repeated stats for non-report assets.
        with os.scandir(directory) as entries:
            for entry in sorted(entries, key=lambda item: item.name):
                name = entry.name
                if (entry.is_symlink() or name.startswith(".")
                        or not all(c.isascii() and (c.isalnum() or c in "_-.") for c in name)):
                    continue
                if entry.is_dir(follow_symlinks=False) and depth and name not in SKIP:
                    scan(pathlib.Path(entry.path), depth - 1)
                elif name in ("index.html", "report.html") and entry.is_file(follow_symlinks=False):
                    if directory in [outputs / scope for scope in SCOPES]:
                        continue
                    record(pathlib.Path(entry.path), entry.stat(follow_symlinks=False))

    for scope in SCOPES:
        directory = outputs / scope
        if any(parent.is_symlink() for parent in (directory, *directory.parents) if parent != outputs.parent):
            continue
        scan(directory, 3)
    for directory in (repo / "fixtures/reports", repo / "fixtures/packs"):
        if directory.is_dir() and not directory.is_symlink():
            for path in sorted(directory.glob("*.json")):
                if not path.is_symlink():
                    record(path)
    return tuple(rows)


class NavigationRefresh:
    def __init__(self, repo, outputs, evaluator, *, interval=1.0):
        self.repo, self.outputs = repo, outputs
        self.command = [str(evaluator), "refresh-reports", "--home-only"]
        self.interval = interval
        self.lock = threading.Lock()
        self.signature = None
        self.checked_at = float("-inf")
        self.error = None

    def refresh(self):
        with self.lock:
            now = time.monotonic()
            if now - self.checked_at < self.interval:
                return self.error is None
            self.checked_at = now
            try:
                signature = fingerprint(self.repo, self.outputs)
                if signature != self.signature or self.error:
                    result = subprocess.run(self.command, cwd=self.repo, capture_output=True,
                                            text=True, timeout=15, check=True)
                    if result.stderr:
                        print(result.stderr, file=sys.stderr, end="")
                    self.signature = signature
                self.error = None
            except (OSError, subprocess.SubprocessError) as error:
                self.error = str(error)
                print(f"Report navigation refresh failed: {error}", file=sys.stderr)
            return self.error is None


def navigation_request(path):
    path = urllib.parse.unquote(urllib.parse.urlsplit(path).path)
    if path in ("/", "/index.html", "/reports", "/reports/", "/reports/index.html"):
        return True
    relative = pathlib.PurePosixPath(path)
    return (relative.parts[:2] == ("/", "reports")
            and len(relative.parts) <= 5
            and relative.parts[2:3] in [("library",), ("history",), ("data",), ("topics",)]
            and (path.endswith("/") or path.endswith("/index.html"))) or path in (
                "/reports/eval/", "/reports/eval/index.html", "/reports/setups/", "/reports/setups/index.html")


class ReportHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, refresh=None, **kwargs):
        self.navigation_refresh = refresh
        super().__init__(*args, **kwargs)

    def send_head(self):
        if navigation_request(self.path) and self.navigation_refresh is not None:
            if not self.navigation_refresh.refresh():
                self.send_error(503, "Report navigation refresh failed; saved reports remain available")
                return None
            # A refreshed page within the same second must not get a stale 304.
            if "If-Modified-Since" in self.headers:
                del self.headers["If-Modified-Since"]
        return super().send_head()

    def end_headers(self):
        if navigation_request(self.path):
            self.send_header("Cache-Control", "no-store")
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--port", default=8000, type=int)
    parser.add_argument("--directory", required=True, type=pathlib.Path)
    parser.add_argument("--repo", type=pathlib.Path)
    parser.add_argument("--evaluator", type=pathlib.Path)
    args = parser.parse_args()
    refresh = None
    if args.evaluator:
        if args.repo is None or args.directory.resolve() != (args.repo / "outputs").resolve():
            parser.error("automatic navigation requires this repository's outputs directory")
        if not args.evaluator.is_file():
            parser.error("build pd-eval before starting automatic report navigation")
        refresh = NavigationRefresh(args.repo.resolve(), args.directory.resolve(), args.evaluator.resolve())
    handler = functools.partial(ReportHandler, directory=str(args.directory), refresh=refresh)
    server_type = http.server.ThreadingHTTPServer
    if ":" in args.bind:
        import socket
        class IPv6Server(server_type):
            address_family = socket.AF_INET6
        server_type = IPv6Server
    with server_type((args.bind, args.port), handler) as server:
        print(f"Serving {args.directory} on {args.bind}:{args.port}; navigation auto-refresh={refresh is not None}", flush=True)
        server.serve_forever()


if __name__ == "__main__":
    main()
