#!/usr/bin/env python3
"""Register card BASE-TYPE-READER-CENSUS's residuals in `named_followups`.

`named_followups` is HAND_MAINTAINED (build_matrix.py's SECTION_OWNERS): waves
add entries to the committed matrix and the generator carries them. Card
C-GATEWAY-INDIRECT-CENSUS-2 left its nine in its report only; this card
registers its own, and registers the one it DISCHARGES together with its
discharge, because that residual was never registered either.

Idempotent: an id already present is left exactly as it is.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
SOURCE = "docs/reports/base-type-reader-census/README.md"

ENTRIES = [
    {
        "id": "BASE-TYPE-READER-UNMEASURED",
        "title": "DISCHARGED by BASE-TYPE-READER-CENSUS (wave 37); registered here with its "
                 "discharge because it was never registered at all",
        "why": "Card C-GATEWAY-INDIRECT-CENSUS-2 (wave 36) named this residual in its report "
               "section 9 and in none of the matrix's 171 entries, together with eight "
               "siblings. It asked for the base-type reader - classify_specifiers, typedef "
               "resolution, read_locals/read_parameters - to be instrumented the same way the "
               "member reader had been, before any card was cut on its 78.5% figure. Card "
               "BASE-TYPE-READER-CENSUS (wave 37) did that and DISCHARGES it. The measurement "
               "REPRODUCES 78.5% exactly (10,514 of 13,396 Unresolved leaves carry no tag) and "
               "then CORRECTS its reading: only 7,115 of those 10,514 (67.7%) are base-type "
               "questions, the other 3,399 being chains whose base the reader DID commit and "
               "which lost the tag at a member or a subscript - the wave-36 card's own "
               "question. The base-type reader's share of the Unresolved row is 53.1%, not "
               "78.5%. Dominant cause: a typedef that DEFINES its aggregate (typedef struct "
               "TAG { ... } t;), 3,220 leaves and 17.4% of the family, which classify_specifiers "
               "answers Opaque for while the forward spelling resolves. Discounted reach: 30 "
               "functions assuming the member then classifies, 12 once it must actually "
               "classify, 17 with a per-member record-body reader beside it, 2 of the 12 with "
               "constraint 5's premise intact - against 285 sole-blocked and 3,239 touched "
               "functions. The card's verdict is that NO base-type reader change survives the "
               "discount as a standalone arbiter, the same answer wave 36 reached by the "
               "access-shape route. No entry to keep open; the open questions it exposed are "
               "the five siblings registered beside it.",
        "source": SOURCE,
    },
    {
        "id": "TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED",
        "title": "158 leaves of the dominant base-type cause name an anonymous aggregate, which "
                 "no tag-resolving reader change can reach",
        "why": "Introduced by BASE-TYPE-READER-CENSUS (wave 37). The card's Rank 1 reader "
               "change resolves TypedefTarget::Aggregate(\"struct TAG\") to "
               "DeclaredType::Record(TAG). 158 of the cause's 3,220 leaves (4.9%) carry the "
               "head `struct` with NO tag - typedef struct { ... } t; - so there is nothing to "
               "resolve to; 33 more carry `union`, which must stay Opaque because a union "
               "member may overlap a pointer, and 4 carry `enum`. Reaching the anonymous ones "
               "needs a SYNTHETIC tag keyed on the typedef name, which is a new record identity "
               "that W4 derivation, W4 validation and certificate rechecking would all have to "
               "reproduce (constraint 8's both-readers rule). DISCHARGE: a card that lands the "
               "synthetic tag with its own measured re-pin, or a recorded measured decision "
               "that 158 leaves do not justify a new record identity. NOT a discharge: citing "
               "the 158 leaves, which is leaf mass and the error this family keeps inviting.",
        "source": SOURCE,
    },
    {
        "id": "BASE-CAUSE-SECOND-DISCOUNT-PARTIAL",
        "title": "The second discount could be applied to one of fourteen base-type causes, so "
                 "the 47-function union is not comparable with the 12 and the 17",
        "why": "Introduced by BASE-TYPE-READER-CENSUS (wave 37). Committing a base's declared "
               "type yields a record TAG, not an admission: the member must then classify and "
               "the access rule must then admit it. The card could test that second step for "
               "exactly ONE cause - TypedefDefinesAggregate, whose payload names the record - "
               "where it cut the cleared set from 30 to 12. Every other cause's CLEAR column in "
               "the card's section 6.1 still carries the untested assumption, so the union "
               "figure of 47 functions and the per-cause figures of 5, 3, 2, 2, 2, 1, 1 and 1 "
               "are upper bounds on upper bounds and must not be added to or compared with the "
               "12 and the 17. DISCHARGE: stream the tag a committed base WOULD have yielded "
               "for the remaining causes and re-run the second discount, or state per cause "
               "that the question cannot be asked and why.",
        "source": SOURCE,
    },
    {
        "id": "BASE-TYPE-CENSUS-FILE-SCOPE-APPROXIMATE",
        "title": "The census's file-scope name table under-reports, so the "
                 "FileScopeDeclaration / Unknown split is approximate in one direction",
        "why": "Introduced by BASE-TYPE-READER-CENSUS (wave 37). To separate `this unit "
               "declares the base at file scope` (161 leaves) from `nothing declares it "
               "anywhere the census can see` (69 leaves), the card builds a file-scope name "
               "table using the SAME is_declaration_candidate / statement_end / read_declaration "
               "reader the function bodies use - deliberately, because a second reader here "
               "would be the mistake the census exists to avoid. The consequence is that a "
               "file-scope declaration that reader will not read is absent from the table and "
               "its name is reported Unknown, so the split under-reports FileScopeDeclaration. "
               "Neither cause decides more than one function, so nothing in the card's "
               "recommendation turns on it. DISCHARGE: measure the gap against a Clang-derived "
               "file-scope set, or record the axis as bounded in the refusing direction and "
               "stop splitting it.",
        "source": SOURCE,
    },
    {
        "id": "W36-CENSUS-FIGURES-PRE-CONTRACT-CARD",
        "title": "C-GATEWAY-INDIRECT-CENSUS-2's 18,555 / 284 / 833 describe the corpus before "
                 "its own wave's contract arbiter landed",
        "why": "Measured by BASE-TYPE-READER-CENSUS (wave 37), which counts 18,549 family "
               "leaves, 285 sole-blocked functions and 835 sole-blocked leaves on the current "
               "baseline by the same method. The delta is attributed exactly rather than "
               "plausibly, by counting the family in each archived diagnostics.json.gz of the "
               "chain: c-gateway-constant-do-while, joined-provenance-cycles and "
               "w36-corpus-contracts-3's BEFORE all hold 18,555 / 284 / 833; "
               "w36-corpus-contracts-3's AFTER and both sides of joined-provenance-call-frames "
               "hold 18,549 / 285 / 835. Wave 36's FIRST, declarative arbiter decided six leaves "
               "of this family and moved one function INTO the sole-blocked set with two "
               "leaves; the wave's second arbiter moved this family by zero. Neither card is "
               "wrong and the frozen archives must not be edited (constraint 21). This is the "
               "first measured instance of the registered residual "
               "JPC-CENSUS-CONTRACT-SET-DEPENDENT, which is therefore confirmed rather than "
               "hypothetical. DISCHARGE: publish the baseline a census was taken against beside "
               "every census figure, as BASE-TYPE-READER-CENSUS section 3.1 does.",
        "source": SOURCE,
    },
    {
        "id": "BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT",
        "title": "The indirect-census feature now carries two cards' instrumentation and is "
                 "still built by no CI job",
        "why": "Raised by C-GATEWAY-INDIRECT-CENSUS-2 (wave 36) as "
               "INDIRECT-CENSUS-FEATURE-UNBUILT, left to the integrator, and not registered. "
               "BASE-TYPE-READER-CENSUS (wave 37) has now paid into it a second time: the "
               "feature gates hooks in legacy_c.rs, typed_field_access.rs and "
               "indirect_census.rs, including note sinks threaded through "
               "classify_specifiers, declared_type, read_parameters, read_locals, "
               "read_declaration and record_variable. No workspace member enables it, so the "
               "standing gate never compiles it; each card adds its own extra clippy pass and "
               "nothing else checks it. A refactor of the declaration reader will break the "
               "census silently and the next card will pay to repair it. DISCHARGE: a CI job "
               "building `cargo clippy -p qcore-c-gateway --features indirect-census "
               "--all-targets -- -D warnings`, or a recorded decision that the instrumentation "
               "is disposable and a future census re-derives it.",
        "source": SOURCE,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    present = {entry["id"] for entry in followups}
    added = []
    for entry in ENTRIES:
        if entry["id"] in present:
            print("already registered, left alone: %s" % entry["id"])
            continue
        followups.append(entry)
        added.append(entry["id"])
    # `indent=1, ensure_ascii=False` is the committed matrix's own convention
    # and the generator's own writer (build_matrix.py), so registering entries
    # reformats nothing.
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n",
                      encoding="utf-8")
    print("registered %d: %s" % (len(added), ", ".join(added)))
    print("named_followups now %d" % len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
