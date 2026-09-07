# V01 / R01: adversarial verification of the live-generation authority gate

Baseline `6c2782f`, 2026-09-07. Card B02 previously classified R01 as
verified-fixed from a suite run. This record is the deeper, targeted
adversarial pass over the generation-liveness gate itself.

**Verdict: no soundness failure found; four coverage gaps closed.** Every
adversarial trace that would have revived a destroyed generation is rejected
by the existing checker. No gate was weakened, no pinned expectation removed,
and no production code changed — the change set is tests plus Lean parity.

## Scope

- Rust: `crates/qcore-core/src/concurrency.rs` (`check_concurrent`), whose
  `destroyed: BTreeSet<SharedResourceId>` is the live-generation ledger, and
  whose `Destroy` arm carries the destruction-authority precondition.
- Lean: `proof/QCore/Concurrency.lean`, namespace
  `QCore.Concurrency.Lifetime` — the executable lifetime projection of that
  ledger — audited by `proof/AuditCycle14.lean`.
- Cited neighbours, not re-verified here: the scoped-task ledger's
  identity-reuse rule in `crates/qcore-core/src/scoped_tasks.rs`, and CT50
  (`docs/reports/w6-reused-address/README.md`) for real address reuse under
  distinct Buffer generations.

Model boundary, unchanged: `SharedResourceId` is a lifetime identity, never a
reusable address or descriptor
(`ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds`). "A fresh
generation at a reused numeric address" is therefore represented as a
*distinct* resource id; reusing the *same* id after destruction is the thing
that must fail. Both directions are exercised below.

## Event-kind inventory

`ConcurrentEvent` admits seven constructors. Three carry a resource and are
therefore subject to the liveness gate; `Access` fans out over four
`AccessKind`s crossed with guarded/unguarded:

| Constructor | Carries a resource | Liveness-gated |
|---|---|---|
| `Access { kind: Read \| Write \| AtomicRead \| AtomicWrite, guarded_by }` | yes | yes |
| `TransferCustody` | yes | yes |
| `Destroy` | yes | yes |
| `Spawn` / `Join` / `Lock` / `Unlock` | no | n/a |

The liveness check in the `Access` arm precedes footprint validation, the
guard branch and the permission check, so it is independent of `AccessKind`
and of `guarded_by`. That is why the Lean projection's single `.access`
constructor is an adequate image of all eight Rust access shapes.

## Coverage matrix — event kind x liveness state -> verdict -> covering test

All tests are in `crates/qcore-core/tests/unit/concurrency.rs` unless marked
otherwise. **(new)** marks a test added by this card; **(strengthened)** marks
an existing test whose assertion was tightened from `is_err()` to the exact
diagnostic.

### Post-destroy events (dead generation)

| Event | State | Expected | Covering test |
|---|---|---|---|
| `Access` Read, unguarded | dead | `DeadResource` | `destroyed_generation_rejects_all_access_kinds` (strengthened) |
| `Access` Write, unguarded | dead | `DeadResource` | same |
| `Access` AtomicRead, unguarded | dead | `DeadResource` | same |
| `Access` AtomicWrite, unguarded | dead | `DeadResource` | same |
| `Access` Read/Write/AtomicRead/AtomicWrite, guarded (lock taken after destroy) | dead | `DeadResource` | same |
| `Access` all four kinds, guarded by a mutex **held across** the destruction | dead | `DeadResource` | `a_guard_held_across_destruction_cannot_revive_the_generation` **(new)** |
| `Access` any kind | dead + accessing task terminal | `TerminalTask` (rejected) | `joined_child_cannot_execute_any_event` |
| `TransferCustody` (synchronized) | dead | `DeadResource` | `destroyed_generation_rejects_transfer` (strengthened) |
| `Destroy` (repeat, same custodian) | dead | `DeadResource`, and **not** `WrongCustodian` | `destroyed_generation_rejects_repeat_destruction` **(new)** |
| `Access` on a *distinct* id after a destroy | live | accepted | `join_orders_prejoin_access_and_allows_destruction_then_distinct_generation` |
| `Destroy` of a *distinct* id after a destroy | live | accepted | same (certificate lists `[7, 8]`) |
| same trace, distinct id rewritten back to the dead id | dead | rejected | same test's second mutation |
| `Access` by a child spawned after the destroy | dead | `DeadResource` | `future_permissions_and_spawned_access_cannot_revive_destroyed_generation` |

