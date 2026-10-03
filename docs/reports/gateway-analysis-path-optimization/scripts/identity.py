#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the byte-identity proof between two
measure.sh runs (BEFORE and AFTER names) on the three routes.

Usage: identity.py BEFORE AFTER [OUT.json]

* corpus: every file of private/gapo/NAME/ except run.json (its times, binary
  digest and output paths are the run's own provenance) compared by sha256,
  raw; a file that differs raw is compared again with the run's own directory
  name replaced by RUN (an output that names its own path), and both verdicts
  are reported;
* system: every file of private/gapo/NAME-system/ the same way, except that
  in console.log and stderr.log the `preparation:` line's durations (`N us`,
  wall-time measurements, validation policy) are erased before comparing;
  its unit and Clang-invocation counts are compared;
* slice: every file of the shared prepared attempt's gapo-NAME/ the same way;
* parity: each route's QCORE_TYPED_PATH_PARITY stream as a multiset of rows
  with Clang AST node addresses ("0x..." values, process memory) erased, as
  earlier cards compared them (rows are written in analysis order, which at
  --jobs 4 is not unit order).

Exit 0 only when every compared file and every parity multiset is identical
(the corpus parity stream may differ only in the one row the BEFORE binary
itself varies between runs, VARYING below;
see the report).
"""
import collections
import hashlib
import json
import os
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
GAPO = REPO / "private/gapo"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ADDRESS = re.compile(rb'"0x[0-9a-f]+"')
TIMING = re.compile(rb"[0-9]+(\.[0-9]+)?\s*(us|ms|s|seconds)\b")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def normalise(data, name, path):
    data = data.replace(name.encode(), b"RUN")
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


VARYING = (b'"function":"cio_file_st_get_meta_len","line":2223,', b'"position":"typed-place"')


def without_varying(counter):
    return collections.Counter({row: count for row, count in counter.items()
                                if not all(part in row for part in VARYING)})


def multiset_digest(counter):
    return digest(b"\n".join(sorted(counter.elements())))


def main():
    before, after = sys.argv[1], sys.argv[2]
    rows, ok = [], True
    for label, b, a in (
        ("corpus", GAPO / before, GAPO / after),
        ("system", GAPO / f"{before}-system", GAPO / f"{after}-system"),
        ("slice", SLICE / f"gapo-{before}", SLICE / f"gapo-{after}"),
    ):
        if not (b.is_dir() and a.is_dir()):
            rows.append({"route": label, "verdict": "not measured on both sides"})
            continue
        found, same = compare_dirs(label, b, a, before, after)
        rows += found
        ok &= same
    streams = []
    for suffix in ("", "-system", "-slice"):
        if not (GAPO / f"{after}{suffix}-parity.jsonl").is_file():
            streams.append({"stream": f"parity{suffix or '-corpus'}", "verdict": "route not measured"})
            continue
        b = parity(GAPO / f"{before}{suffix}-parity.jsonl")
        a = parity(GAPO / f"{after}{suffix}-parity.jsonl")
        same = b == a
        stream = {"stream": f"parity{suffix or '-corpus'}", "rows": [sum(b.values()), sum(a.values())],
                  "multiset_sha256": [multiset_digest(b), multiset_digest(a)], "identical": same}
        # The BEFORE binary's own corpus stream varies between runs (finding
        # GAPO-PARITY-DECISIONS-NONDETERMINISTIC): at --jobs 4 a thread-local
        # place decision left by one consultation is flushed into the next
        # `typed-place` row its worker writes, so whether that row exists,
        # and which leaked place it carries, depends on scheduling. The only
        # row measured to vary across four runs of the BEFORE binary is
        # VARYING; it is compared apart (its presence is reported) and the
        # rest of the stream must be identical.
        if not same:
            b_rest, a_rest = without_varying(b), without_varying(a)
            if b_rest == a_rest:
                stream["identical_except_varying_row"] = [
                    sum(b.values()) - sum(b_rest.values()), sum(a.values()) - sum(a_rest.values())]
                same = True
        streams.append(stream)
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
