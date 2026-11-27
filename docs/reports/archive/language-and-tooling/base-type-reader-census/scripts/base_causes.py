#!/usr/bin/env python3
"""Card BASE-TYPE-READER-CENSUS: the base-type reader, measured.

Residual `BASE-TYPE-READER-UNMEASURED` (wave 36) records that 78.5% of the
`indirect memory access is outside the resource projection` family's
`Unresolved` members were never a member question: the chain's tag was already
absent, because the declaration reader had not committed to the BASE's declared
type. This tool measures that reader.

It deliberately reuses card C-GATEWAY-INDIRECT-CENSUS-2's join verbatim (the
unit-ordinal bijection, the hardest-statement attribution, the exchange proxy),
so the two cards' populations are the same population and any difference is a
difference in what was instrumented rather than in how it was counted.

Usage: python3 base_causes.py <run-dir> <out-dir>
"""
import collections
import json
import pathlib
import re
import sys

INDIRECT = "indirect memory access is outside the resource projection"
STEP = {"Arrow": "->", "Dot": ".", "Subscript": "[]", "Deref": "*"}


# ---------------------------------------------------------------- wave-36 join
def normalize(description):
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
    chains = record["chains"]
    if not chains:
        return ("no-chain", "-", "-", "-", record["position"])
    worst = max(chains, key=lambda c: (c["base_may_bind"], c["depth"]))
    return (
        worst["base_kind"],
        "may-bind" if worst["base_may_bind"] else "never-bound",
        "".join(STEP[s] for s in worst["steps"]) or "-",
        worst["member_class"],
        record["position"],
    )


# ------------------------------------------------------------ the cause vocab
def cause_key(chain):
    """The base-type cause, as one comparable string, in the reader's own words.

    A payload-carrying variant keeps its *kind* here and its payload is reported
    separately: an arbiter legislates for `a typedef that defines its aggregate`,
    not for one named struct.
    """
    cause = chain["base_cause"]
    if isinstance(cause, str):
        return cause
    (name, payload), = cause.items()
    if name == "FunctionRefused":
        inner = payload if isinstance(payload, str) else list(payload)[0]
        return "FunctionRefused/" + inner
    if name == "NotDeclared":
        return "NotDeclared/" + payload
    if name == "DeclaredOpaque":
        inner = payload if isinstance(payload, str) else list(payload)[0]
        return "DeclaredOpaque/" + inner
    return name


def cause_payload(chain):
    cause = chain["base_cause"]
    if isinstance(cause, str):
        return None
    (name, payload), = cause.items()
    if isinstance(payload, dict):
        (_, inner), = payload.items()
        return inner
    return None


def blocking_chain(record):
    """The chain whose base an arbiter would have to commit first.

    A statement clears only if EVERY chain in it clears, so the census reports
    the hardest: a base the reader will not commit to outranks one it did, and
    among those a `may_bind` base outranks a never-bound one (constraint 5).
    """
    chains = record["chains"]
    if not chains:
        return None
    return max(
        chains,
        key=lambda c: (
            cause_key(c) != "Committed",
            c["base_may_bind"],
            c["depth"],
        ),
    )


