#!/usr/bin/env python3
"""Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: read the destination-suffix
census and forecast the repair.

Usage: suffix_census.py CENSUS_JSONL DIAGNOSTICS_JSON [--json OUT] [--text OUT]

CENSUS_JSONL is the `QCORE_DESTINATION_SUFFIX_CENSUS` stream of a BEFORE-tree
census run; DIAGNOSTICS_JSON is that run's (or the byte-identical BEFORE run's)
`resource-diagnostics.json`.

Join: the gateway carries no unit label at the classifier's depth, so each
record's `unit_seq` is matched to the document unit whose function list
contains every function the sequence's records name; the one ambiguous
sequence of the measured run is broken by preprocessed-line overlap with the
unit's diagnostic lines. Records repeat (a function is analysed more than once:
W4 summary rounds, contextual recursion), so they are de-duplicated on
(unit, function, line, clause, texts).

Classes:

* `declarator` — the prefix before the named suffix is a declarator prefix
  (`is_declarator_prefix`): the repair names it exactly as before;
* `sub-object` — every other record: the reader used to name the suffix and
  now names nothing, and the lexical gate refuses the statement.

Forecast, per sub-object record, from the classifier's BEFORE outcome:

* `unchanged-pre-gate` — refused BEFORE by a rule that runs before the lexical
  gate (braces, formal mutation, shadowing, an increment, a withdrawn
  declaration), or admitted by a typed route before the gate that reads no
  destination (the scalar-field no-op, W42's member store): same answer;
* `unchanged-gate` — refused BEFORE by the lexical gate itself;
* `exchange` — refused BEFORE for another reason after the gate: the leaf's
  description becomes the gate's at the same line;
* `typed` — admitted BEFORE, and a typed route consulted after the gate
  refusal admits it (no call): lowered as a typed access;
* `refused` — admitted BEFORE and refused AFTER: the function gains a
  refusal, so a BEFORE-Safe function becomes Unknown.
"""

import collections
import json
import pathlib
import sys

GATE = "indirect memory access is outside the resource projection"
PRE_GATE = (
    "braced initializer, expression, or declaration is not modeled",
    "unmodeled expression may mutate a formal resource identity",
    "local declaration may shadow a formal resource identity",
    "unmodeled increment or decrement expression",
)
PRE_GATE_PARTS = (
    "outside its declared literal-argument premise",
    "whose declared contract was withdrawn",
)


def load(path):
    records = []
    with open(path) as stream:
        for line in stream:
            records.append(json.loads(line))
    return records


def join_units(records, document):
    units = document["units"]
    by_function = collections.defaultdict(set)
    lines = collections.defaultdict(set)
    for index, unit in enumerate(units):
        for function in unit["functions"]:
            by_function[function["function"]].add(index)
            for diagnostic in function["diagnostics"]:
                lines[(index, function["function"])].add(diagnostic["line"])
    sequences = collections.defaultdict(list)
    for record in records:
        sequences[record["unit_seq"]].append(record)
    mapping, ambiguous = {}, []
    for seq, members in sequences.items():
        candidates = None
        for record in members:
            found = by_function.get(record["function"], set())
            candidates = found if candidates is None else candidates & found
        candidates = sorted(candidates or [])
        if len(candidates) == 1:
            mapping[seq] = candidates[0]
            continue
        # Break a tie by line overlap with each candidate's diagnostic lines.
        scores = []
        for candidate in candidates:
            score = sum(
                1
                for record in members
                if lines[(candidate, record["function"])]
                and min(lines[(candidate, record["function"])]) - 400
                <= record["line"]
                <= max(lines[(candidate, record["function"])]) + 400
            )
            scores.append((score, candidate))
        scores.sort(reverse=True)
        ambiguous.append(
            {"unit_seq": seq, "candidates": [units[c]["file"] for c in candidates], "scores": scores}
        )
        if scores and (len(scores) == 1 or scores[0][0] > scores[1][0]):
            mapping[seq] = scores[0][1]
    return mapping, ambiguous


def forecast(record):
    outcome = record["outcome"]
    description = record["description"]
    guess = record["forecast"]
    if outcome == "Unknown":
        if description == GATE:
            return "unchanged-gate"
        if description in PRE_GATE or any(part in description for part in PRE_GATE_PARTS):
            return "unchanged-pre-gate"
        return "exchange"
    if guess["scalar_field_noop"] or guess["scalar_member_store"]:
        return "unchanged-pre-gate"
    if not guess["contains_call"] and (guess["typed_scalar"] or guess["chain"] or guess["stepped"]):
        return "typed"
    return "refused"


