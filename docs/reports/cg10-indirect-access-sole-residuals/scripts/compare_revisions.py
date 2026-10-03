#!/usr/bin/env python3
"""compare_revisions.py -- card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: whether
each source file the card's contracts were read from is byte-identical at the
two audited Fluent Bit revisions (66910c10, the corpus tree; 046accf6, the
system tree). Read only."""
import hashlib
import pathlib
import sys

OLD = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
NEW = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")
FILES = [
    "lib/librdkafka-2.15.0/src/rdatomic.h",
    "lib/librdkafka-2.15.0/src/rdavg.h",
    "lib/librdkafka-2.15.0/src/rdhdrhistogram.c",
    "lib/librdkafka-2.15.0/src/rdhdrhistogram.h",
    "lib/librdkafka-2.15.0/src/tinycthread_extra.c",
    "lib/librdkafka-2.15.0/src/tinycthread_extra.h",
    "lib/cmetrics/include/cmetrics/cmt_atomic.h",
    "lib/cmetrics/src/cmt_atomic_clang.c",
    "lib/cmetrics/src/cmt_atomic_gcc.c",
    "lib/cmetrics/src/cmt_atomic_generic.c",
    "lib/cmetrics/src/cmt_atomic_msvc.c",
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else "absent"


def main() -> int:
    differ = 0
    for name in FILES + sys.argv[1:]:
        old, new = digest(OLD / name), digest(NEW / name)
        same = old == new and old != "absent"
        differ += not same
        print("%s %s %s" % ("same" if same else "DIFF", old[:12], name))
    return 1 if differ else 0


if __name__ == "__main__":
    raise SystemExit(main())
