# EXTERN-MODES — the released and reallocated call modes

**Card:** `EXTERN-MODES` (wave 17). Advances the `EXTERN-CALL-F` follow-on
family of `source.extern-call.v1`, unmoved since wave 12.

**Base:** main `8ec8402`. **Branch:** `card/extern-modes`, one commit
(`604b210`). Not pushed, not merged.

**What landed.** Two of the five open call modes are now callable, each at its
own measured boundary; the third is refused on a ground this card measured
rather than assumed; the fourth (the sentinel/errno adapter) is a recorded
design boundary this card did not take.

| §C04 mode | Before | After | Where |
|---|---|---|---|
| `resource_released(p, nonnull)` | refused, `BridgeRefusal::ReleasedParameter` | **callable** | §2 |
| `resource_released(p, nullable)` | refused (same) | refused, `NullableReleasedParameter` | §2.3 |
| `resource_return_reallocated(p, nullable)` over an `Owned` subject | refused, `ReallocatedReturn` | **callable** | §3 |
| `resource_return_reallocated(p, nonnull)` | refused (same) | refused, `NonnullReallocatedReturn` | §3.5 |
| `resource_removed(p, owner, n)` | refused, `RemovedParameter` | refused, same name, **measured ground** | §4 |
| sentinel/errno adapter (O03 interaction) | out of scope | out of scope, boundary recorded | §5 |

Nothing else in the profile changed. No diagnostic code was added, changed or
removed; no reserved word was added; the contract corpus under `contract/qcore/`
is untouched; `Q020`/`Q021` are untouched. The whole `crates/` diff is
`qcore-core::extern_call`, `qcore-core::extern_c_emit` and
`qcore-frontend::source_extern`.

---

## 1. Why these two, in this order

The mission's soundness order put `resource_released` first. Reading the two
specs together explains why the pair belongs in one card.

Contract spec §C04.2 draws a distinction the profile could not previously
express:

- `resource_consumed(p, n)` — "the call transfers the obligation away from the
  caller. Consumption is not a proof of destruction or endpoint release."
- `resource_released(p, n)` — "the trusted call contract **discharges** the
  resource obligation. A false declaration here can invalidate a safety
  conclusion."

Card `EXTERN-ERR-REL`'s own module documentation records the consequence, in its
own words: *"an extern-acquired identity is released only by calling a declared
`resource_consumed` callee and this profile cannot invent one."* The error arm
renders `cleanup_lowering`'s cleanup edge, and a cleanup edge **releases**. The
profile was spelling a release as a transfer because transfer was all it had.

That is not a soundness defect — the caller-side ledger delta of the two modes
is identical, so no leak accounting moved and none moves now. It is an
**expressiveness** defect with a published consequence: a reader of a
certificate could not tell which callees a leak-freedom reading actually
depends on. After this card they can (§2.2).

The second mode is here because of a boundary card `EXTERN-ERR-REL` named and
could not cross. `ExternDiagnostic::AdapterDepartingArgument` refuses an
acquiring call that also carries departing custody, with a stated reason: *"a
declaration says what holds on its normal exit only, so an acquiring call that
also carries departing custody leaves that custody's status undecided on the
failure arm."* Contract spec §C04.3 makes exactly one departing mode not
silent there:

> `resource_return_reallocated(p, n)` — The incoming `p` MUST be `Owned`. A null
> failure preserves that incoming custody; a successful replacement
> consumes/replaces the old resource exactly once and establishes the result
> identity.

So a reallocation is the one case where the refusal's own premise fails. It is
admitted **without weakening the refusal**: the subject stays on the error arm's
live list and must be dispositioned there like any other live identity, which is
the same abandonment rule applied to an identity an ordinary consumption would
have taken off the arm. §3.3 is the pin that proves the distinction is
load-bearing.

---

## 2. `resource_released(p, nonnull)`

### 2.1 Admission

