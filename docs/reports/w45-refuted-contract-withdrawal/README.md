# W45-REFUTED-CONTRACT-WITHDRAWAL: a contract the audit refutes does not serve its callers

Card W45-REFUTED-CONTRACT-WITHDRAWAL, 2026-09-26. Base `12553fb3` ("Record
the w45-gateway-memo-key integration gate"). `main` was merged in twice:
`675e7289` (W45-AUDIT-CHECKPOINTS) at `e359fd3c`, then `edaeade7`
(W45-CONTAINER-ROOT-REACH, and the user's `eac26af7`) at `368062bc`, which is
the measured code. W45-CONTAINER-ROOT-REACH landed first with gateway rule 10f,
so this card's rule is **10g**.
[PREDICTION.md](PREDICTION.md) was frozen before implementation, in `8c094092`.

## Headline

* **Rule** (contract spec §C06.4, gateway constraint 10g). A declared contract
  of a function whose body the audit analyses is **refuted** by that body when
  either of two things holds:
  * a counted-user clause of the function's own row is refuted, whether
    declared or the implicit uncovered-exit obligation of §C04.7;
  * the body's validated W4 summary contradicts a declared custody *duty*
    (constraint 8's mode arm).

  A refuted contract stays in force for its own body, so the defining row keeps
  the refutation. It is withdrawn from every caller in every unit. Each held call
  refuses with the reason key `contract-refuted-by-body` and is **Unknown**.
* **Cross-unit design: two phases.** The first phase analyses every unit and
  folds the rows' `contract_refutations` into a **refutation table**. The table
  is a set with a SHA-256 digest, and it does not depend on `--jobs`. When the
  table is not empty, the second phase analyses every unit with the table in
  `ResourceContracts::refuted_by_body`. All four audit routes do this (`current`,
  `contracts`, configured, corpus arbiter).
* **Corpus: no movement.** `resource-diagnostics.json` and
  `chunk-contracts.json` are byte-identical to the baseline
  (`w45-container-root-reach/integration/artifacts`), because the table is
  empty. The contracts sidecar gains one additive section. There are no
  corrections.
* **Slice at `046accf6`: no status movement.** The table is {OR-7
  `flb_output_task_flush`, OR-1 `flb_output_task_singleplex_enqueue`}. The 10
  caller call rows are held: 3 functions × 2 variants, 4 of them cross-unit
  through the table. All of these callers were already Unknown. Against
  `main`'s slice, unsupported goes from 3,327 to 3,321.
* **Ledger:**
  * `W45-FB-SINGLEPLEX-CUSTODY-DECLARED` is **retired** (discharged).
  * `W45-FB-CROSS-UNIT-USER-PREMISES` is **re-scoped**.
  * Three follow-ups are registered.
* **Lean:** `QCore.RefutedContractWithdrawal`, 15 audited theorems.

## 1. Bases and BEFORE

| Measurement | Result |
|---|---|
| Arbiter BEFORE (base binary, `--jobs 8`) against `w45-fb-backlog-custody/integration/artifacts` | `verdict_bearing_identical: true`. Diagnostics, `chunk-contracts` and `source` are byte-identical. The contracts sidecar differs only by the worktree path and the two `meaning` strings that W45-USER-CONTRACT-PLUGINS rewrote (its integrator's `contracts-raw-diff.log`). ([before-identity.json](artifacts/arbiter/before-identity.json)) |
| Slice BEFORE (fresh private prepare at `046accf6`, base binary) | units 0 / 2 / 13; functions 28 / 2 / 204 / 3. **0** rows differ from the published W45-FB-BACKLOG-CUSTODY slice ([comparison](artifacts/slice/before-vs-published-backlog-custody.json)) |

## 2. Decisions

### 2.1 What a refutation is

The gateway already produces two refutation signals:

* a `refuted` record on the defining function's own counted-user clauses, which
  constraint 10e decides per path;
* a `SummaryConflict` whose derived and declared contracts differ in a custody
  duty (`mode_contradiction`).

A `SummaryConflict` that refines one duty is **not** a refutation. The brief
lists "a SummaryConflict" as an example, so this departure needs a reason. The
only conflict on the pinned corpus is `rd_kafka_mem_free`, which declares
`Released{nullable}` while W4 derives `Released{nonnull}`. Constraint 8 records
that this declaration is the faithful one: `rd_free(NULL)` reaches `free(NULL)`.
The inference picked a narrower reading that the body also satisfies. It did
not find a path that contradicts the declaration. Withdrawing on such a conflict
would label a faithful declaration as refuted.

Container clauses (§C04.8) are never checked against a body, so they can be
unchecked but never refuted (`W45-CONTAINER-CLAUSE-BODY-CHECK`).

**Composition with W45-CONTAINER-ROOT-REACH's summary gate.** That card makes
`observations_match_contract` refuse a body with a *foreign unlink*: a body that
takes custody of an element of a container its caller supplied. Such a body
publishes no W4 summary. So the custody-duty arm has no `SummaryConflict` to
read, and the body neither establishes nor refutes a custody declaration: the
declaration is unchecked and stays applied. The counted-user arm does not read a
W4 summary and is unaffected. The control
`crates/qcore-c-gateway/tests/refuted_contract_foreign_unlink.rs` pins this. A
`drain` declared `resource_borrowed(q)` whose body unlinks and frees the queue's
elements is Safe on its own row, carries no conflict and no refutation, and its
caller's call is not held. That the declaration stays trusted there is the
registered `W45-UNCHECKED-DECLARATIONS-TRUSTED`, not a refutation this rule
misses.

A declaration is **unchecked** in any of these cases, and stays applied as the
declared premise of constraints 4 and 8:

* one of its clauses is undecided;
* its body does not validate;
* it has no analysed body at all.

### 2.2 The held value: Unknown, not Pending

`Pending` is reserved for a named human-resolvable reason. In each such reason,
the human act itself resolves the row, and the tool accepts the result without
new evidence:

* a review edit;
* a re-pin after review;
* a checkable certificate.

A refutation is the tool's own decision under stated premises. Nothing a person
does can be accepted in its place. If the declaration or the body changes, the
tool decides again and may refute again. So the held call is `Unknown`.

Its category is `stale-evidence`, the same as constraint 8's mode withdrawal:
the declaration is evidence about a body it does not describe. That category's
subject is the analyser, so it never supports a provisional `Unsafe`.
`ReasonCategory::classify` gains the rule, and the reason corpus gains the
string.

A pin mismatch together with a refutation joins to `Unknown`. Lean proves this
(`pinned_pending_refuted_unknown_join_unknown`), and the gateway fixture
confirms it (`c_pinned` is Pending, `c_both` is Unknown).

### 2.3 `resource_body_checked` stays opt-in; a separate withdrawal rule

As a default, the C06.3 marker withdraws in two situations:

* in every unit that does not define the body (rule 1), which covers every
  cross-unit application;
* in the defining unit, whenever the body does not *establish* the declaration.

Establishing requires a fully modelled, diagnostic-free body. Measured on the
corpus ([census](artifacts/body-checked-default-census.json)):

* 174 declared names in total;
* 24 have an analysed main-source body;
* only **2** of those 24 are Safe in every defining unit.

So a default marker would withdraw at least 22 of the 24 on no evidence against
them, and would withdraw all cross-unit use of every one. The marker stays the
opt-in strict form: the declaration is in force only where it is established.
The new rule withdraws only on evidence against the declaration.

The residual trust in unchecked declarations of audited bodies is registered as
`W45-UNCHECKED-DECLARATIONS-TRUSTED`, sized at 22. True externals (libc) have no
body, so they can never enter the table. Lean proves this
(`an_external_contract_is_never_in_the_table`).

### 2.4 Cross-unit: option (a), two phases

Option (b) uses the lock's body sites to find the defining unit. That only says
*where* the body lives. Its refutation is still unknown until that unit has been
analysed, so every caller's unit would have to wait on the defining unit's
result. That is option (a) with a partial order instead of a barrier.

Option (a) is simpler, and each unit's work stays independent within a phase:

* **Phase one** is the existing run. Each unit also holds calls to the
  declarations its own bodies refute. This is a local decision in
  `refuted_contract::settle`: after `checked_spec::settle`, the calls are held
  and the unit is settled again over the held stream.
* **Table:** `qcore_audit::refuted_contracts::table` takes the union of every
  row's `contract_refutations`. Each entry records the unit, the kind, the
  clause, the origin and the reason.
* **Phase two** runs only when the table is not empty. Every unit is analysed
  again under `ResourceContracts::refuted_by_body`. With an empty table, phase
  two would repeat phase one exactly (the hold is the identity), so it is not
  run.

Three further properties hold:

* **Name keying.** A unit holds the table's names everywhere except inside the
  refuted function's own body. A static function that shares a refuted name is
  held too; this errs in the refusing direction.
* **Constraint 8's key is kept.** A name the unit's own mode gate withdrew keeps
  `mode-*-contradiction` there.
* **Phase two finds only the table.** In this run, the published rows' own
  refutation digest equals the table digest in both the slice and the corpus.
  The premise that a held call can only make a refutation undecided is stated,
  not proved (§6).

**Checkpoints (W45-AUDIT-CHECKPOINTS, merged from `main` `675e7289` during
this card; the coordinator asked for option (a)).** The wiring is small and
local, and it is done:

1. **Identity.** `ContractSetIdentity` gains `refuted_table_sha256`, the
   `contract_refutation::table_digest` of the table the group was analysed
   under. The digest is defined for the empty table too, so a phase-one group
   is keyed as "analysed under the empty table". The `.qo` schema goes from 1
   to 2, and `checkpoint_cli`'s schema pin is moved deliberately.
2. **Compose check.** Compose already refuses members whose contract sets
   differ ("disagree on the contract set"), so checkpoints sealed under
   different tables never compose. Resume recomputes a group whose digest
   changed.
3. **Two phases.** A checkpointed `current` run first seals and composes every
   group under the empty table. When the composed rows record a refutation, it
   seals and composes every group again under that table
   (`current_route.rs`, `checkpoint_run::run` returns the composed
   refutations).
4. **Section.** The run records the applied table in its assumptions before
   any unit, under `refuted_contracts.applied`, so every checkpoint's `global`
   section carries it. The composer renders the `refuted_contracts` section and
   tally from it and from the rows, one unit at a time
   (`refuted_contracts::RowTally`). The whole run's `finish` replaces the same
   key, so the two sidecars are byte-identical.

`checkpoint_cli`'s whole-run identity test still holds at schema 2: outputs and
console match the whole run at `--jobs 1` and `--jobs 4`. The slice measured
through checkpoints is §5.2. Lean proves that a grouped two-phase run equals
the whole run (`two_phases_over_groups_equal_the_whole_run`).

## 3. Mechanism

* `crates/qcore-schemas/src/contract_refutation.rs` (new) defines:
  * `RefutationKind` and `ContractRefutation`;
  * `RefutationTable`, with `insert_refutation`, `union` and `table_digest`.

  `ResourceContracts` gains the field `refuted_by_body`. It is omitted from
  serialization when empty, so every existing environment's bytes are
  unchanged. `set_refuted_by_body` keeps only names that are in force.
* `crates/qcore-c-gateway/src/legacy_refuted_contract.rs` (new) provides:
  * `refutations`, which reads a row's own refutations;
  * `hold_call_sites`, which holds call sites and exempts the function's own
    body;
  * `settle`, which is the local hold plus the table hold;
  * the hold description, `held_call` and `CONTRACT_REFUTED_BY_BODY`.

  Hooks: one in `legacy_c.rs` (the table read before the ABI gate rebuilds the
  environment, `settle`, and `summary.refutations`), two description sites, and
  the generic-selection name map. `legacy_counted_users.rs` gains
  `refuted_own_clauses`, which shares `check`'s body through
  `evaluate_function`. `legacy_declared_modes.rs` gains `withdrawn_names`.
* `crates/qcore-audit/src/refuted_contracts.rs` (new) provides the table,
  `run_two_phases`, `with_table`, the `refuted_contracts` sidecar section and
  the console tally. The routes hook in as follows:
  * `current_route.rs` / `current.rs`, `no_build.rs` and `lib.rs` (configured)
    run through `run_two_phases`;
  * the arbiter (`compiler/examples/support/cycle19_compatibility.rs`) runs its
    unit steps again in phase two and reuses each library's prelude context;
  * `report.rs` writes `contract_refutations` on a row only when it is present;
  * `revision_compare.rs` names the new reason key.
* In every route the outcomes are collected, then printed and folded in task
  order. The first error in task order stops the fold, as before. A stopped
  phase one publishes as it stopped.

## 4. Controls

The controls are in `crates/qcore-c-gateway/tests/refuted_contract_withdrawal.rs`
(11 tests, held-out `w_*` family) and in `qcore_audit::refuted_contracts` unit
tests (4).

| Control | Expected | Result |
|---|---|---|
| The defining unit's table | exactly `w_bad` (counted-user) and `w_keep` (custody duty) | as expected |
| A refuted body under the table | keeps its own refutation, provisional Unsafe | as expected |
| A refinement conflict (`w_rel`) | not a refutation | no refutation recorded |
| **Accepting:** an established contract, cross-unit under the table and in its unit | still applied: `c_peek` Safe, `c_ok` not held | as expected |
| **The gap:** a cross-unit caller before the table | `c_keep` **Safe** over a borrowed declaration whose body frees | Safe (the unsound certificate) |
| **Refusing:** the same callers under the table | Unknown, held `contract-refuted-by-body` | as expected |
| **Refusing, in unit, with no table** | `c_bad` held; `c_keep` keeps constraint 8's `mode-parameter-contradiction` | as expected |
| Held value | Unknown, not a Pending held call | as expected |
| **Externals:** `c_extern` (no body) | unaffected, byte-identical diagnostics; a table name not in force is dropped | as expected |
| **Pinned with refuted** | pin alone Pending; pin with refutation Unknown | as expected |
| The table in the environment | serialized only when non-empty; digest differs from the empty table's | as expected |
| Table set semantics, groups, phase-two triggering, stopped phase one (audit) | union over units and groups equals whole; phase two only on a refutation | as expected |

The gap control is the concrete unsoundness this card closes. `w_keep` is
declared `resource_borrowed`, but its body frees `p`. A caller in another unit
that frees the object again was certified **Safe**. It is now Unknown.

## 5. Measurements

### 5.1 Corpus arbiter

`run_audit.py … --jobs 8` at `368062bc`, against the new baseline `w45-container-root-reach/integration/artifacts` (the earlier tips `a3677482` and `e359fd3c` gave the same artifacts against the backlog-custody baseline):
[after-identity.json](artifacts/arbiter/after-identity.json),
[contracts-delta.json](artifacts/arbiter/contracts-delta.json) (against BEFORE),
[contracts-delta-vs-baseline.json](artifacts/arbiter/contracts-delta-vs-baseline.json),
[refuted-contracts-section.json](artifacts/arbiter/refuted-contracts-section.json).

* `verdict_bearing_identical: true`. Diagnostics, `chunk-contracts` and `source`
  are byte-identical to the baseline. `source-effects` and `monkey-source.json`
  are byte-identical to this worktree's BEFORE run (`source-effects` differs from
  the archived baseline in checkout-path digests only, as in every worktree run).
* Against BEFORE, and against the new baseline after the root alias, the
  contracts sidecar differs by exactly `/refuted_contracts`:
  1 phase, 0 declarations, table digest `44136fa3…` (the empty table). The
  audit log gains the tally line.
* Functions 398 / 9 / 4,427; units 5 / 8 / 168 / 4; leak 34; unknown_alias
  3,571; unsupported 41,314; Pending 0: the new baseline's figures, all
  unchanged.
* **No corrections.** No corpus body refutes its declaration. No row has
  `user_count_clauses`, and the one conflict is a refinement.
* **Baseline:** no new baseline is archived, because nothing verdict-bearing
  moved. `w45-container-root-reach/integration/artifacts` stays the corpus
  baseline, plus the additive sidecar section.

### 5.2 Fluent Bit system slice at `046accf6`

The command is the backlog card's §4 command (`scripts/run-current.sh
attempt-1 RUN BINARY --selection contract/profiles/fluent-bit/system-selection.json
--jobs 8`) with a fresh private prepare. Artifacts:
[after report](artifacts/slice/after-046accf6.json.gz),
[moves](artifacts/slice/after-vs-before.json),
[section](artifacts/slice/refuted-contracts-section.json).

BEFORE is `main`'s own slice, W45-CONTAINER-ROOT-REACH's archived AFTER
(`docs/reports/w45-container-root-reach/artifacts/slice/after.json.gz`); the
card-base comparison (3,338 → 3,332, before that card's foreign unlink) is
[after-vs-card-base.json](artifacts/slice/after-vs-card-base.json).

| | BEFORE (`main` `edaeade7`) | AFTER (`368062bc`) |
|---|---:|---:|
| Units Safe / Unsafe / Unknown | 0 / 2 / 13 | 0 / 2 / 13 |
| Functions Safe / Unsafe / Unknown / not projected | 28 / 2 / 204 / 3 | 28 / 2 / 204 / 3 |
| unsupported / unknown_alias / counted_user | 3,327 / 713 / 4 | **3,321** / 713 / 4 |

**Table** (2 phases, digest `e727ccc6…`; the published rows' own refutations
have the same digest):

| Declaration | Refuted by (its own row) | Held |
|---|---|---|
| OR-7 `flb_output_task_flush` | `resource_user_exit(task, 0, added)`, `src/flb_output.c:541` | 6 call rows in 3 functions |
| OR-1 `flb_output_task_singleplex_enqueue` | `resource_user_exit(task, -1, restored)`, `src/flb_output.c:376` | 4 call rows in 2 functions |

**Attribution.** Three distinct functions change leaves; none changes status,
and each figure below counts both variants:

* **`flb_engine_dispatch_retry`** (`src/flb_engine_dispatch.c`, cross-unit
  through the table). At `:179` and `:186`, the hold replaces two things:
  * the unattributed counted-user operation leaves (the declaration is no
    longer applied);
  * the `indirect memory access` refusal of `retry->o_ins->…` at `:179`, which
    is the same statement refused earlier (as for a pin or review hold).

  Leaves: −6, +4.
* **`tasks_start`** (`src/flb_engine_dispatch.c`, cross-unit). At `:319` and
  `:327`, the same exchange happens. Leaves: −8, +4.
* **`flb_output_task_queue_flush_one`** (`src/flb_output.c`, in unit). At
  `:333`, the unattributed operation leaf becomes the hold leaf. Leaves: −2, +2.

The net change is unsupported −6, identical before and after the merge of
W45-CONTAINER-ROOT-REACH: the same 10 held rows, the same leaves exchanged. The two OR functions keep their refutations
and stay provisional Unsafe. OR-1's undecided success clause keeps its blocker
text.

**`--jobs` independence:** at `--jobs 1`, `resource-diagnostics.json` and its
contracts sidecar are byte-identical to the `--jobs 8` run
([jobs-independence.json](artifacts/slice/jobs-independence.json)).

**Checkpointed:** the same run with `--checkpoint-dir` seals the one group
(`fluent-bit-system`) twice, first under the empty table and then under the
two-entry table, and composes a report and sidecar byte-identical to the whole
run's ([checkpointed.json](artifacts/slice/checkpointed.json),
[stderr](artifacts/slice/checkpointed.stderr.log)). Its console prints the
first phase's composition too, which the whole run does not. Both `--jobs 1`
and checkpointed runs were repeated at `368062bc`.

### 5.3 Criterion 3, the concrete case

After this card, **no caller of either OR declaration applies it**. Every
textual call site in the slice is held:

* in `src/flb_engine_dispatch.c`: `:179`, `:186`, `:319` and `:327`;
* in `src/flb_output.c`: `:333`.

Each is held in both variants. No Safe rested on OR-7 or OR-1 before this card,
because the callers were already Unknown through the counted-user pass.
Structurally, none can rest on them now: the Lean theorem
`never_rests_on_a_refuted_specification` and the gateway hold both guarantee
it.

## 6. Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. Corpus: empty table, byte-identical diagnostics, one additive sidecar section, no correction | **as predicted** |
| 2. Slice table: exactly OR-7 and OR-1 | **as predicted** |
| 3. 10 held call rows at `:179/:186/:319/:327/:333` | **as predicted** |
| 4. No status movement; leaf exchanges only in the three callers | **as predicted**. I predicted possible `unknown_alias` additions after `:179`; there were none. The exchanged leaves also include 3 `indirect memory access` refusals at held statements, per variant. I did not predict these. |
| 5. No leak or violation leaf added | **as predicted** |
| 6. Retire SINGLEPLEX, re-scope CROSS-UNIT | **as predicted** |

**Classification.** No leak, violation or `counted_user` leaf was added or
removed in either measurement, so nothing needed classifying.

## 7. Lean

`QCore.RefutedContractWithdrawal`
([module](../../../proof/QCore/RefutedContractWithdrawal.lean),
[audit](../../../proof/AuditRefutedContractWithdrawal.lean), 15 printed
theorems). It is built on `QCore.VerdictSemigroup`.

* **The held value:**
  * `a_held_call_is_unknown`;
  * `a_held_call_is_never_pending`;
  * `pinned_pending_refuted_unknown_join_unknown`.
* **Never resting on a refuted specification:**
  * `never_rests_on_a_refuted_specification`: with a held call, the rank is at
    least `Unknown`;
  * `the_verdict_ignores_what_a_refuted_contract_claims`;
  * `an_unrefuted_contract_still_serves`;
  * `an_external_contract_is_never_in_the_table`.
* **Monotone withdrawal:**
  * `item_value_under_more`;
  * `withdrawal_never_certifies`: a caller that is `Safe` or `Pending` under a
    larger refuted set has the same verdict under a smaller one.
* **Two phases:**
  * `the_union_of_group_tables_is_the_whole_table`;
  * `two_phases_over_groups_equal_the_whole_run`;
  * `mem_table_iff`;
  * `the_table_is_independent_of_unit_order`, whose premise is that the second
    pass reads its table only by membership.
* **Non-vacuity:**
  * `a_refuted_caller_moves_from_safe_to_unknown`;
  * `an_independent_violation_stays`.

"Monotone" means the verdict never moves *towards `Safe`*. A caller's
provisional `Unsafe` that rests only on the refuted call's contribution can
become `Unknown`. That is the direction the brief's criterion 6 requires: a
verdict never rests on a refuted specification. It is a model of the decision,
not an extraction of the Rust (trust boundary 28). The Rust side is pinned by
§4's controls.

## 8. Ledger

* **Retired:** `W45-FB-SINGLEPLEX-CUSTODY-DECLARED`. Its second discharge
  condition, "a check-only declaration form that callers never apply", is met
  for both declarations. Each is refuted by its own body, so every caller call
  site is held: 10 held rows, and 0 rows apply either declaration.
* **Re-scoped:** `W45-FB-CROSS-UNIT-USER-PREMISES`. Both premises are unchecked,
  not refuted, and stay applied:
  * `flb_task_retry_destroy` is undecided in `src/flb_task.c`;
  * `flb_output_thread_pool_flush` has no analysed body.

  Their only consumers are the two refutations. No Safe rests on either.
* **Registered** ([followups.json](followups.json)): `named_followups` goes from
  456 (on `main` `edaeade7`) to 458. The checkpoint key first drafted as a
  follow-up was implemented instead (§2.4).
  * `W45-CONTAINER-CLAUSE-BODY-CHECK`: 2 declarations.
  * `W45-UNCHECKED-DECLARATIONS-TRUSTED`: 22 corpus declarations, a lower bound.
  * `W45-REFUTED-HOLD-UNIT-REASON`: 0 units today.
* **Noted, not in the ledger:** the report-only residual
  `LOADER-ABI-MODES-CROSS-UNIT` (remaining-loader-abi-modes) is now answered for
  the *refuting* direction. A duty contradiction found in the defining unit
  holds callers in every unit. A declaration whose body no unit analyses is
  still never compared.
* The corpus row is unchanged. `build_matrix.py .` regenerated the derived
  counts: `lean_audits` 206, follow-ups 458. The gate figures are carried and
  labelled as carried, for the integrator's `--gate`.

## 9. Gate

Run on `5bb3664b` (the tree merged with `main` `edaeade7`) with `scripts/gate.sh`. Every exit goes to its own file, with
`CARGO_INCREMENTAL=0`. Logs are worktree-private under
`private/w45-refuted-contract-withdrawal/gate-5bb3664b/`
([gate.json](gate.json)). The provider packages are built outside the
repository and then removed.

| Step | Result |
|---|---|
| submodule; fmt; clippy `-D warnings`; both reevaluation probes | 0 each |
| Rust suite (`scripts/run_test_binaries.py`) | 0: 312 units, 322 summaries; **5,953 passed**, 0 failed, 11 ignored |
| `cargo clean --profile dev` | 0 |
| Lean audits | 0: **206** passed |
| Root Python tests; `build_docs_index.py --check` | 0: **358** tests; 0 |
| Provider gate: `contracts.py check`, package build and verify, `providers/tests`, provider `cargo test`, fmt, clippy | 0 each: 15 Python tests; 129 Rust passed, 0 failed, 1 ignored |

**Discarded attempts**, recorded in `gate.json`:

* **`e747c546` (before the merge).** `provider-tests` exited 101. Two
  `configured_source_contract` process-provider tests timed out (`QP006`) while
  concurrent cards loaded the host. They are load-sensitive (constraint 16).
  They were not weakened, and they pass in the final gate.
* **`0d4f42f1` (after the merge).** The suite exited 101 with 1 failure.
  `checkpoint::tests::a_future_schema_version_is_refused` pinned the "future"
  schema as 2, which this card makes current. The pin moved to 3.
* **`c6de5e51`** was green (5,936 passed, 205 Lean audits) before `main`
  `edaeade7` was merged; this gate on the merged tree supersedes it.

**Pins moved deliberately:**

* the `.qo` schema, from 1 to 2 (`checkpoint_cli` and the checkpoint unit test);
* the matrix's derived `lean_audits` and follow-up counts.

## 10. Boundaries

* **Premises stated, not proved:**
  * the second pass reads the table only by membership;
  * a held call can only make a refutation undecided, never create one. In this
    run, the published refutations equal the table in both measurements;
  * the refutations the rows record are the ones the gateway finds (trust
    boundary 28).
* A refutation is relative to the unit's other declarations (the
  `LOADER-ABI-MODES-RELATIVE` boundary). For example, OR-7's refutation rests on
  `flb_output_thread_pool_flush`'s declared neutrality (the re-scoped entry).
* The OR refutations are **provisional Unsafe** candidates and not upstream
  defect claims. Nothing was reported upstream.
* Unknowns stay in the denominator. No function entered Safe.

## Reproduce

```sh
cargo build --release -p qcore-audit --bin quirin-audit -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
docs/reports/w45-refuted-contract-withdrawal/scripts/prepare-attempt.sh attempt-N BINARY
docs/reports/w45-refuted-contract-withdrawal/scripts/run-current.sh attempt-N RUN BINARY \
  --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
python3 docs/reports/w45-refuted-contract-withdrawal/scripts/slice_tally.py AFTER --before BEFORE
python3 docs/reports/w45-refuted-contract-withdrawal/scripts/or_rows.py REPORT
python3 docs/reports/w43-return-expression/scripts/run_audit.py FRESH_DIR COMMIT --jobs 8
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py FRESH_DIR \
  docs/reports/w45-container-root-reach/integration/artifacts --root-alias WORKTREE=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
python3 docs/reports/w45-refuted-contract-withdrawal/scripts/sidecar_delta.py BEFORE_SIDECAR AFTER_SIDECAR
python3 docs/reports/w45-refuted-contract-withdrawal/scripts/body_checked_census.py FRESH_DIR OUT.json
docs/reports/w45-refuted-contract-withdrawal/scripts/archive.sh
python3 docs/reports/w45-refuted-contract-withdrawal/scripts/update_ledger.py   # applied once
docs/reports/w45-refuted-contract-withdrawal/scripts/gate.sh LOG_DIR
```
