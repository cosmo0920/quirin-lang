#!/usr/bin/env python3
"""Move only reviewed lock module paths after the EXT-02 source rename.

The baseline is committed before source mutation. This tool requires every
lock to be byte-identical to that baseline before editing and rechecks every
entry after changing the exact owner-scoped `module` fields. It never invokes
`pin --regenerate` or changes declaration/review identities.
"""

import json
import pathlib
import sys

from verify_paths import BASELINE, ROOT, sha256


def main():
    if sys.argv[1:] != ["--apply"]:
        raise ValueError("pass --apply after the EXT-01 gate and source rename")
    data = json.loads(BASELINE.read_text())
    staged = []
    changed = 0
    for relative, before in data["locks"].items():
        path = ROOT / relative
        if sha256(path) != before["sha256"]:
            raise ValueError(f"lock differs from pre-migration baseline: {relative}")
        lock = json.loads(path.read_text())
        entries = lock["pinned"] + lock["flexible"]
        if len(entries) != len(before["entries"]):
            raise ValueError(f"lock entry count changed: {relative}")
        for entry, expected in zip(entries, before["entries"], strict=True):
            if entry["function"] != expected["function"] or entry["module"] != expected["old_module"]:
                raise ValueError(f"lock entry order or identity changed: {relative}")
            if entry["module"] != expected["new_module"]:
                entry["module"] = expected["new_module"]
                changed += 1
        staged.append((path, json.dumps(lock, indent=2) + "\n"))
    if changed != 230:
        raise ValueError(f"expected 230 reviewed module-path edits, found {changed}")
    for path, text in staged:
        path.write_text(text)
    print(f"migrated {changed} exact module fields across {len(staged)} locks")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        print(f"lock migration refused: {error}", file=sys.stderr)
        raise SystemExit(1)
