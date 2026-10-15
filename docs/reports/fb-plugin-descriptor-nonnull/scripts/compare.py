#!/usr/bin/env python3
"""compare.py BEFORE.jsonl AFTER.jsonl [OUT.json] -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (from NULL-MEMBER-INVARIANT-SCOPE, NULL-CANDIDATE-TRIAGE):
the null property's BEFORE and AFTER rows of one route, joined by (library,
file, function, configuration index): verdict moves (Safe split into unconditional and
conditional), each moving row attributed to what changed it (this card: a
fact a stable member kept (`stable-member`), a scope entry read in the body
(`scope-entry`), a scope entry's caller check (`scope-caller-check-`
candidate/discharged), a check a withdrawn declaration demoted
(`withdrawn`); and the earlier causes), candidates by kind before and after,
and the custody column's identity, and every check whose outcome changed in
a row that did not move (`check_changes`)."""
import collections
import json
import sys

NEW_CLAUSES = set()
ORDER = ["allocation", "lookup", "nullable-result", "null-literal", "uninitialized",
         "uncontracted-result", "formal", "global", "memory-field", "other", "withdrawn"]


def load(path):
    """Rows keyed by (library, file, function, n): n numbers the rows of one
    function in stream order, which is unit order (the static and shared
    configurations of a core file are separate units with equal names)."""
    rows = {}
    seen = collections.Counter()
    for line in open(path):
        if line.strip():
            row = json.loads(line)
            base = (row["library"], row["file"], row["function"])
            rows[base + (seen[base],)] = row
            seen[base] += 1
    return rows


def verdict(row):
    null = row.get("null") or {}
    v = null.get("verdict", "absent")
    if v == "Safe":
        return "Safe (unconditional)" if null.get("scope") == "unconditional" else "Safe (conditional)"
    return v


def kinds(rows):
    by = collections.Counter()
    for row in rows.values():
        for check in (row.get("null") or {}).get("checks") or []:
            if check["outcome"] != "candidate":
                continue
            ks = sorted({s.split(":")[0] for s in check.get("sources") or []},
                        key=lambda k: ORDER.index(k) if k in ORDER else 99)
            by[ks[0] if ks else "other"] += 1
    return dict(by)


def causes(before, after):
    out = set()
    checks = (after.get("null") or {}).get("checks") or []
    old = {(c["kind"], c["line"], c["subject"]): c for c in (before.get("null") or {}).get("checks") or []}
    for check in checks:
        proofs = check.get("proofs") or []
        subject = check.get("subject", "")
        if check["outcome"] == "finding":
            out.add("finding")
        if any(p.startswith("result:stable:") for p in proofs):
            out.add("stable-member")
        if any("nonnull_member_scope(" in p for p in proofs):
            out.add("scope-entry")
        if "nonnull_member_scope(" in subject:
            out.add("scope-caller-check-" + ("candidate" if check["outcome"] == "candidate" else "discharged"))
        if any(s.startswith("withdrawn:") for s in check.get("sources") or []):
            out.add("withdrawn")
        if any(p.startswith("result:member:") for p in proofs):
            out.add("declared-member")
        if any(p.startswith("result:declared-result:") for p in proofs):
            out.add("declared-result")
        if any(p.startswith("result:cross-unit:") for p in proofs):
            out.add("cross-unit-result")
        if "at cross-unit inferred" in subject:
            out.add("travelled-clause-" + ("candidate" if check["outcome"] == "candidate" else "discharged"))
        if any(f"`{name}`" in subject or subject.endswith(f"at quant null-deref #{name}") for name in NEW_CLAUSES):
            out.add("new-declared-clause")
        if " at quant null-deref" in subject and (check["kind"], check["line"], check["subject"]) not in old:
            out.add("new-declared-clause")
    deps = set((after.get("null") or {}).get("contract_dependencies") or [])
    if deps & NEW_CLAUSES:
        out.add("new-declared-clause")
    if after["function"] in NEW_CLAUSES:
        out.add("own-new-declared-clause")
    return sorted(out) or ["unattributed"]


before = load(sys.argv[1])
after = load(sys.argv[2])
moves = collections.Counter()
by_cause = collections.defaultdict(collections.Counter)
moving = []
custody_moves = 0
for key in sorted(set(before) | set(after)):
    b, a = before.get(key), after.get(key)
    if b is None or a is None:
        moves[f"{'absent' if b is None else verdict(b)} -> {'absent' if a is None else verdict(a)}"] += 1
        continue
    if b.get("custody") != a.get("custody"):
        custody_moves += 1
    vb, va = verdict(b), verdict(a)
    if vb == va:
        continue
    move = f"{vb} -> {va}"
    moves[move] += 1
    reason = causes(b, a)
    for r in reason:
        by_cause[move][r] += 1
    moving.append({"library": key[0], "file": key[1], "function": key[2], "configuration": key[3], "move": move, "causes": reason,
                   "safe_to_non_safe": vb.startswith("Safe") and not va.startswith("Safe")})
check_changes = collections.Counter()
added_candidates = []
removed_candidates = collections.Counter()
for key in sorted(set(before) & set(after)):
    old = collections.Counter((c["kind"], c["line"], c["subject"], c["outcome"])
                              for c in (before[key].get("null") or {}).get("checks") or [])
    new = collections.Counter((c["kind"], c["line"], c["subject"], c["outcome"])
                              for c in (after[key].get("null") or {}).get("checks") or [])
    for (kind, line, subject, outcome), n in (old - new).items():
        check_changes[f"removed {outcome}"] += n
    for (kind, line, subject, outcome), n in (new - old).items():
        check_changes[f"added {outcome}"] += n
        if outcome == "candidate":
            added_candidates.append({"function": key[2], "file": key[1], "configuration": key[3],
                                     "line": line, "subject": subject, "count": n,
                                     "verdict_before": verdict(before[key]), "verdict_after": verdict(after[key])})
    for (kind, line, subject, outcome), n in (old - new).items():
        if outcome == "candidate":
            removed_candidates[(key[2], key[3])] += n
summary = {
    "check_changes": dict(check_changes),
    "rows_before": len(before),
    "rows_after": len(after),
    "custody_column_moves": custody_moves,
    "verdicts_before": dict(collections.Counter(verdict(r) for r in before.values())),
    "verdicts_after": dict(collections.Counter(verdict(r) for r in after.values())),
    "moves": dict(moves),
    "moves_by_cause": {m: dict(c) for m, c in by_cause.items()},
    "candidates_by_kind_before": kinds(before),
    "candidates_by_kind_after": kinds(after),
    "safe_to_non_safe": sum(1 for m in moving if m["safe_to_non_safe"]),
    "moving": moving,
    "added_candidates": added_candidates,
    "removed_candidates_by_function": {f"{f}#{n}": c for (f, n), c in sorted(removed_candidates.items())},
}
text = json.dumps(summary, indent=1, ensure_ascii=False)
if len(sys.argv) > 3:
    open(sys.argv[3], "w").write(text + "\n")
print(json.dumps({k: v for k, v in summary.items() if k not in ("moving", "added_candidates", "removed_candidates_by_function")}, indent=1))
