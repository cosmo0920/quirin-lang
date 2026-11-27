#!/usr/bin/env python3
"""CG00-CLANG-GAP-BASELINE: per-card function figures from the leaf attribution.

usage: functions.py DIAGNOSTICS LEAVES_JSONL_GZ OUT_JSON

For every Unknown function (keyed by unit file and name) the blockers are the
set of cards its unsupported leaves are attributed to, an `unattributed:*`
reason counting as a blocker of its own. A card **frees a function alone**
when the function's blockers are exactly that card; `strict` additionally
requires no `unknown_alias` leaf (an alias leaf may be poisoned by the
refusal and clear with it, rule 10a, or may stay: the unsupported-coverage
§5 upper-bound caveat), and no leak or summary conflict in either variant. A
card frees a function **with one other card** when its blockers are exactly
two cards. These are upper bounds: a modelled access can still fail its
obligations. Sums are reconciled to the arbiter's leaf and function totals,
and the script refuses (exit 3) when they do not reconcile.
"""

import collections
import gzip
import json
import sys


def main():
    diagnostics = json.load(open(sys.argv[1]))
    with gzip.open(sys.argv[2], "rt") as handle:
        rows = [json.loads(line) for line in handle][1:]
    out = sys.argv[3]

    status = {}
    alias = collections.Counter()
    blocked_other = collections.Counter()
    unsupported_total = 0
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            status[key] = function["status"]
            for leaf in function["diagnostics"]:
                if leaf["category"] == "unknown_alias":
                    alias[key] += 1
                elif leaf["category"] == "unsupported":
                    unsupported_total += 1
                else:
                    blocked_other[key] += 1
    if len(rows) != unsupported_total:
        print(f"leaf rows {len(rows)} != unsupported leaves {unsupported_total}")
        return 3

    blockers = collections.defaultdict(set)
    card_leaves = collections.Counter()
    entry_leaves = collections.Counter()
    entry_functions = collections.defaultdict(set)
    card_functions = collections.defaultdict(set)
    reason_leaves = collections.Counter()
    for row in rows:
        key = (row["file"], row["function"])
        blocker = row["card"] if row["card"] != "unattributed" else row["entry"]
        blockers[key].add(blocker)
        card_leaves[blocker] += 1
        entry_leaves[row["entry"]] += 1
        entry_functions[row["entry"]].add(key)
        card_functions[blocker].add(key)
        reason_leaves[(row["entry"], row["reason"].split(",")[0][:60])] += 1

    unknown = sorted(k for k, s in status.items() if s == "Unknown")
    alone = {"strict": collections.defaultdict(list), "with_alias": collections.defaultdict(list)}
    pair = collections.defaultdict(list)
    pair_partners = collections.defaultdict(collections.Counter)
    partition = collections.Counter()
    for key in unknown:
        cards = blockers.get(key, set())
        name = f"{key[0]}::{key[1]}"
        if not cards:
            partition["no unsupported leaf (alias, leak or conflict only)"] += 1
            continue
        if any(c.startswith("unattributed") for c in cards):
            partition["blocked by an unattributed leaf"] += 1
            continue
        if blocked_other[key]:
            partition["attributed, but a leak or summary conflict remains"] += 1
            continue
        if len(cards) == 1:
            partition["one card"] += 1
            card = next(iter(cards))
            alone["with_alias"][card].append(name)
            if not alias[key]:
                alone["strict"][card].append(name)
        elif len(cards) == 2:
            partition["two cards"] += 1
            left, right = sorted(cards)
            pair[left].append(name)
            pair[right].append(name)
            pair_partners[left][right] += 1
            pair_partners[right][left] += 1
        else:
            partition["three or more cards"] += 1
    if sum(partition.values()) != len(unknown):
        print("partition does not reconcile")
        return 3

    cards = sorted({c for c in card_leaves})
    table = []
    for card in cards:
        table.append({
            "card": card,
            "leaves": card_leaves[card],
            "functions_touched": len(card_functions[card]),
            "unknown_functions_touched": sum(1 for k in card_functions[card] if status[k] == "Unknown"),
            "freed_alone_with_alias": len(alone["with_alias"].get(card, [])),
            "freed_alone_strict": len(alone["strict"].get(card, [])),
            "freed_with_one_other_card": len(pair.get(card, [])),
            "top_partners": pair_partners[card].most_common(5),
        })
    phase = [r for r in table if r["card"].startswith("CG") and int(r["card"][2:]) <= 19]
    ranking = sorted(phase, key=lambda r: (-r["freed_alone_with_alias"], -r["freed_alone_strict"],
                                           -r["freed_with_one_other_card"], -r["leaves"], r["card"]))
    result = {
        "schema": "qcore.cg00-clang-gap-functions.v1",
        "unsupported_leaves": unsupported_total,
        "unknown_functions": len(unknown),
        "status": dict(collections.Counter(status.values())),
        "partition": dict(partition),
        "cards": table,
        "phase_1_3_ranking": [r["card"] for r in ranking],
        "entries": {e: {"leaves": entry_leaves[e], "functions": len(entry_functions[e])}
                    for e in sorted(entry_leaves)},
        "reasons": [{"entry": e, "reason": r, "leaves": n}
                    for (e, r), n in sorted(reason_leaves.items(), key=lambda kv: -kv[1])],
        "freed_alone": {variant: {card: sorted(names) for card, names in table_.items()}
                        for variant, table_ in alone.items()},
        "freed_with_one_other_card": {card: sorted(names) for card, names in pair.items()},
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, sort_keys=True)
        handle.write("\n")
    assert sum(card_leaves.values()) == unsupported_total
    print(f"leaves {unsupported_total}; Unknown {len(unknown)}; partition {dict(partition)}")
    for r in ranking:
        print(f"{r['card']}: leaves {r['leaves']}, fns {r['unknown_functions_touched']}, "
              f"alone {r['freed_alone_with_alias']}/{r['freed_alone_strict']}, pair {r['freed_with_one_other_card']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
