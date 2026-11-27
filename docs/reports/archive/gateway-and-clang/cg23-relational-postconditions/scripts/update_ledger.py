#!/usr/bin/env python3
"""CG23-RELATIONAL-POSTCONDITIONS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG23-RELATIONAL-POSTCONDITIONS (every DISCHARGE clause holds: the
BEFORE reproduces the cited refusals, PREDICTION.md was frozen before any
AFTER, the AFTER attributes every moving function, each absorbed follow-up is
re-registered below with re-measured sizes, Rust refusal controls and
`QCore.RelationalPost` are audited). Amends the six absorbed entries in place
(none is discharged: each names a shape the return-code vocabulary does not
state, or a declaration left to a provider card), registers the card's
residuals, and keeps `dimensions.named_followups` equal to the list length.
Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline).

Sizes: this worktree's BEFORE and AFTER corpus arbiter runs (main
`b6a3dc42`), `scripts/sizes.py`, `scripts/census.py`, `scripts/compare.py`.
"""

import json
import sys

REPORT = "docs/reports/cg23-relational-postconditions/README.md"
BY = "CG23-RELATIONAL-POSTCONDITIONS"

RETIRE = ["CG23-RELATIONAL-POSTCONDITIONS"]

AMEND = {
    "CONDITIONAL-RELEASE-BY-LITERAL": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): re-measured on main b6a3dc42, the two call "
        "sites of rd_kafka_topic_partition_destroy0 are 2 leaves in 2 functions, both sole-blocked Unknown. Return-code "
        "dispositions (contract spec C04.13) condition custody on the returned value, not on an argument, and the "
        "exit-specific summary route reads per-literal returns; neither states `released only when do_free != 0`, so "
        "the entry stays open on its own discharge condition."
    ),
    "W45-FB-BACKLOG-MAP-ARMS": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): re-read on the prepared source "
        "(src/flb_input_chunk.c), flb_input_chunk_map returns NULL on four arms and closes the chunk on one of them "
        "(the routes_mask allocation failure, :1512-1516). No return-code class separates the arms, since every failure "
        "arm returns the same NULL, and a disposition over a pointer result is not in the vocabulary (C04.13 admits "
        "signed integer results only). The function's own corpus row is Unknown (50 unsupported, 41 unknown_alias "
        "leaves). The entry stays open: it needs a per-arm source argument or a caller-side relation, not a "
        "return-code disposition. System route at main b6a3dc42: flb_input_chunk_map 1 call leaf in 1 function; "
        "cb_queue_chunks Unknown (20 unsupported, 12 unknown_alias)."
    ),
    "W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): re-measured on main b6a3dc42, "
        "rd_kafka_oidc_assertion_read_from_file is Safe and ut_sasl_oauthbearer_oidc_assertion_parse_from_file "
        "provisional Unsafe with 2 leak leaves, as recorded. The NULL is conditioned on an argument, and the "
        "exit-specific route admits integer-literal returns only, so return-code dispositions do not state it; the "
        "entry stays open on its own discharge condition."
    ),
    "W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): re-measured on main b6a3dc42, realpath 1 leaf / "
        "1 function / 0 sole-blocked, pthread_getspecific 24 leaves / 16 functions / 1 sole-blocked (the recorded 16 / "
        "15 is a correction), flb_output_thread_instance_get absent from the corpus and 8 leaves in 4 functions on the system route (recorded as six call leaves). realpath's result "
        "mode depends on an argument and pthread_getspecific's result is retained storage; a return-code disposition "
        "(C04.13) states neither, so the entry stays open on its own discharge condition."
    ),
    "W43-DESCRIPTOR-REFUSED-SHAPES": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): the status-conditioned out list is now "
        "expressible: `resource_on_return(eq, 0, resource_out(res, nonnull))` beside `resource_on_return(ne, 0, "
        "resource_borrowed(res))` (contract spec C04.13; the gateway instantiates it where the call's value, or the "
        "value stored by the statement right before, is tested in an `if`, which is every corpus spelling of "
        "getaddrinfo), exercised on the synthetic rp_open_list. getaddrinfo and freeaddrinfo stay undeclared in "
        "quirin-provider-libc (a provider card: contract-corpus pins and the provider gate). Re-measured on main "
        "b6a3dc42 (leaves / functions / sole-blocked Unknown): getaddrinfo 4/4/0, freeaddrinfo 6/4/0, fcntl 1/1/1, "
        "accept4 1/1/1 (recorded 1/1/0: a correction), fts_open and fts_close 1/1/0 each, glob and globfree 1/1/0 "
        "each, pipe, fdopen, mmap and munmap no leaf. The other shapes (per-command premise set, integer out slot, "
        "transfer on success of a pointer result, pointer sentinel) are not return-code dispositions and stay open."
    ),
    "CJSON-DELETE-SIBLING-CHAIN": (
        " AMENDED (CG23-RELATIONAL-POSTCONDITIONS, gateway rule 11r): cJSON_Delete carries no leaf on main b6a3dc42 "
        "(its item-only declaration is in force). A release of a set reachable from a parameter is not a return-code "
        "relation, so the entry stays open on its own discharge condition."
    ),
}

