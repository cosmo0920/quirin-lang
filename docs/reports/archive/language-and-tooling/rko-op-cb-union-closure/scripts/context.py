#!/usr/bin/env python3
"""context.py FILE:LINE... -- print each librdkafka source line (with the line
before and after, joined) from the audited Fluent Bit tree at 66910c10, for the
census table. A reading aid."""
import pathlib
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/"
                    "fluent-bit-clang-source/lib/librdkafka-2.15.0/src")
for site in sys.argv[1:]:
    name, line = site.rsplit(":", 1)
    lines = (ROOT / name).read_text(errors="replace").splitlines()
    n = int(line)
    text = " ".join(part.strip() for part in lines[max(0, n - 2):n + 1])
    print(f"{site}: {text}")
