#!/usr/bin/env python3
"""Card C-GATEWAY-CONSTANT-DO-WHILE, section 1.

Re-derive the `do { ... } while (0)` idiom count on the PREPROCESSED text of
the pinned Cycle 19 corpus (the card's point: macro expansion is where these
live, so a source-level grep measures the wrong text).

Writes nothing to disk: every preprocessor run goes to a pipe.
"""
import collections
import json
import pathlib
import re
import subprocess
import sys

PIN = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin"
)
BASE = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a4b9c03fd33dec337"
)
PROD = PIN / "fluent-bit-clang-production" / "compile_commands.json"
MONKEY = PIN / "monkey" / "monkey-build" / "compile_commands.json"

# The gateway analyses `-E -P` text (constraint 12a): no line markers.
WHILE_ZERO = re.compile(r"while\s*\(\s*0\s*\)")
DO_TOKEN = re.compile(r"\bdo\b")


def corpus_units():
    import gzip

    raw = gzip.decompress(
        (
            BASE
            / "docs/reports/c-gateway-return-call-w4/artifacts/diagnostics.json.gz"
        ).read_bytes()
    )
    d = json.loads(raw)
    return [(u["file"], u["library"]) for u in d["units"]]


def load_db(path):
    entries = json.load(open(path))
    by_base = {}
    for entry in entries:
        base = pathlib.Path(entry["file"]).name
        by_base.setdefault(base, entry)
    return by_base, entries


def resolve(base, by_base, entries):
    """Unit `file` fields are sometimes path-qualified (`nanopb/pb_decode.c`).
    Fall back to a suffix match against the database's full paths."""
    hit = by_base.get(base)
    if hit is not None:
        return hit
    hit = by_base.get(pathlib.Path(base).name)
    if hit is not None:
        return hit
    for entry in entries:
        if entry["file"].endswith("/" + base):
            return entry
    return None


def preprocess(entry):
    args = entry.get("arguments")
    if args is None:
        import shlex

        args = shlex.split(entry["command"])
    out = []
    skip = False
    for i, a in enumerate(args):
        if skip:
            skip = False
            continue
        if i == 0:
            out.append("/usr/bin/clang")
            continue
        if a in ("-o", "-MF", "-MT", "-MQ"):
            skip = True
            continue
        if a in ("-c",) or a.startswith("-M"):
            continue
        out.append(a)
    out[1:1] = ["-E", "-P"]
    try:
        r = subprocess.run(
            out,
            cwd=entry["directory"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=180,
        )
    except subprocess.TimeoutExpired:
        return None
    if r.returncode != 0:
        return None
    return r.stdout.decode("utf-8", "replace")


def main():
    units = corpus_units()
    db, entries = load_db(PROD)
    mdb, mentries = load_db(MONKEY)
    db.update(mdb)
    entries = entries + mentries
    per_lib = collections.Counter()
    per_unit = {}
    missing = []
    failed = []
    for base, lib in units:
        entry = resolve(base, db, entries)
        if entry is None:
            missing.append((base, lib))
            continue
        text = preprocess(entry)
        if text is None:
            failed.append((base, lib))
            continue
        n = len(WHILE_ZERO.findall(text))
        per_unit[base] = n
        per_lib[lib] += n
    total = sum(per_lib.values())
    print("=== preprocessed `while (0)` census over the pinned 185-unit corpus ===")
    for lib, n in sorted(per_lib.items()):
        print(f"  {lib:>18}  {n:>7}")
    print(f"  {'TOTAL':>18}  {total:>7}")
    print()
    print(f"units measured: {len(per_unit)}  missing-from-db: {len(missing)}  "
          f"preprocess-failed: {len(failed)}")
    if missing:
        print("missing:", missing[:12])
    if failed:
        print("failed:", failed[:12])
    print()
    print("top 15 units by occurrence:")
    for base, n in sorted(per_unit.items(), key=lambda kv: -kv[1])[:15]:
        print(f"  {n:>6}  {base}")
    json.dump(
        {"per_library": dict(per_lib), "per_unit": per_unit,
         "missing": missing, "failed": failed, "total": total},
        open(sys.argv[1], "w") if len(sys.argv) > 1 else sys.stdout,
        indent=1, sort_keys=True,
    )


if __name__ == "__main__":
    main()
