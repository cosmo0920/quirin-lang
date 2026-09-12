# CT47-COPY-CAPTURE — the boundary refuses by the judgment, not the backend

Wave 29, card `CT47-COPY-CAPTURE`, the follow-on the wave-26 CT lane named at
`docs/reports/ct-wire-lane/README.md` §3.8 and §10. Base: `main` at `5d8b2ff1`
(the wave-28 close).

**What moved, in one sentence.** A task-creation capture that the
transferability judgment (`qcore_core::transfer`, card `CT46-SENDSYNC`) and
the capture enumeration agree to refuse is now refused **by the judgment**,
attributed to the capture at the crossing construct — `Q600`, the Concurrency
pass, a condition widening stated in spec §L21 — with a message naming the
missing transfer/share contract and the contracts the kind does carry; before
this card the same shapes fell through to the enumeration's executable-subset
catch-all at `Q500` (`CEmission`), a statement about what the backend can
encode rather than about why the value may not cross.

**Nothing is admitted and nothing stops refusing.** The refused set is
unchanged — proved, not asserted: `QCore.Transferability` now decides the
refusal reasons beside the admissions and shows refusal and admission
partition both matrices whole (`theBoundaryIsDecidedExactlyOnce`, 13 × 4 × 13
cells, every cell decided exactly once). Zero new diagnostic codes (the
Q-space icebox is untouched), zero reserved words, no parser change, no
backend change.

---

## 1. The problem, as the wave-26 lane measured it

CT47 reads *"Raw pointer is Copy but no transfer/share contract exists"*,
expected **Reject automatic cross-thread acceptance**. The lane's three
measured shapes (§3.8):

| Shape | Measured (wave 26) |
|---|---|
| `spawn s reader(buffer)` — no declared mode | `Q000`: *capture requires copy, move, read ... share, or write* |
| `spawn s reader(copy buffer)` — a bit-copy of an owned handle | `Q500`: *only copy i32, move own<Buffer> and read own<Buffer> share FULL captures are executable* |
| `spawn s reader(read buffer share 512)` | admitted, emitted |

The refusals were real; the **attribution** was not mappable. The middle
refusal is the C-emission catch-all — precisely the "the profile cannot
express this at all" vacuity the wave-9 attribution rule excludes — even
though the transferability judgment had been computing the right refusal at
that very capture since CT46-SENDSYNC and its result was thrown away in favor
of the catch-all's text.

## 2. What refuses where — before → after

| Shape | Before | After |
|---|---|---|
| capture with no declared mode | `Q000`, grammar | `Q000`, grammar — **byte-identical, untouched** (there is still no automatic mode at a thread boundary) |
| `copy` of `own<Buffer>` | `Q500` catch-all | **`Q600`, the judgment:** *"the transferability judgment refuses this capture: no transfer or share contract admits `copy` of `own<Buffer>` across a spawn boundary; `own<Buffer>` crosses only under `move` (custody transferred to the worker) or `read PLACE share N` (a counted read share)"* |
| `read … share …` delivered to an `own<Buffer>` parameter | `Q500` catch-all | `Q600`, the judgment: an admitted transfer that delivers `borrow<Buffer>` where the worker declares `own<Buffer>` |
| `move` delivered to a `borrow<Buffer>` parameter | `Q500` catch-all | `Q600`, the judgment: the mirror-image delivered-kind mismatch |
| `write` capture of any kind | `Q500` catch-all | `Q600`, the judgment: a `write` capture names no transfer or share contract |
| `move` / `read … share …` of `i32` | `Q500` catch-all | `Q600`, the judgment: `i32` crosses only under `copy` (an independent duplicate; no custody crosses) |
| `read buffer share FULL` into `borrow<Buffer>` | admitted | admitted — **unchanged** |
| everything typed wrong before the mode is asked | `Q020` | `Q020` — unchanged (the place/parameter type check still runs first) |

One message per `TransferRefusal` variant, each naming the judgment, the
capture, the missing contract, and — for a transferable kind — the contracts
it does carry, read from the judgment's own classification rather than
restated by hand (`admitted_contracts` in
`crates/qcore-frontend/src/source_tasks.rs`).

## 3. The change, mechanically

