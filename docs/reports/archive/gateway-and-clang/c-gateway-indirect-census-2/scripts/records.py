#!/usr/bin/env python3
"""Why is a member unclassifiable? The record-body reader's own answer.

`TypedAccessModel::read_record_body` is total-or-reject over the WHOLE body:

    if body.iter().any(|t| matches!(*t, "{" | "}" | "(" | ")" | ":" | "=")) {
        return None;
    }

so one member declaration the reader will not parse — a function pointer's
`(`, a bitfield's `:`, a nested anonymous aggregate's `{` — makes every member
of that record unclassifiable, including the plain scalars beside it. This
tool separates the record that is genuinely opaque from the record that is
collateral damage, and crosses the answer with the refused accesses that
actually land on each tag.

Usage: python3 records.py <run-dir> <out-dir>
"""
import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from analyse import load_document, load_census, match_units, shape  # noqa: E402

INDIRECT = "indirect memory access is outside the resource projection"


def main():
    run = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)

    # One reading per (tag, refused_on, body_tokens); the same header is read
    # once per translation unit that includes it, so duplicates are expected.
    readings = {}
    disagreeing = set()
    for line in open(run / "record-bodies.jsonl"):
        note = json.loads(line)
        key = note["tag"]
        if key in readings and readings[key]["refused_on"] != note["refused_on"]:
            disagreeing.add(key)
        readings[key] = note

    report = []

    def say(text=""):
        print(text)
        report.append(text)

    def table(title, counter, limit=None):
        say()
        say(title)
        total = sum(counter.values()) or 1
        for name, count in counter.most_common(limit):
            say("   %8d  %5.1f%%  %s" % (count, 100.0 * count / total, name))

    say("=" * 78)
    say("THE RECORD-BODY READER, MEASURED")
    say("=" * 78)
    say("distinct record tags whose body the reader met: %d" % len(readings))
    say("tags read differently in different units:       %d" % len(disagreeing))
    refused = {t: n for t, n in readings.items() if n["refused_on"]}
    say("tags the reader REFUSED WHOLE:                  %d  (%.1f%%)"
        % (len(refused), 100.0 * len(refused) / max(len(readings), 1)))
    table("the token that refused the body:",
          collections.Counter(n["refused_on"] for n in refused.values()))

    # Cross with the refusals that actually land on each tag.
    blockers, unit_leaves, leaf_functions = load_document(run / "resource-diagnostics.json")
    census, _ = load_census(run / "indirect-leaves.jsonl")
    assignment = match_units(census, unit_leaves)

    statements = {}
    for ukey, keys in unit_leaves.items():
        records = census.get(assignment.get(ukey), {})
        for key in keys:
            statements[ukey + key] = records.get(key, [])

    sole = {f for f in {leaf_functions[k] for k in statements}
            if set(blockers[f]) == {INDIRECT}}

    by_tag = collections.Counter()
    by_tag_sole = collections.Counter()
    unresolved_with_tag = 0
    unresolved_no_tag = 0
    for key, records in statements.items():
        if not records:
            continue
        hardest = max(records, key=shape)
        chains = hardest["chains"]
        if not chains:
            continue
        chain = max(chains, key=lambda c: (c["base_may_bind"], c["depth"]))
        if chain["member_class"] != "Unresolved":
            continue
        tag = chain.get("tag")
        if not tag:
            unresolved_no_tag += 1
            continue
        unresolved_with_tag += 1
        by_tag[tag] += 1
        if leaf_functions[key] in sole:
            by_tag_sole[tag] += 1

    say()
    say("=" * 78)
    say("THE UNRESOLVED-MEMBER LEAVES, CROSSED WITH THE READER'S ANSWER")
    say("=" * 78)
    say("leaves whose deciding chain ends at an Unresolved member: %d"
        % (unresolved_with_tag + unresolved_no_tag))
    say("  the reader committed to a record tag for the base:     %d" % unresolved_with_tag)
    say("  no tag at all (the BASE's type was not committed):     %d" % unresolved_no_tag)

    split = collections.Counter()
    split_sole = collections.Counter()
    for tag, count in by_tag.items():
        note = readings.get(tag)
        if note is None:
            label = "record never declared in the units that use it"
        elif note["refused_on"]:
            label = "record body REFUSED WHOLE on `%s`" % note["refused_on"]
        else:
            label = "record body read; this member is simply in no class"
        split[label] += count
        split_sole[label] += by_tag_sole[tag]
    table("of the %d tagged Unresolved leaves, why:" % unresolved_with_tag, split)
    table("the same, restricted to sole-blocked functions "
          "(%d leaves):" % sum(split_sole.values()), split_sole)

    say()
    say("top record tags by tagged Unresolved leaves:")
    say("   %6s %6s  %-34s %s" % ("leaves", "sole", "tag", "reader's answer"))
    for tag, count in by_tag.most_common(20):
        note = readings.get(tag)
        answer = ("never declared" if note is None
                  else ("REFUSED on `%s`" % note["refused_on"] if note["refused_on"]
                        else "read; member in no class"))
        say("   %6d %6d  %-34s %s" % (count, by_tag_sole[tag], tag[:34], answer))

    # ------------------------------------------------------------- the ceiling
    #
    # A per-member total-or-reject reader would give a class to the members of a
    # record the current whole-body reject refuses. It cannot give a class to a
    # member whose OWN declaration is the unparseable one, and it cannot help a
    # base whose type was never committed. So the honest statement is an UPPER
    # BOUND: the functions that would hold no leaf if every member of a
    # whole-body-refused record became classified AND the resulting access were
    # then admitted — the second conjunct being exactly what a wave-37 arbiter
    # would still have to earn, shape by shape, under constraint 5's premise.
    say()
    say("=" * 78)
    say("CEILING FOR A PER-MEMBER RECORD READER")
    say("=" * 78)
    say("This is an UPPER BOUND and nothing else. It counts a function only if")
    say("it is sole-blocked and every statement of every leaf it holds is")
    say("(a) non-exchanging and (b) barred by an Unresolved member on a record")
    say("the reader refused WHOLE. It assumes, without measuring, that every")
    say("such member would become classified AND that the resulting access")
    say("would then be admitted. Both assumptions are generous; the second is")
    say("the one constraint 5's premise governs, and 87.2%% of the sole-blocked")
    say("leaves fail that premise, so the realised number will be far below.")

    def exchanges(record):
        if record["contains_call"] and not record.get("call_contracted"):
            return True
        shapes = {shape(record)}
        return len(shapes) > 1

    own = collections.defaultdict(list)
    for key, records in statements.items():
        own[leaf_functions[key]].extend(records)

    ceiling, premise_holds = [], []
    for f in sole:
        rs = own[f]
        if not rs or any(exchanges(r) for r in rs):
            continue
        blocked_by_refused_record = True
        premise = True
        for r in rs:
            for c in r["chains"]:
                if c["member_class"] != "Unresolved":
                    blocked_by_refused_record = False
                    break
                note = readings.get(c.get("tag") or "")
                if note is None or not note["refused_on"]:
                    blocked_by_refused_record = False
                    break
                if c["base_may_bind"]:
                    premise = False
            if not blocked_by_refused_record:
                break
        if blocked_by_refused_record:
            ceiling.append(f)
            if premise:
                premise_holds.append(f)
    say()
    say("sole-blocked functions barred ONLY by a whole-body-refused record: %d"
        % len(ceiling))
    say("  of those, every base also satisfies constraint 5's premise:      %d"
        % len(premise_holds))
    say("  (the second number is the one an arbiter could pursue without")
    say("   touching the premise the wave-17 fail-closed pin already caught)")
    for f in sorted("/".join(str(x) for x in g) for g in premise_holds)[:25]:
        say("      %s" % f)

    json.dump({
        "ceiling_functions": len(ceiling),
        "ceiling_functions_premise_holds": len(premise_holds),
        "ceiling_premise_holds_names": sorted(
            "/".join(str(x) for x in g) for g in premise_holds),
        "tags_seen": len(readings),
        "tags_refused_whole": len(refused),
        "refusing_tokens": dict(collections.Counter(
            n["refused_on"] for n in refused.values())),
        "unresolved_leaves_with_tag": unresolved_with_tag,
        "unresolved_leaves_without_tag": unresolved_no_tag,
        "why": dict(split),
        "why_sole_blocked": dict(split_sole),
        "top_tags": [{"tag": t, "leaves": c, "sole_blocked_leaves": by_tag_sole[t],
                      "refused_on": (readings.get(t) or {}).get("refused_on")}
                     for t, c in by_tag.most_common(40)],
    }, open(out / "records.json", "w"), indent=1)
    (out / "records.txt").write_text("\n".join(report) + "\n")


if __name__ == "__main__":
    main()
