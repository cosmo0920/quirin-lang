#!/usr/bin/env python3
"""CG00-CLANG-GAP-BASELINE: write the frozen per-card predictions CG02–CG15.

usage: predict.py FUNCTIONS_JSON LEAVES_JSONL_GZ MATRIX_JSON OUT_DIR

CG01 is not written: it landed (CG01-T-OPERATOR-CLOSURE, 7e7ba401) before
this card finished, and the corpus arbiter runs no T route.

For each card: the Unknown functions it would free alone (from
`functions.py`), each with its blocking leaves and the obligations those
accesses would carry once the card models them, and a predicted verdict:

* **Safe** (upper bound) when the function has no `unknown_alias` leaf
  (`freed_alone_strict`): the card's modelled events are its only blockers,
  so the verdict is decided by the obligations listed, which the card must
  discharge from existing facts;
* **Unknown** when an `unknown_alias` leaf remains (`with_alias` only): the
  alias may be poisoned by the refusal and clear (rule 10a) or may stay, so
  the conservative prediction is Unknown until the alias census says which.

No provisional Unsafe is predicted: none of these leaves is a violation
candidate, and a modelled access that exposes one would be a finding to
classify by reading, never presented as an upstream defect.

The absorbed follow-ups are listed with their registered size, and with the
CG00 attribution of the card's entries beside it (a different measure: CG00
counts leaves by entry precedence, not by the follow-up's own definition).
"""

import collections
import gzip
import json
import pathlib
import sys

OBLIGATIONS = {
    "E03": "single evaluation of the destination place; the old-value read and the write-back as one read-modify-write event; the value is custody-neutral scalar arithmetic",
    "E04": "the assignment's value is the stored value after conversion; the store and the value use are sequenced as C sequences them",
    "E05": "old and new values kept apart; the update is a store event on the place; custody-neutral for a scalar",
    "E06": "the unevaluated operand produces no event; the result is a 0/1 scalar",
    "E07": "only the selected arm's events apply; the result's alias and custody state is the join of the selected arms",
    "E08": "the left operand completes before the right; the value is the right operand's",
    "E09": "acquisition identity bound to the destination; NULL/sentinel split on the controlling test; release obligation on the success branch only",
    "E10": "the inner call's contract applied first (Owned/Released/Consumed/Out), its result connected to the outer actual",
    "E11": "each nested call applied inside-out; a call within a larger actual evaluated before the outer call",
    "E12": "sibling calls checked in both orders, or commutativity shown from their footprints",
    "E13": "every leaf of the controlling expression elaborated; each call's contract applied once on the edge C evaluates it",
    "E15": "the cast is a representation with an explicit loss/provenance flag; no custody change",
    "E16": "evaluated VLA operands separated; the value is Clang's constant, never the marker 1",
    "E17": "only Clang's selected association is evaluated",
    "E19": "the inner scope's calls and escapes compose into the outer scope; the final expression is the value",
    "E20": "a named opaque boundary per builtin, or refusal",
    "K03": "formal substitution value -> identity -> formal over the typed place",
    "K12": "the contract applied exactly once at the position where callee and actuals are established",
    "M01": "each pointer hop of the typed place non-null and live (per-hop obligation); a tracked root needs one known non-null live identity",
    "I04": "the access bound by AST node identity, not by its line's token shape",
    "M02": "the store is a value store plus an alias addition; the overwritten value's obligation and the stored value's escape are separate events",
    "M03": "the owner's frame identity from the caller (formal or alias) established before the field-update transfer",
    "M04": "the store generalised over nested fields, elements and function-pointer members",
    "M05": "the stored identity connected to the post-cast view by contract or derivation",
    "M06": "the union member's overlapping storage and effective-type view stated at a boundary",
    "M07": "the bit-field's storage unit is one memory location (read-modify-write)",
    "M08": "field-wise alias and custody transfer of the aggregate",
    "M09": "designated and implicit zero-fill initialisation with initial aliases",
    "M10": "the compound literal's identity and block/file lifetime",
    "M11": "string/array initialiser extent (only if bounds are claimed)",
    "M23": "each volatile access a separate observable effect with count and order",
    "M24": "static/thread-local identity and lifetime",
}

