#!/usr/bin/env python3
"""callers.py RUN SOURCE_ROOT [FILE...]: every analysed function whose body
calls a condition-variable, join or spawn function, with its status and the
leaves that name the callee (card W47-CONDVAR-JOIN-PUBLICATION).

Adapted from docs/reports/w47-loop-owner-contracts/scripts/callers.py (the
same crude column-0 brace reader). RUN is a resource-diagnostics.json[.gz] or
a directory holding one. The call sites are read from SOURCE_ROOT, so a call
that produced no leaf is listed too. Only units the run analysed are read;
FILE restricts that further. The kept refusals (broadcast, C11 cnd_*, rwlocks,
try/timed locks) are listed too, so the prediction can say where they stand."""
import gzip
import json
import pathlib
import re
import sys

DECLARED = ["pthread_cond_wait", "pthread_cond_timedwait", "pthread_cond_signal",
            "pthread_cond_init", "pthread_cond_destroy", "pthread_join"]
OTHERS = ["pthread_create", "pthread_cond_broadcast", "cnd_wait", "cnd_timedwait",
          "cnd_signal", "cnd_broadcast", "pthread_rwlock_rdlock", "pthread_rwlock_wrlock",
          "pthread_mutex_trylock", "pthread_mutex_timedlock", "pthread_timedjoin_np",
          "pthread_tryjoin_np", "mk_utils_worker_spawn", "flb_start", "flb_stop"]
OPS = DECLARED + OTHERS
CALL = re.compile(r"\b(" + "|".join(OPS) + r")\s*\(")
DEFINITION = re.compile(r"^[A-Za-z_][^;(]*?\b([A-Za-z_]\w*)\s*\([^;]*$")


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        gz = path / "resource-diagnostics.json.gz"
        path = gz if gz.exists() else path / "resource-diagnostics.json"
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def calls_by_function(text):
    """(function, callee, line) for every call inside a column-0 definition."""
    current, pending, found = None, None, []
    for number, line in enumerate(text.splitlines(), 1):
        if current is None:
            match = DEFINITION.match(line)
            if match and not line.startswith(("typedef", "#")):
                pending = match.group(1)
            elif line and not line[0].isspace() and not line.startswith(("{", "#")):
                pending = None if line.rstrip().endswith(";") else pending
            if line.startswith("{"):
                current, pending = pending, None
            continue
        if line.startswith("}"):
            current = None
            continue
        for match in CALL.finditer(line):
            found.append((current, match.group(1), number))
    return found


def main():
    data = load(sys.argv[1])
    source = pathlib.Path(sys.argv[2])
    wanted = set(sys.argv[3:])
    rows = []
    for unit in data["units"]:
        path = unit["file"]
        if wanted and path not in wanted:
            continue
        candidates = [source / path, source / "src" / path, source / "lib/monkey" / path]
        candidate = next((c for c in candidates if c.is_file()), None)
        status = {f["function"]: f.get("status") for f in unit.get("functions", [])}
        leaves = {f["function"]: f.get("diagnostics", []) for f in unit.get("functions", [])}
        if candidate is None:
            continue
        for function, callee, line in calls_by_function(candidate.read_text(errors="replace")):
            if function == callee:
                continue
            rows.append({
                "file": path, "function": function, "callee": callee, "line": line,
                "declared_by_this_card": callee in DECLARED,
                "status": status.get(function, "not analysed"),
                "leaves": sorted({d.get("description") for d in leaves.get(function, [])
                                  if callee in (d.get("description") or "")}),
            })
    json.dump(rows, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
