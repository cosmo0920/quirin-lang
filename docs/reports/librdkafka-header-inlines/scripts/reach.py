#!/usr/bin/env python3
"""reach.py LPP_DIR OUT.json -- copied from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED for
LIBRDKAFKA-HEADER-INLINES (canonical tree roots; walks this card's module only). The census's
reading aid (never evidence for a clause: the gateway reads Clang's AST).

From the preprocessed librdkafka units at both audited revisions
(`LPP_DIR/pp-66910c10`, `LPP_DIR/pp-046accf6`, written by preprocess.py, so
every macro such as `rd_kafka_dbg` or `rd_kafka_assert` is expanded), it
builds an over-approximating call graph of librdkafka's function definitions:

* a direct edge for every call of a name that librdkafka defines (static
  functions of the same name in two units are merged: more edges, never
  fewer);
* a call of a name declared at file scope but defined nowhere in librdkafka is
  an external call (libc, OpenSSL, zstd, ...), listed;
* a call through a member (`X->m(` / `X.m(`) is an rk_conf callback sink when
  `m` is a function-pointer member of `struct rd_kafka_conf_s` (the
  OP-APP-RK census list, `scram_H` excepted: no property or setter writes it,
  only librdkafka's own SHA stores), an interceptor sink when `m` is an
  `on_*` method (`rdkafka_interceptor.c`), and otherwise an unresolved
  indirect call;
* a call of any other name (a parameter, local or global function pointer)
  or of a parenthesised expression is an unresolved indirect call.

For every declaration of the five custody modules (and, for the record, the
86 null views and the other librdkafka modules) it reports: the definition's
unit and line and its preprocessed body digest at both revisions; the rk_conf
members and interceptor methods reachable through direct edges, with the
shortest path to each; the shortest path to an unresolved indirect call; the
external callees reached; and the classification:

* `rk-conf`: a direct path reaches an rk_conf callback or interceptor call;
* `indirect`: no direct path does, but one reaches an unresolved indirect
  call, whose targets this graph does not close (premise given: the
  conservative label);
* `none`: neither; no premise.

Writes OUT.json and prints a summary."""
import collections
import hashlib
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
RK = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka"
CUSTODY = ["header_inlines.qv"]  # LIBRDKAFKA-HEADER-INLINES: this card's module only
OTHER = []
PREMISE = "rk_plugin_callback_closure"
TAGS = ["66910c10", "046accf6"]

# Function-pointer members of struct rd_kafka_conf_s (OP-APP-RK census,
# docs/reports/op-app-rk-callbacks/artifacts/rk-conf-census.json), by their
# last path component.
RK_CONF = {
    "cert_verify_cb", "token_refresh_cb", "rebalance_cb", "offset_commit_cb", "dr_cb", "dr_msg_cb",
    "consume_cb", "log_cb", "error_cb", "throttle_cb", "stats_cb", "socket_cb", "connect_cb",
    "closesocket_cb", "open_cb", "resolve_cb", "background_event_cb", "handle_ProduceResponse",
}
KEYWORDS = {
    "if", "while", "for", "switch", "return", "sizeof", "do", "else", "case", "goto",
    "__typeof__", "typeof", "__typeof", "_Alignof", "__alignof__", "__attribute__", "__extension__",
    "__asm__", "asm", "__asm", "_Static_assert", "__builtin_offsetof", "__builtin_va_arg",
    "_Generic", "__volatile__", "volatile",
}

# Compiler builtins (atomics included): no body, no call.
BUILTIN_PREFIXES = ("__builtin_", "__atomic_", "__sync_", "__c11_atomic_")

MARKER = re.compile(r'^# (\d+) "([^"]*)"')
CALL = re.compile(r"([A-Za-z_]\w*)\s*\(")


def blank_literals(text):
    """Blank string and character literals, keeping offsets and newlines."""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c in "\"'":
            j = i + 1
            while j < n and text[j] != c:
                j += 2 if text[j] == "\\" else 1
            for k in range(i + 1, min(j, n)):
                if out[k] != "\n":
                    out[k] = " "
            i = j + 1
        else:
            i += 1
    return "".join(out)


ROOTS = [
    "/media/Data/GitHub/quirin-lang/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source",
    "/media/Data/GitHub/quirin-lang/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
]