ABSORBED = {
    "CG02": ["W43-RETURN-COMMA-MACRO"],
    "CG03": ["W46-ACCESS-PATH-PARITY"],
    "CG04": ["W44-CONDITION-CALL-ASSIGNMENT", "W44-CONDITION-CALL-FREE-OPERAND", "W44-CONDITION-CALL-TYPE-OPERATOR",
             "W44-CONDITION-CALL-REFUSED-SHAPES", "W44-CONDITION-DISCARDED-DERIVED-RESULT",
             "W43-INDEX-TRAVERSAL-CONDITION-WRITE", "CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED"],
    "CG05": ["NESTED-CALL-IN-ARGUMENT-POSITION", "W44-PRINTF-NESTED-INNER-CALLEES", "W43-RETURN-COMPOUND-CALL",
             "CJSON-DELETE-NESTED-DETACH"],
    "CG06": ["RETURN-POSITION-CONDITIONAL", "W43-RETURN-CONDITIONAL-OPERANDS", "W47-ACCESS-PATH-POSITION-RESIDUALS"],
    "CG07": [],
    "CG08": ["W44-SIZEOF-OPERAND-RESIDUALS", "W44-SIZEOF-CAST-SCALAR-QUERY", "SIZEOF-TYPE-FOLD-NO-LEAN",
             "CAST-RETURN-UNCOMMITTED-OPERAND", "W45-CALLEE-CAST-LEXICAL-PREMISE"],
    "CG09": [],
    "CG10": ["W46-ACCESS-PATH-TRACKED-MIXED-PLACES", "W44-CHAIN-NESTED-AGGREGATE", "W43-INDIRECT-UNTYPED-SINGLE-ACCESS",
             "REFUSED-HANDON-LINE-GRANULARITY", "W44-AGGREGATE-PREMISE3-MEMBER-NAME"],
    "CG11": ["W43-INDIRECT-POINTER-VALUE-STORE", "W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED", "W45-SUB-OBJECT-STORE-CHAIN",
             "W45-SUB-OBJECT-STORE-ROOT-UNRESOLVED", "W45-SUB-OBJECT-STORE-FORMAL-VALUE",
             "W45-SUB-OBJECT-STORE-OWNED-VALUE", "W45-SUB-OBJECT-STORE-VALUE-SHAPES", "W45-SUB-OBJECT-STORE-PREFIX",
             "W44-CALL-RESULT-INDIRECT-DESTINATION", "W43-COLLECTION-NAME-STORE"],
    "CG12": ["W47-FIELDS-BY-MEMBER-VOID-ESCAPES"],
    "CG13": ["W46-UNION-MEMBER-PATH", "W44-CHAIN-INCOMPLETE-RECORD", "W43-LIBC-GETPEERNAME-SOCKADDR-UNION"],
    "CG14": [],
    "CG15": ["GATEWAY-VOLATILE-PREMISE", "CT41-VOLATILE-COMPLEMENT-UNMEASURED"],
}

ENTRIES = {
    "CG02": "E03 E04 E05 E08", "CG03": "K12 K03", "CG04": "E09 E13", "CG05": "E10 E11 E12",
    "CG06": "E06 E07", "CG07": "E17 E19", "CG08": "E15 E16 N11 M18 N08", "CG09": "C01 C02 C03",
    "CG10": "M01 I04", "CG11": "M02 M03 M04", "CG12": "M05", "CG13": "M06 M07",
    "CG14": "M08 M09 M10 M11", "CG15": "M24 M23",
}


