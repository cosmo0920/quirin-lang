# T06: one owned Buffer in a typed worker-error payload

Baseline `c60fd91`, implemented at `0dfb839`, 2026-09-08. `ScopedErrorPayloadV1`
extends the checked finite task profile with one explicitly tagged error variant
carrying **exactly one owned Buffer** out of one child, through its completion
capsule and its creator's join. This is the failure-edge counterpart of the
owned-result transfer that `ScopedOwnedTasksV1` already checks: the predecessor
handler path (CT20) rethrows a *payload-free* `SpawnError`, and this card is the
increment that lets one resource ride the error instead.

Nothing here promotes any raw Cycle 19 unit, changes any diagnostic code, or
changes any C-gateway behaviour.

## 1. The admitted profile

A declaration names the payload; nothing else can.

```qcore
error Recovery { Failed(own<Buffer>); }
```

* A payload variant is spelled exactly `VARIANT(own<Buffer>);`. A payload-free
  variant keeps its existing `VARIANT;` spelling.
* A declared error row is **uniform**: either every variant carries a payload or
  none does. A joined error therefore always holds exactly one Buffer and the
  parent's take is total. A mixed row is a represented refusal (§3), not a
  silent widening — with a mixed row the parent's take would succeed on one
  variant and be refused on another, and this profile does not represent the
  variant discrimination that would be needed to write such a handler.
* `AllocError` and `SpawnError` can never appear in a payload row: the runtime
  raises them after a failure that acquired nothing.
* The task entry may declare no payload row at all, because it has no external
  custodian — exactly as it may not declare an `own<Buffer>` result.

Raising and taking:

```qcore
raise Recovery::Failed(payload);              // moves one owned Buffer in
let recovered = qcore_error_payload(failure); // moves it back out, once
```

`raise E::V(b);` admits only a name bound to `own<Buffer>`, and consumes it.
`qcore_error_payload(e)` is the only way a payload leaves an error; the error
keeps its tag and loses its payload, so a second take is refused and, until the
take happens, the bound error may neither be discarded at an exit nor re-raised.
A taken payload is an ordinary owned Buffer carrying the ordinary duty.

The complete admitted grammar is
[G02bc in the implemented-grammar record](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02bc-tagged-error-payload).
The canonical program used throughout this record is the `PAYLOAD` fixture in
`crates/qcore-frontend/tests/error_payloads.rs`: the child owns the parent's
Buffer, allocates two locals, and on either allocation failure releases whatever
it acquired and raises `Recovery::Failed` carrying the parent's Buffer back.

## 2. What the checker was changed to enforce

`crates/qcore-core/src/scoped_tasks.rs`
* `TaskOutcome::ErrorWithBuffer { tag, buffer }`. Completion holds it to the
  same rule as a typed owned result: `resources` must be exactly `[buffer]`, and
  that list must equal every identity the child still owns — so a local the
  child acquired and did not release refuses the completion. An empty tag is an
  `EmptyErrorLabel` exactly like an empty `Error` label.
* Between completion and join the payload is `Custodian::Capsule(child)`:
  neither parent nor child may use or release it, and the lending scope stays
  busy. This is the existing capsule rule, now reached on an error edge.

`crates/qcore-core/src/task_execution.rs`
* `TaskFunction::error_payloads`, a declared subset of `errors`, checked before
  any path is explored (SpawnError/AllocError and the entry are refused there).
* `TaskTerminatorKind::Raise` gains an optional payload operand. Only a literal
  `Move` is admitted, so a payload is never a copy, a scalar or a borrowed
  identity, and its presence must agree with the declared row in both
  directions.
* `TaskInstructionKind::TakeErrorPayload`, and an exit rule that names an
  undelivered payload separately from a leak
  (`return/error discards an undelivered error payload`).
* Worker exit summaries carry the payload's checked identity (`Input(i)` or a
  proved fresh local), resolved caller-side exactly like `returned`; the two can
  never both be populated on one exit.
* The scalar-result overapproximation is **not** applied to a function with a
  payload row. Its premise is that every actual exit drops every input, which a
  payload edge breaks by construction; such a function keeps its actual checked
  exits, exactly as an owned-result worker already does. Exits are deduplicated.
* The conservative empty-error arm of `TryOutcome` is not explored for a
  payload-carrying Result. Synthesizing a payload-free error state there would
  be a false state rather than a conservative one. No coverage is lost: a
  payload row is nonempty by construction, so every such creation has a genuine
  error alternative that checks the same arm, and a declared payload error the
  worker never raises leaves that arm unreachable, which the profile refuses.

