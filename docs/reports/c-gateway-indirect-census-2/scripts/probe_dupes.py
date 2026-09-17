#!/usr/bin/env python3
"""Why do two census records for one (function, line) disagree?

W4 derives a custody summary and then *validates it by re-analysis*
(constraint 4), so one function body is classified several times under
different contract environments. The exempted spans the lexical gate subtracts
(`borrowed_actual`, `aggregate_store`, `variadic_nested`) are computed from
those contracts, so the gate's own token window legitimately differs between
rounds. This probe measures the shape of that disagreement so the census can
say which round it publishes and how much the answer depends on the choice.
"""
import collections
import json
import sys

path = sys.argv[1]
by_key = collections.defaultdict(list)
seqs = collections.Counter()
for line in open(path):
    r = json.loads(line)
    seqs[r["unit_seq"]] += 1
    by_key[(r["unit_seq"], r["function"], r["line"])].append(r)

print("distinct unit_seq values: %d" % len(seqs))
print("records: %d" % sum(seqs.values()))
print("keys including unit_seq: %d" % len(by_key))

disagree_in_unit = 0
multi = 0
axis_moves = collections.Counter()
for key, records in by_key.items():
    if len(records) < 2:
        continue
    multi += 1
    blobs = {json.dumps(r, sort_keys=True) for r in records}
    if len(blobs) > 1:
        disagree_in_unit += 1
        first, last = records[0], records[-1]
        for axis in ("position", "site", "contains_call", "tokens"):
            if first[axis] != last[axis]:
                axis_moves["%s: %s -> %s" % (axis, first[axis], last[axis])] += 1
        if len(first["chains"]) != len(last["chains"]):
            axis_moves["chain count: %d -> %d"
                       % (len(first["chains"]), len(last["chains"]))] += 1

print("keys seen more than once WITHIN one unit_seq: %d" % multi)
print("  of those, disagreeing: %d" % disagree_in_unit)
print()
print("what moves between the first and last round of one key:")
for name, count in axis_moves.most_common(15):
    print("   %6d  %s" % (count, name))

# Does a key ever appear under two different unit_seq values?
across = collections.defaultdict(set)
for (seq, fn, line) in by_key:
    across[(fn, line)].add(seq)
shared = {k: v for k, v in across.items() if len(v) > 1}
print()
print("(function, line) keys appearing under MORE THAN ONE unit_seq: %d"
      % len(shared))
print("  (these are the cross-unit name collisions the join must account for)")
