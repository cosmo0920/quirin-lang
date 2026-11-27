#!/usr/bin/env python3
"""Census of Fluent Bit header wrappers around declared libc functions
(card CONTRACT-INHERIT-ADOPTION).

Usage: wrapper_census.py REPOSITORY FLUENT_BIT_SOURCE OUT.json

Reads every `static inline` (or `inline static`) function named `flb_*`
defined in `FLUENT_BIT_SOURCE/include/fluent-bit/**/*.h` and records the
callees its body names. A wrapper is a *candidate* when its body calls at
least one function that `quirin-provider-libc` declares. Each candidate is
then classified by reading its body shape:

* `exact-forward`: the body is a single `return CALLEE(ARGS);` (or
  `CALLEE(ARGS);` for a `void` wrapper) whose arguments are exactly the
  wrapper's parameters in order, and CALLEE is a declared libc function;
* `single-callee`: the body calls exactly one function, a declared libc one, but
  is not a bare forward (a guard such as `size == 0 -> NULL`, a loop, a
  changed argument or result);
* `composite`: the body does more than forward (several calls, loops,
  arithmetic on the result).

The classification is a textual reading and is re-read by hand in the report;
it runs nothing and decides nothing by itself.
"""
import json
import re
import sys
from pathlib import Path

FN = re.compile(
    r"^(?:static\s+inline|static\s+__inline__|static\s+__inline|inline\s+static|static\s+FLB_INLINE)\b"
    r"[^;{}]*?\b(flb_\w+)\s*\(([^)]*)\)\s*\{",
    re.M,
)
# `FLB_HAVE_TESTS_OSSFUZZ` is not defined by the pinned build's generated
# configuration, so its (non-nested) arms are not part of the audited bodies.
OSSFUZZ = re.compile(r"#\s*ifdef\s+FLB_HAVE_TESTS_OSSFUZZ\b.*?#\s*endif", re.S)
KEYWORDS = {"if", "while", "for", "return", "sizeof", "switch", "defined"}


def libc_declared(repository):
    names = set()
    for path in (repository / "providers/quirin-provider-libc/contracts/libc").glob("*"):
        if path.suffix not in (".qcore", ".qv"):
            continue
        text = "\n".join(line.split("//", 1)[0] for line in path.read_text().split("\n"))
        names |= set(re.findall(r"extern\s+fn\s+(\w+)", text))
    return names


def strip_c_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in text.split("\n"))


def parameter_names(params):
    names = []
    for item in params.split(","):
        item = item.strip()
        if not item or item == "void":
            continue
        match = re.search(r"([A-Za-z_]\w*)\s*(?:\[\s*\])?$", item)
        names.append(match.group(1) if match else item)
    return names


def classify(body, params, libc):
    statements = [s.strip() for s in body.strip().split(";") if s.strip()]
    calls = re.findall(r"\b([A-Za-z_]\w*)\s*\(", body)
    callees = [c for c in calls if c not in KEYWORDS]
    declared = sorted({c for c in callees if c in libc})
    if len(statements) == 1 and "#" not in body:
        match = re.fullmatch(r"(?:return\s+)?(?:\([^()]*\)\s*)?(\w+)\s*\(([^()]*)\)", statements[0])
        if match and match.group(1) in libc:
            args = [a.strip() for a in match.group(2).split(",") if a.strip()]
            if args == params:
                return "exact-forward", declared
    if len(set(callees)) == 1 and declared:
        return "single-callee", declared
    return "composite", declared


def main():
    repository = Path(sys.argv[1]).resolve()
    source = Path(sys.argv[2]).resolve()
    out = Path(sys.argv[3])
    libc = libc_declared(repository)
    rows = []
    headers = sorted((source / "include/fluent-bit").rglob("*.h"))
    inline_functions = 0
    for header in headers:
        text = OSSFUZZ.sub("", strip_c_comments(header.read_text(errors="replace")))
        for match in FN.finditer(text):
            inline_functions += 1
            index, depth = match.end(), 1
            while depth and index < len(text):
                depth += {"{": 1, "}": -1}.get(text[index], 0)
                index += 1
            body = text[match.end():index - 1]
            params = parameter_names(match.group(2))
            shape, declared = classify(body, params, libc)
            if not declared:
                continue
            line = text[:match.start()].count("\n") + 1
            rows.append({
                "header": header.relative_to(source).as_posix(),
                "line_in_comment_stripped_text": line,
                "wrapper": match.group(1),
                "parameters": params,
                "declared_libc_callees": declared,
                "all_callees": sorted({c for c in re.findall(r"\b([A-Za-z_]\w*)\s*\(", body)} - KEYWORDS),
                "shape": shape,
            })
    result = {
        "libc_declared_names": len(libc),
        "headers": len(headers),
        "inline_flb_functions": inline_functions,
        "candidates": len(rows),
        "by_shape": {s: sum(r["shape"] == s for r in rows)
                     for s in ("exact-forward", "single-callee", "composite")},
        "rows": rows,
    }
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "rows"}))
    for row in rows:
        print(f"  {row['shape']:15} {row['wrapper']} ({row['header']}) -> {row['declared_libc_callees']} "
              f"of {row['all_callees']}")


if __name__ == "__main__":
    main()