`crates/qcore-frontend/src/source_tasks.rs::spawn_inputs` already computed the
derived judgment and the enumeration at every capture and required them to
agree (CT46-SENDSYNC's fail-closed composition — untouched, including the
disagreement refusal). This card adds one step **after** the agreement check:
where both sides agree the capture is refused (`derived` is `Err`), the
refusal is emitted from the judgment's own `TransferRefusal` at
`SemanticPass::Concurrency`, attributed to `capture.span`. The `Q500`
executable-subset catch-all below it is **kept, unedited**, as the fail-closed
backstop: by the identity evidence (the enumeration-reproduction test, which
still passes) no reachable triple can now arrive there, and anything
unforeseen refuses rather than passes.

Decision order at one capture, after this card: effect gate → scope gate →
arity → place/parameter type check (`Q020`) → CT46 non-scalar sendability
guard → **agreement check** (disagreement refuses, as before) → **the
judgment's refusal (`Q600`, this card)** → the admitted lowering, with the
`Q500` catch-all as unreachable backstop.

### Attribution or semantics?

Both halves, stated separately per the card's own instruction:

* **The refused set of programs did not move** — no program changes verdict,
  no `Unknown`-class outcome flips, nothing is promoted. In that sense this is
  an attribution move of existing refusals (constraint 1's file/line/span
  clause covers the span; the *code* moves too, which is more than that clause
  covers).
* **The code and message move**, `Q500 → Q600`, and constraint 1 calls a code
  change semantic. It is treated as one: the widening is stated in spec §L21
  (the `Q6xx` row) and in `AGENTS.md`'s summary table, this report says it,
  and the Lean model gained the matching refusal-reason judgment with audited
  theorems (invariant 6) rather than riding on "only attribution moved". No
  new code is allocated — this is the standing condition-widening pattern
  (`SOCKET-EVENT-COMPOSE` precedent) the icebox prescribes.

## 4. Lean (invariant 6)

`proof/QCore/Transferability.lean` gains the refusal half of the judgment,
mirroring `qcore_core::transfer::TransferRefusal` and the frontend's decision
order: `Refusal`, `refuse : Kind → Mode → Option Refusal`,
`refuseCapture : Kind → Mode → Kind → Option Refusal`. Registered in
`proof/AuditTransferability.lean` (the existing audit file; the audit count
stays 115):

| Theorem | What it pins |
|---|---|
| `refusalComplementsAdmission` | refusal and admission partition the 13 × 4 matrix — every cell decided exactly once |
| `theBoundaryIsDecidedExactlyOnce` | the same partition over the whole 13 × 4 × 13 boundary matrix — which is what makes moving the attribution incapable of moving the refused set |
| `aWriteCaptureIsRefusedByName` | `write` refuses for its own missing lowering, for every kind |
| `aNeverKindRefusesWithItsClassReason` | a kind that never crosses refuses with its class's own reason, over the complete reason inventory |
| `copyOfTheOwnedBufferIsRefusedForItsMode` | **CT47's cell**: the copy of the one sendable-not-freely-shareable kind is refused *for its mode*, the kind still sendable — not a vacuous kind refusal, not a backend statement |
| `aDeliveredKindMismatchIsItsOwnReason` | the two spellable delivered-kind mismatches are `receivedKindMismatch`, not mode refusals |

## 5. Scope: what is landed and what is measured boundary

**Landed:** the judgment's refusal for every capture shape the task route can
spell — which is the *nearest expressible shape* half the card brief allows.
CT47's literal premise names a **raw pointer**, and this language still has no
pointer type for a program to copy; `DeclaredTypeTag::Pointer` remains
`Unmodeled`, and a declared pointer parameter still refuses long before a
capture is considered. The `copy` capture of an owned handle is the nearest
expressible shape to the case's bit-copy, and it now refuses by a
transfer/share analysis with the missing contract named.

**Measured boundary, published rather than papered over:**

* The **modeless** capture (`spawn s reader(buffer)`) still refuses at the
  grammar (`Q000`), deliberately: "no automatic cross-thread acceptance" is a
  property the grammar enforces by having no modeless form, and this card was
  forbidden from weakening that gate. A lane mapping CT47 on the copy shape
  should say the modeless shape's refusal remains syntactic.
* The raw-pointer premise itself stays inexpressible. If a lane requires the
  literal construct, CT47 stays unmapped and that is the honest answer.