`crates/qcore-core/src/task_c_emit.rs` — the payload rides in the existing
`qcore_task_result.value` alongside the error tag; the raise, the error edge and
the take each move the pointer and clear the source in the same statement, so no
live C state names the Buffer twice. A tagged completion prints
`QCORE_TASK_RESULT task ERROR_BUFFER tag generation`, distinctly from `ERROR tag`.

`crates/qcore-frontend/src/source_tasks.rs` — declaration parsing, the uniform
row rule, `raise E::V(b);`, `qcore_error_payload`, and the `?` refusal.
`qcore_error_payload` is added to `is_reserved_source_word` so it resolves
without being declared; a same-named local is inert rather than capturing it
(pinned by test).

`crates/qcore-verification/src/driver.rs` publishes the new
`SourceExecutionProfile::ScopedErrorPayloadV1`, selected ahead of the read-loan
and owned-task labels whenever any checked function declares a payload row.
`custody_payloads` is `TypedTaskBuffers` for such a module.

## 3. Refusals, with their measured diagnostics

Every row is a test in `crates/qcore-frontend/tests/error_payloads.rs` or
`compiler/tests/error_payload_tasks.rs`, asserting the message actually
measured, so a refusal that migrates to a different gate is a visible change.

| Refused program | Gate | Measured message |
|---|---|---|
| Error handler returns without taking the payload | core | `return/error discards an undelivered error payload` |
| Child fails while still holding an acquired local | core | `return/error discards a handle, unconsumed Result or owned buffer` |
| Taken payload never released | core | `return/error discards a handle, unconsumed Result or owned buffer` |
| Payload taken twice | frontend | `qcore_error_payload requires a bound error that still holds an owned payload` |
| Second join of the same handle | core | `join requires a live handle slot` |
| Bound error re-raised while loaded | core (IR) | `a bound error must deliver its payload before it is re-raised` |
| `?` on a payload-carrying Result | frontend | ``an error that carries an owned payload requires an explicit Result match, not `?` `` |
| `borrow<Buffer>` as a payload | frontend | `an error payload requires own<Buffer>` |
| Already-released Buffer as a payload | core | `slot N is not an initialized owned buffer` |
| Task entry declares a payload row | core | `owned task entry error payloads have no external custodian in this profile` |
| Payload declared but not raised, or raised but not declared | frontend | `raised payload disagrees with the declared error variant` |
| Mixed payload / payload-free declared row | frontend | `a declared error row must carry an owned payload on every variant or on none` |
| `V(i32)`, `V(borrow<Buffer>)`, `V()`, two fields, `V(own<Buffer> use one)`, missing `;`, duplicate variant | frontend | `… payload-free or carry exactly one own<Buffer>` |

No diagnostic code was added or changed: the frontend refusals keep `Q300`
(typed errors) and `Q500` (C emission) as `source_tasks::failure` already
assigns them, and every core task refusal keeps `Q600`.

### Two core gates the source surface cannot reach

The uniform row rule and the entry rule refuse earlier than the `?` gate and the
rethrow gate in every program the surface can express, because a function that
catches a payload error and then propagates or rethrows it would have to declare
that tag beside `AllocError` or `SpawnError`. This is recorded rather than
papered over:

* The `?` refusal **is** reachable, on the one shape that admits it: a worker
  whose whole declared row carries payloads, joined with `?` by an entry whose
  own row is also uniform. That program is a source-level test.
* The rethrow refusal is not reachable, so it is exercised on IR the lowering
  produced and the test then mutated
  (`a_bound_error_cannot_be_re_raised_while_it_holds_a_payload`). If the surface
  is widened later, that gate cannot be silently bypassed.

## 4. Evidence

### Source-level acceptance — 17 tests

`crates/qcore-frontend/tests/error_payloads.rs`. Every case runs the real
pipeline (parse → elaborate → lower → `check_task_module`), so an acceptance is
an all-paths result of the checker the driver publishes.

Positive: the payload row comes from the declaration and not from a use site;
the join transfers it exactly once and the parent touches it only afterwards;
partial acquisition releases the local it does not carry, on two distinct
failure traces; the success path releases everything inside the child; a failed
creation transfers nothing; one Buffer is a normal owned result on one path and
a payload on another; a same-named local does not capture the intrinsic.

### Native — 6 fixed Linux runs under ASan + UBSan + LSan

`compiler/tests/error_payload_tasks.rs`. Each run compiles the real emitted C
and compares stdout **line for line** with the independently computed reference
execution of the same checked module, and additionally records every Buffer
allocation and release by address.