def main():
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, streamed = load_census(run / "indirect-leaves.jsonl")
    assignment = match_units(census, unit_leaves)

    bodies = {}
    for line in open(run / "record-bodies.jsonl"):
        note = json.loads(line)
        bodies.setdefault(note["tag"], note)

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

    say("=" * 78)
    say("JOIN (card C-GATEWAY-INDIRECT-CENSUS-2's, verbatim)")
    say("=" * 78)
    say("statement records streamed:                  %d" % streamed)
    say("distinct translation-unit ordinals:          %d" % len(census))
    say("leaf-carrying units in the document:         %d" % len(unit_leaves))
    say("units the join assigned:                     %d" % len(assignment))
    say("units the join could NOT assign:             %d"
        % len(set(unit_leaves) - set(assignment)))
    say("record bodies the reader met:                %d" % len(bodies))

    statements = {}
    for ukey, keys in unit_leaves.items():
        seq = assignment.get(ukey)
        records = census.get(seq, {}) if seq is not None else {}
        for key in keys:
            statements[ukey + key] = records.get(key, [])
    total_leaves = len(statements)
    covered = sum(1 for v in statements.values() if v)
    functions = sorted({leaf_functions[k] for k in statements})
    say("published leaves of this family:             %d" % total_leaves)
    say("  explained by at least one statement:       %d  (%.2f%%)"
        % (covered, 100.0 * covered / total_leaves))
    say("functions carrying one:                      %d" % len(functions))

    def hardest(records):
        return max(records, key=lambda r: shape(r)) if records else None

    # ------------------------------------------------------------ reproduction
    say()
    say("=" * 78)
    say("REPRODUCING THE WAVE-36 FIGURE BEFORE EXTENDING IT")
    say("=" * 78)
    say("The wave-36 card reports 13,396 leaves ending at an `Unresolved`")
    say("member, of which 10,514 (78.5%) carry `tag: null` -- which that card")
    say("read as `the BASE's type was never committed`. It is reproduced here")
    say("from the same field, and then DECOMPOSED, because `tag` is the tag of")
    say("the LAST selection: a chain whose base DID commit can still lose the")
    say("tag at an intermediate member or a subscript, and that is the wave-36")
    say("card's own question rather than this one's.")
    unresolved = 0
    tagless = 0
    decomp = collections.Counter()
    for records in statements.values():
        record = hardest(records)
        if record is None:
            continue
        chains = record["chains"]
        if not chains:
            continue
        worst = max(chains, key=lambda c: (c["base_may_bind"], c["depth"]))
        if worst["member_class"] != "Unresolved":
            continue
        unresolved += 1
        if worst["tag"] is not None:
            continue
        tagless += 1
        if cause_key(worst) == "Committed":
            if worst["tag_lost_step"] is not None:
                decomp["base COMMITTED; tag lost at step %d (a member question)"
                       % worst["tag_lost_step"]] += 1
            else:
                decomp["base COMMITTED; tag never lost (a member question)"] += 1
        else:
            decomp["BASE NOT COMMITTED (this card's question)"] += 1
    say()
    say("leaves whose deciding chain ends `Unresolved`:      %d" % unresolved)
    say("   of those, carrying no record tag at all:         %d  (%.1f%%)"
        % (tagless, 100.0 * tagless / (unresolved or 1)))
    table("DECOMPOSITION of that tagless row:", decomp)

    # -------------------------------------------------------------- the census
    say()
    say("=" * 78)
    say("WHY THE BASE TYPE DID NOT COMMIT")
    say("  Per LEAF, attributed its hardest statement and that statement's")
    say("  hardest chain -- a statement clears only if every chain in it does.")
    say("=" * 78)

    def leaf_causes():
        counter = collections.Counter()
        for records in statements.values():
            record = hardest(records)
            if record is None:
                counter["(unexplained leaf)"] += 1
                continue
            chain = blocking_chain(record)
            counter["(no chain)" if chain is None else cause_key(chain)] += 1
        return counter

    causes = leaf_causes()
    table("base-type cause, whole family (%d leaves):" % total_leaves, causes)

    payloads = collections.Counter()
    for records in statements.values():
        record = hardest(records)
        if record is None:
            continue
        chain = blocking_chain(record)
        if chain is None:
            continue
        p = cause_payload(chain)
        if p is not None:
            payloads["%s  ->  %s" % (cause_key(chain), p)] += 1
    table("top payloads (the spelling the reader refused):", payloads, limit=20)

    # For the typedef-defines-aggregate cause the payload NAMES the record tag,
    # so the census can ask the second question too: if the base type committed,
    # would the member then classify, or is that record's body refused whole?
    say()
    say("--- the typedef-defines-aggregate cause, crossed with the RECORD BODY")
    say("    reader (residual RECORD-BODY-TOTAL-OR-REJECT) ---")
    say("The cause payload names the definition's head, so this is the one")
    say("cause where the census can ask whether committing the base type would")
    say("actually resolve the member, or merely move the refusal one step.")
    cross = collections.Counter()
    for records in statements.values():
        record = hardest(records)
        if record is None:
            continue
        chain = blocking_chain(record)
        if chain is None or cause_key(chain) != "DeclaredOpaque/TypedefDefinesAggregate":
            continue
        head = cause_payload(chain) or ""
        parts = head.split()
        if len(parts) != 2 or parts[0] != "struct":
            cross["head is `%s` -- no struct tag to resolve to" % head] += 1
            continue
        tag = parts[1]
        note = bodies.get(tag)
        if note is None:
            cross["tag `struct X` whose body the reader never met"] += 1
        elif note["refused_on"] is not None:
            cross["tag known, but its record BODY is refused whole (on `%s`)"
                  % note["refused_on"]] += 1
        else:
            cross["tag known and its record body WAS read -- both readers agree"] += 1
    table("would committing the base type resolve the member?", cross)

    # ----------------------------------------------------------- sole-blocked
    say()
    say("=" * 78)
    say("THE SOLE-BLOCKED SUBSET, BY CAUSE")
    say("=" * 78)
    sole = {f for f in functions if set(blockers[f]) == {INDIRECT}}
    sole_leaves = [k for k in statements if leaf_functions[k] in sole]
    say("functions whose ONLY blocker family is this one: %d" % len(sole))
    say("leaves they hold: %d  (%.1f%% of %d)"
        % (len(sole_leaves), 100.0 * len(sole_leaves) / total_leaves, total_leaves))

    sole_causes = collections.Counter()
    for key in sole_leaves:
        record = hardest(statements[key])
        if record is None:
            sole_causes["(unexplained leaf)"] += 1
            continue
        chain = blocking_chain(record)
        sole_causes["(no chain)" if chain is None else cause_key(chain)] += 1
    table("base-type cause, sole-blocked leaves only (%d):" % len(sole_leaves),
          sole_causes)

    prem = collections.Counter()
    for key in sole_leaves:
        record = hardest(statements[key])
        if record is None:
            continue
        chain = blocking_chain(record)
        if chain is None:
            continue
        prem["%s  |  %s" % (cause_key(chain),
                            "may-bind" if chain["base_may_bind"] else "NEVER-BOUND")] += 1
    table("cause x constraint 5's premise (may_bind), sole-blocked leaves:",
          prem, limit=16)

    # ------------------------------------------------- per-cause prediction
    say()
    say("=" * 78)
    say("DISCOUNTED THREE-WAY PREDICTION PER CAUSE")
    say("  (1) leaf-free   (2) exchanged for another refusal   (3) unmoved")
    say("=" * 78)
    say("Method. For a candidate cause C -- a reader change that committed the")
    say("declared type of every base blocked by C -- take every function")
    say("carrying a leaf one of whose statements has a chain blocked by C. It")
    say("is predicted leaf-free only if:")
    say("  - its only blocker family is this one; else UNMOVED;")
    say("  - every chain of every statement of every leaf it holds in this")
    say("    family is either already Committed or blocked by C; else UNMOVED,")
    say("    because the chains C does not reach still refuse;")
    say("  - and no such statement carries an UNCONTRACTED call; else")
    say("    EXCHANGED, because admitting the access leaves the statement")
    say("    facing the uncontracted-call gate.")
    say("This is GENEROUS in one further way the shape-level prediction was")
    say("not, and it is stated rather than buried: committing a base's type")
    say("yields a TAG, not an admission. The member must then classify, and")
    say("the access rule must then admit it. So CLEAR is an upper bound on an")
    say("upper bound -- see the record-body cross-tab above for the one cause")
    say("where the second question can be asked.")

    own_statements = collections.defaultdict(list)
    touched_by = collections.defaultdict(set)
    leaves_where_blocking = collections.Counter()
    for key, records in statements.items():
        f = leaf_functions[key]
        own_statements[f].extend(records)
        for record in records:
            for chain in record["chains"]:
                c = cause_key(chain)
                if c != "Committed":
                    touched_by[c].add(f)
        record = hardest(records)
        if record is not None:
            chain = blocking_chain(record)
            if chain is not None and cause_key(chain) != "Committed":
                leaves_where_blocking[cause_key(chain)] += 1

    def exchanges(record):
        return bool(record["contains_call"]) and not record.get("call_contracted")

    def covered_by(record, cause):
        return all(cause_key(c) in ("Committed", cause) for c in record["chains"])

    predictions = []
    for cause in touched_by:
        cleared, exchanged, unmoved = [], [], []
        for f in touched_by[cause]:
            if set(blockers[f]) != {INDIRECT}:
                unmoved.append(f)
                continue
            own = own_statements[f]
            if not own or any(not covered_by(r, cause) for r in own):
                unmoved.append(f)
                continue
            if any(exchanges(r) for r in own):
                exchanged.append(f)
                continue
            cleared.append(f)
        predictions.append({
            "cause": cause,
            "leaves_where_blocking": leaves_where_blocking[cause],
            "functions_touched": len(touched_by[cause]),
            "cleared": len(cleared),
            "exchanged": len(exchanged),
            "unmoved": len(unmoved),
            "cleared_functions": sorted("/".join(str(x) for x in f) for f in cleared),
        })
    predictions.sort(key=lambda p: (-p["cleared"], -p["leaves_where_blocking"]))

    say()
    say("%-46s %7s %7s %6s %6s %6s"
        % ("cause (ranked by CLEAR)", "leaves", "funcs", "CLEAR", "exch", "unmov"))
    for p in predictions:
        say("%-46s %7d %7d %6d %6d %6d"
            % (p["cause"][:46], p["leaves_where_blocking"], p["functions_touched"],
               p["cleared"], p["exchanged"], p["unmoved"]))
    say()
    say("%-46s %7s %7s %6s %6s %6s"
        % ("cause (ranked by LEAF MASS)", "leaves", "funcs", "CLEAR", "exch", "unmov"))
    for p in sorted(predictions, key=lambda p: -p["leaves_where_blocking"]):
        say("%-46s %7d %7d %6d %6d %6d"
            % (p["cause"][:46], p["leaves_where_blocking"], p["functions_touched"],
               p["cleared"], p["exchanged"], p["unmoved"]))
    say()
    say("causes deciding at least one function: %d of %d"
        % (len([p for p in predictions if p["cleared"]]), len(predictions)))
    say("best single cause decides:             %d function(s)"
        % (predictions[0]["cleared"] if predictions else 0))
    union = {f for p in predictions for f in p["cleared_functions"]}
    say("union of EVERY cause's cleared set:    %d function(s)" % len(union))
    say("   (the ceiling if one card committed every base type at once, still")
    say("    assuming every member then classifies and every access is then")
    say("    admitted -- against %d sole-blocked and %d touched functions)"
        % (len(sole), len(functions)))

    # ------------------------------------------------------------------ pairs
    say()
    say("=" * 78)
    say("PAIRS OF CAUSES COMMITTED TOGETHER")
    say("=" * 78)
    pairs = []
    keys = sorted(touched_by)
    for i, a in enumerate(keys):
        for b in keys[i + 1:]:
            decided = []
            for f in touched_by[a] | touched_by[b]:
                if set(blockers[f]) != {INDIRECT}:
                    continue
                own = own_statements[f]
                if not own:
                    continue
                if any(not all(cause_key(c) in ("Committed", a, b) for c in r["chains"])
                       for r in own):
                    continue
                if any(exchanges(r) for r in own):
                    continue
                decided.append(f)
            if decided:
                pairs.append((len(decided), a, b))
    pairs.sort(reverse=True, key=lambda row: row[0])
    for count, a, b in pairs[:10]:
        say("  %3d functions   A: %-38s B: %s" % (count, a, b))
    if not pairs:
        say("  no pair decides any function.")

    say()
    say("=" * 78)
    say("NAMED CLEARED FUNCTIONS (so the recommendation can be checked)")
    say("=" * 78)
    for p in predictions[:6]:
        if not p["cleared_functions"]:
            continue
        say("%s  (%d)" % (p["cause"], p["cleared"]))
        for f in p["cleared_functions"][:20]:
            say("    %s" % f)

    (out / "base-causes.txt").write_text("\n".join(report) + "\n")
    json.dump(predictions, open(out / "predictions.json", "w"), indent=2)
    print()
    print("written: %s" % (out / "base-causes.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