def main():
    census_path, diagnostics_path = sys.argv[1], sys.argv[2]
    options = sys.argv[3:]
    json_out = text_out = None
    while options:
        if options[0] == "--json":
            json_out = options[1]
        elif options[0] == "--text":
            text_out = options[1]
        else:
            raise SystemExit(f"unknown option {options[0]}")
        options = options[2:]
    raw = load(census_path)
    document = json.loads(pathlib.Path(diagnostics_path).read_text())
    units = document["units"]
    mapping, ambiguous = join_units(raw, document)
    distinct = {}
    unjoined = 0
    for record in raw:
        unit = mapping.get(record["unit_seq"])
        if unit is None:
            unjoined += 1
            continue
        key = (unit, record["function"], record["line"], record["clause"], tuple(record["texts"]))
        distinct.setdefault(key, record)
    status = {}
    diagnostics = {}
    for index, unit in enumerate(units):
        for function in unit["functions"]:
            status[(index, function["function"])] = function["status"]
            diagnostics[(index, function["function"])] = function["diagnostics"]

    lines = []
    out = {"raw_records": len(raw), "distinct_records": len(distinct), "unjoined_raw": unjoined,
           "ambiguous_sequences": ambiguous}
    lines.append(f"raw records {len(raw)}, distinct {len(distinct)}, unjoined raw {unjoined}")
    classes = collections.Counter(
        "declarator" if record["declarator_prefix"] else "sub-object" for record in distinct.values()
    )
    out["classes"] = dict(classes)
    lines.append(f"classes: {dict(classes)}")

    declarators = [record for record in distinct.values() if record["declarator_prefix"]]
    shapes = collections.Counter(
        ("comma" if "," in record["prefix"] else "no-comma")
        + ("+star" if "*" in record["prefix"] else "")
        for record in declarators
    )
    out["declarator_shapes"] = dict(shapes)
    out["declarator_outcomes"] = dict(
        collections.Counter(record["outcome"] for record in declarators)
    )
    lines.append(f"declarator prefix shapes: {dict(shapes)}")
    comma_examples = sorted(
        {" ".join(record["texts"][: record["equal"]]) for record in declarators if "," in record["prefix"]}
    )
    out["declarator_comma_examples"] = comma_examples[:40]

    sub = [(key, record) for key, record in distinct.items() if not record["declarator_prefix"]]
    comparison = [
        (key, record)
        for key, record in sub
        if record["texts"][record["equal"] + 1 : record["equal"] + 2] == ["="]
    ]
    out["sub_object_first_equal_is_a_comparison"] = len(comparison)
    last = collections.Counter(record["prefix"][-1] for _, record in sub)
    out["sub_object_last_prefix_token"] = dict(last.most_common())
    lines.append(f"sub-object: {len(sub)} distinct; first '=' is '==': {len(comparison)}")
    lines.append(f"  last prefix token: {dict(last.most_common(12))}")
    outcomes = collections.Counter(
        (record["outcome"], record["description"]) for _, record in sub
    )
    out["sub_object_outcomes"] = [
        {"outcome": outcome, "description": description, "count": count}
        for (outcome, description), count in outcomes.most_common()
    ]
    admitted = [(key, record) for key, record in sub if record["admitted"]]
    out["sub_object_admitted"] = len(admitted)
    lines.append(f"  admitted BEFORE: {len(admitted)}; outcomes {dict(collections.Counter(r['outcome'] for _, r in admitted))}")
    kinds = collections.Counter(record["named_root"]["kind"] for _, record in admitted)
    out["admitted_named_root_kind"] = dict(kinds)
    later = collections.Counter(
        (record["named_root"]["kind"], record["named_root"]["later_mentions"] > 0,
         record["named_root"]["later_call_argument"])
        for _, record in admitted
    )
    out["admitted_named_root_later"] = [
        {"kind": k, "later_mention": m, "later_call_argument": a, "count": c}
        for (k, m, a), c in sorted(later.items())
    ]
    lines.append(f"  admitted named-root kinds: {dict(kinds)}")
    lines.append("  admitted (kind, later mention, later call argument): "
                 + ", ".join(f"{k}/{m}/{a}: {c}" for (k, m, a), c in sorted(later.items())))

    forecasts = collections.Counter()
    per_function = collections.defaultdict(lambda: collections.Counter())
    records_by_function = collections.defaultdict(list)
    for key, record in sub:
        verdict = forecast(record)
        forecasts[verdict] += 1
        per_function[(key[0], key[1])][verdict] += 1
        records_by_function[(key[0], key[1])].append((key, record, verdict))
    out["forecast_records"] = dict(forecasts)
    lines.append(f"  forecast per record: {dict(forecasts)}")

    moves = []
    affected = collections.Counter()
    for function_key, counts in sorted(per_function.items()):
        before = status.get(function_key)
        affected[before] += 1
        if counts["refused"] == 0:
            continue
        leaks = [
            diagnostic for diagnostic in diagnostics.get(function_key, [])
            if diagnostic["category"] == "leak"
        ]
        predicted = {"Safe": "Unknown", "Unknown": "Unknown"}.get(before, "Unsafe or Unknown")
        moves.append({
            "unit": units[function_key[0]]["file"],
            "function": function_key[1],
            "before": before,
            "predicted": predicted,
            "refused_records": [
                {"line": key[2], "clause": key[3], "texts": " ".join(record["texts"]),
                 "outcome": record["outcome"], "named": record["place"],
                 "named_root_kind": record["named_root"]["kind"],
                 "later_mentions": record["named_root"]["later_mentions"],
                 "later_call_argument": record["named_root"]["later_call_argument"]}
                for key, record, verdict in records_by_function[function_key]
                if verdict == "refused"
            ],
            "leaks_before": [diagnostic["line"] for diagnostic in leaks],
        })
    out["functions_with_sub_object_records_by_status"] = dict(affected)
    out["functions_gaining_a_refusal"] = moves
    by_before = collections.Counter(move["before"] for move in moves)
    out["functions_gaining_a_refusal_by_status"] = dict(by_before)
    lines.append(f"functions with sub-object records, by BEFORE status: {dict(affected)}")
    lines.append(f"functions gaining a refusal, by BEFORE status: {dict(by_before)}")
    safe = [move for move in moves if move["before"] == "Safe"]
    lines.append(f"Safe functions whose certification relied on a misnamed destination: {len(safe)}")
    for move in safe:
        lines.append(f"  {move['unit']} {move['function']}:")
        for record in move["refused_records"]:
            lines.append(
                f"    L{record['line']} {record['outcome']} named `{record['named']}` "
                f"({record['named_root_kind']}, later {record['later_mentions']}"
                f"{', call argument' if record['later_call_argument'] else ''}): {record['texts'][:120]}"
            )
    unsafe = [move for move in moves if move["before"] == "Unsafe"]
    lines.append(f"provisional-Unsafe functions gaining a refusal: {len(unsafe)}")
    for move in unsafe:
        lines.append(f"  {move['unit']} {move['function']} leaks at {move['leaks_before']}")

    # Leaf forecast: one gate leaf per refused or exchanged line, the exchanged
    # description removed when no other statement at that line carries it.
    added_gate = removed = 0
    for function_key, entries in records_by_function.items():
        existing = {
            (diagnostic["category"], diagnostic.get("description"), diagnostic["line"])
            for diagnostic in diagnostics.get(function_key, [])
        }
        gate_lines = set()
        for key, record, verdict in entries:
            if verdict in ("refused", "exchange"):
                if ("unsupported", GATE, key[2]) not in existing and key[2] not in gate_lines:
                    added_gate += 1
                    gate_lines.add(key[2])
            if verdict == "exchange":
                removed += 1
    # An `unknown_alias` leaf at a refused line that names the old alias's
    # source is the old lowering's own report of the misnamed `Alias`; it goes
    # with the lowering unless another statement at that line reports it too.
    alias_at_risk = []
    for function_key, entries in records_by_function.items():
        present = {
            (diagnostic.get("description"), diagnostic["line"])
            for diagnostic in diagnostics.get(function_key, [])
            if diagnostic["category"] == "unknown_alias"
        }
        for key, record, verdict in entries:
            if verdict != "refused" or record["outcome"] != "Alias":
                continue
            source = record["texts"][record["equal"] + 1:]
            if len(source) != 1:
                continue
            description = f"unresolved resource identity `{source[0]}`"
            if (description, key[2]) in present:
                alias_at_risk.append((units[function_key[0]]["file"], function_key[1], key[2], description))
    alias_at_risk = sorted(set(alias_at_risk))
    out["leaf_forecast"] = {"gate_leaves_added_at_most": added_gate,
                            "exchanged_descriptions_at_most": removed,
                            "unknown_alias_leaves_of_refused_aliases": len(alias_at_risk)}
    out["unknown_alias_leaves_of_refused_aliases"] = [list(item) for item in alias_at_risk]
    lines.append(f"leaf forecast: gate leaves added <= {added_gate}; exchanged descriptions <= {removed}; "
                 f"unknown_alias leaves reported by a refused Alias: {len(alias_at_risk)}")
    text = "\n".join(lines) + "\n"
    if json_out:
        pathlib.Path(json_out).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    if text_out:
        pathlib.Path(text_out).write_text(text)
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