`crates/qcore-core/src/extern_call.rs`. A `ptr<mut, u8>` parameter declared
`resource_released(p, nonnull)` bridges to `CallParameter::Released`. Three
refusals guard it, each with its own name:

- `ScalarCustody` — a custody mode on a scalar (unchanged rule);
- `NullableReleasedParameter` — §2.3;
- `UnsupportedReleasedPointer` — any pointer other than `ptr<mut, u8>`.

The blanket `BridgeRefusal::ReleasedParameter` is **retired by name**. Its
load-bearing halves survive as the two narrower refusals; the core unit test
`a_released_parameter_is_not_callable` is replaced by three tests
(`a_nullable_released_parameter_is_not_callable`,
`releasing_a_const_pointer_is_not_callable`,
`releasing_a_scalar_is_not_callable`) plus the admission test.

### 2.2 What changes in the published evidence

`ExternCallCertificate` gains `releases: Vec<ResourceId>`, kept apart from
`consumptions`, and the assumption vocabulary gains
`ExternAssumption::DeclaredRelease(name)` beside the existing
`DeclaredContract(name)`. The premise text quotes §C04.2's warning: this
profile checks that the caller supplied a live identity and stopped using it; it
never checks that the C implementation destroys anything.

Both directions are pinned in `compiler/tests/extern_calls.rs`
(`a_released_argument_is_recorded_apart_from_a_consumed_one`): the release
fixture records `releases == [1]` with empty `consumptions` and carries
`DeclaredRelease`; the unchanged v1 round trip records `consumptions == [1]`
with empty `releases` and carries **no** `DeclaredRelease`. A refactor that
collapsed the two would fail in both directions.

### 2.3 The reach limit, recorded not weakened

`resource_released(p, nullable)` is refused, mirroring the v1 rule for
`resource_consumed(p, nullable)`: a nullable release discharges nothing when the
modeled value is null (§C04.2), so admitting it would make the discharge
conditional on a value this profile does not model.

The cost is measured and named: **contract spec §C04's own minimal `release`
example** declares `resource_released(p, nullable)` and is therefore not
callable through this profile. Recorded as follow-on
**`EXTERN-MODES-NULLABLE-RELEASE`**. Admitting it soundly needs a checked
statement that no null actual reaches a released position — true by construction
of the current source subset (every `move NAME` argument resolves to a live
binding) but checked nowhere, and the v1 consumed rule is the precedent for not
believing a by-construction fact at a custody position. The refusal is pinned at
the source boundary by
`a_nullable_release_declaration_refuses_at_the_bridging_table`, which substitutes
one word in the contract fixture and asserts `["Q024"]`.

### 2.4 Source, emission, reference execution

No new syntax. `move NAME` at a declared release is spelled exactly as at a
declared consumption or store; which of the (now four) departing modes ran comes
from the declaration, never from the call site. **No reserved word was added**,
so the `succeeds` / `work`→`effort` compatibility-hazard procedure does not
apply to this card.

The emitted C is a direct call, identical in shape to a consumption. The
observation prefix is `f:` (freed), distinct from `c:` (consumed), so an
observation still records which custody rule ran. Reference execution and
emission are written against the model, not against each other's output.

---

## 3. `resource_return_reallocated(p, nullable)`

### 3.1 Admission

`resource_owned` was previously callable in exactly one designated position —
the owner of a `resource_stored` parameter, which §C04.2 forces. This card adds
the **second** designated position and no more: §C04.3 forces a reallocation
subject to be `Owned`, so `resource_owned` becomes callable there and nowhere
else. `an_owned_parameter_beside_a_reallocation_of_another_parameter_refuses`
pins that the first position did not widen: a second `Owned` parameter that
neither owns a store nor is the declared subject still refuses with the original
`BridgeRefusal::OwnedParameter`.

Six named refusals guard the mode:

