#!/usr/bin/env python3
"""CT42-CELL-ROUTE: the tracker edits, in one place (the wave-38 close's precedent).

`named_followups` is hand-maintained (the generator carries it verbatim), so this
appends / amends / removes and rewrites in the generator's byte format
(indent=1, ensure_ascii=False, trailing newline); `dimensions.named_followups`
is then re-derived by

    python3 -B docs/reports/remaining-d02/build_matrix.py . \\
        --gate docs/reports/ct42-cell-route/gate.json

Phase 1 (`--show`): print the full text of every entry this card proposes to
retire, so each discharge condition is read before anything moves.
Phase 2 (no flag): register two, amend one, retire five, withdraw one of this
card's own (see WITHDRAW). Idempotent.
"""
import json
import pathlib
import sys

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
REPORT = "docs/reports/ct42-cell-route/README.md"
CARD = "CT42-CELL-ROUTE"

# Each retirement's discharge condition, quoted from the entry, and where this
# card meets it (report section). Verified registered by --show before running.
RETIRE = {
    "CT42-CELL-UNROUTED": "discharge `wire the deriver into the socket route behind source syntax that asks the disjointness question (a route card, not a mapping lane), after which CT42's socket arm becomes mappable`: met by `disjoint NAME NAME;` (README section 4) and the CT42 mapping (section 5).",
    "CT42-MAP": "discharge `a fixture, a manifest row and stage evidence on one of the two arms`: met on the socket arm by compiler/tests/fixtures/ct/CT42/, the manifest row and docs/reports/ct42-cell-route/ct_stage_evidence.json (section 5).",
    "CT42-CELL-FALLBACK-ARM-DIRECTION": "discharge `publish the correction beside the frozen report (constraint 21) and make both arms Err (or prove pairing agreement) before any source certificate carries a derived cell`: met by the appended note in docs/reports/ct42-protocol-cell/README.md and both arms now `Err` (section 2), landed before the route (section 4).",
    "CT42-CELL-DERIVED-PREMISE": "discharge `the T09 checker edit, or a recorded statement that the derived case stays a declared premise`: met by the edit, FootprintEvent::DeriveDescriptorAlias stamping no DeclaredProtocolCellIdentity, audited as QCore.SocketCellRoute.a_derived_projection_stamps_no_declared_identity (section 3).",
    "CT42-CELL-UDP-QUEUE": "discharge `a later card either re-affirms the decision with its ground or lands the datagram cell, in both cases citing this entry`: re-affirmed with its ground, citing the entry, by the admitted datagram_pair.qcore control and the socket_footprint module doc (section 4.3).",
}

AMEND = {
    "CT42-CELL-PENDING-PHASE": (
        " AMENDED by card CT42-CELL-ROUTE (wave 39): the latent gap's admitting arm is gone. The "
        "deriver's two fallback arms - an accept with nothing queued, a connect the resolution gives "
        "no listener - are now refusals reported at Q085 (SocketCellError::UnpairedAccept / "
        "UnpairedConnect, audited as QCore.SocketFootprint.an_unqueued_accept_is_refused and "
        "an_unresolved_connect_is_refused), so a route that admitted a surviving pending connect "
        "would reach a refusal and not a singleton cell judged disjoint. The DISCHARGE stands "
        "unchanged: the pending-phase cell or refusal must still land together with whatever card "
        "admits the surviving pending connect; nothing here admits one."
    ),
}


def entry(id_, title, why, source):
    return {"id": id_, "title": title, "why": why, "source": source, "recorded_by": CARD}


