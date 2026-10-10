"""Approach-owned bindings for the actual installed learning runtime."""
from pathlib import Path
import sys
import sysconfig
import gymnasium
import numpy
import stable_baselines3
import torch


def backend_paths():
    """Actual installed code/libraries, not just a lockfile version claim."""
    sources, binaries = [], []
    for module in [gymnasium, numpy, stable_baselines3, torch]:
        root = Path(module.__file__).resolve().parent
        sources.extend(sorted(root.rglob("*.py")))
        binaries.extend(path for path in root.rglob("*") if path.is_file() and ".so" in path.name)
        sibling = root.parent/(root.name+".libs")
        if sibling.is_dir():
            binaries.extend(path for path in sibling.rglob("*") if path.is_file())
    library = Path(sysconfig.get_config_var("LIBDIR"))/sysconfig.get_config_var("LDLIBRARY")
    if library.exists():
        binaries.append(library)
    return sorted(set(str(path.resolve()) for path in sources)), sorted(set(str(path.resolve()) for path in binaries))



def backend_versions():
    return {"python": sys.version, **{module.__name__: module.__version__ for module in [gymnasium, numpy, stable_baselines3, torch]}}