| Refusal | Ground |
|---|---|
| `NonnullReallocatedReturn` | a trusted non-nullness assertion with no checked branch, which would also erase §C04.3's preserving failure arm |
| `ReallocSubjectOutOfRange` | the result names a parameter the declaration does not have |
| `ReallocSubjectNotOwned` | §C04.3's own well-formedness rule |
| `NullableReallocSubject` | an absent subject supplies no custody to replace exactly once |
| `UnsupportedReallocSubjectPointer` | a subject on other than `ptr<mut, u8>` |
| `UnsupportedReallocatedResultType` | a reallocated result on other than `ptr<mut, u8>` |

Exclusivity is unchanged in substance and extended in coverage: a reallocation
is a third acquisition source, so it refuses beside an out slot
(`MultipleAcquisitions`) and beside a store (`StoredWithAcquisition`). The
adapter still binds exactly one identity.

The blanket `BridgeRefusal::ReallocatedReturn` is **retired by name**. Two core
unit tests moved deliberately, each keeping its negative half: the shape that
used to refuse with `OwnedParameter` now refuses with `NonnullReallocatedReturn`
in its nonnull form, and the borrowed-subject shape now refuses with
`ReallocSubjectNotOwned`.

### 3.2 Ledger semantics

Both arms come straight from §C04.3.

- **Success.** The subject leaves the ledger and the result identity enters it,
  at the same step. The certificate records it as one
  `(replaced, established)` pair in `reallocations`, not as an unrelated
  consumption plus acquisition — a reader that saw those two separately could
  not tell a replacement from a coincidence.
- **Failure.** Nothing is replaced. `reallocations` is empty, the subject is
  still live, and the mandatory `succeeds BOOL else … raise ERROR` arm must
  dispose of it like any other live identity.

The frontend lowering places the subject's `SourceOp::Drop` **after** the
`SourceOp::Acquire` rather than before it, which is what makes the failure arm
correct: `cleanup_lowering` gives a failed acquisition the cleanup edge over
everything live at that point, and a failed acquisition never reaches an
operation recorded after it. Every other departing mode keeps its drop before
the acquire. The declared resource peak counts the replaced and established
identities as briefly coexisting; that is an upper bound and deliberately
conservative.

### 3.3 The pin that keeps it honest

`a_failed_reallocation_whose_arm_abandons_the_subject_refuses` deletes the
disposition from `realloc_failure.qcore` and asserts `Q025`. Had the checker
treated the subject as departed at the call — the way it treats a consumption —
that source would have checked and **the emitted program would have leaked**.
The core-level twin
`a_reallocation_whose_arm_abandons_its_preserved_subject_refuses` asserts the
exact `AdapterAbandonsCustody { resource: 1 }` vector.

The asymmetry is pinned in one test from both sides:
`a_departing_consumption_refuses_where_a_reallocation_subject_does_not` runs a
control that checks, an ordinary consumption beside an acquisition that refuses,
and the reallocation in the same position that checks with its arm releasing the
subject.

### 3.4 Source, emission, reference execution

Again no new syntax: `move NAME` at a declared subject. The `let` binder names
the established identity. Emission is
`uint8_t *r2 = qcore_extern_grow(r1, 32);` followed by the existing
declared-outcome check, so an implementation that stopped matching its declared
arm exits 70 rather than being believed. The observation line is
`realloc <callee> <established> g:<subject> … ok|null`, carrying both identities
and the arm — the two facts a reader needs.

### 3.5 What is premised, not proved

`ExternAssumption::DeclaredReallocation(name)` names both halves of §C04.3: that
a null result left the incoming custody intact, and that a non-null result
replaced it exactly once. Neither is checked. The emitted program compares the
observed nullness against the declared control, which refutes a disagreeing
implementation *for the observed run* and establishes nothing beyond it.

---

## 4. `resource_removed` — refused, on a measured ground

This card read the mode against the model rather than deferring it again.

A `resource_removed(p, owner, n)` call takes a child **out** of owner-indexed
custody and hands it to the caller as directly-held custody. To call it, the
caller must (a) supply the child pointer at `p` and (b) end up holding a caller
identity for it.

