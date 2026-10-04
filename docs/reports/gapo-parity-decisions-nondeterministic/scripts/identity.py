#!/usr/bin/env python3
"""GAPO-PARITY-DECISIONS-NONDETERMINISTIC: the identity proof across runs,
successor of GATEWAY-ANALYSIS-PATH-OPTIMIZATION's identity.py with no row
compared apart.

Usage: identity.py OUT.json REFERENCE NAME [NAME...]

Every NAME is compared with REFERENCE (runs of measure.sh under
private/gapo-nd/):

* corpus: every file of private/gapo-nd/NAME/ except run.json (its times,
  binary digest and output paths are the run's own provenance) by sha256,
  raw; a file that differs raw is compared again with the run's own name
  replaced by RUN (an output that names its own path);
* system: every file of private/gapo-nd/NAME-system/ the same way, with the
  `preparation:` line's durations erased in console.log and stderr.log (a
  wall-time measurement, validation policy);
* slice: every file of the shared prepared attempt's gapo-nd-NAME/ the same
  way;
* parity: each route's QCORE_TYPED_PATH_PARITY stream as a multiset of rows
  with Clang AST node addresses ("0x..." values, process memory) erased.
  Rows are written in analysis order, which at --jobs 4 is not unit order,
  so the raw byte order is not compared; the multiset is, in full: the
  optimization card's VARYING exception is gone.

A route a run did not measure is reported as such. For each pair of
differing parity streams the rows only one side has are listed with their
counts. Exit 0 only when every compared file and every parity multiset is
identical.
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
RUNS = REPO / "private/gapo-nd"
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


def compare_dirs(label, reference_dir, run_dir, reference, run, skip=("run.json", "exit")):
    rows, ok = [], True
    names = sorted({p.name for p in reference_dir.iterdir()} | {p.name for p in run_dir.iterdir()})
    for name in names:
        if name in skip:
            continue
        b, a = reference_dir / name, run_dir / name
        if not (b.is_file() and a.is_file()):
            rows.append({"route": label, "file": name, "verdict": "missing on one side"})
            ok = False
            continue
        bd, ad = b.read_bytes(), a.read_bytes()
        raw = bd == ad
        normalised = raw or normalise(bd, reference, b) == normalise(ad, run, a)
        rows.append({"route": label, "file": name, "bytes": len(ad), "sha256": digest(ad),
                     "raw_identical": raw, "identical_after_normalisation": normalised})
        ok &= normalised
    return rows, ok


def parity(path):
    counter = collections.Counter()
    for line in path.read_bytes().splitlines():
        counter[ADDRESS.sub(b'"0x"', line)] += 1
    return counter


def multiset_digest(counter):
    return digest(b"\n".join(sorted(counter.elements())))


def compare(reference, run):
    rows, ok = [], True
    for label, b, a in (
        ("corpus", RUNS / reference, RUNS / run),
        ("system", RUNS / f"{reference}-system", RUNS / f"{run}-system"),
        ("slice", SLICE / f"gapo-nd-{reference}", SLICE / f"gapo-nd-{run}"),
    ):
        if not (b.is_dir() and a.is_dir()):
            rows.append({"route": label, "verdict": "not measured on both sides"})
            continue
        found, same = compare_dirs(label, b, a, reference, run)
        rows += found
        ok &= same
    streams = []
    for suffix in ("", "-system", "-slice"):
        b_path = RUNS / f"{reference}{suffix}-parity.jsonl"
        a_path = RUNS / f"{run}{suffix}-parity.jsonl"
        stream = f"parity{suffix or '-corpus'}"
        if not (b_path.is_file() and a_path.is_file()):
            streams.append({"stream": stream, "verdict": "not measured on both sides"})
            continue
        b, a = parity(b_path), parity(a_path)
        same = b == a
        entry = {"stream": stream, "rows": [sum(b.values()), sum(a.values())],
                 "multiset_sha256": [multiset_digest(b), multiset_digest(a)], "identical": same}
        if not same:
            entry["only_reference"] = [{"count": c, "row": r.decode()} for r, c in (b - a).items()]
            entry["only_run"] = [{"count": c, "row": r.decode()} for r, c in (a - b).items()]
        streams.append(entry)
        ok &= same
    return {"reference": reference, "run": run, "identical": ok, "files": rows, "parity": streams}


def main():
    out, reference, runs = sys.argv[1], sys.argv[2], sys.argv[3:]
    results = [compare(reference, run) for run in runs]
    pathlib.Path(out).write_text(json.dumps(results, indent=1) + "\n")
    for result in results:
        for row in result["files"]:
            if "verdict" in row or not row["identical_after_normalisation"]:
                print(result["run"], json.dumps(row))
        for stream in result["parity"]:
            summary = {k: v for k, v in stream.items() if k not in ("only_reference", "only_run")}
            print(result["run"], json.dumps(summary))
            for side in ("only_reference", "only_run"):
                for item in stream.get(side, []):
                    print(f"  {side} x{item['count']}: {item['row'][:400]}")
        compared = sum(1 for r in result["files"] if "sha256" in r)
        print(result["run"], "IDENTICAL" if result["identical"] else "DIFFERENT",
              f"({compared} files compared)")
    return 0 if all(result["identical"] for result in results) else 1


if __name__ == "__main__":
    sys.exit(main())
