#!/usr/bin/env python3
"""artifacts.py -- NULL-TRIAGE-UNRESOLVED-PATHS (from NULL-CANDIDATE-TRIAGE): the report's artifacts from the b1
(BEFORE, main d7b0882ac) and a3 (AFTER, the rebased tip) runs (private/ntu/, the slice under the shared
prepared attempt). Run from the repository root after triage.py and
compare.py. The findings artifact carries, per finding, the file, line,
function, class, NULL condition, path (each step with its original line)
and source excerpts from the pinned corpus revision 66910c10 (the read-only
corpus tree)."""
import collections
import gzip
import json
import pathlib
import shutil

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/ntu"
OUT = REPO / "docs/reports/null-triage-unresolved-paths/artifacts"
SOURCE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/lib")
LIBRARY_DIRS = {
    "librdkafka": "librdkafka-2.15.0/src",
    "cmetrics": "cmetrics/src",
    "ctraces": "ctraces/src",
    "monkey-libevent": "monkey",
    "chunkio": "chunkio/src",
}
OUT.mkdir(parents=True, exist_ok=True)
_cache = {}


def source_lines(library, origin_file):
    key = (library, origin_file)
    if key in _cache:
        return _cache[key]
    base = SOURCE / LIBRARY_DIRS.get(library, "")
    found = sorted(p for p in base.rglob(pathlib.Path(origin_file).name) if str(p).endswith(origin_file))
    lines = found[0].read_text(errors="replace").splitlines() if found else None
    _cache[key] = (str(found[0].relative_to(SOURCE.parent)) if found else None, lines)
    return _cache[key]


def excerpt(library, origin, around=2):
    if not origin or ":" not in origin:
        return None
    path, line = origin.rsplit(":", 1)
    line = int(line)
    where, lines = source_lines(library, path)
    if lines is None:
        return None
    lo, hi = max(1, line - around), min(len(lines), line + around)
    return {"path": where, "lines": [f"{n:5d}{'>' if n == line else ' '} {lines[n - 1]}" for n in range(lo, hi + 1)]}


def copy(source, name):
    shutil.copyfile(source, OUT / name)


triage = json.loads((PRIVATE / "a3-triage.json").read_text())
findings = []
for entry in triage["findings"]:
    path = entry.get("path") or []
    event = next((s for s in path if s["step"] == "null-event"), None)
    findings.append({
        "library": entry["library"],
        "file": entry["file"],
        "function": entry["function"],
        "line": entry["origin"],
        "check": entry["kind"],
        "subject": entry["subject"],
        "class": entry["class"],
        "condition": entry["reason"],
        "path": [{"origin": s.get("origin"), "step": s["step"], "text": s["text"]} for s in path],
        "excerpt_dereference": excerpt(entry["library"], entry["origin"]),
        "excerpt_null_event": excerpt(entry["library"], event.get("origin")) if event else None,
        "status": "provisional: a validated path in the function under any arguments its preconditions admit; never an upstream defect until reviewed",
    })
(OUT / "findings.json").write_text(json.dumps({
    "revision": "66910c10a4d7eafa810b84229cf2dcf7b0f26f97",
    "findings": len(findings),
    "by_class": dict(collections.Counter(f["class"] for f in findings)),
    "functions": len({(f["library"], f["file"], f["function"]) for f in findings}),
    "entries": findings,
}, indent=1, ensure_ascii=False) + "\n")
(OUT / "triage-corpus.json").write_text(json.dumps({
    k: triage[k] for k in ("searched", "by_status", "findings_by_class", "reasons", "not_findings")
}, indent=1, ensure_ascii=False) + "\n")
for route, name in (("", "corpus"), ("-system", "system"), ("-slice", "slice")):
    copy(PRIVATE / f"b1{route}-null.summary.json", f"null-summary-before-{name}.json")
    copy(PRIVATE / f"a3{route}-null.summary.json", f"null-summary-after-{name}.json")
print(len(findings), "findings")
