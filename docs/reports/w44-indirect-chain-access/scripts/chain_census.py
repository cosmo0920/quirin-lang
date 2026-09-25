#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: split the member-chain family and forecast the rule.

Input: one census run (`run_audit.py OUT COMMIT --census --binary <census build>`)
holding `resource-diagnostics.json`, `statement-census.jsonl`,
`traversal-census.jsonl` and `chain-census.jsonl`. The chain stream carries, for
every refused statement, `for` clause and controlling expression that selects a
member, the answer of the very recogniser the rule calls
(`indirect_chain_access::chain_statement` / `chain_condition`) and every
chain-shaped access the chain reader met, with its refusal reason, depth, root
and final member.

Join (W43-INDIRECT-ACCESS-GENERAL's): `unit_seq` is matched to a document unit
by greatest overlap of (function, line) keys over all three streams. A leaf of
an Unknown function is explained by every distinct chain record at its
(function, line) of the matching kind: a statement record whose outcome is the
leaf's description, or a condition record at the leaf's position.

The **chain family** is every such leaf one of whose records carries a
chain-shaped access (an `ok` parse, or a refusal other than `not-a-chain`) or
a subscript followed by a member. Each leaf is split along four axes, taken from
the hardest record at the key:

* `kind` - `store`, `compound`, `increment`, `read`, `declaration`,
  `condition:<position>` or `other`;
* `depth` - the largest number of member selections in one access;
* `root` - `formal` / `local` / `undeclared`, crossed with `never-bound` or
  `may-bind` (card C02-P premise 4, `UntrackedBases::never_bound`);
* `reader` - `admitted` (the recogniser admits every record at the key and the
  route is consulted for it: a consulted refusal, no call, no brace), or the
  first reason it does not: `call`, `not-consulted`, a chain reader refusal
  (`dot-link`, `subscript`, `void-link`, `untyped-pointer-link`,
  `non-pointer-link`, `incomplete-record`, `root-undeclared`, `root-untyped`,
  `depth-limit`, `call`, `malformed`), `pointer-member`, `other-member`, or
  `operand` (every chain parses, but another operand of the statement is
  outside the typed walker).

Forecast. An admitted leaf whose every chain root is never bound passes the
untracked arm of the obligation unless the root's binding at the statement is
possibly null: it is forecast `clear` (an upper bound; the flow half may
exchange it for `unknown_alias`). An admitted leaf with a may-bind root needs
one live identity and a bound member place: it is forecast `flow` (at most
cleared, expected mostly exchanged). A function is forecast Unknown -> Safe when
every one of its leaves is forecast `clear`.

Usage: chain_census.py RUN_DIR OUT_JSON OUT_TXT
"""

import collections
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "w43-indirect-access-general" / "scripts"))

import shape_census as census  # noqa: E402


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


w43_residuals = _load("w43_residuals", HERE.parents[1] / "w43-indirect-access-general" / "scripts" / "residuals.py")
w43_predict = _load("w43_predict", HERE.parents[1] / "w43-indirect-access-general" / "scripts" / "predict.py")
w43_traversal = _load("w43_traversal", HERE.parents[1] / "w43-index-traversal" / "scripts" / "residuals.py")
traversal_predict = _load("traversal_predict", HERE.parents[1] / "w43-index-traversal" / "scripts" / "predict.py")

CONDITION = re.compile(
    r"unmodeled call, aggregate or indirect access in "
    r"(branch condition|loop condition|for-loop condition|do-while condition|switch expression)"
)
POSITION = {
    "branch condition": {"if"},
    "loop condition": {"while"},
    "for-loop condition": {"for"},
    "do-while condition": {"do-while"},
    "switch expression": {"switch"},
}
CHAIN_REFUSALS = {
    "dot-link",
    "subscript",
    "void-link",
    "untyped-pointer-link",
    "non-pointer-link",
    "incomplete-record",
    "root-undeclared",
    "root-untyped",
    "depth-limit",
    "call",
    "malformed",
}


def load_chain_stream(path):
    stream = collections.defaultdict(lambda: collections.defaultdict(list))
    seen = set()
    for line in open(path):
        record = json.loads(line)
        blob = line.strip()
        if blob in seen:
            continue
        seen.add(blob)
        seq = record.pop("unit_seq")
        stream[seq][(record["function"], record["line"])].append(record)
    return stream


def load_keys(path):
    keys = collections.defaultdict(set)
    if not path.exists():
        return keys
    for line in open(path):
        record = json.loads(line)
        if "function" in record and "line" in record:
            keys[record["unit_seq"]].add((record["function"], record["line"]))
    return keys


def access_paths(texts):
    """Every access path of a token run, read from tokens only.

    A path starts at a name that is not itself a selected member (not after
    `.` or `->`), or at `( * NAME ) .`, and continues through `.` NAME,
    `->` NAME and `[ ... ]` groups. Yields (start, end, selections, members,
    subscripted), where a subscript group counts as a selection, so `a[i].f`
    has two selections as W43's chain class reads it. Names inside a
    subscript are scanned as roots of their own paths.
    """
    paths = []
    for index, text in enumerate(texts):
        after_member = index >= 1 and (
            texts[index - 1] == "." or (texts[index - 1] == ">" and index >= 2 and texts[index - 2] == "-")
        )
        if texts[index : index + 2] == ["(", "*"] and index + 4 < len(texts) and census.is_name(
            texts[index + 2]
        ) and texts[index + 3] == ")" and texts[index + 4] == ".":
            cursor = index + 4
        elif census.is_name(text) and not after_member:
            cursor = index + 1
        else:
            continue
        selections = members = 0
        subscripted = False
        while cursor < len(texts):
            if texts[cursor] == "[":
                level = 0
                while cursor < len(texts):
                    if texts[cursor] == "[":
                        level += 1
                    elif texts[cursor] == "]":
                        level -= 1
                        if level == 0:
                            break
                    cursor += 1
                cursor += 1
                selections += 1
                subscripted = True
                continue
            if texts[cursor] == "." and cursor + 1 < len(texts) and census.is_name(texts[cursor + 1]):
                selections += 1
                members += 1
                cursor += 2
                continue
            if texts[cursor : cursor + 2] == ["-", ">"] and cursor + 2 < len(texts):
                selections += 1
                members += 1
                cursor += 3
                continue
            break
        if members:
            paths.append((index, cursor, selections, members, subscripted))
    return paths


def subscript_member(texts):
    return any(
        texts[i] == "]" and (texts[i + 1] == "." or texts[i + 1 : i + 3] == ["-", ">"])
        for i in range(len(texts) - 2)
    )


def statement_kind(record):
    if record["kind"] == "condition":
        return "condition:" + record["position"]
    texts = record["texts"]
    equal, operator = census.top_level_assignment(texts)
    if equal is not None:
        left = texts[: equal - len(operator)]
        chained_left = any(
            occurrence["index"] < equal - len(operator) for occurrence in record["occurrences"]
        ) or subscript_member(left)
        if chained_left:
            return "compound" if operator else "store"
        if len(left) >= 2 and all(census.is_name(t) or t == "*" for t in left):
            return "declaration"
        return "read"
    if census.has_increment(texts):
        return "increment"
    return "other"


def record_axes(record):
    """The axes of one census record; see the module docstring.

    Only a path of two or more selections is a chain (W43's class: a member
    chain of depth two or more, or a subscript followed by a member). The
    Rust reader's answer is attached by the path's start index; a path it did
    not record starts at a subscripted root, which it refuses as `subscript`.
    """
    texts = record["texts"]
    by_index = {o["index"]: o for o in record["occurrences"] if o["result"] != "not-a-chain"}
    chains = []
    for start, end, selections, members, subscripted in access_paths(texts):
        if selections < 2:
            continue
        occurrence = by_index.get(start)
        if occurrence is None:
            occurrence = {
                "index": start, "result": "subscript", "selections": None,
                "root": texts[start], "root_kind": "subscript-root", "root_type": "none",
                "root_never_bound": False, "root_may_bind": True, "member_kind": None,
            }
        elif subscripted and occurrence["result"] == "ok":
            occurrence = dict(occurrence, result="subscript")
        chains.append((occurrence, selections, members))
    depth = max((members for _, _, members in chains), default=0)
    roots = {(o["root_kind"], o["root_never_bound"]) for o, _, _ in chains}
    if not roots:
        root = "no-chain"
    else:
        kinds = sorted({kind for kind, _ in roots})
        bound = "never-bound" if all(nb for _, nb in roots) else "may-bind"
        root = "+".join(kinds) + ":" + bound
    if not chains:
        reader = "no-member-chain"
    elif record["admitted"] is not None and record["consulted"] and not record["contains_call"]:
        reader = "admitted"
    elif record["contains_call"]:
        reader = "call"
    elif not record["consulted"]:
        reader = "not-consulted"
    else:
        refused = [o["result"] for o, _, _ in chains if o["result"] != "ok"]
        if refused:
            reader = refused[0]
        elif any(o["member_kind"] == "pointer" for o, _, _ in chains):
            reader = "pointer-member"
        elif any(o["member_kind"] == "other" for o, _, _ in chains):
            reader = "other-member"
        else:
            reader = "operand"
    return {
        "kind": statement_kind(record),
        "depth": depth,
        "root": root,
        "reader": reader,
        "is_chain": bool(chains),
        "never_bound_roots": all(
            chain["root_never_bound"] for chain in (record["admitted"] or [])
        ),
    }


def main():
    run = pathlib.Path(sys.argv[1])
    out_json, out_txt = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    chains = load_chain_stream(run / "chain-census.jsonl")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    conditions = collections.defaultdict(lambda: collections.defaultdict(list))
    for seq, records in traversal_predict.load(run / "traversal-census.jsonl").items():
        for record in records:
            if record["kind"] == "condition":
                conditions[seq][(record["function"], record["line"])].append(record)
    streams = [
        indirect,
        statements,
        {seq: {(f, l, None): [] for f, l in found} for seq, found in conditions.items()},
        {seq: {(f, l, None): [] for f, l in found} for seq, found in chains.items()},
    ]
    assignment = census.match_units(streams, unit_keys)

    leaves = {}
    unexplained = collections.Counter()
    for fkey, data in functions.items():
        if data["status"] != "Unknown":
            continue
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            match = CONDITION.fullmatch(description)
            if match:
                found = [
                    r for r in conditions.get(seq, {}).get((fkey[2], line), [])
                    if r["position"] in POSITION[match.group(1)]
                ]
                if not found or w43_traversal.condition_class(found[0]["texts"]) != "chain":
                    continue
                family = "condition"
            elif w43_predict.intercepted(description):
                key = (fkey[2], line, description)
                found = statements.get(seq, {}).get(key, [])
                if description == census.INDIRECT:
                    found = indirect.get(seq, {}).get(key, []) or found
                classes = {w43_residuals.classify(r) for r in found} or {"other"}
                if min(classes, key=w43_residuals.ORDER.index) != "chain":
                    continue
                family = "statement"
            else:
                continue
            records = chains.get(seq, {}).get((fkey[2], line), []) if seq is not None else []
            if match:
                records = [
                    r for r in records
                    if r["kind"] == "condition" and r["position"] in POSITION[match.group(1)]
                ]
            else:
                records = [
                    r for r in records
                    if r["kind"] == "statement"
                    and census.outcome_family(r["outcome"]) == description
                ]
            axes = [record_axes(r) for r in records]
            if not axes:
                # No record of this card's census at the key. W43's place
                # reader classes every multi-token place that starts with a
                # name as `chain`, so a typed declaration (`T * x = e`,
                # `size_t n = e`) lands in the class with no member selection
                # at all; this card's census records only runs that select a
                # member. Split the two so the ledger size is not misread.
                selecting = [
                    r for r in found
                    if w43_residuals.classify(r) == "chain"
                    and any(
                        t == "." or (t == "-" and i + 1 < len(r["texts"]) and r["texts"][i + 1] == ">")
                        for i, t in enumerate(r["texts"])
                    )
                ]
                label = "unexplained" if selecting else "declarator-not-a-chain"
                unexplained[label] += 1
                axes = [{
                    "kind": label, "depth": 0, "root": label,
                    "reader": label, "is_chain": True, "never_bound_roots": False,
                }]
            order = [
                "declarator-not-a-chain", "unexplained", "no-member-chain", "call", "not-consulted",
            ] + sorted(CHAIN_REFUSALS) + ["pointer-member", "other-member", "operand", "admitted"]
            chain_axes = [a for a in axes if a["is_chain"]] or axes
            hardest = min(chain_axes, key=lambda a: order.index(a["reader"]))
            admitted = all(a["reader"] == "admitted" for a in axes)
            forecast = (
                ("clear" if all(a["never_bound_roots"] for a in axes) else "flow")
                if admitted
                else "refused"
            )
            leaves[(fkey, line, description)] = {
                "family": family,
                "kind": hardest["kind"],
                "depth": max(a["depth"] for a in axes),
                "root": hardest["root"],
                "reader": "admitted" if admitted else hardest["reader"],
                "forecast": forecast,
            }

    # The rule's whole reach, in every status and every family: a leaf whose
    # every chain-census record at its key is admitted leaves its family
    # (removed, or exchanged for the flow half's `unknown_alias`).
    reach = collections.Counter()
    reach_leaves = {}
    for fkey, data in functions.items():
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            match = CONDITION.fullmatch(description)
            records = chains.get(seq, {}).get((fkey[2], line), []) if seq is not None else []
            if match:
                records = [
                    r for r in records
                    if r["kind"] == "condition" and r["position"] in POSITION[match.group(1)]
                ]
            else:
                records = [
                    r for r in records
                    if r["kind"] == "statement"
                    and census.outcome_family(r["outcome"]) == description
                ]
            if not records:
                continue
            axes = [record_axes(r) for r in records]
            if not all(a["reader"] == "admitted" for a in axes):
                continue
            arm = "clear" if all(a["never_bound_roots"] for a in axes) else "flow"
            in_family = (fkey, line, description) in leaves
            reach[(data["status"], "condition" if match else census.normalize(description), arm,
                   "chain-family" if in_family else "outside-family")] += 1
            reach_leaves["/".join(str(p) for p in fkey) + f":{line}:{description}"] = arm

    unknown = {f for f, d in functions.items() if d["status"] == "Unknown"}
    family_functions = {f for (f, _, _) in leaves}

    def sole(fkey, description):
        """W43's reading: the leaf's own family is the function's only one."""
        families = {census.normalize(d) for _, d in functions[fkey]["leaves"]}
        return families == {census.normalize(description)}

    def chain_only(fkey):
        return all((fkey, line, d) in leaves for line, d in functions[fkey]["leaves"])

    def table(axis):
        rows = collections.defaultdict(lambda: [0, set(), set()])
        for (fkey, line, description), leaf in leaves.items():
            key = tuple(leaf[a] for a in axis) if isinstance(axis, tuple) else leaf[axis]
            row = rows[key]
            row[0] += 1
            row[1].add(fkey)
            if sole(fkey, description):
                row[2].add(fkey)
        return {
            (" / ".join(map(str, key)) if isinstance(key, tuple) else key): {
                "leaves": n, "functions": len(fs), "sole": len(ss)
            }
            for key, (n, fs, ss) in sorted(rows.items(), key=lambda kv: -kv[1][0])
        }

    decided_clear = sorted(
        f for f in unknown
        if all(leaves.get((f, l, d), {}).get("forecast") == "clear" for l, d in functions[f]["leaves"])
    )
    decided_flow = sorted(
        f for f in unknown
        if all(
            leaves.get((f, l, d), {}).get("forecast") in ("clear", "flow")
            for l, d in functions[f]["leaves"]
        )
        and f not in decided_clear
    )
    result = {
        "family": {
            "leaves": len(leaves),
            "functions": len(family_functions),
            "sole": len({f for (f, _, d) in leaves if sole(f, d)}),
            "chain_only_functions": sum(1 for f in family_functions if chain_only(f)),
            "unexplained_leaves": dict(unexplained),
        },
        "by_family": table("family"),
        "by_kind": table("kind"),
        "by_depth": table("depth"),
        "by_root": table("root"),
        "by_reader": table("reader"),
        "by_forecast": table("forecast"),
        "by_family_reader": table(("family", "reader")),
        "by_kind_reader": table(("kind", "reader")),
        "by_depth_reader": table(("depth", "reader")),
        "by_root_reader": table(("root", "reader")),
        "forecast_functions_clear": ["/".join(str(p) for p in f) for f in decided_clear],
        "forecast_functions_flow": ["/".join(str(p) for p in f) for f in decided_flow],
        "reach_all_statuses": {" / ".join(key): n for key, n in sorted(reach.items())},
        "reach_leaves": reach_leaves,
        "forecast_leaves": {
            "/".join(str(p) for p in fkey) + f":{line}:{description}": leaf["forecast"]
            for (fkey, line, description), leaf in sorted(leaves.items(), key=str)
            if leaf["forecast"] != "refused"
        },
    }
    out_json.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    lines = [
        "W44-INDIRECT-CHAIN-ACCESS member-chain family (leaves in Unknown functions)",
        "",
        f"family: {result['family']['leaves']} leaves, {result['family']['functions']} functions, "
        f"{result['family']['sole']} sole-blocked (W43's reading), "
        f"{result['family']['chain_only_functions']} whose every leaf is in the family; "
        f"unexplained {result['family']['unexplained_leaves']}",
        "",
    ]
    for axis in (
        "by_family", "by_kind", "by_depth", "by_root", "by_reader", "by_forecast",
        "by_family_reader", "by_kind_reader", "by_depth_reader", "by_root_reader",
    ):
        lines.append(f"{axis}: key leaves functions sole")
        for key, row in result[axis].items():
            lines.append(f"  {str(key):56} {row['leaves']:6} {row['functions']:6} {row['sole']:5}")
        lines.append("")
    lines.append("reach in every status: status / family / arm / in the chain family: leaves")
    for key, n in result["reach_all_statuses"].items():
        lines.append(f"  {key:88} {n:5}")
    lines.append("")
    lines.append(f"forecast Unknown -> Safe (every leaf clear): {len(decided_clear)}")
    lines.extend("  " + name for name in result["forecast_functions_clear"])
    lines.append(f"forecast Unknown -> Safe only if flow-dependent leaves admit: {len(decided_flow)}")
    lines.extend("  " + name for name in result["forecast_functions_flow"])
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
