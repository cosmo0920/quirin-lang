# EXTERN-F1 — the `resource_out` and `resource_stored` call modes

Card record for `EXTERN-F1`, the next bounded slice of the `EXTERN-CALL-F`
follow-on list recorded in
[`docs/reports/extern-call-v1/README.md`](../extern-call-v1/README.md) §8.
Date: 2026-09-09. Wave 10.

**What this increment establishes.** Two more C04 parameter modes are callable
from QCore source: `resource_out` (the callee writes fresh custody through an
out-pointer parameter) and `resource_stored` (the passed `own<Buffer>` becomes
indexed beneath a declared owner and leaves the caller's ledger permanently).
The out mode is exercised end to end — source, checker, certificate, reference
execution, C11 emission and two fixed ASan/UBSan/LSan runs against a real linked
C implementation, both arms, native matching reference line for line.

**What it does not establish.** Nothing about any C implementation: an admitted
contract is `ContractTrust::Declared` (contract spec §C06) and every verdict is
valid only *relative to* it. Nothing is promoted: no Cycle 19 raw unit changes,
no CT case is mapped, the contract corpus is unchanged at 60 modules, and full
language conformance stays false.

**The card's principal measured finding**, stated before the details because it
changes what "done" means for the store mode: **`resource_stored` is admitted by
the bridging table and checked by the model, but is not reachable from a single
source module in `source.extern-call.v1`.** Contract spec §C04.2 requires a
stored parameter's owner to be a *distinct `Owned` parameter*, so a store needs
two simultaneously live checked identities — the child and its owner — and this
profile admits at most one, because its adapter's error arm may abandon nothing
(v1 report §7 deviation 5). The blocker is upstream of the store and is pinned
as such: the fixture refuses at `Q025` on its **second acquisition**, never at
`Q024` on the contract. See §5.

## 1. The two modes, by declared semantics

### 1.1 `resource_out(p, nullable)` — an acquisition through a slot

C04.2: "The operation establishes fresh custody through the modeled output
destination. It requires the pointer-to-pointer ABI projection."

The source form reuses the `succeeds` adapter discipline unchanged, because the
out write is an acquisition and every acquisition arm in this profile stays
checked:

```qcore
let b = qcore_extern_open(out, 8) succeeds true else raise Failure;
```

`out` marks the declared slot's *parameter position*; the `let` binder names the
identity the callee writes into it. The binding materializes on the success arm
only — the error arm raises immediately, so nothing after it is observed, which
is exactly what the null-arm fixture's two-line trace measures (§4).

Only the **nullable** form is admitted. `resource_out(p, nonnull)` asserts the
written pointer is non-null with no checked branch, and is refused
(`NonnullOutParameter`) for the same stated reason `resource_return_owned(nonnull)`
is: keep every acquisition arm checked. That is a deliberate narrowing relative
to the card brief, which named only nonnull *owned returns* as staying refused;
admitting a nonnull out slot would have introduced the first unchecked
acquisition on this route.

A declaration establishes **at most one** fresh identity, so two out slots
(`MultipleAcquisitions`), an out slot beside an owned return
(`MultipleAcquisitions`) and a store beside either (`StoredWithAcquisition`) are
each refused by name. This keeps the adapter binding exactly one identity, which
is what the reference execution, the emission and the Lean step function all
assume.

### 1.2 `resource_stored(p, owner, nonnull)` — a permanent departure

C04.2: "Custody of `p` becomes indexed beneath the designated owner in the
abstract resource model." The caller-side delta is *exactly* a consumption: the
identity leaves the ledger and every later use is refused. What distinguishes a
store is **where** it went, and that is declared, recorded and premised — never
checked. Two premises are added to the certificate vocabulary:

| Assumption | Content |
|---|---|
| `StoredCustodyUnderDeclaredOwner(callee)` | The stored identity leaves this profile's accounting permanently. That the declared owner keeps, and eventually releases, it is asserted by the declaration and discharged nowhere in this repository. |
| `OutSlotWrittenByCallee(callee)` | That the callee actually writes its declared slot is the declaration's claim. The emitted program initializes the slot to `NULL` and compares the observed nullness against the declared control, which **refutes** a disagreeing implementation for the observed run and establishes nothing beyond it. |

Source spelling is `move NAME`, identical to a consumption. That is deliberate:
the caller-side ledger delta *is* a move, the destination comes from the
contract rather than the call site, and a second spelling would decorate a
distinction the source cannot verify. The distinction is not lost — it is
carried in the derived custody vector (`ExternCustody::Stored` versus
`Consumed`), in the certificate's `stored: Vec<(child, owner)>` field, in the
observation rendering (`s:` versus `c:`) and in the assumption above. A contract
mutation from `consumed` to `stored` therefore moves the published certificate
rather than passing silently.

### 1.3 The forced admission of `resource_owned`

The card brief specifies exactly two modes. Contract spec §C04.2's own
well-formedness rule forces a third, in one position only: a stored parameter's
owner **MUST** be a distinct parameter whose mode is `Owned`, and the loader
enforces that before any bridging runs. `resource_owned` is therefore admitted
**only** where some `resource_stored` parameter of the same contract designates
it — nonnull, on `ptr<mut, u8>`, bridged as a loan the caller keeps, which is
precisely what C04.2 says the mode means ("the caller supplies the required live
custody, and the call retains it in the modeled parameter disposition").

Every other `resource_owned` parameter keeps its existing `OwnedParameter`
refusal. The load-bearing negative half is pinned by
`an_owned_parameter_that_owns_nothing_still_refuses` (Rust) and
`an_owned_parameter_outside_owner_position_refuses` (Lean, stated for arbitrary
positions).

## 2. Bridging rows

Added to the closed table (`qcore_core::extern_call::bridge_contract`):

| C04 declaration | Bridged as | Source form |
|---|---|---|
| `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` | `CallParameter::OutSlot` | `out` at that position, under the mandatory adapter |
| `resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` | `CallParameter::Stored { owner }` | `move NAME` |
| `resource_owned(p, nonnull)` on `ptr<mut, u8>`, designated by some stored parameter | `CallParameter::Owner` | `NAME` (a live binding the caller keeps) |

Derived quantities, all functions of the admitted callee alone:

- **custody vector** — `Stored` and `Owner` join `Borrowed`/`Consumed`; an out
  slot carries no *caller* identity and is absent, so the vector stays aligned
  with the caller's argument identity list. The identity an out slot establishes
  is the adapter's, recorded separately in `ExternCallCertificate::out_acquisitions`.
- **interprocedural summary** — `Stored → ParameterOwnership::Consumed`,
  `Owner → Borrowed`; a callee that acquires through either source contributes
  the same `Flow::acquire`.
- **effect row** — `Alloc` is now derived for *either* acquisition source
  (`ExternCallee::acquires()`); `Mut` holds for a store, an owner and an out
  slot, all of which are `mut` pointers.

## 3. Refusal matrix

Every row is one named constructor, asserted by one Rust test and (where the
shape is expressible in the model) one Lean theorem. No catch-all.

| Refused shape | `BridgeRefusal` | Rust test | Lean theorem |
|---|---|---|---|
| `resource_out(_, nonnull)` | `NonnullOutParameter` | `a_nonnull_out_slot_is_not_callable` | `a_nonnull_out_parameter_refuses` |
| out slot on a flat pointer | `UnsupportedOutPointer` | `an_out_slot_on_a_flat_pointer_is_not_callable` | `an_out_parameter_on_other_than_the_pointer_to_pointer_projection_refuses` |
| out slot on a scalar | `ScalarCustody` | `an_out_slot_on_a_scalar_is_not_callable` | (covered by the above) |
| two out slots | `MultipleAcquisitions` | `two_out_slots_are_not_callable` | `two_out_slots_refuse` |
| out slot + owned return | `MultipleAcquisitions` | `an_out_slot_beside_an_owned_return_is_not_callable` | `an_out_slot_beside_an_owned_return_refuses` |
| `resource_stored(_, _, nullable)` | `NullableStoredParameter` | `a_nullable_store_is_not_callable` | `a_nullable_stored_parameter_refuses` |
| store on `ptr<const, u8>` | `UnsupportedStoredPointer` | `a_store_on_a_const_pointer_is_not_callable` | `a_stored_parameter_on_other_than_a_flat_mutable_pointer_refuses` |
| store beneath a non-`Owned` parameter | `StoredOwnerNotOwned` | `a_store_beneath_a_non_owned_parameter_is_not_callable` | `a_stored_parameter_beneath_a_non_owned_parameter_refuses` |
| store beneath itself | `StoredOwnerIsItself` | `a_store_beneath_itself_is_not_callable` | `a_stored_parameter_beneath_itself_refuses` |
| store + any acquisition | `StoredWithAcquisition` | `a_store_beside_an_owned_return_is_not_callable` | `a_stored_parameter_beside_an_owned_return_refuses` |
| nullable declared owner | `NullableOwnerParameter` | `a_nullable_declared_owner_is_not_callable` | `a_nullable_owner_refuses` |
| declared owner on `ptr<const, u8>` | `UnsupportedOwnerPointer` | `a_declared_owner_on_a_const_pointer_is_not_callable` | `an_owner_on_other_than_a_flat_mutable_pointer_refuses` |
| `resource_owned` owning nothing | `OwnedParameter` | `an_owned_parameter_that_owns_nothing_still_refuses`, `an_owned_parameter_is_not_callable` | `an_owned_parameter_outside_owner_position_refuses`, `owned_parameter_alone_refuses` |

Still excluded and **re-pinned unchanged**, each by its own name:
`resource_released` (`ReleasedParameter`), `resource_removed`
(`RemovedParameter`), `resource_return_reallocated` (`ReallocatedReturn`), a
nonnull owned return (`NonnullOwnedReturn`), a nullable consumed parameter
(`NullableConsumedParameter`), a custody mode on a scalar (`ScalarCustody`), a
borrowed `ptr<mut, ptr<mut, u8>>` (`UnsupportedBorrowedPointer`), a consumed
`ptr<const, u8>` (`UnsupportedConsumedPointer`), an owned non-pointer result
(`UnsupportedOwnedResultType`). Sentinel/errno adapters, buffer extents,
variadics, and concurrent or error-region contexts are untouched.

Call-site refusals added:

| Condition | Diagnostic | Where |
|---|---|---|
| a stored argument naming the same identity as its declared owner | `Q024` (`StoredUnderItself`) | `check_extern_program`, reachable from source |
| an acquiring call without the adapter — now including an out slot | `Q025` (`AdapterMissing`) | unchanged condition, widened subject |

**No new diagnostic code.** `Q024` already owns "argument form"; a stored
argument that names its own owner is an argument-form refusal, and §L21's row is
extended to say so in the same change. `Q025` already owns the adapter
discipline; the change is that "the mandatory nullable-owned-return adapter"
becomes "the mandatory acquisition adapter", covering both admitted sources.
`Q020`/`Q021` are untouched, `Q022`, `Q023`, `Q026` and `Q027` keep their exact
conditions, and `docs/QCORE_SPEC_r0_3.md` §L14 and §L21 plus
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d are updated in this same change.

One new reserved word: **`out`**. It names a parameter position, never a
binding, so reserving it keeps a local from being read as the slot and the slot
from being read as an unresolved name (`Q013`). Fixture-collision audit: a
repository-wide search for `out` as an identifier in `.qcore`/`.qc` files finds
only comment text and the contract-module lifetime label `'out` in
`contract/qcore/monkey/composition.qcore`, which the apostrophe rule already
skips; the whole workspace test suite passes unchanged. A test pins that the
marker cannot be smuggled elsewhere: `out` at a declared loan is simply not a
live resource binding (`the_out_marker_is_not_an_argument_anywhere_else`).

## 4. Reference execution, emission and native evidence

Observation renderings now carry the *declared mode* each argument bridged to,
so a trace records which custody rule ran and not only which identity was
passed: `d:` data, `b:` borrowed, `c:` consumed, `s:` stored, `w:` declared
owner, `o:` out slot. An out call renders as `out CALLEE ARGS ok|null`, keeping
the outcome word last so both arms share a prefix.

Emission for an out call declares the local as `uint8_t *rN = NULL;`, passes
`&rN`, then runs the existing declared-outcome check. Because the slot starts
null, a callee that never wrote it is *observed* as the failure arm and refuted
by that check (exit 70) rather than believed — which is the runtime nullness
check the card asks for, sited where it can actually fail.

`ExternEmissionRefusal::UnboundIdentity` is added, fail-closed: the backend
declares a C variable per adapter-bound identity, so an argument naming an
identity no adapter established is refused instead of emitted as a translation
unit that only the C compiler could reject. The reference interpreter prints
identities symbolically and keeps no such requirement; the asymmetry is
deliberate and documented at the constructor.

**Native evidence.** `out_slot_fixtures_run_natively_against_a_linked_c_implementation`
(`compiler/tests/extern_calls.rs`, Unix targets) compiles the emitted C together
with `compiler/tests/fixtures/extern_calls/buffer_api.c` — real implementations
of `qcore_extern_open`, `qcore_extern_open_failing` and `qcore_extern_attach`
alongside the v1 callees — under
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined`
with `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and requires the native
observation to equal the reference execution **line for line**.

| Fixture | Native/reference observation |
|---|---|
| out, success arm | `out qcore_extern_open o:r1 d:8 ok` / `call qcore_extern_fill b:r1 d:7` / `call qcore_extern_sum b:r1 d:8` / `call qcore_extern_release c:r1` / `return` |
| out, null arm | `out qcore_extern_open_failing o:r1 d:8 null` / `raise Failure` |

The success arm allocates one real heap buffer through the slot, writes it
through a `mut` loan, reads it through a `const` loan and frees it through the
consuming callee. The null arm allocates nothing and — the card's "refused
unbound on the error arm" requirement, measured rather than asserted — observes
**none** of the three statements that follow, all of which read the out binding.
The v1 fixtures' two traces still run unchanged in the same target.

Scope of that evidence: one Linux x86-64 target, one compiler invocation, two
processes, two traces and this implementation. It is not evidence about the
declared contracts in general, about buffer extents, about any other callee, or
about a callee that writes its slot differently — that last case is exactly what
the emitted status-70 check refuses to hide.

## 5. The store's measured reachability boundary

`compiler/tests/fixtures/extern_calls/stored.qcore` acquires an owner and a
child and attaches one to the other. It is refused, and the refusal is
**`Q025`** — the adapter rule at the second acquisition — not `Q024`. That is
the whole finding, asserted exactly:

```rust
assert_eq!(codes, ["Q025"], "the blocker is the adapter rule, not the store");
```

The store's own admission and argument bridging *are* reachable from source,
because a self-store needs only one live identity:
`qcore_extern_attach(owner, move owner)` reaches `CallParameter::Owner` and
`CallParameter::Stored` at the call site and is refused by name (`Q024`,
`StoredUnderItself`), as is `qcore_extern_attach(move owner, move owner)`
(`Q024`, argument form at the owner position). The loader's own §C04.2 rule is
re-pinned at this boundary too: a selection whose owner is `resource_borrowed`
refuses the whole environment at `Q023`.

Everything else about the store is checked and proved at the model level:
admission (§3), the ledger departure, the certificate's `stored` pairs, the
owner surviving the store, the reference-execution rendering, and the
emission-side `UnboundIdentity` refusal that makes the C-level gap structural
rather than latent. The core tests construct those traces directly, which is how
`qcore-core`'s unit tests already work.

Not repaired here, on purpose: lifting the single-live-acquisition restriction
needs a way to release custody on an extern error arm, which needs a per-callee
cleanup relation this profile does not have. It is already the named
`EXTERN-CALL-F` follow-on "releasing custody on an extern error arm, and
therefore more than one simultaneously live acquisition". Nothing in this card
weakens it — the pinned refusal `an_adapter_that_would_abandon_live_custody_refuses`
is byte-for-byte unchanged.

## 6. Lean inventory

`proof/QCore/ExternCall.lean` grows from 27 to **48 public theorems**, audited by
`proof/AuditExternCall.lean` (53 `#print axioms` lines, including five from
`QCore.ModuleImports`). No new axiom; every theorem depends only on `propext`
and `Quot.sound`. The audit set stays at 48 files.

Structural change: admission is now contract-level rather than per-parameter,
because owner position is a property of the whole declaration
(`bridgeParamAt ps i`, indexed, with `isDesignatedOwner`), plus two
contract-level rules (`outCount ≤ 1`, no store beside an acquisition).
`Callee` gains an `outSlot` field and `acquires` is the disjunction of the two
sources.

| Obligation | Theorems |
|---|---|
| conservation for the out mode — an acquisition on success only | `an_out_write_is_an_acquisition_only_on_success`, `a_failed_acquisition_binds_nothing`, `a_successful_acquisition_binds_exactly_the_fresh_identity` |
| conservation for the store mode — a consumption with no return | `a_store_binds_no_identity`, `a_stored_identity_does_not_survive`, `the_owner_survives_a_store`, `a_departing_identity_does_not_survive` |
| refusal completeness, still-excluded modes re-pinned | the nine v1 refusal theorems, re-proved under the new `bridge` |
| refusal completeness, the two new modes' excluded shapes | the nine out/store/owner refusal theorems of §3 |
| non-vacuity | `the_admitted_table_is_inhabited`, `the_out_table_is_inhabited`, `the_store_table_is_inhabited` |
| derivation is a function | `admission_is_deterministic`, `an_out_slot_carries_no_caller_custody`, `a_store_and_its_owner_both_carry_custody` |
| derived effect row | `alloc_is_derived_exactly_for_an_acquisition`, `alloc_is_derived_for_an_out_slot`, `a_row_without_alloc_refuses_an_out_writing_call`, plus the four v1 rows |
| adapter totality — no third exit | `adapter_totality` (restated over `acquires`) |

**Deliberate pin updates**, each recorded rather than silently dropped:

1. `out_parameter_refuses` and `stored_parameter_refuses` are **retired by
   name**: they asserted that *every* out/stored declaration refuses, which this
   card makes false. Each is replaced by the narrower refusals in §3, and the
   audit file says so at the point of removal.
2. `owned_parameter_refuses` is replaced by
   `an_owned_parameter_outside_owner_position_refuses` (general, over arbitrary
   positions) and `owned_parameter_alone_refuses` (a `bridge`-level instance).
   The refusal is narrowed exactly to owner position, not removed.
3. `alloc_is_derived_exactly_for_an_owned_return` becomes
   `alloc_is_derived_exactly_for_an_acquisition`, an iff over both sources.
4. `a_data_call_never_binds_an_identity` and
   `a_data_call_never_takes_the_error_arm` gain the hypothesis
   `k.outSlot = false`. Without it they are false, because a data-result callee
   with an out slot does acquire. Adding the hypothesis is the honest repair;
   dropping the theorems would have lost the claim.
5. `a_consumed_identity_does_not_survive` keeps its name and statement and is
   now a corollary of the shared departure rule.

Premises stated in the theorems and discharged nowhere: that a callee's C
implementation satisfies its declared contract, that the Rust extraction agrees
with the Lean transcription, and — new — that a stored identity's declared owner
keeps and eventually releases it.

## 7. Test matrix

**`crates/qcore-core/tests/unit/extern_call.rs` — 67 tests** (37 before; two
obsolete admission rows removed, 32 added). New: two positive bridging rows for
the out and store shapes; twelve admission-refusal rows (§3); the owner-owns-
nothing control; two derivation rows; out-slot call-site checking (acceptance
with certificate rederivation, out-source recording, adapter-missing,
double-bind, both arms and the live-custody control, argument form); store
call-site checking (acceptance with the `stored` pairs and assumption, the
ledger departure with its no-store control, store-beneath-itself, both argument
forms); reference execution of both out arms and of a store; out emission
content; and the unbound-identity emission refusal.

**`compiler/tests/extern_calls.rs` — 47 tests** (34 before, 13 added). New: the
out round trip with certificate rederivation; out emission content and reference
agreement; the null arm binding and reading nothing; the two-fixture native
ASan/UBSan/LSan run; four out refusals (`nonnull`, adapter-missing, a binding in
slot position, the marker elsewhere); a mutated out contract; and four store
rows (the `Q025` reachability assertion, self-store, owner argument form, and
the loader's owner rule).

Contract fixtures stay under `compiler/tests/fixtures/extern_calls/contracts/`,
never `contract/qcore/`, so the 60-module corpus pin and the loader's rejected
count are untouched.

## 8. Gate

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1931 passed, 0 failed, 4 ignored** (1888 + 43 new), exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **48 Lean audits** pass |
| `providers/` workspace | **75 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **125 passed** |
| Native ASan/UBSan/LSan | 4 fixtures (2 v1 + 2 out), all pass, observations equal |

Reported honestly: three of the recorded load-flaky tests were observed failing
during interleaved runs on this host and each passed on a rerun, with no
assertion weakened. `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (a
wall-clock bound, `docs/reports/remaining-p03/README.md` §9.1) failed while its
19 siblings ran in parallel and passed 3/3 alone;
`descendants_do_not_outlive_the_execution` failed once with
`Spawn("Text file busy")`, a fixture write/exec race; and the providers'
`configured_second_codebase_has_real_process_checked_source_evidence` failed once
with `QP006 provider timed out`. None of the three touches any file in this
diff, and the final clean runs above have zero failures.

**Corpus artifacts are untouched by construction**, asserted from the diff and
not by re-measurement, exactly as the v1 card's §8 requires:
`crates/qcore-c-gateway/`, `crates/qcore-audit/` and `crates/qcore-schemas/` do
not appear in `git diff --name-only`, and neither do
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json` or
`docs/reviews/r0_3/acceptance_matrix.json`. `contract/qcore/` is untouched, so
the corpus stays at 60 modules and the loader's rejected count stays at 145. No
raw unit is promoted, no CT case is mapped, `qcore-schemas` stays 8.0.0 and no
wire version moved.

## 9. Remaining `EXTERN-CALL-F` follow-ons, restated

Unchanged by this card except where noted:

- inline source-contract enforcement (the `Q020`/`Q021` consumer);
- the `resource_removed`, `resource_return_reallocated` and `resource_released`
  call modes, and nonnull owned returns — and now also **nonnull out slots**,
  refused here for the same stated reason;
- **releasing custody on an extern error arm**, and therefore more than one
  simultaneously live acquisition. This is now also the sole blocker on
  reaching `resource_stored` from source (§5), which raises its priority
  without changing its content;
- sentinel and `errno` adapters (interacts with O03's post-state work);
- buffer extents; variadics, function pointers, callbacks, structs, floats;
- extern calls in concurrent or error-region contexts, and in more than one
  source function per module;
- cost summaries; linkage verification;
- a `qcorec` selection flag — the CLI still selects no extern contract
  environment, so a source file calling an extern function is refused at `Q023`
  through that entry point;
- any gateway, audit-route or Cycle 19 interaction (untouched by construction).

New, from this card:

- **EXTERN-F1-STORE-REACH** — a source-reachable `resource_stored` fixture, and
  with it native evidence for the store mode. Blocked on the error-arm release
  above; the admission, checking, certificate content and proofs are in place
  and waiting for it.
- **EXTERN-F1-OUT-MULTI** — more than one out slot per declaration, and an out
  slot beside an owned return. Both are refused here to keep the adapter binding
  exactly one identity; admitting them needs a multi-binding adapter form.

## 10. Files

New: `compiler/tests/fixtures/extern_calls/{out_slot.qcore,stored.qcore}`, this
report.

Changed: `crates/qcore-core/src/{extern_call.rs,extern_c_emit.rs}`,
`crates/qcore-core/tests/unit/extern_call.rs`,
`crates/qcore-frontend/src/{source_extern.rs,frontend.rs}`,
`compiler/tests/extern_calls.rs`,
`compiler/tests/fixtures/extern_calls/{buffer_api.c,contracts/buffer_api.qcore}`,
`proof/QCore/ExternCall.lean`, `proof/AuditExternCall.lean`,
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`.

`crates/qcore-contract/src/extern_bridge.rs` needed **no change**: it already
mapped the complete C04 parameter vocabulary into the model's typed view, so the
two modes reached `bridge_contract` unaltered and the widening is confined to
the admission table itself.
