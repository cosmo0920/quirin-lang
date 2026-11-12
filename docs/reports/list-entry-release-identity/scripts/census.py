#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""census.py ROUTE REPORT_JSON SOURCE_ROOT OUT_JSON -- LIST-ENTRY-RELEASE-IDENTITY.

The release-actual census on one measured route. Every leaf of a non-Safe row
that stands on a line calling a release position — a declared teardown
destroyer of FB-TEARDOWN-DESTROY-DECLARATIONS, an element destroyer, or a
plain `free`-family release — and reads `unresolved resource identity X` or
`indirect memory access`, classified by the lexical shape of the actual X in
the enclosing body (read only, from the prepared checkout):

* `element-formal`:  X's every write is `X = mk_list_entry*(...)` /
  `container_of(...)` whose pointer is a safe-iteration cursor, an
  `->next`/`->prev` lookup or an `mk_list_entry_first/last` of a container
  rooted at a formal of the function (or a `&G...` static) -- rule 10f(2)'s
  admitted element shape;
* `element-other`:   the same binding over any other container (a local
  list, a callee's result, a member of an element);
* `formal`:          X is a formal of the function;
* `member`:          the actual is a member read (`a->b`);
* `other`:           anything else (a local acquisition, a cast, ...).

Also every `unlink by mk_list_del of X, which this function has not
published` leaf, and every `call to uncontracted function F` leaf for F in
the six undeclared list-entry teardown functions. A census of shapes, not a
proof of what any rule decides.
"""
import collections
import json
import re
import sys

DESTROY = re.compile(
    r"\b(flb_input_chunk_destroy|flb_task_destroy|flb_task_retry_destroy|flb_input_instance_destroy|"
    r"flb_output_instance_destroy|flb_upstream_conn_release|flb_upstream_destroy|flb_event_chunk_destroy|"
    r"flb_sched_request_destroy|flb_sched_timer_destroy|flb_output_flush_destroy|flb_input_coro_destroy|"
    r"flb_coro_destroy|flb_input_collector_destroy|flb_task_queue_destroy|flb_tail_file_remove|"
    r"flb_tail_config_destroy|flb_free|free|flb_sds_destroy|mk_mem_free)\s*\(\s*([^,()]*(?:\([^()]*\))?[^,()]*)")
UNDECLARED = {"flb_task_retry_clean", "flb_sched_request_invalidate", "flb_output_exit",
              "flb_input_exit_all", "flb_engine_destroy_tasks", "flb_tail_file_remove_all"}
UNRESOLVED = re.compile(r"unresolved resource identity `([^`]+)`")
INDIRECT = re.compile(r"indirect memory access")
UNLINK = re.compile(r"unlink by `mk_list_del` of `([^`]+)`, which this function has not published")
UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")
ITER = re.compile(r"mk_list_foreach(?:_safe)?(?:_r)?\s*\(\s*(\w+)\s*,\s*(?:(\w+)\s*,\s*)?([^)]*?)\)\s*\{?")
IDENT = re.compile(r"^[A-Za-z_]\w*$")


def body_of(lines, function, line):
    """The lines of `function`'s definition enclosing 1-based `line`."""
    start = None
    for n in range(line - 1, -1, -1):
        text = lines[n]
        if re.search(r"\b" + re.escape(function) + r"\s*\(", text) and not text.rstrip().endswith(";") \
                and not text.startswith((" ", "\t")):
            start = n
            break
    if start is None:
        return None, []
    depth = 0
    seen = False
    for n in range(start, len(lines)):
        depth += lines[n].count("{") - lines[n].count("}")
        if "{" in lines[n]:
            seen = True
        if seen and depth <= 0:
            return start, lines[start:n + 1]
    return start, lines[start:]


def formals(signature):
    head = signature.split("{")[0]
    inside = head[head.find("(") + 1: head.rfind(")")]
    names = set()
    for part in inside.split(","):
        m = re.search(r"(\w+)\s*(\[\s*\])?\s*$", part.strip())
        if m:
            names.add(m.group(1))
    return names


def root(container):
    c = container.strip()
    m = re.match(r"^&?\s*\(?\s*(\w+)", c)
    return m.group(1) if m else None


def classify(actual, body, params, statics):
    a = actual.strip()
    if "->" in a or "." in a:
        return "member", None
    if not IDENT.match(a):
        return "other", None
    if a in params:
        return "formal", None
    text = "\n".join(body)
    writes = re.findall(r"(?<![\w>.])" + re.escape(a) + r"\s*=(?!=)\s*([^;]*);", text)
    if not writes:
        return "other", None
    containers = []
    for value in writes:
        if not re.search(r"mk_list_entry|container_of", value):
            return "other", None
        m = re.search(r"mk_list_entry_(?:first|last)\s*\(\s*([^,]+),", value)
        if m:
            containers.append(m.group(1))
            continue
        m = re.search(r"(?:mk_list_entry|container_of)\s*\(\s*([^,]+),", value)
        cursor = m.group(1).strip() if m else ""
        nm = re.match(r"^\(?\s*([^)]*?)\s*\)?\s*->\s*(next|prev)$", cursor)
        if nm:
            containers.append(nm.group(1))
            continue
        found = None
        for it in ITER.finditer(text):
            if it.group(1) == cursor:
                found = it.group(3)
        if found is None:
            return "element-other", "cursor-not-iteration"
        containers.append(found)
    for c in containers:
        r = root(c)
        if not (r in params or r in statics):
            return "element-other", c.strip()
    return "element-formal", ";".join(sorted({c.strip() for c in containers}))


def main():
    route, report, source_root, out_path = sys.argv[1:5]
    cache = {}

    def file_lines(path):
        if path not in cache:
            try:
                cache[path] = open(f"{source_root}/{path}", encoding="utf-8", errors="replace").read().split("\n")
            except OSError:
                cache[path] = []
        return cache[path]

    rep = json.load(open(report))
    leaves = []
    for unit in rep["units"]:
        for function in unit["functions"]:
            if function["status"] == "Safe":
                continue
            diagnostics = function.get("diagnostics", []) or []
            for d in diagnostics:
                desc = d.get("description", "") or ""
                origin = d.get("origin") or ""
                path, _, n = origin.rpartition(":")
                n = int(n) if n.isdigit() else 0
                lines = file_lines(path) if path else []
                text = lines[n - 1] if 0 < n <= len(lines) else ""
                kind = None
                actual = None
                callee = None
                m = UNCONTRACTED.search(desc)
                if m and m.group(1) in UNDECLARED:
                    kind, callee = "undeclared-teardown", m.group(1)
                elif UNLINK.search(desc):
                    kind, actual, callee = "unlink", UNLINK.search(desc).group(1), "mk_list_del"
                elif (UNRESOLVED.search(desc) or INDIRECT.search(desc)) and DESTROY.search(text):
                    dm = DESTROY.search(text)
                    callee, actual = dm.group(1), dm.group(2)
                    kind = "unresolved" if UNRESOLVED.search(desc) else "indirect"
                if kind is None:
                    continue
                shape, container = None, None
                if actual is not None and lines:
                    start, body = body_of(lines, function["function"], n)
                    params = formals("\n".join(body[:6])) if body else set()
                    shape, container = classify(actual, body, params, set())
                leaves.append({
                    "unit": unit["file"], "function": function["function"], "status": function["status"],
                    "row_leaves": len(diagnostics), "kind": kind, "callee": callee, "actual": actual,
                    "shape": shape, "container": container, "origin": origin,
                    "description": desc, "source": text.strip()[:160],
                })
    by_shape = collections.Counter((leaf["kind"], leaf["shape"]) for leaf in leaves)
    by_callee = collections.Counter((leaf["kind"], leaf["callee"]) for leaf in leaves)
    rows = collections.defaultdict(set)
    for leaf in leaves:
        rows[(leaf["kind"], leaf["shape"])].add(f"{leaf['unit']}::{leaf['function']}")
    out = {
        "route": route, "report": report,
        "by_shape": [{"kind": k, "shape": s, "leaves": v, "rows": len(rows[(k, s)])}
                     for (k, s), v in by_shape.most_common()],
        "by_callee": [{"kind": k, "callee": c, "leaves": v} for (k, c), v in by_callee.most_common()],
        "leaves": leaves,
    }
    json.dump(out, open(out_path, "w"), indent=1)
    for entry in out["by_shape"]:
        print(f"{entry['kind']:20} {str(entry['shape']):16} leaves {entry['leaves']:4} rows {entry['rows']}")


if __name__ == "__main__":
    main()
