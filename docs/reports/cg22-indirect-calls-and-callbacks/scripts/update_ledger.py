#!/usr/bin/env python3
"""CG22-INDIRECT-CALLS-AND-CALLBACKS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG22-INDIRECT-CALLS-AND-CALLBACKS (every DISCHARGE clause holds: BEFORE
reproduces the cited refusals and records the corrections, PREDICTION.md was
frozen, AFTER attributes every move, the absorbed entries are closed or
amended with re-measured sizes, Rust refusal controls and audited Lean
`QCore.IndirectCalls`) and LOADER-ABI-INLINE-FNPTR-DECLARATOR (its discharge
condition: the inline form is read, on CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's
precedent, `fnptr`), amends EXTERN-CALL-F, W47-LOOP-OWNER-FIELD-KEYED-V-AND-F,
CG06-VALUE-CONNECTIVE-RESIDUALS, CG03-RETURN-OPERAND-RESIDUALS and
W43-RETURN-COMPOUND-CALL with the re-measured sizes and corrections, registers
the card's residuals, and keeps `dimensions.named_followups` equal to the list
length. Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline). Sizes: the census runs at main b6a3dc42
(artifacts/distinct-*.json) and the AFTER arbiter (artifacts/corpus-comparison.json).
"""

import json
import sys

REPORT = "docs/reports/cg22-indirect-calls-and-callbacks/README.md"
BY = "CG22-INDIRECT-CALLS-AND-CALLBACKS"

RETIRE = ["CG22-INDIRECT-CALLS-AND-CALLBACKS", "LOADER-ABI-INLINE-FNPTR-DECLARATOR"]

AMEND = {
    "EXTERN-CALL-F": (
        " AMENDED by CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p, main b6a3dc42): the C gateway's "
        "function-pointer part is generalised there (a call through a function pointer is resolved only to a "
        "candidate set the unit's Clang AST closes and rewritten to the calls C may make; an open set keeps its "
        "refusal), and an inline function-pointer parameter declarator is now read as `fnptr`. The extern-call "
        "route (qcore-contract::extern_bridge) is unchanged: function pointers and callbacks there stay carried."
    ),
    "W47-LOOP-OWNER-FIELD-KEYED-V-AND-F": (
        " AMENDED by CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p, main b6a3dc42): the general candidate "
        "sets admit nothing through a member read through a pointer, the vtable dispatch this census keys by "
        "field or record type: any object of the type may be the one read, so the set is open unless the program "
        "is closed and that closure is stated. Measured as open sets on the census runs: corpus 220 distinct "
        "sites (plus 66 `p->a.f` chains), system 362, slice 513. Instance-sensitive V (which plugin `ins->p` is) "
        "and F still need configuration flow; this census was not re-run (nothing it reads changed)."
    ),
    "CG06-VALUE-CONNECTIVE-RESIDUALS": (
        " RE-MEASURED by CG22-INDIRECT-CALLS-AND-CALLBACKS (main b6a3dc42): the class `under an indirect callee "
        "(CG22)` is 15 distinct statements; 8 are `__builtin_assume` (a builtin's `BuiltinFnToFnPtr` call: "
        "CG21's builtin boundary, a correction to the CG22 attribution) and 7 genuine indirect callees "
        "(`decode_interface->decode_error`, `rk_conf.connect_cb`), every one an open set under rule 11p (a member "
        "read through a pointer), so they stay refused. The `rd_kafka_log0` class round 8 named is the lexical "
        "indirect-access gate (`&rk->rk_conf`, a place: CG10 / CG05-CALL-ROOTED-PLACES), not an indirect call; "
        "unchanged by CG22 (LOG0_SIZE)."
    ),
    "CG03-RETURN-OPERAND-RESIDUALS": (
        " RE-MEASURED by CG22-INDIRECT-CALLS-AND-CALLBACKS (main b6a3dc42): the condition desugar's "
        "`IndirectCallee` answer covers 40 distinct consultations: 19 are a cast's `) (` (`(int)(…)` in the "
        "`isspace`/`isprint` expansions, `(size_t)(…)`), not a callee (a correction to the CG22 attribution), "
        "and 21 genuine indirect callees, every one a member read through a pointer (nanopb's "
        "`stream->callback`, `rd_avl`'s `ravl_cmp`, SASL providers): open sets under rule 11p, refused by design."
    ),
    "W43-RETURN-COMPOUND-CALL": (
        " RE-MEASURED by CG22-INDIRECT-CALLS-AND-CALLBACKS (main b6a3dc42): its `indirect callee (CG22)` "
        "sub-population is the condition desugar's `IndirectCallee` answer, whose 40 distinct consultations are "
        "19 casts misread as a callee and 21 open sets (members through a pointer); none is resolvable "
        "(see CG03-RETURN-OPERAND-RESIDUALS)."
    ),
}

