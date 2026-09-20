# Cycle 18 callback timeout self teardown

This increment models the `cb_timeout` failure paths without weakening the
reactor's existing refusal of `FreeEvent` while a callback is running.  A new,
separate transition applies only when the event is the current callback on the
same dispatcher, a current-callback delete has already occurred, and the event
does not borrow a descriptor.  Event allocation, callback-map argument, and
socket descriptors remain separate custody items.

## Source and policy boundary

`contract/qcore/monkey/timeout_teardown.qcore` is executable acceptance policy.
It has five declarations and 22 clauses, in this order:

1. `source_identity` — 4 clauses.
2. `cb_timeout_source` — 7 clauses.
3. `cb_timeout_teardown_path` — 1 clause.
4. `libevent_self_teardown` — 5 clauses.
5. `teardown_obligations` — 5 clauses.

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
trace, resource identities, and observed close outcomes.  Failed close results
are classified by the existing platform relation; an unpinned failure remains
refused, while a pinned Linux failure that retires the number can discharge the
descriptor obligation.

The callback retires only pipe endpoint 1.  Endpoint 0 remains caller-owned.
The bounded checker therefore accepts a separate `peer_close` observation to
finish the enclosing kernel trace, and the returned certificate records that
separation as an assumption.  It does not credit the peer close to `cb_timeout`.

## Native evidence boundary

No new native execution was produced in this worktree.  The already archived
Cycle 19 observation has two concrete self-teardown executions:

- `docs/reports/cycle19-native-rules/monkey_observation.jsonl`, sequences
  51–56, event 4: callback enter, event delete, event free, callback exit.
- The same file, sequences 73–78, event 7: callback enter, event delete, event
  free, callback exit.
- `docs/reports/cycle19-native-rules/monkey_bindings.jsonl` records the matching
  `evutil_closesocket`, `event_del`, `event_free`, and `mk_mem_free` calls and
  successful close results for both executions.

The integration owner must parse those observations (or a fresh equivalent)
through the configured audit.  The unit tests exercise the strict evidence
boundary and platform relation; they are not a replacement for that configured
native binding.

## Negative controls

The Rust model refuses ordinary `FreeEvent` during a callback, generic
nonblocking delete as authorization for self-free, wrong dispatchers, an event
descriptor borrow, duplicate event/map release, delete/add/activate after event
self-free, wrong callback exit, and payload access after map release.  Evidence
tests refuse reordered traces, aliased peer/write descriptors, unpinned close
failure, source cleanup reordering, and unknown JSON fields.  The Lean audit
checks the matching positive transition and rejects ordinary free, blocking
delete, activation, or a second free from `selfTeardown`.

## Focused validation

All logs below are unique to this run and live outside the repository at
`/tmp/qcore-cycle18-timeout-teardown-20260920-a54aa8/`.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `cargo test -p qcore-core event_reactor --lib` | 0 | 17 passed, 0 failed | `core-event-reactor.log` |
| `cargo test -p qcore-verification --test timeout_teardown` | 0 | 5 passed, 0 failed | `verification-timeout-teardown.log` |
| `cargo clippy -p qcore-core -p qcore-verification --tests -- -D warnings` | 0 | clean | `clippy.log` |
| `cargo fmt --all -- --check` | 0 | clean | `fmt.log` |
| `lake build QCore.EventDriven QCore.TimeoutTeardown` | 0 | 3 jobs built | `lean-build.log` |
| `lake env lean AuditTimeoutTeardown.lean` | 0 | audit accepted | `lean-audit.log` |

The global contract-corpus test is intentionally left to integration because
the integrator owns its declaration/clause pins.  The repository-wide standing
gate and configured native parity are also integration responsibilities.

## Integrator reconciliation list

- Add `monkey/timeout_teardown.qcore` to the corpus pin table as 5 declarations
  and 22 clauses, preserving the declaration order above.
- Register `QCore.TimeoutTeardown` in `proof/QCore.lean` and run the full Lean
  audit/no-placeholder gate.
- Wire the configured audit to the actual pinned Monkey and Libevent sources
  and to strict native observation parsing; pass the observed Libevent revision
  into `project_cb_timeout_source`.
- Keep peer endpoint cleanup classified as enclosing caller evidence, not a
  callback-source action.
- Update the acceptance matrix, status, increment record, and global exports as
  required by the integration branch.
- Run the complete workspace/corpus/CLI parity gate after all sibling commits
  and pins are reconciled.
