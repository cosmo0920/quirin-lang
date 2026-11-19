#!/usr/bin/env python3
"""refine.py PREMISES.jsonl.gz PPDIR [MOVED.json] -- AG1: the sole-blocker
functions by concrete sub-shape and owner.

Each access leaf of a sole-blocker function gets one concrete sub-shape from
its AST premise, its paths and its analysed statement (first match in the
order of SHAPES); a function counts under a sub-shape *alone* when every one
of its leaves has it. MOVED.json (optional): functions the card's rule moved,
listed under `rule-61` whatever their leaves say."""
import collections
import gzip
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from leaves import statement  # noqa: E402

IND = "indirect memory access is outside the resource projection"
RELEASE = re.compile(r"(^|_)(free|destroy|fclose|close|clear|unref|release|term)$")


def shape(leaf, text):
    premise = leaf["premise"]
    paths = leaf["paths"]
    callees = [c for c in leaf["calls"]]
    if "rd_kafka_op_err_destroy" in callees and "rd_kafka_op_req" in callees:
        return "nested-return-op-req", "contract: rd_kafka_op_err_destroy (cross-unit, uncontracted)"
    if premise == "form:union":
        if any(p["access"] == "store" and p["value"] == "pointer" for p in paths):
            return "union-member-pointer-store", "rule 10w (a pointer store through a union refuses)"
        return "union-member-access", "rule 10w residual"
    if premise.startswith("store:pointer"):
        roots = {p["root_kind"] for p in paths if p["access"] == "store"}
        rhs = " ".join(p.get("rhs", "") for p in paths if p["access"] == "store")
        if premise == "store:pointer-call-result":
            if "toppar_keep" in rhs:
                return "refcount-keep-stored", "decision class 3 (reference counts)"
            if "formal" in roots:
                return "owned-result-into-formal-member", "MEMBER-EFFECT-STORES (rule 38i names it)"
            return "owned-result-into-local-member", "rule 61 (1) where the base is alias-spelled; else B08 premises"
        if premise == "store:pointer-name":
            return "pointer-name-into-member", "decision class 1 (retained_by) / rule 10v"
        if "arithmetic" in rhs:
            return "pointer-arithmetic-store", "rule 10v (identity-carrying value)"
        return "pointer-other-store", "rule 10v (identity-carrying value)"
    if premise == "read:pointer-member":
        released = [p for p in paths if p.get("callee") and RELEASE.search(p["callee"])]
        if released and any(s["op"] == "[]" for p in released for s in p["steps"]):
            return "formal-array-element-released", "rule 5k / W45-MEMBER-RELEASE-PREMISES (by design)"
        if released:
            return "member-released-parent-kept", "rule 38j / MEMBER-EFFECT-SUMMARY"
        if re.search(r"rd_kafka_log0|rd_kafka_dbg", text):
            return "log-macro-actuals", "contract/decision: rd_kafka_log0 (callback, class 1)"
        if re.search(r"\(\s*\w+\s*\)\s*=\s*\(\(.*tqh_first", text) or "tqe_next" in text:
            return "queue-macro-iteration", "rule 5l list iteration residual"
        return "pointer-member-read-actual", "callee's mode or another premise of the statement"
    if premise == "address:sub-object":
        callees_at = {p.get("callee") for p in paths if p["value"] == "address"}
        if any(c and re.match(r"rd_list_(add|destroy|clear|init)|mk_list_init|cfl_list", c) for c in callees_at):
            return "embedded-list-address", "container custody (rd_list_*): decision class 1 / rule 46"
        if any(c and re.match(r"mtx_|cnd_|rd_atomic|cmt_atomic", c) for c in callees_at):
            return "lock-or-atomic-address", "nested borrowed calls (rule 14a: one call) / uncontracted"
        if any(c and "interceptor_method_add" in c for c in callees_at):
            return "embedded-list-address", "container custody (interceptor list): decision class 1"
        return "sub-object-address-actual", "callee's mode or another premise"
    if premise == "call:uncontracted-callee":
        return "uncontracted-callee-unread-actual", "contracts (rule 12k cannot name: an actual it does not read)"
    if premise in ("form:indirect-call", "call:indirect-callee"):
        return "indirect-call", "rule 11p"
    if premise == "read:custody-free":
        if len(re.findall(r"\b\w+\s*\(", text)) >= 2:
            return "two-borrowed-calls", "rule 14a (`do not admit two calls`)"
        return "custody-free-read-other", "?: / cast / descriptor"
    return premise, "see premise table"


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    ppdir = sys.argv[2]
    moved = set()
    if len(sys.argv) > 3:
        moved = {tuple(item) for item in json.load(open(sys.argv[3]))}
    by = collections.defaultdict(list)
    for row in rows:
        if set(row["families"]) <= {IND} and row["status"] == "Unknown":
            by[(row["file"], row["function"])].append(row)
    cache = {}
    alone = collections.defaultdict(list)
    mentions = collections.Counter()
    owners = {}
    for key, leaves in sorted(by.items()):
        path = os.path.join(ppdir, key[0] + ".i")
        if path not in cache:
            cache[path] = open(path, errors="replace").read().split("\n") if os.path.exists(path) else []
        lines = cache[path]
        shapes = set()
        for leaf in leaves:
            text = statement(lines, leaf["line"] - 1)[0] if lines else leaf["origin_text"]
            name, owner = shape(leaf, text)
            owners[name] = owner
            shapes.add(name)
        for name in shapes:
            mentions[name] += 1
        if key in moved:
            alone["rule-61 (moved)"].append(key[1])
        elif len(shapes) == 1:
            alone[next(iter(shapes))].append(key[1])
        else:
            alone["several"].append(key[1])
    print("| sub-shape | sole functions mentioning | sole functions blocked by it alone | owner / reason | sample |")
    print("|---|---:|---:|---|---|")
    order = sorted(set(mentions) | set(alone), key=lambda n: (-len(alone.get(n, [])), -mentions.get(n, 0)))
    for name in order:
        sample = ", ".join(f"`{f}`" for f in alone.get(name, [])[:3])
        print(f"| `{name}` | {mentions.get(name, 0)} | {len(alone.get(name, []))} | {owners.get(name, '')} | {sample} |")
    print(f"\ntotal sole functions {len(by)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
