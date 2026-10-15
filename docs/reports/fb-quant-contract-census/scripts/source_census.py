#!/usr/bin/env python3
"""FB-QUANT-CONTRACT-CENSUS: read-only source census of the five patterns.

Reads two prepared Fluent Bit exports (never the user's checkout, never git):
the system route's 046accf6 export and the tail-pause slice's 90978ac6 export.
For each pattern it records file:line evidence by regular expression over the
text, restricted to the route's selected files where the pattern is sized per
route, and whole-tree counts where the brief asks for the tree (plugin
registrations, FLB_OUTPUT_RETURN sites). Output: artifacts/source-census.json.

This is source reading, not an audit verdict: a match is a site, not a finding.
"""
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
SYSTEM = "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"
SLICE = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source"
SYSTEM_FILES = [
    "plugins/in_storage_backlog/sb.c", "src/flb_input_chunk.c", "src/flb_storage.c",
    "src/flb_task.c", "src/flb_engine.c", "src/flb_engine_dispatch.c",
    "src/flb_scheduler.c", "src/flb_output.c",
]
SLICE_FILES = [
    "src/flb_input.c", "src/flb_input_thread.c", "src/flb_lib.c",
    "plugins/in_tail/tail.c", "plugins/in_tail/tail_config.c", "plugins/in_tail/tail_db.c",
    "plugins/in_tail/tail_dockermode.c", "plugins/in_tail/tail_file.c",
    "plugins/in_tail/tail_file_budget.c", "plugins/in_tail/tail_fs_inotify.c",
    "plugins/in_tail/tail_fs_stat.c", "plugins/in_tail/tail_multiline.c",
    "plugins/in_tail/tail_scan.c",
]

PATTERNS = {
    # P1: a plugin callback member invoked through an instance (`ins->p->cb_X(`,
    # `p->cb_X(` after `p = ins->p`), and tested for presence.
    "P1-invoke": re.compile(r"\b(?:\w+->)*p->cb_[a-z_]+\s*\("),
    "P1-test": re.compile(r"\bp->cb_[a-z_]+\b(?!\s*\()"),
    # P2: return-code-dependent ownership.
    "P2-output-return": re.compile(r"\bFLB_OUTPUT_RETURN\s*\("),
    "P2-task-ret": re.compile(r"\bFLB_TASK_RET\s*\(|\bret\s*[!=]=\s*FLB_(?:OK|RETRY|ERROR)\b"),
    "P2-filter-ret": re.compile(r"\bFLB_FILTER_(?:MODIFIED|NOTOUCH)\b"),
    # P3: post-call member states in destroy and teardown paths.
    "P3-destroy-call": re.compile(
        r"\b(flb_input_chunk_destroy|flb_task_destroy|flb_task_retry_destroy|flb_input_instance_destroy|"
        r"flb_output_instance_destroy|flb_upstream_conn_release|flb_upstream_destroy|flb_event_chunk_destroy|"
        r"flb_sched_request_destroy|flb_sched_timer_destroy|flb_output_flush_destroy|flb_input_coro_destroy|"
        r"flb_coro_destroy|flb_input_collector_destroy|flb_input_collectors_signal_exit)\s*\("),
    # P4: fresh constructors with empty callback members (zeroed allocation of a
    # struct that carries a callback member).
    "P4-calloc": re.compile(r"\bflb_calloc\s*\(\s*1\s*,\s*sizeof\s*\(\s*struct\s+(\w+)\s*\)"),
    "P4-cb-store": re.compile(r"->(?:cb_[a-z_]+|handler|cb)\s*=\s*(?!=)"),
    # P5: event-loop registrations.
    "P5-collector": re.compile(r"\bflb_input_set_collector_(?:time|event|socket|server)\s*\("),
    "P5-mk-event": re.compile(
        r"\b(mk_event_add|mk_event_del|mk_event_timeout_create|mk_event_timeout_destroy|"
        r"mk_event_channel_create|mk_event_channel_destroy|mk_event_inject)\s*\("),
}


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def strip_comment(line):
    s = line.strip()
    return s.startswith("*") or s.startswith("//") or s.startswith("/*")


def scan(root, files):
    sites = {k: [] for k in PATTERNS}
    digests = {}
    for rel in files:
        path = os.path.join(root, rel)
        digests[rel] = sha256(path)
        with open(path, encoding="utf-8", errors="replace") as f:
            for n, line in enumerate(f, 1):
                if strip_comment(line):
                    continue
                for key, rx in PATTERNS.items():
                    m = rx.search(line)
                    if m:
                        sites[key].append({"site": f"{rel}:{n}", "text": line.strip()[:160]})
    return sites, digests


def tree_files(root, sub, suffix=".c"):
    out = []
    for d, _, names in os.walk(os.path.join(root, sub)):
        for name in names:
            if name.endswith(suffix):
                out.append(os.path.relpath(os.path.join(d, name), root))
    return sorted(out)


