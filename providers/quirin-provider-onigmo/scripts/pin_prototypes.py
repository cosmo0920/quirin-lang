#!/usr/bin/env python3
"""Pin the prototypes of the contracted Onigmo functions from the vendored
header the audited build uses.

    python3 pin_prototypes.py --clang CLANG --onigmo-dir DIR --provenance TEXT \
        --out pinned/onigmo-6.2.0.json FUNCTION...

Card ONIGMO-PROVIDER-PLUGIN. Fluent Bit vendors Onigmo at `lib/onigmo` and
compiles every unit that includes it with `-I lib/onigmo`, so the header is
`onigmo.h` there. This preprocesses one translation unit including
`<onigmo.h>` under `-I DIR`, reads the version macros
(`ONIGMO_VERSION_MAJOR`, `_MINOR`, `_TEENY`), and records each function's
declaration as the preprocessor leaves it (`ONIG_EXTERN` expands to
`extern`, which is dropped), with the header's SHA-256. Every typedef a
prototype uses is recorded with the C type it denotes, each established by
compiling `_Static_assert(__builtin_types_compatible_p(TYPEDEF, TYPE))` under
the same flags; the canonical parameter and result types are the
declaration's with those typedefs substituted. The checker embeds the
result: the header declaration is the cross-check of a contract's
signature, never the source of a version bound (Onigmo's API documentation
is).
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

HEADER = "onigmo.h"
# typedef -> the C type it denotes in onigmo.h (lines cited); each is
# verified by the compiler below, never trusted.
TYPEDEFS = {
    "OnigUChar": "unsigned char",                                  # onigmo.h:79
    "OnigPosition": "long",                                        # onigmo.h:83 ptrdiff_t, LP64
    "OnigOptionType": "unsigned int",                              # onigmo.h:448
    "OnigEncoding": "const struct OnigEncodingTypeST *",           # onigmo.h:182
    "OnigRegex": "struct re_pattern_buffer *",                     # onigmo.h:803
    "OnigRegion": "struct re_registers",                           # onigmo.h:712
    "OnigSyntaxType": "OnigSyntaxType",                            # a struct typedef, opaque here
    "OnigErrorInfo": "OnigErrorInfo",                              # onigmo.h:741-745, opaque here
}
OPAQUE = {"OnigSyntaxType", "OnigErrorInfo"}


def split_parameters(inner):
    """Split a parameter list at the commas outside parentheses."""
    pieces, depth, start = [], 0, 0
    for index, character in enumerate(inner):
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
        elif character == "," and depth == 0:
            pieces.append(inner[start:index])
            start = index + 1
    pieces.append(inner[start:])
    return [piece.strip() for piece in pieces if piece.strip()]


def strip_name(parameter):
    """A parameter's type without its name. A function pointer
    `int (*func)(...)` is `fnptr`; otherwise drop a trailing identifier when
    a type remains before it."""
    if re.search(r"\(\s*\*\s*\w*\s*\)\s*\(", parameter):
        return "fnptr"
    match = re.match(r"^(.*?[\s*])(\w+)$", parameter)
    if match and match.group(1).strip() and match.group(2) not in {
            "char", "int", "long", "short", "unsigned", "signed", "void", "const"} \
            and match.group(2) not in TYPEDEFS:
        return re.sub(r"\s+", " ", match.group(1)).strip()
    return re.sub(r"\s+", " ", parameter).strip()


def spelled(c_type):
    """A type as the header spells it (typedef names kept), with the pointer
    spelling normalised: what the gateway's ABI reader projects
    (`legacy_declared_abi.rs`), so what the checker maps."""
    if c_type == "fnptr":
        return c_type
    c_type = re.sub(r"\s*\*", " *", c_type)
    c_type = re.sub(r"\s+", " ", c_type).strip()
    return c_type.replace("* *", "**")


def canonical(c_type):
    """Substitute every typedef by the type it denotes, then normalise the
    pointer spelling (`T*` and `T *` alike)."""
    if c_type == "fnptr":
        return c_type
    for name, denoted in TYPEDEFS.items():
        if name in OPAQUE:
            continue
        c_type = re.sub(rf"\b{name}\b", denoted, c_type)
    c_type = re.sub(r"\s*\*", " *", c_type)
    c_type = re.sub(r"\*\s+\*", "**", c_type)
    c_type = re.sub(r"\s+", " ", c_type).strip()
    # `const struct X * *` -> `const struct X **`
    return c_type.replace("* *", "**")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--clang", required=True)
    parser.add_argument("--onigmo-dir", required=True, type=Path)
    parser.add_argument("--provenance", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("functions", nargs="+")
    args = parser.parse_args()
    directory = args.onigmo_dir.resolve()
    flags = ["-I", str(directory)]
    source = f"#include <{HEADER}>\n"

    def run(*extra, text=source):
        return subprocess.run([args.clang, "-E", "-x", "c", *flags, *extra, "-"], input=text,
                              capture_output=True, text=True, check=True)

    macros = run("-dM").stdout
    version = []
    for name in ("ONIGMO_VERSION_MAJOR", "ONIGMO_VERSION_MINOR", "ONIGMO_VERSION_TEENY"):
        version.append(int(re.search(rf"^#define {name} (\d+)$", macros, re.M).group(1)))
    text = "\n".join(line for line in run().stdout.splitlines() if not line.startswith("#"))
    flat = re.sub(r"\s+", " ", text)
    for name, denoted in TYPEDEFS.items():
        if name in OPAQUE:
            continue
        probe = source + f"_Static_assert(__builtin_types_compatible_p({name}, {denoted}), \"t\");\n"
        completed = subprocess.run([args.clang, "-fsyntax-only", "-x", "c", *flags, "-"],
                                   input=probe, capture_output=True, text=True)
        if completed.returncode != 0:
            raise SystemExit(f"{name} is not {denoted}: {completed.stderr}")
    prototypes = {}
    for function in args.functions:
        match = re.search(
            rf"(?:^|[;}}])\s*((?:extern\s+)?[^;{{}}]*?\b{re.escape(function)}\s*\(((?:[^()]|\((?:[^()]|\([^()]*\))*\))*)\))\s*;",
            flat)
        if not match:
            raise SystemExit(f"no declaration of {function}")
        declaration = re.sub(r"\s+", " ", match.group(1)).strip()
        declaration = re.sub(r"^extern ", "", declaration)
        result = declaration[:declaration.index(function)].strip()
        inner = match.group(2).strip()
        parameters = [] if inner in ("", "void") else [strip_name(p) for p in split_parameters(inner)]
        prototypes[function] = {
            "declaration": declaration,
            "header": HEADER,
            "result": spelled(result),
            "parameters": [spelled(p) for p in parameters],
            "canonical_result": canonical(result),
            "canonical_parameters": [canonical(p) for p in parameters],
        }
    header = directory / HEADER
    record = {
        "schema_version": 1,
        "generated_by": "providers/quirin-provider-onigmo/scripts/pin_prototypes.py",
        "version": ".".join(str(part) for part in version),
        "provenance": args.provenance,
        "headers": {HEADER: hashlib.sha256(header.read_bytes()).hexdigest()},
        "typedefs": {name: denoted for name, denoted in sorted(TYPEDEFS.items())},
        "prototypes": dict(sorted(prototypes.items())),
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
