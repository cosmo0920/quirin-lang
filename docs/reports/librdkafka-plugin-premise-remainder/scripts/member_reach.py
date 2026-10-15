#!/usr/bin/env python3
"""member_reach.py LPPR_DIR OUT.json -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER:
the member census's reading aid (never evidence for a clause: the gateway
reads Clang's AST, and the run's fold checks every analysed store).

It builds LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's over-approximating call graph
(`reach.py`, imported unchanged) from the preprocessed librdkafka units at
both audited revisions (`LPPR_DIR/pp-66910c10`, `LPPR_DIR/pp-046accf6`) and
applies this card's rule to every member declaration of
`null_deref.qv` (`nonnull_member_stable`, `nonnull_member_scope`, and, for
the record only, rule 14t's `nonnull_member`):

  A member declaration names P-RK-NO-PLUGIN-LIBRARY when a callback outside
  the run could falsify its fact, that is when either
  (a) a function that may store into TAG.FIELD (an assignment, a compound
      update or an initializer naming the member, or a whole-object fill or
      copy of a struct TAG, read from the preprocessed bodies) can run
      inside a plugin-registered callback: it is reachable on the graph from
      a function with external linkage, which a callback a plugin library
      registers may call (the RD_EXPORT functions of rdkafka.h are listed
      first as the public entries); or
  (b) a plugin-registered callback can run while the fact is relied on: the
      declared function's own body reaches an rk_conf callback or an
      interceptor call (reach.py's class `rk-conf`), or an unresolved
      indirect call (`indirect`), on the same graph.
  A store an analysed unit makes is checked by the run's fold whatever
  context runs it (rules 14t, 14x); what neither check covers is a store
  made by code outside the run, which the premise excludes. Since
  librdkafka runs callbacks on its own threads, (b) is recorded but (a)
  alone decides for a member whose fact persists across the run (rule 14y).

Writes OUT.json and prints a summary."""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
EARLIER = REPO / "docs/reports/librdkafka-plugin-premise/scripts/reach.py"
NULL_VIEW = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv"
PUBLIC_HEADER = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/"
    "fluent-bit-clang-source/lib/librdkafka-2.15.0/src/rdkafka.h")

source = EARLIER.read_text().replace("\nmain()\n", "\n")
reach = {"__file__": str(EARLIER), "__name__": "reach"}
exec(compile(source, str(EARLIER), "exec"), reach)

FILL = ("memset", "memcpy", "memmove", "bzero", "explicit_bzero")
TYPEDEFS = {"rd_kafka_broker_s": "rd_kafka_broker_t", "rd_kafka_toppar_s": "rd_kafka_toppar_t",
            "rd_kafka_topic_s": "rd_kafka_topic_t", "rd_kafka_cgrp_s": "rd_kafka_cgrp_t",
            "rd_kafka_transport_s": "rd_kafka_transport_t"}


