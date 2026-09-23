# MAIN-GUARD-REMOVAL

Removal of the versioned main-ref guard, 2026-09-23, at the integrator's
direction. Base: `c495199f` (tree identical to the card tip `064c6777`).

## Scope

The integrator directed that the guard not be installed and be removed from the
repository. This change deletes every tracked piece of it:

| Removed | What it was |
|---|---|
| `.githooks/reference-transaction` | refused unauthorised updates to `refs/heads/main` from every worktree |
| `.githooks/pre-push` | refused unauthorised pushes to a remote's `main` |
| `.githooks/qcore-main-guard.sh` | shared provenance and token decision logic |
| `.githooks/README.md` | activation notes |
| `scripts/install_main_guard.py` | installer that set `core.hooksPath` and the token |
| `scripts/tests/test_main_guard_hooks.py` | guard behaviour tests |
| `scripts/tests/mutate_main_guard.py` | guard mutation runner |

Live guidance is edited in `README.md` (activation paragraph removed) and
`docs/agents/operations.md`. The rule that a delegated card never advances main
is **kept as policy**; the sentence now says no hook enforces it.

Not changed: historical evidence stays frozen, including
`docs/reports/card-scope-guard/`, `docs/reports/wave36-close/`, the stability
reevaluations, `docs/reports/task-context-contract-bundle/operations-before.txt`
and the increment records. No Rust, Lean, contract, corpus or verdict change.

## Local activation is separate

The hooks act only through repository-local configuration, which no commit can
change. On the integration host, `core.hooksPath` pointed at the main
checkout's `.githooks/`, with `qcore.mainguard.token` and
`qcore.mainguard.vetochildsession` beside it. Those keys are removed by the
integrator directly. The agent's attempt to unset them was refused by the
session's permission classifier and was not worked around. Until
`core.hooksPath` is unset, a checkout whose `.githooks/` is partially or wholly
deleted aborts every ref update, because `reference-transaction` fails in its
`prepared` state. `.git/qcore-main-guard.log` is untracked and is left in place
as the record of past main updates.

## Ledger

Seven follow-ups describe the removed guard's own reach and defects and are
retired because their subject no longer exists; none is discharged by a repair:

| Retired | Disposition |
|---|---|
| `MAIN-GUARD-BYPASSABLE` | moot: no guard remains to bypass |
| `MAIN-GUARD-NAMED-WORKTREE` | moot: the provenance veto is removed |
| `MAIN-GUARD-ANCESTRY-PORTABILITY` | moot: the `/proc` ancestry arm is removed |
| `MAIN-GUARD-ANCESTRY-REACH` | moot: the token layer is removed |
| `MAIN-GUARD-LOG-UNCONSUMED` | moot: nothing writes the log any more; the file is kept |
| `MAIN-GUARD-REFUSES-PACK-REFS` | moot: no hook sees `pack-refs` transactions |
| `MAIN-GUARD-PACK-REFS-UNPINNED` | moot: its unpinned refusal cannot recur |

One residual is registered: `MAIN-ADVANCE-POLICY-ONLY`. Nothing now refuses a
delegated card that advances or pushes main (the wave-33 incident shape), so the
rule rests on briefs and integrator review. The integrator can discharge it by
recording that as a deliberate decision or by reinstating a mechanism.
Registered follow-ups: **282 → 276**.

## Validation

Standing per-change gate over this change's working tree on base `064c6777`
(tree-identical to `c495199f`), run by the integrator session on 2026-09-23 with
Clang 21.1.8 present. Each step's exit was captured to a file, not piped.

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | libevent at its pinned revision |
| `cargo test --workspace --no-fail-fast` | 0 | **5,085 passed / 0 failed / 11 ignored**, 271 summaries |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | both exit 0 |
| `python3 scripts/check_lean_audits.py` | 0 | **165** Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | **313** tests |

Rust and Lean counts are unchanged from the preceding card's gate. The Python
count falls from 340 to 313, exactly the 27 tests of the deleted
`scripts/tests/test_main_guard_hooks.py`. `build_docs_index.py --check` exits 0
after adding this report's index row. Private logs remain in the integrator
session's scratch directory `gate-remove-main-guard/`.
