#!/usr/bin/env python3
"""CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (every DISCHARGE clause holds);
registers the two residual classes with their sizes; amends W43-LIBC-NORETURN
and W44-BSEARCH-UNDECLARED with this card's measurements; sets
`dimensions.named_followups` to the list length (it was one short on the base,
659 entries against 658). Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's corpus runs (private/cg07/{before,after2}, corpus arbiter
--jobs 4, base c374de96), read with scripts/census.py, scripts/generic_lines.py
and scripts/sole_blockers.py; the artifacts are in ../artifacts/.
"""

import json
import sys

REPORT = "docs/reports/cg07-generic-and-statement-expressions/README.md"
BY = "CG07-GENERIC-AND-STATEMENT-EXPRESSIONS"

RETIRE = ["CG07-GENERIC-AND-STATEMENT-EXPRESSIONS"]

REGISTER = [
    {
        "id": "CG07-STATEMENT-EXPRESSION-POSITIONS",
        "title": "GNU statement expressions outside the four admitted positions stay refused: 69 braced leaves on statement-expression lines, 46 declined consultations on 28 functions",
        "why": (
            "Measured by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (gateway rule 11g) on its AFTER corpus run (base c374de96): "
            "a statement expression is lowered only when discarded, a whole one-declarator initializer, the whole value "
            "assigned to a named object (NAME or *&NAME) or the whole return operand. The hook declined 46 consultations "
            "(28 functions): 14 under a ?: arm, 11 a call actual (librdkafka's `({ rd_kafka_fetch_pos_t _fetchpos = { … }; "
            "_fetchpos; })` among them), 11 assigned to a member or other path, 9 with no unique Clang node (the run's "
            "tokens rewritten by an earlier stream pass), 1 under a cast; and statement expressions in controlling "
            "expressions and for clauses never reach it (libcurl's curl_easy_getinfo type check in an if condition). "
            "69 `braced initializer, expression, or declaration is not modeled` leaves remain on lines holding `({` "
            "(1,338 at BEFORE). DISCHARGE: an operand-position lowering that hoists the block only where the elaborator's "
            "graph shows every sibling unordered with it commutes and none sequenced before it (the elaborator's "
            "straight-line arm and lifetime commutation exist), controlling-expression positions through the condition "
            "routes, and a path destination under CG10's place premises, each measured with Lean parity."
        ),
    },
    {
        "id": "CG07-GENERIC-TOKEN-AST-MATCH",
        "title": "A statement whose _Generic route 1 lowered no longer matches its Clang node, so the AST-driven fallbacks decline it: 52 leaves on 19 functions on _Generic lines",
        "why": (
            "Measured by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS: every corpus _Generic is glibc 2.43's selection-invariant "
            "__glibc_const_generic shape, which W44's route 1 lowers to the call before the body is parsed; no corpus "
            "leaf is a refusal of a selection (BEFORE and AFTER: 0 `call to uncontracted function `_Generic`` leaves). The "
            "lowered statement's tokens no longer equal its Clang node's (which still spells the selection), so CG02's "
            "elaborator, CG05's nested call and CG06's value connective decline it with `no unique Clang node` "
            "(rd_kafka_conf_validate_single's `return !strchr (…) && !strchr (…)`, its only leaf). 52 leaves on 19 "
            "functions sit on lines holding `_Generic`, unchanged by the card (scripts/generic_lines.py). DISCHARGE: node "
            "matching that applies the same route-1/route-2 lowering to a node's own tokens (with the selections under it "
            "read as their selected association, as the elaborator already does), measured with Lean parity."
        ),
    },
]

AMEND = {
    "W43-LIBC-NORETURN": (
        " MEASURED by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (gateway rule 11g): glibc 2.43's assert expands to a "
        "statement expression `({ if (E) ; else __assert_fail (…); })`, which CG07 now lowers; its `__assert_fail` "
        "(declared noreturn by glibc, undeclared by any contract) is the corpus's largest remaining call leaf of the "
        "family: 1,232 `call to uncontracted function `__assert_fail`` leaves on 533 functions at CG07's AFTER (base "
        "c374de96), 10 Unknown functions blocked by it alone (tinycthread_extra.c's four rwlock wrappers, "
        "rd_kafka_cgrp_set/clear_wait_resp, rd_list_set_cnt, rd_kafka_confval_get_int, isFullyBalanced0 and snappy.c's "
        "get_u32_at_offset: artifacts/assert-fail-sole.json, an upper bound). A no-return path end (card CG21) would "
        "decide them."
    ),
    "W44-BSEARCH-UNDECLARED": (
        " RE-MEASURED by CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (base c374de96): 7 `call to uncontracted function "
        "`bsearch`` leaves on 6 functions, before and after the card: every corpus bsearch and memchr selection is "
        "selection-invariant and read by route 1 as its call, so Clang's selection (route 2) changes none."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in RETIRE:
        assert name in ids, f"{name} is not registered"
    for name in AMEND:
        assert name in ids, f"{name} is not registered"
    for entry in REGISTER:
        assert entry["id"] not in ids, f"{entry['id']} already registered"
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        entries.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
