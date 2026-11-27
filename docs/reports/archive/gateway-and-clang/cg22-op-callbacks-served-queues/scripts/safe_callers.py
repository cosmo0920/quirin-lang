#!/usr/bin/env python3
"""safe_callers.py RUN OUT.json -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: which
`Safe` (and `Pending`) rows of one corpus run could move when the proposal
declares its members. (1) Every such row whose function body, read from the
audited tree, calls a member the proposal would declare (textual: a reading
aid; a hit is then read by hand). (2) From the run's contract-dependency
sidecar, the `Safe` rows resting on `rd_kafka_q_wait_result` or
`rd_kafka_q_purge0` (the served-queue premise). Writes OUT.json and prints
both."""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
DECLARED = ["rd_kafka_q_serve0", "rd_kafka_q_serve", "rd_kafka_q_serve_maybe_consume",
            "rd_kafka_q_pop_serve0", "rd_kafka_q_pop_serve", "rd_kafka_q_pop_serve_maybe_consume",
            "rd_kafka_op_call", "rd_kafka_op_destroy"]
PREMISED = ["rd_kafka_q_wait_result", "rd_kafka_q_purge0"]
CALL = re.compile(r"\b(%s)\s*\(" % "|".join(DECLARED))


INDEX = {}


def paths(base):
    """Every file under the audited tree's lib/, src/ and plugins/ named BASE."""
    if not INDEX:
        for top in ("lib", "src", "plugins"):
            for found in (ROOT / top).rglob("*.c"):
                INDEX.setdefault(found.name, []).append(found)
    return INDEX.get(pathlib.Path(base).name, [])


def body(path, name):
    for candidate in paths(path):
        found = body_in(candidate.read_text(errors="replace"), name)
        if found is not None:
            return found
    return None


def body_in(text, name):
    head = re.compile(r"\b%s\s*\([^;{]*\)\s*\{" % re.escape(name), re.S)
    match = head.search(text)
    if not match:
        return None
    depth, i = 0, match.end() - 1
    while i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[match.end():i]
        i += 1
    return None


def main():
    run = pathlib.Path(sys.argv[1])
    report = json.loads((run / "resource-diagnostics.json").read_text())
    hits = []
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] not in ("Safe", "Pending"):
                continue
            text = body(unit["file"], function["function"])
            if text is None:
                continue
            called = sorted(set(m.group(1) for m in CALL.finditer(text)))
            if called:
                hits.append({"unit": unit["file"], "function": function["function"],
                             "status": function["status"], "calls": called})
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    declarations = sidecar["contract_dependencies"]["declarations"]
    premised = {name: declarations.get(name, {}).get("safe_rows") for name in PREMISED}
    pathlib.Path(sys.argv[2]).write_text(json.dumps({"safe_callers": hits, "premised": premised},
                                                    indent=1) + "\n")
    print("Safe/Pending rows calling a proposed member:", len(hits))
    for hit in hits:
        print("  %s::%s %s %s" % (hit["unit"], hit["function"], hit["status"], hit["calls"]))
    for name, rows in premised.items():
        print("%s safe rows: %s" % (name, rows))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
