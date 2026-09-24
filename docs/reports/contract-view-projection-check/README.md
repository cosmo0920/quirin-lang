# CONTRACT-VIEW-PROJECTION-CHECK: checked relations between the views of one function

Card CONTRACT-VIEW-PROJECTION-CHECK, 2026-09-25, on base `f8b41209` (main after
the CONTRACT-INHERIT-ADOPTION matrix repair).

The user asked for duplicated contract definitions to be removed, then chose
**checked projections**: delete genuine duplicates, keep one hand-written view
per consuming checker, and make every remaining multi-view declaration a
checked relation so the views cannot drift apart. No language change.

## Headline

* **Census, re-measured.** At base, **11** functions were declared in more than
  one of the corpus's 108 modules (the integrator's reading had 10: it missed
  `cb_wait_2_timeout`, and `cio_chunk_close` is a triple, not a pair). One,
  `source_identity`, was token-identical. After the card: **10** functions,
  **12** module pairs, every pair a registered, checked relation.
* **Removed.** `monkey/timeout_teardown.qcore`'s `source_identity`, a textual
  copy of `monkey/wait_source_policy.qcore`'s. Its consumer,
  `qcore_verification::timeout_teardown`, cannot include (a typed-policy
  consumer, and `qcore-provider-*` is off the load path), so it was
  **extended**: it evaluates the owner's declaration, refuses a redeclaration,
  and binds the owner's SHA-256 (`source_identity_digest`) into its projection
  and its native evidence.
* **Repaired.** `chunkio/chunk_destructor.qcore` declared `cio_chunk_close`'s
  second parameter `bool`; C says `int delete`, and the other two declarations
  say `i32`. The agreement check refused it; it is now `i32`.
* **Checked.** 5 projections decided by entailment, 1 renamed equivalence
  decided on both real evaluators, 5 source-shape agreements, 1 lifecycle
  agreement. Each check refuses the mutated contracts in §6.
* **Findings.** No view claims a custody mode its detailed contract
  contradicts. One view is **not implied** outside a premise:
  `chunkio/memory_close.qcore` supports `resource_released(chunk, nonnull)` only
  on memory-backed chunks. Every view also assumes premises the v1 checker does
  not enforce. Both are registered.
* **Zero verdict movement.** All five audit artifacts of the AFTER run are
  byte-identical to the BEFORE run on `f8b41209` in this checkout; the
  diagnostics are byte-identical to the W43-LIBC-RESULT-ALIAS baseline.
* **Lean.** `proof/QCore/ContractViewProjection.lean`, audited by
  `AuditContractViewProjection.lean`: 22 theorems, no placeholder, no project
  axiom.
* **Follow-ups 299 → 305.** Six registered, none retired (§9).

## 1. The census tool

`qcore_contract::contract_views` (library) and
`cargo run -q -p qcore-contract --example contract_view_census [INDEX]`
(JSON). It reads the corpus `contract/corpus.json` selects — every `.qcore`
under its 11 roots, a module that uses inclusion read as its elaboration, the
same discovery the corpus test uses (`compiler/tests/contract_view_census.rs`
asserts the two readers agree byte for byte). There is no core `contract/`
root holding modules; fixtures and report probes are not corpus modules.

For every contract-bearing `extern fn` it records the module, parameters,
result, clauses, and a **vocabulary** read off the text (`resource_v1`,
`integer_observation`, `atomic_lifecycle`, `language`,
`source_shape_metadata`, `typed_policy`), and whether all declarations of a
name are token-identical. [`scripts/census.py`](scripts/census.py) adds the
**consumers**, measured by scanning code and manifests (not documentation) for
each module's logical path and classifying the binding (`bundled` via
`include_str!`, `path`, `manifest`, `inventory`, `test`). Every module is also
read by the corpus-wide discovery and QCore front end compile.

Records: [`census/census.json`](census/census.json) (after, exit 0),
[`census/base-census.json`](census/base-census.json) and
[`census/base-refusals.txt`](census/base-refusals.txt) (base, exit 1).

### 1.1 The census

