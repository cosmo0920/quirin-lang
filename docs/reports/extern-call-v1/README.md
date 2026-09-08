# `source.extern-call.v1` — bounded extern-call semantics

Card record for `source.extern-call.v1`, specified in
[`docs/design/EXTERN_CALL_SEMANTICS_V1.md`](../../design/EXTERN_CALL_SEMANTICS_V1.md).
Implementation checkpoint: this branch. Date: 2026-09-08.

**What this increment establishes.** A QCore source file may now call a
**declared** extern function, at one bounded boundary, and the result compiles,
checks, emits C11 and runs against a real linked C implementation. **What it
does not establish:** anything about that C implementation. A contract admitted
here is `ContractTrust::Declared` — a trusted assumption (contract spec §C06) —
and every verdict downstream is valid only *relative to* it. Nothing is
promoted: no Cycle 19 raw unit changes, no CT case is mapped, the contract
corpus is unchanged, and full language conformance stays false.

## 1. Semantics implemented, by design section

### §1 — the three admission conditions

| Condition | Where | Refusal |
|---|---|---|
| The calling file opts in with `import std.extern;` | `qcore_core::module_imports` (`EXTERN_MODULE`, `ModuleImports::extern_calls`), gated in `qcore_frontend::module_imports::check_source_imports` before any checking | `Q014` unknown module name; `Q022` extern call without the import |
| The callee has a trusted declared contract from an explicit `CompileOptions` selection | `CompileOptions::extern_contracts`, loaded by the unchanged `qcore_contract::contract_loader::load_resource_contract_sources` | `Q023` |
| The call site satisfies §3 | `qcore_core::extern_call`, `qcore_frontend::source_extern` | `Q024`, `Q025`, `Q026`, `Q027` |

Resolution follows the `import std.concurrency;` precedent exactly: the core
owns exact bundled-module resolution, the frontend refuses before checking each
file, and importing the module in **another** compilation input grants this file
nothing (pinned by
`importing_the_module_in_another_file_does_not_grant_this_one_the_call`).

One detector (`source_extern::called_extern_names`) serves both the import gate
and route selection, so a file can never be routed to this profile without
having needed the import, nor need the import without being routed here. It keys
on the two **call positions** the grammar can express — the head of an
expression statement, and the operand immediately after `=` in a `let` — at any
nesting depth. An extern name in a `defer`, a `raise` or a `spawn` operand is not
an extern call in this grammar and keeps its existing refusal on the route that
owns that form. This is deliberate: it is why `elaborates_names_and_attributes_…`
and `scope_and_worker_roles_cannot_be_satisfied_by_other_visible_names` still
observe exactly the diagnostics they observed before.

### §3.1 — admitted callee shapes

`qcore_core::extern_call::bridge_contract` is total over the complete C04
vocabulary. Admitted:

| C04 declaration | Bridged as |
|---|---|
| scalar parameter with `resource_borrowed` | `CallParameter::Data` (data only; a scalar may carry a descriptor number, never custody) |
| `ptr<mut, u8>` / `ptr<const, u8>` with `resource_borrowed` | `CallParameter::Borrowed` |
| `ptr<mut, u8>` with `resource_consumed(p, nonnull)` | `CallParameter::Consumed` |
| `resource_return_none()` with `unit` or a scalar result | `CallResult::Data` |
| `resource_return_owned(nullable)` returning `ptr<mut, u8>` | `CallResult::OwnedNullable` |

Refused, each by its own named constructor rather than a catch-all:

