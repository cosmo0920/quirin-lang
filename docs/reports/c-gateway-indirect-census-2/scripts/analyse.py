#!/usr/bin/env python3
"""Card C-GATEWAY-INDIRECT-CENSUS-2: the census of the corpus's largest refusal
family, read from the analyzer's own projection.

Input is the instrumented run's `indirect-leaves.jsonl` — one record per refused
*statement*, written at the moment the classifier committed to refusing — and
the same run's `resource-diagnostics.json`, which is the authority for which
*leaves* exist and for every function's other blockers.

Three facts about the join, each measured rather than assumed, and each making
the answer more conservative than a naive one:

  1. `unit_seq` is a genuine translation-unit ordinal (the census records 171
     distinct values against the document's 171 leaf-carrying units), so the
     join is unit-exact and the handful of C functions whose names recur across
     units do not have to be guessed at.
  2. A published leaf's identity is `(unit, function, preprocessed line)`, and
     ONE preprocessed line can carry SEVERAL refused statements — that is what
     macro expansion does. A leaf is therefore explained by a *set* of
     statements, and it is cleared only if every one of them is admitted.
  3. W4 validates a derived summary by re-analysis (constraint 4), so the same
     statement is classified several times. Byte-identical repeats are one
     statement; only distinct records are distinct statements.

Usage: python3 analyse.py <run-dir> <out-dir>
"""
import collections
import json
import pathlib
import re
import sys

INDIRECT = "indirect memory access is outside the resource projection"
STEP = {"Arrow": "->", "Dot": ".", "Subscript": "[]", "Deref": "*"}


def normalize(description):
    """A blocker's family, with the site-specific names abstracted away."""
    return re.sub(r"`[^`]*`", "`X`", description)


def load_document(path):
    doc = json.load(open(path))
    blockers = {}
    unit_leaves = collections.defaultdict(set)
    leaf_functions = {}
    for unit in doc["units"]:
        ukey = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            fkey = ukey + (function["function"],)
            blockers[fkey] = [normalize(d["description"]) for d in function["diagnostics"]]
            for d in function["diagnostics"]:
                if d["description"] == INDIRECT:
                    unit_leaves[ukey].add((function["function"], d["line"]))
                    leaf_functions[ukey + (function["function"], d["line"])] = fkey
    return blockers, dict(unit_leaves), leaf_functions


def load_census(path):
    """unit_seq -> {(function, line) -> [distinct records]}"""
    census = collections.defaultdict(lambda: collections.defaultdict(list))
    seen = collections.defaultdict(set)
    total = 0
    for line in open(path):
        r = json.loads(line)
        total += 1
        seq = r.pop("unit_seq")
        key = (r["function"], r["line"])
        blob = json.dumps(r, sort_keys=True)
        if blob in seen[(seq, key)]:
            continue
        seen[(seq, key)].add(blob)
        census[seq][key].append(r)
    return census, total


def match_units(census, unit_leaves):
    """Assign each document unit the census unit_seq that explains it.

    Greedy on overlap, then checked: the assignment must be injective and must
    cover every leaf-carrying unit, or the mismatch is reported rather than
    papered over.
    """
    scores = []
    for ukey, keys in unit_leaves.items():
        for seq, records in census.items():
            overlap = len(keys & set(records))
            if overlap:
                scores.append((overlap, ukey, seq))
    scores.sort(reverse=True, key=lambda row: row[0])
    assigned, used = {}, set()
    for overlap, ukey, seq in scores:
        if ukey in assigned or seq in used:
            continue
        assigned[ukey] = seq
        used.add(seq)
    return assigned


def shape(record):
    """The shape an arbiter would have to legislate for, for ONE statement.

    Coarse where an arbiter cannot act and exact where it must: the deciding
    chain's base kind, whether this function may bind that base to a resource
    identity (constraint 5's premise), the chain's step spelling, the final
    member's class, and the syntactic position.
    """
    chains = record["chains"]
    if not chains:
        return ("no-chain", "-", "-", "-", record["position"])
    # Where a statement carries several chains every one must be admitted, so
    # the hardest decides: a may-bind base first, then the deeper chain.
    worst = max(chains, key=lambda c: (c["base_may_bind"], c["depth"]))
    return (
        worst["base_kind"],
        "may-bind" if worst["base_may_bind"] else "never-bound",
        "".join(STEP[s] for s in worst["steps"]) or "-",
        worst["member_class"],
        record["position"],
    )


