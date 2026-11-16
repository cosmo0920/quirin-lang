#!/usr/bin/env python3
"""Pin the prototypes of the contracted OpenSSL functions from the headers
the audited build uses.

    python3 pin_prototypes.py --clang CLANG --out pinned/openssl-3.5.5.json FUNCTION...

Preprocesses one translation unit that includes every OpenSSL header the
functions are declared in (no include flag: the prepared Fluent Bit build
passes OpenSSL none, so Clang's default search path is the build's), reads
the version macros, and for each function its declaration as the
preprocessor leaves it (OpenSSL 3 declares `X509_free` only through
`DECLARE_ASN1_FUNCTIONS`, so the header text alone does not show it), with
the header the line markers attribute it to and every header's SHA-256. The
checker embeds the result: a header declaration is the cross-check of a
contract's signature, never the source of its `since` (user decision
2026-11-16).
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

HEADERS = ["openssl/opensslv.h", "openssl/err.h", "openssl/ssl.h", "openssl/x509.h",
           "openssl/evp.h", "openssl/bio.h", "openssl/engine.h", "openssl/crypto.h"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--clang", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("functions", nargs="+")
    args = parser.parse_args()
    source = "".join(f"#include <{h}>\n" for h in HEADERS)
    run = lambda *extra: subprocess.run([args.clang, "-E", "-x", "c", *extra, "-"], input=source,
                                        capture_output=True, text=True, check=True)
    preprocessed = run().stdout
    macros = run("-dM").stdout
    version = {}
    for name in ("OPENSSL_VERSION_MAJOR", "OPENSSL_VERSION_MINOR", "OPENSSL_VERSION_PATCH"):
        version[name] = int(re.search(rf"^#define {name} (\d+)$", macros, re.M).group(1))
    # Attribute every preprocessed line to its header through the line markers.
    current, lines = None, []
    headers = set()
    for line in preprocessed.splitlines():
        marker = re.match(r'^# \d+ "([^"]+)"', line)
        if marker:
            current = marker.group(1)
            if current.startswith("/") and "/openssl/" in current:
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
        # The header: the last marker before the name in the unflattened text.
        index = text.find(function + "(")
        header = None
        for offset, name in owner:
            if offset > index:
                break
            header = name
        prototypes[function] = {"declaration": declaration, "header": header}
    record = {
        "schema_version": 1,
        "generated_by": "providers/quirin-provider-openssl/scripts/pin_prototypes.py",
        "version": f"{version['OPENSSL_VERSION_MAJOR']}.{version['OPENSSL_VERSION_MINOR']}."
                   f"{version['OPENSSL_VERSION_PATCH']}",
        "headers": {path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
                    for path in sorted(headers)},
        "prototypes": dict(sorted(prototypes.items())),
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
