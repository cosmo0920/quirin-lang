#!/usr/bin/env python3
"""arms.py FACTS.json -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: from the member
facts the census example wrote (`member_facts_census`), list each arm of
`rd_kafka_op_s.rko_u` that holds a function pointer, directly or through a
record it holds by value (rule 12n's `Tables::carries_function`), and whether
each record the union embeds carries one. A reading aid."""
import json
import sys

facts = json.load(open(sys.argv[1]))
records = {}
for unit in facts.values():
    if "unread" in unit:
        continue
    for tag, keys in unit.get("records", {}).items():
        if keys:
            records.setdefault(tag, keys)


def function_fields(keys, path, seen, out):
    for name, field in keys["fields"].items():
        here = path + [name]
        if field["field"] == "function":
            out.append(".".join(here))
        elif field["field"] == "named" and field["record"] not in seen:
            record = field["record"]
            if record in records:
                function_fields(records[record], here + [f"<{record}>"], seen | {record}, out)
        elif field["field"] == "inline":
            function_fields(field["keys"], here, seen, out)
    return out


union = records["rd_kafka_op_s"]["fields"]["rko_u"]
arms = union["keys"]["fields"]
summary = {}
for arm, field in arms.items():
    found = []
    if field["field"] == "inline":
        function_fields(field["keys"], [arm], set(), found)
    elif field["field"] == "named":
        function_fields(records.get(field["record"], {"fields": {}}), [arm, f"<{field['record']}>"], set(), found)
    elif field["field"] == "function":
        found.append(arm)
    if found:
        summary[arm] = found
print(json.dumps(summary, indent=1))
for record in ["rd_list_s", "rd_kafka_timer_s", "rd_kafka_replyq_s", "rd_kafka_fetch_pos_s",
               "rd_kafka_msg_s", "rd_kafka_AdminOptions_s", "rd_kafka_msgq_s",
               "rd_kafka_group_member_s"]:
    keys = records.get(record)
    print(record, "undefined" if keys is None else function_fields(keys, [], {record}, []))
