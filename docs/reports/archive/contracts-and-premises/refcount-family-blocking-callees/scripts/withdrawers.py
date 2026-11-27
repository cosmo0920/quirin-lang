#!/usr/bin/env python3
"""withdrawers.py REPORT OUT.json -- card REFCOUNT-FAMILY-BLOCKING-CALLEES,
task 1: which refused statement withdrew the reference family a
`reference-drop-unheld` drop (or a `reference-unbalanced` exit) needed.

Rule 13b withdraws a family when a refused statement may hand it on (the
refused hand-on rule, `legacy_refused_operands.rs`: a mention of the name as a
value, a call actual or an operand, never a member value read `p->f` nor the
destination of a simple assignment). This script reads that lexically, from
the pinned 66910c10 tree, per non-Safe function of REPORT:

* every `reference-drop-unheld` leaf whose dropped operand is a plain name
  (as origins.py reads it) gets its family's origin (origins.py's
  `origin_of`: the last assignment before the drop, or a formal), and its
  **withdrawers**: every refused leaf of the same function on a line from the
  origin's assignment line (inclusive: an access-gate refusal of the acquiring
  statement itself prevents the family) up to the drop line (exclusive) whose
  statement text hands on the name. The first one is the family's breaker.
* each withdrawer is named by its leaf: ``call to uncontracted function `F` ``
  gives `uncontracted:F`; any other refusal gives `KIND@G`, KIND the leaf
  family (access gate, unresolved identity, branch condition, nested call,
  cast, ...) and G the first called name of the statement (or `-`).

A `reference-unbalanced` leaf names no operand; it is attributed to the
families of the same function (it disappears with the refusal on its path),
and counted per function.

Per withdrawer: `families` (drops whose first breaker it is), `sole`
(drops whose every withdrawer is it), `decided` (functions whose every
plain-name unheld drop has only it as withdrawer and whose every other
reference leaf is unbalanced or an unheld drop: the reference route would
decide them if it were contracted), `safe` (of those, functions whose every
other leaf is ``call to uncontracted function `F` `` for the same F: a
function needing another unread actual or another uncontracted call is not
counted). Lexical and approximate: AFTER is the arbiter.
"""
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import origins  # noqa: E402

REASON = re.compile(r"\((reference-[a-z-]+)\)")
UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]*)`$")
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "do", "else"}
# Declared reference-returning calls and keeps (client.qv), by the spellings
# callers write (macros included). A family an origin outside this set never
# formed, so no refused statement is its breaker.
DECLARED = {
    "rd_kafka_toppar_get", "rd_kafka_toppar_get0", "rd_kafka_toppar_desired_get",
    "rd_kafka_toppar_new", "rd_kafka_toppar_new0", "rd_kafka_toppar_desired_add",
    "rd_kafka_toppar_get2", "rd_kafka_toppar_get_avail", "rd_kafka_q_new",
    "rd_kafka_consume_q_new", "rd_kafka_q_new0", "ut_create_mock_toppar",
    "ut_create_mock_queue", "rd_kafka_toppar_keep", "rd_kafka_toppar_keep0",
    "rd_kafka_toppar_keep_fl", "rd_kafka_q_keep", "rd_kafka_q_keep_nolock",
    "rd_kafka_topic_keep",
}


def family_of(description):
    if UNCONTRACTED.match(description):
        return "uncontracted"
    reason = REASON.search(description)
    if reason:
        return reason.group(1)
    text = description.split("`")[0].strip()
    return text[:60]


def statement(lines, index):
    """The statement starting at INDEX: up to the first `;` or `{` at depth
    zero, at most eight lines."""
    text, depth = "", 0
    for at in range(index, min(index + 8, len(lines))):
        for char in lines[at]:
            text += char
            if char in "([":
                depth += 1
            elif char in ")]":
                depth -= 1
            elif char in ";{" and depth <= 0:
                return text
        text += " "
    return text


def hands_on(text, name):
    """Whether TEXT names NAME as a value, an actual or an operand (not a
    member value read, not a plain assignment's destination, not sizeof)."""
    for match in re.finditer(r"(?<![\w.>])" + re.escape(name) + r"\b", text):
        after = text[match.end():].lstrip()
        before = text[:match.start()].rstrip()
        if after.startswith("->") or after.startswith(".") or after.startswith("["):
            if not before.endswith("&"):
                continue
        if re.match(r"=(?!=)", after) and not re.search(r"[=!<>]$", before):
            continue
        if before.endswith("sizeof") or before.endswith("sizeof("):
            continue
        return True
    return False


def first_call(text):
    for match in CALL.finditer(text):
        if match.group(1) not in KEYWORDS:
            return match.group(1)
    return "-"


