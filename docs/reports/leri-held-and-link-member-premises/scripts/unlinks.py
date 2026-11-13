#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""unlinks.py OUT_JSON TREE... -- LERI-HELD-AND-LINK-MEMBER-PREMISES (copied from
LIST-ENTRY-RELEASE-IDENTITY, with mk_http_thread_destroy added; the member
each unconditional unlink names is what `resource_unpublished(P, MEMBER)` states).

For every destroyer this card declares `resource_unpublished` for, read its
body in each prepared Fluent Bit tree (read only) and record: where it is
defined, the SHA-256 of the body text, every `mk_list_del(&P->F)` of its
object parameter P, whether one of them is unconditional at the body's top
level (brace depth 1), and the release of P (`flb_free(P)`). The declaration
is faithful when the object's list link is unlinked before the release on
every path that releases it.
"""
import hashlib
import json
import re
import sys

DESTROYERS = {
    "flb_sched_timer_destroy": ("src/flb_scheduler.c", "timer"),
    "flb_sched_request_destroy": ("src/flb_scheduler.c", "req"),
    "flb_task_destroy": ("src/flb_task.c", "task"),
    "flb_task_retry_destroy": ("src/flb_task.c", "retry"),
    "flb_input_chunk_destroy": ("src/flb_input_chunk.c", "ic"),
    "flb_output_instance_destroy": ("src/flb_output.c", "ins"),
    "flb_input_instance_destroy": ("src/flb_input.c", "ins"),
    "flb_output_flush_destroy": ("include/fluent-bit/flb_output.h", "out_flush"),
    "flb_input_coro_destroy": ("src/flb_input.c", "input_coro"),
    "flb_tail_file_remove": ("plugins/in_tail/tail_file.c", "file"),
    "mk_http_thread_destroy": ("lib/monkey/mk_server/mk_http_thread.c", "mth"),
}


def body(text, name):
    m = re.search(r"^[^\n;]*\b" + re.escape(name) + r"\s*\([^;{]*\)\s*\{", text, re.M)
    if not m:
        return None
    start = m.start()
    depth = 0
    for i in range(m.end() - 1, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    return None


def top_level(text, index):
    depth = 0
    for ch in text[:index]:
        depth += ch == "{"
        depth -= ch == "}"
    return depth == 1


def main():
    out, trees = sys.argv[1], sys.argv[2:]
    rows = []
    for name, (path, param) in DESTROYERS.items():
        for tree in trees:
            try:
                text = open(f"{tree}/{path}", encoding="utf-8", errors="replace").read()
            except OSError:
                rows.append({"function": name, "tree": tree, "defined": False})
                continue
            b = body(text, name)
            if b is None:
                rows.append({"function": name, "tree": tree, "defined": False})
                continue
            dels = [m for m in re.finditer(r"mk_list_del\s*\(\s*&\s*" + re.escape(param) + r"\s*->\s*(\w+)\s*\)", b)]
            frees = [m.start() for m in re.finditer(r"flb_free\s*\(\s*" + re.escape(param) + r"\s*\)", b)]
            rows.append({
                "function": name, "tree": tree, "path": path, "defined": True,
                "body_sha256": hashlib.sha256(b.encode()).hexdigest(),
                "unlinks": [{"field": m.group(1), "top_level": top_level(b, m.start()),
                             "before_free": bool(frees) and m.start() < min(frees)} for m in dels],
                "frees": len(frees),
            })
    json.dump(rows, open(out, "w"), indent=1)
    for row in rows:
        if row["defined"]:
            print(row["function"], row["tree"].rsplit("/", 2)[-2], row["body_sha256"][:12],
                  [(u["field"], u["top_level"], u["before_free"]) for u in row["unlinks"]], row["frees"])
        else:
            print(row["function"], row["tree"], "NOT DEFINED")


if __name__ == "__main__":
    main()
