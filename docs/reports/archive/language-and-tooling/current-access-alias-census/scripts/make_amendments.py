#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: write integration-amendments.json.

usage: make_amendments.py OUT_JSON
The figures are the README's (artifacts/rank-coarse.json): rows are unit
configurations of the user's `current` run at base 7b726587b.
"""
import json
import pathlib
import sys

SRC = "docs/reports/current-access-alias-census/README.md"
CARD = "CURRENT-ACCESS-ALIAS-CENSUS"
PRE = "CURRENT-ACCESS-ALIAS-CENSUS (round 26 D, base 7b726587b, the user's `current` run: 4,912 function rows, 3,832 Unknown): "

AMEND = {
    "W44-CALL-ARGUMENT-UNTYPED-ACTUAL":
        "bundle K1 (call-actual access in statements, returns and conditions: 6,718 leaves on 1,651 Unknown rows) frees 138 rows alone (135 distinct functions), 371 marginal once every other bundle has gone; expected Safe 51 (band 34-68) at the W46 P3 yield, plus 11 through the same-unit summary rule. First of seven proposed implementation cards (rules 10k, 10n, 10u); frozen candidate list in docs/reports/current-access-alias-census/predictions/K1.md. 62 to 74% of the touched rows also carry a not-Safe run-defined callee, so the yield is bounded by W48-BOTTOM-UP-RESIDUAL-PLAN.",
    "W44-CALL-ARGUMENT-NON-STATEMENT":
        "the condition and return arms of bundle K1: `expr:cond:call-arg` 1,492 leaves on 605 rows (16 freed alone), `expr:return:call-arg` 110 leaves on 102 rows (48 freed alone); launch with W44-CALL-ARGUMENT-UNTYPED-ACTUAL.",
    "W44-CALL-ARGUMENT-UNCONTRACTED":
        "the statement call-actual access shape is 5,116 IND leaves on 1,415 Unknown rows (69 freed alone), 1,096 of the 1,651 K1-touched rows carry a run-defined uncontracted call as next blocker, so this arm is gated on the callee-closure cards, not on the access reader.",
    "CG11-UNRESOLVED-OWNER-POINTER-STORES":
        "bundle K3 (`pointer-valued-store`: 2,428 leaves, IND 2,102, COND 259, RET 67, on 929 Unknown rows) frees 53 rows alone (53 distinct functions), 167 marginal; expected Safe 20 (band 13-26), plus 15 through the same-unit rule. Second proposed implementation card (rule 10v); candidate list in predictions/K3.md. Related siblings CHAINED-ASSIGNMENT-INNER-DESTINATION and REFUSED-STORE-OF-CALL-RESULT-ESCAPE are not duplicated.",
    "CG11-UNREAD-POINTER-PLACES":
        "part of bundle K3 on the current run: pointer-valued stores are the second-ranked shape (53 rows freed alone); see CG11-UNRESOLVED-OWNER-POINTER-STORES.",
    "W47-ALIAS-FLOW-RESIDUALS":
        "bundle K5 (ten local-origin alias groups, 6,823 leaves on 1,654 rows: guarded-other 674, borrowed-path-unguarded 875, derived-local-unguarded 1,037, non-pointer-name 1,072, poisoned-by-refusal 1,281, address-taken 284) frees 43 rows alone, 232 marginal; expected Safe 16 (band 11-22), plus 6 through the same-unit rule. Third proposed card; list in predictions/K5.md.",
    "W47-ALIAS-TYPED-SCALAR-NAMES":
        "on the current run `alias:non-pointer-name` is 1,072 leaves on 347 Unknown rows, 7 freed alone (K5); the largest alias group after formal-handed-uncontracted.",
    "W47-ALIAS-NONNULL-ALLOCATOR-RESULTS":
        "on the current run `alias:contracted-result-unguarded` is 587 leaves on 206 Unknown rows, 1 freed alone (K5); the pair with the callee cards matters more than the group alone.",
    "CG13-UNION-LEAVES-OTHER-CAUSES":
        "bundle K7 (residual causes: union 967 leaves on 323 rows, aggregate-value 252, call 137, indirect-call 185, cast 71, pointer-arithmetic 33, static-local 64) frees 40 rows alone, 184 marginal; union alone frees 8. Expected Safe 15 (band 10-20) for the bundle; one card per cause. List in predictions/K7.md.",
    "CG14-AGGREGATE-VALUE-POSITIONS":
        "on the current run `aggregate-value` is 252 leaves on 169 Unknown rows, 9 freed alone (bundle K7).",
    "CG22-OPEN-INDIRECT-CALLS":
        "on the current run `indirect-call` in an access path is 185 leaves on 142 Unknown rows, 6 freed alone (bundle K7).",
    "CAST-FORMALIZATION-RESIDUAL-CENSUS":
        "on the current run `cast` in an access path is 71 leaves on 38 Unknown rows, 0 freed alone (bundle K7).",
    "CG07-STATEMENT-EXPRESSION-POSITIONS":
        "on the current run `statement-expression` in an access path is 118 leaves on 72 Unknown rows, 0 freed alone (bundle K7).",
    "W46-LEXICAL-GATE-NON-ACCESS":
        "bundle K2 on the current run: 1,058 leaves with no typed access (COND 952, IND 77, RET 29) on 604 Unknown rows, 40 freed alone (39 distinct), 87 marginal; expected Safe 15 (band 10-20), plus 7 through the same-unit rule. Fifth proposed card; list in predictions/K2.md. Most of it is conditions refused for a call or an assignment, W44-CONDITION-CALL-REFUSED-SHAPES' territory.",
    "W44-CONDITION-CALL-REFUSED-SHAPES":
        "the condition share of bundle K2 on the current run: 952 COND leaves carry no typed access; launch with W46-LEXICAL-GATE-NON-ACCESS.",
    "W47-ACCESS-PATH-POSITION-RESIDUALS":
        "bundle K6 (direct-position expressible access: statement 985, condition 528, return 58 leaves on 768 rows) frees 21 rows alone, 142 marginal; expected Safe 8 (band 5-10), plus 6 through the same-unit rule. Sixth proposed card; list in predictions/K6.md. The refusal reasons at those lines are mostly the root premise and value form (parity stream).",
    "CG10-BOUND-ROOT-PATH-REFUSED":
        "the root-premise arm of bundle K6 on the current run (`expr:statement:direct` 985 leaves, 6 rows freed alone; most direct statement accesses are stores and null stores through local or formal roots).",
    "W47-ALIAS-CALLEE-BORROWED-CLOSURE":
        "bundle K4 on the current run: `alias:formal-handed-uncontracted` is the largest alias group (3,803 leaves on 886 Unknown rows) and frees 0 rows alone; bundle K4 (four formal-custody groups, 4,289 leaves) frees 16 rows alone, 163 marginal, 59 with the cross-unit closure; expected Safe 6 (band 4-8). Last of the seven proposed cards because it pairs with the callee-closure work; list in predictions/K4.md.",
    "W47-ALIAS-FORMAL-COPY":
        "on the current run `alias:formal-copied` is 273 leaves on 241 Unknown rows, 13 freed alone (bundle K4).",
    "W48-BOTTOM-UP-RESIDUAL-PLAN":
        "the access and alias census of the user's current run shows the callee closure is the binding constraint: 62 to 74% of the Unknown rows each access or alias bundle touches also carry a not-Safe run-defined callee; removing every IND, ALIAS, COND and RET leaf frees 753 rows (882 same-unit, 1,031 cross-unit with the closure), removing UNC-RUN as well frees 2,118. No bundle covers a whole unit (5 of 164 Unknown configurations with all four families gone).",
}

ledger = [
    {"id": CARD, "operation": "register",
     "title": "The IND, ALIAS, COND and RET leaves of the user's current run, ranked by typed-path shape, with the call-graph closure",
     "text": "Registered by its own card at landing (round 26 D, the user's 2026-11-06 request to cover more areas that block Safe or provisional Unsafe units); retired by the next entry.",
     "source": SRC, "recorded_by": CARD},
    {"id": CARD, "operation": "retire",
     "why": "DISCHARGED by CURRENT-ACCESS-ALIAS-CENSUS (docs/reports/current-access-alias-census/README.md): every IND, COND, RET and unknown_alias leaf of the current run at base 7b726587b is classified by typed path or alias group (160 of 13,209 IND/COND/RET groups and one unit unjoined), ranked alone, by marginal gain and by unit coverage, with the call-graph closure; seven implementation bundles are proposed against their existing follow-up owners with per-card PREDICTION drafts.",
     "source": SRC},
    {"id": "CURRENT-CENSUS-UNIT-STREAM-GAP", "operation": "register",
     "title": "The census access-path stream records 183 analyses for 186 configurations and one unit's recorded text is shorter than its report lines",
     "text": "Registered by CURRENT-ACCESS-ALIAS-CENSUS (report section 1). On the user's current run the QCORE_ACCESS_PATH_CENSUS stream holds 183 analysis texts; lib/cmetrics/src/cmt_map.c has report lines up to 5,877 but its only recorded text has 5,664 lines, so a second analysis path of that unit is not hooked (10 functions, 33 alias and 39 IND/COND groups unjoined; rdhttp.c conditions 59 groups also bind to no construct). The census holds these as `unjoined` and never counts them removable. Find the unhooked entry point and add the stream hook there; measurement tooling only.",
     "source": SRC, "recorded_by": CARD},
]
for ident, text in AMEND.items():
    ledger.append({"id": ident, "operation": "amend", "text": PRE + text, "source": SRC})

document = {
    "schema": "quirin-amendments/1",
    "card": CARD,
    "base": "7b726587b",
    "measured_tip": "7b726587b",
    "increment_paragraph": (
        "**CURRENT-ACCESS-ALIAS-CENSUS** (round 26 D; records only, no verdict moves): the user's "
        "`quirin-audit current` run at Fluent Bit `046accf6` re-measured on base `7b726587b` with the census build "
        "(4,912 function rows, 3,832 Unknown; 9,490 IND, 3,517 COND, 202 RET and 11,177 alias leaves classified by "
        "typed path or alias group, 160 groups and one unit unjoined). Ranked alone, by marginal gain and by unit "
        "coverage: the statement call-actual access reads free 69 rows alone, pointer-valued stores 53, return "
        "call-actual reads 48, leaves with no typed access 40; all four families gone free 753 rows (882 same-unit, "
        "1,031 cross-unit with the closure) and cover 5 of 164 Unknown configurations, and 62 to 74% of the rows a "
        "bundle touches also carry a not-Safe run-defined callee. Seven bundles are proposed against their existing "
        "follow-up owners, with per-card PREDICTION drafts and frozen candidate lists "
        "([report](../../reports/current-access-alias-census/README.md))."),
    "status_paragraph": None,
    "ledger": ledger,
}
pathlib.Path(sys.argv[1]).write_text(json.dumps(document, indent=2, ensure_ascii=False) + "\n")
print(len(ledger), "ledger operations")