Neither is available, and the reason is a deliberate decision of card
`EXTERN-F1`: a stored identity leaves this profile's accounting **permanently**,
with the residual beneath the declared owner recorded as the premise
`ExternAssumption::StoredCustodyUnderDeclaredOwner` and **not** as a modeled
owner-to-child index. There is no expression in the admitted source grammar that
reads a child back out of an owner, and no structure in the model to read it out
of. Admitting removal would therefore mint a fresh caller identity out of a
premise — an unchecked acquisition, which is precisely what the mandatory
adapter exists to prevent.

Refusal retained under its existing name `BridgeRefusal::RemovedParameter`, with
the ground written into its documentation; pinned at the source boundary by
`a_removed_parameter_is_still_not_callable`. Follow-on
**`EXTERN-MODES-REMOVED-INDEX`**: admitting removal requires a modeled
owner-to-child index, which is a change to what `resource_stored` publishes, not
a change to this bridging table.

---

## 5. Sentinel/errno adapter — boundary recorded, not taken

The mission made this conditional on the released/reallocated work leaving room.
It did not, and the card records the boundary rather than a partial attempt.

What is known from the two specs and card O03:

- Contract spec §C04.4 is explicit that "non-null error sentinels such as
  `MAP_FAILED` and scalar failure sentinels such as `-1` are **not**
  representable as tracked endpoint acquisition outcomes by the existing
  nullability-only return mode", and that a broader profile "MUST model success
  tests, endpoint kinds, generations and failure residuals explicitly; it MUST
  NOT reinterpret v1 nullability silently."
- The current adapter's whole checkable content is the nullness comparison the
  emitted C performs. A sentinel adapter has nothing to compare against until
  the *contract vocabulary* can state the success test — which is a change to
  the resource-contract v1 loader, not to this bridging table.
- The O03 precedent (`O03-REACTOR`) is binding on how the errno half must work
  if taken: the classification must come from the pinned
  `qcore_core::os_endpoint` relation, never from declared text alone.

Recorded as **`EXTERN-MODES-SENTINEL`**, with those three constraints. This card
adds nothing that presumes an outcome for it.

---

## 6. Diagnostics and reserved words

**No change.** Every new refusal falls inside the existing `ExternCall` band and
its existing category:

- bridging-table refusals (§2.1, §3.1) surface as `Q024`, or as `Q023` when the
  loader refuses the whole selection first;
- adapter refusals (§3.3) surface as `Q025`;
- `Q020`/`Q021` are untouched, and this card is still not the inline
  source-contract consumer they are waiting for.

Spec §L21's `Q022`–`Q027` row is updated in the same change for two facts it
would otherwise state falsely: the adapter is now mandatory for **three**
acquisition sources, and the departing-argument refusal now has one measured
exception. No code's ownership, range or meaning moved.

**No reserved word was added.** `move NAME` covers all four departing modes.
This is the one design deviation worth naming explicitly: the v1 design sketched
distinct spellings per mode, and measurement showed the mode is a property of
the declaration, so distinguishing at the call site would have been redundant
syntax that could disagree with the contract.

---

## 7. Native evidence