def registrations(root):
    """Plugin structs and their designated cb_ initializers over plugins/."""
    head = re.compile(r"^struct flb_(input|output|filter|processor|custom)_plugin\s+(\w+)\s*=\s*\{")
    member = re.compile(r"^\s*\.(cb_[a-z_]+)\s*=\s*([\w&]+)")
    plugins = []
    for rel in tree_files(root, "plugins"):
        with open(os.path.join(root, rel), encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
        i = 0
        while i < len(lines):
            m = head.match(lines[i])
            if not m:
                i += 1
                continue
            entry = {"kind": m.group(1), "name": m.group(2), "site": f"{rel}:{i + 1}", "members": {}}
            j = i + 1
            while j < len(lines) and not lines[j].startswith("};"):
                mm = member.match(lines[j])
                if mm:
                    entry["members"][mm.group(1)] = {"value": mm.group(2), "site": f"{rel}:{j + 1}"}
                j += 1
            plugins.append(entry)
            i = j
    return plugins


def output_returns(root):
    rx = PATTERNS["P2-output-return"]
    arg = re.compile(r"FLB_OUTPUT_RETURN\s*\(\s*([^)]*)\)")
    sites = []
    for rel in tree_files(root, "plugins") + tree_files(root, "src"):
        with open(os.path.join(root, rel), encoding="utf-8", errors="replace") as f:
            for n, line in enumerate(f, 1):
                if rx.search(line) and not strip_comment(line):
                    a = arg.search(line)
                    sites.append({"site": f"{rel}:{n}", "argument": a.group(1).strip() if a else None})
    return sites


def main():
    out = {
        "card": "FB-QUANT-CONTRACT-CENSUS",
        "kind": "read-only source census; sites, not findings",
        "system_source": SYSTEM,
        "slice_source": SLICE,
        "system_revision": "046accf6aded7906f66a1baf0202da349e521736 (prepare.json)",
        "slice_revision": "90978ac6a59e96d19621108f63e115eb4cd1c576 export (tail-pause-selection.json)",
    }
    for label, root, files in (("system", SYSTEM, SYSTEM_FILES), ("slice", SLICE, SLICE_FILES)):
        sites, digests = scan(root, files)
        out[label] = {"file_sha256": digests, "sites": sites,
                      "counts": {k: len(v) for k, v in sites.items()}}
    regs = registrations(SYSTEM)
    by_kind = {}
    member_counts = {}
    for p in regs:
        by_kind[p["kind"]] = by_kind.get(p["kind"], 0) + 1
        for m in p["members"]:
            member_counts[m] = member_counts.get(m, 0) + 1
    out["tree_registrations"] = {
        "revision": "system export (046accf6)",
        "plugins_by_kind": by_kind,
        "designated_cb_members": dict(sorted(member_counts.items(), key=lambda kv: -kv[1])),
        "selected": [p for p in regs if p["name"] in ("in_storage_backlog_plugin",)],
    }
    out["slice_registrations"] = [p for p in registrations(SLICE) if p["name"] == "in_tail_plugin"]
    rets = output_returns(SYSTEM)
    args = {}
    for r in rets:
        args[r["argument"]] = args.get(r["argument"], 0) + 1
    out["tree_output_returns"] = {"revision": "system export (046accf6)", "sites": len(rets),
                                  "files": len({r["site"].rsplit(":", 1)[0] for r in rets}),
                                  "by_argument": dict(sorted(args.items(), key=lambda kv: -kv[1])[:20])}
    tree = {}
    tree_rx = {
        "timer_callback_registrations": re.compile(r"\bflb_sched_timer_(?:coro_)?cb_create\s*\("),
        "collector_registrations": re.compile(r"\bflb_input_set_collector_(?:time|event|socket|server)\s*\("),
        "mk_event_add_or_timeout_create": re.compile(r"\bmk_event_(?:add|timeout_create)\s*\("),
        "plugin_member_invocations": PATTERNS["P1-invoke"],
    }
    for key, rx in tree_rx.items():
        sites = []
        for rel in tree_files(SYSTEM, "src") + tree_files(SYSTEM, "plugins"):
            with open(os.path.join(SYSTEM, rel), encoding="utf-8", errors="replace") as f:
                for n, line in enumerate(f, 1):
                    if rx.search(line) and not strip_comment(line):
                        sites.append(f"{rel}:{n}")
        tree[key] = {"sites": len(sites), "files": len({s.rsplit(':', 1)[0] for s in sites})}
    out["tree_counts"] = {"revision": "system export (046accf6), src/ and plugins/ .c files, definitions included",
                          "counts": tree}
    dest = os.path.join(REPORT, "artifacts", "source-census.json")
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
        f.write("\n")
    for label in ("system", "slice"):
        print(label, out[label]["counts"])
    print("plugins", by_kind)
    print("output returns", out["tree_output_returns"]["sites"], "in", out["tree_output_returns"]["files"], "files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
