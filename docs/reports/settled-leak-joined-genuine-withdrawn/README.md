# SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN: a withdrawn leak with an avoiding path is published

Card SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN (USER DECISION 2026-11-25:
"lanich four of queued cards"). Base `43410b230` (main). The probe
(`d09a6ae0f`) and [PREDICTION.md](PREDICTION.md) (`62daafe3c`) were
committed before the implementation (`8c3daa33b`). Gateway rule
[84](../../agents/gateway.md#constraint-84), amending rule 74c (A);
`QCore.SettledLeakJoined` with its audit. It discharges the follow-up of the
same name, which NULLABLE-GUARDED-RELEASE recorded.

## 1. Probe at base

Ten fixtures over the generic descriptor surface
(`fixtures/contracts/descriptor_generic.qv`), in
[`tests/settled_leak_joined.rs`](../../../crates/qcore-c-gateway/tests/settled_leak_joined.rs).
At `43410b230` all ten are `Unknown` with no leak, beside `nullable resource
… used by a non-null contract`:

| shape | a path avoiding the refused release leaves the identity live | base | tip |
|---|---|---|---|
| `if (flag && k_close(fd) == 0) {}` (the pinned instance) | yes | Unknown | **Unsafe** |
| `if (flag) { k_close(fd); }` | yes | Unknown | **Unsafe** |
| `while (n > 0) { k_close(fd); n = u_read(); }` | yes (zero iterations) | Unknown | **Unsafe** |
| `if (flag) k_close(fd); else if (n) k_close(fd);` | yes | Unknown | **Unsafe** |
| `if (flag) k_close(fd); if (n) return 1; return 0;` | yes | Unknown | **Unsafe** |
| `s = k_fopen(path); if (flag) k_fclose(s);` | yes | Unknown | **Unsafe** |
| `k_close(fd);` on the only path | no | Unknown | Unknown |
| `k_close(fd)` on both arms | no | Unknown | Unknown |
| `do { k_close(fd); … } while (n > 0);` | no | Unknown | Unknown |
| other arm `u_keep(s)` (a refused hand-on) | no | Unknown | Unknown |

Two more controls were added with the implementation and stay `Unknown`: an
other arm that closes a valid descriptor (`else if (fd >= 0) k_close(fd);`)
and an other arm that returns it.

Only the first row was ever claimed. The other five were never claimed on
any iterate: the marked arm reaches the exit before the exit is first
processed, so the union-joined state is all the exit ever sees. Rule 74c (A)
and the follow-up's framing ("withdraws a held report") cover the pinned
instance only. The same genuine leak was hidden by schedule in the other
five, so rule 84 decides every identity (A) would withdraw at a settled
exit, held or not.

## 2. The change

**Rule 84** (`crates/qcore-c-gateway/src/legacy_joined_leak_witness.rs`, new).

* **Candidates.** At every exit, the current-generation identities whose
  settled incoming state marks them unresolved with a refused release among
  the marks, when the function carries an `Unknown` leaf. These are exactly
  the reports rule 74c (A) withdraws.
* **The witness.** The custody work list is replayed for the candidate with
  the same per-node step (generation shift, joined-provenance release,
  refused hand-ons, transfer, field-link prune), the same edge refinements
  (descriptor protocol, pointer refinement with rule 77's hook, refused-test
  absent edge, collection edges), the same join and the same FIFO schedule.
  The one difference: an edge whose state marks the identity unresolved, or
  is `unmodeled`, is not propagated. The restricted states are joins over
  exactly the paths on which nothing marked the identity and nothing was
  refused. The leak is published when the restricted exit output claims the
  identity by `report_leaks`' own conditions and is not `unmodeled`, and a
  breadth-first path from a producer of the identity to that exit keeps it
  live, unmarked and unrefused on every edge and node output it takes.
* **The refusal cut is required.** The first implementation cut only marked
  edges. The gateway suite then failed five pins in `ampersand_run_parity`
  and `nullable_guarded_release`: a refused guard's null edge (rule 10b, the
  binding poisoned by a refused statement after `&p`) carried the site live
  where the test would have found NULL. That is the Metadata shape rule 74c
  (A) was written to withdraw. Cutting `unmodeled` edges removes every path
  through a refused statement, an undecided test edge or a retained
  infeasible loop exit. All five pins pass again, unchanged.

**Premises.**

* (P1) The analyzer's path-feasibility reading, the one every published leak
  rests on: an edge no refinement proves infeasible is taken, and a `LIVE`
  bit at an exit is the evidence `report_leaks` publishes.
* (P2) Every refusing branch of a release calls `mark_unresolved_effect` on
  its targets.
* (P3) The replay is the work list. Under `debug_assertions`, `witness`
  asserts that the uncut replay equals the settled states. The gateway suite
  runs that check on every fixture with a candidate, so a later work-list
  hook missing from the replay fails there.