def read_unit(path):
    """The unit's text without line markers (each tree's root spelled
    `<root>`, so `__FILE__` expansions compare equal), and each line's
    (file, line)."""
    lines, where = [], []
    current, number = "?", 0
    text = path.read_text(encoding="utf-8", errors="replace")
    for root in ROOTS:
        text = text.replace(root, "<root>")
    for raw in text.splitlines():
        m = MARKER.match(raw)
        if m:
            number, current = int(m.group(1)), m.group(2)
            continue
        if raw.startswith("#"):  # pragmas
            raw = ""
        lines.append(raw)
        where.append((current, number))
        number += 1
    return "\n".join(lines), where


def scan(text):
    """Top-level function definitions: (name, header_start, body_start,
    body_end) and the names called at file scope (prototypes)."""
    clean = blank_literals(text)
    defs, declared = [], set()
    depth = paren = 0
    seg = 0
    i, n = 0, len(clean)
    while i < n:
        c = clean[i]
        if c == "(":
            paren += 1
        elif c == ")":
            paren -= 1
        elif c == "{":
            if depth == 0 and paren == 0:
                header = clean[seg:i]
                name = definition_name(header)
                # find the matching brace
                d, j = 0, i
                while j < n:
                    if clean[j] == "{":
                        d += 1
                    elif clean[j] == "}":
                        d -= 1
                        if d == 0:
                            break
                    j += 1
                if name:
                    defs.append((name, seg, i, j + 1))
                    declared.add(name)
                else:
                    for m in CALL.finditer(header):
                        declared.add(m.group(1))
                i = j + 1
                seg = i
                continue
            depth += 1
        elif c == "}":
            depth -= 1
        elif c == ";" and depth == 0 and paren == 0:
            for m in CALL.finditer(clean[seg:i]):
                declared.add(m.group(1))
            seg = i + 1
        i += 1
    return clean, defs, declared


def definition_name(header):
    h = header.rstrip()
    if not h.endswith(")") or "=" in h.replace("==", ""):
        return None
    # the parameter list's opening parenthesis
    d, k = 0, len(h) - 1
    while k >= 0:
        if h[k] == ")":
            d += 1
        elif h[k] == "(":
            d -= 1
            if d == 0:
                break
        k -= 1
    m = re.search(r"([A-Za-z_]\w*)\s*$", h[:k])
    if not m or m.group(1) in KEYWORDS or m.group(1) == "__attribute__":
        return None
    return m.group(1)


CRASH = "rd_kafka_crash"
CRASH_NULL = "rd_kafka_crash(rk=NULL)"


def null_rk(body, open_paren):
    """Whether the call whose argument list opens at `open_paren` passes a
    null pointer constant as its fourth argument."""
    depth, start, args = 0, open_paren + 1, []
    for k in range(open_paren, len(body)):
        c = body[k]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                args.append(body[start:k])
                break
        elif c == "," and depth == 1:
            args.append(body[start:k])
            start = k + 1
    if len(args) < 4:
        return False
    return re.sub(r"\s+", "", args[3]) in ("((void*)0)", "(((void*)0))", "0", "(0)")


def calls(body):
    """(kind, name) for every call in a blanked body."""
    out = []
    for m in CALL.finditer(body):
        name = m.group(1)
        if name in KEYWORDS or name.startswith(BUILTIN_PREFIXES):
            continue
        before = body[:m.start()].rstrip()
        if before.endswith("->") or before.endswith("."):
            out.append(("member", name, before[-40:].split()[-1] if before[-40:].split() else ""))
        elif name == CRASH and null_rk(body, m.end() - 1):
            # rd_kafka_crash(file, line, function, NULL, reason) prints and
            # aborts; only a non-null rk reaches rd_kafka_dump0 (rdkafka.c:6640-6649).
            out.append(("name", CRASH_NULL, ""))
        else:
            out.append(("name", name, ""))
    for m in re.finditer(r"\)\s*\(", body):
        # `(*fp)(` or `(expr)(`: a call through an expression, unless the
        # parenthesis closes a cast or a keyword's operand.
        k, d = m.start(), 0
        while k >= 0:
            if body[k] == ")":
                d += 1
            elif body[k] == "(":
                d -= 1
                if d == 0:
                    break
            k -= 1
        inner = body[k + 1:m.start()].strip()
        prev = body[:k].rstrip()
        pm = re.search(r"([A-Za-z_]\w*)$", prev)
        if pm and (pm.group(1) in KEYWORDS or pm.group(1).startswith(BUILTIN_PREFIXES)):
            continue
        if pm:
            continue  # f(a)(b): the call of f is counted; its result's call follows
        if re.fullmatch(r"\*\s*[\w\->.\[\]\s]+", inner):
            out.append(("expr", inner, ""))
        elif re.fullmatch(r"[\w\s\*]+\*", inner) or re.fullmatch(r"(const\s+|unsigned\s+|signed\s+|struct\s+|volatile\s+)*\w+", inner):
            continue  # a cast `(T *)(x)` or `(T)(x)`
        else:
            out.append(("expr", inner, ""))
    return out


