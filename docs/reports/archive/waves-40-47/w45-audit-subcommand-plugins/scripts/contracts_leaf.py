#!/usr/bin/env python3
"""List every leaf in which a run's contracts sidecar differs from a baseline's.

Usage: contracts_leaf.py RUN_CONTRACTS BASELINE_CONTRACTS[.gz]

The run's worktree root (this repository) is first aliased to the baseline's
worktree root, as `baseline_identity.py --root-alias` does. Prints one line per
differing leaf and exits 0; an empty listing means identical after the alias.
"""

import gzip
import json
import pathlib
import sys

ROOT = str(pathlib.Path(__file__).resolve().parents[4])
BASELINE_ROOT = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da"


def norm(value):
    if isinstance(value, str):
        return value.replace(ROOT, BASELINE_ROOT)
    if isinstance(value, list):
        return [norm(item) for item in value]
    if isinstance(value, dict):
        return {norm(key): norm(item) for key, item in value.items()}
    return value


def diff(run, base, path, out):
    if type(run) is not type(base):
        out.append(f"type {path}")
    elif isinstance(run, dict):
        for key in sorted(set(run) | set(base)):
            if key not in run:
                out.append(f"only-baseline {path}/{key}")
            elif key not in base:
                out.append(f"only-run {path}/{key} = {json.dumps(run[key])[:200]}")
            else:
                diff(run[key], base[key], f"{path}/{key}", out)
    elif isinstance(run, list):
        if len(run) != len(base):
            out.append(f"length {path} {len(run)} {len(base)}")
        for index, (left, right) in enumerate(zip(run, base)):
            diff(left, right, f"{path}[{index}]", out)
    elif run != base:
        out.append(f"value {path} {str(run)[:120]} | {str(base)[:120]}")


def main():
    run = norm(json.loads(pathlib.Path(sys.argv[1]).read_text()))
    base_path = pathlib.Path(sys.argv[2])
    raw = base_path.read_bytes()
    base = json.loads(gzip.decompress(raw) if base_path.suffix == ".gz" else raw)
    out = []
    diff(run, base, "", out)
    print("\n".join(out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