| Excluded shape | `BridgeRefusal` | Rust test | Lean theorem |
|---|---|---|---|
| `resource_owned` | `OwnedParameter` | `an_owned_parameter_is_not_callable` | `owned_parameter_refuses` |
| `resource_released` | `ReleasedParameter` | `a_released_parameter_is_not_callable` | `released_parameter_refuses` |
| `resource_out` | `OutParameter` | `an_out_parameter_is_not_callable` | `out_parameter_refuses` |
| `resource_stored` | `StoredParameter` | `a_stored_parameter_is_not_callable` | `stored_parameter_refuses` |
| `resource_removed` | `RemovedParameter` | `a_removed_parameter_is_not_callable` | `removed_parameter_refuses` |
| `resource_consumed(_, nullable)` | `NullableConsumedParameter` | `a_nullable_consumed_parameter_is_not_callable` | `nullable_consumed_parameter_refuses` |
| custody mode on a scalar | `ScalarCustody` | `a_custody_mode_on_a_scalar_is_not_callable` | `scalar_custody_refuses` |
| borrowed `ptr<mut, ptr<mut, u8>>` | `UnsupportedBorrowedPointer` | `borrowing_a_pointer_to_pointer_is_not_callable` | `borrowing_a_pointer_to_pointer_refuses` |
| consumed `ptr<const, u8>` | `UnsupportedConsumedPointer` | `consuming_a_const_pointer_is_not_callable` | `consuming_a_const_pointer_refuses` |
| `resource_return_owned(nonnull)` | `NonnullOwnedReturn` | `a_nonnull_owned_return_is_not_callable` | `nonnull_owned_return_refuses` |
| `resource_return_reallocated` | `ReallocatedReturn` | `a_reallocated_return_over_admitted_parameters_still_refuses_by_its_own_name` | `reallocated_return_refuses` |
| owned result that is not `ptr<mut, u8>` | `UnsupportedOwnedResultType` | `an_owned_scalar_result_is_not_callable` | `owned_result_requires_a_mutable_pointer` |
| `unit` parameter | `UnitParameter` | (unreachable through the loader; kept so admission is total) | — |

Non-vacuity is pinned on both sides: `the_admitted_table_is_inhabited` (Lean)
and the four positive bridging tests (Rust).

### §3.2 — type bridging, and the extent premise

The bridging table is closed; `AbiType::parse` accepts exactly the loader's
spellings and nothing else. **Length and extent are not transported.** C04
pointers are extentless `u8` projections, so no declaration on this route
constrains how many bytes a callee touches. That is recorded as
`ExternAssumption::NoExtentTransport` on every certificate, not checked. Buffer
extents remain a named follow-on.

### §3.3 — the nullable-owned-return adapter

Source form: `let NAME = CALLEE(ARGS) succeeds BOOL else raise ERROR;`.
`succeeds BOOL` is the fixture's **literal outcome control**, exactly the
`qcore_acquire`/`qcore_check` philosophy: a controlled model operation, not a
claim about when a production allocator fails. Both arms are exercisable, and
the null arm lowers onto the existing typed-error/`AllocError` route through
`SourceOp::Acquire { succeeds, error }` rather than a new mechanism.

`succeeds` is a new reserved source word, added for the same reason `mutex`,
`guards`, `protected`, `close` and `borrow` were reserved by T03: so a local
binding cannot shadow the adapter form.

Refusals (all `Q025`): the adapter omitted on an owned nullable return; the
adapter applied to a callee that returns no custody; an adapter naming a
declared name that is not an error type. Naming an **undeclared** name is `Q013`
at name resolution, which runs first and owns that condition — the extern pass
does not duplicate it under a second code
(`an_adapter_raising_an_unknown_name_refuses_at_name_resolution`).

One bounded restriction, stated as a restriction: the v1 adapter admits its
error arm only where **no acquired identity is live**
(`AdapterAbandonsCustody`). Releasing custody on an extern error arm would need
a per-callee cleanup relation this profile does not have. Consequence: two
acquisitions cannot be live simultaneously, and a second acquisition is
admitted only after the first is consumed
(`a_second_acquisition_after_the_first_is_consumed_is_admitted`).

### §3.4 — checker semantics