Only `Unknown -> Unsafe` moves follow, plus extra `Leak` leaves in rows
already `Unsafe`. Each published leak sits beside the refused release's own
refusal and is provisional. Nothing becomes `Safe`.

**Hooks in shared files** (each commented with the card name):

* `legacy_transfer.rs`, `analyze_cfg_with_entry_collected`: one call to
  `joined_leak_route::settle` right after `held_leaks.settle` (15 lines).
* `legacy_c.rs`: the `mod joined_leak_route` declaration beside
  `settled_leak_route`.
* `legacy_leak_witness.rs` (census only, `indirect-census`): a
  `refused_release` field per witness.

**Lean.** `QCore.SettledLeakJoined`
(`proof/QCore/SettledLeakJoined.lean`, audit
`proof/AuditSettledLeakJoined.lean`) proves the following.

* The cut join is the avoiding join.
* A witness passes no refused release (`a_witness_avoids_every_refused_release`)
  and no refusal at all (`a_witness_passes_no_refusal`).
* A leak whose every live path passes a refused release stays withdrawn.
* The decision table `publishVectors` is shared with the Rust
  `PUBLISH_VECTORS` by `tests/unit/legacy_joined_leak_witness.rs`.
* A publication folds to provisional `Unsafe`.
* The pinned instance publishes and the both-arms control does not.

The model contains no `sorry` and no project axioms (only `propext` and
`Quot.sound`).

**Deliberate pin update.** In `condition_call_desugar`
`a_descriptor_keeps_the_sentinel_validity_rules`,
`if (flag && k_close(fd) == 0) {}` moves Unknown -> Unsafe with one leak.
This is the status it had before rule 74c, now published on a witness. The
same file's `k_read` declaration is spelled `__PTRDIFF_TYPE__` /
`__SIZE_TYPE__` (the LLP64 fixture rule; the scan flagged the pre-existing
plain `long`).

