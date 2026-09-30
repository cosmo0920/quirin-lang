#!/usr/bin/env python3
"""identity.py BEFORE_DIR AFTER_DIR -- card CG09-TYPED-SOURCE-CONTROL: compare two
corpus arbiter runs file by file.

For every file of either run it records the SHA-256 on each side. For the two
files that carry run metadata it states what differs: `run.json` (timestamps,
paths, the binary digest) and `audit.log` (its output-path lines, compared with
those lines removed). `parity.jsonl` is compared as a multiset of records with Clang node addresses
replaced (they are process addresses), since
`--jobs` streams it in scheduling order. The deduplicated verdict totals come
from CG14's `compare.py`, which reads `resource-diagnostics.json`.
"""
import collections
import hashlib
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
COMPARE = HERE.parents[1] / "cg14-aggregate-values-and-initializers/scripts/compare.py"


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit_lines(path: pathlib.Path, root: pathlib.Path) -> list[str]:
    return [line.replace(str(root), "OUT") for line in path.read_text().splitlines()]


ADDRESS = re.compile(rb"0x[0-9a-f]+")


def records(path: pathlib.Path) -> collections.Counter:
    # Clang JSON node IDs are process addresses (`"decl":"0x652a…"`), which
    # differ between any two runs; they are replaced before comparing.
    with path.open("rb") as handle:
        return collections.Counter(
            hashlib.sha256(ADDRESS.sub(b"0xADDR", line)).hexdigest() for line in handle)


def main() -> int:
    before, after = (pathlib.Path(arg).resolve() for arg in sys.argv[1:3])
    names = sorted({p.name for p in before.iterdir()} | {p.name for p in after.iterdir()})
    files = {}
    for name in names:
        a, b = before / name, after / name
        row = {
            "before": digest(a) if a.exists() else None,
            "after": digest(b) if b.exists() else None,
        }
        row["identical"] = row["before"] == row["after"]
        if not row["identical"] and a.exists() and b.exists():
            if name == "audit.log":
                row["identical_with_output_paths_normalised"] = (
                    audit_lines(a, before) == audit_lines(b, after))
            elif name.endswith(".jsonl"):
                ra, rb = records(a), records(b)
                row["same_records_in_any_order"] = ra == rb
                row["records"] = [sum(ra.values()), sum(rb.values())]
            elif name == "run.json":
                ja, jb = json.loads(a.read_text()), json.loads(b.read_text())
                row["differing_keys"] = sorted(k for k in ja.keys() | jb.keys()
                                               if ja.get(k) != jb.get(k))
        files[name] = row
    totals = json.loads(subprocess.run(
        [sys.executable, str(COMPARE), str(before), str(after)],
        check=True, capture_output=True, text=True).stdout)
    print(json.dumps({
        "method": __doc__.strip(),
        "before": str(before),
        "after": str(after),
        "files": files,
        "totals": totals["totals"],
        "move_counts": totals["move_counts"],
        "same_status_functions_with_changed_diagnostics":
            totals["same_status_functions_with_changed_diagnostics"],
    }, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
