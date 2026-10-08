#!/usr/bin/env python3
"""size_members.py REPORT [REPORT...] -- REFCOUNT-RESOURCE-KIND (adapted from SLOT-MOVE-AND-INTERIOR-BORROW, from W43-ABI-ENUM-AND-CONST-PP, from CALLEE-RETENTION-CLASS1): for every
member the card names (class 2, class 8 and the derived out slot), per route report, the leaves that name it
(``call to uncontracted function `F` `` or a rule-12t/11r refusal naming it),
the functions carrying such a leaf, and the functions it decides alone (every
leaf of the function names it). Prints one JSON object per report."""
import collections
import json
import re
import sys

MEMBERS = {
    "class2": ["sds_cat_safe", "cfl_sds_cat_safe", "format_metric", "mk_header_prepare",
               "rd_kafka_parse_Metadata0", "setupRackAwareAssignment0",
               "ut_init_member_internal", "flb_config_task_map_grow"],
    "class8": ["rd_kafka_admin_result_ret_groups", "rd_kafka_admin_result_ret_resources",
               "rd_kafka_admin_result_ret_topics", "get_object_item",
               "get_or_create_external_metadata_kvlist",
               "rd_kafka_topic_partition_get_private"],
    "out-slot": ["rd_slice_reader0", "cJSON_ParseWithLengthOpts", "strtol", "strtoll",
                 "strtoul", "strtoull", "strtod", "strtof", "strtold"],
}
NAMED = re.compile(r"`([^`]*)`")


def named(description):
    if description.startswith("call to uncontracted function `"):
        return NAMED.search(description).group(1)
    if description.startswith("call to `") and (
            "relational-" in description or "retention-" in description
            # SLOT-MOVE-AND-INTERIOR-BORROW: a slot clause the call does not
            # decide (rules 12x, 12z), and a withdrawn declaration.
            or " with slot `" in description or "was withdrawn against" in description):
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