NEW = [
    entry(
        "SOCKET-CELL-COUPLING-UNPROVED",
        "That the transport model's cross-name coupling IS the shared logical state of a connection is measured, not proved",
        "Card CT42-CELL-ROUTE (wave 39) routes the derived protocol cell from source and audits the "
        "DERIVATION (QCore.SocketFootprint: an accepted pair shares one cell, a fresh cell is no "
        "existing cell, the three unpaired arms refuse, an admitted accept takes an existing cell) "
        "and the T09 narrowing (QCore.SocketCellRoute). What no theorem states is the ground the "
        "cell rests on: that in qcore_core::std_socket's transport model an operation through one "
        "end of a completed connection is decided by state the other end established (a send's "
        "in-flight bound read off the peer's stream; a close refused while the peer's bytes sit "
        "unread; a send refused when the peer is closed). Those are the Rust checker's measured "
        "rules, documented in socket_footprint.rs and observed by the route's tests, and "
        "QCore.StdSocket carries no theorem about peer coupling. So the sentence `the two ends of "
        "one connection share one protocol cell` is proved as a fact about the derivation and "
        "observed as a fact about the transport model. DISCHARGE: a QCore.StdSocket theorem that a "
        "connected pair's admissible operations are mutually dependent (a send's admission depends "
        "on the peer's state, a close's on the peer's committed bytes), cited from the "
        "socket_footprint module doc in place of the measured sentence.",
        REPORT + " section 3 and section 8",
    ),
    entry(
        "CT42-CONTROLS-NATIVE-UNRUN",
        "CT42's three accepted controls emit C that no runner compiles or executes",
        "Card CT42-CELL-ROUTE (wave 39) maps CT42 as a sequential-route refusal; its three accepted "
        "controls (listener_question.qcore, two_connections.qcore, datagram_pair.qcore) are admitted "
        "and emit C - the first byte-identical to the case's twin without the question, measured by "
        "compiler/tests/socket_cell_route.rs - but run_sequential_control_list only asserts that C "
        "exists, and no runner of ct_corpus.rs compiles or runs a sequential_reject control under the "
        "sanitizers. The route's native evidence is the published examples' (tcp_echo, udp_echo), "
        "not these programs'. The question erases from emission, so what an accepted control would "
        "add natively is a run of a program that spells one; nothing is claimed for it. DISCHARGE: a "
        "named workspace test that compiles and runs listener_question.qcore's emitted C under "
        "ASan/UBSan and compares its streams with the route's observation, or a sequential-reject "
        "control runner on the CT34-SANITIZED precedent, crediting the control's own file and never "
        "the case's.",
        REPORT + " section 5 and section 8",
    ),
]

#: Registered by this card's FIRST run of this script on a measurement the card
#: then superseded: `quirin check` on an HTTP module carrying `disjoint client
#: server;` was expected to refuse at Q090 and instead ADMITTED the program
#: with no judgement (the HTTP reader hands unknown statements to the socket
#: reader; the HTTP driver never ran the cell check). The card closed that
#: gap in the driver (`compile_http_source` now composes `check_socket_cells`
#: over its re-lowered transport, pinned by two socket_cell_route.rs tests),
#: so the entry's text was false and the entry is withdrawn by the same card
#: before merge — recorded here rather than silently dropped.
WITHDRAW = ["SOCKET-CELL-QUESTION-COMPOSED-ROUTES"]

_SUPERSEDED = [
    entry(
        "SOCKET-CELL-QUESTION-COMPOSED-ROUTES",
        "The disjoint question is admitted on source.socket.v1 and its reactor composition only; the HTTP composition refuses it by shape",
        "Card CT42-CELL-ROUTE (wave 39) lands `disjoint NAME NAME;` in source_socket.rs's reader, "
        "which source.socket.v1 and its SOCKET-EVENT-COMPOSE composition share, so a question over "
        "a watched (loaned) socket is admitted exactly as a send on one is (a Read under a shared "
        "loan) - measured by a socket_cell_route.rs test. source.http.v1 lowers its own statements "
        "to a SocketProgram through source_http.rs, which does not read the word: a `disjoint` "
        "statement in an HTTP module refuses at Q090 as a statement neither that route nor its "
        "transport spells, measured with quirin check and pinned. That is fail-closed and stated "
        "rather than widened. DISCHARGE: admit the question on the HTTP composition only together "
        "with a control showing the message's transport socket and its accepted peer refused as one "
        "cell there, or re-affirm the refusal as the composition's boundary.",
        REPORT + " section 4 and section 8",
    ),
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    nf = matrix["named_followups"]
    ids = {e["id"]: e for e in nf}
    if "--show" in sys.argv:
        for name, ground in RETIRE.items():
            e = ids.get(name)
            print("##", name, "-", "REGISTERED" if e else "ABSENT")
            if e:
                print(json.dumps(e, indent=1, ensure_ascii=False))
            print("   met by:", ground)
        for name in AMEND:
            print("## amend target", name, "-", "REGISTERED" if name in ids else "ABSENT")
        for e in NEW:
            print("## new", e["id"], "-", "ALREADY PRESENT" if e["id"] in ids else "absent, will register")
        return
    before = len(nf)
    for e in NEW:
        if e["id"] in ids:
            print("skip (present):", e["id"])
            continue
        nf.append(e)
        ids[e["id"]] = e
        print("registered:", e["id"])
    for name, text in AMEND.items():
        e = ids[name]
        if text.strip() in e["why"]:
            print("skip (amended):", name)
            continue
        e["why"] = e["why"] + text
        print("amended:", name)
    for name in RETIRE:
        if name not in ids:
            print("skip (already retired):", name)
            continue
        assert ids[name] in nf
        nf.remove(ids[name])
        del ids[name]
        print("retired:", name)
    for name in WITHDRAW:
        if name not in ids:
            print("skip (already withdrawn):", name)
            continue
        nf.remove(ids[name])
        del ids[name]
        print("withdrawn (this card's own superseded entry):", name)
    matrix["named_followups"] = nf
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print("named_followups", before, "->", len(nf))


if __name__ == "__main__":
    main()
