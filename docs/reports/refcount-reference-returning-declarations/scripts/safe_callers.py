#!/usr/bin/env python3
"""safe_callers.py REPORT CALLEE... -- card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS,
task 1: the `Safe` rows of REPORT whose own source text (in the pinned
66910c10 tree, read lexically from the definition's line to its closing
brace at column 0) spells a call to one of CALLEE (macro spellings included
by the caller), i.e. the rows a re-declaration of a callee could move from
Safe. Lexical and approximate: AFTER is the arbiter."""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
SOURCES = {}


def lines_of(name):
    if name not in SOURCES:
        found = sorted(ROOT.glob("**/" + name))
        SOURCES[name] = found[0].read_text(errors="replace").splitlines() if found else []
    return SOURCES[name]


def body(lines, function):
    pattern = re.compile(r"^[^\s#/].*\b" + re.escape(function) + r"\s*\(")
    for index, line in enumerate(lines):
        if pattern.search(line) and not line.rstrip().endswith(";"):
            for end in range(index, len(lines)):
                if lines[end].startswith("}"):
                    return "\n".join(lines[index:end + 1])
    return ""


def main():
    report = json.load(open(sys.argv[1]))
    callees = sys.argv[2:]
    call = re.compile(r"\b(" + "|".join(map(re.escape, callees)) + r")\s*\(")
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] != "Safe":
                continue
            text = body(lines_of(unit["file"]), function["function"])
            names = sorted(set(call.findall(text)))
            if names:
                print("%s:%s %s" % (unit["file"], function["function"], " ".join(names)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
