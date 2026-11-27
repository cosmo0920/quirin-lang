#!/usr/bin/env python3
"""OWNED-FIELD-ROUTE-REACH: the tracker edits, in one place, on the wave-39
`register.py` precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so
this appends, removes and amends in the generator's byte format (indent=1,
ensure_ascii=False, trailing newline); `dimensions.named_followups` is then
re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/owned-field-route-reach/gate.json

Phase 1 (`--show`): print the full text of every entry this card retires or
amends, so each discharge condition is read before anything moves.
Phase 2 (no flag): retire three, amend four, register five. Idempotent.
"""

import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/owned-field-route-reach/README.md"
CARD = "OWNED-FIELD-ROUTE-REACH"

#: Retired, each because its own DISCHARGE text is met word for word (see the
#: card's report section 7 for the clause-by-clause reading).
RETIRE = [
    "OWNERSHIP-COPY-OP-UNPRODUCED",
    "OWNED-FIELD-IR-REFUSALS-UNREACHED",
    "OWNED-FIELD-BOUNDARIES-ARE-ABSENCE",
]

#: Entries whose text this card makes FALSE and therefore amends in place, each
#: named in the report. An amendment appends; it never rewrites what a previous
#: card measured.
AMEND = {
    "AGGREGATE-VIEW-FIELD": (
        " AMENDED by OWNED-FIELD-ROUTE-REACH (wave 41): the closing sentence "
        "'until then the field type refuses at Q051 naming AGGREGATE-FIELD-TYPES' "
        "is no longer true and is superseded here. `FIELD: string` now refuses at "
        "Q051 naming AGGREGATE-VIEW-FIELD itself, with the reason stated (a view is "
        "{storage, offset, len} borrowed for an exact lexical window, and an "
        "aggregate has nowhere to carry that window's end), pinned by "
        "compiler/tests/owned_field_route_reach.rs::a_view_field_refuses_by_its_own_name. "
        "The DISCHARGE above is unchanged and unmet; what changed is that a card "
        "lifting this boundary now has a message to supersede."
    ),
    "AGGREGATE-NESTED": (
        " AMENDED by OWNED-FIELD-ROUTE-REACH (wave 41): 'refuses at Q051 "
        "(UnknownFieldType)' is superseded. A field declared with the name of a "
        "record type the module declares refuses at Q051 naming AGGREGATE-NESTED "
        "itself, in EITHER declaration order - the classification reads the "
        "module's declaration kinds rather than the type table being built, so a "
        "forward reference is the same refusal as a backward one - pinned by "
        "a_nested_record_field_refuses_by_its_own_name_in_both_declaration_orders. "
        "The DISCHARGE above is unchanged and unmet."
    ),
    "AGGREGATE-TASK-CAPTURE": (
        " AMENDED by OWNED-FIELD-ROUTE-REACH (wave 41): 'until then the "
        "composition is unreachable by route selection and no refusal names it' is "
        "half superseded. It is still unreachable by route selection - dispatch "
        "takes the task route first - but a record type declared in a "
        "checked-concurrency module ALREADY refused, at Q500, in the task route's "
        "declaration walk, and that refusal now names AGGREGATE-TASK-CAPTURE at the "
        "same code, the same span and over the same refused set (the "
        "CT24-CLEANUP-PREEMPTED shape), with the same module minus the record "
        "pinned as admitted so the refusal is attributable. Pinned by "
        "a_record_in_a_task_module_refuses_by_its_own_name. The DISCHARGE above is "
        "unchanged and unmet."
    ),
    "OWNED-FIELD-SEQUENTIAL-UNREACHED": (
        " RE-SCOPED by OWNED-FIELD-ROUTE-REACH (wave 41), which re-verified the "
        "measurement and found the entry's scope too narrow: it is not about Q103's "
        "grown vocabulary but about ownership's whole operation vocabulary. The "
        "sequential route's ownership_operations is built at two sites "
        "(source_extern.rs:1215, driver.rs:1713) which between them construct FOUR "
        "variants - Acquire, TransferOut, Drop, Branch - so Copy, the five aggregate "
        "operations, LendRange, WriteRange, ReadRange, both Borrow* variants, Move, "
        "Read and Write are all unreachable from any sequential source program, not "
        "only the aggregate ones. This card gives Copy a source spelling on "
        "source.buffer.v1, which inherits the checker and reports at its own Q054, "
        "so Q103 is unmoved. The DISCHARGE above is unchanged and unmet."
    ),
}


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "AGGREGATE-OWNER-POSITION-UNREACHABLE",
        "AggregateAtOwnerPosition and NotAnAggregate remain unreachable from any source program, and are kept as the IR's fail-closed backstop",
        "Successor to OWNED-FIELD-IR-REFUSALS-UNREACHED, which OWNED-FIELD-ROUTE-REACH (wave 41) retired by "
        "the documentation arm of its own discharge. The measurement survives the retirement and is recorded "
        "here so it does not disappear: the two IR refusals are UNREACHABLE FROM ANY SOURCE PROGRAM. Verified "
        "by this card through the real pipeline (docs/reports/owned-field-route-reach/scripts/reach.py, probes "
        "ir-aggregate-at-owner-lend, ir-aggregate-at-owner-index, ir-not-an-aggregate): a lend of a record "
        "refuses at Q051 naming OWNED-FIELD-LEND-WINDOW, a byte access on one at Q052 ('read the field you mean "
        "first'), and a field path on a plain buffer at Q052 ('is not a record this entry constructed') - the "
        "route's own gates deciding every shape that would build such an operation, and firing first BY DESIGN "
        "because each gives the better-localised message. This is a deliberate backstop rather than dead code: "
        "it fails closed for any future route that reaches the IR without such a gate. DISCHARGE: either a "
        "source fixture on some route that reaches either refusal through the real pipeline with the message "
        "pinned - the two candidates are named, AGGREGATE-ROUTE-WIDENING (a record admitted on the sequential "
        "route, where whole_program builds operations from names with no kind gate) and OWNED-FIELD-LEND-WINDOW's "
        "discharge (the lend projection generalised to field paths) - or a decision to delete the variants once "
        "no route can present the shape at all, which would be a weakening and needs invariant 1 argued.",
        REPORT + " section 3",
    ),
    entry(
        "COPY-BIND-SOCKET-SHAPE-COLLISION",
        "The copy bind and source.socket.v1's creation form are the same token shape, separated by route selection alone",
        "MEASURED by OWNED-FIELD-ROUTE-REACH (wave 41) in its constraint-14 census "
        "(docs/reports/owned-field-route-reach/scripts/census.py, 535 .qcore/.qc files): `let NAME = NAME;` "
        "occurs 29 times in the tree and EVERY occurrence is source.socket.v1's `let listener = tcp;` / "
        "`let client = udp;` in the socket, HTTP, event and CT42 families. The card therefore did not add the "
        "shape to source_std_buffer::buffer_use - the function that decides route selection and that the "
        "composed string/buffer route reads this route's statements through - so the two constructs are "
        "separated by route selection and by nothing else. That separation is currently pinned by two tests "
        "(the_socket_routes_creation_form_is_untouched, the_composed_string_route_keeps_its_own_refusal) and "
        "by the examples suite, but it is a property of the DISPATCH ORDER rather than of either grammar: a "
        "future route-selection change could move 29 published programs between routes without either reader "
        "changing. DISCHARGE: either a structural guard that makes the collision impossible to resolve wrongly "
        "(the two shapes told apart by a token rather than by which route was selected), or a route-selection "
        "test that enumerates every program of the claimed shape in the tree and asserts its profile, so the "
        "29 are checked rather than sampled.",
        REPORT + " sections 2 and 5",
    ),
    entry(
        "BUFFER-UNUSED-BIND-WERROR",
        "An admitted source.buffer.v1 program whose let binding is never read emits C that -Wall -Wextra -Werror refuses",
        "MEASURED in passing by OWNED-FIELD-ROUTE-REACH (wave 41), and INHERITED rather than introduced: "
        "`let n = 5;` with no later use is admitted by the route and emits `int32_t s_n = INT32_C(5);`, which "
        "clang 21.1.8 refuses under the examples suite's own flags with "
        "-Werror,-Wunused-variable (measured: quirin emit exit 0, cc exit 1). The copy bind this card lands "
        "inherits the property exactly - an unused copy is an unused binding - and so do the acquisition, the "
        "extent read and the byte read. It is fail-closed (a failure, not a silent miscompilation) and no "
        "shipped program is affected, because every published program uses what it binds; the composed "
        "string/buffer route already solves it by emitting an explicit (void) discard for an unconsumed "
        "binding (std_string_c_emit::buffer_consumers). DISCHARGE: either the same discard scan on the plain "
        "buffer route, or a route-level refusal of an unused binding with its own condition stated in section "
        "L21 - the second being the fail-closed direction and the one that tells the author rather than the C "
        "compiler.",
        REPORT + " section 5",
    ),
    entry(
        "COPY-BIND-BOUND-TIER",
        "A loop bound or a lend footprint spelled through a copy bind is refused rather than followed, because the lowering's constant tier is deliberately narrower than the checker's",
        "Landed by OWNED-FIELD-ROUTE-REACH (wave 41) as a deliberate non-widening, the LOOP-BOUND-FROM-BYTE and "
        "WRITE-LEND-BOUND-FROM-BYTE shape. LoweringConstants::observe gives the copy bind an explicit "
        "do-nothing arm: the lowering's tier is an integer literal, a literal `let` and an `extent` binding, "
        "and following a copy there would decide a loop bound and a lend footprint in a second place. Measured "
        "through the real pipeline: `let n = 2; let m = n; for i in 0..m` refuses at Q098 and "
        "`lend(b, 0, m)` at Q053 (a_bound_spelled_through_a_copy_is_refused_rather_than_followed). The "
        "CHECKER's tier does follow the copy - both the flat and the lend-structured custody projections carry "
        "a copied scalar's value forward, so an INDEX written through a copy resolves - which is why the two "
        "tiers are named separately here. DISCHARGE: a card that widens the lowering tier once (for the byte "
        "read, the field read and the copy together, with the bound decided in one place), or a decision that "
        "the tier stays literal-only, recorded with the three refusals as its consequence.",
        REPORT + " section 2",
    ),
    entry(
        "COPY-BIND-ROUTE-SCOPE",
        "The copy bind is admitted on source.buffer.v1 alone; every other route's refusal is measured and pinned rather than widened",
        "Recorded by OWNED-FIELD-ROUTE-REACH (wave 41) as the boundary of its own admission, the "
        "LOOP-ROUTE-WIDENING shape. `let NAME = NAME;` is read by source_std_buffer::lower_let and by nothing "
        "else: it is deliberately NOT a buffer_use, so the composed string/buffer route never lowers one and "
        "`let t = s;` there keeps the Q068 it always had (measured, the_composed_string_route_keeps_its_own_refusal), "
        "and the sequential, standard-IO, extern, socket, event, HTTP and task routes keep their own refusals "
        "untouched. The consequence worth naming: the ownership IR's copy rule is now reached by ONE route, so "
        "'no program can copy an owner' is a statement about source.buffer.v1 and not about the language. "
        "DISCHARGE: either the copy bind lowered on the sequential route (which would also be "
        "OWNED-FIELD-SEQUENTIAL-UNREACHED's first arm, since whole_program would then build a Copy reaching "
        "Q103), or a recorded decision that the other routes' value binds are each that route's own construct, "
        "with the per-route refusal measured as this card measured the composed string route's.",
        REPORT + " sections 2 and 5",
    ),
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    nf = matrix["named_followups"]
    ids = {e["id"]: e for e in nf}
    if "--show" in sys.argv:
        for id_ in RETIRE + list(AMEND):
            print(f"## {id_}")
            print(
                json.dumps(ids[id_], indent=1, ensure_ascii=False)
                if id_ in ids
                else "ABSENT"
            )
        return
    before = len(nf)
    for id_ in RETIRE:
        if id_ in ids:
            nf.remove(ids[id_])
            print(f"retired: {id_}")
        else:
            print(f"already absent: {id_}")
    for id_, addition in AMEND.items():
        if id_ not in ids:
            print(f"cannot amend, absent: {id_}")
            continue
        if addition.strip() in ids[id_]["why"]:
            print(f"already amended: {id_}")
            continue
        ids[id_]["why"] = ids[id_]["why"] + addition
        print(f"amended: {id_}")
    present = {e["id"] for e in nf}
    for e in NEW:
        if e["id"] in present:
            print(f"already registered: {e['id']}")
            continue
        nf.append(e)
        print(f"registered: {e['id']}")
    matrix["named_followups"] = nf
    MATRIX.write_text(
        json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(f"named_followups: {before} -> {len(nf)}")


if __name__ == "__main__":
    main()
