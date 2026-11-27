#!/usr/bin/env python3
"""MONKEY-VHOST-PROPERTY-CUSTODY: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends W45-CONTAINER-ROOT-REACH (the entry blocker it names is gone; the
owner relation stays open, re-measured), W44-CALLEE-CROSS-UNIT-SUMMARY (its
`mk_string_dup` part is discharged by a declared contract) and
SUP04-VARIADIC-UPSTREAM-CANDIDATES (a pointer to the second Monkey candidate);
registers the DocumentRoot leak candidate, the false accusation the contract
exposed, and the carry's teardown residual; keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: the corpus arbiter, system route and slice at the card's tip rebased
onto main bfbf0d673 (private/mvp/after2*), against BEFORE on bfbf0d673
(before2*), `scripts/census.py`, `scripts/changed.py` and W45-CONTAINER-ROOT-REACH's
`census.py` (artifacts/census/).
"""

import json
import sys

REPORT = "docs/reports/monkey-vhost-property-custody/README.md"
BY = "MONKEY-VHOST-PROPERTY-CUSTODY"

AMEND = {
    "W45-CONTAINER-ROOT-REACH": (
        " AMENDED by MONKEY-VHOST-PROPERTY-CUSTODY (gateway rule 12d; main bfbf0d673): the entry blocker named "
        "above is gone (`mk_string_dup` is declared, providers/qcore-provider-libevent/contracts/monkey/string.qv), "
        "and the owner of a field-linked child is no longer withdrawn when each child is exactly live and "
        "unencumbered: rule 12d carries the children into the container and an unlink hands them back. Re-measured "
        "with this follow-up's census.py on the AFTER corpus: mk_vhost_set_property's publication (mk_lib.c:499) now "
        "carries no leaf (the function stays Unknown on its DocumentRoot branch, MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE), "
        "mk_rconf_read's (mk_rconf.c:463) none either, and cio_chunk_open's and flb_input_chunk_create's `owns linked "
        "objects` refusals are lifted (both stay Unknown on other blockers); mk_vhost_create (mk_lib.c:481) and "
        "mk_vhost_set_single (mk_vhost.c:611) now stop at the owner relation itself. The owner relation is unchanged "
        "in size: 8 corpus sites in 5 functions (owner-returned 5, owner-published 3: mk_string_split_line, "
        "mk_vhost_read, mk_vhost_set_single, mk_vhost_create, mk_server_listen_init) plus 1 owner-local "
        "(mk_vhost_fdt_worker_init), each still Unknown; 85 mk_list_add/del sites in all. DISCHARGE unchanged for "
        "the owner relation."
    ),
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        " AMENDED by MONKEY-VHOST-PROPERTY-CUSTODY: the `mk_string_dup` part is discharged by a declared, pinned "
        "contract instead of a cross-unit route (monkey/string.qv: borrowed s, owned nullable result; pinned at "
        "66910c10, 046accf6 and Monkey 21a2c190). Its corpus leaves `call to uncontracted function mk_string_dup` "
        "went 16 -> 0 over 12 functions (main bfbf0d673); no function became Safe (each has other blockers), and "
        "one became provisional Unsafe by an exposed imprecision (mk_http_error_page, "
        "MONKEY-REFUSED-NULL-TEST-JOINED-LEAK). The rest of this follow-up is unchanged."
    ),
    "SUP04-VARIADIC-UPSTREAM-CANDIDATES": (
        " AMENDED by MONKEY-VHOST-PROPERTY-CUSTODY: a second Monkey candidate, by reading only, is registered "
        "beside the mk_vhost_set half: MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE (mk_vhost_set_property's "
        "DocumentRoot store, reached through mk_vhost_set, at master and at the tip 21a2c190)."
    ),
}

