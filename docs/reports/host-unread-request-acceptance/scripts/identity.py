#!/usr/bin/env python3
"""HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING (from
EXECUTION-QUOTA-LOAD-INDEPENDENCE's identity.py): BEFORE (base release
binary) against AFTER (this tip's) on the three routes, file for file, with the
null property selected on both sides (NULL=1) and no parity stream (PARITY=0).

Corpus: private/hur/before vs private/hur/after. System: before-system vs
after-system. Slice: the shared prepared attempt's hur-before vs hur-after.
Every report and sidecar is compared byte for byte; run.json and audit.log are
compared after removing the run directory name, the binary digest and the
timestamps, which are the only fields that differ by construction, and so is
the invocation record (its argv names the run directory). The null-property
streams are compared byte for byte, else as a multiset of rows with Clang node
addresses normalised. Prints one line per file and a verdict."""
import collections
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/hur"
SLICES = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
PAIRS = [
    ("corpus", PRIVATE / "before", PRIVATE / "after"),
    ("system", PRIVATE / "before-system", PRIVATE / "after-system"),
    ("slice", SLICES / "hur-before", SLICES / "hur-after"),
]
STREAMS = [
    ("corpus null", PRIVATE / "before-null.jsonl", PRIVATE / "after-null.jsonl"),
    ("system null", PRIVATE / "before-system-null.jsonl", PRIVATE / "after-system-null.jsonl"),
    ("slice null", PRIVATE / "before-slice-null.jsonl", PRIVATE / "after-slice-null.jsonl"),
]
VOLATILE = {"console.log", "stderr.log", "exit"}
ADDRESS = re.compile(rb'"0x[0-9a-f]+"')


def normalized(path: pathlib.Path, before: str, after: str) -> bytes:
    text = path.read_text(errors="replace").replace(after, before)
    if path.name == "run.json":
        data = json.loads(text)
        for key in ("binary_sha256", "start", "end", "seconds"):
            data.pop(key, None)
        text = json.dumps(data, sort_keys=True)
    return text.encode()


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()[:16]


def main() -> int:
    ok = True
    for route, before, after in PAIRS:
        names = sorted({p.name for p in before.iterdir()} | {p.name for p in after.iterdir()})
        for name in names:
            if name in VOLATILE:
                continue
            a, b = before / name, after / name
            if not (a.exists() and b.exists()):
                print(f"{route} {name}: MISSING on one side")
                ok = False
                continue
            if name in ("run.json", "audit.log", "resource-diagnostics.invocation.json"):
                same = normalized(a, before.name, after.name) == normalized(b, before.name, after.name)
                how = "identical after removing run name, binary digest and times"
            else:
                same = a.read_bytes() == b.read_bytes()
                how = f"byte-identical {digest(a.read_bytes())}"
            print(f"{route} {name}: {how if same else 'DIFFERENT'}")
            ok &= same
    for label, a, b in STREAMS:
        if not (a.exists() and b.exists()):
            print(f"{label}: not present ({a.name}, {b.name})")
            continue
        if a.read_bytes() == b.read_bytes():
            print(f"{label}: byte-identical {digest(a.read_bytes())}")
            continue
        # Clang AST node addresses ("0x…") differ between any two runs, and
        # the stream's row order follows the --jobs workers' finishing order
        # (GAPO-RETURN-CORRECTNESS's parity.py compares the same way): compare
        # the rows as a multiset with the addresses normalised.
        rows_a = collections.Counter(ADDRESS.sub(b'"0x?"', a.read_bytes()).splitlines())
        rows_b = collections.Counter(ADDRESS.sub(b'"0x?"', b.read_bytes()).splitlines())
        same = rows_a == rows_b
        print(f"{label}: {sum(rows_a.values())} rows; "
              f"{'the same rows (as a multiset, Clang node addresses normalised)' if same else 'DIFFERENT'}")
        ok &= same
    print("IDENTICAL" if ok else "MOVED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