Three new fixed ASan/UBSan/LSan fixtures join the existing four in
`accepted_fixtures_run_natively_against_a_linked_c_implementation`
(`compiler/tests/extern_calls.rs`), compiled `-std=c11 -Wall -Wextra -Werror
-pedantic -g -O0 -fsanitize=address,undefined` and run with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`,
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, each comparing the native
observation with the independently computed reference execution **line for
line**:

| Fixture | What the run measures |
|---|---|
| `released.qcore` | a callee declared `resource_released` whose C implementation is `free`; LSan accounts for the one allocation |
| `realloc.qcore` | a real `realloc` replacement: the old block is released by the callee exactly once, the result is released by the source |
| `realloc_failure.qcore` | the preserving arm — the callee returns null and leaves the block untouched, and the checked disposition frees it; **a double free here is what ASan would report** |

All seven fixtures pass, together with the two out-slot fixtures in the sibling
test. The C implementations live in
`compiler/tests/fixtures/extern_calls/buffer_api.c`; their agreement with the
declared contracts is exactly the premise the certificates record, and a leak
checker can refute it for these fixtures without establishing it in general.

---

## 8. Lean

`proof/QCore/ExternCall.lean` is extended and `proof/AuditExternCall.lean`
registers the new theorems. No new axiom; every theorem depends only on
`propext` / `Quot.sound` as before.

Model changes mirror the Rust: `CallParam` gains `released` and
`reallocSubject`, `CallResult` gains `reallocatedNullable`, `Callee` gains
`reallocSubject`, `bridgeParamAt` is indexed by the subject the result mode
designates, and `bridgeResult` carries §C04.3's well-formedness rule.

**Two theorems are retired by name**, each replaced by the narrower refusals
that survive:

- `released_parameter_refuses` → `nullable_released_parameter_refuses`,
  `releasing_other_than_a_flat_mutable_pointer_refuses`, plus
  `the_release_table_is_inhabited` for non-vacuity;
- `reallocated_return_refuses` → `nonnull_reallocated_return_refuses`,
  `a_reallocation_subject_out_of_range_refuses`,
  `a_reallocation_subject_that_is_not_owned_refuses`,
  `a_reallocated_result_on_other_than_a_flat_mutable_pointer_refuses`,
  `a_nullable_reallocation_subject_refuses`,
  `a_reallocation_subject_on_other_than_a_flat_mutable_pointer_refuses`, plus
  `the_reallocation_table_is_inhabited`.

`an_owned_parameter_outside_owner_position_refuses` and
`owned_parameter_alone_refuses` gain a hypothesis naming the second designated
position, which is the honest statement now that one exists.

New ledger content: `stepRealloc` with `reallocation_adapter_totality`,
`a_successful_reallocation_replaces_its_subject` (the subject is absent and the
established identity present in the resulting ledger),
`a_failed_reallocation_binds_nothing`,
`a_preserved_subject_is_released_by_the_arm` and its refusing direction
`a_failed_reallocation_whose_arm_ignores_the_subject_refuses`, plus
`a_departing_identity_is_absent_where_a_subject_is_present`, which states the
§3.3 asymmetry as one sentence about the two positions. Three `decide`d
round-trip theorems pin non-vacuity in both arms and the empty-arm refusal.

Premises unchanged and still undischarged: that a callee's C implementation
satisfies its declared contract, and that the Rust extraction agrees with the
Lean transcription.

---

## 9. Corpus neutrality

By construction, argued from the diff rather than re-measured, exactly as the
design's §8 acceptance gate directs.

The complete `crates/` diff is `qcore-core/src/extern_call.rs`,
`qcore-core/src/extern_c_emit.rs`, `qcore-core/tests/unit/extern_call.rs` and
`qcore-frontend/src/source_extern.rs`. No file under `crates/qcore-c-gateway`,
`crates/qcore-audit`, `crates/qcore-verification`, `crates/qcore-contract`,
`crates/qcore-verifier-host`, `crates/qcore-schemas` or `compiler/src` changed.
`contract/qcore/` is untouched, so the 67-module corpus pin and the loader's
rejected count are unchanged; the card's contract fixtures live under
`compiler/tests/fixtures/`.

The two touched `qcore-core` modules are reachable only from the extern-call
route: the audit never constructs an `ExternProgram`, and
`qcore_frontend::source_extern` runs only when `has_source_extern` is true.
Outside tests, the only non-route reference to either module is the
`compiler/src/lib.rs` re-export and `qcore-contract::extern_bridge`, which is
itself part of the route.

**Expectation for consolidation:** all pinned 185-unit artifacts byte-identical;
the canonical CMetrics and chunkio-core artifacts byte-identical; contract corpus
67 modules; CT coverage 32/50 unchanged; no verdict moves in either direction.
The prepared pinned build trees are not present in this worktree, so this is an
expectation for the integrator's measurement, not a measurement.

---

## 10. Deviations and findings by measurement

1. **No new reserved word, against the v1 sketch.** §6. The mode is a property of
   the declaration; a call-site spelling would be redundant syntax that could
   disagree with the contract.
2. **`resource_removed` is not merely "not yet done".** §4. The blocker is
   structural and inherited from `EXTERN-F1`'s deliberate decision to premise the
   stored residual rather than model an index. The refusal's documentation now
   says so.
3. **The v1 nonnull-only release rule costs the spec's own example.** §2.3. Named
   `EXTERN-MODES-NULLABLE-RELEASE` rather than resolved by weakening the rule.
4. **The departing-argument refusal has exactly one exception, and it is
   declared, not inferred.** §1, §3.3. `AdapterDepartingArgument`'s stated reason
   is that ordinary declarations are silent on the failure arm; §C04.3 is not
   silent. Admitting the exception required no change to the refusal.
5. **A test pin moved and is recorded.** `an_excluded_contract_mode_refuses_at_
   the_bridging_table` substituted `resource_released(buffer, nonnull)`, which is
   now admitted. Its negative half — an excluded C04 mode refuses at the source
   boundary under `Q024`, not somewhere later — is kept on the nullable consumed
   mode, and the admitting direction is pinned by the new tests.
6. **Lowering-order finding.** The reallocation subject's drop must sit after the
   acquire, not before it; the pre-existing name-list/custody-list zip in the
   lowering could not express that and was replaced by an explicit
   `(name, identity, mode)` triple. §3.2.

---

## 11. Validation

Run in this worktree at base `8ec8402`.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2,775 passed, 0 failed, 8 ignored** (171 targets) — see §11.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **72 audits pass**, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | **161 passed** |
| `providers/` workspace tests | **109 passed, 0 failed** |
| ASan/UBSan/LSan fixtures | 3 new + 4 existing, all pass (§7) |

27 new Rust tests; 28 new `#print axioms` lines in `AuditExternCall.lean` (98
total), against 2 theorems retired by name (§8).

