#!/usr/bin/env python3
"""FB-QUANT-CONTRACT-CENSUS: size the five patterns on one measured route.

usage: classify.py ROUTE REPORT_JSON NULL_JSONL SOURCE_ROOT OUT_JSON [UNIT]

UNIT, when given, keeps only the units (and null rows) whose file equals it
(the corpus's one Fluent Bit unit, `flb_input_chunk.c`).

ROUTE is `system`, `slice` or `corpus`. Reads the route's custody report
(`resource-diagnostics.json`), its null-property stream (QCORE_NULL_PROPERTY)
and the prepared source export the route analysed. Every custody leaf
(a function row's diagnostic) and every unresolved null check is classified by
the text of its origin source line and, for custody leaves, its description;
a leaf may fall in several patterns or none.

For each pattern it reports:
  rows_total  - non-Safe rows with at least one leaf of the pattern;
  rows_alone  - non-Safe rows whose every leaf is of the pattern (removing the
                pattern's leaves alone would leave no leaf; it does not say the
                row would be Safe);
  via_callee  - non-Safe rows that carry no leaf of the pattern but name, in an
                `uncontracted function` leaf, a function of this route whose
                own row carries one (one step, recorded separately);
  leaf texts with counts; the null rows (non-Safe null verdicts) whose
  unresolved checks fall on the pattern's lines, total and alone.

The classification is lexical (a regular expression over the origin line and
the description). It is a census of where the pattern stands, not a proof that
removing it would decide anything.
"""
import collections
import json
import re
import sys

DESTROY = (r"flb_input_chunk_destroy|flb_task_destroy|flb_task_retry_destroy|flb_input_instance_destroy|"
           r"flb_output_instance_destroy|flb_upstream_conn_release|flb_upstream_destroy|flb_event_chunk_destroy|"
           r"flb_sched_request_destroy|flb_sched_timer_destroy|flb_output_flush_destroy|flb_input_coro_destroy|"
           r"flb_coro_destroy|flb_input_collector_destroy|flb_input_collectors_signal_exit|flb_task_retry_clean|"
           r"flb_task_queue_destroy|flb_input_chunk_release_route|flb_task_users_release|flb_task_retry_remove|"
           r"flb_sched_request_invalidate|flb_input_exit_all|flb_output_exit|flb_engine_destroy_tasks|"
           r"flb_tail_file_remove|flb_tail_file_remove_all|flb_tail_config_destroy|flb_input_thread_instance_exit")
CONSTRUCT = (r"collector_create|flb_sched_request_create|flb_sched_timer_create|flb_coro_create|"
             r"flb_output_flush_create|flb_input_new|flb_output_new|flb_filter_new|flb_processor_create|"
             r"flb_custom_new")
LOOP = (r"mk_event_add|mk_event_del|mk_event_timeout_create|mk_event_timeout_destroy|mk_event_channel_create|"
        r"mk_event_channel_destroy|mk_event_inject|mk_event_wait|mk_event_wait_2|mk_event_foreach|"
        r"flb_input_set_collector_time|flb_input_set_collector_event|flb_input_set_collector_socket|"
        r"flb_input_set_collector_server|flb_input_collector_start|flb_input_collector_pause|"
        r"flb_input_collector_resume|flb_input_collector_delete|flb_input_collector_running|"
        r"flb_input_collectors_start|flb_input_collectors_resume_all|flb_input_collector_fd|"
        r"flb_input_pause|flb_input_resume|flb_input_pause_all|flb_input_resume_all|flb_sched_timer_cb_create")

