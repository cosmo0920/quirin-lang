#!/usr/bin/env python3
"""rank.py PREMISES.jsonl.gz PPDIR OUT.json -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
round 4, step (a): the functions of the class "uncontracted callee with an
actual neither reader reads" (round 3's refine.py shape
`uncontracted-callee-unread-actual`, every access leaf of the function), each
with its uncontracted callees, the statement the gateway read, the statement
form (plain call, whole-call return, scalar assignment, member destination)
and the actual features neither reader reads (a nested call, a cast, errno,
`?:`, a member destination). The callee's contract question is read off the
body by hand and recorded in CALLEES below (the report's census table cites
each body); a function is freed by `contract` when every callee is `neutral`
and no actual feature is left, `both` when a feature is left, and is
`blocked` when a callee's contract is not a neutral one (its custody needs a
decision or another card).

The round-3 classifier is imported unchanged (`refine.shape`), so the class
is exactly round 3's."""
import collections
import gzip
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
R3 = os.path.join(HERE, "..", "..", "cg10-indirect-access-sole-residuals-r3", "scripts")
sys.path.insert(0, R3)
from leaves import statement  # noqa: E402
from refine import shape  # noqa: E402

IND = "indirect memory access is outside the resource projection"
CLASS = "uncontracted-callee-unread-actual"

# The contract question of each callee, read off its body at 046accf6 (and
# 66910c10 where it differs). `neutral`: every formal Borrowed, scalar or no
# result, nothing freed, stored or returned (a declaration this card may
# write). Every other value names why no neutral declaration is honest.
CALLEES = {
    "rd_jitter": "neutral",
    "cmt_mpack_consume_string_tag": "out-parameter: allocates a cfl_sds into *output_buffer (an owned value into the caller's member: MEMBER-EFFECT-STORES)",
    "ctr_mpack_consume_string_or_nil_tag": "out-parameter: allocates a cfl_sds into *output_buffer (an owned value into the caller's member: MEMBER-EFFECT-STORES)",
    "ctr_mpack_consume_string_tag": "out-parameter: allocates a cfl_sds into *output_buffer (MEMBER-EFFECT-STORES)",
    "rd_atomic32_add": "reference counts (decision class 3; memory.qv keeps it undeclared)",
    "rd_atomic32_sub": "reference counts (decision class 3; memory.qv keeps it undeclared)",
    "rd_list_elem": "container element custody (decision class 1, rule 46): the element is the list's",
    "rd_list_find": "container element custody and a comparator callback (decision class 1)",
    "rd_list_copy": "owned copy of a container (container custody, decision class 1)",
    "rd_kafka_topic_partition_list_copy": "owned constructor result stored into a member (MEMBER-EFFECT-STORES / field link)",
    "rd_kafka_topic_partition_list_new": "owned constructor result stored into a member (MEMBER-EFFECT-STORES / field link)",
    "rd_kafka_topic_partition_copy": "owned constructor result stored into a member (MEMBER-EFFECT-STORES / field link)",
    "rd_kafka_MemberDescription_new": "static constructor in the caller's unit (owned result; W4 summary route)",
    "rd_kafka_group_result_new": "owned constructor (copies; its error actual is a nested owned call)",
    "rd_kafka_error_copy": "owned copy (nested in an actual)",
    "rd_kafka_timer_start0": "registers a timer callback with its argument (decision class 1, callbacks)",
    "rd_kafka_log0": "declared with three named premises (log_caller_frame.qv): never Safe outright",
    "rd_kafka_err2str": "returns static or thread-local storage, nested in a logger actual",
    "rd_kafka_err2name": "returns static or thread-local storage, nested in a logger actual",
    "rd_kafka_topic_name": "returns a borrowed interior string, nested in a logger actual",
    "rd_kafka_topic_name_str_safe": "returns a borrowed interior string, nested in a logger actual",
    "rd_kafka_offset2str": "returns thread-local storage, nested in a logger actual",
    "rd_kafka_msgq_enq_sorted0": "enqueues the message (custody into the queue)",
    "rd_kafka_gzip_compress": "out-parameter: allocates the compressed buffer",
    "rd_kafka_lz4_compress": "out-parameter: allocates the compressed buffer",
    "rd_kafka_snappy_compress_slice": "out-parameter: allocates the compressed buffer",
    "rd_kafka_q_fwd_set0": "queue forwarding: takes and drops queue references (decision class 3)",
    "rd_kafka_timer_schedule_next": "static timer list insertion in the caller's unit (container custody)",
    "rd_clock": "declared elsewhere; nested in an actual",
    "rd_kafka_HeartbeatRequest": "builds and enqueues a request with a reply queue (custody into the broker)",
    "rd_kafka_replyq_make": "takes a queue reference (decision class 3), nested in an actual",
    "XXH32_finalize": "neutral",
    "XXH64_finalize": "neutral",
    "pb_encode_string": "writes through the stream's callback member (callback invocation), and the actual is a return-position cast view (CG10-CAST-VIEW-RETURN-POSITION)",
    "mk_list_init": "self-initialises an embedded list head (container custody, W48-CMT-TRACKED-INTERIOR-SELF-INIT's question for Monkey)",
}