def main():
    functions = json.load(open(sys.argv[1]))
    with gzip.open(sys.argv[2], "rt") as handle:
        rows = [json.loads(line) for line in handle][1:]
    matrix = json.load(open(sys.argv[3]))
    out = pathlib.Path(sys.argv[4])
    out.mkdir(parents=True, exist_ok=True)
    followups = {e["id"]: e for e in matrix["named_followups"]}
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[f"{row['file']}::{row['function']}"].append(row)
    cards = {c["card"]: c for c in functions["cards"]}
    summary = {}
    for card, entries in ENTRIES.items():
        figures = cards.get(card, {"leaves": 0, "unknown_functions_touched": 0, "freed_alone_with_alias": 0,
                                   "freed_alone_strict": 0, "freed_with_one_other_card": 0, "top_partners": []})
        strict = set(functions["freed_alone"]["strict"].get(card, []))
        loose = functions["freed_alone"]["with_alias"].get(card, [])
        lines = [
            f"# {card}: frozen prediction (from CG00-CLANG-GAP-BASELINE)",
            "",
            "Frozen by CG00 on the corpus arbiter baseline (`ae17152b…`, 526 Safe / 10 provisional",
            "Unsafe / 4,298 Unknown). Generated by `scripts/predict.py`; method in the",
            "[report](../README.md#attribution). Upper bounds: a modelled access can still fail its",
            "obligations, and the route (G) is the custody gateway.",
            "",
            f"**Entries:** {entries}. **Route:** " + ("T (the corpus arbiter runs no T route: 0 corpus functions)" if card == "CG09" else "G") + ".",
            "",
            "| Measure | Value |",
            "|---|---:|",
            f"| unsupported leaves attributed | {figures['leaves']} |",
            f"| Unknown functions touched | {figures['unknown_functions_touched']} |",
            f"| freed alone, alias leaves allowed | {figures['freed_alone_with_alias']} |",
            f"| freed alone, no alias leaf (strict) | {figures['freed_alone_strict']} |",
            f"| freed together with exactly one other card | {figures['freed_with_one_other_card']} |",
            "",
            "Top partner cards (functions freed only together): "
            + (", ".join(f"{p} {n}" for p, n in figures["top_partners"]) or "none") + ".",
            "",
            "## Absorbed follow-ups",
            "",
        ]
        absorbed = ABSORBED[card]
        if not absorbed:
            lines.append("None registered.")
        for name in absorbed:
            entry = followups.get(name)
            why = entry["why"] if entry else "(not in the ledger)"
            lines.append(f"- `{name}`: registered size carried unchanged (not re-run under its own definition by CG00). "
                         f"Ledger text: {why[:240]}…")
        entry_counts = functions["entries"]
        lines += ["", "CG00 attribution of this card's entries (leaves / Unknown-or-other functions): "
                  + ", ".join(f"{e} {entry_counts.get(e, {}).get('leaves', 0)} / {entry_counts.get(e, {}).get('functions', 0)}"
                              for e in entries.split()) + ".", "",
                  "## Predicted verdicts", "",
                  f"- **Safe (upper bound): {len(strict)}** functions with no alias leaf; the card's events are their only blockers.",
                  f"- **Unknown: {len(loose) - len(strict)}** functions whose `unknown_alias` leaves may stay.",
                  "- **Provisional Unsafe: 0** predicted.", "",
                  "## Functions it could free alone", "",
                  "Each with its blocking leaves (entry, internal line, CG00 reason) and the obligations",
                  "their accesses would carry.", ""]
        used = sorted({r["entry"] for name in loose for r in by_function[name]})
        for name in loose:
            leaves = by_function[name]
            verdict = "Safe" if name in strict else "Unknown (alias)"
            detail = "; ".join(f"{r['entry']}@{r['line']} {r['reason'].split(',')[0][:40]}" for r in leaves[:6])
            more = f" (+{len(leaves) - 6} more)" if len(leaves) > 6 else ""
            lines.append(f"- `{name}` — {verdict}: {detail}{more}")
        lines += ["", "## Obligations by entry", ""]
        for entry in used:
            lines.append(f"- **{entry}:** {OBLIGATIONS.get(entry, 'see the plan section of this card')}")
        (out / f"{card}.md").write_text("\n".join(lines) + "\n")
        summary[card] = {"freed_alone_with_alias": len(loose), "freed_alone_strict": len(strict),
                         "predicted_safe": len(strict), "predicted_unknown": len(loose) - len(strict),
                         "predicted_provisional_unsafe": 0}
    (out / "summary.json").write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
