#!/usr/bin/env python3
"""Pin the prototypes of the contracted libcurl functions from the headers
the audited build uses.

    python3 pin_prototypes.py --clang CLANG [--include-dir DIR] \
        --out pinned/libcurl-8.18.0.json FUNCTION...

Card LIBCURL-PROVIDER-PLUGIN. Preprocesses one translation unit including
`<curl/curl.h>` (it includes `curlver.h`, `easy.h`, `urlapi.h` and the rest),
with `-isystem DIR` when the build passes libcurl an include directory, reads
the version macros, and records each function's declaration as the
preprocessor leaves it (`CURL_EXTERN` expands to a visibility attribute, which
is dropped), the header the line markers attribute it to, and every libcurl
header's SHA-256. The enumeration types a prototype uses are recorded with
the integer type Clang gives them (contract specification C02, rule 12u),
established by compiling static assertions on their size and signedness
under the same flags. The checker embeds the result: a header declaration is
the cross-check of a contract's signature, never the source of its `since`
(the official manual pages are).
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

HEADERS = ["curl/curl.h"]
ENUMS = ["CURLcode"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--clang", required=True)
    parser.add_argument("--include-dir", action="append", default=[])
    parser.add_argument("--provenance", required=True,
                        help="where the headers come from (the package, as installed)")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("functions", nargs="+")
    args = parser.parse_args()
    flags = [flag for directory in args.include_dir
             for flag in ("-isystem", str(Path(directory).resolve()))]
    source = "".join(f"#include <{h}>\n" for h in HEADERS)

    def run(*extra, text=source):
        return subprocess.run([args.clang, "-E", "-x", "c", *flags, *extra, "-"], input=text,
                              capture_output=True, text=True, check=True)

    preprocessed = run().stdout
    macros = run("-dM").stdout
    version = {}
    for name in ("LIBCURL_VERSION_MAJOR", "LIBCURL_VERSION_MINOR", "LIBCURL_VERSION_PATCH"):
        version[name] = int(re.search(rf"^#define {name} (\d+)$", macros, re.M).group(1))
    # Attribute every preprocessed line to its header through the line markers.
    current, lines = None, []
    headers = set()
    for line in preprocessed.splitlines():
        marker = re.match(r'^# \d+ "([^"]+)"', line)
        if marker:
            current = marker.group(1)
            if current.startswith("/") and "/curl/" in current:
                headers.add(current)
            continue
        lines.append((current, line))
    text = ""
    owner = []
    for header, line in lines:
        owner.append((len(text), header))
        text += line + "\n"
    flat = re.sub(r"__attribute__\s*\(\((?:[^()]|\((?:[^()]|\([^()]*\))*\))*\)\)", "", text)
    flat = re.sub(r"\s+", " ", flat)
    prototypes = {}
    for function in args.functions:
        match = re.search(rf"(?:^|[;}}])\s*((?:extern\s+)?[^;{{}}]*?\b{re.escape(function)}\s*\([^;{{}}]*\))\s*;",
                          flat)
        if not match:
            raise SystemExit(f"no declaration of {function}")
        declaration = re.sub(r"\s+", " ", match.group(1)).strip()
        declaration = re.sub(r"^extern ", "", declaration)
        declaration = re.sub(r"\s*\(\s*", "(", declaration, count=1)
        index = text.find(function + "(")
        header = None
        for offset, name in owner:
            if offset > index:
                break
            header = name
        prototypes[function] = {"declaration": declaration, "header": header}
    enums = {}
    for name in ENUMS:
        for spelling, assertion in (("unsigned int", f"(({name})-1) > 0"),
                                    ("int", f"(({name})-1) < 0")):
            probe = (source + f"_Static_assert(sizeof({name}) == sizeof(int), \"size\");\n"
                     f"_Static_assert({assertion}, \"sign\");\n")
            completed = subprocess.run([args.clang, "-fsyntax-only", "-x", "c", *flags, "-"],
                                       input=probe, capture_output=True, text=True)
            if completed.returncode == 0:
                enums[name] = spelling
                break
        else:
            raise SystemExit(f"cannot establish the integer type of {name}")
    record = {
        "schema_version": 1,
        "generated_by": "providers/quirin-provider-libcurl/scripts/pin_prototypes.py",
        "version": f"{version['LIBCURL_VERSION_MAJOR']}.{version['LIBCURL_VERSION_MINOR']}."
                   f"{version['LIBCURL_VERSION_PATCH']}",
        "provenance": args.provenance,
        "headers": {"curl/" + Path(path).name: hashlib.sha256(Path(path).read_bytes()).hexdigest()
                    for path in sorted(headers)},
        "enums": enums,
        "prototypes": {function: {"declaration": entry["declaration"],
                                  "header": "curl/" + Path(entry["header"]).name}
                       for function, entry in sorted(prototypes.items())},
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