| Run | Injected outcome | Observed |
|---|---|---|
| 1 | none | 3 objects, each allocated and released once; nothing crosses the join; no `ERROR_BUFFER` |
| 2 | child allocation 1 fails | 1 object, allocated and released once; one `ERROR_BUFFER Recovery::Failed 1`; join precedes the parent's read, which precedes its release |
| 3 | child allocation 2 fails | 2 objects, each allocated and released once; the local is acquired then released inside the child before the tagged completion; the child never releases generation 1 |
| 4 | `thrd_create` refuses | 1 object; `QCORE_TASK_CREATE_FAILED 1`; no child, no completion, no payload |
| 5 | `thrd_join` refuses | 1 object allocated, **zero** releases; exit 77 (trapped abort), never 91 (a forbidden release); no join, no result, no transfer |

Run 5 is the acceptance-3 "failure cannot double-transfer or free the result"
observation, and LeakSanitizer is deliberately off for it: the retained
allocation *is* the observation.

An emission test additionally pins, without hardcoding slot identities, that the
raise carries the payload in the result value and consumes its slot (both
raises), that the error edge moves the payload into the bound error and clears
the Result's own copy, that exactly one take consumes the bound error's payload,
and that the tagged print form exists.

**No race detector runs, and that is measured rather than chosen.**
ThreadSanitizer does not intercept C11 `thrd_create`, which this backend uses; a
`-fsanitize=thread` build of any emitted task program dies in the child before
the worker body runs. That was measured for card T04 against a fixture with no
payload at all, so it is a property of the backend's thread primitive. The
payload's single-transfer property rests on the checker's rule.

### Ledger — 2 tests

`crates/qcore-core/src/scoped_tasks.rs`. A tagged error payload is named exactly
(omitting, substituting, duplicating, padding and an empty tag each reject with
the ledger unchanged), belongs to nobody until the creator's join, is
transferred once (a second join is `HandleConsumed`), and cannot be released
twice. A completion is refused while an unreleased worker local is still live.

### Lean — 19 audited theorems, no new axiom

`proof/QCore/ErrorPayload.lean`, audited by `proof/AuditErrorPayload.lean`
(bringing `scripts/check_lean_audits.py` from 23 to **24** audits). Every
theorem depends only on `propext` (two also on `Quot.sound`). Seven decidable
scenarios mirror the Rust and source fixtures.

The child's inventory is enumerated over an explicit finite pool of identities
rather than assumed, so a local the child forgot is visible to the completion
rule instead of being assumed away. Covered: an outcome carries at most one
identity; payload-free outcomes carry none; completion holds exactly what it
names, with both refusing directions (an unreleased local, a foreign payload);
a completed payload belongs to nobody; completion parks exactly the carried
identity; the join delivers a tagged payload *into the bound error* and an owned
result directly, touching nothing else; a second join transfers nothing; a
payload inside a bound error cannot be released or re-raised; only the holder
can take it; taking yields exclusive authority to exactly the holder, cannot
happen twice, admits the re-raise afterwards, and preserves every other
identity; and the whole admitted round trip ends released with every other
identity untouched.

**These are model theorems.** Source lowering, the Rust checker and this model
agreeing, native scheduling and the C memory model are not proved here.

## 5. Gate

Run from a clean worktree at `0dfb839` (submodule initialised).

| Gate | Before (`c60fd91`) | After |
|---|---|---|
| `cargo test --workspace` | 1141 passed | **1166 passed**, 0 failed |
| `cargo fmt --all -- --check` | clean | clean |
| `cargo clippy --all-targets -- -D warnings` | clean | clean |
| `reevaluation_probe` | exit 0 | **exit 0** |
| `reevaluation_followup` | exit 0 | **exit 0** |
| `python3 -B -m unittest discover -s scripts/tests` | 43 passed | 43 passed |
| `(cd proof && lake build)` | success | success |
| `python3 -B scripts/check_lean_audits.py` | 23 audits | **24 audits** |
| `providers`: `cargo test` / fmt / clippy | 56 passed, clean | 56 passed, clean |

The 25 new tests are 17 source-level, 6 native and 2 ledger.
`lake` is not on `PATH` by default on this host; the Lean gates were run with
`PATH="$HOME/.elan/bin:$PATH"`.

## 6. What did not change

* **No raw Cycle 19 unit is promoted.** Totals stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units.
* **The four pinned 185-unit artifacts were not re-measured**, and are unchanged
  by construction rather than by measurement: the entire diff is the source/task
  route (`qcore-core` task execution and ledger, `qcore-frontend` source
  lowering, one `qcore-verification` profile label, one `qcorec` label) plus
  documentation and proof. `qcore-c-gateway`, `qcore-contract`, `qcore-audit`
  and every provider are untouched. The canonical CMetrics audit could not be
  re-run here either: the prepared Clang build tree
  (`build/cycle19/fluent-bit-clang-production`) is absent from this worktree.
  CORPUS-REPIN is unaffected by this card.