def build(pp):
    """name -> {sites: [(file, line, digest)], calls: set}; and the declared set."""
    funcs = collections.defaultdict(lambda: {"sites": set(), "calls": set()})
    declared = set()
    for unit in sorted(pp.glob("*.c.i")):
        text, where = read_unit(unit)
        clean, defs, decl = scan(text)
        declared |= decl
        for name, hstart, bstart, bend in defs:
            line_index = text.count("\n", 0, bstart)
            file, line = where[line_index] if line_index < len(where) else ("?", 0)
            body = clean[bstart:bend]
            digest = hashlib.sha256(re.sub(r"\s+", " ", text[hstart:bend]).encode()).hexdigest()
            rel = file.split("<root>/")[-1]
            funcs[name]["sites"].add((rel, line, digest))
            funcs[name]["calls"] |= set(calls(body))
    return funcs, declared


# External functions that call a function pointer they are passed (the
# callback is then an indirect call of the caller's).
CALLING_EXTERNALS = {"qsort", "qsort_r", "bsearch", "thrd_create", "pthread_create",
                     "call_once", "pthread_once"}

# The indirect calls the census resolved by reading, per declaration:
# {declaration: {call: (candidates, reason)}}. A call whose candidates are
# decided by a callback clause the declaration states has no candidates here:
# each candidate's own declaration is consulted (and its premises published)
# where the call is decided (rules 13u, 13w). Every other unresolved indirect
# call is reached conservatively.
DECLARED_REGISTRY = ("the declaration states `resource_callback_registry_invoked`: the call is "
                     "decided over the registration in force at each call site, whose candidates' "
                     "own declarations are consulted and publish their premises (rules 13u, 13w)")
DECLARED_INVOKED = ("the declaration states `resource_callback_invoked(cmp, _, _, opaque)`: the "
                    "comparator is decided at each call site over its own candidates' declarations "
                    "(rule 13u)")
