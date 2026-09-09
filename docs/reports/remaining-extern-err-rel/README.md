# EXTERN-ERR-REL — the extern adapter's error-arm release

Card record for `EXTERN-ERR-REL`, the follow-on named by
[`docs/reports/extern-call-v1/README.md`](../extern-call-v1/README.md) §8
("releasing custody on an extern error arm, and therefore more than one live
identity") and re-named by
[`docs/reports/extern-call-f1/README.md`](../extern-call-f1/README.md) §9 as the
blocker under `EXTERN-F1-STORE-REACH`. Date: 2026-09-09. Wave 12.
Base: `67961f1`.

**What this increment establishes.** An acquiring extern call's error arm may
now carry explicit dispositions of the other live checked acquisitions, so a
raise no longer requires an empty ledger. With that admitted, the
`resource_stored` mode `EXTERN-F1` measured as *checked but unreachable from a
single source module* is reachable: two source fixtures compile, check, emit,
link against a real C implementation and run under fixed ASan/UBSan/LSan with
every allocation accounted, on both the success path and the
store-then-callee-fails path.

**What it does not establish.** Nothing about any C implementation: an admitted
contract is `ContractTrust::Declared` (contract spec §C06) and every verdict is
valid only relative to it. In particular the store's own premise —
`ExternAssumption::StoredCustodyUnderDeclaredOwner`, that the declared owner
keeps and eventually releases the child — is *satisfied by the fixture's C for
its own runs* and is discharged nowhere. Nothing is promoted: no Cycle 19 raw
unit changes, no CT case is mapped, the contract corpus is unchanged at 60
modules and full language conformance stays false.

## 1. The discipline, and why it is not a parallel rule

The card brief asked for the error arm to compose with the existing
typed-error/cleanup discipline rather than invent one. Reading that discipline
is what fixed the design.

`qcore_core::cleanup_lowering::lower_cleanup_cfg` gives every potentially
failing operation an edge-specific cleanup block, and
`cleanup_block(program, id, error, live)` emits, verbatim:

```rust
let instructions = live.iter().rev().copied().map(CfgInstruction::Release).collect();
```

So the sequential lowering **already** accounts for every identity live before a
failing acquisition: it releases them all, in reverse acquisition order, on that
acquisition's cleanup edge. The extern profile's own lowering has produced that
edge since `source.extern-call.v1` landed — `SourceOp::Acquire { succeeds, error }`
is what `lower_source_extern` emits for an adapter.

What the profile lacked was a **rendering** of that edge. An extern-acquired
identity is not released by a backend instruction: it is released by calling a
declared `resource_consumed` callee, and this profile cannot invent one. With no
way to emit the releases, `EXTERN-F1`'s checker refused any arm that would have
needed them (`AdapterAbandonsCustody`), which is the conservative and correct
thing to have done.

The discipline this card lands is therefore:

> Between `else` and `raise`, the handler names the calls that acquisition's
> cleanup edge performs — one per still-live identity — and admission requires
> the arm to **be** that edge: the same identities, each released exactly once,
> in reverse acquisition order.

The source states what the machinery does, and disagreement is refused rather
than reconciled. Concretely (`compiler/tests/fixtures/extern_calls/stored.qcore`):

```qcore
fn main() -> unit effects { io, alloc, mut } {
    let owner = qcore_extern_alloc(16) succeeds true else raise Failure;
    let child = qcore_extern_alloc(8) succeeds true
        else qcore_extern_release(move owner) raise Failure;
    qcore_extern_attach(owner, move child);
    qcore_extern_release_owner(move owner);
    return;
}
```

Grammar delta (`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d, updated in this
change):

```ebnf
acquisition   = "let", NAME, "=", CALLEE, "(", [ arg, { ",", arg } ], ")",
                "succeeds", BOOL, "else", { disposition },
                "raise", ERROR_NAME, ";" ;
disposition   = CALLEE, "(", [ arg, { ",", arg } ], ")" ;
```

Zero dispositions is the v1 shape, spelled exactly as before, and it still means
what it meant: an arm that abandons nothing because nothing else is live.

### 1.1 What "carried into the error path" turned out to be

The brief offered a second disposition — carrying an identity into the error
path where the error-region machinery accounts for it. That was measured and is
**not** implemented, for a stated reason rather than by omission:

- `qcore_core::error_region::RegionOp::MoveToErrorPayload` exists and does carry
  a resource into an error payload, but it belongs to the error-region model,
  which this route does not compose: `lower_source_extern` sets
  `region_operations: Vec::new()`, and "extern calls in concurrent or
  error-region contexts" is an explicitly out-of-scope follow-on of the v1
  design (§7).
- The route this profile *does* lower onto has no payload carrier at all. Its
  error exit is `CfgTerminator::ReturnError(ErrorTag)` and its observation is
  `ExecutionOutcome::Error { name, tag }` — no payload field, in either. Adding
  one is a change to the sequential CFG, the error ABI, the C backend and every
  pin that reads them, which is not this card.

So on this route "carried into the error path" and "released by the cleanup
edge" are the same thing, and the cleanup edge releases. That is what the
implemented disposition renders. The payload form is recorded as a follow-on in
§8 with the two blockers named.

## 2. The narrowing, stated

`ExternDiagnostic::AdapterAbandonsCustody` keeps its code (`Q025`), its
constructor, its `{ event, resource }` shape and its subject. What changed is
**when** it fires:

| | `EXTERN-F1` | after this card |
|---|---|---|
| some other identity is live at the raise | refuse | refuse, unless the arm releases it |
| the arm releases it | not expressible | admitted, and required to be the cleanup edge |

That is a narrowing: strictly fewer programs are refused *by this constructor*,
and no program that was refused for a reason other than "the arm could not
release" becomes admitted. The two load-bearing pins are unchanged and still
pass byte-for-byte:

- `crates/qcore-core/tests/unit/extern_call.rs::an_adapter_that_would_abandon_live_custody_refuses`
  (empty arm, one live identity, exact diagnostic vector) — untouched except for
  the mechanical `releases: Vec::new()` field the struct gained.
- `compiler/tests/extern_calls.rs::an_adapter_that_would_abandon_live_custody_refuses`
  (end-to-end, `Q025`) — untouched.

Two further tests pin the narrowing itself in the refusing direction:
`an_empty_arm_with_live_custody_still_abandons` (core) and
`an_acquisition_whose_arm_disposes_of_nothing_still_refuses` (end-to-end), both
built by *emptying* the arm of the now-admitted fixture, so a future widening
that quietly drops the requirement fails them.

## 3. What the arm may contain, and every refusal by name

`check_error_arm` in `crates/qcore-core/src/extern_call.rs` simulates the arm
over the identities live at the acquisition. Each refusal is its own
constructor; none is a catch-all.

| Condition | Diagnostic | Code |
|---|---|---|
| an identity reaches the raise still live | `AdapterAbandonsCustody` | `Q025` |
| the arm calls a callee with no admitted contract | `ReleaseUnknownCallee` | `Q023` |
| wrong argument count | `ReleaseArity` | `Q024` |
| argument does not bridge to its declared mode | `ReleaseArgumentForm` | `Q024` |
| the disposition itself acquires | `ReleaseAcquires` | `Q025` |
| the disposition stores beneath a declared owner | `ReleaseStores` | `Q025` |
| the call discharges other than exactly one identity | `ReleaseNotSingular` | `Q025` |
| it names an identity not live on the arm | `ReleaseNotLive` | `Q025` |
| the order is not the cleanup edge's | `ReleaseOrder` | `Q025` |
| the acquiring call also moves custody out | `AdapterDepartingArgument` | `Q025` |

No diagnostic code is added or changed: everything lands inside the existing
`Q022`–`Q027` band of the `ExternCall` pass, so no §L21 allocation was needed.
`docs/QCORE_SPEC_r0_3.md` §L21's `Q022`–`Q027` row and
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d are updated in this same change
to enumerate the widened conditions.

Three of those rows deserve their reasoning recorded.

**`ReleaseAcquires`.** An arm that acquires opens a fresh obligation on a path
taken because something already failed, and it has no checked adapter of its own
— every acquisition arm in this profile stays checked, which is the same rule
that keeps `resource_return_owned(nonnull)` and `resource_out(_, nonnull)`
refused.

**`ReleaseStores`.** A store converts an obligation into a declared premise
(`StoredCustodyUnderDeclaredOwner`). Doing that on a failure path would let a
module discharge its custody by asserting rather than by releasing, so it is
refused.

**`AdapterDepartingArgument`.** A contract's clauses are `ensures normal`: they
say what holds on the callee's *normal* exit. An acquiring call that also
carries a `resource_consumed` or `resource_stored` argument therefore leaves
that argument's status undecided on the failure arm — the arm can neither
release it (the callee may already have) nor leave it (the callee may not have).
This shape was already refused under `EXTERN-F1`, as abandonment; it is now
refused by its own name, with the reason in the message, and
`an_acquiring_call_that_also_consumes_refuses` pins it over a real
consume-and-return-owned contract the bridging table admits.

**The order rule.** The arm is required to run in reverse acquisition order
because that is the order `cleanup_block` releases in. An arm in another order
would mean the extern route's native rendering and the sequential route's
cleanup CFG describe different programs for the same module. Both directions are
pinned: `an_arm_in_acquisition_order_refuses_and_reverse_order_is_admitted`
(core) and the end-to-end pair
`an_arm_whose_order_disagrees_with_the_cleanup_edge_refuses` /
`an_arm_in_reverse_acquisition_order_is_admitted`, so the refusal is about the
order and not about arms of length two.

**The arm is a path, not a place.** A disposition never removes a binding from
the success path. `Lowering::arguments` gained an explicit `consume: bool` and
the arm resolves with `consume = false`, reading the bindings live at the
attempt without deleting them; that is exactly why the store two statements
later can still name its owner.
`a_disposition_does_not_consume_the_identity_on_the_success_arm` and
`a_disposition_is_not_a_consumption_on_the_success_path` pin it at both levels,
by asserting the owner is consumed **once**, on the normal trace.

## 4. STORE-REACH, closed

`EXTERN-F1` §5 recorded: "`resource_stored` is admitted by the bridging table
and checked by the model, but is not reachable from a single source module …
the fixture refuses at `Q025` on its **second acquisition**, never at `Q024` on
the contract."

That refusal is gone because its cause is gone, and the closure is evidenced at
four levels.

1. **Checker.** `compiler/tests/fixtures/extern_calls/stored.qcore` compiles.
   `a_store_is_reachable_once_the_error_arm_disposes_of_the_other_identity`
   asserts the whole certificate content: `acquisitions == [1, 2]`,
   `stored == [(2, 1)]` (child 2 beneath owner 1), `consumptions == [1]`, and
   `error_dispositions == [(1, []), (2, [1])]` — the first acquisition abandons
   nothing because nothing else is live, the second disposes of exactly the
   owner, and the certificate keeps both so a reader can tell an empty arm from
   an absent one. The `StoredCustodyUnderDeclaredOwner` premise is present on
   the certificate, and the driver's independent recheck reproduces the whole
   thing.

2. **Reference execution**, both arms. Success:

   ```
   acquire qcore_extern_alloc r1 ok
   acquire qcore_extern_alloc r2 ok
   call qcore_extern_attach w:r1 s:r2
   call qcore_extern_release_owner c:r1
   return
   ```

   Failure (`stored_failure.qcore`, the store-then-callee-fails path):

   ```
   acquire qcore_extern_alloc r1 ok
   acquire qcore_extern_alloc r2 ok
   call qcore_extern_attach w:r1 s:r2
   acquire qcore_extern_alloc_failing r3 null
   call qcore_extern_release_owner c:r1
   raise Failure
   ```

   The disposition is observed as the call it is, before the raise ends the
   trace; nothing after the raise is observed.

3. **Native, sanitized.** Both fixtures are added to
   `accepted_fixtures_run_natively_against_a_linked_c_implementation`, which
   emits the C11, compiles it with `-std=c11 -Wall -Wextra -Werror -pedantic
   -fsanitize=address,undefined` against the real `buffer_api.c`, runs it under
   `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
   `UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and requires the native
   observation to equal the reference **line for line**. Four labelled runs now
   pass on this route: `success`, `null-arm`, `store`, `store-failure`.

   Allocation accounting, both store runs: two `calloc`s (owner 16 bytes, child
   8 bytes), two `free`s, LSan clean. On the success path
   `qcore_extern_release_owner` frees the child it reads out of the owner and
   then the owner; on the failure path the same call runs inside the emitted
   failure branch. The emitted branch is asserted verbatim, both in the core
   (`emission_puts_the_arms_releases_inside_the_failure_branch`, matching
   `"    if (r2 == NULL) {\n        qcore_extern_release(r1);\n"`) and
   end-to-end.

4. **Proof.** `the_store_is_reachable_from_two_live_identities` states both
   halves at once: the store contract bridges to the attach callee, and stepping
   that callee over the two-identity ledger consumes the child and keeps the
   owner. Its input ledger is produced by
   `a_second_acquisition_is_admitted_when_its_arm_releases_the_first`, and
   `the_same_acquisition_with_an_empty_arm_refuses` reproduces `EXTERN-F1`'s
   boundary in the same file rather than deleting it.

### 4.1 The one honest caveat about the native store runs

For LSan to account for both allocations, some C function has to free the child.
The fixture supplies `qcore_extern_release_owner`, declared with exactly the
same clauses as the existing `qcore_extern_release`
(`resource_consumed(owner, nonnull)`, `resource_return_none()`), whose C body
additionally frees the pointer `qcore_extern_attach` stored in the owner's first
bytes.

That is the fixture **satisfying** `StoredCustodyUnderDeclaredOwner` for its own
runs. It is not the premise being discharged: no clause in the contract module
says the owner releases the child, nothing in the checker checks it, and a
different implementation of the same declaration could leak the child without
contradicting anything this repository verified. The premise remains named on
every certificate this route publishes. Both the contract comment and the C
comment say so at the site.

## 5. Certificate and rederivation

`ExternCallCertificate` gains one field:

```rust
/// Per acquisition, in trace order: the identity the adapter binds and the
/// identities its error arm releases before the raise, in the order it
/// releases them.
pub error_dispositions: Vec<(ResourceId, Vec<ResourceId>)>,
```

An empty inner vector is recorded rather than omitted, so an arm that abandons
nothing is distinguishable from an arm that was never examined. The arm's calls
also join the certificate's `effects` union — a release the error arm performs
is a call this module makes, and the frontend independently requires the entry's
declared row to admit each arm callee's derived row
(`an_arm_call_outside_the_declared_effect_row_refuses`).

Substitution refusals, both levels: `a_certificate_that_hides_a_disposition_fails_its_recheck`
(core, exact `ExternRecheckFailure::Mismatch`) and
`substituting_the_recorded_dispositions_fails_the_recheck` (end-to-end, through
the driver's own recheck). The driver already refused to publish a certificate
it could not rederive; that path is unchanged and now covers the new field.

## 6. Lean

`proof/QCore/ExternCall.lean` grows from 48 to **67 audited theorems**: 19 new
theorems, every one of them named in `AuditExternCall.lean`. No new axiom, no
placeholder; the whole file depends only on
`propext`, `Classical.choice` and `Quot.sound` as `AuditAll.lean` requires.

New model: `Disposition`, `armReleases`, `isRelease`, `armAdmits` and
`stepAdapter`. `armAdmits live ds` is `ds.all isRelease && armReleases ds = live.reverse`
— equality with the reversed ledger *is* the cleanup edge, stated once.

Theorems, by what they establish:

- **Conservation, both directions.** `error_arm_conservation` (under an admitted
  arm every live identity is released) and `an_abandoned_identity_refuses` (an
  identity the arm does not release refuses). `an_admitted_error_exit_is_empty`
  reads the same fact on the error exit.
- **The narrowing, named.** `an_empty_arm_with_live_custody_refuses` is the v1
  refusal kept, and `an_empty_arm_with_nothing_live_is_admitted` is the v1 shape
  kept admitted.
- **A disposition is a release and nothing else.**
  `an_acquiring_disposition_refuses`, `a_storing_disposition_refuses`.
- **Exactly once, in the edge's order.** `an_arm_of_the_wrong_length_refuses`
  with `a_repeated_disposition_refuses` as its double-release corollary,
  `a_disposition_of_a_departed_identity_refuses`,
  `an_arm_in_the_wrong_order_refuses` and the concrete two-identity
  `two_dispositions_must_run_in_reverse_acquisition_order`, which shows the same
  pair admitted in reverse order and refused in acquisition order.
- **The arm is a path.** `the_success_arm_keeps_every_dispositioned_identity`,
  and `adapter_with_arm_totality` — an admitted acquisition still has exactly
  two exits and a refused arm has none rather than a third.
- **STORE-REACH non-vacuity.** `the_release_callee_is_a_disposition`,
  `the_attach_callee_is_not_a_disposition`,
  `a_second_acquisition_is_admitted_when_its_arm_releases_the_first`,
  `the_same_acquisition_with_an_empty_arm_refuses`,
  `the_store_is_reachable_from_two_live_identities`.

No existing theorem was retired or restated. The Rust-to-Lean transcription
remains an explicit premise, exactly as `EXTERN-F1` recorded it.

## 7. Validation

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 2150 passed, 0 failed, 4 ignored (was 2119 / 4 at `67961f1`) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | 53 audits passed |
| `providers/` `cargo test --workspace` | 76 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 125 passed |

Per-suite deltas: `compiler/tests/extern_calls.rs` 47 → 61 tests,
`crates/qcore-core/tests/unit/extern_call.rs` 46 → 63 tests.

**Flake observed and not weakened.** `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs:794`) failed once during this
card's runs on `assert!(elapsed < Duration::from_millis(500))`, an 8 MiB-payload
timing assertion. Measured in isolation on this host: 2 passes, 1 failure in
three consecutive runs, with no file this card touches on its path. This is the
load-flaky `vp04` already named in the wave brief; the assertion is unchanged.
The recorded full-workspace run above passed it.

**Corpus untouched by construction**, asserted rather than argued:
`git diff --stat -- contract docs/reevaluation docs/reviews docs/reports`
against the base is empty, so the contract corpus (60 modules), the CT manifest,
`acceptance.json`, `acceptance_matrix.json` and every pinned audit artifact are
byte-identical. `qcore-schemas` is untouched at 8.0.0 — this card added no
schema type; `ExternRelease` and the certificate field live in `qcore-core`,
which no wire format reads.

Files changed: `crates/qcore-core/src/{extern_call.rs,extern_c_emit.rs}`,
`crates/qcore-core/tests/unit/extern_call.rs`,
`crates/qcore-frontend/src/source_extern.rs`, `compiler/tests/extern_calls.rs`,
`compiler/tests/fixtures/extern_calls/{stored.qcore,buffer_api.c,contracts/buffer_api.qcore}`,
new `compiler/tests/fixtures/extern_calls/stored_failure.qcore`,
`proof/QCore/ExternCall.lean`, `proof/AuditExternCall.lean`,
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`,
`docs/design/EXTERN_CALL_SEMANTICS_V1.md`, this report.

No new reserved word. The arm's dispositions are ordinary call heads in a
position (between `else` and `raise`) that previously admitted nothing at all,
so the fixture-collision audit had nothing to find: no identifier anywhere in
the repository changes meaning, and `drop`/`carry`/`release` acquire no new
lexical status. `called_extern_names` was extended to scan that position so a
callee a module uses only to release custody still has its contract *required*
rather than reported missing later.

## 8. Open boundaries

Recorded, not repaired.

- **EXTERN-ERR-REL-PAYLOAD.** The error arm releases; it cannot transfer an
  identity into an error payload. Two named blockers, both measured (§1.1): the
  sequential lowering's error exit carries no payload
  (`CfgTerminator::ReturnError(ErrorTag)`, `ExecutionOutcome::Error { name, tag }`),
  and `RegionOp::MoveToErrorPayload` belongs to the error-region model this route
  does not compose. Lifting either is its own card.
- **EXTERN-ERR-REL-ARM-SHAPE.** An arm may contain only dispositions. A
  logging call, a branch, a nested acquisition or any other statement is
  refused. The single-discharge rule (`ReleaseNotSingular`) is deliberately
  strict: a callee that consumes two identities at once cannot be a disposition
  even though it would discharge both.
- **EXTERN-F1-OUT-MULTI** is unchanged and still open: one out slot per
  declaration, never beside an owned return.
- **The declared premises are unchanged and undischarged.**
  `DeclaredContract`, `NoExtentTransport`, `LinkageSuppliedByBuilder`,
  `OutSlotWrittenByCallee` and `StoredCustodyUnderDeclaredOwner` all still
  appear on every certificate this route publishes. §4.1 records precisely how
  far the store fixture's C goes and where that stops being evidence.
- **Rust/Lean/C refinement** remains a premise on all three edges, as
  `EXTERN-F1` recorded.
- **The rest of `EXTERN-CALL-F`** — inline source-contract enforcement, the
  remaining C04 call modes, sentinel/errno adapters, buffer extents, variadics,
  extern calls in concurrent or error-region contexts, cost summaries and
  linkage verification — is untouched by this card.