## 3. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6
([scripts/opt-in-current.sh](scripts/opt-in-current.sh), a verbatim copy of
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST's; [scripts/run.sh](scripts/run.sh)
extracts a `git archive` of the revision, builds that tree's release
`quirin-audit` and runs it over the prepared inputs
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`). Each run
used its own tree and binary under the worktree's `private/slj/` (not
committed). Every run went through `scripts/gate_slot.py audit`.

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---:|---|
| base | `git archive 43410b230` | `58389a4f…6c4e` | 00:27:02 / 01:30:15 | 0 | 1,187 / 2 / 3,607 / 8 |
| tip | `git archive 8c3daa33b` | `b985ad60…054b` | 02:17:24 / 03:10:26 | 0 | **1,187 / 2 / 3,607 / 8** |
| census | `git archive 8c3daa33b`, `--features qcore-c-gateway/indirect-census`, `QCORE_LEAK_WITNESS` | `72cde489…1fec` | 02:17:25 / 04:09:32 | 0 | 1,187 / 2 / 3,607 / 8 |

`8c3daa33b` is the last commit that changes the gateway's Rust; later
commits change a test fixture's spelling, scripts and records. Both runs
report `1187 Safe row(s): 448 on no declaration, 125 only on validated
declarations, 45 on a partly validated declaration, 552 on a declared
assumption`.

**No status moves and no leaf changes.** Every row's status and diagnostics
are identical between base and tip:
[moved-rows.jsonl](evidence/moved-rows.jsonl) holds only the empty move
total, and [changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt) is
empty. Totals are in [totals-base.json](evidence/totals-base.json) and
[totals-tip.json](evidence/totals-tip.json). No Unknown -> Unsafe move exists
to justify, and no row moved to Safe.

**Against the prediction.** As predicted: no status moves on the opt-in
route, and the base's eight provisional Unsafe rows are unchanged. The
`measure-prediction` block (`none` on every route) stands for the opt-in
route. The prediction did not foresee that rule 84 would take candidates
that were never claimed (§1); the corpus result is the same either way.
On this route, no identity that a refused release marks at a settled exit
has a path that avoids every mark and every refusal and stays live. The
census agrees: no published leak's witness carries `refused_release`.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host: there are no `build/cycle19` inputs or baselines. That route was not
run and nothing is reported for it. For the integrator: no status moves are
expected on the corpus, system or tail-pause slice. The possible exception is
a row where a refused release marks a leaked identity and a refusal-free path
avoids it. Such a row is a provisional Unknown -> Unsafe, to be reported as
unpredicted.

## 4. Census: provisional Unsafe rows resting on an intermediate-iterate claim

The census build writes one line per published leak and analysed
invocation ([scripts/run-census.sh](scripts/run-census.sh), 460 lines). It
gains a `refused_release` field: a refused release is among the site's
marks at the settled exit. [scripts/census.py](scripts/census.py) classifies
the current-generation lines and matches each Unsafe row to the invocation it
published ([evidence/census-tip.json](evidence/census-tip.json)):

* **Witness lines:** 60 `settled` (reproduced at the settled exit), 31
  `settled: false`, and 0 `rule84` (unresolved with a refused release among
  the marks). All 31 `settled: false` lines have the site marked unresolved
  by a refusal that is not a release:
  * `ut_testSameSubscriptions`: 16 lines, 4 leaks over 4 invocations;
  * `rd_kafka_snappy_java_uncompress`: 12 lines, rule 42's refused
    re-acquisition;
  * `rd_kafka_cgrp_calculate_subscribe_revoking_partitions`: 3 lines, status
    3, an `Unknown` row whose leak is not published.

  Rule 74c (A) does not withdraw these, and rule 84 does not read them.
* **The eight provisional Unsafe rows:**
  * Two rest only on intermediate-iterate claims (`settled: false`):
    `ut_testSameSubscriptions` (4 leaks) and `rd_kafka_snappy_java_uncompress`
    (3 leaks, beside rule 42's refusal and a possibly-null return).
  * Four rest on a settled leak: `ut_case_acknowledge_accept`,
    `ut_case_acknowledge_multiple_partitions`, `mk_http_error` and
    `mk_net_conn_create`.
  * `decode_statsd_message` has no leak leaf (a repeated release).
  * `cmt_map_metric_get` has no witness line. Its leak is not reported by a
    custody analysis the census observes.

Both intermediate-only rows were read by earlier cards as real leaks behind
another refusal: NULLABLE-GUARDED-RELEASE §4 for the `RD_UT_ASSERT` returns,
and rule 42 for the snappy shape. Their publication still depends on the
schedule. An iterate claimed the site before another refusal's marker joined,
and nothing withdraws the claim. That is recorded as a follow-up (§6), not
changed here. Each is a provisional Unsafe and no upstream defect claim.

## 5. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 43410b230
--out <worktree>/private/gate` (Clang 21, Lean 4.35). The plan was taken at
`65870f6f4` (mode full: the diff reaches `qcore-c-gateway`). The slot-queued
steps ran on the working tree after `da06cea81` (the `k_read` spelling) and
`5f2f0e1eb` (records) were committed: the rust suite was admitted at about
04:27 UTC and the Python suite ran at about 04:45 UTC. Record:
[gate/gate.json](gate/gate.json). Logs are in the worktree's `private/gate/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` (24 packages) | 101 | 697 units, 742 summaries; 10,107 passed, **6 failed**, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` (all) | 0 | 509 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

**The six failures are the base failures** named in the brief: the
K5-LOCAL-ORIGIN-ALIASES (rule 77) pins awaiting a user decision, one per unit,
and they fail identically at `43410b230`. This card did not edit them:

* `scalar_read_identity::pointer_destination_retains_opaque_identity_refusal`;
* `detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor`;
* `access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch`;
* `typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read`;
* `lexical_closure_nested_assignment::a_never_bound_root_still_certifies`;
* `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local`.

No other test failed. Clippy with `--features indirect-census` on
`qcore-c-gateway` (the census field) was also run separately and exited 0.
No provider file changed, so the provider gate was not selected. The LLP64
fixture scan over the changed test files exits 0, and
`apply_amendments.py --check --profiles-base 43410b230` exits 0.

## 6. Ledger and follow-ups

The follow-up **SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN** is retired in
[integration-amendments.json](integration-amendments.json). The refused
release's mark is kept path-separated by the restricted replay. A leak with a
path avoiding every refused release (and every other refusal) is published,
and the census in §4 is taken.

Registered:

* **SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE**: a leak claimed on an iterate whose
  settled exit marks the site by a refusal other than a release (a refused
  hand-on, test, re-acquisition or escape) is published or not depending on
  the work-list schedule. Two provisional Unsafe rows rest only on such
  claims (§4). The rule 84 replay is the tool for deciding them from settled
  facts.

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`: the call after
  `held_leaks.settle`. The replay mirrors the work-list loop of
  `analyze_cfg_with_entry_collected`. A card that adds a state-changing
  hook to that loop (as rule 77 did) must add it to `Replay::step` too, or
  the `debug_assertions` check fails on a fixture with a candidate.
* `crates/qcore-c-gateway/src/legacy_c.rs`: the module list.
* `crates/qcore-c-gateway/src/legacy_leak_witness.rs`: the `Witness` struct.
* `crates/qcore-c-gateway/tests/condition_call_desugar.rs`: the descriptor
  pin and declarations.
* Generated files (`docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean`): rerun their generators on conflict.