`check_whole_program_with_externs` is the two-way dispatch. `check_whole_program`
delegates to it with an empty extern map, so its behavior is unchanged, and the
literal string `"call target is not a source function"` still fires for a callee
in neither set. Extern leaves carry no body: they contribute no call-graph edge,
never enter the recursive post-fixed-point rule, and never receive a
`CheckedFunction`. `CallSite` gains `result: Option<ResourceId>` so a call
returning fresh custody binds an identity `check_identity_transfer` can see; the
result is introduced **after** the same call's consumptions, so a callee that
both consumes and produces cannot reuse an argument's identity.

Derived summary (`derived_custody`, `derived_effects`, `source_extern::extern_summary`):

- parameters → `ParameterOwnership::Borrowed` / `Consumed`, data parameters excluded;
- flow → release per consumed pointer, then acquire for an owned return;
- effects → `{Io}`, plus `Alloc` iff the contract returns owned custody, plus
  `Mut` iff any parameter is a `mut` pointer.

The caller's declared `effects { io, alloc, mut }` row must admit the derived
row; containment is checked by the established typed-error pass under `Q300`, so
this card adds no code for it. An entry with **no** row therefore admits no
extern call at all (`an_entry_with_no_effect_row_refuses_every_extern_call`),
which keeps the opt-in visible interprocedurally. An effect name outside
`io`/`alloc`/`mut` is `Q020`.

Contexts. Extern calls are admitted in a flat sequential entry body only. A
nested block, a second source function and an entry with parameters are `Q026`.
The concurrent routes are selected **first**, so a module that also spawns or
locks stays with the task/mutex route and keeps its existing refusal
(`a_concurrent_module_keeps_its_existing_refusal`); `source.exec12` is untouched
and still refuses every call whose target is not a zero-parameter source
function.

Leaks are not re-decided here. An unreleased owned return is refused by the
custody/cleanup passes under `Q103`/`Q202`
(`an_unreleased_owned_return_refuses_under_the_established_custody_pass`).

### §3.5 — reference execution and emission

`extern_c_emit::interpret_extern_program` computes the observation from the
checked event list; `emit_extern_c11` prints a prototype per callee from the ABI
table and calls it directly. Neither is derived from the other's output.

The emitted program does **not** trust its own literal outcome control: every
adapter emits a runtime comparison of the real result's nullness against the
declared control and exits with status 70 if they disagree, so a fixture whose C
implementation stops matching its declared arm fails loudly instead of silently
observing the other branch.

Symbol resolution and linkage are the builder's premise, recorded as
`ExternAssumption::LinkageSuppliedByBuilder`.

## 2. Diagnostic inventory and the §L21 delta

New pass `SemanticPass::ExternCall`, mapped to `Q022` in
`qcore_frontend::frontend::source_diagnostic` (the authoritative pass → code
map). Spec §L21 gains one row and one clarifying paragraph in the same change.

| Code | Condition | Emitted by |
|---|---|---|
| `Q022` | an extern call in a file without `import std.extern;` | `qcore-frontend::module_imports` |
| `Q023` | the callee has no admitted declared contract in the explicit selection — no selection, a selection that does not load, a duplicate declaration across selected modules, an absent name, or an environment with no retained source text | `qcore-verification::driver`, `qcore-frontend::source_extern` |
| `Q024` | the closed v1 bridging table refuses this contract mode, ABI type, argument form or argument count | both |
| `Q025` | the nullable-owned-return adapter is missing, applied to a callee returning no custody, names a declared non-error, or cannot discharge live custody on its error arm | both |
| `Q026` | an extern call in a refused context: a nested block, a non-entry source function, or an entry with parameters | `qcore-frontend::source_extern` |
| `Q027` | a declared input and the checked artifact disagree: an `extern fn` declaration against its selected contract, an inline `contract { … }` on a called extern declaration, a selection that no longer reproduces its own sources, or a published certificate against its independent rederivation | `qcore-verification::driver` |

