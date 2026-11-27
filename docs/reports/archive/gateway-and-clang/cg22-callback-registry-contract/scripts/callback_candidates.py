#!/usr/bin/env python3
"""callback_candidates.py REPORT OUT.json -- CG22-CALLBACK-REGISTRY-CONTRACT:
for every call of a synchronous-invocation or table-dispatch member in the
corpus decoders (and the libc/librdkafka sorting and search members), the
function designators it hands the callee (lexically: the callback argument,
or every designator of the caller's local table initializer), with each
candidate's status in REPORT. A reading aid for the prediction: a candidate
that is not `Safe` has no validated summary, so the invocation stays Unknown
(rule 13u). Lexical and approximate; the AFTER run is the arbiter."""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
SOURCES = {
    "cmt_decode_msgpack.c": ROOT / "lib/cmetrics/src/cmt_decode_msgpack.c",
    "ctr_decode_msgpack.c": ROOT / "lib/ctraces/src/ctr_decode_msgpack.c",
}
ARRAY = re.compile(r"(cmt|ctr)_mpack_unpack_array\(\s*reader,\s*(\w+)")
MAP = re.compile(r"(cmt|ctr)_mpack_unpack_map\(\s*reader,\s*(\w+)")
ENTRY = re.compile(r"\{\s*\"[^\"]*\",\s*(\w+)\s*\}")
FUNCTION = re.compile(r"^(?:static\s+)?int\s+(\w+)\s*\(", re.M)


def main():
    report = json.loads(pathlib.Path(sys.argv[1], "resource-diagnostics.json").read_text())
    status = {(u["file"], f["function"]): f["status"]
              for u in report["units"] for f in u["functions"]}
    out = {}
    for unit, path in SOURCES.items():
        text = path.read_text()
        starts = [(m.start(), m.group(1)) for m in FUNCTION.finditer(text)]
        def owner(at):
            best = None
            for start, name in starts:
                if start <= at:
                    best = name
            return best
        for m in ARRAY.finditer(text):
            caller = owner(m.start())
            out.setdefault("%s::%s" % (unit, caller), []).append(
                {"kind": "array", "candidates": {m.group(2): status.get((unit, m.group(2)))}})
        for m in MAP.finditer(text):
            caller = owner(m.start())
            start = [s for s, n in starts if n == caller][0]
            table = ENTRY.findall(text[start:m.start()])
            out.setdefault("%s::%s" % (unit, caller), []).append(
                {"kind": "map", "candidates": {n: status.get((unit, n)) for n in table}})
    for key, calls in out.items():
        for call in calls:
            call["caller_status"] = status.get(tuple(key.split("::")))
            call["all_safe"] = all(s == "Safe" for s in call["candidates"].values())
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    for key, calls in sorted(out.items()):
        for call in calls:
            print("%-60s %-5s %-7s all-safe=%s %s" % (
                key, call["kind"], call["caller_status"], call["all_safe"],
                ",".join("%s:%s" % (n, (s or "-")[0]) for n, s in call["candidates"].items())))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
