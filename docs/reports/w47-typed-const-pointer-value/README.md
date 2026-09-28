# W47-TYPED-CONST-POINTER-VALUE

Discharges the follow-up of the same name, which W47-TYPED-ROOT-DECLARATION-READER
registered after measuring that ten of its twelve forecast misses refuse on
the value's type with every premise granted. Base `66be515f` (local `main` =
`origin/main`, "Merge PROVIDER-PY-DUAL-STEM-FIXTURE-STALE onto the round-1
integration"). The prediction, the census and the controls were frozen and
run at the base before any analyzer change ([PREDICTION.md](PREDICTION.md),
commit `b2cd2226`). Gateway rule [10m](../../agents/gateway.md#constraint-10m);
Lean model `QCore.TypedConstPointerValue`.

## Mechanism (rule 10m)

`typed_const_pointer_value.rs` holds both steps. Three thin hooks call it:
the value check in `Reader::path`, the value check in
`typed_access_positions::Reader::value`, and the two pointer-destination
sources in `Reader::statement`. `is_pointer`'s text test is split out as
`is_pointer_type`, with its behavior unchanged.

- **Step 1, a `T *const` value.** A pointer member read through a `const`
  pointee, or an element of a `T *const[]` member, is a `T *const` lvalue.
  The value read from it has the unqualified type (C11 6.3.2.1p2), which is
  also the type Clang gives its `LValueToRValue` conversion. The value check
  now admits a type whose canonical Clang spelling (`desugaredQualType`, else
  `qualType`) splits at its last declarator `*` into two parts:
  - an unqualified type that the unchanged positive pointer test admits;
  - a qualifier list that is exactly `const`.

  Anything else after the last `*` refuses. That covers `volatile`,
  `restrict`, any combination, `[N]` and `)(…)`. `_Atomic(T *)` still fails
  the pointer test.

  Only the value's own qualifier is erased. A middle hop is already read
  through Clang's unqualified conversion. These are unchanged: every per-hop
  obligation, the mixed-path untracked-root premise, and rule 5b's
  never-bound, not-`may_bind` premise on returned roots.
- **Step 2, a `void *` bit-cast.** At the top of a pointer-destination
  statement (`NAME = VALUE`, or `T *NAME = VALUE` with one declarator), the
  value may be an `ImplicitCastExpr <BitCast>`. It is admitted when all of
  these hold:
  - the cast's only child has the type exactly `void *`;
  - the cast's type and the destination's declared type are object pointers,
    meaning the pointer test admits them and the pointee is not `void`;
  - `strip` reads the value under the cast.

  The statement is then read exactly as the uncast statement: the destination
  is P2's borrowed, target-free alias, and the obligations are the inner
  path's. These keep the refusal: an explicit cast, any other conversion, a
  function or `void` pointer target, a cast from another object pointer, and a
  cast inside a path, which `path` refuses hop by hop. Returns, conditions and
  actuals already strip an implicit `BitCast` (P3); only statements change.

The arms are the constants `CONST_VALUE_ARM` and `VOID_BIT_CAST_ARM`. Step 1
was committed and measured with the second off (`fecfb1bd`), and step 2
turned it on (`5138ed38`).

## Measurement

Arbiter: `QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl scripts/run_pinned_audit.py
OUT <commit> --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19
--jobs 5` and `w46-contract-gap-expansion/scripts/run_system.py OUT --jobs 5`
at `046accf6`. Each ran one at a time, exit 0. Private runs are under
`private/cp/` of this worktree.

**BEFORE** (binary `46b58b57…`) reproduces the round-1 baseline archive
`w47-callee-closure-r2/integration/artifacts/`. `scripts/unpack.py` checks every
manifest raw SHA-256, and `scripts/identity.py` compares file by file, with the
worktree root aliased to `w47-int-c`.
- Corpus: the diagnostics, `monkey-source.json` and the chunk contracts are
  byte-identical, and so is the sorted parity stream. The contracts sidecar is
  identical once the root is aliased. The source-effects sidecar differs only
  in cwd-bound build, configuration and task digests and the ids derived from
  them.
- System: 0 changed rows. The raw file differs only in the path-bound
  `resource_contract_sources` and `inventory_sha256` digests.

The comparisons are in [before-identity/](artifacts/before-identity/). The
comparison used is the verdict-and-leaf identity of `scripts/compare.py` plus
raw SHA-256s; `baseline_identity.py` cannot read these manifests, as the brief
notes.

| Measure | BEFORE | Step 1 (`fecfb1bd`) | Step 2 (`5138ed38`) |
|---|---:|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 510 / 10 / 4,314 | **520** / 10 / **4,304** | 520 / 10 / 4,304 |
| Corpus units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | same | same |
| Leak / double release / use after release | 36 / 0 / 0 | same | same |
| unknown_alias / unsupported | 7,443 / 36,209 | 7,485 / 36,112 | **7,585 / 36,048** |
| System configuration rows (Safe / Unsafe / Unknown) | 63 / 4 / 381 | same | same |
| System unique functions | 33 / 2 / 199 | same | same |
| System unknown_alias / unsupported | 985 / 2,908 | 993 / 2,892 | **1,007 / 2,884** |

The AFTER archive (step 2, binary `ced057da…`) is the new baseline:
[artifacts/after/corpus/](artifacts/after/corpus/) and
[artifacts/after/system/](artifacts/after/system/). Comparisons per step and
cumulative are in [artifacts/after/comparison/](artifacts/after/comparison/).

### Step 1: ten Unknown → Safe, every leaf attributed

74 corpus function rows change. 66 of them change only at a typed row that
step 1 newly admits: each removed leaf is a COND, RET or IND refusal on such a
row, and each added leaf is an `unknown_alias` (`compare.py` `direct`). The
leaves are:
- removed: IND 29, COND 53, RET 9, and 7 uncontracted-call leaves;
- `unknown_alias`: 50 added, 8 removed;
- RET: 1 added.

The ten moves:

| Function | Reason |
|---|---|
| `rd_kafka_Node_host`, `rd_kafka_Node_rack`, `rd_kafka_mock_cluster_bootstraps`, `rd_kafka_mock_cluster_handle`, `rd_kafka_opaque`, `rd_kafka_handle_mock_cluster`, `rd_kafka_messages_get`, `rd_kafka_share_partition_offsets_list_get` | the eight `T *const` returns the follow-up names; each one leaf, the return refusal, now a typed read with a `NoOp` exit from an unwritten formal |
| `rd_kafka_DeleteAcls_result_response_matching_acls` | `return (const T **) result_response->matching_acls.rl_elems`. W44-LEAK-PIN-AUDIT's cast-return rule decides it as the uncast return, and that is a `void **const` value through a `const` pointee, now read. The return was the body's only refusal. The `unknown_alias:matching_acls_cntp` at the out-parameter store `*matching_acls_cntp = …` goes with it, although that store's row is unchanged. We attribute this by reading, not by trace: the body's W4 summary now validates, and in the reported run it makes the formal an out slot. |
| `rd_kafka_topic_partition_cmp_opaque` | a summary cascade: its one leaf was "call to uncontracted function `rd_kafka_topic_partition_cmp`"; the callee's actuals `a->topic`, `b->topic` (`char *const`) are now read, so its W4 summary (`Owned`, `Owned`, result none) validates; see [the two cascades](#the-two-cascades) |

Eight rows fall outside `direct`: the two cascade moves above and six rows
that do not move.
- `rd_kafka_topic_partition_cmp` stays Unknown (next section).
- `rd_slice_dump` exchanges `unknown_alias:slice` for `unknown_alias:seg` at
  the newly admitted `seg = slice->seg`.
- `rd_list_dump` loses the IND leaf of a two-line statement whose actual
  (`rl->rl_elems`, next line) is now admitted.
- `do_unittest_read_verify0` and `do_test_rd_uvarint_enc_i64` lose six
  "uncontracted `rd_slice_offset`" leaves, and `rd_slice_reader0` loses four
  `unknown_alias:slice` leaves: the header body `rd_slice_offset` (census rows
  `! slice->seg`) now validates a summary that its callers' calls read
  (attributed by reading).

**System:** 6 configuration rows change, and none moves. The changes are in
`flb_input_chunk_write_header_v2` (4 COND and 2 IND removed per
configuration) and in `input_chunk_collect_output_references` and
`task_collect_output_references` (IND removed, `unknown_alias` on `route` and
`label` added). All are at census rows.

### Step 2: no status move

55 corpus rows change, all `direct`: 59 IND and 5 COND leaves are removed, and
100 `unknown_alias` leaves are added. The two named getters, whose
`parpriv = rktpar->_private` is now P2's borrowed read, stay Unknown. Each has
exactly one leaf, `unknown_alias:parpriv`, because the null test on `parpriv`
keeps rule 10b's identity refusal. `mk_iov_print` exchanges its IND leaf for
`unknown_alias:c`.

**System:** 6 configuration rows change (`flb_input_chunk_protect`,
`flb_input_chunk_release_space_compound`, `flb_sched_event_handler`): 8 IND
leaves are removed and 14 `unknown_alias` leaves are added. There is no
status move.

### Corrections, leaks and violations

No Safe → non-Safe move, so there is no correction. No unit moves. No leak,
double release, use after release or provisional Unsafe is added in either
step, so there is nothing to classify.

**Interlock** (`scripts/interlock.py`,
[interlock.json](artifacts/after/comparison/interlock.json)): the ten new Safe
functions' admitted-row roots are unwritten formals. None of them is in rule
10l's disputed class (the AST binds, the lexical closure calls it never bound),
and none is in the nested-assignment subset. All ten pass.

## The two cascades

Both cascades come from W4 summaries this step made derivable. In `cmp` and
`cmp_opaque` the cascade crosses a boundary that the card measured and
registered rather than changed. W4 validates `rd_kafka_topic_partition_cmp`'s
summary in a run where the environment does not yet hold that summary. So
`protected_resource_formals` protects neither `_a` nor `_b`, even though the
run seeds them as entry-bound `Owned` resources. In that run the copies
`a = _a` and `b = _b` count as never-bound roots, and the typed reads rooted
at them are admitted.

The reported run has the summary in force. It protects `_a` and `_b`, binds
`a` and `b`, and refuses. The function row is therefore Unknown: a RET leaf
(the return family now declines on a root the body may bind) replaces the
`unknown_alias:a` and `unknown_alias:b` leaves it had before, beside its
unchanged IND and `_a`/`_b` leaves. Meanwhile `cmp_opaque` certifies on the
summary.

A fixture reproduces the shape and pins it
(`a_comparator_summary_validates_where_its_own_row_refuses`). The summary is
custody-correct for a body that only reads. The validation run still checks
every base obligation; what it does not hold is rule 5b's premise that a
never-bound root reaches no tracked place. That premise is false there for a
copy of an entry-seeded formal. The corpus extent is not measured. This is
registered as `W47-SUMMARY-VALIDATION-PROTECTED-FORMALS`. `cmp_opaque`'s Safe
is reported as resting on that summary.

## Prediction against outcome

1. Eight named Safe in step 1: **holds**.
2. `rd_kafka_message_leader_epoch` Safe: **missed**. The operand
   `! rkmessage->rkt` is now admitted, but the desugared condition still
   declines at a later operand that the typed reader is not asked about. Its
   parity stream holds only the first operand's row, as the prediction warned.
   It stays Unknown with its one COND leaf.
3. The two array decays stay Unknown under rule 5j: **holds**.
4. Leaf exchanges only at census rows: **holds for 66 of 74**. **Missed**: the
   two cascades and the three summary effects above, which no census row
   covers. These are the header `rd_slice_offset` and the comparator's own
   summary. Prediction 5 had allowed for header-body cascades but forecast
   none.
5. to 6. No correction, leak or violation: **holds**.
7. to 9. Step 2 moves nothing; the leader-epoch getters and `mk_iov_print`
   exchange for `unknown_alias`: **holds exactly** (55 rows against the 57
   forecast, an upper bound).
10. Interlock: **holds** for all ten.
11. System: **holds**. There are no moves; the rows the census named change,
    6 per step.
12. The two redeclared-typedef controls now certify: **holds**. They are
    renamed as the next section records.

Of the ten functions the follow-up names, **eight** become Safe. The two
leader-epoch getters now wait only on rule 10b's identity refusal of a
null-tested borrowed read, the `W47-ALIAS-FLOW-RESIDUALS` class; they no
longer wait on the typed reader.

## Tests and controls

`crates/qcore-c-gateway/tests/typed_const_pointer_value.rs` has 7 tests. The
negative controls were written and run at the base first
([controls-at-base.txt](artifacts/cause/controls-at-base.txt)).
- A `T *const` value certifies as a return, a condition operand, a `Borrowed`
  actual, a declaration and an assignment. At the base each was Unknown.
- `volatile`, `const volatile`, `restrict` and `_Atomic` values keep the
  return refusal.
- A `T *const *` middle hop certifies under a formal root and refuses under a
  bound root, both as at the base.
- A `const` value from a bound root keeps the return refusal (plus the
  body's own leak), and so does a root bound by a call in a nested condition
  assignment.
- The `void *` bit-cast is a borrowed read: an `unknown_alias` exchange after
  a null test, Safe otherwise.
- These keep their base refusal: an explicit cast, a function-pointer
  destination, an integer cast, a cast mid-path in a statement and in a
  condition, and a bit-cast from another object pointer in a statement and in
  a condition assignment.
- The comparator cascade boundary is pinned.

The module has 5 unit tests for the qualifier split, the value check and every
conversion.

Two controls were **renamed** because their cause is exactly what step 1
admits:
- P3's `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal` is now
  `a_redeclared_typedef_formal_reads_its_const_pointer_return` and asserts
  `Safe`;
- rule 10l's `the_redeclared_typedef_control_refuses_for_its_const_pointee`
  is now `the_redeclared_typedef_control_certifies_with_its_const_pointee`.

The pointer-return refusals they guarded stay pinned by P3's
`a_return_through_a_released_or_tracked_root_is_not_safe` and by this file.

## Lean

`proof/QCore/TypedConstPointerValue.lean` is registered in
`proof/AuditTypedConstPointerValue.lean` (37 theorems; audit total 218). It
imports `QCore.TypedAccessPathPositions` and proves the following.

**Step 1:**
- The rule only adds, and an unqualified value is decided as before.
- An admitted value erases to an old admission (`an_admitted_value_erases_to_an_old_admission`).
- Erasure preserves the shape, and with it P2's statement lowering, the
  borrowed read, P3's return lowering and P3's position decision
  (`erasure_preserves_the_*`). The value is the same pointer, so every P3
  theorem carries over.
- A `const` pointer return hands nothing back, and one from a tracked root
  refuses (rule 5b, unchanged).
- A `const` read binds no custody.
- `volatile`, `restrict`, `_Atomic` and any unlisted list refuse in every
  position.

**Step 2:**
- The bit-cast arm lowers to the inner read's borrowed read, so every
  obligation is preserved (`the_bit_cast_arm_preserves_every_obligation`).
- It binds no custody.
- These refuse: an explicit cast, any other implicit conversion, a function or
  `void` pointer target, an object-pointer source, a non-pointer destination,
  a non-path value and a cast in the middle of a path.

**Non-vacuity:** the `char *const` witness is admitted where the old reader
refused it, and `const volatile` is refused. The leader-epoch bit-cast is
admitted, and the same value under an explicit cast is refused.

The Rust-to-Lean correspondence and Clang's printed types are review premises
(constraint 28).

## Gate

[gate.json](gate.json), with exits in [gate/](gate/). The Rust, Lean and
census steps ran on the code at `0e6b69cf`; later commits change docs, the
gate record and the regenerated matrix and index only. Each step's exit was
captured to a file, never through a pipe.

- Submodule update: exit 0.
- `run_test_binaries.py`: exit 0. **342 units, 355 summaries; 6,152 passed,
  0 failed, 11 ignored** (`private/test-binaries/20260928T033233Z-1312543/summary.json`).
- `cargo clean --profile dev`: exit 0.
- fmt and clippy: exit 0 each.
- Both reevaluation examples: exit 0.
- `check_lean_audits.py`: exit 0, **218** audits.
- Clang 21.1.8 recorded.
- Census-feature gateway clippy: exit 0. Its library tests: exit 0, 628
  passed.
- A final `cargo clean --profile dev`: exit 0.

After the matrix regeneration (`build_matrix.py . --gate gate.json`) and the
index regeneration:
- `unittest discover -s scripts/tests`: exit 0, **365** tests.
- `build_docs_index.py --check`: exit 0.

No `providers/` path is touched, so the provider gate does not apply;
`provider_tests` is carried in gate.json.

## Follow-ups

- **Retired:** `W47-TYPED-CONST-POINTER-VALUE`. Both mechanisms are
  implemented and measured, with audited Lean and negative controls. The
  re-pin names which of the ten became Safe (eight) and why the other two did
  not.
- **Registered** ([followups.json](followups.json)):
  `W47-SUMMARY-VALIDATION-PROTECTED-FORMALS`. One measured instance; the
  corpus extent is not measured.
- **Boundaries, not tracked here:**
  - A `T *const` destination or plain name is not rule 10m's; the rule reads
    values, not declarations. The census did not measure it.
  - `rd_kafka_message_leader_epoch`'s later operand belongs to
    `W47-ACCESS-PATH-POSITION-RESIDUALS`.
  - The leader-epoch aliases belong to `W47-ALIAS-FLOW-RESIDUALS`.

## Files likely to conflict

These are thin hooks that the other round-2 cards also touch:
- `crates/qcore-c-gateway/src/typed_access_path.rs`: one import, the value
  check in `path`, two sources in `statement`, and the `is_pointer_type`
  split;
- `crates/qcore-c-gateway/src/typed_access_positions.rs`: one import and one
  line;
- `crates/qcore-c-gateway/src/lib.rs`: one `mod`.

The two renamed tests are in
`crates/qcore-c-gateway/tests/{access_path_positions.rs,typed_root_declarations.rs}`.

These files are shared records that every round-2 card appends to:
- `proof/QCore.lean`;
- `docs/agents/{gateway.md,README.md}`;
- `docs/STATUS.md`;
- `docs/plans/unsupported-coverage/README.md`;
- `docs/development/increments/wave-47.md`;
- `docs/reviews/r0_3/acceptance_matrix.json`;
- `docs/reports/INDEX.md`.