**No existing code changed.** `Q020` and `Q021` keep their exact behavior and
stay separate; this card is deliberately *not* the inline source-contract
consumer they are waiting for, and an inline `contract { … }` on a called extern
declaration is refused (`Q027`) rather than enforced. `Q020` gains one emission
site (an unmodeled effect name), which is inside its existing meaning
"unsupported or ill-typed ordinary construct" and is recorded in §L21.

Also updated in the same change, so neither statement goes stale:
`docs/QCORE_SPEC_r0_3.md` line 312's `source.exec12` paragraph now points at the
bounded path, §L02 gains a `source.extern-call.v1` profile row, §L14 gains the
bounded statement, and `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` gains §G02d and
a pointer from its driver-body-path paragraph.

## 3. Lean inventory

`proof/QCore/ExternCall.lean` — 27 public theorems, audited by
`proof/AuditExternCall.lean` (32 `#print axioms` lines, including five from
`QCore.ModuleImports`). No new axiom; the audit set moves from 34 to 35 and
`AuditAll.lean`'s closure check is unchanged.

| §5 obligation | Theorems |
|---|---|
| the call step as an opaque transition whose custody delta is the declared summary | `a_data_call_never_binds_an_identity`, `a_successful_acquisition_binds_exactly_the_fresh_identity`, `a_failed_acquisition_binds_nothing` |
| conservation under a derived summary, both parameter and both return modes | `the_custody_delta_is_exactly_the_declared_one`, `a_consumed_identity_does_not_survive` |
| adapter totality — no third exit | `adapter_totality`, `a_data_call_never_takes_the_error_arm` |
| refusal completeness for every non-admitted C04 mode | the twelve refusal theorems of §1 above, plus `the_admitted_table_is_inhabited` for non-vacuity |
| summary derivation is a function | `admission_is_deterministic` |
| the derived effect row | `every_derived_row_contains_io`, `alloc_is_derived_exactly_for_an_owned_return`, `mut_is_derived_exactly_for_a_mutating_parameter`, `a_const_loan_of_a_scalar_call_derives_no_mutation`, `an_empty_declared_row_admits_no_extern_call`, `a_row_without_alloc_refuses_an_acquiring_call` |

`proof/QCore/ModuleImports.lean` is extended with a second independent
capability field and five theorems:
`empty_imports_grant_no_extern_calls`,
`admitted_extern_calls_require_the_extern_import`,
`the_concurrency_import_alone_grants_no_extern_call`,
`sequential_code_needs_no_extern_import`, `both_imports_are_independent`. The
existing concurrency theorems keep their names and meanings; their anonymous
constructors were widened from `⟨false⟩` to `⟨false, false⟩` because the
modelled structure gained a field, and `unknown_module_rejects` now carries both
"unknown" hypotheses. `AuditConcurrencyPrelude.lean` is unchanged.

**Premises stated in the theorems, discharged nowhere:** that a callee's C
implementation satisfies its declared contract, and that the Rust extraction of
a source module agrees with the Lean transcription.

## 4. Native evidence

`accepted_fixtures_run_natively_against_a_linked_c_implementation`
(`compiler/tests/extern_calls.rs`, Unix targets) compiles the emitted C together
with `compiler/tests/fixtures/extern_calls/buffer_api.c` — a real implementation
of all five declared callees — under
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined`
with `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and requires the native
observation to equal the reference execution **line for line**.

Two traces, both passing:

| Fixture | Native/reference observation |
|---|---|
| success arm | `acquire qcore_extern_alloc r1 ok` / `call qcore_extern_fill b:r1 d:7` / `call qcore_extern_sum b:r1 d:8` / `call qcore_extern_release c:r1` / `return` |
| null arm | `acquire qcore_extern_alloc_failing r1 null` / `raise Failure` |