RESOLVED = {
    # LIBRDKAFKA-HEADER-INLINES: not a call. `(unsigned long long)(rk->rk_curr_msgs.size
    # + size)` (rdkafka_int.h:1186) is a cast of a parenthesised sum, which the
    # lexical scan reads as a call of an expression.
    "rd_kafka_curr_msgs_add": {"(unsigned long long)(": ([], "a cast, not a call (rdkafka_int.h:1186)")},
    "rd_list_destroy": {"rl->rl_free_cb(": ([], DECLARED_REGISTRY)},
    "rd_list_clear": {"rl->rl_free_cb(": ([], DECLARED_REGISTRY)},
    "pb_write": {"(!stream->callback(": ([], DECLARED_REGISTRY)},
    "pb_encode_varint": {"(!stream->callback(": ([], DECLARED_REGISTRY)},
    "rd_kafka_topic_partition_list_sort": {"external:qsort": ([], DECLARED_INVOKED)},
    "rd_kafka_interceptor_method_add": {
        "cmp(": (["rd_kafka_interceptor_method_cmp"],
                 "rd_list_find's comparator is the body's only one, rd_kafka_interceptor_method_cmp "
                 "(rdkafka_interceptor.c:151)"),
        "external:bsearch": (["rd_kafka_interceptor_method_cmp"], "the same comparator, through bsearch"),
    },
    "rd_kafka_aborted_txns_next_offset": {
        "ravl->ravl_cmp(": (["rd_kafka_aborted_txn_cmp_by_pid"],
                            "the aborted-transactions tree is initialised only with "
                            "rd_kafka_aborted_txn_cmp_by_pid (rdkafka_msgset_reader.c:1829)"),
    },
    "rd_kafka_metadata_copy_internal": {
        "external:bsearch": (["rd_kafka_metadata_broker_internal_cmp"],
                             "the body's bsearch passes rd_kafka_metadata_broker_internal_cmp "
                             "(rdkafka_metadata.c:298-302)"),
        "cmp(": (["rd_strcmp2"], "rd_list_deduplicate's comparator is rd_strcmp2 (rdkafka_metadata.c:313)"),
        "external:qsort": (["rd_strcmp2"], "the same comparator, through rd_list_sort"),
        "external:qsort_r": (["rd_strcmp2"], "the same comparator, through rd_list_sort"),
        "rl->rl_free_cb(": ([], "the deduplicated list is rd_list_new(0, NULL) (rdkafka_metadata.c:290) "
                                "and its copy keeps that free callback (rdlist.c:400): none"),
    },
    # cJSON's allocator hooks are malloc/free unless cJSON_InitHooks installs
    # a pair; librdkafka never calls it (premise `cjson_allocator_hooks`).
    **{f: {c: ([], "cJSON's hooks (premise cjson_allocator_hooks): malloc / free in global_hooks "
                   "(cJSON.c:178, :186); librdkafka never calls cJSON_InitHooks")
           for c in ("re:hooks(\\.|->)(allocate|deallocate|reallocate)\\($",)}
       for f in ("cJSON_Delete", "cJSON_New_Item", "replace_item_in_object", "cJSON_ParseWithLengthOpts",
                 "print", "add_item_to_array", "get_object_item")},
    "rd_kafka_ConsumerGroupDescription_new": {
        "copy_cb(": (["rd_kafka_MemberDescription_list_copy"],
                     "rd_list_copy_to's copier is the body's only one, "
                     "rd_kafka_MemberDescription_list_copy (rdkafka_admin.c:8007-8008)"),
    },
    "rd_kafka_ConfigResource_destroy": {
        "rl->rl_free_cb(": (["rd_kafka_ConfigEntry_free"],
                            "config->config is initialised only by rd_kafka_ConfigResource_new "
                            "(rdkafka_admin.c:2971) and each entry's synonyms only by "
                            "rd_kafka_ConfigEntry_new / _copy (:2782, :2810, rd_list_init_copy keeps "
                            "the source's), each with rd_kafka_ConfigEntry_free"),
    },
    "rd_kafka_NewPartitions_destroy": {
        "rl->rl_free_cb(": (["rd_list_destroy_free"],
                            "newps->replicas is initialised by rd_kafka_NewPartitions_new "
                            "(rdkafka_admin.c:2483) or copied from one (:2509), with "
                            "rd_list_destroy_free; its elements are rd_list_new(0, NULL) int32 "
                            "lists (:2563) or rd_list_copy_preallocated copies (rdlist.c:539-541), "
                            "with no free callback"),
    },
    "rd_kafka_NewTopic_destroy": {
        "rl->rl_free_cb(": (["rd_list_destroy_free", "rd_kafka_ConfigEntry_free"],
                            "new_topic->replicas is initialised with rd_list_destroy_free "
                            "(rdkafka_admin.c:1905, copied at :1939) over rd_list_new(0, NULL) "
                            "int32 lists (:2010) and new_topic->config with "
                            "rd_kafka_ConfigEntry_free (:1911, copied at :1943), whose synonyms "
                            "use the same (:2782, :2810)"),
    },
}


def classify(funcs, declared):
    """Per function: edges and local sinks."""
    graph, local = {}, {}
    for name, f in funcs.items():
        edges, sinks = set(), []
        for kind, callee, ctx in f["calls"]:
            if kind == "member":
                if callee in RK_CONF:
                    sinks.append(("rk-conf", callee))
                elif callee.startswith("on_") and ctx.endswith("u."):
                    sinks.append(("interceptor", callee))
                else:
                    sinks.append(("indirect", f"{ctx}{callee}("))
            elif kind == "expr":
                sinks.append(("indirect", f"({callee})("))
            elif callee == CRASH_NULL:
                sinks.append(("external", "abort"))
            elif callee in funcs:
                edges.add(callee)
            elif callee in declared:
                sinks.append(("external", callee))
                if callee in CALLING_EXTERNALS:
                    sinks.append(("indirect", f"external:{callee}"))
            else:
                sinks.append(("indirect", f"{callee}("))
        graph[name] = sorted(edges)
        local[name] = sinks
    return graph, local