REGISTER = [
    {
        "id": "CG22-OPEN-INDIRECT-CALLS",
        "title": "Calls through a function pointer whose candidate set the unit does not close (corpus 572 distinct sites, 235 in 185 reported functions; system 432; slice 585)",
        "why": (
            "Registered by CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p). Rule 11p resolves a callee only to a "
            "candidate set the unit's Clang AST closes; on the census runs at main b6a3dc42 no site's set is closed. "
            "Corpus (distinct function, line, callee): a member read through a pointer 286 (vtable-like: "
            "`rk->rk_conf.log_cb`, `prop->set`, nanopb callbacks), a local defined by an explicit cast 172 (xxHash's "
            "`*_conv`), a formal 102, a mutable static's member or element 6, a local without an initializer 2; "
            "system: members through a pointer 362, formals 70; slice: 513 and 72. Every one keeps its older "
            "refusal. Route T is unchanged (rule 10r: no consumer classifies an indirect call). DISCHARGE: per "
            "class, a stated closed-program or configuration premise for a member through a pointer (with "
            "W47-LOOP-OWNER-FIELD-KEYED-V-AND-F's instance sensitivity), a caller-supplied candidate set for a "
            "formal (an interprocedural summary), or a cast-preserving signature check for the cast locals, each "
            "with Lean parity and a measured re-pin; never a set of every function with the signature."
        ),
    },
    {
        "id": "CG22-CALLBACK-REGISTRY-CONTRACT",
        "title": "No contract vocabulary states a callback capture: registrations stay uncontracted or Borrowed (corpus 676 distinct registrations, 584 in reported functions; system 131; slice 157)",
        "why": (
            "Registered by CG22-INDIRECT-CALLS-AND-CALLBACKS (gateway rule 11p). The callback relation is decided in "
            "the core (`callback_trace`, QCore.IndirectCalls: registration needs a live context and no earlier "
            "teardown, an invocation the registration in force and the context live, the context's release a prior "
            "unregistration; a trace ending registered is Unknown), but resource v1 states no capture and no "
            "unregistration, so route G sees no registry: a registration call stays the call it is. Census at main "
            "b6a3dc42 (a call passing a function designator): corpus 676 distinct, 2 to a declared registrar "
            "kind that captures (`msgpack_packer_init` in `flb_input_chunk_create` and `flb_input_chunk_map`: the "
            "packer keeps `data` behind a `Borrowed` formal, the contract's word), 3 to "
            "`rd_kafka_topic_partition_list_sort` (synchronous comparator); the relation finds a named context "
            "released later in the same body in 5 corpus, 1 system and 1 slice registrations "
            "(`flb_input_chunk_map`/`msgpack_packer_init`, `thrd_create` in `rd_kafka_broker_add`, "
            "`flb_ml_stream_create` in `flb_tail_file_append`, ...): census classes, no verdict. DISCHARGE: a "
            "registry contract vocabulary (capture at registration, borrow during invocation, release at "
            "unregistration or teardown) checked against bodies, with the relation consumed by route G and a "
            "measured re-pin; the msgpack `data` capture re-examined under it."
        ),
    },
    {
        "id": "CG22-INLINE-FNPTR-LOCAL-DECLARATION",
        "title": "A local declared with an inline function-pointer declarator is read as a call of its type keyword (corpus 1 leaf, 1 function)",
        "why": (
            "Found by CG22-INDIRECT-CALLS-AND-CALLBACKS. `void (*f)(void *) = g;` reaches the lexical classifier as "
            "`void (` and refuses as `call to uncontracted function `void``; the typedef'd spelling `cb_t f = g;` "
            "does not. Measured at main b6a3dc42 (scripts/keyword_callees.py): corpus 1 leaf in 1 function, never "
            "its sole leaf; system and slice 0. Fail-closed (a refusal). DISCHARGE: the declaration read from "
            "Clang (a function-pointer local whose initializer carries no identity, rule 10v's value) with a "
            "control, or a recorded decision to keep it refused."
        ),
    },
    {
        "id": "CG22-DESIGNATOR-ACTUAL-ELABORATION",
        "title": "The elaborator refuses a function designator's decay as an `indirect call (CG22)` wherever it stands (212 distinct rows: 162 builtins, 47 designator actuals, 3 genuine)",
        "why": (
            "Found by CG22-INDIRECT-CALLS-AND-CALLBACKS at main b6a3dc42 (BEFORE parity, `indirect call (CG22)`): "
            "elaboration.rs refuses `FunctionToPointerDecay` and every callee `direct_callee` does not read. Of 212 "
            "distinct rows, 162 are builtins (`__builtin_assume`, `__builtin_alloca`, `__builtin_clz*`: CG21's "
            "boundary), 47 a function designator passed as an actual (a callback registration: "
            "`rd_kafka_q_serve(..., rd_kafka_poll_cb, ...)`), 3 genuine indirect callees (`prop->set`, "
            "`prop->copy`, open). The label is misleading for the first two classes; the refusal itself is sound. "
            "DISCHARGE: the elaborator reads a designator actual as an identity-free value and a builtin through "
            "CG21's boundary, each refusing by its own name, measured; or a recorded decision."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    for amended, text in AMEND.items():
        entry = next((entry for entry in followups if entry["id"] == amended), None)
        if entry is None:
            raise SystemExit(f"{amended} is not registered")
        if BY in entry["why"]:
            raise SystemExit(f"{amended} already amended")
        entry["why"] += text
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")


if __name__ == "__main__":
    main()