* The `write` capture keyword still parses and still has no lowering; its
  refusal moved from the catch-all to the judgment's own
  `WriteCaptureUnimplemented` arm, but the keyword's status is unchanged.

## 6. CT47 readiness statement

For a future mapping lane (this card edits neither
`docs/reevaluation/ct_manifest.json` nor
`docs/reviews/r0_3/acceptance_matrix.json` — CT47 is a key of the derived
`newly_decidable_not_mapped_reasons` map, constraint 25, and its row handling
belongs to a mapping lane):

* The reject half of CT47 — *"reject automatic cross-thread acceptance"* — is
  now attributable to a transferability judgment at the crossing construct:
  `Q600`, message naming the missing transfer/share contract, fixture shape
  `spawn s reader(copy buffer)` with `reader(payload: borrow<Buffer>)`. The
  three wave-26 shapes are re-measured and pinned by
  `crates/qcore-frontend/tests/transferability.rs::ct47s_measured_shapes_a_copy_capture_is_refused_by_the_judgment`:
  shape 1 (`Q000`) byte-identical, shape 2 moved `Q500 → Q600` with the
  judgment's message, shape 3 admitted, unchanged.
* An attribution control and a boundary control in the wave-26 style are
  available for free: the same program under `move buffer` (custody
  transferred — admitted against an `own<Buffer> use one` worker) and under
  `read buffer share N` (the declared share contract — admitted) demonstrate
  that supplying the named contract is exactly what flips the verdict.
* What a lane must still decide: whether the copy-of-an-owned-handle shape
  satisfies CT47's raw-pointer premise, and how to treat the modeless shape's
  remaining `Q000`. Both facts are published here and in `acceptance.json`'s
  hand-kept CT47 reason (updated by this card; the derived matrix copy is a
  mapping lane's to regenerate).

## 7. Files

* `crates/qcore-frontend/src/source_tasks.rs` — the judgment-first refusal in
  `spawn_inputs`, plus the message builders (`scalar_spelling`,
  `mode_spelling`, `admitted_contracts`, `transfer_refusal_message`).
* `crates/qcore-frontend/tests/transferability.rs` — pins moved deliberately
  (`Q500` catch-all rows → the judgment's `Q600` messages, verbatim), plus the
  CT47 three-shape test; the accepting controls and the identity-evidence test
  unchanged.
* `crates/qcore-frontend/tests/read_loans.rs` — three catch-all substrings →
  the judgment's own messages (deliberate pin update).
* `proof/QCore/Transferability.lean`, `proof/AuditTransferability.lean` — §4.
* `docs/QCORE_SPEC_r0_3.md` §L21 `Q6xx` row, `AGENTS.md` `Q600` summary row —
  the condition widening, stated.
* `docs/reviews/r0_3/acceptance.json` — the hand-kept CT47
  `newly_decidable_not_mapped_reasons` row updated to the post-card
  measurement (staleness-checked only against the manifest; no derived field
  touched).
* `DEVELOPMENT.md` — the wave-29 increment paragraph.

## 8. What this does not establish

* **No mapping.** CT47 stays unmapped (41/50 mapped, unchanged); mapping is a
  mapping lane's decision and its row is in a derived, test-compared field
  this card may not regenerate.
* **No data-race claim, no native-refinement claim.** The judgment's per-kind
  soundness rests where CT46-SENDSYNC left it: on the scoped-task and
  read-share ledgers' own theorems.
* **No new capability.** No program that was refused is admitted; the
  admitted three capture forms are exactly the three that were admitted
  before.
* **Nothing else moved.** No gateway, contract, audit or provider file is
  touched; the Cycle 19 corpus rows and the leak pin (266) are untouched by
  construction.

## 9. Gate

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | **4,393 covered (4,392 + this card's one new test), 8 ignored: 4,392 passed in the single run, 1 failed** — `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, one of constraint 16's five recorded load-sensitive rows (wall-clock; recorded failing at load 50 and passing at load 37 on the same binary). Re-verified in isolation per that constraint: `cargo test -p qcore-compiler --test cycle20_verifier_plugins -- --test-threads=1` → **20 passed, 0 failed**. Both facts reported; no assertion touched |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 115 Lean audits** (the baseline count; the new theorems register in the existing `AuditTransferability.lean`) |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** (no matrix file touched) |
