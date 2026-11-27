#!/usr/bin/env python3
"""refusals.py REPORT OUT.json -- card REFCOUNT-RESOURCE-KIND: size the
reference route's refusals on one corpus report, for the residuals.

Every leaf whose description carries one of rule 13b's or 13c's reasons
(`reference-…`) is counted by reason, and the keep/drop refusals by the
operand's shape read from the source line the leaf's `origin` names (the
pinned 66910c10 tree): the argument of the keep/drop macro or call on that
line is a member read (`->`/`.`), an address, a cast, or a plain name. Per
class: leaves, functions carrying one, and functions whose every leaf is of
that class (decided alone).
"""
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
REASON = re.compile(r"\((reference-[a-z-]+)\)")
CALLS = re.compile(
    r"\b(rd_kafka_toppar_destroy|rd_kafka_toppar_destroy0|rd_kafka_toppar_keep|rd_kafka_toppar_keep0|"
    r"rd_kafka_q_destroy|rd_kafka_q_destroy0|rd_kafka_q_destroy_owner|rd_kafka_q_keep|"
    r"rd_kafka_q_keep_nolock|rd_kafka_topic_keep|rd_kafka_topic_destroy0|rd_kafka_enq_once_trigger)\s*\(")
SOURCES = {}


def source_line(origin):
    if not origin or ":" not in origin:
        return None
    name, number = origin.rsplit(":", 1)
    if name not in SOURCES:
        found = list((ROOT / "lib/librdkafka-2.15.0/src").glob(name))
        SOURCES[name] = found[0].read_text(errors="replace").splitlines() if found else []
    lines = SOURCES[name]
    index = int(number) - 1
    return " ".join(lines[index:index + 3]) if 0 <= index < len(lines) else None


def operand(text):
    if text is None:
        return "unread"
    match = CALLS.search(text)
    if not match:
        return "unread"
    depth, start = 0, match.end()
    for at in range(start, len(text)):
        char = text[at]
        if char == "(":
            depth += 1
        elif char == ")":
            if depth == 0:
                arguments = text[start:at]
                break
            depth -= 1
    else:
        return "unread"
    parts, depth, current = [], 0, ""
    for char in arguments:
        if char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        if char == "," and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += char
    parts.append(current)
    actual = parts[-1].strip() if match.group(1) != "rd_kafka_q_destroy0" else parts[0].strip()
    if match.group(1) == "rd_kafka_enq_once_trigger":
        actual = parts[0].strip()
    if "->" in actual or "." in actual or "[" in actual:
        return "member"
    if actual.startswith("&"):
        return "address"
    if actual.startswith("("):
        return "cast"
    return "name"


def main() -> int:
    report = json.load(open(sys.argv[1]))
    leaves, carrying, alone = collections.Counter(), collections.Counter(), collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] == "Safe":
                continue
            classes = []
            for leaf in function.get("diagnostics") or []:
                match = REASON.search(leaf.get("description", ""))
                if not match:
                    classes.append(None)
                    continue
                reason = match.group(1)
                if reason in ("reference-drop-unheld", "reference-undecided"):
                    shape = operand(source_line(leaf.get("origin")))
                    if "kept into" in leaf.get("description", ""):
                        shape = "kept-into-place"
                    reason = "%s:%s" % (reason, shape)
                classes.append(reason)
            present = {c for c in classes if c}
            for reason in present:
                carrying[reason] += 1
            for reason in classes:
                if reason:
                    leaves[reason] += 1
            if present and all(classes) and len(present) == 1:
                alone[next(iter(present))] += 1
    record = {reason: {"leaves": leaves[reason], "functions": carrying[reason],
                       "alone": alone[reason]} for reason in sorted(leaves)}
    with open(sys.argv[2], "w") as out:
        out.write(json.dumps(record, indent=1) + "\n")
    for reason, row in record.items():
        print("  %-48s leaves %4d functions %4d alone %3d" % (
            reason, row["leaves"], row["functions"], row["alone"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