| Function | Modules (vocabulary) | Measured consumers (non-test) | Relation |
|---|---|---|---|
| `flb_malloc`, `flb_calloc`, `flb_free` | `fluent_bit/memory.qcore` (integer observation) · `fluent_bit/resource_memory.qcore` (v1, inherits libc) | memory: `scripts/cycle19_native_observations.py` → `check_contract_observations` (observation interpreter). resource_memory: `cycle19-resource-contracts.json` (resource loader) | projection |
| `cio_chunk_close` | `chunkio/chunk_destructor.qcore` (language) · `chunkio/memory_close.qcore` (lifecycle mechanism) · `chunkio/resource_open_close.qcore` (v1) | destructor: front end only (corpus compile; a loader test shows v1 refuses it). memory_close: `compiler/src/verifier_plugins/chunkio/memory_close.rs` (bundled). open_close: `cycle19-resource-contracts.json` | projection ×2, lifecycle agreement ×1 |
| `cio_error_reset` | `chunkio/error_frame.qcore` · `chunkio/error_state.qcore` (frame vocabulary) | error_frame: `contract/profiles/fluent-bit/source-contracts.json` (BooleanRecord flavour). error_state: `compiler/src/verifier_plugins/chunkio/memory_close.rs`, `compiler/examples/support/cycle19_compatibility.rs` (bundled; Chunk flavour) | renamed equivalence |
| `_mk_event_loop_create`, `_mk_event_loop_destroy`, `_mk_event_wait_2`, `_mk_event_wait_with_flags` | `monkey/wait_source_policy.qcore` (source-shape metadata) · `monkey/wrapper_paths.qcore` (typed behavioural) | policy: `crates/qcore-verification/src/monkey_wait_projection.rs` (bundled). wrapper_paths: `monkey_wait_projection.rs`, `cycle18_composition.rs`, `compiler/src/verifier_plugins/libevent/monkey_source.rs`, the Cycle 19 adapter (bundled) | source-shape agreement |
| `cb_wait_2_timeout` | `monkey/wait_source_policy.qcore` · `monkey/composition.qcore` (callback summary) | composition: `monkey_wait_projection.rs`, `cycle18_composition.rs`, Cycle 19 adapter (bundled) | source-shape agreement (C parameter 2 only) |
| `source_identity` (base only) | `monkey/timeout_teardown.qcore` · `monkey/wait_source_policy.qcore`, **token-identical** | `timeout_teardown.rs`, `monkey_wait_projection.rs` (bundled) | removed (§2) |

The integrator's reading had the Monkey pair's direction reversed:
`wait_source_policy.qcore` holds the parameterless `signature == "…"`
metadata, `wrapper_paths.qcore` the typed declarations.

## 2. Genuine duplicates removed

`source_identity` was the only token-identical pair. `monkey/wait_source_policy.qcore`
keeps it: its consumer requires the declaration once among its reserved names.
Before removing the copy from `monkey/timeout_teardown.qcore`, the card checked
that its consumer still gets the declaration:

* `qcore_verification::timeout_teardown` bundles the owner
  (`include_str!` of `wait_source_policy.qcore`) and evaluates `source_identity`
  there with the same four facts as before.
* `TimeoutTeardownContracts::parse_policies` is total-or-refuse: the owner must
  declare `source_identity` (its revision literal is read), and the timeout
  policy must **not** redeclare it, so exactly one declaration is evaluated.