def main():
    report = json.load(open(sys.argv[1]))
    drops, per_function = [], {}
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] == "Safe":
                continue
            leaves = function.get("diagnostics") or []
            if not any(REASON.search(l.get("description", "")) for l in leaves):
                continue
            key = "%s:%s" % (unit["file"], function["function"])
            record = {"drops": [], "unbalanced": 0, "other_reference": [], "others": []}
            refused = []
            for leaf in leaves:
                description = leaf.get("description", "")
                origin = leaf.get("origin") or ""
                if ":" in origin:
                    file, number = origin.rsplit(":", 1)
                    refused.append((file, int(number), description))
            for leaf in leaves:
                description = leaf.get("description", "")
                reason = REASON.search(description)
                origin = leaf.get("origin") or ""
                if reason and reason.group(1) == "reference-unbalanced":
                    record["unbalanced"] += 1
                    continue
                if not reason or reason.group(1) != "reference-drop-unheld":
                    if reason:
                        record["other_reference"].append(reason.group(1))
                    else:
                        record["others"].append(description)
                    continue
                if ":" not in origin:
                    record["drops"].append({"origin": origin, "shape": "unread"})
                    continue
                file, number = origin.rsplit(":", 1)
                lines = origins.lines_of(file)
                index = int(number) - 1
                text = " ".join(lines[index:index + 3]) if 0 <= index < len(lines) else ""
                name = origins.dropped_name(text)
                if name is None:
                    record["drops"].append({"origin": origin, "shape": "not-a-name"})
                    continue
                kind, rhs = origins.origin_of(lines, function["function"], index, name)
                callee = kind[5:] if kind.startswith("call:") else None
                declared = callee in DECLARED
                # the origin's assignment line: the last line before the drop
                # assigning the name (formal: the function's start)
                start = origins.definition_start(lines, function["function"], index) or 0
                assign = re.compile(r"(?<![\w>.])" + re.escape(name) + r"\s*=(?!=)")
                origin_line = start
                for at in range(start, index):
                    if assign.search(lines[at]):
                        origin_line = at
                withdrawers = []
                for leaf_file, leaf_line, leaf_description in sorted(refused, key=lambda r: r[1]):
                    if leaf_file != file or not (origin_line + 1 <= leaf_line < index + 1):
                        continue
                    if REASON.search(leaf_description) and "reference-" in leaf_description and \
                            "reference-undecided" not in leaf_description:
                        continue
                    body = statement(lines, leaf_line - 1)
                    if leaf_line - 1 == origin_line:
                        # the acquiring statement itself: refused means the
                        # family never formed
                        hands = True
                    else:
                        hands = hands_on(body, name)
                    if not hands:
                        continue
                    match = UNCONTRACTED.match(leaf_description)
                    label = ("uncontracted:" + match.group(1)) if match else \
                        "%s@%s" % (family_of(leaf_description), first_call(body))
                    withdrawers.append({"line": leaf_line, "label": label,
                                        "acquiring": leaf_line - 1 == origin_line})
                record["drops"].append({"origin": origin, "name": name, "kind": kind,
                                        "declared": declared, "withdrawers": withdrawers})
            per_function[key] = record
    # per withdrawer
    families, sole = collections.Counter(), collections.Counter()
    decided, safe = collections.defaultdict(list), collections.defaultdict(list)
    shapes = collections.Counter()
    for key, record in per_function.items():
        named = [d for d in record["drops"] if "name" in d]
        for drop in record["drops"]:
            if "name" not in drop:
                shapes[drop["shape"]] += 1
                continue
            if not drop["declared"]:
                shapes["origin-undeclared:" + drop["kind"].split(":")[0]] += 1
                continue
            if not drop["withdrawers"]:
                shapes["declared-no-withdrawer"] += 1
                continue
            shapes["declared-withdrawn"] += 1
            families[drop["withdrawers"][0]["label"]] += 1
            labels = {w["label"] for w in drop["withdrawers"]}
            if len(labels) == 1:
                sole[next(iter(labels))] += 1
        if not named or len(named) != len(record["drops"]) or record["other_reference"]:
            continue
        labels = set()
        ok = True
        for drop in named:
            if not drop["declared"] or not drop["withdrawers"]:
                ok = False
                break
            labels |= {w["label"] for w in drop["withdrawers"]}
        if not ok or len(labels) != 1:
            continue
        label = next(iter(labels))
        decided[label].append(key)
        if label.startswith("uncontracted:"):
            callee = label[len("uncontracted:"):]
            if all(UNCONTRACTED.match(o) and UNCONTRACTED.match(o).group(1) == callee
                   for o in record["others"]):
                safe[label].append(key)
    rows = []
    for label in sorted(set(families) | set(decided),
                        key=lambda l: (-families[l], -len(decided[l]), l)):
        rows.append({"withdrawer": label, "families": families[label], "sole": sole[label],
                     "decided": len(decided[label]), "safe": len(safe[label]),
                     "decided_functions": sorted(decided[label]),
                     "safe_functions": sorted(safe[label])})
    out = {"report": sys.argv[1], "shapes": dict(shapes), "ranking": rows,
           "functions": per_function}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    total = sum(len(r["drops"]) for r in per_function.values())
    print("functions with a reference leaf %d; unheld drops %d; unbalanced %d" % (
        len(per_function), total, sum(r["unbalanced"] for r in per_function.values())))
    for shape, count in shapes.most_common():
        print("  %-44s %4d" % (shape, count))
    print("  %-60s %4s %4s %4s %4s" % ("withdrawer", "fam", "sole", "dec", "safe"))
    for row in rows[:60]:
        print("  %-60s %4d %4d %4d %4d" % (row["withdrawer"][:60], row["families"], row["sole"],
                                          row["decided"], row["safe"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
