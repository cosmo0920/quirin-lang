#!/usr/bin/env python3
"""summarize.py BEFORE AFTER -- SERVE-FORWARDED-CALLBACK-FORMALS: the
measurement summary (artifacts/summary.json) of two runs named in private/sf
(measure.sh NAME): each route's report digest, the null-property rows'
digests, the binaries, the wall times, and the corpus parity log compared as a
multiset with Clang AST node addresses normalised (they differ per process)."""
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
PRIVATE = REPO / "private/sf"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
REPORT = REPO / "docs/reports/serve-forwarded-callback-formals"


def digest(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def parity(path):
    lines = pathlib.Path(path).read_text().splitlines()
    normal = sorted(re.sub(r"0x[0-9a-f]+", "0xADDR", line) for line in lines)
    return len(lines), hashlib.sha256("\n".join(normal).encode()).hexdigest()


def main():
    before, after = sys.argv[1], sys.argv[2]
    reports = {
        "corpus": lambda n: PRIVATE / n / "resource-diagnostics.json",
        "system": lambda n: PRIVATE / f"{n}-system" / "resource-diagnostics.json",
        "slice": lambda n: SLICE / f"sf-{n}" / "resource-diagnostics.json",
    }
    nulls = {
        "corpus": lambda n: PRIVATE / f"{n}-null.jsonl",
        "system": lambda n: PRIVATE / f"{n}-system-null.jsonl",
        "slice": lambda n: PRIVATE / f"{n}-slice-null.jsonl",
    }
    out = {"card": "SERVE-FORWARDED-CALLBACK-FORMALS", "before": before, "after": after}
    out["binaries"] = {
        n: (PRIVATE / "bin" / f"{n}.sha256").read_text().split()[0] for n in (before, after)
    }
    out["after_built_from"] = (PRIVATE / f"build-{after}.head").read_text().strip()
    out["corpus_run_json_binary_sha256"] = {
        n: json.loads((PRIVATE / n / "run.json").read_text())["binary_sha256"]
        for n in (before, after)
    }
    out["routes"] = {}
    for route in reports:
        b, a = digest(reports[route](before)), digest(reports[route](after))
        nb, na = digest(nulls[route](before)), digest(nulls[route](after))
        out["routes"][route] = {
            "report_before": b, "report_after": a, "report_identical": a == b,
            "null_before": nb, "null_after": na, "null_identical": na == nb,
        }
    out["seconds"] = {
        f"{n}{suffix}": (PRIVATE / f"{n}{suffix}.seconds").read_text().strip()
        for n in (before, after) for suffix in ("", "-system", "-slice")
    }
    pb, pa = parity(PRIVATE / f"{before}-parity.jsonl"), parity(PRIVATE / f"{after}-parity.jsonl")
    out["corpus_parity"] = {"before": pb, "after": pa, "equal_modulo_addresses": pb == pa}
    (REPORT / "artifacts/summary.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
