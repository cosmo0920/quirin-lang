#!/usr/bin/env python3
"""update_ledger.py -- FB-TEARDOWN-DESTROY-DECLARATIONS: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `FB-TEARDOWN-DESTROY-DECLARATIONS` (its DISCHARGE holds: the
  user's decision on the parameter guard, "Yes, allow it", 2026-10-16; rule
  30 read at a direct call with Lean parity, `QCore.MemberPathDirectCall`;
  the destroyers declared and checked against their bodies where a unit
  defines them; measured on the corpus, system and slice routes with every
  moved row read).
* Amended: `W45-FB-BACKLOG-REACHABLE-RELEASE` (the six teardown functions
  that destroy list entries reachable from a borrowed formal, left
  undeclared by this card).
* Registered: `FB-TEARDOWN-RELEASE-ACTUAL-IDENTITIES` (the declared
  destroyers' calls whose actual the reported routes do not bind)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
SOURCE = "docs/reports/fb-teardown-destroy-declarations/README.md"
RETIRE = ["FB-TEARDOWN-DESTROY-DECLARATIONS"]
AMEND = {
    "W45-FB-BACKLOG-REACHABLE-RELEASE": (
        " FB-TEARDOWN-DESTROY-DECLARATIONS (2026-10-16) leaves six teardown functions of the "
        "same class undeclared: flb_task_retry_clean (a retry of task->retries), "
        "flb_sched_request_invalidate (a request of the scheduler's lists), flb_output_exit and "
        "flb_input_exit_all (every instance of config's lists), flb_engine_destroy_tasks (every "
        "task of a list) and flb_tail_file_remove_all (every file of ctx's lists): declaring "
        "them resource_borrowed would hide the release of an object a caller may still track "
        "(system 16 and slice 4 uncontracted leaves at 8b360766c)."
    ),
}
REGISTER = [{
    "id": "FB-TEARDOWN-RELEASE-ACTUAL-IDENTITIES",
    "title": "Declared Fluent Bit destroyers decide nothing where the reported routes do not bind "
             "the released actual (a caller's formal, a list entry, a member read)",
    "why": "FB-TEARDOWN-DESTROY-DECLARATIONS declared fourteen teardown callees "
           "(fluent_bit/teardown_destroy.qv). Their calls traded `call to uncontracted function` "
           "for `unresolved resource identity` (system 80, slice 78), `indirect memory access` "
           "(system 16 and corpus 2, member-read actuals such as `ic->task`, "
           "`ins->singleplex_queue`) and, in flb_tail_config_create, `nullable resource ctx used "
           "by a non-null contract` (27): the actual is a list entry (mk_list_entry), a formal "
           "whose binding an earlier unmodelled step poisoned, or an object the analysis does "
           "not track, so no declaration decides the call and rule 30's direct-call reading "
           "fired nowhere on the three routes (one Safe gain, flb_sched_timer_cb_destroy, whose "
           "formal stays bound). Related: W45-FB-OR6-ELEMENT-IDENTITY (container elements have "
           "no identity), W45-FB-BACKLOG-REACHABLE-RELEASE. DISCHARGE: an identity for the "
           "released list entry or formal on these routes (or a container-ownership relation), "
           "re-measured on the system and slice routes with every moved row read.",
    "source": SOURCE,
    "recorded_by": "FB-TEARDOWN-DESTROY-DECLARATIONS",
}]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [entry for entry in entries if entry.get("id") not in RETIRE]
    for entry in entries:
        extra = AMEND.get(entry.get("id"))
        if extra and extra.strip() not in entry["why"]:
            entry["why"] += extra
    present = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] not in present:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