**Environment note, reported honestly.** The shared disk on this host reached
100% during the wave (eight agent worktrees, 20–26 GB of Rust build output
each). The workspace test build was completed after `cargo clean` and with
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=false CARGO_PROFILE_TEST_DEBUG=false`
to fit; those settings affect local build artifacts only, not any result. The
`third_party/libevent` submodule was not initialised in this worktree and was
initialised per the standing gate's own instruction before the run.

### 11.1 The batch run's four failures, and why they are not this card's

The single `--no-fail-fast` batch recorded 2,771 passed / 4 failed. All four are
environment, and all four pass in isolation on the same tree:

| Target | Batch result | Cause | Isolated rerun |
|---|---|---|---|
| `qcore-compiler --test libevent_submodule` (1 test) | FAILED | the `third_party/libevent` submodule was not initialised in this fresh worktree; the pin gate correctly refused | 1 passed |
| `qcore-compiler --test profile_loader` (2 of 18) | FAILED | same missing submodule (declared pins against checkouts) | 18 passed |
| `qcore-verifier-host --test confined_process_execution` (1 of 12) | FAILED, `Broken pipe (os error 32)` | the host's disk was at 100% during the batch; this is the recorded load-flake class, and the assertion was **not** weakened | 12 passed |

The submodule was initialised with the standing gate's own
`git submodule update --init --recursive` after the batch had already run those
targets. Effective total: **2,775 passed, 0 failed, 8 ignored**.

None of the four touches the extern-call route. The card's own targets —
`qcore-core --lib extern_call` (104 passing, 576 filtered) and
`qcore-compiler --test extern_calls` (70 passing, including all seven native
sanitizer fixtures) — passed in every run.

---

## 12. Files

New:

- `compiler/tests/fixtures/extern_calls/released.qcore`
- `compiler/tests/fixtures/extern_calls/realloc.qcore`
- `compiler/tests/fixtures/extern_calls/realloc_failure.qcore`
- this report

Changed:

- `crates/qcore-core/src/extern_call.rs` — admission, custody, certificate,
  call-site checking, premises
- `crates/qcore-core/src/extern_c_emit.rs` — argument rendering, acquisition
  prefixes, prototypes
- `crates/qcore-core/tests/unit/extern_call.rs` — 2 pins moved, ~19 tests added
- `crates/qcore-frontend/src/source_extern.rs` — argument resolution, lowering
  order, summary mapping
- `compiler/tests/extern_calls.rs` — 1 pin moved, 9 tests added, 3 fixtures
  joined the native loop
- `compiler/tests/fixtures/extern_calls/contracts/buffer_api.qcore` — three new
  declarations
- `compiler/tests/fixtures/extern_calls/buffer_api.c` — three new
  implementations
- `docs/QCORE_SPEC_r0_3.md` §L21 — three acquisition sources; the one measured
  exception to the departing-argument refusal
- `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d — callable shapes, refusal
  list, the two measured grounds
