#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: write the per-card PREDICTION drafts.

usage: make_drafts.py RUN_REPORT RANK_COARSE_JSON COOCCUR_JSON OUT_DIR

One draft per proposed implementation bundle (K1..K7): the owning follow-up ids
and rules, the census figures, the frozen candidate list (the Unknown functions
whose every access/alias blocker the bundle removes: the ceiling, not a
forecast), the expected-Safe estimate and its method. A draft is not a card's
PREDICTION.md: the implementing card re-reads the bodies, freezes its own
list and one `measure-prediction` block before it changes code.
"""
import json
import pathlib
import sys

report_path, rank_path, cooccur_path, out = (pathlib.Path(a) for a in sys.argv[1:5])
report = json.load(open(report_path))
rank = json.load(open(rank_path))
cooccur = json.load(open(cooccur_path))
library = {u["file"]: u.get("library") for u in report["units"]}

CARDS = {
    "K1": dict(
        title="Call-actual access reads (statement, return, condition)",
        ids="W44-CALL-ARGUMENT-UNTYPED-ACTUAL (primary), W44-CALL-ARGUMENT-NON-STATEMENT, W44-CALL-ARGUMENT-UNCONTRACTED",
        rules="gateway 10k (position reader) and 10n (residual steps), 10u (call contract instantiation, "
              "custody-taking positions stay refused: W44-CALL-ARGUMENT-CUSTODY-POSITION)",
        shape="an expressible typed access path (no refusal cause) inside a call actual, whose call the typed "
              "layer still refuses",
        effort="card-high"),
    "K2": dict(
        title="Leaves with no typed access (lexical gate over-refusal)",
        ids="W46-LEXICAL-GATE-NON-ACCESS (primary), W44-CONDITION-CALL-REFUSED-SHAPES, W44-CONDITION-CALL-ASSIGNMENT",
        rules="gateway 10k/10n position readers; the condition desugaring census codes",
        shape="an IND/COND/RET leaf whose construct carries no access path at all (declarator `*`, `sizeof(*p)`, "
              "multiplication, a cast, an array declarator, a call or an assignment in a condition)",
        effort="card-high"),
    "K3": dict(
        title="Pointer-valued stores (FIELD-CUSTODY)",
        ids="CG11-UNRESOLVED-OWNER-POINTER-STORES (primary), CG11-UNREAD-POINTER-PLACES; related siblings "
            "CHAINED-ASSIGNMENT-INNER-DESTINATION and REFUSED-STORE-OF-CALL-RESULT-ESCAPE are not duplicated",
        rules="gateway 10v (pointer-typed store on the typed statement route)",
        shape="a store whose destination is a pointer-valued field, slot or element (refusal cause "
              "`pointer-valued-store`)",
        effort="card-high"),
    "K4": dict(
        title="Formal-custody aliases",
        ids="W47-ALIAS-CALLEE-BORROWED-CLOSURE (primary), W47-ALIAS-FORMAL-COPY",
        rules="gateway 5i/5l (never-bound closure), paired with the P4 callee-closure cards",
        shape="an unresolved identity for a pointer formal (handed to an uncontracted callee, copied, handed to a "
              "contracted one, or not handed)",
        effort="card-high"),
    "K5": dict(
        title="Local-origin aliases",
        ids="W47-ALIAS-FLOW-RESIDUALS (primary), W47-ALIAS-TYPED-SCALAR-NAMES, W47-ALIAS-NONNULL-ALLOCATOR-RESULTS",
        rules="gateway 5i/5l and 10a (poisoning refusal); provider crate for allocator results",
        shape="an unresolved identity for a derived or guarded local, a borrowed path read, a scalar name, an "
              "allocator result or a statement expression",
        effort="card-high"),
    "K6": dict(
        title="Direct-position expressible access",
        ids="W47-ACCESS-PATH-POSITION-RESIDUALS (primary), CG10-BOUND-ROOT-PATH-REFUSED, "
            "CG10-BOUND-ROOT-NULLABLE-OR-TRACKED",
        rules="gateway 10k/10n (root premise), CG10 typed place",
        shape="an expressible typed access path directly in a statement, return or condition (no call actual) "
              "that the typed layer refuses for its root premise or value form",
        effort="card-high"),
    "K7": dict(
        title="Residual refusal causes (union, aggregate value, call in path, indirect call, cast, arithmetic)",
        ids="CG13-UNION-LEAVES-OTHER-CAUSES, CG14-AGGREGATE-VALUE-POSITIONS, CG22-OPEN-INDIRECT-CALLS, "
            "CAST-FORMALIZATION-RESIDUAL-CENSUS, CG07-STATEMENT-EXPRESSION-POSITIONS (one card per cause)",
        rules="gateway 10w (union), 11b/12h (casts), 11p (indirect calls), 10x/14 aggregate positions",
        shape="the named refusal causes of the access census other than the pointer-valued store",
        effort="card-high"),
}
YIELD = (0.25, 0.38, 0.50)
out.mkdir(parents=True, exist_ok=True)
bundles = {name.split()[0]: (name, v) for name, v in rank["bundles"].items()}
index = []
for key, meta in CARDS.items():
    name, v = bundles[key]
    functions = rank["alone_lists"][key]
    alone = len(functions)
    low, mid, high = (round(alone * y) for y in YIELD)
    cascade_same = v["closure_same_unit"]["freed"] - v["alone"]
    cascade_cross = v["closure_cross_unit"]["freed"] - v["alone"]
    nxt = cooccur[key]["next_blocker"]
    touched = cooccur[key]["touched"]
    lines = [
        f"# PREDICTION draft: {key} {meta['title']}",
        "",
        "Draft from CURRENT-ACCESS-ALIAS-CENSUS (base `7b726587b`, the user's `quirin-audit current` command at",
        "Fluent Bit `046accf6`, census build). It is not a PREDICTION.md: the implementing card re-reads the",
        "bodies, freezes its own list and exactly one `measure-prediction` block before it changes code.",
        "",
        f"- **Owner follow-ups (amend, do not duplicate):** {meta['ids']}.",
        f"- **Rule ownership:** {meta['rules']}.",
        f"- **Effort:** {meta['effort']}; Lean parity for every semantic rule; fail-closed throughout.",
        f"- **Shape:** {meta['shape']}.",
        f"- **Census tags:** {', '.join('`' + t + '`' for t in v['tags'])}.",
        "",
        "## Figures (function rows are unit configurations; Unknown rows only)",
        "",
        "| Measure | Value |",
        "|---|---:|",
        f"| leaves carrying the shape | {v['leaves']} |",
        f"| Unknown function rows it touches | {v['functions_touched']} |",
        f"| rows freed alone by removing the shape (ceiling) | {v['alone']} |",
        f"| distinct functions in the frozen list below | {alone} |",
        f"| rows freed only if every bundle K1..K7 goes, attributable to this one (marginal) | {v['marginal_last']} |",
        f"| same-unit closure (existing summary rule), rows / rounds | {v['closure_same_unit']['freed']} / {v['closure_same_unit']['rounds']} |",
        f"| cross-unit closure (no rule does this today), rows / rounds | {v['closure_cross_unit']['freed']} / {v['closure_cross_unit']['rounds']} |",
        f"| units whose whole blocker set it covers | {v['units_covered']} |",
        "",
        f"Next blocker of the {touched} touched rows once the access/alias shapes are gone: "
        + ", ".join(f"{k} {n}" for k, n in nxt.items()) + ".",
        "",
        "## Expected Safe",
        "",
        f"Ceiling {alone} distinct functions. The estimate applies the W46 P3 yield (83 Safe of 220 freed, 38%)",
        f"with a 25-50% band, because the shape's freed functions still need their own obligations (non-null and",
        f"live per hop, custody frame) discharged: **{mid} Safe (band {low}-{high})**, plus a callee cascade of",
        f"about {max(cascade_same, 0)} rows through the existing same-unit summary rule ({max(cascade_cross, 0)} if a",
        "cross-unit route were built). Everything freed but not Safe stays Unknown with its named reason; no",
        "provisional Unsafe is expected without a validated reachable violation. These are forecasts to be",
        "replaced by the card's body-read list; they are not evidence.",
        "",
        "## Draft block (text only, not a `measure-prediction` block)",
        "",
        "```text",
        "route corpus",
        "none expected until the card measures the eight-home corpus rows of the list below",
        "route system",
        "none expected",
        "route slice",
        "none expected",
        f"user current run: up to {alone} functions leave the blocked set; the dominant side effect is the loss of",
        f"{v['leaves']} {meta['title'].lower()} leaves; every unit status unchanged except where the unit-coverage",
        f"figure above is non-zero",
        "```",
        "",
        "## Frozen candidate list (ceiling: freed alone; library::file::function)",
        "",
        "```text",
    ]
    lines += [f"{library.get(f, '?')}::{f}::{n}" for f, n in functions]
    lines += ["```", ""]
    path = out / f"{key}.md"
    path.write_text("\n".join(lines))
    index.append((key, meta["title"], alone, mid, low, high))
(out / "README.md").write_text(
    "# PREDICTION drafts for the proposed implementation cards\n\n"
    "Generated by `../scripts/make_drafts.py` from the census outputs. Each is a ceiling and an estimate, not a\n"
    "frozen PREDICTION.md.\n\n| Draft | Shape | Ceiling (functions) | Expected Safe | Band |\n|---|---|---:|---:|---|\n"
    + "\n".join(f"| [{k}]({k}.md) | {t} | {a} | {m} | {lo}-{hi} |" for k, t, a, m, lo, hi in index) + "\n")
print(index)
