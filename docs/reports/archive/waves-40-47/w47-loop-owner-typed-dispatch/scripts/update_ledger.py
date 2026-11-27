#!/usr/bin/env python3
"""Apply W47-LOOP-OWNER-TYPED-DISPATCH's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: typed dispatch discharges the cross-struct part of
  W47-LOOP-OWNER-FIELD-KEYED-V-AND-F, not its DISCHARGE clause (instance-
  sensitive V and a configured F), and Step B derives no instance at the
  export, so W47-LOOP-OWNER-INSTANCE-FLOW's clause is not met either;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-FIELD-KEYED-V-AND-F,
  W47-LOOP-OWNER-INSTANCE-FLOW and W47-LOOP-OWNER-COROUTINE-POST-YIELD, once
  each.

It applies what it can and reports, rather than refuses, an entry already
registered or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-loop-owner-typed-dispatch"
AMEND = {
    "W47-LOOP-OWNER-FIELD-KEYED-V-AND-F": (
        " AMENDED by W47-LOOP-OWNER-TYPED-DISPATCH (90978ac6, configured census): V is "
        "now keyed by the Clang record type where the build's typed reading names both "
        "the dispatch and the registration (typed_dispatch_census.rs; an untyped site "
        "keeps this field-keyed reading, see W47-LOOP-OWNER-UNTYPED-SITES). That "
        "discharges the cross-struct part: edges 37,188 -> 33,573, the Go proxy's "
        "go.c:248 dispatch (struct flbgo_input_plugin) reaches no registrant, and its 8 "
        "proxy-only sites are discharged. The remaining refusals are same-type or F: "
        "125 sites (16 in_tail, 109 other-plugin; 121 raw) stay not discharged, 105 of the "
        "other-plugin ones through the threaded test's T call of flb_input_plugin_pause "
        "(56) or _resume (49), where ins->p->cb_pause is typed struct flb_input_plugin and "
        "every input plugin's callbacks are its registrants, and 4 in_calyptia_fleet "
        "sites on (input_thread, NonThreaded) (F). What remains is instance-sensitive V "
        "(which plugin ins->p is) and F."
    ),
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-LOOP-OWNER-TYPED-DISPATCH: the route switch reads the "
        "instance's runs_in_coroutine, which only flb_input_new writes, from the creating "
        "plugin's flags (src/flb_input.c:625), so a runtime copy's own flags cannot decide "
        "it. Step B (thread_entry_census.rs, derive_instance_flags) derives a copy's flag "
        "from its data flow and joins it with the creating plugin's (flb_input_new's "
        "literal plugin name, the one descriptor declaring it); its fixtures discharge "
        "the shape on flb_input_new(..., \"tail\", ...). At 90978ac6 it derives nothing for "
        "cb_tail_pause_test (3 sites): the instance is the config's list head "
        "(ins = mk_list_entry(head, ...), tests/runtime/in_tail.c:2552; the test creates it "
        "with flb_input(ctx->flb, \"tail\", NULL), :226, which returns a descriptor number) "
        "and the copy's holder escapes (ins->data = &test, :2557). Its yield path is now "
        "all same-type (flb_input_pause > flb_input_plugin_pause > in_emitter "
        "cb_emitter_pause > ... > flb_processor_run > filter_ecs cb_ecs_filter > "
        "flb_http_do > ... > net_io_write_async)."
    ),
    "W47-LOOP-OWNER-COROUTINE-POST-YIELD": (
        " AMENDED by W47-LOOP-OWNER-TYPED-DISPATCH (configured census): with typed "
        "dispatch the Go proxy collector's post-yield reason holds no declared site (its 8 "
        "proxy-only sites are discharged); 4 bodies hold 8 sites: "
        "in_calyptia_fleet_collect_once (3), in_event_test's cb_collector_server_socket "
        "and cb_collector_server_client (2) and tests/runtime/in_tail.c::"
        "cb_tail_pause_test (3, while its instance flag is underived, see "
        "W47-LOOP-OWNER-INSTANCE-FLOW)."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    kept = [dict(entry) for entry in entries]
    amended = 0
    for entry in kept:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        if text.strip() in entry["why"]:
            print(f"{entry['id']}: already amended")
        else:
            entry["why"] += text
            amended += 1
    for missing in sorted(set(AMEND) - ids):
        print(f"{missing}: not registered, amendment not applied")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired 0; "
          f"amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
