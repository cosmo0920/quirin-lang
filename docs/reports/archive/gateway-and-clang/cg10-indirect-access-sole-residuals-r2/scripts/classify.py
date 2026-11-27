#!/usr/bin/env python3
"""classify.py RUN CENSUS_PREFIX [--json OUT] -- card
CG10-INDIRECT-ACCESS-SOLE-BLOCKERS, task 1: the Unknown functions whose only
refusal family is `indirect memory access is outside the resource projection`
(blocked by it alone), and each of their leaves by the reason the readers gave.

Inputs (one corpus run of scripts/census-run.sh, the `indirect-census` feature
on; its verdicts are byte-identical to the measured binary's):

* RUN/resource-diagnostics.json -- the leaves;
* CENSUS_PREFIX-indirect.jsonl -- the lexical gate's own record of each refusal
  (`site`, `position`, the chains it read with each base's kind and the
  closure's `may_bind`/`never_bound`);
* CENSUS_PREFIX-call-argument.jsonl -- rule W44's own record of each call it
  was consulted for: per actual, the contract position (`Position::of`), whether
  it reads memory, and the recogniser's answer (`recognised` or its refusal
  label), and the probe's outcome;
* RUN/return-census.jsonl -- the refused `return` expressions;
* CENSUS_PREFIX-parity.jsonl -- the typed readers' rows (`elaboration-statement`
  with the elaborator's refusal, `typed-place` with the place decision,
  `pointer-store`/`pointer-store-flow` with rule 10v's decisions).

Rows are keyed by (function, internal line); a key with several rows (the W4
fixed point classifies a body more than once) takes the last row of the stream,
the final pass. A leaf is classified by the first matching arm, in this order:

1. `return-call-actual`: the leaf's line carries a refused `return` (return
   census) whose expression is one call, or one call inside a scalar
   expression, and the call-argument row of that call has every
   memory-reading actual recognised at a `Borrowed` or tail position (the
   statement position would admit it); `return-call-custody` /
   `return-call-unrecognised` / `return-other` otherwise;
2. `pointer-store-*`: a rule-10v row at the line (`pointer-store-flow`
   decision, or the classifier's value class);
3. call statements, from the call-argument row: `uncontracted-callee` (the
   callee's position is `uncontracted`: no contract or summary in force),
   `custody-position:MODE` (an actual reading memory at a custody-taking
   position), `unrecognised-actual:LABEL` (the recogniser refused an actual at an
   admitting position), `call-result-store` (no memory-reading actual: the access
   is the destination, a call result stored into a member), `probe-declined`;
4. statements without a call: the `elaboration-statement`/`typed-place`
   refusal (`place:CODE`, `elaboration:REASON`);
5. `unclassified` otherwise (read by hand).

Functions freed alone by a sub-class: functions all of whose leaves are in it.
"""
import argparse
import collections
import json
import pathlib
import re

IND = "indirect memory access is outside the resource projection"
TEMPLATE = re.compile(r"`[^`]*`")
CUSTODY = {"owned", "consumed", "released", "out", "stored", "removed"}
RETURN_PLACE = "__qcore_returned_value"


def last_rows(path, kind=None, names=None):
    rows = {}
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if kind and row.get("kind") != kind:
                continue
            if names is not None and row.get("function") not in names:
                continue
            rows[(row.get("function"), row.get("line"))] = row
    return rows


def parity_rows(path, names):
    rows = collections.defaultdict(list)
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            if not any(f'"{position}"' in line for position in (
                    "elaboration-statement", "typed-place", "pointer-store", "pointer-store-flow")):
                continue
            row = json.loads(line)
            if row.get("function") in names:
                rows[(row["function"], row.get("line"))].append(row)
    return rows


