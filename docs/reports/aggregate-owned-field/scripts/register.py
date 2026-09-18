#!/usr/bin/env python3
"""AGGREGATE-OWNED-FIELD: the tracker edits, in one place, on the wave-38 close's
register.py precedent.

`named_followups` is hand-maintained (the generator carries it verbatim), so
this appends and removes and rewrites in the generator's byte format (indent=1,
ensure_ascii=False, trailing newline); `dimensions.named_followups` is then
re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/aggregate-owned-field/gate.json

Phase 1 (`--show`): print the full text of the entry this card retires, so the
discharge condition is read before anything moves.
Phase 2 (no flag): register six, retire one. Idempotent.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/aggregate-owned-field/README.md"
CARD = "AGGREGATE-OWNED-FIELD"

RETIRE = ["AGGREGATE-OWNED-FIELD"]


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "AGGREGATE-VIEW-FIELD",
        "A record field of string type: a view stored in an aggregate, which the composed string route's exact borrow window cannot see through",
        "Named by card AGGREGATES-V1 (docs/reports/aggregates-v1/README.md section 3.3 and section 12) and carried "
        "in AGENTS.md standing constraint 15k since wave 27; registered by AGGREGATE-OWNED-FIELD (wave 39), which "
        "landed the owned field beside it and did not land this. What it needs, in the constraint's words: "
        "source.string.v1's borrow WINDOW (constraint 15h) 'runs from the borrow to the last statement reading "
        "the view or anything derived from it', and a view stored in a record makes 'derived from it' reach "
        "through an aggregate the window computation cannot see into - a real rule, not a type-table entry. "
        "The composed string/buffer route also refuses every record statement at Q068 today "
        "(AGGREGATE-STRING-COMPOSE), so a view field has no route to land on until that is lifted. DISCHARGE: "
        "a card that extends last_view_uses (or its successor) to reach through a record binding to the views "
        "its fields hold, admits `FIELD: string` on a route that holds both records and views, and re-measures "
        "constraint 15h's exactness argument with the aggregate enumerated; until then the field type refuses "
        "at Q051 naming AGGREGATE-FIELD-TYPES.",
        "AGENTS.md standing constraint 15k; docs/reports/aggregates-v1/README.md section 3.3; " + REPORT,
    ),
    entry(
        "AGGREGATE-NESTED",
        "A record field whose type is another record: the owner path is one level deep by measurement, and a nested field needs a recursive path and a termination argument for the declaration graph",
        "Named by card AGGREGATES-V1 (section 12) and made concrete by AGGREGATE-OWNED-FIELD (wave 39): the "
        "cycle-3 ownership IR's Place::Field is deliberately ONE level (`NAME.FIELD`), measured sufficient for "
        "every record the route admits and for the CT scenarios the record family maps (CT09), so `p.inner.buf` "
        "has no spelling in the IR and `type Outer { inner: Inner }` refuses at Q051 (UnknownFieldType). What "
        "it needs: a recursive owner path in the IR with the drop-through rule stated over a tree of fields "
        "(declaration order at every level, nothing twice), a recursive construction and projection rule in "
        "source_record, a termination argument for the declaration graph (a record may not contain itself), "
        "and Lean parity for each. DISCHARGE: those four landed together, with the one-level `Place::Field` "
        "generalised rather than a second path type added beside it, and the byte-identity census re-run.",
        "docs/reports/aggregates-v1/README.md section 12; " + REPORT,
    ),
    entry(
        "AGGREGATE-TASK-CAPTURE",
        "A record with an owned field inside a task-creation capture: the transferability judgment has no aggregate kind, and the scoped-task ledger addresses owners by plain name",
        "Registered by AGGREGATE-OWNED-FIELD (wave 39) from its own boundary: the owned field is admitted on "
        "source.buffer.v1 only, and a module that spawns never reaches that route (it keeps its Q600), so a "
        "record holding a buffer cannot today be named in a spawn capture at all. What it needs: a kind for the "
        "aggregate in the transferability judgment (crates/qcore-core/src/transfer.rs) deciding whether a record "
        "is sendable exactly when every owned field is, an owner-path vocabulary in the scoped-task ledger "
        "(crates/qcore-core/src/scoped_tasks.rs, which addresses owners by name as ownership.rs did before this "
        "card), the capture's move of the whole record into the worker with the worker's drop-through, and Lean "
        "parity for the sendability rule. DISCHARGE: a card landing those in the task route with the "
        "CT47-COPY-CAPTURE refusal re-measured for the aggregate kind; until then the composition is "
        "unreachable by route selection and no refusal names it.",
        REPORT,
    ),
    entry(
        "OWNED-FIELD-LEND-WINDOW",
        "A lend in an entry that composes a record with an owned field is refused by name: the lend's custody projection addresses owners by plain name",
        "Landed by AGGREGATE-OWNED-FIELD (wave 39) as a by-name refusal at Q051. The write lend's custody "
        "projection (std_buffer::lend_custody_operations) builds LendRange nodes by a segment walk that "
        "addresses every owner as a plain name; a projection that also addressed field paths would be a second "
        "custody projection beside the first, and the card refused the combination rather than approximating "
        "it (the SLICE-PERMISSIONS-LOOP-WINDOW shape). Pinned by "
        "a_lend_beside_an_owned_field_and_an_owned_field_in_a_loop_body_refuse_by_name. DISCHARGE: the lend "
        "projection extended to compose aggregates and lend through a field path (`lend(RECORD.FIELD, LOW, "
        "HIGH)`), with the IR's LendRange owner generalised to a Place, the exclusivity rule re-proved over "
        "paths, and the refusal replaced by that projection in the same change.",
        REPORT + " section 5",
    ),
    entry(
        "OWNED-FIELD-LOOP-BODY",
        "An owned-field statement inside a for body is refused by name: the unrolled custody walk scopes acquisitions by name and a composed record's fields by path",
        "Landed by AGGREGATE-OWNED-FIELD (wave 39) as a by-name refusal at Q051, covering a construction of an "
        "owned-field type in a body, every field-path statement in a body, and a field read or release of an "
        "OUTER composed record in a body (the move would recur per iteration). The unrolled walk "
        "(std_buffer::custody_operations_with_gates, the Repeat arm owned by LOOPS-ON-COMPOSED-ROUTES this "
        "wave) renames each iteration's body-local acquisitions with an iteration scope; a composed aggregate "
        "and each of its field paths would need the same scoping, and the card did not edit that arm. "
        "DISCHARGE: the Repeat arm scoping aggregate bindings and their field paths per iteration, with the "
        "per-iteration custody identity gate (source_loop::iteration_is_custody_identity) shown to decide a "
        "body that composes and drops a record, and the refusal replaced in the same change.",
        REPORT + " section 5",
    ),
    entry(
        "OWNED-FIELD-SEQUENTIAL-UNREACHED",
        "Q103 covers the path-extended ownership vocabulary by definition, and no sequential program reaches it: the sequential route builds no aggregate operation",
        "Recorded by AGGREGATE-OWNED-FIELD (wave 39) as a measured boundary rather than a claim: the "
        "sequential route (qcore_frontend::whole_program) hands check_ownership an operation list built from "
        "plain-name ownership operations only, so the new OwnershipError variants (AggregateAtOwnerPosition, "
        "NotAnAggregate) and the path spellings of the existing ones can surface at Q103 only through a "
        "model-level caller, never through a source program on that route; Q103's row in section L21 and "
        "AGENTS.md says so. DISCHARGE: either a sequential-route construct that builds a Compose (a record "
        "with an owned field admitted on that route, AGGREGATE-ROUTE-WIDENING's territory) reaching Q103 "
        "through whole_program.rs with a pinned test, or the rows restated as not applicable when the "
        "namespace decision is taken; until then the row is a vocabulary statement and not a reachable refusal.",
        REPORT + " section 5",
    ),
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    nf = matrix["named_followups"]
    ids = {e["id"]: e for e in nf}
    if "--show" in sys.argv:
        for id_ in RETIRE:
            if id_ in ids:
                print(f"## {id_}")
                print(json.dumps(ids[id_], indent=1, ensure_ascii=False))
            else:
                print(f"## {id_} - ABSENT")
        return
    before = len(nf)
    for id_ in RETIRE:
        if id_ in ids:
            nf.remove(ids[id_])
            print(f"retired: {id_}")
        else:
            print(f"already absent: {id_}")
    present = {e["id"] for e in nf}
    for e in NEW:
        if e["id"] in present:
            print(f"already registered: {e['id']}")
            continue
        nf.append(e)
        print(f"registered: {e['id']}")
    matrix["named_followups"] = nf
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups: {before} -> {len(nf)}")


if __name__ == "__main__":
    main()
