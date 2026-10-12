"""Build an installed bridge and retain a matching create-only receipt.

Run in a dedicated Python environment. Workspace Rust/report inputs and loaded
native modules are bound; evaluate.py separately binds installed learning code.
This records a reproducible local build, not a hermetic toolchain attestation.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import sysconfig


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_paths(workspace):
    paths = [workspace/"Cargo.toml", workspace/"Cargo.lock"]
    for package in ("pd-core", "pd-run", "pd-missions", "pd-eval", "pd-report", "pd-python"):
        paths.append(workspace/package/"Cargo.toml")
        paths.extend(p for p in (workspace/package/"src").rglob("*") if p.is_file())
    return sorted(paths)


def build(output):
    workspace = Path(__file__).resolve().parents[2]
    output = Path(output).resolve()
    if sys.prefix == sys.base_prefix:
        raise ValueError("build requires a dedicated Python environment")
    paths = source_paths(workspace)
    before = {str(p): sha(p) for p in paths}
    output.mkdir()
    command = [sys.executable, "-m", "maturin", "develop", "--release", "--manifest-path", str(workspace/"pd-python/Cargo.toml")]
    with (output/"build.log").open("x") as log:
        result = subprocess.run(command, cwd=workspace, stdout=log, stderr=subprocess.STDOUT)
    after = {str(p): sha(p) for p in source_paths(workspace)}
    if result.returncode or before != after:
        raise RuntimeError("native build failed or source changed; retain failed build directory")
    sources = []
    for path in paths:
        retained = output/"sources"/path.relative_to(workspace)
        retained.parent.mkdir(parents=True, exist_ok=True)
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != before[str(path)]:
            raise RuntimeError("compiled source changed before retention")
        with retained.open("xb") as stream: stream.write(data)
        sources.append({"path": str(path), "retained_path": str(retained), "sha256": before[str(path)]})
    # Import after installation so the receipt describes the executing build.
    from pd_env import native_module_paths
    import pd_native
    if not hasattr(pd_native, "describe_collection"):
        raise RuntimeError("installed bridge lacks current collection metadata API")
    receipt = {"schema_version": 1,
        "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=workspace, text=True).strip(),
        "workspace": str(workspace), "sources": sources,
        "native_modules": {str(p): sha(p) for p in native_module_paths()},
        "build": {"command": command, "source_unchanged": True,
                  "cargo_target_dir": os.environ.get("CARGO_TARGET_DIR", str(workspace/"target")),
                  "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                  "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip()},
        "python_abi": {"executable": sys.executable, "version": sys.version,
                       "soabi": sysconfig.get_config_var("SOABI"), "environment": sys.prefix},
        "retention_note": "New path-bound receipt; historical receipts and environments remain unchanged."}
    with (output/"native-build-receipt.json").open("x") as stream:
        json.dump(receipt, stream, indent=2); stream.write("\n")
    print(json.dumps({"receipt": str(output/"native-build-receipt.json"), "sources": len(sources),
                      "native_modules": len(receipt["native_modules"]), "source_unchanged": True}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("--output", type=Path, required=True)
    build(parser.parse_args().output)