The success arm allocates one real heap buffer, writes it through a `mut` loan,
reads it through a `const` loan, and frees it through the consuming callee; the
null arm allocates nothing. LSan is live in this environment (a deliberate
64-byte leak probe exits 1), so a passing run is a real observation, not a
vacuous one.

Scope of that evidence: one Linux x86-64 target, one compiler invocation, one
process, two traces and this implementation. It is not evidence about the
declared contracts in general, about buffer extents, about any other callee, or
about a callee that does not match its declared arm — that last case is exactly
what the emitted status-70 check refuses to hide.

## 5. Test matrix (§6)

**`crates/qcore-core/tests/unit/extern_call.rs` — 37 tests.** Four positive
bridging rows; thirteen admission-refusal rows (the table in §1); determinism
and data-parameter exclusion of the derived custody vector; three derived-effect
rows; accepted trace with certificate rederivation; both adapter arms;
adapter-missing, adapter-unexpected, unknown-error, abandoned-custody,
duplicate-acquisition, arity, argument-form and unknown-callee refusals; a
tampered certificate failing its recheck; reference execution of both arms;
emission content; and emission refusing a certificate from another entry.

**`compiler/tests/extern_calls.rs` — 34 tests.** Route selection and the
explicit checked-artifact variant; certificate rederivation end to end and its
tampered control; emission content and reference agreement; both adapter arms;
the native sanitizer run; five import-gate rows including cross-file
non-leakage and a declaration-only file keeping its old route; four `Q023` rows
(no selection, absent name, unloadable selection, duplicate across two selected
modules); five `Q024` rows; four `Q025` rows; three `Q026` rows including the
concurrent-module control; four `Q027` rows including the inline-contract
refusal and a **mutated contract file** that silently widens `fill` to consume
its argument and is no longer admitted; three declared-effect-row rows; and the
leak row that stays with the custody pass.

Contract fixtures live under `compiler/tests/fixtures/extern_calls/contracts/`,
**not** `contract/qcore/`, so the 60-module corpus pin is untouched.

## 6. Acceptance gate (§8)

