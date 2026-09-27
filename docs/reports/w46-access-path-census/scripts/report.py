#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: tallies, class re-measurement, form ordering
and external-callee ranking, from access_census.py and predict.py outputs.

usage: report.py CENSUS_OUT PREDICT_OUT RUN_DIR DIAGNOSTICS(.json.gz) CONTRACTS.json.gz OUT_JSON OUT_TXT

Writes `census.json` (schema line first, then one JSON document): leaf and
path tallies, the P2 and P3 form orderings, the refusal causes, the
upper-bound population's fates, and the closure with its external and
in-corpus rankings. The registered classes are classes.py's.
"""

import collections
import gzip
import importlib.util
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("predict", HERE / "predict.py")
predict = importlib.util.module_from_spec(spec)
spec.loader.exec_module(predict)

SCHEMA = "qcore.w46-access-path-census.v1"


def form(path):
    ops = "".join(step["op"] if step["op"] != "[]" else ("[]" if step.get("through") == "pointer" else "[a]")
                  for step in path["steps"])
    return f"{path['root_kind']}:{ops}:{path['access']}"


def main():
    census = pathlib.Path(sys.argv[1])
    predicted = pathlib.Path(sys.argv[2])
    run = pathlib.Path(sys.argv[3])
    diagnostics = predict.load(sys.argv[4])
    contracted, releasing, custody = predict.contracts(sys.argv[5])
    out_json = pathlib.Path(sys.argv[6])
    out_txt = pathlib.Path(sys.argv[7])
    rows = predict.jsonl(census / "leaves.jsonl.gz")
    unit_map = predict.load(census / "unit-map.json")
    records = predict.jsonl(predicted / "prediction.jsonl.gz")
    summary = predict.load(predicted / "prediction-summary.json")
    unit_of = unit_map["unit_of"]
    seq_file = {seq: file for file, seq in unit_of.items()}
    lines = []

    def say(text=""):
        lines.append(text)

    status = {}
    families_of = collections.defaultdict(collections.Counter)
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            status[key] = function["status"]
            for leaf in function["diagnostics"]:
                if leaf["category"] != "unsupported":
                    families_of[key][leaf["category"]] += 1
                else:
                    fam, _ = predict_family(leaf["description"])
                    families_of[key][fam] += 1

    # ---- 1. leaf and path tallies --------------------------------------
    leaf_total = collections.Counter()
    by_scope = collections.Counter()
    by_position = collections.Counter()
    causes_leaves = collections.Counter()
    causes_sole = collections.Counter()
    root_kinds = collections.Counter()
    values = collections.Counter()
    steps_ops = collections.Counter()
    no_access = collections.Counter()
    path_total = 0
    expressible_paths = 0
    typed_steps = 0
    for row in rows:
        n = row["leaves"]
        leaf_total[row["family"]] += n
        group, causes = predict.group_scope(row)
        paths = [p for c in row["constructs"] for p in c["paths"]]
        if not paths:
            no_access[row["family"]] += n
        by_scope[(row["family"], group or "refused")] += n
        positions = {("call-argument" if p["call_argument"] else
                      {"statement": "statement", "condition": "condition", "return": "return"}[c["kind"]])
                     for c in row["constructs"] for p in c["paths"]}
        by_position[(row["family"], "+".join(sorted(positions)) or "none")] += n
        for cause in causes:
            causes_leaves[cause] += n
        for p in paths:
            path_total += 1
            expressible_paths += not p["causes"]
            root_kinds[p["root_kind"]] += 1
            values[(p["value"], p["access"])] += 1
            for step in p["steps"]:
                steps_ops[step["op"]] += 1
                typed_steps += bool(step["type"])

    say("## Leaves by family and scope (leaf counts; a group is every leaf of one family at one line)")
    for key, value in sorted(by_scope.items()):
        say(f"{key[0]:4} {key[1]:8} {value}")
    say(f"total leaves joined: {dict(leaf_total)}")
    say(f"no typed access (lexical reading of a non-access): {dict(no_access)}")
    say()
    say("## Leaves by family and position of their paths")
    for key, value in sorted(by_position.items(), key=lambda kv: -kv[1]):
        say(f"{key[0]:4} {key[1]:40} {value}")
    say()
    say(f"## Paths: {path_total}, expressible {expressible_paths}; steps {sum(steps_ops.values())}, typed {typed_steps}")
    say(f"root kinds: {dict(root_kinds.most_common())}")
    say(f"step ops: {dict(steps_ops.most_common())}")
    say(f"value/access: {dict((f'{a}/{b}', c) for (a, b), c in values.most_common())}")
    say()

    # ---- sole-blocked per refusal cause (P3 scope) ---------------------
    for record in records:
        blockers = record["P3"]["blockers"]
        refused = {b for b in blockers if b.startswith("refused:")}
        others = set(blockers) - refused
        if len(refused) == 1 and not others and not record["P3"]["unserved"]:
            causes_sole[next(iter(refused))[8:]] += 1
    say("## Refusal causes (leaves carrying the cause; functions whose only P3 blocker it is)")
    for cause, count in causes_leaves.most_common():
        say(f"{cause:28} {count:6} leaves  {causes_sole.get(cause, 0):4} sole-blocked functions")
    say()

    # ---- 2. registered classes: classes.py (their own registering scripts)

    # ---- 3. predictions --------------------------------------------------
    say("## Prediction summary")
    for scope, value in summary["scopes"].items():
        say(f"{scope}: freed {value['freed']} {value['verdicts']} rounds {[(r['round'], r['freed'], r['safe']) for r in value['rounds']]}")
        for library, tally in value["by_library"].items():
            say(f"    {library:12} {tally}")
    say()

    # Why the upper-bound population is not freed (P3, same-unit).
    bound = [r for r in records
             if set(families_of[(r["file"], r["function"])]) <= {"IND", "COND", "RET"}]
    why = collections.Counter()
    for r in bound:
        hit = r["P3"]["same-unit"]
        if hit:
            why["freed:" + hit["verdict"]] += 1
            continue
        blockers = r["P3"]["blockers"]
        if blockers:
            # Precedence: the typed model's own refusal, then a condition or
            # return refused for a non-access reason, then a custody position.
            refused = sorted(b for b in blockers if b.startswith("refused:"))
            if refused:
                why["refused path (" + (refused[0][8:] if len(refused) == 1 else "several causes") + ")"] += 1
            else:
                why[sorted(blockers)[0]] += 1
            continue
        unserved = sorted({u[1] for u in r["P3"]["unserved"]})
        why["residual call: " + "+".join(unserved)] += 1
    say(f"## Upper-bound population (blocked only by IND/COND/RET): {len(bound)}")
    for key, value in why.most_common():
        say(f"  {key:60} {value}")
    say()

    # Reasons of predicted Unknown.
    reasons = collections.Counter()
    for r in records:
        hit = r["P3"]["same-unit"]
        if hit and hit["verdict"] != "Safe":
            for part in hit["reason"].split("; "):
                reasons[re.sub(r"`[^`]*`", "X", re.sub(r"line \d+", "line N", part))] += 1
    say("## Reasons of non-Safe predictions (P3, same-unit; reason parts, up to four per function)")
    for key, value in reasons.most_common(12):
        say(f"  {value:5}  {key}")
    say()

    # ---- 5. path forms, ordered greedily by Safe-predicted functions freed
    orders = {}
    for scope in ("P2", "P3"):
        by_function_forms = {}
        for r in records:
            if (r[scope]["same-unit"] or {}).get("verdict") != "Safe":
                continue
            key = (r["file"], r["function"])
            forms = set()
            for row in rows_by_function(rows, key):
                for c in row["constructs"]:
                    for p in c["paths"]:
                        where = "call-argument" if p["call_argument"] else c["kind"]
                        forms.add(form(p) + ("" if scope == "P2" else "@" + where))
            by_function_forms[key] = forms or {"(no typed access)"}
        admitted = set()
        order = []
        remaining = dict(by_function_forms)
        while remaining:
            gains = collections.Counter()
            for key, forms in remaining.items():
                need = forms - admitted
                if len(need) == 1:
                    gains[next(iter(need))] += 1
            if not gains:
                gains = collections.Counter(f for forms in remaining.values() for f in forms - admitted)
            # Deterministic: most functions first, then the form's spelling.
            pick = min(gains.items(), key=lambda item: (-item[1], item[0]))[0]
            admitted.add(pick)
            freed = [k for k, forms in remaining.items() if forms <= admitted]
            for k in freed:
                remaining.pop(k)
            order.append({"form": pick, "functions_freed": len(freed)})
        orders[scope] = order
        say(f"## {scope} path forms in greedy order of Safe-predicted functions freed (root:steps:access{'' if scope == 'P2' else '@position'})")
        running = 0
        for item in order:
            running += item["functions_freed"]
            say(f"  {item['form']:52} +{item['functions_freed']:4}  cumulative {running}")
        say()
    order = orders

    # ---- 4. closure and external ranking --------------------------------
    say("## Closure")
    for scope, value in summary["scopes"].items():
        say(f"  {scope}: rounds {value['rounds']}")
    # Functions whose every leaf P3 removes but a call is unserved after the
    # same-unit closure: split by what the unserved callees are.
    closure_safe = {(r["file"], r["function"]) for r in records
                    if (r["P3"]["same-unit"] or {}).get("verdict") == "Safe"}
    call_only = collections.Counter()
    external_alone = collections.Counter()
    in_corpus_alone = collections.Counter()
    external_any = collections.Counter()
    for r in records:
        if r["P3"]["same-unit"] is not None or r["P3"]["blockers"]:
            continue
        unserved = [u for u in r["P3"]["unserved"]
                    if not (u[1] == "same-unit" and (r["file"], u[0]) in closure_safe)]
        kinds = {u[1] for u in unserved}
        if not unserved:
            call_only["served after closure (verdict not Safe or later round)"] += 1
            continue
        if kinds <= {"external"}:
            call_only["external only"] += 1
            for u in unserved:
                external_any[u[0]] += 1
            if len(unserved) == 1:
                external_alone[unserved[0][0]] += 1
        elif kinds <= {"same-unit", "cross-unit", "cross-unit-ambiguous", "same-unit-unreported-refused"}:
            call_only["in-corpus only"] += 1
            if len(unserved) == 1:
                in_corpus_alone[(unserved[0][0], unserved[0][1], unserved[0][2])] += 1
        else:
            call_only["mixed or indirect: " + "+".join(sorted(kinds))] += 1
    say("  P3 call-blocked functions (every leaf removed, a call unserved after the same-unit closure):")
    for key, value in call_only.most_common():
        say(f"    {key:60} {value}")
    say("  external callees, by functions they unblock alone (then by functions naming them):")
    for name, count in sorted(external_any.items(), key=lambda kv: (-external_alone.get(kv[0], 0), -kv[1], kv[0]))[:25]:
        say(f"    {name:40} alone {external_alone.get(name, 0):3}  named by {count:3}")
    say("  in-corpus callees, by functions they unblock alone, with what blocks the callee itself (P3):")
    by_name = collections.defaultdict(list)
    for r in records:
        by_name[r["function"]].append(r)
    callee_blockers = {}
    for (name, kind, st), count in in_corpus_alone.most_common(20):
        own = by_name.get(name, [])
        if own:
            b = collections.Counter()
            for r in own:
                b.update({k.split(":")[0] if not k.startswith("refused:") else k: v
                          for k, v in r["P3"]["blockers"].items()})
                if r["P3"]["same-unit"]:
                    b["freed:" + r["P3"]["same-unit"]["verdict"]] += 1
                unserved = [u for u in r["P3"]["unserved"]]
                if unserved and not r["P3"]["blockers"]:
                    b["calls"] += len(unserved)
            text = ", ".join(f"{k} {v}" for k, v in b.most_common(4))
        else:
            text = "not an Unknown function of the corpus"
        callee_blockers[name] = text
        say(f"    {name:36} {kind:10} {st or '-':8} {count:3}   callee: {text}")
    closure_doc = {"call_blocked": dict(call_only),
                   "external": [{"callee": n, "alone": external_alone.get(n, 0), "named_by": c}
                                for n, c in sorted(external_any.items(), key=lambda kv: (-external_alone.get(kv[0], 0), -kv[1], kv[0]))],
                   "in_corpus_alone": [{"callee": n, "kind": k, "status": s, "alone": c,
                                        "callee_blockers": callee_blockers.get(n)}
                                       for (n, k, s), c in in_corpus_alone.most_common()]}
    document = {"schema": SCHEMA, "form_order": order, "closure": closure_doc,
                "prediction": summary["scopes"], "upper_bound_population": {"functions": len(bound), "why": dict(why)},
                "refusal_causes": {c: {"leaves": n, "sole_blocked": causes_sole.get(c, 0)} for c, n in causes_leaves.items()},
                "leaves": {"joined": dict(leaf_total), "no_typed_access": dict(no_access),
                           "by_scope": {f"{a}/{b}": v for (a, b), v in by_scope.items()}},
                "paths": {"total": path_total, "expressible": expressible_paths,
                          "root_kinds": dict(root_kinds), "step_ops": dict(steps_ops)}}
    out_json.write_text(json.dumps({"schema": SCHEMA}) + "\n" + json.dumps(document, indent=1) + "\n")
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines))


_rows_index = {}


def rows_by_function(rows, key):
    if not _rows_index:
        for row in rows:
            _rows_index.setdefault((row["file"], row["function"]), []).append(row)
    return _rows_index.get(key, [])


def predict_family(description):
    if description == "indirect memory access is outside the resource projection":
        return "IND", None
    prefix = "unmodeled call, aggregate or indirect access in "
    if description.startswith(prefix):
        rest = description[len(prefix):]
        return ("RET" if rest == "return expression" else "COND"), None
    if description.startswith("call to uncontracted function"):
        return "UNC", None
    return "OTHER", None


if __name__ == "__main__":
    main()
