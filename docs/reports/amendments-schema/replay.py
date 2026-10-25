#!/usr/bin/env python3
"""Replay an integration commit's amendments with scripts/apply_amendments.py
(card AMENDMENTS-SCHEMA) and compare the result with what the integrator
committed by hand.

    python3 docs/reports/amendments-schema/replay.py <commit> <amendments path> [--legacy] [--out DIR]

The parent tree's increment index, current increment record, STATUS.md and
acceptance matrix are read from git objects (never the working tree) into a
fresh directory, the amendments file is read at <commit>, the tool is applied
there, and each file is compared byte for byte with <commit>'s version. For the
matrix it also compares `named_followups` alone, because a landing that
regenerated `dimensions`/`validation` in the same commit (build_matrix.py owns
those) differs there by design. Prints one JSON object; exit 0 when every
compared file the commit changed is identical.
"""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))
import apply_amendments  # noqa: E402

FILES = [apply_amendments.INCREMENT_INDEX, apply_amendments.STATUS, apply_amendments.MATRIX]


def show(revision, path):
    return subprocess.run(["git", "-C", str(ROOT), "show", f"{revision}:{path}"],
                          check=True, capture_output=True).stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("commit")
    parser.add_argument("amendments")
    parser.add_argument("--legacy", action="store_true")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    parent = f"{args.commit}^1"
    out = args.out or Path(tempfile.mkdtemp(prefix="amendments-replay-"))
    out.mkdir(parents=True, exist_ok=True)
    for path in FILES:
        (out / path).parent.mkdir(parents=True, exist_ok=True)
        (out / path).write_bytes(show(parent, path))
    record = apply_amendments.current_increment_record(out, must_exist=False)
    (out / record).write_bytes(show(parent, record))
    amendments = out / "amendments.json"
    amendments.write_bytes(show(args.commit, args.amendments))
    argv = [str(amendments), "--root", str(out)] + (["--legacy"] if args.legacy else [])
    exit_code = apply_amendments.main(argv)
    result = {"commit": args.commit, "parent": parent, "amendments": args.amendments,
              "legacy": args.legacy, "tool_exit": exit_code, "root": str(out), "files": {}}
    ok = exit_code == 0
    for path in (record, apply_amendments.STATUS, apply_amendments.MATRIX):
        before = show(parent, path)
        expected = show(args.commit, path)
        produced = (out / path).read_bytes()
        entry = {"changed_by_commit": before != expected,
                 "changed_by_tool": before != produced,
                 "byte_identical": produced == expected}
        if path == apply_amendments.MATRIX:
            entry["named_followups_identical"] = (
                json.loads(produced)["named_followups"] == json.loads(expected)["named_followups"])
            other = sorted(key for key in json.loads(expected)
                           if json.loads(expected)[key] != json.loads(produced).get(key))
            entry["other_keys_differing"] = other
            ok &= entry["named_followups_identical"]
        elif entry["changed_by_commit"] or entry["changed_by_tool"]:
            ok &= entry["byte_identical"]
        result["files"][str(path)] = entry
    result["parity"] = ok
    print(json.dumps(result, indent=1))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
