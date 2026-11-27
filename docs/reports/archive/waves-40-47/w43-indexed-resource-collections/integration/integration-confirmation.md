# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-26 for merge commit `d2cdfd71`, which
merges this card onto main `d2dd61c1` after W43-DESCRIPTOR-RESOURCE-TRACKING.
The card had already merged `d2dd61c1` itself (`eea0b30b`) and resolved the
gateway conflicts with W43-INDEX-TRAVERSAL, W43-RESULT-ALIAS-RETURNED and
W43-DESCRIPTOR-RESOURCE-TRACKING; its README §9 records those resolutions. The
integration merge therefore had no conflicts. The run used the integrator's own
worktree with `CARGO_INCREMENTAL=0` and Clang 21.1.8. Figures are in
[gate.json](gate.json).

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,362 passed / 0 failed / 11 ignored**, 289 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **176** audits |
| `unittest discover -s scripts/tests` | 0 | 330 tests |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w43-descriptor-resource-tracking/integration/artifacts` | 1 | the diagnostics `scope` string only; see below |
| composition check (zero expected delta) | 0 | **composes**: 0 leaves, 0 function or unit moves |
| `build_matrix.py --gate`, then Python | 0 / 0 | 336 follow-ups; 330 tests |

## Corpus arbiter

The integration run was compared with the previous combined baseline, which was
captured in this same worktree:

* **diagnostics:** equal once the `scope` string is set aside. This card adds a
  clause to that string saying collection ranges state which indices owe a
  release, never that an index is in bounds.
* **contracts, source-effects, chunk-contracts:** byte-identical.

The card is **verdict-neutral** on the combined tree:

* functions stay at 294 / 85 / 4,455;
* units stay at 3 / 43 / 135 / 4;
* leak stays at 271.

All 27 element operations the census recognises keep their original refusal. The
`sizeof(*v)` allocation shape (`W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`) is
the lever that would change that.

The run is archived as the new baseline under [artifacts/](artifacts/), in the
integrator worktree `/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da`.

## Open user decision

`W43-COLLECTION-UNREACHED-ELEMENT-NODE` is still open. When the certificate check
looks for leftover `Unknown` instructions, it does not see an element node the
analysis never reached. The recommendation is to record which nodes were reached
and require every unreached element node to lie behind an edge proved infeasible.
No corpus verdict depends on this. It affects only fixture `Safe` verdicts.

## Ledger

The card merged the ledger entry by entry: 329 on main plus 7 of its own, **336**.
