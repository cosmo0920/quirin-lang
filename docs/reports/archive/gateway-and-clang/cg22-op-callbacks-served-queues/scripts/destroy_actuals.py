#!/usr/bin/env python3
"""destroy_actuals.py OUT.json -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: a
reading aid for the census (textual, never evidence for a clause). For every
call `rd_kafka_op_destroy(ACTUAL)` in librdkafka's sources at 66910c10, the
enclosing function and where ACTUAL's value comes from in that function: a
formal of the function, a local assigned from a named call (which one), a
member read, or something else. Prints the tally and writes OUT.json."""
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/lib/librdkafka-2.15.0/src")
CALL = re.compile(r"\brd_kafka_op_destroy\s*\(\s*([^()]*?(?:\([^()]*\))?[^()]*?)\s*\)\s*;")
FUNC = re.compile(r"^[A-Za-z_][\w \*]*?\b(\w+)\s*\(([^;{]*)\)\s*\{", re.M | re.S)


def functions(text):
    """(name, params, start, end) of each top-level definition."""
    out = []
    for match in FUNC.finditer(text):
        brace = match.end() - 1
        depth, i = 0, brace
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        out.append((match.group(1), match.group(2), brace, i))
    return out


def origin(actual, params, body):
    if re.fullmatch(r"\w+", actual):
        if re.search(r"\b%s\b" % re.escape(actual), params):
            return "formal"
        assigned = re.findall(r"\b%s\s*=\s*([A-Za-z_]\w*)\s*\(" % re.escape(actual), body)
        if assigned:
            return "local from " + ",".join(sorted(set(assigned)))
        if re.search(r"(RD_LIST_FOREACH|TAILQ_FOREACH\w*)\s*\(\s*%s\b" % re.escape(actual), body):
            return "loop variable over a container"
        return "local, other"
    if "->" in actual or "." in actual:
        return "member read"
    return "other"


def main():
    rows = []
    for path in sorted(ROOT.glob("*.[ch]")):
        text = path.read_text(errors="replace")
        for name, params, start, end in functions(text):
            body = text[start:end]
            for match in CALL.finditer(body):
                actual = " ".join(match.group(1).split())
                line = text.count("\n", 0, start + match.start()) + 1
                rows.append({"file": path.name, "line": line, "function": name,
                             "actual": actual, "origin": origin(actual, params, body)})
    tally = collections.Counter(r["origin"] for r in rows)
    pathlib.Path(sys.argv[1]).write_text(json.dumps({"rows": rows, "tally": tally}, indent=1) + "\n")
    print("call sites:", len(rows), "functions:", len({(r["file"], r["function"]) for r in rows}))
    for key, count in tally.most_common():
        print("  %4d  %s" % (count, key))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