PATTERNS = {
    "P1-plugin-callback-member": {
        "line": re.compile(r"\b\w+->cb_[a-z_]+\b|\bp->cb_[a-z_]+\b|\bcoll(?:ector)?->cb_collect\b|->p->cb_"),
        "desc": re.compile(r"callback-member|member invocation"),
    },
    "P2-return-code-ownership": {
        "line": re.compile(r"\bFLB_OUTPUT_RETURN\b|\bFLB_TASK_RET\b|\bFLB_TASK_SET\b|\bFLB_(?:OK|RETRY|ERROR)\b|"
                           r"\bFLB_FILTER_(?:MODIFIED|NOTOUCH)\b|\bflb_output_return(?:_do)?\b"),
        "desc": re.compile(r"`flb_output_return(?:_do)?`|`flb_output_flush_finished`|`handle_output_event`"),
    },
    "P3-teardown-member-state": {
        "line": re.compile(r"\b(?:%s)\s*\(" % DESTROY),
        "desc": re.compile(r"`(?:%s)`" % DESTROY),
    },
    "P4-fresh-constructor-callback-members": {
        "line": re.compile(r"\b(?:%s)\s*\(|->(?:cb_[a-z_]+|cb|handler)\s*=[^=]|\bMK_EVENT_(?:ZERO|NEW|INIT)\b" % CONSTRUCT),
        "desc": re.compile(r"`(?:%s)`" % CONSTRUCT),
    },
    "P5a-event-loop-registration": {
        "line": re.compile(r"\b(?:%s)\s*\(" % LOOP),
        "desc": re.compile(r"`(?:%s)`" % LOOP),
    },
    # Leaves of the already-declared loop-owner obligation (rule 10p): only the
    # thread-entry census route can discharge them; separated from P5a.
    "P5b-loop-owner-obligation": {
        "line": re.compile(r"(?!)"),
        "desc": re.compile(r"outside its declared loop-owner obligation"),
    },
}
UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


class Source:
    def __init__(self, root):
        self.root = root
        self.cache = {}

    def line(self, origin):
        if not origin or ":" not in origin:
            return ""
        path, _, n = origin.rpartition(":")
        try:
            n = int(n)
        except ValueError:
            return ""
        if path not in self.cache:
            try:
                with open(f"{self.root}/{path}", encoding="utf-8", errors="replace") as f:
                    self.cache[path] = f.read().split("\n")
            except OSError:
                self.cache[path] = None
        lines = self.cache[path]
        if not lines or n < 1 or n > len(lines):
            return ""
        return lines[n - 1]


def classify(text, desc):
    hit = set()
    for name, p in PATTERNS.items():
        if p["line"].search(text) or (desc and p["desc"].search(desc)):
            hit.add(name)
    if "P5b-loop-owner-obligation" in hit:
        # a declared loop-owner refusal is not a registration leaf
        hit.discard("P5a-event-loop-registration")
    return hit


def norm(desc):
    return re.sub(r"`[^`]*`", "`X`", desc) if desc else desc


