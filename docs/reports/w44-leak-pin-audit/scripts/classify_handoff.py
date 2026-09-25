#!/usr/bin/env python3
"""Overlay the refused hand-on class on the frozen classification.

Usage: classify_handoff.py CLASSIFICATION.json HANDED_JOINED.json OUT.json OUT.md

CLASSIFICATION.json is the frozen per-leaf classification of the 271 baseline
leaves (`classify.py`, frozen with PREDICTION.md; not rewritten here).
HANDED_JOINED.json is `join_witnesses.py`'s output over the **rule-off**
census: the cast-return tree built with the refused hand-on rule's `apply`
disabled (evidence/census-rule-off.patch), whose diagnostics differ from the
baseline only by the cast-return leaf. Its `handed_path` is the rule's exact
population under the union join: a path from a producer of the leaked site to
a refused node whose statement names the site (as a store value, a `return`
operand, a call argument, an operand or its address `&x`, as the hand-on
reader sees it; the `withdraw` cascade over `contained` and `field_links`
included) and on to the exit over feasible edges.

Every baseline leaf gets one row:

* `mentioned`: a handed path exists in some joined invocation — the leaked
  identity is named in a refused statement on some path before the exit, as
  far as the classifier can see (a mention inside an unexpanded macro body the
  reader does not see does not count);
* `withdrawn`: the leaf is predicted withdrawn by the card — the cast-return
  leaf, or a handed path in **every** joined invocation;
* `attribution`: `b:cast-return`, `b:refused-escape` (the frozen class (b)
  refused-escape leaves), `h:refused-handoff` (every other withdrawn leaf, its
  frozen reading kept in `prior_class`/`prior_mechanism`), or the frozen
  `class:mechanism` for a leaf that stays.

Exits 1 when a baseline leaf is missing from the frozen classification or a
non-cast leaf is missing from the join.
"""

import collections
import json
import pathlib
import re
import sys


def acquired(description):
    match = re.search(r"acquired at (\S+)$", description)
    return match.group(1) if match else "?"


def how(step):
    if step.get("refused_read"):
        return "value"
    return "operand"


def main() -> int:
    frozen = json.loads(pathlib.Path(sys.argv[1]).read_text())["classification"]
    joined = json.loads(pathlib.Path(sys.argv[2]).read_text())["joined"]
    census = collections.defaultdict(list)
    for leaf in joined:
        census[(leaf["unit"], leaf["function"], leaf["line"], acquired(leaf["description"]))].append(leaf)
    failures = []
    rows = []
    for entry in frozen:
        key = (entry["unit"], entry["function"], entry["line"], entry["acquired"])
        row = {
            "library": entry["library"],
            "unit": entry["unit"],
            "function": entry["function"],
            "line": entry["line"],
            "exit": entry["exit"],
            "acquired": entry["acquired"],
            "prior_class": entry["class"],
            "prior_mechanism": entry["mechanism"],
        }
        if entry["mechanism"] == "cast-return":
            row.update(mentioned=None, withdrawn=True, attribution="b:cast-return", handing=[], reason=entry["reason"])
            rows.append(row)
            continue
        matches = census.get(key) or []
        if not matches:
            failures.append(f"{key}: not in the rule-off join")
            continue
        leaf = matches.pop(0)
        witness = leaf["witness"]
        some = witness["handed_path_in_some_invocation"]
        every = witness["handed_path_in_every_invocation"]
        handing = sorted(
            {
                f"{step.get('origin') or step['line']}: {step['instruction']} [{how(step)}]"
                for step in witness["handed_path"] or []
                if step["refused_call"]
            }
        )
        if every and entry["class"] == "b":
            attribution = f"b:{entry['mechanism']}"
        elif every:
            attribution = "h:refused-handoff"
        else:
            attribution = f"{entry['class']}:{entry['mechanism']}"
        row.update(
            mentioned=some,
            withdrawn=every,
            invocations_disagree=some != every,
            clean_path=witness["clean_path"] is not None,
            strict_path=witness["strict_path"] is not None,
            attribution=attribution,
            handing=handing,
            reason=entry["reason"],
        )
        rows.append(row)
    leftovers = [key for key, rest in census.items() if rest]
    failures += [f"{key}: joined but not classified" for key in leftovers]
    counts = collections.Counter(row["attribution"] for row in rows)
    prior = collections.Counter(
        f"{row['prior_class']}:{row['prior_mechanism']}" for row in rows if row["attribution"] == "h:refused-handoff"
    )
    mentioned_prior = collections.Counter(row["prior_class"] for row in rows if row.get("mentioned"))
    kinds = collections.Counter(
        "value" if any(h.endswith("[value]") for h in row["handing"]) else "operand"
        for row in rows
        if row["attribution"] == "h:refused-handoff"
    )
    document = {
        "leaves": len(rows),
        "mentioned": sum(1 for row in rows if row.get("mentioned")),
        "mentioned_by_prior_class": dict(sorted(mentioned_prior.items())),
        "withdrawn": sum(1 for row in rows if row["withdrawn"]),
        "invocations_disagree": sum(1 for row in rows if row.get("invocations_disagree")),
        "attribution": dict(sorted(counts.items())),
        "handoff_by_prior": dict(sorted(prior.items())),
        "handoff_by_hand_on": dict(sorted(kinds.items())),
        "failures": failures,
        "classification": rows,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(document, indent=1) + "\n")
    lines = [
        "# Refused hand-on overlay of the 271 baseline leak leaves",
        "",
        "Generated by `scripts/classify_handoff.py` from the frozen",
        "[classification.json](classification.json) and the rule-off census join. `Mentioned`:",
        "the leaked identity is named, as the hand-on reader sees it, in a refused statement on",
        "some path before the exit. `Withdrawn`: predicted withdrawn by the card. A withdrawn",
        "leaf is `Unknown` evidence, never `Safe`; a leaf that stays is provisional-`Unsafe`",
        "evidence, never an upstream defect claim.",
        "",
        "| # | Attribution | Prior | Function | Exit | Acquired | Mentioned | Withdrawn | Refused hand-on on the path |",
        "|---:|---|---|---|---|---|---|---|---|",
    ]
    for index, row in enumerate(rows, 1):
        handing = "; ".join(row["handing"][:2]).replace("|", "\\|")
        if len(row["handing"]) > 2:
            handing += f"; +{len(row['handing']) - 2}"
        mentioned = {None: "n/a", True: "yes", False: "no"}[row["mentioned"]]
        lines.append(
            f"| {index} | {row['attribution']} | {row['prior_class']}:{row['prior_mechanism']} | "
            f"`{row['function']}` | {row['exit']} | {row['acquired']} | {mentioned} | "
            f"{'yes' if row['withdrawn'] else 'no'} | {handing} |"
        )
    lines.append("")
    lines.append("Totals: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    pathlib.Path(sys.argv[4]).write_text("\n".join(lines) + "\n")
    print(json.dumps({k: v for k, v in document.items() if k != "classification"}, indent=1))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