REGISTER = [
    {
        "id": "MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE",
        "title": "Monkey leak candidates by reading: a second DocumentRoot loses the first copy (mk_vhost_set_property), and mk_config_listen_parse drops its port copy",
        "why": (
            "Registered by MONKEY-VHOST-PROPERTY-CUSTODY (gateway rule 12d). By reading only, not validated, never "
            "presented as an upstream defect. (1) mk_server/mk_lib.c:502 (master and the tip 21a2c190 of "
            "cosmo0920-missing-va_end-on-null, same body): `vh->documentroot.data = mk_string_dup(v);` overwrites "
            "the host's member without releasing what it held; mk_vhost_destroy (mk_server/mk_vhost.c:752, "
            "releasing the member at :795 through mk_ptr_free) shows the host owns that copy, and mk_vhost_create "
            "zero-allocates the host, so a second DocumentRoot for one host (two pairs in one mk_vhost_set call, or "
            "two calls) loses the first copy. The gateway keeps its refusal (`indirect memory access` at :502/:503), "
            "so mk_vhost_set_property, and with it mk_vhost_set at the tip, stay Unknown: no contract says whose "
            "the member's old value is. (2) mk_server/mk_config.c:222 (same in Monkey and Fluent Bit's lib/monkey): "
            "mk_config_listen_parse sets `port = NULL` after a failed strtol on its mk_string_dup copy, and the "
            "`error:` label frees only a non-null port, so the copy is lost; the gateway does not reach it (strtol "
            "is uncontracted there). Size: 2 functions, 1 site each. DISCHARGE: each validated natively against "
            "Monkey 21a2c190 and master (a driver under LeakSanitizer or valgrind calling mk_vhost_set twice with "
            "DocumentRoot, and mk_config_listen_parse with an unparsable port), then recorded as a validated leak "
            "with the evidence an upstream report needs, or as a refuted reading; no upstream report is filed "
            "without the user's decision."
        ),
    },
    {
        "id": "MONKEY-REFUSED-NULL-TEST-JOINED-LEAK",
        "title": "A refused null test of a joined binding leaves a nullable site live on its null edge: mk_http_error_page is a false provisional Unsafe",
        "why": (
            "Registered by MONKEY-VHOST-PROPERTY-CUSTODY. Declaring mk_string_dup moved mk_server/mk_http.c "
            "mk_http_error_page Unknown -> provisional Unsafe (`leak: live obligation acquired at "
            "mk_server/mk_http.c:395`, reported at the `return -1` of :399). By reading it is not a leak: `temp` is "
            "joined from a refused mk_ptr_to_buf statement (:392, unbound) and the mk_string_dup copy (:395, "
            "nullable); the null test at :398 refuses (`unresolved resource identity temp`) and, under rule 10b, "
            "narrows the binding but keeps the site, so the null edge's exit still holds the copy, which on that "
            "path is NULL or was never acquired. Fail-closed holds (not Safe), but the published Unsafe is wrong. "
            "Size: 1 corpus function (main bfbf0d673), 0 system, 0 slice. DISCHARGE: a refused null test whose "
            "binding may denote nullable sites withdraws those sites' accusation on its null edge (an unresolved "
            "effect, as a refused statement's hand-on does), or a correlation of a nullable site with its unique "
            "binding across the join; measured on the arbiter (every withdrawn leak attributed), with Lean parity."
        ),
    },
    {
        "id": "CARRIED-CHILD-TEARDOWN-AFTER-UNLINK",
        "title": "After an unlink hands a carried child back, no route releases it: the full teardown is Unknown",
        "why": (
            "Registered by MONKEY-VHOST-PROPERTY-CUSTODY (gateway rule 12d). An unlink of a published owner hands "
            "its carried field-linked children back (LIVE), so releasing only the owner leaks them, as it must "
            "(P-B08-SHALLOW). But the body has no admitted way to release such a child: read back through its "
            "member it is a borrowed alias (rule 10t), rule 5k releases a member only through a formal, and a "
            "store of a local pointer into the member is not a field link, so `publish; unlink; free(child); "
            "free(owner)` is Unknown, not Safe (fixture monkey_vhost_property_custody.rs). Size: 0 corpus, system "
            "or slice sites (no audited body unlinks an owner it published with a carried child). DISCHARGE: a "
            "bound read of a member whose link holds (O04-READ-BOUND) admitted at a release position after the "
            "hand-back, with Lean parity in QCore.CarriedPublication, measured."
        ),
    },
]


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    for key, text in AMEND.items():
        entry = next(entry for entry in followups if entry["id"] == key)
        assert BY not in entry["why"], key
        entry["why"] += text
    for record in REGISTER:
        assert record["id"] not in ids, record["id"]
        followups.append({**record, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
