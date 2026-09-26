#!/usr/bin/env python3
"""W45-FB-ENGINE-PROGRESS: compare source files between two Fluent Bit trees.

Usage: pin_compare.py OLD_TREE NEW_TREE [--out FILE] FILE...

Read-only. Prints, per repository-relative FILE, the SHA-256 in both trees and,
when they differ, a unified diff's changed-line count. With --out, writes the
record as JSON. Used to re-check the cb_queue_chunks observation's lines after
the pin moved from 864c3d97 to 046accf6.
"""

import argparse
import difflib
import hashlib
import json
import pathlib


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("old", type=pathlib.Path)
    parser.add_argument("new", type=pathlib.Path)
    parser.add_argument("files", nargs="+")
    parser.add_argument("--out", type=pathlib.Path)
    args = parser.parse_args()
    rows = []
    for rel in args.files:
        a = (args.old / rel).read_bytes()
        b = (args.new / rel).read_bytes()
        row = {
            "file": rel,
            "old_sha256": hashlib.sha256(a).hexdigest(),
            "new_sha256": hashlib.sha256(b).hexdigest(),
            "identical": a == b,
        }
        if a != b:
            diff = list(difflib.unified_diff(a.decode(errors="replace").splitlines(),
                                             b.decode(errors="replace").splitlines(),
                                             lineterm="", n=0))
            row["changed_lines"] = sum(1 for d in diff if d[:1] in "+-" and d[:3] not in ("+++", "---"))
            row["hunks"] = [d for d in diff if d.startswith("@@")]
        rows.append(row)
        print(("same " if a == b else "DIFF ") + rel + ("" if a == b else f" ({row['changed_lines']} lines)"))
    if args.out:
        args.out.write_text(json.dumps({"old": str(args.old), "new": str(args.new), "files": rows}, indent=1) + "\n")


if __name__ == "__main__":
    main()
