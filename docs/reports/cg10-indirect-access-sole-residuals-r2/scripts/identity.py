#!/usr/bin/env python3
"""identity.py BEFORE AFTER [OUT.json] -- DEEP-AST-TRANSPORTS-AND-WALL-TIME
(adapted from GATEWAY-ANALYSIS-PATH-OPTIMIZATION's scripts/identity.py): the
byte-identity proof between two measure.sh runs on the three routes.

* corpus: every file of private/cg10r2/NAME/ except run.json (its times, binary
  digest and output paths are the run's own provenance) compared by sha256,
  raw; a file that differs raw is compared again with the run's own name
  replaced by RUN (an output that names its own path), and both verdicts are
  reported;
* system: every file of private/cg10r2/NAME-system/ the same way, except that in
  console.log and stderr.log the `preparation:` line's durations (wall-time
  measurements) are erased before comparing;
* slice: every file of the shared prepared attempt's dat-NAME/ the same way;
* parity: each route's QCORE_TYPED_PATH_PARITY stream as a multiset of rows
  with Clang AST node addresses ("0x..." values, process memory) erased (rows
  are written in analysis order, which at --jobs 4 is not unit order).

Exit 0 only when every compared file and every parity multiset is identical.
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
DAT = REPO / "private/cg10r2"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ADDRESS = re.compile(rb'"0x[0-9a-f]+"')
TIMING = re.compile(rb"[0-9]+(\.[0-9]+)?\s*(us|ms|s|seconds)\b")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def normalise(data, name, path):
    # Only the run's own output paths: a short run name must not rewrite a
    # digest or any other text that happens to contain it.
    for own in (f"/private/cg10r2/{name}/", f"/private/cg10r2/{name}-system/", f"/cg10r2-{name}/"):
        data = data.replace(own.encode(), b"/RUN/")
    if path.name in ("stderr.log", "console.log"):
        data = b"\n".join(TIMING.sub(b"T", line) if b"preparation" in line else line
                          for line in data.split(b"\n"))
    return data


def compare_dirs(label, before_dir, after_dir, before, after, skip=("run.json", "exit")):
    rows, ok = [], True
    names = sorted({p.name for p in before_dir.iterdir()} | {p.name for p in after_dir.iterdir()})
    for name in names:
        if name in skip:
            continue
        b, a = before_dir / name, after_dir / name
        if not (b.is_file() and a.is_file()):
            rows.append({"route": label, "file": name, "verdict": "missing on one side"})
            ok = False
            continue
        bd, ad = b.read_bytes(), a.read_bytes()
        raw = bd == ad
        normalised = raw or normalise(bd, before, b) == normalise(ad, after, a)
        rows.append({"route": label, "file": name, "bytes": len(bd), "sha256": digest(bd),
                     "raw_identical": raw, "identical_after_normalisation": normalised})
        ok &= normalised
    return rows, ok


def parity(path):
    counter = collections.Counter()
    if path.is_file():
        for line in path.read_bytes().splitlines():
            counter[ADDRESS.sub(b'"0x"', line)] += 1
    return counter


def multiset_digest(counter):
    return digest(b"\n".join(sorted(counter.elements())))


def main():
    before, after = sys.argv[1], sys.argv[2]
    rows, ok = [], True
    for label, b, a in (
        ("corpus", DAT / before, DAT / after),
        ("system", DAT / f"{before}-system", DAT / f"{after}-system"),
        ("slice", SLICE / f"cg10r2-{before}", SLICE / f"cg10r2-{after}"),
    ):
        if not (b.is_dir() and a.is_dir()):
            rows.append({"route": label, "verdict": "not measured on both sides"})
            ok = False
            continue
        found, same = compare_dirs(label, b, a, before, after)
        rows += found
        ok &= same
    streams = []
    for suffix, tail in [(s, t) for s in ("", "-system", "-slice")
                         for t in ("-parity.jsonl", "-parity.jsonl.contexts.jsonl")]:
        label = f"{tail[1:]}{suffix or '-corpus'}"
        bp, ap = DAT / f"{before}{suffix}{tail}", DAT / f"{after}{suffix}{tail}"
        if not (bp.is_file() and ap.is_file()):
            streams.append({"stream": label, "verdict": "not on both sides",
                            "present": [bp.is_file(), ap.is_file()]})
            ok &= not (bp.is_file() or ap.is_file())
            continue
        b, a = parity(bp), parity(ap)
        same = b == a
        streams.append({"stream": label, "rows": [sum(b.values()), sum(a.values())],
                        "multiset_sha256": [multiset_digest(b), multiset_digest(a)], "identical": same})
        ok &= same
    result = {"before": before, "after": after, "identical": ok, "files": rows, "parity": streams}
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    for row in rows:
        if not row.get("identical_after_normalisation", False) or not row.get("raw_identical", True):
            print(json.dumps(row))
    for stream in streams:
        print(json.dumps(stream))
    print("IDENTICAL" if ok else "DIFFERENT", f"({sum(1 for r in rows if 'sha256' in r)} files compared)")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
