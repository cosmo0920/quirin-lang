#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: per-function prediction and callee closure.

usage: predict.py CENSUS_OUT RUN_DIR DIAGNOSTICS(.json.gz) CONTRACTS.json.gz OUT_DIR

Reads access_census.py's `leaves.jsonl.gz`, `functions.jsonl.gz` and
`unit-map.json`, the baseline diagnostics and contracts, and the census run's
lexical streams (for the analyzer's own `may_bind` premise). Writes
`prediction.jsonl.gz` (schema line first; every Unknown function with its
blockers per scope, its unserved callees and, when freed, its predicted
verdict, reason, obligations and closure round) and
`prediction-summary.json` (counts per scope and closure variant, by library,
with rounds). Closure variants: `same-unit` serves a call only through a
declared contract or a same-unit Safe callee (the existing summary rule);
`cross-unit` also through a Safe callee another unit defines (no rule does
this today); `cross-unit-ceiling` takes every freed function Safe, the
closure's upper bound.

Scopes. P2 removes an `IND` leaf group whose every construct is a statement
(or `for` clause) and every path in it is expressible and outside a call
argument. P3 removes every `IND`, `COND` and `RET` group whose every path is
expressible, in any position. An `IND` group with no typed access at all (the
lexical gate's reading of an unevaluated or non-access token) is removable in
the scope of its position. A `COND` group stays refused when the condition
desugaring's own census code at the leaf is not about an access or a call
(`ACCESS_CODES`), or when it has neither a path nor a call; a `RET` group with
no path stays refused (its refusal is a call, aggregate or form, not an
access). A group's calls stay: after the access is modelled
the statement is a call, served by a declared contract, a same-unit summary
of a Safe callee (the existing rule), or else a blocker.

Verdict. A function is *freed* in a scope when every leaf it has is removed
by the scope and every residual call is served. Its accesses then carry: per
pointer the path reads through, non-null and live; a non-pointer store is
frame-neutral; a pointer read is a borrowed alias without custody. An
untracked root (`may_bind` false: the analyzer never binds it to a resource
identity) and every pointer loaded from memory discharge by scope, exactly
as the admitted routes' scope string says ("never the liveness or nullness of
an object the projection does not track"). A tracked root (`may_bind` true,
or unobserved: fail-closed) needs one known non-null live identity: it is
predicted discharged only when a controlling expression at an earlier line
null-tests it after its last assignment. A tracked root passed to a
releasing call earlier in the same block, with no assignment between, is a
use-after-release candidate: provisional Unsafe.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

SCHEMA = "qcore.w46-access-path-prediction.v1"
NEUTRAL = {"__builtin_expect"}
UNC = re.compile(r"call to uncontracted function `([^`]*)`")


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def jsonl(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        rows = [json.loads(line) for line in handle]
    return rows[1:] if rows and "schema" in rows[0] else rows


def contracts(path):
    data = load(path)
    names, releasing, custody = set(), set(), set()
    for source in data["sources"].values():
        for block in re.split(r"(?=\bextern\s+fn\s)", source):
            match = re.match(r"extern\s+fn\s+([A-Za-z_]\w*)", block)
            if not match:
                continue
            name = match.group(1)
            names.add(name)
            if "resource_released" in block:
                releasing.add(name)
            if re.search(r"resource_(released|consumed|stored|published|out)\b", block):
                custody.add(name)
    for module in data["contract_inclusion"]["modules"].values():
        for name, parents in module.get("inherited", {}).items():
            names.add(name)
            for parent in parents:
                if parent.get("declaration") in releasing:
                    releasing.add(name)
                if parent.get("declaration") in custody:
                    custody.add(name)
    return names, releasing, custody


def may_bind_sets(run):
    """Per (unit_seq, function): names the analyzer's `may_bind` answers
    true for, and names it answered false for, from every lexical stream."""
    may = collections.defaultdict(set)
    never = collections.defaultdict(set)
    ident = re.compile(r"[A-Za-z_]\w*$")

    def stream(name):
        path = run / name
        if not path.exists():
            return []
        with open(path) as handle:
            return [json.loads(line) for line in handle]

    for record in stream("statement-census.jsonl"):
        key = (record["unit_seq"], record["function"])
        binds = set(record.get("may_bind", []))
        may[key] |= binds
        for text in record.get("texts", []):
            if ident.match(text) and text not in binds:
                never[key].add(text)
    for name in ("indirect-census.jsonl", "return-census.jsonl"):
        for record in stream(name):
            key = (record["unit_seq"], record["function"])
            for chain in record.get("chains", []):
                (may if chain.get("base_may_bind") else never)[key].add(chain.get("base"))
    for record in stream("chain-census.jsonl"):
        key = (record["unit_seq"], record["function"])
        for occurrence in record.get("occurrences", []):
            (may if occurrence.get("root_may_bind") else never)[key].add(occurrence.get("root"))
    return may, never


# Condition-call census outcomes (W44-BRANCH-CONDITION-CALLS's codes) that
# say the condition was refused for an access or a call: the typed layer and
# the residual-call rule decide those. Every other code (a type operator, an
# assignment, braces, a custody result, recursion, ...) stays refused.
ACCESS_CODES = {"no-call", "call-free-leaf-refused", "call-indirect-argument", "call-uncontracted", "admitted"}
CONDITION_POSITION = {"if": "if", "while": "while", "for": "for", "do": "do-while", "switch": "switch"}


def condition_census_codes(run):
    codes = collections.defaultdict(set)
    path = run / "condition-call-census.jsonl"
    with open(path) as handle:
        for line in handle:
            record = json.loads(line)
            outcome = record["outcome"]
            if outcome == "call-refused":
                reason = record.get("reason") or ""
                if reason.startswith("indirect memory access"):
                    outcome = "call-indirect-argument"
                elif reason.startswith("call to uncontracted function"):
                    outcome = "call-uncontracted"
                else:
                    outcome = "call-other"
            codes[(record["unit_seq"], record["function"], record["line"], record["position"])].add(outcome)
    return codes


def group_scope(row):
    """(scope, detail) for one leaf group: 'P2', 'P3' or None when refused."""
    paths = [p for c in row["constructs"] for p in c["paths"]]
    causes = sorted({cause for p in paths for cause in p["causes"]})
    kinds = {c["kind"] for c in row["constructs"]}
    in_call = any(p["call_argument"] for p in paths)
    if causes:
        return None, causes
    if row["family"] == "IND" and kinds <= {"statement"} and not in_call:
        return "P2", []
    return "P3", []


class Oracle:
    def __init__(self, diagnostics, unit_map, contracted):
        self.contracted = contracted
        self.status = {}
        self.defined_in = collections.defaultdict(list)
        self.unc_in_unit = collections.defaultdict(set)
        for unit in diagnostics["units"]:
            for function in unit["functions"]:
                self.status[(unit["file"], function["function"])] = function["status"]
                for leaf in function["diagnostics"]:
                    match = UNC.match(leaf["description"])
                    if match:
                        self.unc_in_unit[unit["file"]].add(match.group(1))
        self.storage = unit_map["definitions"]
        for file, names in self.storage.items():
            for name, storage in names.items():
                if (file, name) in self.status:
                    self.defined_in[name].append((file, storage))

    def callee(self, file, name):
        """Classify one residual callee of a function in `file`."""
        if name in NEUTRAL:
            return ("neutral", None)
        if name == "<indirect>":
            return ("indirect", None)
        if name in self.contracted:
            return ("contracted", None)
        if (file, name) in self.status:
            return ("same-unit", (file, name))
        if name in self.storage.get(file, {}):
            # Defined in this unit but outside the audited scope (a header
            # inline): served iff the corpus never names it uncontracted here.
            if name in self.unc_in_unit[file]:
                return ("same-unit-unreported-refused", None)
            return ("same-unit-unreported", None)
        external = [(f, s) for f, s in self.defined_in.get(name, []) if s != "static"]
        if len(external) == 1:
            return ("cross-unit", external[0][0:1] + (name,))
        if len(external) > 1:
            return ("cross-unit-ambiguous", None)
        return ("external", None)


def main():
    census = pathlib.Path(sys.argv[1])
    run = pathlib.Path(sys.argv[2])
    diagnostics = load(sys.argv[3])
    contracted, releasing, custody = contracts(sys.argv[4])
    out = pathlib.Path(sys.argv[5])
    out.mkdir(parents=True, exist_ok=True)
    unit_map = load(census / "unit-map.json")
    unit_of = unit_map["unit_of"]
    rows = jsonl(census / "leaves.jsonl.gz")
    function_rows = {(r["file"], r["function"]): r for r in jsonl(census / "functions.jsonl.gz")}
    may, never = may_bind_sets(run)
    condition_codes = condition_census_codes(run)
    oracle = Oracle(diagnostics, unit_map, contracted)

    groups = collections.defaultdict(list)
    for row in rows:
        groups[(row["file"], row["function"])].append(row)

    functions = {}
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            functions[key] = {"library": unit.get("library"), "file": unit["file"],
                              "function": function["function"], "status": function["status"],
                              "diagnostics": function["diagnostics"]}

    def fate(key, scope):
        """Blockers left after the scope, and the residual callees."""
        entry = functions[key]
        blockers = collections.Counter()
        callees = []
        grouped = {(r["line"], r["family"], r["condition"]) for r in groups.get(key, [])}
        for leaf in entry["diagnostics"]:
            category = leaf["category"]
            if category != "unsupported":
                blockers[category] += 1
                continue
            description = leaf["description"]
            match = UNC.match(description)
            if match:
                callees.append(match.group(1))
                continue
            if description.startswith("indirect memory access") or description.startswith(
                    "unmodeled call, aggregate or indirect access in "):
                continue  # judged per group below
            blockers["other:" + description.split("`")[0].strip()] += 1
        for row in groups.get(key, []):
            group, causes = group_scope(row)
            if group is None:
                for cause in causes:
                    blockers["refused:" + cause] += 1
                continue
            if scope == "P2" and group != "P2":
                blockers["out-of-scope:" + ("call-argument" if row["family"] == "IND" else row["family"])] += 1
                continue
            paths = [p for c in row["constructs"] for p in c["paths"]]
            if row["family"] == "COND":
                # The condition desugaring's own answer at this leaf: a code
                # that is not about an access or a call stays refused.
                codes = condition_codes.get((unit_of[key[0]], key[1], row["line"],
                                             CONDITION_POSITION[row["condition"]]), set())
                other = codes - ACCESS_CODES
                if other:
                    for code in sorted(other):
                        blockers["condition:" + code] += 1
                    continue
                if not paths and not any(c["calls"] for c in row["constructs"]):
                    blockers["condition:no-access"] += 1
                    continue
            if row["family"] == "RET" and not paths:
                # A return refused for its call, aggregate or form, not an access.
                blockers["return:no-access"] += 1
                continue
            for construct in row["constructs"]:
                callees.extend(construct["calls"])
                if scope == "P3":
                    for path in construct["paths"]:
                        if path["call_argument"] and path["value"] in ("pointer", "address"):
                            for callee, _ in construct["call_args"]:
                                if callee in custody:
                                    blockers["custody-position"] += 1
                                    break
        return blockers, callees

    def verdict(key):
        """Predicted verdict of a freed function from its access obligations."""
        entry = functions[key]
        seq = unit_of[entry["file"]]
        fkey = (seq, entry["function"])
        constructs = function_rows.get(key, {}).get("constructs", [])
        tests = [(min(c["lines"]) if c["lines"] else 0, set(c["null_tested"])) for c in constructs if c["kind"] == "condition"]
        assigns = collections.defaultdict(list)
        releases = []
        for c in constructs:
            line = min(c["lines"]) if c["lines"] else 0
            for name in c["assigned"]:
                assigns[name].append((line, c["block"]))
            for callee, arguments in c["call_args"]:
                if callee in releasing:
                    for name in arguments:
                        if name:
                            releases.append((name, line, c["block"]))
        reasons = []
        unsafe = []
        obligations = collections.Counter()
        for row in groups.get(key, []):
            for construct in row["constructs"]:
                line = row["line"]
                for path in construct["paths"]:
                    if path["hops"] == 0:
                        obligations["no-pointer-hop"] += 1
                        continue
                    loaded = path["hops"] - (1 if path["root_hop"] else 0)
                    if loaded:
                        obligations["loaded-pointer:scope"] += loaded
                    if not path["root_hop"]:
                        continue
                    root = path["root"]
                    if path["root_kind"] == "global":
                        obligations["root-global:scope"] += 1
                        continue
                    if path["root_kind"] not in ("local", "formal"):
                        obligations["root-other"] += 1
                        reasons.append(f"root {path['root_kind']} `{root}`")
                        continue
                    tracked = root in may[fkey] or root not in never[fkey]
                    if not tracked:
                        obligations["root-untracked:scope"] += 1
                        continue
                    last = max([l for l, _ in assigns.get(root, []) if l < line] or [0])
                    guarded = any(last <= l < line and root in names for l, names in tests)
                    block = construct["block"]
                    for name, rline, rblock in releases:
                        if name == root and rline < line and block and rblock and rblock[0] == block[0] \
                                and rblock[1] < block[1] and not any(
                                    ablock and ablock[0] == block[0] and rblock[1] < ablock[1] <= block[1]
                                    for _, ablock in assigns.get(root, [])):
                            unsafe.append(f"`{root}` released at line {rline} before its access at line {line}")
                    if guarded:
                        obligations["root-tracked-guarded"] += 1
                    else:
                        obligations["root-tracked-unguarded"] += 1
                        reasons.append(f"tracked root `{root}` not null-tested before line {line}"
                                       + ("" if root in may[fkey] else " (may_bind unobserved: fail-closed)"))
        if unsafe:
            return "provisional Unsafe", "use-after-release candidate: " + "; ".join(sorted(set(unsafe))), obligations
        if reasons:
            return "Unknown", "; ".join(sorted(set(reasons))[:4]), obligations
        return "Safe", "every pointer hop is untracked, loaded or a null-tested tracked root", obligations

    missing = sorted({key[0] for key in functions if key[0] not in unit_of})  # data-only units have no key
    if missing and "--partial" not in sys.argv:
        raise SystemExit(f"units without a census match: {missing}")
    unknown = [key for key, entry in functions.items() if entry["status"] == "Unknown" and key[0] in unit_of]
    safe_now = {key for key, entry in functions.items() if entry["status"] == "Safe"}
    results = {}
    closure = {}
    for scope in ("P2", "P3"):
        for variant in ("same-unit", "cross-unit", "cross-unit-ceiling"):
            safe = set(safe_now)
            predicted = {}
            rounds = []
            round_number = 0
            while True:
                added = []
                for key in unknown:
                    if key in predicted:
                        continue
                    blockers, callees = fate(key, scope)
                    if blockers:
                        continue
                    served = True
                    for name in callees:
                        kind, target = oracle.callee(key[0], name)
                        if kind in ("neutral", "contracted", "same-unit-unreported"):
                            continue
                        if kind == "same-unit" and target in safe:
                            continue
                        if kind == "cross-unit" and variant != "same-unit" and target in safe:
                            continue
                        served = False
                        break
                    if not served:
                        continue
                    result = verdict(key)
                    if variant == "cross-unit-ceiling":
                        # The closure's ceiling: every freed function taken Safe.
                        result = ("Safe", "ceiling: " + result[0], result[2])
                    predicted[key] = (result, round_number)
                    if result[0] == "Safe":
                        added.append(key)
                rounds.append({"round": round_number,
                               "freed": sum(1 for v in predicted.values() if v[1] == round_number),
                               "safe": len(added)})
                if not added:
                    break
                safe |= set(added)
                round_number += 1
            closure[(scope, variant)] = {"predicted": predicted, "rounds": rounds}

    # Per-function records.
    records = []
    for key in unknown:
        entry = functions[key]
        record = {"library": entry["library"], "file": entry["file"], "function": entry["function"]}
        for scope in ("P2", "P3"):
            blockers, callees = fate(key, scope)
            classes = collections.Counter(oracle.callee(key[0], name)[0] for name in callees)
            unserved = set()
            for name in callees:
                kind, target = oracle.callee(key[0], name)
                if kind in ("neutral", "contracted", "same-unit-unreported"):
                    continue
                if kind == "same-unit" and target in safe_now:
                    continue
                unserved.add((name, kind, None if target is None else functions.get(target, {}).get("status")))
            record[scope] = {
                "blockers": dict(blockers),
                "callees": dict(classes),
                "unserved": sorted([list(item) for item in unserved], key=lambda item: [str(x) for x in item]),
            }
            for variant in ("same-unit", "cross-unit"):
                hit = closure[(scope, variant)]["predicted"].get(key)
                record[scope][variant] = None if hit is None else {
                    "verdict": hit[0][0], "reason": hit[0][1], "round": hit[1],
                    "obligations": dict(hit[0][2])}
        records.append(record)

    with gzip.open(out / "prediction.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": SCHEMA, "kind": "function-predictions"}) + "\n")
        for record in records:
            handle.write(json.dumps(record) + "\n")

    summary = {"schema": SCHEMA, "unknown_functions": len(unknown), "scopes": {}}
    for (scope, variant), value in closure.items():
        tally = collections.Counter()
        by_library = collections.defaultdict(collections.Counter)
        round0 = collections.Counter()
        for key, (result, round_number) in value["predicted"].items():
            tally[result[0]] += 1
            by_library[functions[key]["library"]][result[0]] += 1
            if round_number == 0:
                round0[result[0]] += 1
        summary["scopes"][f"{scope}/{variant}"] = {
            "freed": sum(tally.values()),
            "verdicts": dict(tally),
            "round0": dict(round0),
            "rounds": value["rounds"],
            "by_library": {k: dict(v) for k, v in sorted(by_library.items())},
        }
    (out / "prediction-summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