def call_class(row):
    if row is None:
        return None
    actuals = [a for a in row.get("arguments") or [] if a.get("access")]
    if any(a["position"] == "uncontracted" for a in row.get("arguments") or []):
        return "uncontracted-callee"
    custody = sorted({a["position"] for a in actuals if a["position"] in CUSTODY})
    if custody:
        return "custody-position:" + ",".join(custody)
    unrecognised = sorted({a.get("reason") or "?" for a in actuals if not a.get("recognised")})
    if unrecognised:
        beyond = [a for a in actuals if a["position"] == "beyond"]
        if beyond:
            return "beyond-declared-parameters"
        return "unrecognised-actual:" + ",".join(unrecognised)
    if not actuals:
        return "call-result-store"
    return "probe-declined"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run")
    parser.add_argument("census")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    run = pathlib.Path(args.run)
    data = json.loads((run / "resource-diagnostics.json").read_text())
    sole = []
    for unit in data["units"]:
        for function in unit["functions"]:
            leaves = function.get("diagnostics") or []
            if function["status"] == "Unknown" and leaves and all(
                    leaf["description"] == IND for leaf in leaves):
                sole.append((unit["file"], function["function"], leaves))
    names = {name for _, name, _ in sole}
    calls = last_rows(args.census + "-call-argument.jsonl", "call_argument", names)
    gate = last_rows(args.census + "-indirect.jsonl", None, names)
    returns = last_rows(run / "return-census.jsonl", None, names)
    parity = parity_rows(args.census + "-parity.jsonl", names)
    by_class = collections.Counter()
    leaves_out = []
    per_function = {}
    for unit, name, leaves in sole:
        classes = []
        for leaf in leaves:
            key = (name, leaf["line"])
            rows = parity.get(key, [])
            cls = None
            ret = returns.get(key)
            call = calls.get(key)
            g = gate.get(key)
            if g is not None and (g.get("texts") or [None])[0] == RETURN_PLACE:
                # A whole-call `return` the return position classified as
                # `PLACE = CALL(...)` (C-GATEWAY-RETURN-CALL-W4's synthetic
                # statement), refused by the gate and re-attributed
                # (W43-RETURN-EXPRESSION): W44 is never consulted there.
                if g.get("call_contracted") is False:
                    cls = "return-uncontracted-callee"
                elif g.get("call_contracted") is True:
                    cls = "return-contracted-call"
                else:
                    cls = "return-no-call"
            elif ret is not None:
                cls = "return-other"
            if cls is None:
                flows = [r for r in rows if r.get("position") == "pointer-store-flow"]
                stores = [r for r in rows if r.get("position") == "pointer-store"]
                if flows:
                    cls = "pointer-store-flow:" + str(flows[-1].get("decision"))
                elif stores:
                    refused = [c for c in stores[-1].get("classifications") or []
                               if not c.get("admitted")]
                    if refused:
                        cls = "pointer-store:" + str(refused[-1].get("value"))
            if cls is None and call is not None:
                cls = call_class(call)
            if cls is None and g is not None and not g.get("contains_call") and g.get("chains") \
                    and all(chain.get("steps") and set(chain["steps"]) == {"Dot"}
                            and chain.get("base_kind") in ("AutomaticLocal", "Formal")
                            for chain in g["chains"]):
                # Every access the gate read is a `.`-only selection from a
                # record object the function declares: no pointer is read.
                cls = "record-object-place"
            if cls is None:
                places = [p for r in rows if r.get("position") == "typed-place"
                          for p in r.get("places") or [] if not p.get("admitted")]
                elaboration = [r for r in rows if r.get("position") == "elaboration-statement"]
                if places:
                    cls = "place:" + str(places[-1].get("refusal"))
                elif elaboration:
                    cls = "elaboration:" + TEMPLATE.sub("`_`", str(elaboration[-1].get("new_refusal")))
            if cls is None:
                g = gate.get(key)
                cls = "unclassified:" + (g["position"] if g else "no-gate-row")
            classes.append(cls)
            by_class[cls] += 1
            leaves_out.append({"unit": unit, "function": name, "line": leaf["line"],
                               "origin": leaf.get("origin"), "class": cls,
                               "texts": " ".join((call or gate.get(key) or {}).get("texts") or [])})
        per_function[(unit, name)] = classes
    alone = collections.Counter()
    touching = collections.Counter()
    for key, classes in per_function.items():
        distinct = set(classes)
        if len(distinct) == 1:
            alone[classes[0]] += 1
        for cls in distinct:
            touching[cls] += 1
    # The fixed point if sub-classes were lifted in rank order (by functions
    # freed alone): the functions all of whose leaves are in the lifted union.
    lifted = set()
    cumulative = []
    for cls, _ in alone.most_common():
        lifted.add(cls)
        freed = sum(1 for classes in per_function.values() if set(classes) <= lifted)
        cumulative.append([cls, freed])
    pairs = {
        "card: return-contracted-call + record-object-place": {
            "return-contracted-call", "record-object-place"},
        "callee closure: uncontracted-callee + return-uncontracted-callee": {
            "uncontracted-callee", "return-uncontracted-callee"},
    }
    record = {
        "sole_functions": len(sole),
        "sole_leaves": sum(len(leaves) for _, _, leaves in sole),
        "leaves_by_class": dict(by_class.most_common()),
        "functions_freed_alone_by_class": dict(alone.most_common()),
        "functions_touched_by_class": dict(touching.most_common()),
        "fixed_point_in_rank_order": cumulative,
        "freed_if_lifted_together": {
            name: sum(1 for classes in per_function.values() if set(classes) <= members)
            for name, members in pairs.items()},
    }
    print(json.dumps(record, indent=1))
    if args.json:
        record["leaves"] = leaves_out
        record["functions"] = [{"unit": u, "function": n, "classes": c}
                               for (u, n), c in sorted(per_function.items())]
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
