#!/usr/bin/env python3
"""CAST-FORMALIZATION: the acceptance-matrix ledger changes.

Retires W45-SR5-SCALAR-READ-STAR-THROUGH-CAST (its DISCHARGE holds), amends
the four absorbed follow-ups with the card's re-measured sizes, and registers
the residual cast census. Idempotent (an amendment is appended once). Writes
with indent=1, ensure_ascii=False and keeps dimensions.named_followups equal
to the list length.
"""
import json
import pathlib
import sys

CARD = "CAST-FORMALIZATION"
REPORT = "docs/reports/cast-formalization/README.md"
MEASURED = ("measured on the card's arbiter, corpus 66910c10, BEFORE main cb3085365 "
            "450a9e54 649/9/4,176, AFTER 63bbb5a3 654/9/4,171; scripts/families.py and "
            "scripts/census.py, artifacts/")

RETIRE = ["W45-SR5-SCALAR-READ-STAR-THROUGH-CAST"]

AMEND = {
    "CG08-CAST-AND-QUERY-RESIDUALS": (
        f" AMENDED ({CARD}, gateway rule 12h; {MEASURED}): the cast parts re-measured. "
        "(b) `unsupported assignment expression` on a line holding an explicit pointer conversion, by "
        "the census's Clang join (a wider measure than CG08's spelling one): 307 leaves in 103 Unknown "
        "functions BEFORE, 282 in 84 AFTER (rule 12h's value-position uncast lowering admitted the rest); "
        "what remains is a cast over arithmetic or a call, a cast of a member path, or a statement the "
        "older routes refuse for another part. (c) The elaborator's CG08 refusals, de-duplicated: 515 "
        "consultations over 297 functions, unchanged by the card (445 `ArrayToPointerDecay`, 25 "
        "`BitCast`, 19 `NullToPointer`, 19 `PointerEscape`, 7 in returns and conditions); the object-"
        "pointer `BitCast`/`NoOp` arm now asks the conversion class and refuses a non-preserving one by "
        "name. The owners of (b) and (c) are unchanged."
    ),
    "W45-SUB-OBJECT-STORE-PREFIX": (
        f" AMENDED ({CARD}, gateway rule 12h; {MEASURED}): the cast root's view is now the conversion "
        "table's in the typed access-path reader (a typed view from `void *` or a character pointer, a "
        "byte view and a qualifier conversion; a punned view refused), so a cast-root access over an "
        "untracked root is read there; the lexical sub-object store route still refuses a cast root. "
        "Re-measured, the cast-root family (`unrecognized assignment destination` or an indirect access on a "
        "line holding a pointer conversion at a member root): 33 leaves in 18 Unknown functions BEFORE and "
        "AFTER. `(*PP)->f` is unchanged."
    ),
    "W43-INDEX-TRAVERSAL-STEP-COMPUTED": (
        f" AMENDED ({CARD}, gateway rule 12h; {MEASURED}): the cast half is read. A refused step whose "
        "source is an identity-preserving pointer conversion of a declared name (`p = (char *) q + n`, a "
        "byte, typed, punned or qualifier view) is lowered as the uncast step by W43's recogniser, kept only "
        "where it admits it (pinned in tests/cast_formalization.rs). On the corpus the pointer-step cast "
        "family (any refusal on a line holding a pointer conversion at a step) is 189 leaves in 74 Unknown "
        "functions BEFORE and AFTER: its casts are over member paths (`(char *) s->buf + n`) or its offsets "
        "carry an access or a call, which the custody DISCHARGE still owns."
    ),
    "CG26-TAIL-RESIDUAL-ACTUALS": (
        f" AMENDED ({CARD}, gateway rule 12h, which owns the cast-shaped trailing actuals; {MEASURED}): a "
        "trailing actual that is an identity-preserving pointer conversion of a plain name (`(void *) src`), "
        "or a scalar conversion of a non-descriptor scalar (`(int) cert_type`), is read as its operand under "
        "the declared tail. Corpus tail-cast family (the tail refusal on a line holding a cast in a variadic "
        "tail) 3 leaves / 3 functions -> 1 / 1 (rd_kafka_conf_set_ssl_cert and rd_kafka_anyconf_get0 "
        "admitted; ut_quantile's `RD_UT_ASSERT(r, …, (int64_t)i)` expansion left); system: "
        "flb_storage_quarantine_chunk's `(void *) src` admitted in both configurations; flb_coro_destroy's "
        "`((char *) coro) + sizeof(struct flb_coro)` (arithmetic over a cast) left. The non-cast shapes are "
        "unchanged."
    ),
}

REGISTER = [
    {
        "id": "CAST-FORMALIZATION-RESIDUAL-CENSUS",
        "title": "Explicit casts on refused statements after rule 12h, by class and position (corpus 7,745 joined refusals, 280 Unknown functions with every leaf on a cast's line)",
        "why": (
            f"Registered by {CARD} (gateway rule 12h; {MEASURED}; artifacts/census-after-corpus.json, the "
            "parity-only census `cast-semantics-census`). 16,743 explicit casts in 3,225 bodies; 10,384 leaves "
            "(7,745 refusals) in 2,268 functions sit on a line holding one, an upper bound (the join is by line, "
            "and a macro line associates every leaf with every cast). Left, by owner: (a) access positions: a "
            "punned view (38 casts, 29 refusals, 12 Unknown functions) and any view of a tracked root stay "
            "refused (no effective-type result is certified; a first-member or common-initial-sequence reading "
            "would need Clang's layout, CG17); (b) value positions: H-U removes only casts of a plain name, so "
            "a cast of a member path or over arithmetic or a call stays (the identity-preserving pointer cells "
            "at value positions: 1,735 casts, 1,046 refusals, 766 Unknown functions, most with another "
            "blocker on the same statement); (c) `scalar` conversions outside a variadic tail are never removed "
            "(CG20's values); (d) `function-object`, `integer-to-pointer`, `pointer-to-integer` and `unlisted` "
            "(256 casts, 141 refusals) are refused by the table; (e) `NULL` is itself a `null-pointer` cast "
            "(7,169 casts, 4,052 refusals), almost all on statements refused for another part; (f) the "
            "elaborator's CG08 refusals (515 / 297, CG08-CAST-AND-QUERY-RESIDUALS (c)). DISCHARGE: each "
            "sub-population closed by its owner with the census re-run and the arbiter re-measured; (a) only "
            "with a layout-checked first-member reading, never by admitting a pun."
        ),
    },
]


def main() -> int:
    repo = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    path = repo / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text and text.strip() not in entry["why"]:
            entry["why"] = entry["why"].rstrip() + text
    known = {entry["id"] for entry in entries}
    for new in REGISTER:
        if new["id"] not in known:
            entries.append({"id": new["id"], "title": new["title"], "why": new["why"],
                            "source": REPORT, "recorded_by": CARD})
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
