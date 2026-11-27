#!/usr/bin/env python3
"""Record the prepared attempt's revision binding without running git in it.

Usage: pin_check.py ATTEMPT_DIR EXPECTED_REVISION OUT_JSON

Reads the clone's `.git/HEAD` (and the ref it names, or `packed-refs`),
`prepare.json` and the digests of the five selected backlog-slice sources.
Exits 1 when the clone's HEAD is not EXPECTED_REVISION.
"""

import hashlib
import json
import pathlib
import sys

FILES = [
    "plugins/in_storage_backlog/sb.c",
    "src/flb_input_chunk.c",
    "lib/chunkio/src/cio_chunk.c",
    "lib/monkey/include/monkey/mk_core/mk_list.h",
    "include/fluent-bit/flb_input_chunk.h",
]


def head_of(git: pathlib.Path) -> str:
    head = (git / "HEAD").read_text().strip()
    if not head.startswith("ref: "):
        return head
    ref = head[5:]
    loose = git / ref
    if loose.exists():
        return loose.read_text().strip()
    for line in (git / "packed-refs").read_text().splitlines():
        if line.endswith(" " + ref):
            return line.split()[0]
    raise SystemExit(f"cannot resolve {ref}")


def main() -> int:
    attempt = pathlib.Path(sys.argv[1])
    expected = sys.argv[2]
    source = attempt / "fluent-bit-clang-source"
    head = head_of(source / ".git")
    prepare = attempt / "prepare.json"
    record = {
        "attempt": str(attempt),
        "expected_revision": expected,
        "clone_head": head,
        "matches": head == expected,
        "prepare_json_sha256": hashlib.sha256(prepare.read_bytes()).hexdigest(),
        "files": {
            name: hashlib.sha256((source / name).read_bytes()).hexdigest()
            for name in FILES
            if (source / name).exists()
        },
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return 0 if record["matches"] else 1


if __name__ == "__main__":
    sys.exit(main())
