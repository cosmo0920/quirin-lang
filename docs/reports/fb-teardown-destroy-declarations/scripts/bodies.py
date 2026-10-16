#!/usr/bin/env python3
"""bodies.py [OUT_DIR] -- FB-TEARDOWN-DESTROY-DECLARATIONS (adapted from
W47-LOOP-OWNER-INSTANCE-FLOW's scripts/bodies.py).

For each destroyer the card reads, prints per tree (the 66910c10 corpus
source, the 046accf6 system clone, the 90978ac6 export) the definition's
first line and the SHA-256 of its text from its name to its closing brace,
and whether the trees agree byte for byte. With OUT_DIR the bodies are
written there for reading. Read-only on every tree.
"""

import hashlib
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/"
                             "c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
                             "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
    "90978ac6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/"
                             "prep/fluent-bit-clang-source"),
}
FUNCTIONS = [
    ("src/flb_scheduler.c", "flb_sched_timer_destroy"),
    ("src/flb_scheduler.c", "flb_sched_request_destroy"),
    ("src/flb_scheduler.c", "flb_sched_timer_invalidate"),
    ("src/flb_scheduler.c", "flb_sched_timer_cb_disable"),
    ("src/flb_task.c", "flb_task_destroy"),
    ("src/flb_task.c", "flb_task_queue_destroy"),
    ("src/flb_task.c", "flb_task_retry_clean"),
    ("src/flb_event.c", "flb_event_chunk_destroy"),
    ("include/fluent-bit/flb_output.h", "flb_output_flush_destroy"),
    ("include/fluent-bit/flb_coro.h", "flb_coro_destroy"),
    ("src/flb_input_chunk.c", "flb_input_chunk_release_route"),
    ("src/flb_output.c", "flb_output_instance_destroy"),
    ("src/flb_output.c", "flb_output_free_properties"),
    ("src/flb_scheduler.c", "flb_sched_request_invalidate"),
    ("src/flb_engine.c", "flb_engine_destroy_tasks"),
    ("src/flb_output.c", "flb_output_exit"),
    ("src/flb_input.c", "flb_input_exit_all"),
    ("src/flb_input.c", "flb_input_instance_destroy"),
    ("src/flb_input.c", "flb_input_coro_destroy"),
    ("src/flb_input.c", "flb_input_collector_destroy"),
    ("plugins/in_tail/tail_file.c", "flb_tail_file_remove"),
    ("plugins/in_tail/tail_file.c", "flb_tail_file_remove_all"),
    ("plugins/in_tail/tail_config.c", "flb_tail_config_destroy"),
]


def body(text, name):
    match = re.search(r"^[^\n;{}]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not match:
        return None, None
    depth = 0
    for index in range(match.end() - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                line = text.count("\n", 0, match.start()) + 1
                return line, text[match.start():index + 1]
    return None, None


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else None
    if out:
        out.mkdir(parents=True, exist_ok=True)
    for path, name in FUNCTIONS:
        digests = {}
        for tree, root in TREES.items():
            try:
                source = (root / path).read_text(errors="replace")
            except OSError:
                print(f"{name} @ {tree}: {path} absent")
                continue
            line, text = body(source, name)
            if text is None:
                print(f"{name} @ {tree}: not found")
                continue
            digests[tree] = hashlib.sha256(text.encode()).hexdigest()
            print(f"{name} @ {tree}: {path}:{line} {digests[tree][:16]}")
            if out:
                (out / f"{name}.{tree}.c").write_text(text + "\n")
        groups = {}
        for tree, digest in digests.items():
            groups.setdefault(digest, []).append(tree)
        print(f"{name}: " + " | ".join("=".join(trees) for trees in groups.values()))


if __name__ == "__main__":
    main()
