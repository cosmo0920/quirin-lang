#!/usr/bin/env python3
"""CG21-CALL-EXITS-AND-BOUNDARIES: census of the leaves each exit or boundary
class blocks, from one run's resource-diagnostics.json.

usage: census.py RUN [--json OUT]

Classes (by leaf description):
  noreturn-declared   uncontracted call to __assert_fail, abort or exit (the
                      three callees this card declares)
  noreturn-other      uncontracted call to another C/POSIX no-return callee
  returns-twice       setjmp family; non-local: longjmp family, pthread_exit
  assembly            the token reader's misnamed asm leaves (volatile,
                      __asm__, asm, goto as a callee) and the card's named ones
  extension-node      __builtin_choose_expr & co. as callees, and named ones
  builtin-call        every other uncontracted __builtin_* callee
  cleanup / seh       the card's named boundaries (AFTER only)
For each class: leaves, functions touched, and Unknown functions whose every
unsupported leaf is in the class ("sole", an upper bound on what lifting the
class alone could move).
"""

import argparse
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from compare import load  # noqa: E402

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")
DECLARED = {"__assert_fail", "abort", "exit"}
NORETURN_OTHER = {"_exit", "_Exit", "quick_exit", "__assert_perror_fail", "__assert",
                  "__builtin_unreachable", "__builtin_trap", "err", "errx", "verr", "verrx",
                  "__stack_chk_fail", "__chk_fail", "__fortify_fail"}
RETURNS_TWICE = {"setjmp", "_setjmp", "__setjmp", "sigsetjmp", "__sigsetjmp", "savectx",
                 "vfork", "getcontext"}
NONLOCAL = {"longjmp", "_longjmp", "siglongjmp", "__longjmp_chk", "setcontext",
            "swapcontext", "pthread_exit", "thrd_exit"}
ASSEMBLY_WORDS = {"volatile", "__asm__", "asm", "__asm", "goto", "inline", "__volatile__"}
EXTENSION = {"__builtin_choose_expr", "__builtin_bit_cast", "__builtin_offsetof",
             "__builtin_types_compatible_p", "__builtin_va_arg", "__builtin_convertvector",
             "__builtin_shufflevector", "__builtin_FILE", "__builtin_FUNCTION",
             "__builtin_LINE", "__builtin_COLUMN"}


def classify(description):
    match = UNCONTRACTED.match(description or "")
    if match:
        callee = match.group(1)
        if callee in DECLARED:
            return "noreturn-declared:" + callee
        if callee in NORETURN_OTHER:
            return "noreturn-other:" + callee
        if callee in RETURNS_TWICE:
            return "returns-twice:" + callee
        if callee in NONLOCAL:
            return "nonlocal:" + callee
        if callee in ASSEMBLY_WORDS:
            return "assembly:" + callee
        if callee in EXTENSION:
            return "extension-node:" + callee
        if callee.startswith("__builtin_"):
            return "builtin-call:" + callee
        return None
    text = description or ""
    if text.startswith("no-return contract of"):
        return "noreturn-unestablished"
    if "is a returns-twice call" in text:
        return "returns-twice:named"
    if "is a non-local exit" in text:
        return "nonlocal:named"
    if "is the Clang extension node" in text:
        return "extension-node:named"
    if "assembly" in text or text.startswith("`asm goto`"):
        return "assembly:named"
    if "cleanup" in text and "attribute" in text:
        return "cleanup:named"
    if "structured exception" in text:
        return "seh:named"
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    _, functions = load(args.run)
    leaves = collections.Counter()
    touched = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    family_sole = collections.defaultdict(set)
    for key, (status, counter) in functions.items():
        unsupported = [(d, n) for (c, d), n in counter.items() if c == "unsupported"]
        classes = {}
        for description, n in unsupported:
            cls = classify(description)
            classes[description] = cls
            if cls:
                leaves[cls] += n
                touched[cls].add(key)
        if status != "Unknown" or not unsupported:
            continue
        found = {classes[d] for d, _ in unsupported}
        if None in found:
            continue
        if len(found) == 1:
            sole[next(iter(found))].add(key)
        families = {cls.split(":")[0] for cls in found}
        if len(families) == 1:
            family_sole[next(iter(families))].add(key)
    record = {
        "classes": {
            cls: {
                "leaves": leaves[cls],
                "functions": len(touched[cls]),
                "unknown_sole": len(sole[cls]),
            }
            for cls in sorted(leaves)
        },
        "family_sole": {family: sorted("::".join(k) for k in keys)
                        for family, keys in sorted(family_sole.items())},
        "sole": {cls: sorted("::".join(k) for k in keys) for cls, keys in sorted(sole.items())},
    }
    for cls, row in record["classes"].items():
        print(f"{cls:40} leaves {row['leaves']:6} functions {row['functions']:5} "
              f"unknown-sole {row['unknown_sole']:4}")
    for family, keys in record["family_sole"].items():
        print(f"family {family}: {len(keys)} Unknown functions blocked only by it")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