| Check | Result |
|---|---|
| `cargo test --workspace` | **1544 passed, 0 failed, 1 ignored** (1473 before + 71 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **35 Lean audits** pass (34 before) |
| `providers/` workspace | 58 passed, 0 failed |
| `scripts/tests` (`python3 -m unittest`) | 95 passed |
| Native ASan/UBSan/LSan | 2 fixtures, both pass, observations equal |

No gateway or audit code is in the diff — `crates/qcore-c-gateway/` and
`crates/qcore-audit/` are untouched — so **the four pinned 185-unit artifacts
are unchanged by construction**, asserted from the diff rather than by
re-measurement, exactly as §8 requires. CORPUS-REPIN stays open and stays
someone else's card. `contract/qcore/` is untouched, so the corpus stays at 60
modules and the loader's rejected count stays at 145. No raw unit is promoted,
no CT case is mapped and `docs/reevaluation/ct_manifest.json` is untouched.
`qcore-schemas` is unchanged at 5.0.0; no wire version moved.

One pre-existing environment issue was hit and fixed, not worked around: the
`third_party/libevent` submodule was uninitialized in this fresh worktree and
`libevent_submodule_matches_the_audited_revision` failed until
`git submodule update --init --recursive` ran. That pin was not weakened.

## 7. Deviations from the design document

1. **Staging.** §9 offers an optional (a)/(b) split and says "one card is
   feasible". The implementation landed as one commit and the specification
   text as a second, rather than at the §9 seam, because the seam would have
   required writing the accepted-path tests against an emission refusal and then
   rewriting them one commit later. Every gate in §6 was run on the merged
   result.
2. **"the exec12 driver path" in §3.4.** The design lists it beside task
   workers, `with lock` bodies and error-region bodies as a context that keeps
   its refusal, while §3.5 requires emission and native linking — which requires
   extern calls to be executable *somewhere*. Read together, the only coherent
   implementation is a **separate route**: `source.exec12`'s statement grammar
   is unchanged and still refuses every non-source-function call, and
   `source.extern-call.v1` is its own `SourceExecutionProfile` with its own
   lowering, checker composition and backend, selected after the concurrent
   routes. "Sequential function bodies only" is read as "not concurrent, not an
   error region", which is what the rest of §3.4 says.
3. **One new reserved word.** `succeeds` had to be reserved for the literal
   outcome control; the design does not mention adding one. No `.qcore` file or
   test in the repository used it as an identifier.
4. **`Q020` gains an emission site** (an effect name outside `io`/`alloc`/`mut`)
   rather than consuming one of `Q022`–`Q027`, because that condition is inside
   `Q020`'s existing meaning and the six new codes are exactly the six
   conditions §4 enumerates.
5. **The adapter's error arm is bounded further than §3.3 states**: it is
   admitted only where no acquired identity is live. §3.3 requires the null arm
   to take the typed error exit but does not say what happens to custody already
   held; releasing it would need a per-callee cleanup relation this profile does
   not have, so the restriction is explicit and named as a follow-on rather than
   silently assumed away.

## 8. Open, restated from §7

Named follow-ons, none of them closed or weakened here:

- inline source-contract enforcement (the `Q020`/`Q021` consumer);
- the `resource_out` / `stored` / `removed` / `reallocated` / `released` call
  modes, and nonnull owned returns;
- sentinel and `errno` adapters (interacts with O03's post-state work);
- **releasing custody on an extern error arm**, and therefore more than one
  simultaneously live acquisition (new, from §7 of this report's deviation 5);
- buffer extents; variadics, function pointers, callbacks, structs, floats;
- extern calls in concurrent or error-region contexts, and in more than one
  source function per module;
- cost summaries; linkage verification;
- a `qcorec` selection flag — the CLI currently selects no extern contract
  environment, so a source file calling an extern function is refused at `Q023`
  through that entry point;
- any gateway, audit-route or Cycle 19 interaction (untouched by construction).

**CT45.** Its unknown-destructor/access-contract scenario is adjacent and looks
closer to mappable now that a declared destructor-like callee can be *called*
from source: `resource_consumed(p, nonnull)` gives a checked move into an
opaque callee, and the `Q024` refusal of `resource_released` gives a precise
negative control. It is still blocked on this profile's single-acquisition
restriction and on having no way to express an unknown-contract callee that is
admitted-but-unmodeled. This is an observation only — `ct_manifest.json` is
untouched and CT coverage stays at 20/50; any mapping proposal is its own
stage-derived record.

## 9. Files

New: `crates/qcore-core/src/extern_call.rs`,
`crates/qcore-core/src/extern_c_emit.rs`,
`crates/qcore-core/tests/unit/extern_call.rs`,
`crates/qcore-frontend/src/source_extern.rs`,
`crates/qcore-contract/src/extern_bridge.rs`,
`compiler/tests/extern_calls.rs`,
`compiler/tests/fixtures/extern_calls/{round_trip.qcore,buffer_api.c,contracts/buffer_api.qcore}`,
`proof/QCore/ExternCall.lean`, `proof/AuditExternCall.lean`, this report.

Changed: `crates/qcore-core/src/{lib.rs,module_imports.rs}`,
`crates/qcore-frontend/src/{lib.rs,frontend.rs,module_imports.rs,whole_program.rs}`,
`crates/qcore-frontend/tests/unit/whole_program.rs`,
`crates/qcore-contract/src/lib.rs`, `crates/qcore-verification/src/driver.rs`,
`crates/qcorec/src/main.rs`, `compiler/src/lib.rs`,
`compiler/tests/{ct_corpus.rs,module_modes.rs}` (new `CompileOptions` field),
`proof/QCore.lean`, `proof/QCore/ModuleImports.lean`,
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`.