- `docs/design/EXTERN_CALL_SEMANTICS_V1.md` §7 — "Landed since (2026-09-10)"
- `proof/QCore/ExternCall.lean`, `proof/AuditExternCall.lean`

---

## 13. Named follow-ons

- **`EXTERN-MODES-REMOVED-INDEX`** — `resource_removed` needs a modeled
  owner-to-child index, which is a change to what `resource_stored` publishes.
- **`EXTERN-MODES-NULLABLE-RELEASE`** — admitting the nullable release needs a
  checked no-null-actual statement at a custody position.
- **`EXTERN-MODES-SENTINEL`** — the sentinel/errno adapter needs contract
  vocabulary for the success test (§C04.4) and must take its errno
  classification from `qcore_core::os_endpoint` (O03-REACTOR precedent).
- Unchanged from the family's earlier records: buffer extents, variadics,
  function pointers, callbacks, structs, concurrent and error-region contexts,
  cost summaries, linkage verification, and the inline `Q020`/`Q021` consumer.

---

## 14. Draft increment paragraph

> Extern released and reallocated modes (EXTERN-MODES): two of the five open
> `source.extern-call.v1` call modes become callable, each at a measured
> boundary. `resource_released(p, nonnull)` is the mode the error arm was
> already describing — contract spec §C04.2 separates discharging an obligation
> from transferring it, and card `EXTERN-ERR-REL` recorded that the profile
> could spell only the transfer; the caller-side ledger delta is unchanged, so
> nothing about leak accounting moves, but a release is now published as a
> release and premised by name. `resource_return_reallocated(p, nullable)` over
> an `Owned` subject is the one departing argument an acquiring call may carry,
> because §C04.3 states its failure arm where ordinary declarations are silent;
> the subject therefore stays live on the error arm and must be dispositioned
> there, which is the unweakened abandonment rule and is pinned by a test whose
> deleted disposition would otherwise have emitted a leak. `resource_removed`
> stays refused on a measured ground — a removal names a child the profile
> stopped tracking when it was stored, and `EXTERN-F1` premised that residual
> rather than modelling an index — and the sentinel/errno adapter is recorded as
> a design boundary with its three constraints. No diagnostic code and no
> reserved word were added: `move NAME` spells all four departing modes and the
> declaration decides which ran. Two Lean theorems retired by name with their
> load-bearing halves kept; three new fixed ASan/UBSan/LSan fixtures link a real
> C `free` and a real `realloc`, including the preserving arm where a double
> free would be reported. Corpus-neutral by construction: the whole `crates/`
> diff is `qcore-core::extern_call`, `qcore-core::extern_c_emit` and
> `qcore-frontend::source_extern`. Evidence:
> `docs/reports/remaining-extern-modes/README.md`.