# Actuals neither reader read with the callee declared, found by a held-out
# fixture of the statement (the report's section 2): the function needs a
# reading as well as the contract.
READINGS = {
    "XXH32_digest_endian": "rule 66: `state->mem32`, a decayed `uint32_t mem32[4]` member (typedef element)",
    "XXH64_digest_endian": "rule 66: `state->mem64`, a decayed `uint64_t mem64[4]` member (typedef element)",
}


def features(text, callees):
    found = []
    stripped = text.strip()
    if stripped.startswith("return"):
        found.append("whole-call return")
    head = stripped.split("=", 1)[0] if "=" in stripped.split("(", 1)[0] else ""
    if head and re.search(r"->|\.", head):
        found.append("member destination")
    elif head:
        found.append("scalar destination")
    if len(re.findall(r"\b[A-Za-z_]\w*\s*\(", text)) > 1 + len(re.findall(r"\b(sizeof|if|while|for|return)\s*\(", text)):
        found.append("nested call")
    if re.search(r"\(\s*(const\s+)?(unsigned\s+|signed\s+)?[A-Za-z_]\w*(\s*\*+)?\s*\)\s*[\w(&*]", text):
        found.append("cast")
    if "__errno_location" in text or re.search(r"\berrno\b", text):
        found.append("errno")
    if "?" in text:
        found.append("?:")
    return found


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    ppdir, out = sys.argv[2], sys.argv[3]
    by = collections.defaultdict(list)
    for row in rows:
        if set(row["families"]) <= {IND} and row["status"] == "Unknown":
            by[(row["file"], row["function"])].append(row)
    cache = {}
    functions = []
    for key, leaves in sorted(by.items()):
        path = os.path.join(ppdir, f"unit-{leaves[0]['unit']}.c")
        if path not in cache:
            cache[path] = open(path, errors="replace").read().split("\n") if os.path.exists(path) else []
        lines = cache[path]
        shapes, statements = set(), []
        for leaf in leaves:
            text = statement(lines, leaf["line"] - 1)[0] if lines else leaf["origin_text"]
            shapes.add(shape(leaf, text)[0])
            statements.append((leaf, text))
        if shapes != {CLASS}:
            continue
        callees = sorted({p["callee"] for leaf in leaves for p in leaf["paths"] if p.get("callee")}
                         | {c for leaf in leaves for c in leaf["calls"]})
        feats = sorted({f for leaf, text in statements for f in features(text, callees)})
        verdicts = {c: CALLEES.get(c, "unread") for c in callees}
        blocking = {c: v for c, v in verdicts.items() if v != "neutral"}
        actual_left = [f for f in feats if f in ("nested call", "errno", "?:", "member destination")]
        if blocking:
            frees = "blocked"
        elif actual_left or key[1] in READINGS:
            frees = "both"
        else:
            frees = "contract"
        functions.append({
            "file": key[0], "function": key[1], "leaves": len(leaves),
            "callees": callees, "callee_questions": verdicts, "features": feats,
            "statements": sorted({text for _, text in statements}),
            "origins": sorted({leaf["origin"] for leaf in leaves}),
            "frees": frees,
            "reading": READINGS.get(key[1]),
        })
    by_callee = collections.Counter()
    for item in functions:
        for callee, verdict in item["callee_questions"].items():
            if verdict != "neutral":
                by_callee[callee] += 1
    summary = {
        "class": CLASS,
        "functions": len(functions),
        "frees": dict(collections.Counter(item["frees"] for item in functions)),
        "blocking_callees": dict(by_callee.most_common()),
    }
    json.dump({"summary": summary, "functions": functions}, open(out, "w"), indent=1)
    print(json.dumps(summary, indent=1))
    for item in functions:
        print(f"{item['frees']:9} {item['function']:55} {','.join(item['callees'])} [{'; '.join(item['features'])}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