def reach(start, graph, local, resolved=None, avoid=()):
    """BFS from start: the shortest path to each sink. An indirect call that
    `resolved` names continues to its candidates instead of being a sink."""
    resolved = resolved or {}
    parent = {start: None}
    order = collections.deque([start])
    found = {}
    used = {}
    while order:
        node = order.popleft()
        nexts = list(graph.get(node, []))
        for sink in local.get(node, []):
            key = None
            if sink[0] == "indirect":
                key = sink[1] if sink[1] in resolved else next(
                    (k for k in resolved if k.startswith("re:") and re.search(k[3:], sink[1])), None)
            if key is not None:
                used[key] = resolved[key]
                nexts.extend(resolved[key][0])
                continue
            if sink not in found:
                path, p = [], node
                while p is not None:
                    path.append(p)
                    p = parent[p]
                found[sink] = list(reversed(path))
        for nxt in nexts:
            if nxt not in parent and nxt not in avoid:
                parent[nxt] = node
                order.append(nxt)
    return found, len(parent), used


def declarations():
    out = []
    for module in CUSTODY + OTHER:
        text = (RK / module).read_text(encoding="utf-8")
        for name, body in re.findall(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", text, re.S):
            out.append({"module": module, "function": name, "has_premise": PREMISE in body,
                        "custody_scope": module in CUSTODY})
    return out


def main():
    lpp = pathlib.Path(sys.argv[1])
    per_tag = {}
    for tag in TAGS:
        funcs, declared = build(lpp / f"pp-{tag}")
        graph, local = classify(funcs, declared)
        per_tag[tag] = (funcs, graph, local)
    rows = []
    for d in declarations():
        name = d["function"]
        row = dict(d)
        for tag in TAGS:
            funcs, graph, local = per_tag[tag]
            if name not in funcs:
                row[tag] = {"defined": False}
                continue
            found, size, used = reach(name, graph, local, RESOLVED.get(name))
            avoiding, _, _ = reach(name, graph, local, RESOLVED.get(name), avoid={CRASH})
            rk = {f"{k}:{v}": p for (k, v), p in found.items() if k in ("rk-conf", "interceptor")}
            rk_avoiding = {f"{k}:{v}": p for (k, v), p in sorted(avoiding.items(), key=lambda kv: (len(kv[1]), kv[0]))
                           if k in ("rk-conf", "interceptor")}
            ind = {v: p for (k, v), p in found.items() if k == "indirect"}
            ext = sorted({v for (k, v), _ in found.items() if k == "external"})
            first_ind = min(ind.items(), key=lambda kv: (len(kv[1]), kv[0])) if ind else None
            row[tag] = {
                "defined": True,
                "sites": sorted([list(s) for s in funcs[name]["sites"]]),
                "reachable_functions": size,
                "rk_conf_sinks": dict(sorted(rk.items(), key=lambda kv: (len(kv[1]), kv[0]))),
                "indirect_calls_reached": len(ind),
                "first_indirect": {"call": first_ind[0], "path": first_ind[1]} if first_ind else None,
                "local_indirect": sorted({v for k, v in local[name] if k == "indirect"}),
                "resolved": {call: {"candidates": c, "reason": r} for call, (c, r) in sorted(used.items())},
                "rk_conf_sinks_avoiding_rd_kafka_crash": rk_avoiding,
                "externals_reached": ext,
                "class": "rk-conf" if rk else ("indirect" if ind else "none"),
            }
        a, b = row[TAGS[0]], row[TAGS[1]]
        row["identical_at_both"] = a == b
        row["class"] = a.get("class", "undefined")
        # The second revision's result is recorded by its definition digests
        # only: everything else is compared above and equal when
        # identical_at_both holds.
        row[TAGS[1]] = {k: b[k] for k in ("defined", "sites", "class") if k in b}
        rows.append(row)
    out = pathlib.Path(sys.argv[2])
    out.write_text(json.dumps({"tags": TAGS, "rk_conf_members": sorted(RK_CONF), "rows": rows},
                              indent=1, ensure_ascii=False) + "\n")
    tally = collections.Counter((r["module"], r["has_premise"], r["class"]) for r in rows)
    for k in sorted(tally):
        print(*k, tally[k])
    print("identical at both:", sum(r["identical_at_both"] for r in rows), "of", len(rows))


main()
