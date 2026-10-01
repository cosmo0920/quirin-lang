#!/usr/bin/env python3
"""CG25-RESOURCE-KINDS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG25-RESOURCE-KINDS (every DISCHARGE clause holds), and the two
follow-ups it absorbs: W44-DLOPEN-DECLARATION-REMEASURE (the
`dynamic_library` kind and `dlopen`/`dlclose` with loader, gateway and Lean
parity, measured by an arbiter that attributes every moved leaf) and
W43-LIBC-IN-PLACE-LIFECYCLE (an in-place lifecycle mode the gateway checks,
with Lean parity; `regcomp`/`regfree` declared). The in-place pairs still
undeclared are re-registered with sizes re-measured on this card's BEFORE, and
the kinded wrapper results and the `dlsym` symbol lifetime are registered. Keeps `dimensions.named_followups`
equal to the list length; writes the matrix in its own form (indent=1,
ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg25-resource-kinds/README.md"
BY = "CG25-RESOURCE-KINDS"

RETIRE = [
    "CG25-RESOURCE-KINDS",
    "W44-DLOPEN-DECLARATION-REMEASURE",
    "W43-LIBC-IN-PLACE-LIFECYCLE",
]

REGISTER = [
    {
        "id": "CG25-IN-PLACE-RESIDUAL-PAIRS",
        "title": "In-place init/destroy pairs still undeclared after CG25 (13 leaves, 10 functions, 5 sole-blocked on the pinned corpus)",
        "why": (
            "Registered by CG25-RESOURCE-KINDS (gateway rule 11v), which re-registers the part of "
            "W43-LIBC-IN-PLACE-LIFECYCLE its in-place mode does not decide. The mode binds an in-place obligation to "
            "one automatic object passed as `&NAME`, its initialiser status held by a plain name; the corpus pairs "
            "below fail that shape, measured on the card's BEFORE (main b6a3dc42, `scripts/kind_census.py`): "
            "pthread_rwlock_init/destroy 2 leaves / 2 functions / 2 sole (tinycthread_extra.c rwlock_init, "
            "rwlock_destroy: storage reached through a formal pointer); mtx_init/mtx_destroy 3 / 3 / 1 "
            "(rd_kafka_global_init0, rd_kafka_sasl_cyrus_global_init/term: a global object, and `thrd_success` is "
            "implementation-defined, 1 in tinycthread, so no zero-status sentinel states it); pthread_key_create "
            "4 / 2 / 1 (an integer key written through an out pointer, plus a destructor callback); "
            "pthread_attr_init 2 / 2 / 0 (mk_utils_worker_spawn, mk_sched_launch_thread: never destroyed, and "
            "pthread_create refuses both regardless); glob/globfree 2 / 1 / 0 (an error callback). The system route "
            "and the tail-pause slice have none. DISCHARGE: per pair, an in-place kind of its own whose storage shape "
            "a consumer admits (an obligation delivered to the caller through a formal, or held by a static owner), "
            "or a sentinel the implementation fixes, with Lean parity and an arbiter measurement."
        ),
    },
    {
        "id": "CG25-KINDED-WRAPPER-RESULTS",
        "title": "A wrapper returning a possibly-NULL kinded handle stays Unknown without a declared contract (2 corpus functions)",
        "why": (
            "Registered by CG25-RESOURCE-KINDS. With dlopen declared (kind dynamic_library), "
            "mk_server/mk_plugin.c:mk_plugin_load_dynamic and rddl.c:rd_dl_open0 return the handle after an "
            "`if (!handle)` arm that does not return, so the result may be NULL; inferred summaries stay heap "
            "(contract spec C04.6 rule 7), so the function's own exit has no nullable kinded result and refuses with "
            "`nullable resource handle used by a non-null contract` + `unknown_alias handle` (mk_plugin_load_dynamic "
            "has no other leaf; rd_dl_open0 also keeps rd_dl_error and snprintf refusals). The heap control of the "
            "same body is Safe (crates/qcore-c-gateway/tests/resource_kinds.rs). The caller mk_plugin_load "
            "keeps `unknown_alias handler` at its two dlclose calls. DISCHARGE: a declared kinded contract for "
            "the wrapper, checked against its body (C04.6 rule 7's declared case), in its library's provider crate, "
            "measured; or a reviewed rule inferring a nullable kinded result with Lean parity."
        ),
    },
    {
        "id": "CG25-DLSYM-SYMBOL-LIFETIME",
        "title": "A dlsym symbol used or returned after its handle's dlclose is not checked",
        "why": (
            "Registered by CG25-RESOURCE-KINDS, which declares dlclose and so revisits dlsym's "
            "`resource_return_none()` (libc/dlfcn.qv). The handle is checked (dlsym(h, ...) after dlclose(h) is a "
            "use_after_release candidate), but the symbol's validity ends at its handle's dlclose, a lifetime "
            "relation the vocabulary cannot state: `resource_return_interior(handle, ...)` would misdescribe the "
            "opaque handle as an object the symbol points into, and the fileno precedent keeps such a value "
            "untracked. Measured: 2 corpus functions call dlsym (mk_plugin_load_symbol Safe, rd_dl_sym Unknown), "
            "neither closes the handle; every corpus use of a symbol is an indirect call or a store. DISCHARGE: a "
            "lifetime relation between a borrowed handle and a non-owning result (CG24's derived-relation "
            "vocabulary or its own), with a gateway rule, Lean parity and a measurement."
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
    for entry in REGISTER:
        if entry["id"] in ids:
            raise SystemExit(f"{entry['id']} is already registered")
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in REGISTER:
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
