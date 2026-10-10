#!/usr/bin/env python3
"""identity.py BEFORE AFTER OUT.json -- NULL-DEREF-QTT-GROUNDING: byte
identity of every output of the measured routes between two runs of
measure.sh (NULL=1, with the parity stream), BEFORE on main's binary and
AFTER on the card's. Per route, every output file is hashed (sha256) on both
sides, and the route is identical only when the file sets and every digest
agree. `run.json` names the binary and wall time, and a route's `exit` and
`stderr.log` are the run's own, so they are reported but not compared. The
console transcripts (`audit.log`, `console.log`) and the system route's
`resource-diagnostics.invocation.json` are compared after normalisation: the
run's own directory names (`ndqg/NAME`, `ndqg-NAME`) are replaced by one
placeholder and the `preparation:` timing line is dropped (validation.md: it
differs between any two runs). Every other file is compared byte for byte.
Exits 0 only when every compared file is identical; nothing is piped."""
import hashlib
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
OUT = REPO / "private/ndqg"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
# Files that record the run itself (binary digest, timings, exit), not its result.
RUN_RECORDS = {"run.json", "exit", "stderr.log"}
NORMALISED = {"audit.log", "console.log", "resource-diagnostics.invocation.json"}


def digest(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def normalised_digest(path: pathlib.Path, name: str) -> str:
    text = path.read_text()
    for token in (f"ndqg/{name}-system", f"ndqg/{name}-slice", f"ndqg/{name}", f"ndqg-{name}"):
        text = text.replace(token, "ndqg/RUN")
    kept = [line for line in text.splitlines() if not line.startswith("preparation:")]
    return hashlib.sha256("\n".join(kept).encode()).hexdigest()


def outputs(name: str) -> dict:
    """Every output file of one run, by route, keyed by a name both runs share."""
    routes = {"corpus": {}, "system": {}, "slice": {}}
    for path in sorted((OUT / name).iterdir()):
        routes["corpus"][f"arbiter/{path.name}"] = path
    for suffix in ("null.jsonl", "null.summary.json", "parity.jsonl", "parity.jsonl.contexts.jsonl"):
        for route, stem in (("corpus", name), ("system", f"{name}-system"), ("slice", f"{name}-slice")):
            path = OUT / f"{stem}-{suffix}"
            if path.exists():
                routes[route][suffix] = path
    for path in sorted((OUT / f"{name}-system").iterdir()):
        routes["system"][f"run/{path.name}"] = path
    for path in sorted((SLICE / f"ndqg-{name}").iterdir()):
        routes["slice"][f"run/{path.name}"] = path
    return routes


def main() -> int:
    before, after, out = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
    left, right = outputs(before), outputs(after)
    result = {"before": before, "after": after, "routes": {}}
    identical = True
    for route in ("corpus", "system", "slice"):
        keys = sorted(set(left[route]) | set(right[route]))
        files = {}
        for key in keys:
            a, b = left[route].get(key), right[route].get(key)
            base = key.rsplit("/", 1)[-1]
            if base in NORMALISED:
                entry = {
                    "before": normalised_digest(a, before) if a else None,
                    "after": normalised_digest(b, after) if b else None,
                    "normalised": True,
                }
            else:
                entry = {
                    "before": digest(a) if a else None,
                    "after": digest(b) if b else None,
                }
            entry["bytes"] = b.stat().st_size if b else None
            if base in RUN_RECORDS:
                entry["compared"] = False
            else:
                entry["compared"] = True
                entry["identical"] = entry["before"] is not None and entry["before"] == entry["after"]
                identical &= entry["identical"]
            files[key] = entry
        result["routes"][route] = {
            "identical": all(f.get("identical", True) for f in files.values()),
            "files": files,
        }
    result["identical"] = identical
    out.write_text(json.dumps(result, indent=1) + "\n")
    for route, value in result["routes"].items():
        compared = [k for k, f in value["files"].items() if f["compared"]]
        print(route, "identical" if value["identical"] else "DIFFERENT", f"({len(compared)} files compared)")
    return 0 if identical else 1


if __name__ == "__main__":
    raise SystemExit(main())