REGISTER = [
    {
        "id": "CG23-EXIT-SPECIFIC-BLOCKED-HELPERS",
        "title": "Integer helpers with literal returns whose own bodies are Unknown get no exit-specific summary (105 lexical candidates, 178 call-site leaves in 100 functions)",
        "why": (
            "Registered by CG23-RELATIONAL-POSTCONDITIONS (gateway rule 11r). The exit-specific route validates each "
            "returned literal's class with the ordinary route on the body whose other returns diverge, so it publishes "
            "only where that body is otherwise modelled. On main b6a3dc42 (scripts/census.py, raw sources, an upper "
            "bound): 105 `int` helpers every `return` of which is a literal (41 only through a capitalised macro) are "
            "the callee of a `call to uncontracted function` leaf; their call sites are 178 leaves in 100 non-Safe "
            "functions (54 tested where they stand or stored and tested next, 124 not), and 3 functions would be freed "
            "alone (cmt_cat.c::append_context, ctr_mpack_consume_int32_tag, ctr_mpack_consume_uint32_tag). Every one of "
            "the 105 helpers' own rows is Unknown, and the AFTER corpus is byte-identical (3d53a5d0): no class "
            "validated. DISCHARGE: per helper, its own refusals lifted (a contract or summary for what it calls), then "
            "the corpus arbiter re-measured; a helper returning enum constants or variables needs its own reading."
        ),
    },
    {
        "id": "CG23-DISPOSITION-TEST-POSITIONS",
        "title": "A return-code disposition is instantiated only by a whole `if` condition; loops, connectives, switch, declarations with an initializer and later tests refuse",
        "why": (
            "Registered by CG23-RELATIONAL-POSTCONDITIONS (gateway rule 11r). A class is applied only where an `if` "
            "condition is exactly one call tested against a plain decimal constant, or tests the value the statement "
            "right before stored (`rc = f(...); if (rc ...)`). `while`/`for`/`do` conditions, a test under `&&`/`||`/"
            "`?:`, `switch (f(...))`, `int rc = f(...); if (rc ...)`, a test after another statement, a constant "
            "spelled by a macro the preprocessor leaves as a name or with a suffix, and a pointer result's nullness "
            "refuse as `relational-untested`. No corpus callee carries a disposition at main b6a3dc42, so the corpus "
            "population is 0; the lexical upper bound over CG23-EXIT-SPECIFIC-BLOCKED-HELPERS' candidates is 124 "
            "untested sites. DISCHARGE: per position, a lowering with Lean parity that decides the class where C "
            "decides it (an assigned name tested later needs the name's value tracked to the test), measured once a "
            "corpus callee carries a disposition."
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
    for name, text in AMEND.items():
        entries = [entry for entry in followups if entry["id"] == name]
        if len(entries) != 1:
            raise SystemExit(f"{name} is not registered exactly once")
        if text.strip() in entries[0]["why"]:
            raise SystemExit(f"{name} already amended")
        entries[0]["why"] = entries[0]["why"].rstrip() + text
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
