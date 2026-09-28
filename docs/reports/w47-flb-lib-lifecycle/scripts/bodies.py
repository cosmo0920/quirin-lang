#!/usr/bin/env python3
"""bodies.py ROOT_A ROOT_B -- compare the bodies the lifecycle contract and
the census rest on (card W47-FLB-LIB-LIFECYCLE) between two Fluent Bit
source roots: flb_create, do_start, flb_start, flb_stop, flb_lib_worker
(src/flb_lib.c), mk_utils_worker_spawn (lib/monkey/mk_core/mk_utils.c) and
the three shutdown joiners. Prints, per function, whether the brace-matched
definition text is byte-identical and each side's first line. Read-only.
"""
import re
import sys

FUNCTIONS = [
    ("src/flb_lib.c", "flb_create"),
    ("src/flb_lib.c", "flb_lib_worker"),
    ("src/flb_lib.c", "do_start"),
    ("src/flb_lib.c", "flb_start"),
    ("src/flb_lib.c", "flb_stop"),
    ("src/flb_lib.c", "flb_destroy"),
    ("src/flb_lib.c", "flb_start_trace"),
    ("lib/monkey/mk_core/mk_utils.c", "mk_utils_worker_spawn"),
    ("src/flb_input_thread.c", "flb_input_thread_instance_exit"),
    ("src/flb_output_thread.c", "flb_output_thread_pool_destroy"),
    ("src/flb_downstream_worker.c", "flb_downstream_worker_runtime_stop"),
]


def body(root, path, name):
    try:
        text = open(f"{root}/{path}", encoding="utf-8", errors="replace").read()
    except FileNotFoundError:
        return None, None
    match = re.search(r"^[^\n;{}]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not match:
        return None, None
    start = match.start()
    depth = 0
    for index in range(match.end() - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[start:index + 1], text.count("\n", 0, start) + 1
    return None, None


def main():
    a, b = sys.argv[1:3]
    for path, name in FUNCTIONS:
        x, lx = body(a, path, name)
        y, ly = body(b, path, name)
        state = "absent" if x is None or y is None else ("identical" if x == y else "differs")
        print(f"{name:40} {state:9} {path}:{lx} | {path}:{ly}")


if __name__ == "__main__":
    main()