def main():
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, streamed = load_census(run / "indirect-leaves.jsonl")
    assignment = match_units(census, unit_leaves)

    report = []

    def say(text=""):
        print(text)
        report.append(text)

    def table(title, counter, limit=None):
        say()
        say(title)
        total = sum(counter.values()) or 1
        rows = counter.most_common(limit)
        for name, count in rows:
            say("   %8d  %5.1f%%  %s" % (count, 100.0 * count / total, name))
        if limit and len(counter) > limit:
            rest = total - sum(c for _, c in rows)
            say("   %8d  %5.1f%%  (%d further values)"
                % (rest, 100.0 * rest / total, len(counter) - limit))

    # ------------------------------------------------------------ provenance
    say("=" * 78)
    say("CENSUS PROVENANCE AND JOIN")
    say("=" * 78)
    say("statement records streamed by the instrumented analyzer: %d" % streamed)
    say("distinct translation-unit ordinals in the stream:        %d" % len(census))
    say("leaf-carrying units in the published document:           %d" % len(unit_leaves))
    say("units the join assigned:                                 %d" % len(assignment))
    unassigned = sorted(set(unit_leaves) - set(assignment))
    say("units the join could NOT assign:                         %d" % len(unassigned))
    for ukey in unassigned[:10]:
        say("   %s" % "/".join(str(x) for x in ukey))

    # Every published leaf, with the statements that explain it.
    statements = {}
    for ukey, keys in unit_leaves.items():
        seq = assignment.get(ukey)
        records = census.get(seq, {}) if seq is not None else {}
        for key in keys:
            statements[ukey + key] = records.get(key, [])
    total_leaves = len(statements)
    covered = sum(1 for v in statements.values() if v)
    say()
    say("published `%s` leaves: %d" % (INDIRECT, total_leaves))
    say("  explained by at least one census statement: %d  (%.2f%%)"
        % (covered, 100.0 * covered / total_leaves))
    say("  UNEXPLAINED:                                %d  (%.2f%%)"
        % (total_leaves - covered, 100.0 * (total_leaves - covered) / total_leaves))
    multi = sum(1 for v in statements.values() if len(v) > 1)
    say("  leaves covering MORE THAN ONE refused statement: %d  (%.1f%%)"
        % (multi, 100.0 * multi / total_leaves))
    say("    (one preprocessed line, several statements: macro expansion.")
    say("     Such a leaf clears only if every one of them is admitted.)")
    widest = max(statements.items(), key=lambda kv: len(kv[1]))
    say("  widest leaf: %d statements at %s"
        % (len(widest[1]), "/".join(str(x) for x in widest[0])))
    functions = sorted({leaf_functions[k] for k in statements})
    say("functions carrying at least one such leaf: %d" % len(functions))
    say("units carrying at least one:               %d" % len(unit_leaves))

    # ----------------------------------------------------------------- axes
    say()
    say("=" * 78)
    say("DISTRIBUTION OVER THE CENSUS AXES")
    say("  Counted per LEAF (the published unit of refusal). A leaf covering")
    say("  several statements contributes its hardest one, because that is the")
    say("  one an arbiter would have to clear.")
    say("=" * 78)

    def hardest(records):
        if not records:
            return None
        return max(records, key=lambda r: shape(r))

    def leafwise(extract):
        counter = collections.Counter()
        for records in statements.values():
            record = hardest(records)
            if record is None:
                continue
            counter[extract(record)] += 1
        return counter

    def worst_of(record):
        if not record["chains"]:
            return None
        return max(record["chains"], key=lambda c: (c["base_may_bind"], c["depth"]))

    def chainwise(extract, default="no-chain"):
        counter = collections.Counter()
        for records in statements.values():
            record = hardest(records)
            if record is None:
                continue
            chain = worst_of(record)
            counter[default if chain is None else extract(chain)] += 1
        return counter

    table("refusal site:", leafwise(lambda r: r["site"]))
    table("syntactic position (which side of the gate fired):",
          leafwise(lambda r: r["position"]))
    table("the refused statement contains a call:",
          leafwise(lambda r: str(r["contains_call"])))
    table("access chains in the refused statement:",
          leafwise(lambda r: str(min(len(r["chains"]), 5))))
    table("base kind of the deciding chain:", chainwise(lambda c: c["base_kind"]))
    table("CONSTRAINT 5's PREMISE — may this function bind that base to a "
          "resource identity?",
          chainwise(lambda c: "may-bind" if c["base_may_bind"] else "NEVER-BOUND"))
    table("UntrackedBases::never_bound (the admission rules' own reading, which "
          "also requires the declaration reader to have succeeded):",
          chainwise(lambda c: "never-bound" if c["base_never_bound"]
                    else "risky-or-unreadable"))
    table("chain depth:", chainwise(lambda c: str(c["depth"])))
    table("step spelling:",
          chainwise(lambda c: "".join(STEP[s] for s in c["steps"]) or "-"), limit=14)
    table("final member class:", chainwise(lambda c: c["member_class"]))

    # The two implementations of constraint 5's premise do not agree, and which
    # one a wave-37 arbiter reads changes the admission space by a factor of
    # four. Constraint 5's own wording is `may_bind`'s.
    cross = collections.Counter()
    for records in statements.values():
        record = hardest(records)
        if record is None:
            continue
        chain = worst_of(record)
        if chain is None:
            continue
        cross["may_bind=%s  never_bound=%s"
              % (chain["base_may_bind"], chain["base_never_bound"])] += 1
    table("CROSS-TAB of the two readings of constraint 5's premise "
          "(FunctionTypes::may_bind vs UntrackedBases::never_bound):", cross)

    # `Unresolved` dominates the member axis, and an arbiter needs to know
    # WHICH reader refused: the base's declared type, or the member's. For a
    # single `->` the two are separable from what is already recorded — the
    # census carries whether the base is a committed `struct TAG *`.
    split = collections.Counter()
    for records in statements.values():
        record = hardest(records)
        if record is None:
            continue
        chain = worst_of(record)
        if chain is None or chain["depth"] != 1 or chain["steps"] != ["Arrow"]:
            continue
        if not chain["base_record_pointer"]:
            split["base type not committed (the reader would not commit to "
                  "`struct TAG *`)"] += 1
        elif chain["member_class"] == "Unresolved":
            split["base committed, MEMBER not classified"] += 1
        else:
            split["base committed, member classified as %s"
                  % chain["member_class"]] += 1
    table("WHY a single `->` leaf is unresolved (depth-1 arrow leaves only, "
          "%d of them):" % sum(split.values()), split)

    # ------------------------------------------------- the sole-blocked set
    say()
    say("=" * 78)
    say("FUNCTIONS BLOCKED BY THIS FAMILY ALONE")
    say("=" * 78)
    sole = {f for f in functions if set(blockers[f]) == {INDIRECT}}
    sole_leaves = [k for k in statements if leaf_functions[k] in sole]
    say("functions in the corpus carrying a leaf of this family: %d" % len(functions))
    say("functions whose ONLY blocker family is this one:         %d" % len(sole))
    say("leaves those functions hold: %d  (%.1f%% of the family's %d)"
        % (len(sole_leaves), 100.0 * len(sole_leaves) / total_leaves, total_leaves))
    say()
    say("This is the card's central asymmetry, re-measured: the family's leaf")
    say("mass and its function reach are nearly disjoint. An arbiter aimed at")
    say("the leaf count clears leaves; only the sole-blocked set can be")
    say("*decided* by this family alone.")

    # The decision-relevant population is the sole-blocked one, so its axes are
    # reported separately rather than inferred from the whole family's.
    say()
    say("--- the same axes, restricted to the %d leaves of the %d sole-blocked "
        "functions ---" % (len(sole_leaves), len(sole)))

    def soleleafwise(extract, chain_level=False, default="no-chain"):
        counter = collections.Counter()
        for key in sole_leaves:
            record = hardest(statements[key])
            if record is None:
                continue
            if not chain_level:
                counter[extract(record)] += 1
                continue
            chain = worst_of(record)
            counter[default if chain is None else extract(chain)] += 1
        return counter

    table("position:", soleleafwise(lambda r: r["position"]))
    table("contains a call, and is it contracted:",
          soleleafwise(lambda r: "no call" if not r["contains_call"]
                       else ("contracted call" if r.get("call_contracted")
                             else "UNCONTRACTED call")))
    table("constraint 5's premise (may_bind):",
          soleleafwise(lambda c: "may-bind" if c["base_may_bind"] else "NEVER-BOUND",
                       chain_level=True))
    table("base kind:", soleleafwise(lambda c: c["base_kind"], chain_level=True))
    table("final member class:",
          soleleafwise(lambda c: c["member_class"], chain_level=True))
    table("step spelling:",
          soleleafwise(lambda c: "".join(STEP[s] for s in c["steps"]) or "-",
                       chain_level=True), limit=10)

    co = collections.Counter()
    for f in functions:
        for other in set(blockers[f]) - {INDIRECT}:
            co[other] += 1
    table("top co-blockers, by how many of the %d functions carry them:"
          % len(functions), co, limit=12)

    # ------------------------------------------- shapes of the sole-blocked
    say()
    say("=" * 78)
    say("THE SOLE-BLOCKED FUNCTIONS, BY SHAPE")
    say("=" * 78)
    per_function = collections.defaultdict(set)
    uncovered_functions = set()
    for key in sole_leaves:
        f = leaf_functions[key]
        records = statements[key]
        if not records:
            uncovered_functions.add(f)
            per_function[f].add(("UNEXPLAINED", "-", "-", "-", "-"))
        for record in records:
            per_function[f].add(shape(record))

    single = {f for f in sole if len(per_function[f]) == 1}
    say("sole-blocked functions with an unexplained leaf: %d" % len(uncovered_functions))
    say("sole-blocked functions whose every leaf-statement has ONE shape: %d  (%.1f%%)"
        % (len(single), 100.0 * len(single) / len(sole)))
    say("sole-blocked functions needing 2+ shapes admitted together:     %d"
        % (len(sole) - len(single)))
    table("distinct shapes needed per sole-blocked function:",
          collections.Counter(str(min(len(per_function[f]), 6)) for f in sole))

    shape_counter = collections.Counter()
    for f in single:
        shape_counter[next(iter(per_function[f]))] += 1
    say()
    say("single-shape sole-blocked functions, by that shape:")
    say("   %5s  %s" % ("funcs", "base kind | premise | steps | member | position"))
    for key, count in shape_counter.most_common(25):
        say("   %5d  %s" % (count, " | ".join(str(x) for x in key)))

    # --------------------------------- the discounted three-way prediction
    say()
    say("=" * 78)
    say("PREDICTED THREE-WAY SPLIT PER CANDIDATE SHAPE")
    say("  (1) leaf-free   (2) exchanged for another refusal   (3) unmoved")
    say("=" * 78)
    say("Method, stated so the numbers can be checked rather than believed.")
    say("For a candidate shape S, take EVERY function carrying a leaf whose")
    say("statements include shape S. It is predicted leaf-free only if:")
    say("  - its only blocker family is this one; otherwise UNMOVED, because")
    say("    another family still refuses it;")
    say("  - every statement of every leaf it holds in this family has shape S;")
    say("    otherwise UNMOVED, because the leaves S does not cover remain;")
    say("  - and no such statement carries a call to an UNCONTRACTED function,")
    say("    nor a second access chain of a different shape; otherwise")
    say("    EXCHANGED, because admitting the access leaves the statement")
    say("    facing the uncontracted-call gate, or a second access S does not")
    say("    cover. A call the analyzer already has a contract for is NOT an")
    say("    exchange: the statement's remaining decision is one the projection")
    say("    can already make.")
    say("The exchange test is a PROXY. It errs toward calling a function")
    say("exchanged rather than cleared, so the CLEAR column is a lower bound on")
    say("nothing and an upper bound on what a wave-37 arbiter could hope for;")
    say("it is a scoping estimate, not a measurement of that arbiter's result.")
    say("Shapes are ranked by predicted DECIDED FUNCTIONS, not by leaf mass —")
    say("ranking by leaves is exactly the error this card exists to prevent.")

    shapes_all = collections.Counter()
    touched_by = collections.defaultdict(set)
    own_statements = collections.defaultdict(list)
    for key, records in statements.items():
        f = leaf_functions[key]
        own_statements[f].extend(records)
        seen_here = set()
        for record in records:
            s = shape(record)
            touched_by[s].add(f)
            seen_here.add(s)
        record = hardest(records)
        if record is not None:
            shapes_all[shape(record)] += 1

    def exchanges(record, candidate):
        """Would admitting `candidate` leave this statement still refused?"""
        if record["contains_call"] and not record.get("call_contracted"):
            return True
        return any(shape_of_chain(c, record) != candidate for c in record["chains"])

    def shape_of_chain(chain, record):
        return (chain["base_kind"],
                "may-bind" if chain["base_may_bind"] else "never-bound",
                "".join(STEP[s] for s in chain["steps"]) or "-",
                chain["member_class"], record["position"])

    predictions = []
    for candidate in shapes_all:
        cleared, exchanged, unmoved = [], [], []
        for f in touched_by[candidate]:
            if set(blockers[f]) != {INDIRECT}:
                unmoved.append(f)
                continue
            own = own_statements[f]
            if not own or any(shape(r) != candidate for r in own):
                unmoved.append(f)
                continue
            if any(exchanges(r, candidate) for r in own):
                exchanged.append(f)
                continue
            cleared.append(f)
        predictions.append({
            "shape": list(candidate),
            "leaves_where_hardest": shapes_all[candidate],
            "functions_touched": len(touched_by[candidate]),
            "cleared": len(cleared),
            "exchanged": len(exchanged),
            "unmoved": len(unmoved),
            "cleared_functions": sorted("/".join(str(x) for x in f) for f in cleared),
        })
    predictions.sort(key=lambda p: (-p["cleared"], -p["leaves_where_hardest"]))

    say()
    say("Ranked by predicted DECIDED functions. Top 12 by that measure, then")
    say("the top 12 by leaf mass, so the gap between the two is visible.")
    say()
    say("%-52s %7s %7s %6s %6s %6s"
        % ("shape (ranked by CLEAR)", "leaves", "funcs", "CLEAR", "exch", "unmov"))
    for p in predictions[:12]:
        say("%-52s %7d %7d %6d %6d %6d"
            % (" | ".join(p["shape"])[:52], p["leaves_where_hardest"],
               p["functions_touched"], p["cleared"], p["exchanged"], p["unmoved"]))
    say()
    say("%-52s %7s %7s %6s %6s %6s"
        % ("shape (ranked by LEAF MASS)", "leaves", "funcs", "CLEAR", "exch", "unmov"))
    for p in sorted(predictions, key=lambda p: -p["leaves_where_hardest"])[:12]:
        say("%-52s %7d %7d %6d %6d %6d"
            % (" | ".join(p["shape"])[:52], p["leaves_where_hardest"],
               p["functions_touched"], p["cleared"], p["exchanged"], p["unmoved"]))
    say()
    say("totals over ALL %d distinct shapes:" % len(predictions))
    say("   shapes deciding at least one function: %d"
        % len([p for p in predictions if p["cleared"]]))
    say("   best single shape decides:             %d function(s)"
        % (predictions[0]["cleared"] if predictions else 0))
    say("   union of every shape's cleared set:    %d function(s)"
        % len({f for p in predictions for f in p["cleared_functions"]}))

    # ------------------------------------------------ pairs, as the card asks
    say()
    say("=" * 78)
    say("PAIRS OF SHAPES ADMITTED TOGETHER")
    say("=" * 78)
    say("%d of the %d sole-blocked functions need two or more shapes admitted"
        % (len(sole) - len(single), len(sole)))
    say("together before they hold no leaf, so a single-shape ranking")
    say("understates what a two-shape arbiter could decide. The same discount")
    say("is applied: a pair decides a function only if the function is")
    say("sole-blocked, every statement of every leaf it holds has one of the")
    say("two shapes, and none of them exchanges.")
    need = {}
    for f in sole:
        need[f] = per_function[f]
    candidates = collections.Counter()
    for f in sole:
        for s in need[f]:
            candidates[s] += 1
    top = [s for s, _ in candidates.most_common(40)]
    pairs = []
    for i, a in enumerate(top):
        for b in top[i + 1:]:
            allowed = {a, b}
            decided = [f for f in sole
                       if need[f] <= allowed
                       and not any(exchanges(r, shape(r)) for r in own_statements[f])]
            if decided:
                pairs.append((len(decided), a, b, decided))
    pairs.sort(reverse=True, key=lambda row: row[0])
    say()
    if pairs:
        say("best pairs, by functions decided:")
        for count, a, b, decided in pairs[:8]:
            say("  %3d functions" % count)
            say("      A: %s" % " | ".join(a))
            say("      B: %s" % " | ".join(b))
    else:
        say("NO pair of shapes decides a single function under the discount.")

    say()
    say("Shapes predicted to clear at least one function, with the functions:")
    any_cleared = False
    for p in predictions:
        if p["cleared"]:
            any_cleared = True
            say("  %s  ->  %d function(s)" % (" | ".join(p["shape"]), p["cleared"]))
            for name in p["cleared_functions"][:25]:
                say("      %s" % name)
    if not any_cleared:
        say("  NONE. Every candidate shape's discounted prediction is zero")
        say("  functions decided. That is the card's result, not a gap in it.")

    json.dump({
        "provenance": {"statement_records": streamed,
                       "unit_ordinals": len(census),
                       "units_assigned": len(assignment),
                       "units_unassigned": len(unassigned)},
        "coverage": {"published_leaves": total_leaves, "explained": covered,
                     "unexplained": total_leaves - covered,
                     "multi_statement_leaves": multi,
                     "functions": len(functions), "units": len(unit_leaves)},
        "sole_blocked": {
            "functions": len(sole), "leaves": len(sole_leaves),
            "single_shape_functions": len(single),
            "by_shape": [{"shape": list(k), "functions": v}
                         for k, v in shape_counter.most_common()]},
        "co_blockers": dict(co.most_common(20)),
        "predictions": predictions,
    }, open(out / "census.json", "w"), indent=1)
    (out / "census.txt").write_text("\n".join(report) + "\n")


if __name__ == "__main__":
    main()
