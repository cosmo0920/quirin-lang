"""CG12-VOID-VIEW-READBACK: the ledger text `update_ledger.py` applies.

Sizes are the card's measurement runs (private/cg12/{before,after*}, corpus
arbiter --jobs 4, system route and tail-pause slice), read with this
directory's `leaf_moves.py`, `rederive.py`, `residual_origins.py` and
`census_escapes.py`, and the parity rows `position: "void-view-readback"`.
"""

AMEND = {
    "CG12-VOID-VIEW-READBACK": (
        " AMENDED by CG12-VOID-VIEW-READBACK (gateway rule 11j), not retired: both halves landed with audited "
        "Lean (QCore.VoidViewReadback, no placeholder or project axiom) and Rust refusal controls, the BEFORE "
        "reproduced (corpus 3a25c4e9, 608 / 10 / 4,216), PREDICTION.md frozen, every moving function attributed "
        "and the absorbed follow-up amended; the DISCHARGE clause that stays open is the entry's scope: no "
        "corpus readback is connected to a stored identity (116 distinct corpus readback sites, 115 with no "
        "admitted store base, 1 whose owner escapes; CG12-READBACK-NO-STORE-BASE). CG00's control was corrected "
        "(its refusal is the store over a record the reader does not read whole) and its 76 M05 leaves are views "
        "of untracked void *, not readbacks. Landed: the view (an object-pointer BitCast from void * at a "
        "read-through position, over an untracked root or a load from a never-bound root) and the readback "
        "(a void * member of an admitted store base read back as the linked identity when the body alone writes "
        "it, every view names the stored type, else a pun refusal, and bound_read_resolves holds). Corpus "
        "+2 Safe on its base (LZ4_write16, LZ4_write32) and +3 on main e93dbe4e (with LZ4_writeLE16, jointly with CG14), system +2, no correction; 18 of the 76 M05 origins lifted, 58 remain "
        "(CG12-VIEW-RESIDUAL-FORMS)."
    ),
    "W47-FIELDS-BY-MEMBER-VOID-ESCAPES": (
        " AMENDED by CG12-VOID-VIEW-READBACK (gateway rule 11j), re-measured (scripts/census_escapes.py over "
        "W47-PREMISES-INSTANCE's archived premised census and its export): of the 80 escapes, 62 are `(void) x;` "
        "discards (Clang ToVoid, no pointer value survives: nothing to follow), 9 are void * actuals "
        "(flb_chunk_trace_context_* x5, flb_sched_timer_cb_create x2, mk_event_channel_create, "
        "flb_processor_create), 2 local conversions and 7 stores into a void * member; rule 11j follows 0 of them: "
        "none of the 7 member stores is into an admitted store base of the storing function read back there "
        "(the one local owner, flb_sp_stream_create's `stream`, never reads `stream->in` back). The other-type "
        "view (in_blob.c:761) is unchanged and not a void * view. What remains is this entry's DISCHARGE: each "
        "escape followed to its conversions back across functions (a cross-function readback through the "
        "member, which rule 11j does not attempt)."
    ),
}

REGISTER = [
    {
        "id": "CG12-READBACK-NO-STORE-BASE",
        "title": "A void * member read back through an owner that is not an admitted store base stays a borrowed alias with no identity: 115 corpus sites",
        "why": (
            "Measured by CG12-VOID-VIEW-READBACK (gateway rule 11j) on its AFTER corpus run (parity rows "
            "`position: \"void-view-readback\"`, distinct (function, line, place)): 116 pointer destinations read a "
            "void * member back; 115 have no admitted store base (the owner is a formal, a caller's object, an "
            "element such as rdkafka's `elem->key`/`elem->value`, or a record the ALIAS-AGGREGATE reader does not "
            "read whole), 1 has an owner that reaches a callee other than its release (mk_server_loop_balancer "
            "`evl->data`); 0 are read back as an identity. Each stays P2's target-free borrowed alias (never a "
            "guessed identity). DISCHARGE: an owner identity for caller-held records whose void * member a summary "
            "states (CG11's owned-field model or a derived Stored summary mode), with a readback across that "
            "summary, Lean parity and a measured re-pin."
        ),
    },
    {
        "id": "CG12-VIEW-RESIDUAL-FORMS",
        "title": "Views of void * CG12 does not read: 58 of CG00's 76 M05 corpus leaves on 26 Unknown functions",
        "why": (
            "Measured by CG12-VOID-VIEW-READBACK on its AFTER corpus run (scripts/residual_origins.py): of the 76 "
            "leaves CG00 attributed to M05 (cause cast-from-void), 58 remain on 26 functions: 39 indirect-access, "
            "6 unrecognized destination, 4 return and 3 branch refusals and 6 unresolved identities. By form: a "
            "view whose operand is char * arithmetic (`_RK_PTR`, rd_kafka_anyconf_copy/_get0, M15/M22), an "
            "assignment used as a value (`clamped = *(T *)field->pData = v`, pb_dec_varint), a view over a call "
            "result (rd_list_elem, rd_kafka_aborted_txns, the sticky assignor), a `**(T **) result` store "
            "(cio_file_native_lookup_*), a view over a root bound to a tracked or unresolved identity or a load "
            "from one (decode_metrics_entry's `instance`, flb_input_chunk_set_limits's `in->storage`), and views "
            "in a `return` whose other operand is a call (XXH_readLE32/64_align, CG06) or a `return f(*p, …)` "
            "pointer actual (rd_strcmp3, which the return-call position refuses without any cast). Also 90 "
            "unresolved-identity leaves on 40 Unknown functions replace 81 indirect-access refusals: the view is "
            "now read and its root does not resolve untracked at the flow. DISCHARGE: each form's owner card "
            "(CG16 for arithmetic, CG05/CG06 for call operands, the return-call position for pointer actuals), "
            "each re-measured against this list."
        ),
    },
]
