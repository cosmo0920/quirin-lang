#!/usr/bin/env python3
"""size_members.py REPORT [REPORT...] -- W43-ABI-ENUM-AND-CONST-PP (adapted from CALLEE-RETENTION-CLASS1): for every
member the card names (enum, const-pp and class-1 shapes), per route report, the leaves that name it
(``call to uncontracted function `F` `` or a rule-12t/11r refusal naming it),
the functions carrying such a leaf, and the functions it decides alone (every
leaf of the function names it). Prints one JSON object per report."""
import collections
import json
import re
import sys

MEMBERS = {
    "enum": ["LZ4_decompress_generic", "rd_kafka_error_new_v", "rd_kafka_error_new",
             "rd_kafka_AclBinding_new0", "rd_kafka_ConsumerGroupDescription_new",
             "rd_kafka_admin_add_config0", "rd_kafka_err_action",
             "rd_kafka_oauthbearer_set_token_failure", "rd_kafka_toppars_pause_resume",
             "getrlimit", "setrlimit"],
    "const-pp": ["rd_flags2str", "rd_kafka_anyconf_dump", "rd_slice_reader0"],
    "class1": ["rd_kafka_confval_set_type", "rd_kafka_interceptor_method_add",
               "add_item_to_object", "cJSON_ParseWithLengthOpts"],
    "also-enum": ["rd_kafka_assignor_add"],
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