* Contract corpus stays 59 modules; `qcore-schemas` stays 5.0.0; plugin protocol
  v1, evidence v2 and the program schema are untouched.
* CT coverage stays **15/50 mapped** — see §7.
* No diagnostic code was added, removed or changed.

## 7. Integrator decisions needed

### 7.1 CT17 and CT18 (manifest change — not made here)

`docs/reevaluation/ct_manifest.json` is outside this card's write scope, and the
CT harness fails closed if a fixture directory exists while the manifest still
reads `unmapped`, so **no fixture directory was added**. Both cases are exactly
this card's subject and both are now decidable:

* **CT17** — "Worker error payload contains owned result / Join transfers it
  exactly once". Covered by the canonical `PAYLOAD` program. Measured stages
  would be parser, type_checker, reference_execution, cfg and native_backend
  **executed** on `evidence_target: linux`; proof **unassessed** (the model
  theorems in §4 are separate evidence, not a source-fixture proof stage). The
  fixture is `compiler/tests/fixtures/ct/CT17/case.qcore` = the `PAYLOAD`
  constant, with a `native_task_scenarios` matrix of the four returning runs in
  §4 and negative controls for the undelivered payload and the unreleased taken
  payload.
* **CT18** — "Worker error payload contains scoped capture borrow / Reject".
  Covered by `a_scoped_borrow_cannot_escape_inside_an_error_payload`; measured
  stages parser and type_checker **executed**, the rest unassessed, since it is
  a pure rejection.

Mapping both would move CT coverage from 15/50 to **17/50**, by executed-stage
evidence only. The integrator should decide whether to land the fixtures and the
manifest rows together in the consolidation commit.

### 7.2 Status-ledger text

Proposed paragraph for AGENTS.md / CLAUDE.md (not written here):

> Typed error payloads (2026-09-08): `ScopedErrorPayloadV1` carries exactly one
> owned Buffer inside one explicitly tagged error, out of one child through its
> completion capsule and its creator's join. A declared error row is uniform,
> `AllocError`/`SpawnError` never carry a payload and the entry declares none.
> `raise E::V(b)` moves one in and `qcore_error_payload(e)` takes it back out,
> once; until then the bound error can be neither discarded nor re-raised.
> Partial worker acquisition releases every local it does not carry; a failed
> creation transfers nothing; a duplicate join and a failed join reclaim
> nothing. Six fixed Linux ASan/UBSan/LSan runs match reference execution line
> for line; ThreadSanitizer still cannot instrument this backend. Nineteen
> audited Lean theorems in a new `QCore.ErrorPayload` namespace, no new axiom.
> CT17/CT18 mappings are proposed in the report only; CT stays 15/50. Several
> payloads, recursive owned graphs, arbitrary error ABIs, nested workers and
> loans inside payloads remain refused. Evidence:
> `docs/reports/remaining-t06/README.md`.

### 7.3 Acceptance records

`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are untouched. The T06 row and its
evidence classes should be reconciled in the consolidation commit, together with
whichever CT decision §7.1 produces.

### 7.4 No schema change

`qcore-schemas` was not touched. The payload travels inside the existing checked
task IR and the existing C `qcore_task_result`, so no wire format, package
version or manifest changed.

## 8. Open boundaries

Named, refused and not claimed:

* **Recursive owned graphs.** A payload is one flat `own<Buffer>`. Nothing
  linked, nested or reachable-through travels this way.
* **Arbitrary error ABI.** One payload field, one type. Scalar payloads,
  several fields, a payload on more than one variant of a *mixed* row, and any
  discriminated take are refused rather than approximated.
* **Nested workers.** Workers remain first-order leaves; a payload cannot cross
  more than one creation.
* **Loans inside payloads.** A `borrow<Buffer>` is refused as a payload. Loans
  and payloads compose only in the sense that both are refused where the other
  applies; a program that lends *and* carries a payload has not been exercised.
* **Payload-carrying rethrow and payload propagation with `?`** are core gates
  the current source surface cannot reach (§3). Widening the surface is future
  work, and the gates are pinned so it cannot happen silently.
* **The uniform-row rule** is a real restriction on expressible programs, not
  only on unsafe ones: a function cannot declare a payload error alongside
  `AllocError` or `SpawnError`. Relaxing it needs variant-discriminating error
  matching, which this profile does not represent.
* **Rust / Lean / C refinement.** The three are separate evidence. The Lean
  model is not extracted from the checker, and the native runs are five fixed
  Linux schedules, not a schedule space.
* **Other platforms.** Native emission requires Linux C11 threads, as before.
* **Data-race freedom** for any concurrent access to a payload is not measured;
  in this profile the payload is never concurrently reachable, because it is in
  the capsule between completion and join, but that is the checker's rule.