* `TimeoutTeardownSourceProjection` and `TimeoutTeardownEvidence` gain
  `source_identity_digest` (the owner's SHA-256); `check_timeout_teardown_evidence`
  refuses evidence naming another owner. `quirin-audit timeout-teardown`
  derives the field from the projection. The configured-file check of the
  timeout policy's own digest is unchanged.
* Tests: `source_identity_has_one_owner` (owner evaluated, redeclaration
  refused, ownerless owner refused, a changed revision refused);
  `source_order_and_evidence_binding_are_load_bearing` gains the wrong-owner
  control.

Inclusion was not used: this consumer reads typed policies, not the v1 loader,
and `qcore-provider-libevent` is off the load path (`QI017`). The typed-policy
consumer was extended rather than taught `#include`, so
`CONTRACT-INHERIT-NON-V1-CONSUMERS` stays open. The timeout policy's digest
moves (`516fa1fe…` → `dfbc9a41…`); the retained Cycle 18 native evidence names
the old one, and no native run was made (`CONTRACT-VIEW-TIMEOUT-POLICY-NATIVE-RERUN`).

`cio_error_reset` was decided with evidence (§4) to be a renamed equivalent,
but is **kept** as two views, one per consuming evaluator.

## 3. Projections: the detailed contract entails the view

A projection check reads the view (v1) clause by clause through a declared
**bridge** from the detailed vocabulary:

| v1 clause | Meaning checked |
|---|---|
| `requires resource_contract_v1` | selector, no obligation |
| `resource_borrowed(p)` | `p` is a scalar in both declarations |
| `resource_return_none()` | the detailed result is `unit` |
| `resource_return_owned(nullable)` (allocator) | `(result_null == 1 and live_after == 0) or (result_null == 0 and live_after == 1 and result_is_backend == 1)` |
| `resource_released(p, nullable)` (allocator) | under `valid_input_extent == 1`: `live_after == 0 and forwarded_input == 1`; `p` a pointer |
| `resource_released(p, nonnull)` (ChunkIO mechanism) | under `nullable_owned_chunk and chunk_was_nonnull`: `chunk_released` |
| `resource_released(p, nonnull)` (ChunkIO language) | under `chunk_is_live`: `chunk_name_and_object_are_released`; `p` is `own<…> use one` |

Allocator domain premise: `result_null` is 0/1 (the native adapter's null
test). A view clause with no bridge rule is refused; so is any other view
`requires`. Signatures must agree positionally in ABI category (scalar
spelling exact; pointer mutability, nominal erased as v1 projects to `u8`).

**Entailment** (`contract_views::entails`) is decided exactly over the
equality fragment the shared interpreter evaluates (literals, names, `==`,
`!=`, `and`, `or`, `not`), with the interpreter's own precedence. Premises are
restricted to the goal's cone of influence; each name ranges over the
involved literals, 0, 1 and enough fresh values for the largest group of
directly compared names. Any integer assignment maps into that domain with
every atom's truth kept, so enumeration decides the question; a premise
outside the fragment is dropped (sound, and reported), a goal outside it is
refused, and more than 2²⁴ assignments is refused. A unit test checks the
fragment evaluator against `contract_observation`'s on a grid.

**Assumed premises are computed**: a detailed `requires` the view's own
custody premise does not entail is an assumption of the relation. The list
must equal the pinned one, each with a class — `Profile`, `Environment` or
`Coverage` — and a `Coverage` premise needs a registered finding.

| Pair | Result | Assumed (computed = pinned) |
|---|---|---|
| `flb_malloc` memory → resource_memory | `return_owned(nullable)` entailed (4,096 assignments); `borrowed(size)` scalar | `ordinary_allocator_profile == 1` (Profile) |
| `flb_calloc` | `return_owned(nullable)` entailed (262,144); both scalars | same |
| `flb_free` | `released(ptr, nullable)` entailed (9); `valid_input_extent` discharged by the view; unit result | same |
| `cio_chunk_close` destructor → open_close | `released(chunk, nonnull)` entailed; `chunk_is_live` discharged; `delete` scalar | `chunk_is_not_busy`, `chunk_has_no_task_or_route_custody` (Environment) |
| `cio_chunk_close` memory_close → open_close | `released(chunk, nonnull)` entailed (262,144); `nullable_owned_chunk` discharged | 10: `nonnull_chunk_has_memory_backend` (**Coverage**), 7 Environment, 2 Profile |

**Finding, not implied.** Outside `nonnull_chunk_has_memory_backend`,
`memory_close.qcore` says nothing about the chunk, yet the resource loader
applies `resource_released(chunk, nonnull)` to every call
(`outside_the_memory_backend_the_view_is_not_implied`). The destructor contract
covers every backend, but it is a language-level contract, not a
source-checked one. The detailed contract was **not** weakened.
`CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`; the unenforced premises are
`CONTRACT-VIEW-UNENFORCED-PREMISES`, the bridges `CONTRACT-VIEW-BRIDGES-DECLARED`.

## 4. `cio_error_reset`: renamed equivalence, decided

* **Syntax.** `error_state.qcore`'s declaration equals `error_frame.qcore`'s
  token for token under `Chunk→cio_chunk`, `chunk_is_live→object_is_live`,
  `exclusive_chunk_access→exclusive_object_access`,
  `lock_is_preserved→modeled_fields_are_preserved`.
* **Semantics.** Both real evaluators — `evaluate_chunk_frame_contract` on
  `error_state` and `evaluate_boolean_frame_contract` on `error_frame` over
  record `cio_chunk` — run over every write vector of length ≤ 2 over
  `error_n`, `lock`, `busy` with values 0/1 (31 summaries), for both protected
  sets the imports declare, `{lock}` and `{busy, lock}`. They agree on all 31
  in both (1 accepted: `error_n = 0`).
* **The condition.** They agree exactly when the protected set contains `lock`
  and excludes the declared write footprint `{error_n}` (Lean
  `the_frame_evaluators_agree_under_the_condition`); a set covering `error_n`
  separates them (`the_frame_equivalence_condition_is_load_bearing`,
  `the_condition_is_load_bearing`). The importer refuses writes to its
  protected vector, so the condition holds for every real import.

**Kept as two.** Each is the pinned contract of a different evaluator flavour
(`FrameContractFlavour::{Chunk, BooleanRecord}`), bound by digest into its
frame-helper task identity; an evaluator refuses the other vocabulary by
design, and removing the Chunk flavour's contract is a route change under
invariant 3. That is "one view per consuming checker".
`CONTRACT-VIEW-ERROR-RESET-RENAMED-EQUIVALENT` records the discharge path.

## 5. Different kinds: everything both state agrees

**Source shape.** For each Monkey pair every `signature == "…"` alternative in
the metadata is parsed (`void`, `int`→`i32`, `short`→`i16`, `long`,
`unsigned int`, `size_t`, Libevent's Unix `evutil_socket_t`→`i32`, `void *`,
`struct TAG *`) and compared with the typed declaration: arity; each position
(a C const pointer must stay const; a C non-const pointer may be borrowed
const); each `struct TAG` against the registered nominal
(`mk_event_loop`→`EventLoop`, `mk_event_ctx`→`EventContext`); the result. Then
all nine `contract_NAME` facts (`parameter_names`, `parameter_types`,
`parameter_usages`, `result_type`, `clause_sequence`, extern/body/unsafe/
effects) are recomputed from the typed declaration and checked with
`check_typed_contract_facts`. `cb_wait_2_timeout`'s summary declares only the
callback's user argument, so it is related to C parameter 2 of both recorded
alternatives. All five agree.

**Lifecycle profiles.** `chunk_destructor.qcore` and `memory_close.qcore` —
neither entails the other — must agree on every position's ABI type and the
result. At base this refused `delete_backing: bool` against `delete: i32`
(`census/base-refusals.txt`); C is `void cio_chunk_close(struct cio_chunk *ch,
int delete)` (`lib/chunkio/include/chunkio/cio_chunk.h:56`, `src/cio_chunk.c:111`
of the pinned audit source), and `bool` projects `_Bool`. Corrected to `i32`;
the module's only code consumer is the front end compile, which passes.

## 6. Negative controls

`crates/qcore-contract/tests/contract_views.rs` (8 tests) mutates the real
corpus in memory and asserts the specific refusal:

| Mutation | Refusal |
|---|---|
| `flb_malloc`: `live_after == (result_null == 0)` → `live_after == 1` | `return_owned(nullable)` not implied |
| `flb_malloc`: drop `result_is_backend` clause | not implied |
| `flb_free`: `live_after == 0` → `== 1` | `released(ptr, nullable)` not implied |
| `flb_malloc`: add `requires size_is_small == 1` | the view assumes a premise not pinned |
| `flb_calloc`: `n: usize` → `i32` | parameter 0 disagrees |
| view: `return_owned(nullable)` → `(nonnull)` | no bridge meaning |
| `memory_close`: `chunk_released` → `name_released` | not implied |
| destructor: drop the release clause | not implied |
| destructor: `i32` → `bool` (the repaired defect) | parameter 1 disagrees |
| destructor: `own<Chunk>` → `borrow<…>` | needs a consumed owned parameter |
| `error_state` / `error_frame` clause edits | not token-equal under the renaming |
| metadata arity, struct tag, scalar type; typed parameter type; `parameter_types` fact; callback projection | each refused |
| restored `source_identity` | unregistered multi-module declaration |
| copied `flb_sds_destroy` | unregistered multi-module declaration |
| stale registration; Coverage without finding; pinned assumptions changed | refused |
| textual duplicate registered as a relation / as `Identical` without finding | refused (with a finding it is admitted: the only way to keep one) |
| 20 compared name pairs | domain too large, refused |

The Lean model states the corresponding negatives (§7).

## 7. Lean

`proof/QCore/ContractViewProjection.lean`, 22 theorems in
`AuditContractViewProjection.lean`, axioms `propext`/`Classical.choice`/
`Quot.sound` only:

* projection: `malloc_entails_owned_nullable`, `calloc_entails_owned_nullable`,
  `free_entails_released_nullable` over unbounded integers; non-vacuity
  (`malloc_contract_has_a_null_and_a_live_model`,
  `calloc_contract_has_a_live_model`); mutated contracts do not entail
  (`the_mutated_malloc_contract_does_not_entail_the_view`,
  `the_mutated_free_contract_does_not_entail_the_view`);
* ChunkIO: `memory_close_entails_released_nonnull`,
  `releasing_only_the_name_does_not_entail_the_view`,
  `outside_the_memory_backend_the_view_is_not_implied`,
  `chunk_destructor_entails_released_nonnull`;
* frames: `the_frame_evaluators_agree_under_the_condition`,
  `the_condition_is_load_bearing`,
  `both_real_protected_sets_satisfy_the_condition`;
* signatures: `an_arity_mismatch_is_refused`,
  `the_monkey_wrapper_signatures_agree`, `a_mistagged_pointer_is_refused`,
  `a_const_pointer_declared_mutable_is_refused`;
* guard: `an_unregistered_pair_is_refused`, `a_stale_registration_is_refused`,
  `an_exactly_registered_census_is_accepted`; `dropping_premises_is_sound`.

Outside every statement: that the Lean clause models transcribe the `.qcore`
text (review, as for every model here), and the enumeration lemma behind
`entails` (`CONTRACT-VIEW-SMALL-MODEL-UNPROVED`).

## 8. Evidence

### 8.1 Native evaluation of `fluent_bit/memory.qcore`

The module is unchanged (SHA-256 `b1c44c63…2645aad`, equal to the Cycle 19
record's `contract_sha256`). Its native consumer was rerun on the 145 retained
executions:
`cargo run -p qcore-compiler --example check_contract_observations -- providers/quirin-provider-fluent-bit/contracts/fluent_bit/memory.qcore docs/reports/cycle19-native-contracts/flb_mem_observation.jsonl`,
exit 0, 145 checked (10/50/40/40/5), output **byte-identical** to
`cycle19-native-contracts/flb_mem_contract_result.json`. No fresh native
execution was compiled: the harness drives git in the shared pinned checkout
outside this worktree.

### 8.2 Corpus arbiter

`run_audit.py` on `f8b41209` (BEFORE) and on `bdf2233b` (AFTER, the last
commit touching analysis code), both in this checkout:

* BEFORE vs the W43-LIBC-RESULT-ALIAS baseline: diagnostics, chunk-contracts
  and source byte-identical; contracts and source-effects differ, as the
  adoption already made them ([`arbiter/before-vs-w43.txt`](arbiter/before-vs-w43.txt)).
* AFTER vs the baseline: the same ([`arbiter/after-vs-w43.txt`](arbiter/after-vs-w43.txt)).
* AFTER vs BEFORE: **all five artifacts byte-identical**, contracts included
  ([`arbiter/after-vs-before.json`](arbiter/after-vs-before.json)). There is
  nothing to attribute: no module the resource manifest selects changed.

Run records: [`arbiter/before-run.json`](arbiter/before-run.json),
[`arbiter/after-run.json`](arbiter/after-run.json).

### 8.3 Gates and packages

GATE-PLACEHOLDER

## 9. Follow-ups

Registered ([`followups.json`](followups.json), applied by
[`scripts/update_ledger.py`](scripts/update_ledger.py)), 299 → 305:
`CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`, `CONTRACT-VIEW-UNENFORCED-PREMISES`,
`CONTRACT-VIEW-ERROR-RESET-RENAMED-EQUIVALENT`, `CONTRACT-VIEW-SMALL-MODEL-UNPROVED`,
`CONTRACT-VIEW-BRIDGES-DECLARED`, `CONTRACT-VIEW-TIMEOUT-POLICY-NATIVE-RERUN`.

Retired: none. No registered follow-up names this work;
`CONTRACT-INHERIT-NON-V1-CONSUMERS` stays open because no non-v1 consumer
learned inclusion.

## 10. Coordination

Shared files touched: `compiler/tests/qcore_contract_corpus.rs` (one pin
removed), `compiler/tests/contract_loader.rs` (rejected 207 → 206),
`providers/qcore-provider-{chunkio,libevent}/contracts.json` (regenerated by
`contracts.py sync`, one line each), the acceptance matrix, `docs/STATUS.md`,
the increment index. No `rd_*` or `mk_mem_*` declaration was touched.
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING also edits the libevent inventory:
resolve by rerunning `sync`. If it moves libevent onto the load path, a
typed-policy consumer could later include the owner instead of bundling it. A
new multi-module declaration from either concurrent card fails
`contract_view_census.rs` until registered.

## Reproduction

```sh
cargo run -q -p qcore-contract --example contract_view_census
python3 docs/reports/contract-view-projection-check/scripts/census.py . <out.json>
cargo test -p qcore-contract --test contract_views
cargo test -p qcore-compiler --test contract_view_census
python3 docs/reports/w43-return-expression/scripts/run_audit.py <out> <commit>
python3 docs/reports/contract-inherit-adoption/scripts/arbiter_compare.py <before> <after> <out.json>
python3 docs/reports/contract-view-projection-check/scripts/run_gate.py <dir> <steps…>
python3 docs/reports/contract-view-projection-check/scripts/update_ledger.py   # once
```
