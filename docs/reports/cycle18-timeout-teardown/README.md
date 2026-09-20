# Cycle 18 callback timeout self teardown

This increment models the `cb_timeout` failure paths without weakening the
reactor's existing refusal of `FreeEvent` while a callback is running.  A new,
separate transition applies only when the event is the current callback on the
same dispatcher, a current-callback delete has already occurred, and the event
does not borrow a descriptor.  Event allocation, callback-map argument, and
socket descriptors remain separate custody items.

## Source and policy boundary

`contract/qcore/monkey/timeout_teardown.qcore` is executable acceptance policy.
It has five declarations and 37 clauses, in this order:

1. `source_identity` — 4 clauses.
2. `cb_timeout_source` — 7 clauses.
3. `cb_timeout_teardown_path` — 1 clause.
4. `libevent_self_teardown` — 5 clauses.
5. `teardown_obligations` — 20 clauses.

Rust derives the callback signature, all three path operation counts, complete
call vectors, qualifying path indices, and the canonical summary digest from a
constructor-private `SourceSummary`.  Candidate cleanup paths are enumerated
and checked by the policy evaluator; Rust does not own the accepted call
vectors.  The two accepted source paths are indices 0 and 1, with 21 and 20
operations.  Both call `evutil_closesocket`, `event_del`, `event_free`, then
`mk_mem_free`; path 0 additionally calls `perror`.  Path 2 is the 10-operation
send-success path and does not teardown.

The Libevent premise is separately pinned to revision
`d82464a277d0f42703702c4dfd9af6af38595a83` and the complete `event.c` SHA-256
`3506c3c54cfde9934a42526dbd4a9fbacc98f65068dbb038209f71eb9a63e346`.
At that source, `event_free` calls `event_del(ev)`, `event_del` selects
`EVENT_DEL_AUTOBLOCK`, and the wait in `event_del_nolock_` is guarded by
`!EVBASE_IN_THREAD(base)`.  This is why a same-dispatcher callback transition
is distinct from the generic blocking and nonblocking delete vocabulary.

The source projection only states obligations.  `TimeoutTeardownEvidence` is a
strict, unknown-field-refusing record that binds the Monkey summary digest,
Libevent digest, policy digest, selected teardown path, exact native callback
steps, resource identities, and observed close outcomes.  The contract compares
primitive facts: nullable descriptor state; callback/delete/free/exit event and
dispatcher identities; event/map pointers; peer/write descriptors; operation
names; and each sequence edge.  Rust does not submit an `accepted_source`,
`same_dispatcher`, or `custody_distinct` boolean.  Failed close results
are classified by the existing platform relation; an unpinned failure remains
refused, while a pinned Linux failure that retires the number can discharge the
descriptor obligation.

The callback retires only pipe endpoint 1.  Endpoint 0 remains caller-owned.
The bounded checker therefore accepts the separate `peer_close` observation
that precedes callback dispatch and causes the send failure; the returned
certificate records that separation as an assumption.  It does not credit the
peer close to `cb_timeout`.

## Native evidence boundary

A fresh disposable Fluent Bit checkout at `66910c10…` and forced-Libevent
Monkey build ran the repository's sanitized `monkey_observation.c` adapter.
The adapter includes the actual pinned `mk_event.c`, invokes the original
`cb_timeout` function pointer, and links the freshly built real `libmk_core.a`
and `libevent.a`.  AddressSanitizer, UndefinedBehaviorSanitizer, and leak
detection all remained enabled.  The first sandboxed attempt was preserved and
failed with `SIGABRT` because local socket operations were denied.  The approved
unsandboxed rerun exited 0.

The fresh run contains two real self-teardown executions.  The configured audit
selected the non-injected execution at sequence 51: event 4, map address
`134224661053792`, peer descriptor 7 closed by the caller, write descriptor 8
closed by the callback, `event_del` at 54, `event_free` and `mk_mem_free` at 55,
and callback exit at 56.  It matched production source path 0 and produced a
15-operation bounded reactor certificate.  A second execution at sequence 73
matches source path 1 and is retained in the raw artifacts.

