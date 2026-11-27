#!/usr/bin/env python3
"""Apply W47-LOOP-OWNER-COROUTINE-HOP's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W47-LOOP-OWNER-COROUTINE-HOP: the derived coroutine hop (hop C,
  input_pre_cb_collect resolved on both loop owners), the plugin-flag gating
  (in_tail's collectors are not dispatched through it) and the structured
  reading of include/fluent-bit/flb_input.h discharge its DISCHARGE clause,
  measured on the census; the 16 sites it named stay unresolved on other,
  registered reasons;
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-FIELD-KEYED-V-AND-F
  and W47-LOOP-OWNER-INSTANCE-FLOW, once each.

It applies what it can and reports, rather than refuses, an entry already
retired, registered or amended. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-loop-owner-coroutine-hop"
RETIRE = {"W47-LOOP-OWNER-COROUTINE-HOP"}
AMEND = {
    "W47-LOOP-OWNER-FIELD-KEYED-V-AND-F": (
        " AMENDED by W47-LOOP-OWNER-COROUTINE-HOP (90978ac6, configured census): "
        "field-keyed V also carries a coroutine body's post-yield reason across struct "
        "types. The Go proxy's plugin->cb_collect(&data, len) (src/proxy/go/go.c:248, a "
        "struct flbgo_input_plugin field bound by dlsym, with no registrant in the read "
        "files) dispatches every flb_input_collector.cb_collect registrant, so the proxy "
        "collector's reason alone holds 8 sites (in_docker_events x4, in_event_test :124 "
        "and :163, in_fluentbit_metrics, in_stdin) and is one of two reasons at the 8 "
        "others; and cb_tail_pause_test reaches a yield only through field-keyed dispatch "
        "(flb_input_pause > flb_input_plugin_pause > in_emitter cb_emitter_pause > "
        "filter_multiline cb_emitter_pause > flb_ml_flush_pending_now > ... > out_gelf "
        "cb_gelf_flush > flb_io_net_write > net_io_write_async, src/flb_io.c:487). A "
        "struct-typed (or arity-checked) V removes both."
    ),
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-LOOP-OWNER-COROUTINE-HOP: the fixed test registers "
        "cb_tail_pause_test (tests/runtime/in_tail.c:2515) on the instance of a runtime "
        "copy of the tail plugin (test.plugin = *ins->p), so the census derives no "
        "coroutine flag for it (P-PLUGIN-DIR covers registrations under plugins/ only) and "
        "keeps the coroutine route for its 3 sites. Deriving the instance "
        "(flb_input(ctx, \"tail\") > in_tail_plugin, .flags = 0, copied before flb_start) "
        "drops that route."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    for retired in sorted(RETIRE - ids):
        print(f"{retired}: already retired")
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    kept = [dict(entry) for entry in entries if entry["id"] not in RETIRE]
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
    for missing in sorted(set(AMEND) - {entry["id"] for entry in kept}):
        print(f"{missing}: not registered, amendment not applied")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; "
          f"retired {before - len(kept) + len(added)}; amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