def main():
    route, report, null_path, root, out_path = sys.argv[1:6]
    only = sys.argv[6] if len(sys.argv) > 6 else None
    src = Source(root)
    rep = json.load(open(report))
    rows = []
    for u in rep["units"]:
        if only and u["file"] != only:
            continue
        for f in u["functions"]:
            rows.append({"unit": u["file"], "function": f["function"], "status": f["status"],
                         "diagnostics": f.get("diagnostics", []) or []})
    status_counts = collections.Counter(r["status"] for r in rows)
    # per-function (name) status across a route's configurations
    defined = collections.defaultdict(set)
    for r in rows:
        defined[r["function"]].add(r["status"])
    result = {"route": route, "report": report, "null_stream": null_path, "source_root": root,
              "custody_status": dict(status_counts), "rows": len(rows), "patterns": {}}
    per_row = []
    for r in rows:
        leaves = []
        for d in r["diagnostics"]:
            text = src.line(d.get("origin"))
            desc = d.get("description", "")
            leaves.append({"origin": d.get("origin"), "description": desc, "category": d.get("category"),
                           "patterns": sorted(classify(text, desc)), "source": text.strip()[:140]})
        per_row.append((r, leaves))
    # null rows
    null_rows = []
    with open(null_path) as f:
        for line in f:
            n = json.loads(line)
            if only and n["file"] != only:
                continue
            v = n["null"]["verdict"]
            # A `candidate` check is what keeps a null row Unknown; a `moved`
            # check became the function's own precondition (Safe conditional)
            # and an `excluded` one is out of the property.
            checks = [c for c in n["null"].get("checks", []) if c.get("outcome") == "candidate"]
            cl = []
            for c in checks:
                text = src.line(c.get("origin"))
                cl.append({"origin": c.get("origin"), "kind": c.get("kind"), "outcome": c.get("outcome"),
                           "subject": c.get("subject"), "patterns": sorted(classify(text, "")),
                           "source": text.strip()[:140]})
            null_rows.append({"file": n["file"], "function": n["function"], "verdict": v,
                              "scope": n["null"].get("scope"), "custody": n.get("custody"),
                              "combined": n.get("combined"), "unresolved": cl})
    result["null_verdicts"] = dict(collections.Counter(
        (n["verdict"] if n["verdict"] != "Safe"
         else ("Safe-unconditional" if n["scope"] == "unconditional" else "Safe-conditional"))
        for n in null_rows))
    for name in PATTERNS:
        total, alone, texts, via = [], [], collections.Counter(), []
        carrying = set()
        for r, leaves in per_row:
            if r["status"] == "Safe":
                continue
            mine = [l for l in leaves if name in l["patterns"]]
            if mine:
                carrying.add(r["function"])
        for r, leaves in per_row:
            if r["status"] == "Safe":
                continue
            mine = [l for l in leaves if name in l["patterns"]]
            key = f"{r['unit']}::{r['function']}"
            if mine:
                total.append({"row": key, "status": r["status"], "leaves": len(leaves), "pattern_leaves": len(mine),
                              "sites": sorted({l["origin"] for l in mine if l["origin"]})[:12]})
                for l in mine:
                    texts[f"{l['category']}: {norm(l['description'])}"] += 1
                if len(mine) == len(leaves):
                    alone.append(key)
            else:
                callees = {m.group(1) for l in leaves for m in [UNCONTRACTED.search(l["description"] or "")] if m}
                hit = sorted(c for c in callees if c in carrying)
                if hit:
                    via.append({"row": key, "callees": hit})
        nt, na = [], []
        for n in null_rows:
            if n["verdict"] == "Safe":
                continue
            mine = [c for c in n["unresolved"] if name in c["patterns"]]
            if mine:
                nt.append({"row": f"{n['file']}::{n['function']}", "verdict": n["verdict"],
                           "unresolved": len(n["unresolved"]), "pattern_checks": len(mine),
                           "sites": sorted({c["origin"] for c in mine if c["origin"]})[:12]})
                if len(mine) == len(n["unresolved"]):
                    na.append(f"{n['file']}::{n['function']}")
        fn = lambda keys: len({k.split("::", 1)[1] for k in keys})
        result["patterns"][name] = {
            "custody_rows_total": len(total), "custody_rows_alone": len(alone),
            "custody_functions_total": fn([t["row"] for t in total]), "custody_functions_alone": fn(alone),
            "custody_functions_via_callee": fn([v["row"] for v in via]),
            "null_functions_total": fn([t["row"] for t in nt]), "null_functions_alone": fn(na),
            "custody_rows_via_callee": len(via), "custody_leaves": sum(t["pattern_leaves"] for t in total),
            "leaf_texts": dict(texts.most_common()),
            "rows": total, "rows_alone": alone, "via_callee": via,
            "null_rows_total": len(nt), "null_rows_alone": len(na), "null_rows": nt, "null_rows_alone_list": na,
        }
    # unclassified leaves, for honesty about coverage
    un = collections.Counter()
    nleaves = 0
    for r, leaves in per_row:
        if r["status"] == "Safe":
            continue
        for l in leaves:
            nleaves += 1
            if not l["patterns"]:
                un[f"{l['category']}: {norm(l['description'])}"] += 1
    result["non_safe_leaves"] = nleaves
    result["unclassified_leaves"] = sum(un.values())
    result["unclassified_top"] = dict(un.most_common(25))
    result["leaves_by_row"] = {f"{r['unit']}::{r['function']}": {"status": r["status"], "leaves": leaves}
                               for r, leaves in per_row if r["status"] != "Safe"}
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(route, dict(status_counts), "null", result["null_verdicts"])
    for name, p in result["patterns"].items():
        print(f"  {name}: functions total {p['custody_functions_total']} alone {p['custody_functions_alone']} "
              f"via {p['custody_functions_via_callee']}; null functions {p['null_functions_total']}/{p['null_functions_alone']}; "
              f"custody rows total {p['custody_rows_total']} alone {p['custody_rows_alone']} "
              f"via-callee {p['custody_rows_via_callee']} leaves {p['custody_leaves']}; "
              f"null total {p['null_rows_total']} alone {p['null_rows_alone']}")
    print(f"  non-Safe leaves {nleaves}, unclassified {result['unclassified_leaves']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
