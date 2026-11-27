#!/usr/bin/env python3
"""Review evidence for the served-queue premise (integrator, 2026-10-09).

`rd_kafka_q_wait_result` and `rd_kafka_q_purge0` are declared `rkq`-borrowed
on the premise that an op callback they may run takes over no reference the
caller holds on the served queue. This script reads a librdkafka `src`
directory (lexically; it is evidence, not proof) and reports:

1. every function with the op-callback signature
   `rd_kafka_op_res_t f(rd_kafka_t *, rd_kafka_q_t *Q, rd_kafka_op_t *)`,
   and whether every callback named at an `rd_kafka_op_new_cb(...)` call or an
   `rko_op_cb = NAME` assignment is one of them;
2. each callback's mentions of its queue parameter Q;
3. each callback's direct calls that keep, drop, forward or destroy a queue;
4. the queue argument `rd_kafka_op_destroy`'s callback invocation passes;
5. per callback, the functions reachable by name through the source's call
   graph that keep, drop, forward or destroy a queue (a superset by name; each
   such function takes or releases its own reference, which a reviewer reads).

Usage: served_queue_callbacks.py SRC_DIR  (prints JSON)
"""
import glob, json, os, re, sys

SIG = re.compile(r"rd_kafka_op_res_t\s+(\w+)\s*\(\s*rd_kafka_t\s*\*\s*\w*\s*,"
                 r"\s*rd_kafka_q_t\s*\*\s*(\w*)\s*,\s*rd_kafka_op_t\s*\*\s*\w*\s*\)\s*\{", re.S)
DEF = re.compile(r"^[A-Za-z_][\w \t\*]*?\b(\w+)\s*\([^;{}]*\)\s*\{", re.M)
QOP = re.compile(r"\b(rd_kafka_q_(?:destroy\w*|keep\w*|fwd_set\w*)|rd_kafka_replyq_destroy)\s*\(")
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")


def body_at(s, i):
    d = 1
    j = i
    while d and j < len(s):
        d += {"{": 1, "}": -1}.get(s[j], 0)
        j += 1
    return s[i:j]


def main(src):
    texts = {os.path.basename(f): open(f, errors="ignore").read()
             for f in sorted(glob.glob(os.path.join(src, "*.c")) + glob.glob(os.path.join(src, "*.h")))}
    bodies = {}
    for t in texts.values():
        for m in DEF.finditer(t):
            bodies.setdefault(m.group(1), body_at(t, m.end()))
    callbacks = {}
    for f, t in texts.items():
        for m in SIG.finditer(t):
            b = body_at(t, m.end())
            q = m.group(2)
            callbacks[m.group(1)] = {
                "file": f,
                "queue_param": q,
                "queue_param_mentions": [l.strip() for l in b.split("\n") if q and re.search(r"\b%s\b" % q, l)],
                "direct_queue_ops": sorted({x.group(1) for x in QOP.finditer(b)}),
            }
    alltext = "".join(texts.values())
    named = set(re.findall(r"rd_kafka_op_new_cb\s*\([^;]*?,\s*(\w+)\s*\)\s*;", alltext, re.S))
    named |= set(re.findall(r"rko_op_cb\s*=\s*(\w+)\s*;", alltext))
    named -= {"NULL"}
    graph = {n: {c.group(1) for c in CALL.finditer(b)} & set(bodies) for n, b in bodies.items()}
    for name, cb in callbacks.items():
        seen, todo = set(), [name]
        while todo:
            n = todo.pop()
            if n in seen:
                continue
            seen.add(n)
            todo.extend(graph.get(n, ()))
        cb["reachable_queue_ops"] = sorted(n for n in seen - {name}
                                           if QOP.search(bodies.get(n, "")))
    destroy = bodies.get("rd_kafka_op_destroy", "") + bodies.get("rd_kafka_op_destroy_free", "")
    out = {
        "callbacks": len(callbacks),
        "named_callbacks_not_matching_signature": sorted(n for n in named if n not in callbacks),
        "named_through_variables": sorted(n for n in named if n in ("cb", "op_cb")),
        "callbacks_mentioning_queue_param": sorted(n for n, c in callbacks.items()
                                                   if c["queue_param_mentions"] and n != "rd_kafka_op_call"),
        "callbacks_with_direct_queue_ops": sorted(n for n, c in callbacks.items() if c["direct_queue_ops"]),
        "op_destroy_callback_invocations": [l.strip() for l in destroy.split("\n") if "rko_op_cb(" in l],
        "detail": callbacks,
    }
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    print()


if __name__ == "__main__":
    main(sys.argv[1])
