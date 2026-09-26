#!/usr/bin/env python3
"""Bind the card's measurements to their inputs: the private clone's HEAD,
the digests of the slice files the obligations read, the audit binaries and
the contract modules. usage: bind.py OUT_JSON (must not exist)."""
import hashlib
import json
import pathlib
import sys

PRIVATE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-retry-lifetime")
ROOT = pathlib.Path(__file__).resolve().parents[4]
SOURCE_FILES = [
    "src/flb_output.c",
    "src/flb_output_thread.c",
    "src/flb_task.c",
    "include/fluent-bit/flb_task.h",
    "include/fluent-bit/flb_output.h",
    "lib/monkey/include/monkey/mk_core/mk_list.h",
]
MODULES = [
    "providers/quirin-provider-fluent-bit/contracts/fluent_bit/task_users.qcore",
    "providers/quirin-provider-fluent-bit/contracts/fluent_bit/output_singleplex.qcore",
    "providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qcore",
]


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def main():
    out = pathlib.Path(sys.argv[1])
    record = {}
    for attempt in ("attempt-1", "attempt-2"):
        source = PRIVATE / attempt / "fluent-bit-clang-source"
        record[attempt] = {
            "clone_head": (source / ".git/HEAD").read_text().strip(),
            "prepare_json_sha256": sha(PRIVATE / attempt / "prepare.json"),
            "compile_commands_sha256": sha(PRIVATE / attempt / "fluent-bit-clang-production/compile_commands.json"),
            "files": {name: sha(source / name) for name in SOURCE_FILES},
        }
    record["binaries"] = {
        name: sha(ROOT / "private/w45-fb-retry-lifetime/bin" / name)
        for name in ("quirin-audit-base", "quirin-audit-e1259190")
    }
    record["contract_modules"] = {name: sha(ROOT / name) for name in MODULES}
    with open(out, "x") as handle:
        handle.write(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
