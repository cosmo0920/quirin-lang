# SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE: a leak marked by any refusal is decided from settled states

Card SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE (USER DECISION 2026-11-26: "launch
the SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE card"). Base `98c51878f` (main).
Commit order:

1. census probe `9676ea22a`;
2. fixture probe `1ebe639f6`;
3. [PREDICTION.md](PREDICTION.md) `5e9971953`;
4. implementation `1ac8ba66b`.

Gateway rule [84](../../../../agents/gateway.md#constraint-84) is amended in place
(one authority). Rules 74c and 6 gain amendment lines pointing to it.
`QCore.SettledLeakJoined` is extended. The card discharges the follow-up of
the same name, which SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN recorded.

## 1. Reproduction at base

**Fixtures** are in
[`tests/settled_leak_other_refusal.rs`](../../../../../crates/qcore-c-gateway/tests/settled_leak_other_refusal.rs).
The probe commit `1ebe639f6` pins the base behaviour; the implementation
commit replaces that test. The fixtures run over the generic `k_*` surface
with uncontracted `u_*` calls. Each refusal kind sits on one arm, and the
other arm leaves the stream open. The same shape is spelled in two
schedules:

* the avoiding arm reaches the exit first: an iterate sees only that arm and
  claims the leak;
* the refusing arm reaches it first: its marker joins first and nothing is
  claimed.

| kind | refusing arm | avoiding arm first (base) | refusing arm first (base) | both orders (tip) |
|---|---|---|---|---|
| hand-on (W44) | `u_keep(s)` | Unsafe, 1 leak | Unknown, 0 | **Unsafe, 1 leak** |
| escape | `*out = s` | Unsafe, 1 leak | Unknown, 0 | **Unsafe, 1 leak** |
| test | `if (u_ok(s))` | Unsafe, 1 leak | Unknown, 0 | **Unsafe, 1 leak** |
| each kind on every path | | Unknown, 0 | Unknown, 0 | Unknown, 0 |

Two existing pins also rested on a schedule claim at base:

* **Re-acquisition (rule 42).**
  `chained_assignment::the_scram_shape_withdraws_the_leak_and_does_not_become_safe`,
  its "old" spelling `d = q_maybe_alloc();` in a pass loop: Unsafe with a
  leak, beside the refused re-acquisition.
* **A refused statement past an undecided loop test.**
  `scalar_route_exit_edge::a_body_that_writes_the_deciding_scalar_reaches_the_unknown_arm`,
  the `flag = z_unknown(1);` row: one leak.

In both, the settled exit marks the identity and every path to it passes a
refusal (the undecided tests of `s` and `d`; the loop header's undecided
test). Both rows are now Unknown with no leak, and never Safe. These are the
deliberate pin updates.

An `RD_UT_ASSERT`-shaped fixture was tried and not kept. A counted loop that
hands the stream on every iteration, followed by a failure return, is
published at the tip on the loop's zero-iteration edge. The scalar route
does not decide that edge on the generic surface, so it is a refusal-free
path under the analyzer's path reading (P1). In `ut_testSameSubscriptions`
every path passes a refusal, and that row is measured below.

**Base census** (probe commit `9676ea22a`, whose analysis is identical to
`98c51878f`; `indirect-census` and `QCORE_LEAK_WITNESS`;
[census-kinds-base.json](evidence/census-kinds-base.json),
[census-base.json](evidence/census-base.json)). There are 64 settled and
**31 `settled: false`** current-generation lines, the same 31 the earlier
card counted. By the refusal kind of the nodes that mark the site at the
settled exit:

| function | lines | marker kinds | the marking refusals |
|---|---:|---|---|
| `ut_testSameSubscriptions` | 16 | hand-on + test | uncontracted `rd_kafka_assignor_run`, `rd_kafka_topic_partition_list_add`, `ut_destroy_metadata`, `rd_kafka_metadata_new_topic_mock`; `verifyValidityAndBalance0` in a branch condition; indirect accesses |
| `rd_kafka_snappy_java_uncompress` | 12 | test (9), hand-on + test (3) | `rd_kafka_snappy_uncompressed_length` and `rd_kafka_snappy_uncompress` in branch conditions |
| `rd_kafka_cgrp_calculate_subscribe_revoking_partitions` | 3 | hand-on + test | undecided tests of `assigned`, `revoking`, `rktpar`; uncontracted `rd_kafka_topic_partition_list_add` |

None is marked by a refused release, an escape or a refused re-acquisition.
Rule 42's re-acquisition refusal in snappy marks the older generation, not
the current one. The census `markers` field lists each refused node whose
settled output marks the site, so it over-approximates inside a loop. The
kinds are read from those nodes' `Unknown` leaves
([scripts/census_kinds.py](scripts/census_kinds.py)).

## 2. The change

**Rule 84, amended** (`crates/qcore-c-gateway/src/legacy_joined_leak_witness.rs`).

* **The candidates** (`candidates`) are taken at every exit. They are the
  current-generation identities that the settled incoming state, or the
  exit's own output computed from it by `Replay::step`, marks unresolved by
  any refusal, when the function carries an `Unknown` leaf (`decides`). They
  are read from the settled states only.
* **Rule 74c's held reports.** `Held::settle` skips every
  `(exit node, identity)` that rule 84 decides. Whether an iterate held a
  report no longer matters.
* **The decision.** A candidate is published exactly when two things hold:
  rule 84's restricted replay (every marked or `unmodeled` edge cut) still
  claims it by `report_leaks`' conditions, and the breadth-first witness
  keeps it live, unmarked and unrefused from a producer to that exit.
  Otherwise it is withdrawn. The replay and the witness are unchanged. The
  restricted replay is computed once per identity and shared by its exits,
  and the `debug_assertions` check (the uncut replay equals the settled
  states) runs once per analysis that has a candidate.
* **Moves.** Unsafe -> Unknown (a claim only an iterate made, with no
  refusal-free path) and Unknown -> Unsafe (a refusal-free arm that the
  schedule hid). Nothing moves to Safe, because a decided report sits beside
  an `Unknown` leaf.
* **Kept accusations.** A refusal-free path keeps rule W44's hand-on
  accusations (the fixtures above). Rule 12e settles its null edge without a
  mark, so its accusations are not candidates; the
  `sup07_refused_test_accusation` pins pass unchanged. Rule 42's
  earlier-generation reports are not current-generation candidates; the
  `loop_reacquisition_leak` pins pass unchanged, including
  `refused-current-generation-leak`, whose current-generation leak is a
  settled claim. The measured snappy row (§3) keeps two leaks on witnesses
  beside rule 42's refusal.
* **Not decided here: a mark beside no `Unknown` leaf.** Rule 74c's held
  report is published as before in that case. A probe over the whole gateway
  suite found 114 such exit marks, all in `Pending`-hold fixtures (a pending
  or deferred contract marks the identity; for example
  `a_unit_status_is_the_fold_of_its_function_statuses` and
  `a_deferred_contract_accuses_nothing`). Withdrawing a leak there could
  leave a row with no blocking leaf, so the case is recorded as a follow-up
  (§6), not changed.

**Hooks in shared files** (each commented with the card name):

* `legacy_transfer.rs`, `analyze_cfg_with_entry_collected`: the
  `joined_leak_route::Replay` is built once before `held_leaks.settle`;
  `candidates` is computed there and passed to both settles. This replaces
  rule 84's previous 15-line call. The work-list loop itself is not touched,
  so `Replay::step` needs no mirror change.
* `legacy_settled_leak.rs`: `Held::settle` takes `decided` and skips it
  (4 lines and the doc comment).
* `legacy_leak_witness.rs` (census only, `indirect-census`): the `markers`
  field and `Marker` struct.

**Lean.** `QCore.SettledLeakJoined`, last section, with audit lines in
`proof/AuditSettledLeakJoined.lean`:

* `Kind` (release, hand-on, test, re-acquisition, escape, other), `KStep`
  and `KRun`;
* `a_witness_avoids_every_refusal_kind`: under `EveryRefusalMarks`, a
  witnessed run passes no refusal of any kind;
* `held` models the work list's iterate claims as the prefix joins of a
  schedule;
* `held_at_an_unmarked_exit`: at an unmarked exit some iterate claims
  exactly when the settled exit does;
* `publication_is_order_independent`: two permutations of the same runs
  decide the same whenever rule 84 decides the report or the exit is
  unmarked;
* `the_old_decision_read_the_schedule`: the hand-on arm and the avoiding arm
  in the two orders decide differently under the old rule;
* `both_orders_publish_the_avoiding_arm`;
* `a_refused_only_leak_stays_withdrawn_any_kind` and
  `every_kind_on_both_arms_is_withdrawn`;
* `a_decided_report_is_never_safe`.

The Rust `decides` is bound to the Lean definition by
`tests/unit/legacy_joined_leak_witness.rs`. There is no `sorry` and no
project axiom.

## 3. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, with the prepared
inputs at `private/fb-046accf6` (read-only). The scripts are
SETTLED-LEAK-JOINED-GENUINE-WITHDRAWN's with this worktree's paths. Each run
extracted a `git archive` of its revision and built its own release
`quirin-audit`, under the worktree's `private/slo/` (not committed). Every
run went through `scripts/gate_slot.py audit`.

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---:|---|
| base | `git archive 98c51878f` | `405ddeab…d43c` | 12:17:28 / 13:26:56 | 0 | 1,205 / 2 / 3,589 / 8 |
| tip | `git archive 1ac8ba66b` | `7f00e1fe…1fe6` | 12:53:35 / 13:55:28 | 0 | **1,205 / 2 / 3,590 / 7** |
| census base | `git archive 9676ea22a`, `indirect-census` | `aafd2c93…a734` | 12:20:08 / 14:29:36 | 0 | 1,205 / 2 / 3,589 / 8 |
| census tip | `git archive 1ac8ba66b`, `indirect-census` | `e0964eb1…bdb1` | 12:53:41 / 14:45:26 | 0 | 1,205 / 2 / 3,590 / 7 |

`1ac8ba66b` is the last commit that changes the gateway's Rust; later
commits change records only. Both runs report the same Safe-row dependency
line: `1205 Safe row(s): 452 on no declaration, 125 only on validated
declarations, 47 on a partly validated declaration, 563 on a declared
assumption`.

**Moved rows** ([moved-rows.jsonl](evidence/moved-rows.jsonl),
[changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)). Two rows
changed. Totals are in [totals-base.json](evidence/totals-base.json) and
[totals-tip.json](evidence/totals-tip.json).

* **`ut_testSameSubscriptions`** (`rdkafka_sticky_assignor.c`):
  **Unsafe -> Unknown**. Its four leaks of the subscription list acquired at
  line 2967 are withdrawn. At base all four rested on `settled: false`
  claims; every census line marks the site by refused hand-ons and
  condition calls. There is **no witness**: the restricted replay finds no
  producer-to-exit path that avoids every mark and every refusal. In the
  source, the list is handed to the uncontracted
  `rd_kafka_topic_partition_list_add` in the first loop, and the assertion
  returns follow refused calls (`rd_kafka_assignor_run` and others). The
  row keeps its refusals, so it is Unknown and never Safe.
* **`rd_kafka_snappy_java_uncompress`** (`snappy.c`): **Unsafe -> Unsafe,
  one leak fewer**. The leak at the `return NULL` after
  `rd_kafka_snappy_uncompressed_length` fails is withdrawn: every path to
  that return passes the refused condition call. Two leaks stay published,
  now each on a **witness**:
  * the `clen > inlen - of` return. A path acquires `outbuf` in pass 1,
    goes round the `for` loop and takes the first chunk test in pass 2,
    with no refusal before it;
  * the `uof <= 0` return. A path acquires in pass 1, goes round the loop
    with the `while` loop skipped, and takes the `pass == 1` branch again.

  Both paths are taken under the analyzer's path reading (P1). In C, pass 2
  re-checks the chunks that pass 1 already validated, and `pass == 1` holds
  only once. That correlation is not modelled (PASS-CORRELATED-LOOP-ALLOCATION).
  The row stays a **provisional Unsafe**, beside rule 42's refusal and the
  possibly-null return. It is not an upstream defect claim.

No row moved Unknown -> Unsafe, and none moved to or from Safe.

**Against the prediction.**

* Safe and Pending moves: 0, as predicted.
* Unsafe -> Unknown: 1, within the predicted 1 to 2. `ut_testSameSubscriptions`
  moved as predicted. `rd_kafka_snappy_java_uncompress` did not move. The
  prediction assumed the refused re-acquisition's `unmodeled` output cuts
  every path. In the restricted replay, though, the back edge that would
  re-enter the acquisition with a live generation passes the refused
  condition calls and is itself cut. The acquisition stays clean there, and
  two returns before any refusal are witnessed.
* Unknown -> Unsafe: 0, within the predicted range.
* Leaf-only changes: one, the snappy leak.

The `measure-prediction` block declares no move on the pinned routes, which
were not run.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host: there are no `build/cycle19` inputs or baselines. That route was not
run and nothing is reported for it. For the integrator, a pinned row whose
leak rests only on an iterate claim marked by a non-release refusal moves
Unsafe -> Unknown, or keeps fewer leaves. A row with a refusal-free arm
beside a marking arm may move Unknown -> Unsafe. Each is to be reported as
unpredicted.

## 4. Census at the tip

The census build of the tip gives [census-tip.json](evidence/census-tip.json)
and [census-kinds-tip.json](evidence/census-kinds-tip.json). There are 64
settled lines and **8 `settled: false`** lines, all in
`rd_kafka_snappy_java_uncompress` (several analysed invocations; marker
kinds: test 6, hand-on + test 2). Every one of them is a leak that rule 84
publishes on a witness. At the tip, a current-generation leak whose settled
exit marks the site beside an `Unknown` leaf is published only by rule 84's
witness. `census.py` still files these lines under its old `intermediate`
class, because it predates the amendment. The lines are not iterate-only
claims.

Of the seven provisional Unsafe rows:

* **Four rest on a settled leak:** `ut_case_acknowledge_accept`,
  `ut_case_acknowledge_multiple_partitions`, `mk_http_error` and
  `mk_net_conn_create`.
* **One rests on witnessed leaks:** `rd_kafka_snappy_java_uncompress` (two
  leaks, §3).
* `decode_statsd_message` has no leak leaf.
* `cmt_map_metric_get` has no witness line.

These match the base. No provisional Unsafe row rests on a claim only an
iterate made.

## 5. Gate

The gate command was `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base 98c51878f --out <worktree>/private/gate`, run with Clang 21 and Lean
4.35. Mode: full, because the diff reaches `qcore-c-gateway`. It ran on
head `1ac8ba66b` with no uncommitted scoped path. The record is
[gate/gate.json](gate/gate.json), and the logs are in the worktree's
`private/gate/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` (24 packages) | 0 | 722 units, 767 summaries; 10,341 passed, **0 failed**, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `gate_lean.py` (`QCore.SettledLeakJoined`) | 0 | 1 audit |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

Other checks:

* `scripts/check_lean_audits.py` over every audit: exit 0, 530 audits passed (the new theorems depend on `propext` and `Quot.sound` at most).
* Clippy on `qcore-c-gateway --features indirect-census` (the census
  field), `-D warnings`: exit 0.
* The LLP64 fixture scan over the changed test files exits 0 ("0 counted").
* `apply_amendments.py --check --profiles-base 98c51878f` exits 0.

No provider file changed, so the provider gate was not selected. The base
had no failing test, and none was observed. Before the pins were updated,
the gateway suite failed exactly the two deliberate pins (§1).

## 6. Ledger and follow-ups

The follow-up **SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE** is retired in
[integration-amendments.json](integration-amendments.json). Every refusal
kind's mark is decided by rule 84's restricted replay from settled states,
and both named rows are measured.

Registered:

* **SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF.** A leak whose settled exit
  marks the identity with no `Unknown` leaf in the function (the
  `Pending`-hold fixtures: a pending or deferred contract's mark) is still
  decided by rule 74c's held report, that is, by whether an iterate claimed
  it. The gateway suite has 114 such exit marks. Deciding them needs a
  choice the fail-closed rule does not settle: withdrawing could leave a row
  with no blocking leaf, and publishing an unwitnessed claim is not
  evidence.

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`: the block after
  `deferred_leaks.settle` (the `Replay` construction, `candidates`, and both
  settles). A card that adds a state-changing hook to the work-list loop
  must mirror it in `Replay::step`, or the `debug_assertions` check fails.
* `crates/qcore-c-gateway/src/legacy_settled_leak.rs`: the `Held::settle`
  signature.
* `crates/qcore-c-gateway/src/legacy_leak_witness.rs`: the `Witness` and
  `Settled` structs.
* `crates/qcore-c-gateway/tests/chained_assignment.rs` and
  `tests/scalar_route_exit_edge.rs`: the deliberate pin updates.
* Generated files (`docs/agents/gateway.md`, `docs/agents/README.md`):
  rerun `scripts/gen_rule_docs.py` on conflict.

Neither `member_path_route.rs` nor `legacy_callback_route.rs` is touched.