Private artifact SHA-256 values:

- observer executable: `107e1a7a22b701b03a34223b1b07f4b19b2129aaf81d0e418000b3c880dcac63`
- observations: `b7af7b443cc11ecda357681608e74ce345dce19ad1a11e4eaf91e7b4daf45cb1`
- binding sidecar: `12f8aa654d2e0b2489012b79fd769b9e561b9cea47f72336ba33b703611946b1`

`qcore-audit::timeout_teardown` loads the sealed production `cb_timeout`
summary with `load_monkey_source`, verifies the linked Libevent source against
the pinned checkout, validates a manifest over the observer, source, policy,
raw outputs, and linked library, derives every primitive evidence fact from the
raw JSONL records, re-reads source/profile/build and selected artifact bytes,
rejects report aliases, then writes the bounded report.

## Negative controls

The Rust model refuses ordinary `FreeEvent` during a callback, generic
nonblocking delete as authorization for self-free, wrong dispatchers, an event
descriptor borrow, duplicate event/map release, delete/add/activate after event
self-free, wrong callback exit, and payload access after map release.  Evidence
tests mutate every primitive QCore relationship, and also refuse aliased
peer/write descriptors, unpinned close failure, source cleanup reordering, and
unknown JSON fields.  Configured negative runs recomputed their manifest hashes
and then refused a changed dispatcher, changed map-free pointer, duplicate
sequence, and output alias; these reached semantic/alias checks rather than
stopping at metadata hashes.  The Lean audit
checks the matching positive transition and rejects ordinary free, blocking
delete, activation, or a second free from `selfTeardown`.

## Focused validation

All logs below are unique to this run and live outside the repository at
`/tmp/qcore-cycle18-timeout-teardown-20260920-a54aa8/`.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `cargo test -p qcore-core event_reactor --lib` | 0 | 17 passed, 0 failed | `core-event-reactor-final.log` |
| `cargo test -p qcore-verification --test timeout_teardown` | 0 | 6 passed, 0 failed | `verification-timeout-teardown-final.log` |
| `cargo test -p qcore-audit timeout_teardown` | 0 | audit targets compiled; name filter selected 0 tests | `audit-timeout-teardown.log` |
| `cargo clippy -p qcore-core -p qcore-verification -p qcore-audit --tests --examples -- -D warnings` | 0 | clean | `clippy-final.log` |
| `cargo fmt --all -- --check` | 0 | clean | `fmt-final.log` |
| `lake build QCore.EventDriven QCore.TimeoutTeardown` | 0 | 3 jobs built | `lean-build-final.log` |
| `lake env lean AuditTimeoutTeardown.lean` | 0 | audit accepted | `lean-audit-final.log` |
| fresh forced-Libevent native build and sanitized observer | 0 | actual `cb_timeout` paths 0 and 1 executed | `native-prepare.log`, `native-build.log`, `native-unsandboxed-run.log` |
| configured timeout audit, sequence 51 | 0 | production path 0 and native evidence checked | `configured-timeout-audit-final.log` |
| dispatcher / map pointer / sequence / alias controls | 1 each | expected semantic refusals | `negative-*/audit.log`, `configured-alias-negative.log` |

The global contract-corpus test is intentionally left to integration because
the integrator owns its declaration/clause pins.  The repository-wide standing
gate and configured native parity are also integration responsibilities.

## Integrator reconciliation list

- Add `monkey/timeout_teardown.qcore` to the corpus pin table as 5 declarations
  and 37 clauses, preserving the declaration order above.
- Register `QCore.TimeoutTeardown` in `proof/QCore.lean` and run the full Lean
  audit/no-placeholder gate.
- Add the root-owned `quirin-audit timeout-teardown` dispatch to
  `run_timeout_teardown_config`; the temporary focused example demonstrates the
  same public route and may be removed during reconciliation.
- Keep peer endpoint cleanup classified as enclosing caller evidence, not a
  callback-source action.
- Update the acceptance matrix, status, increment record, and global exports as
  required by the integration branch.
- Run the complete workspace/corpus/CLI parity gate after all sibling commits
  and pins are reconciled.
