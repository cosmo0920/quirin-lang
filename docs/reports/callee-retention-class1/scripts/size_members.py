#!/usr/bin/env python3
"""size_members.py REPORT [REPORT...] -- CALLEE-RETENTION-CLASS1: for every
class-1 member the card names, per route report, the leaves that name it
(``call to uncontracted function `F` `` or a rule-12t/11r refusal naming it),
the functions carrying such a leaf, and the functions it decides alone (every
leaf of the function names it). Prints one JSON object per report."""
import collections
import json
import re
import sys

MEMBERS = {
    "a": ["add_item_to_object", "add_item_to_array", "replace_item_in_object",
          "cfl_kvlist_insert_array", "cfl_kvlist_insert_kvlist",
          "ctr_attributes_set_array", "ctr_attributes_set_kvlist"],
    "b": ["LZ4_compress_HC_extStateHC", "LZ4_compress_HC_continue",
          "LZ4_compress_fast_continue", "LZ4_compress_fast_extState",
          "append_to_ring_buffer", "rd_kafka_confval_set_type",
          "cJSON_ParseWithLengthOpts", "rd_kafka_interceptor_method_add"],
    "c": ["rd_list_destroy", "rd_list_clear", "cmt_mpack_unpack_array",
          "cmt_mpack_unpack_map", "ctr_mpack_unpack_array", "ctr_mpack_unpack_map",
          "pb_write", "pb_encode_varint", "rd_kafka_q_serve0", "rd_kafka_q_pop_serve0",
          "rd_kafka_q_serve_maybe_consume", "rd_kafka_op_destroy"],
    "older": ["bsearch", "qsort", "pthread_once", "pthread_getspecific",
              "pthread_setspecific", "mk_sched_get_thread_conf"],
}
NAMED = re.compile(r"`([^`]*)`")


def named(description):
    if description.startswith("call to uncontracted function `"):
        return NAMED.search(description).group(1)
    if description.startswith("call to `") and (
            "relational-" in description or "retention-" in description):
        return NAMED.search(description).group(1)
    return None


def main():
    for path in sys.argv[1:]:
        report = json.load(open(path))
        rows = collections.defaultdict(lambda: {"leaves": 0, "functions": 0, "alone": 0})
        for unit in report["units"]:
            for function in unit["functions"]:
                leaves = function.get("diagnostics") or []
                names = [named(leaf.get("description", "")) for leaf in leaves]
                for member in {n for n in names if n}:
                    count = sum(1 for n in names if n == member)
                    rows[member]["leaves"] += count
                    rows[member]["functions"] += 1
                    if function["status"] != "Safe" and all(n == member for n in names):
                        rows[member]["alone"] += 1
        out = {}
        for shape, members in MEMBERS.items():
            out[shape] = {m: dict(rows[m]) for m in members}
        print(json.dumps({"report": path, "members": out}, indent=1))


if __name__ == "__main__":
    main()
