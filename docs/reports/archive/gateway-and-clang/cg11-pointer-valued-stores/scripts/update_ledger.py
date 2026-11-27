#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends CG11-POINTER-VALUED-STORES (not every DISCHARGE clause holds: the
formal-value, owned-value and named-value stores the entry scopes stay
refused) and the ten absorbed follow-ups with what rule 10v measured for
them, registers the card's residuals with their measured sizes, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are from the AFTER corpus arbiter's parity stream
(`private/cg11/after-parity.jsonl`, `scripts/census.py`, deduplicated per
statement): rule 10v's own typed partition of the pointer-typed stores it
read. The absorbed entries' registered sizes were measured by their own
census scripts, which this card did not re-run; the partition is not those
definitions and is recorded beside them, not in their place.
"""

import json
import sys

REPORT = "docs/reports/cg11-pointer-valued-stores/README.md"
BY = "CG11-POINTER-VALUED-STORES"

CENSUS = (
    "rule 10v read 2,836 pointer-typed stores on the typed statement route of the AFTER corpus run "
    "(deduplicated per statement): 794 carry no identity (null 557, function 100, interior address 70, "
    "the destination's own object 30, function-pointer name 22, string literal 15), of which 133 are "
    "admitted in the analysis that reports the function (120 in every analysis) on 58 functions; 470 "
    "stay refused because the destination's owner is unresolved; 2,042 are refused at classification "
    "(destination not a CG10 place 519 / 162 functions, pointer read 421 / 187, formal's pointer 376 / "
    "180, local's pointer 263 / 169, another object's address 217 / 73, other 186 / 133, arithmetic "
    "53 / 30, call 7 / 4)"
)

REGISTER = [
    {
        "id": "CG11-UNRESOLVED-OWNER-POINTER-STORES",
        "title": "Identity-free pointer stores whose owner the flow does not resolve stay refused (470 statements on 210 functions)",
        "why": (
            "Measured by CG11-POINTER-VALUED-STORES (gateway rule 10v) on the AFTER corpus arbiter's parity "
            "stream (scripts/census.py, deduplicated per statement): 470 stores of a value with no identity "
            "(null, a function, an interior address, a function-pointer name, a string literal) are refused in "
            "every analysis because the destination's chain does not resolve to an untracked owner: the root "
            "is bound but carries no tracked identity (the CG10-BOUND-UNTRACKED-ROOTS premise), released, or "
            "possibly null. A caller-site binding while a summary is derived is the tracked case and is "
            "deliberate. DISCHARGE: CG10-BOUND-UNTRACKED-ROOTS' per-path premise (no custody position received "
            "the root before the store), after which rule 10v reads the owner as untracked; measured, with "
            "Lean parity."
        ),
    },
    {
        "id": "CG11-POINTER-STORE-SUMMARY-MODE",
        "title": "A body rule 10v admits publishes no summary: its callers keep an uncontracted-call refusal (5 leaves on 4 callers)",
        "why": (
            "Rule 10v (CG11-POINTER-VALUED-STORES) withholds the derived summary of every body with an admitted "
            "pointer store (`has_pointer_store` in the W4 gate), because a caller would read `Borrowed` for "
            "the formal whose object the body overwrote. Measured on the AFTER corpus (scripts/callers.py): "
            "the five newly Safe bodies are called at 5 `call to uncontracted function` leaves on 4 Unknown "
            "callers (cio_create, rd_kafka_broker_fail, rd_kafka_new, rd_kafka_mock_cluster_new). DISCHARGE: "
            "a summary mode that states which of a formal's slots the body overwrote, with an identity-free "
            "value (callers forget their bindings and field links of those slots), inferred and validated like "
            "the other W4 modes; measured, with Lean parity."
        ),
    },
    {
        "id": "CG11-FUNCTION-POINTER-PARAMETER-CLOSURE",
        "title": "A function-pointer parameter leaves the function's other formals unread as never bound (92 corpus functions declare one, all Unknown; 23 carry an unknown_alias on another parameter)",
        "why": (
            "Found by CG11-POINTER-VALUED-STORES' probe (archived under its report): `void subject(struct S "
            "*s, void (*g)(void)) { s->a = 1; }` is Unknown with `unknown_alias s`, while the same body without "
            "the function-pointer parameter is Safe; the mechanism (the formal list reader or the never-bound "
            "closure) was not isolated. Fail-closed, not a soundness hole. Measured lexically over the pinned "
            "sources (scripts/fp_param_census.py, BEFORE and AFTER alike): 92 corpus functions declare a "
            "`(*NAME)(` parameter, all Unknown; 23 of them carry an `unknown_alias` naming another of their "
            "parameters (an upper bound on the reach: each may carry other blockers). DISCHARGE: the formal "
            "reader reads a function-pointer declarator as one formal, with the never-bound premise decided "
            "for the others as for any function; measured."
        ),
    },
    {
        "id": "CG11-UNREAD-POINTER-PLACES",
        "title": "Pointer-typed store destinations the CG10 place grammar does not read stay refused (519 statements on 162 functions)",
        "why": (
            "Measured by CG11-POINTER-VALUED-STORES on the AFTER corpus's parity stream: 519 pointer-typed "
            "stores whose destination `typed_place::read_place` refuses (a union or bit-field member, a cast "
            "base such as `((T *) p)->f`, an array or call root, a volatile base). Rule 10v never reads their "
            "value. DISCHARGE: the owning cards' place readings (CG13 unions and bit-fields, CG08 casts), after "
            "which rule 10v decides the value; measured."
        ),
    },
]

AMEND = {
    "CG11-POINTER-VALUED-STORES": (
        " AMENDED by CG11-POINTER-VALUED-STORES (gateway rule 10v; report docs/reports/cg11-pointer-valued-stores/): "
        "landed a pointer-valued store read from the Clang AST on the typed statement route, the value store, "
        "overwrite, old obligation and escape kept apart, admitted only for a value with no identity (null, "
        "string literal, function, a function-pointer name bound to nothing, the destination's own object) into "
        "an owner whose root and member object are untracked, with no summary published (QCore.PointerStore, "
        "18 theorems, shared vectors). Corpus 543 / 10 / 4,281 -> 548 / 10 / 4,276 (the 5 predicted: "
        "cio_options_init and four TAILQ_INIT bodies; no correction, no leak or violation added, unknown_alias "
        "unchanged). BEFORE reproduced; of CG00's 37 freed-alone functions 4 moved, 33 stay Unknown for named "
        "reasons. Remains, with the missing premise named: a formal's object pointer stored into another "
        "formal's object (Q36's example C: no contract or tracked state gives `*s`'s owner or bounds `*q`'s "
        "lifetime by it; a derived Stored summary mode), an owned or named identity stored into a caller's or "
        "a tracked owner's record, `*slot` stores, element stores an older route decides, and the residuals "
        "CG11-UNRESOLVED-OWNER-POINTER-STORES, CG11-POINTER-STORE-SUMMARY-MODE and CG11-UNREAD-POINTER-PLACES. "
        + CENSUS + "."
    ),
    "W43-INDIRECT-POINTER-VALUE-STORE": (
        " AMENDED by CG11-POINTER-VALUED-STORES (rule 10v): the custody effect of an identity-free pointer "
        "stored into an untracked object is stated (none: no custody moves, the body's summary is withheld) "
        "and admitted; a tracked or untracked identity stored anywhere keeps the refusal. Registered size "
        "carried (not re-run by its census); " + CENSUS + "."
    ),
    "W45-SUB-OBJECT-STORE-MEMBER-UNRESOLVED": (
        " AMENDED by CG11-POINTER-VALUED-STORES (rule 10v): an identity-free pointer store into a member W45's "
        "lexical reader refuses (a record with a function-pointer or bit-field member elsewhere) is read by "
        "Clang's field id and admitted under rule 10v's premises (cio_options_init moves to Safe). Scalar "
        "stores and every other value keep this entry's refusal. Registered size carried, not re-run by "
        "its census."
    ),
    "W45-SUB-OBJECT-STORE-CHAIN": (
        " AMENDED by CG11-POINTER-VALUED-STORES (rule 10v): an identity-free pointer store into a chain's member "
        "on a never-bound root (`h->rec->name = 0`) is read with the chain's own obligations and admitted; the "
        "W45 pin a_member_chain_keeps_its_refusal is flipped by name. Scalar chains and identity-bearing "
        "values keep the refusal. Registered size carried, not re-run by its census."
    ),
    "W45-SUB-OBJECT-STORE-ROOT-UNRESOLVED": (
        " AMENDED by CG11-POINTER-VALUED-STORES (rule 10v): rule 10v reads the root by its Clang declaration, so "
        "an identity-free pointer store through an untagged or pointer-typedef root is admitted when its owner "
        "is untracked; file-scope roots stay CTWIN-PUBLISH's, and scalar stores and other values keep this "
        "entry. Registered size carried, not re-run by its census."
    ),
    "W45-SUB-OBJECT-STORE-FORMAL-VALUE": (
        " AMENDED by CG11-POINTER-VALUED-STORES (rule 10v): the one formal value that moves no custody, the "
        "destination's own object (`r->next = r`, mk_list_init), is admitted with the body's summary withheld "
        "(the W45 pin a_formal_pointer_stored_into_a_member_keeps_its_refusal flipped for that row by name); "
        "every other formal's pointer keeps the refusal, Q36's example C among them. rule 10v's typed census "
        "on the AFTER corpus: 376 formal-pointer stores on 180 functions (30 self stores on 4 functions "
        "admitted in the classification). DISCHARGE unchanged."
    ),
    "W45-SUB-OBJECT-STORE-OWNED-VALUE": (
        " AMENDED by CG11-POINTER-VALUED-STORES: rule 10v keeps the refusal (an identity-bearing value into any "
        "owner refuses; Lean QCore.PointerStore.the_member_release_shape_is_refused). Its typed census on the "
        "AFTER corpus: 263 local-pointer stores on 169 functions; a store whose value is a call never reaches "
        "the typed statement route."
    ),
    "W45-SUB-OBJECT-STORE-VALUE-SHAPES": (
        " AMENDED by CG11-POINTER-VALUED-STORES: rule 10v reads null, string, function and same-object address "
        "values; the rest keep the refusal. Its typed census on the AFTER corpus: pointer read 421 / 187 "
        "functions, another object's address 217 / 73, other (casts, conditionals) 186 / 133, arithmetic "
        "53 / 30."
    ),
    "W45-SUB-OBJECT-STORE-PREFIX": (
        " AMENDED by CG11-POINTER-VALUED-STORES: a member-address root `(&R->F)->f` is read as the chain it is "
        "(rule 10n's `arrow_through_address`) and admitted for an identity-free value (the TAILQ_INIT "
        "expansions: rd_kafka_bufq_init, rd_kafka_coord_cache_init, rd_kafka_coord_reqs_init, "
        "rd_kafka_mock_sharegrps_init move to Safe); a cast root and `(*PP)->f` keep the refusal "
        "(CG11-UNREAD-POINTER-PLACES, CG10's element-place decision). Registered size carried, not re-run."
    ),
    "W44-CALL-RESULT-INDIRECT-DESTINATION": (
        " AMENDED by CG11-POINTER-VALUED-STORES: unchanged by rule 10v, which reads no call: a statement "
        "holding a call never reaches the typed statement route, and a call result stored into a caller's "
        "record would carry custody that no summary mode states (W45-SUB-OBJECT-STORE-OWNED-VALUE)."
    ),
    "W43-COLLECTION-NAME-STORE": (
        " AMENDED by CG11-POINTER-VALUED-STORES: unchanged; `v[i] = p` is decided by the older element route "
        "(W43-INDEXED-RESOURCE-COLLECTIONS) before rule 10v is asked (hard invariant 3), and rule 10v would "
        "refuse a named pointer value anyway."
    ),
}


def main():
    path = sys.argv[1]
    matrix = json.load(open(path))
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    missing = set(AMEND) - ids
    if missing:
        raise SystemExit(f"missing amendment targets: {missing}")
    if {entry["id"] for entry in REGISTER} & ids:
        raise SystemExit("already applied")
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        followups.append(dict(entry, source=REPORT, recorded_by=BY))
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(followups))


if __name__ == "__main__":
    main()
