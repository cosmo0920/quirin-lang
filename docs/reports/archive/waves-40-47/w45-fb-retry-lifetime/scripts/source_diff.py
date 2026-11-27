#!/usr/bin/env python3
"""Compare the slice's source files between two prepared private clones.

usage: source_diff.py OLD_SOURCE_ROOT NEW_SOURCE_ROOT [--out FILE]

For each file: SHA-256 at both roots, changed-line counts, and, for the five
singleplex functions and their helpers, whether the function body text is
identical (compared with leading/trailing whitespace of each line kept).
Reads the two trees only.
"""
import argparse
import difflib
import hashlib
import json
import pathlib
import re
import sys

FILES = [
    "src/flb_output.c",
    "src/flb_scheduler.c",
    "src/flb_engine.c",
    "src/flb_engine_dispatch.c",
    "src/flb_task.c",
    "include/fluent-bit/flb_task.h",
    "include/fluent-bit/flb_output.h",
    "src/flb_output_thread.c",
    "src/flb_input_chunk.c",
    "src/flb_storage.c",
    "plugins/in_storage_backlog/sb.c",
    "lib/monkey/include/monkey/mk_core/mk_list.h",
]

FUNCTIONS = {
    "src/flb_output.c": [
        "flb_output_task_queue_enqueue",
        "flb_output_task_queue_flush_one",
        "flb_output_task_singleplex_enqueue",
        "flb_output_task_singleplex_flush_next",
        "flb_output_task_flush",
        "flb_output_task_flush_error",
    ],
    "include/fluent-bit/flb_task.h": [
        "flb_task_users_release",
        "flb_task_users_inc",
        "flb_task_users_dec",
    ],
    "src/flb_task.c": ["flb_task_retry_destroy"],
    "src/flb_output_thread.c": ["flb_output_thread_pool_flush"],
    "lib/monkey/include/monkey/mk_core/mk_list.h": ["mk_list_add", "mk_list_del", "__mk_list_del"],
}


def body(text, name):
    match = re.search(r"^[^\n;{}]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not match:
        return None
    depth = 0
    for index in range(match.end() - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                line = text.count("\n", 0, match.start()) + 1
                return line, text[match.start():index + 1]
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("old")
    parser.add_argument("new")
    parser.add_argument("--out")
    args = parser.parse_args()
    result = {"old": args.old, "new": args.new, "files": {}}
    for name in FILES:
        old = pathlib.Path(args.old, name).read_text(errors="replace")
        new = pathlib.Path(args.new, name).read_text(errors="replace")
        diff = [line for line in difflib.unified_diff(old.splitlines(), new.splitlines(), lineterm="", n=0)
                if line[:1] in "+-" and not line.startswith(("+++", "---"))]
        record = {
            "old_sha256": hashlib.sha256(old.encode()).hexdigest(),
            "new_sha256": hashlib.sha256(new.encode()).hexdigest(),
            "added_lines": sum(1 for line in diff if line.startswith("+")),
            "removed_lines": sum(1 for line in diff if line.startswith("-")),
        }
        functions = {}
        for function in FUNCTIONS.get(name, []):
            before = body(old, function)
            after = body(new, function)
            functions[function] = {
                "old_line": before[0] if before else None,
                "new_line": after[0] if after else None,
                "body_identical": bool(before and after and before[1] == after[1]),
                "present": [before is not None, after is not None],
            }
        if functions:
            record["functions"] = functions
        result["files"][name] = record
    text = json.dumps(result, indent=1) + "\n"
    if args.out:
        with open(args.out, "x") as handle:
            handle.write(text)
    sys.stdout.write(text)


if __name__ == "__main__":
    main()