The repeat-destruction case is the one where diagnostic *identity* matters and
not merely rejection: the destroy arm removes the custody entry, so a checker
that dropped the liveness test would still reject — but as `WrongCustodian`,
i.e. as an ownership complaint rather than a use-after-free. The new test pins
`DeadResource` and asserts the absence of `WrongCustodian`.

### Destruction preconditions (live generation, incompatible authority)

| Residual authority at the destroy | Expected | Covering test |
|---|---|---|
| positive share held by a live spawned child (loan/share) | `NonquiescentDestruction` | `destruction_requires_other_permission_holders_to_be_terminal` (strengthened) |
| positive share held by a task never spawned in the trace (future holder) | `NonquiescentDestruction` | `future_permissions_and_spawned_access_cannot_revive_destroyed_generation` |
| positive share held only by an already-joined (terminal) task | accepted | `join_orders_prejoin_access_and_allows_destruction_then_distinct_generation` |
| open guard held by another task | `NonquiescentDestruction` | `destruction_requires_ordered_accesses_and_closed_other_guards` |
| another task's access not ordered before the destroy | `NonquiescentDestruction` | same |
| other task's access ordered via a mutex release/acquire edge, guards closed | accepted | same test's tail |
| custody moved into a live child (outstanding task capture) | `WrongCustodian` | `outstanding_child_custody_blocks_parent_destruction` **(new)** |
| no custodian recorded at all | `WrongCustodian`, generation stays **live** | `destruction_without_custody_is_rejected_and_leaves_the_generation_live` **(new)** |
| destroying task already joined | `TerminalTask` | `joined_child_cannot_execute_any_event` |
| custodian is the destroying task, all other authority quiescent | accepted | `synchronized_transfer_moves_unique_destruction_custody` |

The no-custodian case is the complement of the liveness rule: it confirms a
rejected destruction does not mark the generation dead, so `DeadResource` can
never be reached through a destruction the checker refused.

Adjacent, already covered elsewhere: dropped identities cannot be reacquired
in the scoped-task ledger
(`crates/qcore-core/src/scoped_tasks.rs`,
`invalid_capture_and_commit_are_atomic_and_generation_ids_never_reuse`), and
real mapping-address reuse under two distinct Buffer generations with
stale-binding rejection is CT50
(`docs/reports/w6-reused-address/README.md`).

## Lean parity

`QCore.Concurrency.Lifetime.step` already gated `.access` and `.transfer` on
`State.Live`, with matching theorems, but the `.destroy` arm's own `Live` and
custody conjuncts had no theorem. Added, with no axioms and no placeholders:

| Lean theorem | Mirrors |
|---|---|
| `destroyed_resource_rejects_destruction` | the repeat-destruction row |
| `foreign_custody_rejects_destruction` | the outstanding-capture and no-custodian rows |
| `destruction_prevents_later_transfer` | `DeadResource` on post-destroy transfer, derived from `destruction_invalidates_generation` |
| `destruction_prevents_repeat_destruction` | idem, for a second destroy by any task |

Seven new decidable examples pin the same transitions executably: repeat
destruction, post-destroy transfer, a positive-share holder blocking
destruction, the accepted control where that holder is terminal, destruction
by a non-custodian, an unordered prior access (`priorAccessesOrdered = false`),
and two distinct generations destroyed independently. All four theorems are
registered in `proof/AuditCycle14.lean` and report
`[propext, Quot.sound]` only.

Deliberately not added: access kinds in the Lean projection. The Rust liveness
check runs before `AccessKind` and `guarded_by` are consulted, so splitting
`.access` would add cases without adding an obligation.

## Gate results (post-change, this worktree)

| Gate | Result |
|---|---|
| `cargo test --workspace` | **823 passed, 0 failed** (819 baseline + 4 new) |
| `cargo test -p qcore-core --lib concurrency::` | 26 passed (22 baseline + 4) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cargo test --manifest-path providers/Cargo.toml` | 45 passed, 0 failed |
| `python3 -B -m unittest discover -s scripts/tests -p 'test_*.py'` | 43 passed |
| `cd proof && lake build` | 62 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | 20 audits passed |

No production Rust changed, so no audit artifact can move; the four full
185-unit Cycle 19 artifacts and every Safe/Unsafe/Unknown verdict are
untouched by construction. No raw unit is promoted and the CT mapping stays
11/50.

## What this does not establish

The checker consumes an explicit trusted event trace. These results bind the
liveness ledger's behaviour on that trace and nothing beyond it: no
source-to-trace refinement, no checked mutex invariant, no claim that the
declared initial permissions or the `synchronized` handoff flags describe real
program authority. Those remain the certificate's named premises.
