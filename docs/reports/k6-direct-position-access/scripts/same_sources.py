#!/usr/bin/env python3
"""Card K6: whether the eight-home corpus tree (Fluent Bit 66910c10) holds the same
librdkafka sources as the user's current tree (046accf6) for the listed files.

usage: same_sources.py FILE... (paths relative to lib/librdkafka-2.15.0/src)
Read-only on both prepared trees.
"""
import hashlib
import pathlib
import sys

CYCLE = pathlib.Path("/media/Data/GitHub/quirin-lang/build/cycle19/clang")
CORPUS = CYCLE / "66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-source/lib/librdkafka-2.15.0/src"
CURRENT = (CYCLE / "046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/"
           "fluent-bit-clang-source/lib/librdkafka-2.15.0/src")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


for name in sys.argv[1:]:
    a, b = digest(CORPUS / name), digest(CURRENT / name)
    print("same" if a and a == b else "DIFF", name, (a or "absent")[:12], (b or "absent")[:12])