def member_declarations():
    text = NULL_VIEW.read_text(encoding="utf-8")
    out = []
    for name, body in re.findall(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", text, re.S):
        for form, args in re.findall(r"requires (nonnull_member(?:_stable|_scope)?)\(([^)]*)\);", body):
            parts = [a.strip() for a in args.split(",")]
            out.append({"function": name, "form": form, "tag": parts[0], "field": parts[1],
                        "parameter": parts[2] if len(parts) > 2 else None,
                        "premised_before": "resource_premise(" in body})
    return out


def public_api():
    text = PUBLIC_HEADER.read_text(encoding="utf-8", errors="replace")
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    names = set()
    for chunk in text.split("RD_EXPORT")[1:]:
        m = re.search(r"([A-Za-z_]\w*)\s*\(", chunk.split(";")[0])
        if m:
            names.add(m.group(1))
    return names


def bodies(pp):
    """name -> {"static": all sites static, "text": [blanked bodies]}."""
    out = collections.defaultdict(lambda: {"static": True, "text": []})
    for unit in sorted(pp.glob("*.c.i")):
        text, _ = reach["read_unit"](unit)
        clean, defs, _ = reach["scan"](text)
        for name, hstart, bstart, bend in defs:
            header = clean[hstart:bstart]
            if not re.search(r"\bstatic\b", header):
                out[name]["static"] = False
            out[name]["text"].append(clean[bstart:bend])
    return out


def stores(text, tag, field):
    """The store kinds a blanked body makes to TAG.FIELD, read textually."""
    kinds = set()
    if re.search(r"(->|\.)\s*" + field + r"\s*(=(?!=)|\+=|-=|\|=|&=|\^=)", text):
        kinds.add("assignment-or-initializer")
    names = [f"struct {tag}", TYPEDEFS.get(tag, tag)]
    if any(n in text for n in names) and any(re.search(r"\b" + f + r"\s*\(", text) for f in FILL):
        kinds.add("possible-whole-object")
    return sorted(kinds)


def multi_source_paths(sources, graph):
    """BFS from every source at once: the shortest path to each node."""
    parent = {s: None for s in sources}
    order = collections.deque(sorted(sources))
    while order:
        node = order.popleft()
        for nxt in graph.get(node, []):
            if nxt not in parent:
                parent[nxt] = node
                order.append(nxt)

    def path(node):
        out = []
        while node is not None:
            out.append(node)
            node = parent[node]
        return list(reversed(out))
    return parent, path


def main():
    lppr = pathlib.Path(sys.argv[1])
    decls = member_declarations()
    api = public_api()
    per_tag = {}
    for tag in reach["TAGS"]:
        funcs, declared = reach["build"](lppr / f"pp-{tag}")
        graph, local = reach["classify"](funcs, declared)
        texts = bodies(lppr / f"pp-{tag}")
        external = {n for n, b in texts.items() if not b["static"]}
        public = external & api
        reach_public, path_public = multi_source_paths(public, graph)
        reach_external, path_external = multi_source_paths(external, graph)
        members = collections.defaultdict(dict)
        for d in decls:
            key = (d["tag"], d["field"])
            if key in members:
                continue
            storing = {n: stores("\n".join(b["text"]), *key) for n, b in texts.items()}
            storing = {n: k for n, k in storing.items() if k}
            members[key] = {
                n: {"kinds": k,
                    "external_linkage": n in external,
                    "path_from_public_api": path_public(n) if n in reach_public else None,
                    "path_from_external_function": path_external(n) if n in reach_external else None}
                for n, k in sorted(storing.items())}
        per_tag[tag] = (funcs, graph, local, members, len(public), len(external))
    rows = []
    for d in decls:
        row = dict(d)
        key = (d["tag"], d["field"])
        for tag in reach["TAGS"]:
            funcs, graph, local, members, n_public, n_external = per_tag[tag]
            store = members[key]
            a = any(s["path_from_external_function"] for s in store.values())
            if d["function"] in funcs:
                found, _, _ = reach["reach"](d["function"], graph, local, reach["RESOLVED"].get(d["function"]))
                rk = {f"{k}:{v}": p for (k, v), p in found.items() if k in ("rk-conf", "interceptor")}
                ind = {v: p for (k, v), p in found.items() if k == "indirect"}
                first = min(rk.items(), key=lambda kv: (len(kv[1]), kv[0])) if rk else None
                b_class = "rk-conf" if rk else ("indirect" if ind else "none")
            else:
                first, b_class = None, "undefined"
            row[tag] = {
                "member_storing_functions": len(store),
                "storing_functions_reachable_from_public_api":
                    sorted(n for n, s in store.items() if s["path_from_public_api"]),
                "rule_a": a,
                "rule_b_class": b_class,
                "rule_b_first_sink": {"sink": first[0], "path": first[1]} if first else None,
            }
        x, y = row[reach["TAGS"][0]], row[reach["TAGS"][1]]
        row["identical_at_both"] = x == y
        row["premise"] = d["form"] != "nonnull_member" and (x["rule_a"] or x["rule_b_class"] != "none")
        row["premise_by_rule_for_the_record"] = x["rule_a"] or x["rule_b_class"] != "none"
        rows.append(row)
    stores_out = {}
    for tag in reach["TAGS"]:
        stores_out[tag] = {f"{t}.{f}": v for (t, f), v in sorted(per_tag[tag][3].items())}
    out = pathlib.Path(sys.argv[2])
    out.write_text(json.dumps({
        "tags": reach["TAGS"],
        "public_api_functions_defined": {t: per_tag[t][4] for t in reach["TAGS"]},
        "external_functions_defined": {t: per_tag[t][5] for t in reach["TAGS"]},
        "member_stores": stores_out,
        "rows": rows,
    }, indent=1, ensure_ascii=False) + "\n")
    tally = collections.Counter((r["form"], r["premise"], r["premise_by_rule_for_the_record"],
                                 r[reach["TAGS"][0]]["rule_a"], r[reach["TAGS"][0]]["rule_b_class"])
                                for r in rows)
    print("form premise by-rule rule_a rule_b count")
    for k in sorted(tally, key=str):
        print(*k, tally[k])
    print("identical at both:", sum(r["identical_at_both"] for r in rows), "of", len(rows))
    for key, store in sorted(per_tag[reach["TAGS"][0]][3].items()):
        print(".".join(key), "storing functions:", len(store), sorted(store))


main()
