# REGION-JOINS-DUTIES — real duties in the composed region's pack

Date: 2026-09-12. Base: `main` at `5d8b2ff1` (the wave-28 close), in an
isolated worktree on branch `claude/vibrant-curie-9f9380`. **Wave 29.**

**What landed:** the remainder card `REGION-CREDIT-JOINS` named (its §5 first
row and §9 first item), which is also **CT25's gap** — *"Cleanup destroys
owner after joins and loan expiry"*, expected **Accept**. A
`source.epoch.tasks.v1` entry's composed region body now admits the task
route's **own** acquisition and release beside the parent's quantities:

```
error region R {
    let owner = qcore_buffer_new(7)?;
    credit 5;
    effort 2;
    qcore_check(false, Failure);
} catch (Failure) quant open R as pack {
    discharge pack;
}
```

The acquisition seals into the pack as a real cleanup duty
(`CleanupAction::Destroy`), a body `drop NAME;` of a region owner erases one
(the model's frame rule), and the opening's `discharge` destroys the remaining
owners in **reverse acquisition order — after every join** — then abandons the
remainder. Both figures are published (`ComposedRegionEvidence` gains
`released: Vec<String>` beside `abandoned_credit`) and both are re-derived
from the reader's independent folds against the model's own cleanup trace
before publication.

**This card deliberately supersedes standing constraint 15m's duty-free
premise** — the premise was recorded as a refusal to survive refactors, and
this card is the named remainder that retires it on purpose, not an accidental
weakening. The proposed amendment text is in §2, for the integrator; this card
does not edit `AGENTS.md`.

**Nothing else moves.** Zero new codes (`Q302` condition widenings under the
`Q-NAMESPACE-DECISION` ice box, stated in §L21 and §4), zero reserved words
(§5), the three composed checkers — the scoped-task route, `check_flat_epoch` +
`EpochCreditRecoveryV1`, `qcore_core::error_region::analyze_error_regions` —
all **unedited**, no `qcore-schemas` change, no CT manifest or acceptance
matrix edit, no corpus artifact touched, and the CT22 fixtures under
`compiler/tests/fixtures/epoch_credit/CT22/` untouched (a parallel lane owns
their move). Every duty-free region admitted before this card publishes
byte-identical certificates, evidence (`released` empty) and emitted C.

---

## 1. The emission decision: the region erases into its straight-line twin

Wave 28's premise was that a sealed duty "would need an emission for the
discharge's releases that this composition does not have". The emission it
needs is the sequential route's own precedent (`QUANT-REGION-SOURCE` §3) read
at the **text** level, which is where this composed route's architecture
already lives (`split_composed_epoch` hands the scoped-task route a derived
text with offsets preserved):

An admitted region has exactly **one** path — the decided error edge, caught,
discharged — so it erases into the straight-line program that path runs. The
split now performs that erasure textually ([`RegionErasure`]):

1. the region statement's whole range is blanked (spaces; newlines kept),
2. each body acquisition and body release is restored **verbatim at its own
   byte offsets** — so every span the task route reports still points at the
   original file,
3. one `drop NAME;` per remaining duty is written into the blanked space
   after the last kept statement, in reverse acquisition order — **first
   fit** over runs of blanked bytes, never crossing a newline, deterministic.
   A region whose own text has no room for its erasure is **refused**, not
   approximated (`REGION-JOINS-ERASURE-FIT`, `Q302`, measured in a test).

The task half is then an ordinary scoped-task program the **unedited** route
compiles, custody-checks, emits and reference-executes: the destruction the
model decided is a real `qcore_buffer_drop` in the emitted C, not a claim. A
duty-free region keeps wave 28's pure blanking byte for byte, because its
erasure keeps nothing and inserts nothing — measured by the wave-28 suite
running unedited (§6).

Two consequences are measured rather than asserted:

* **The twin identity.** `the_region_erases_into_its_straight_line_twin_byte_for_byte`
  compiles the flagship against a **hand-written** twin — the same file shape
  with the erased text written literally, an independent oracle computed by
  hand in the test, not a readback of the split — and the emitted C, entry and
  reference observation are byte-identical, with source-map **positions**
  identical row for row. Exactly one row's rendered `line_source` may differ:
  the synthesized release's position lies inside the region statement's own
  extent, so the region variant renders the original file's line there — the
  same attribution rule every erased span of this route has.
* **The native positive control.** `the_native_run_frees_every_owner`
  (Linux/macOS, the `owned_source_tasks` pattern) compiles the CT25 case's
  emitted C with counting allocator hooks and runs it: **two** allocations
  (worker result, region owner), the freed set equal to the allocated set,
  exit success. A discharge that lost the owner's duty surfaces here as an
  unfreed allocation, not only as a disagreement.

## 2. Proposed amendment text for standing constraint 15m

For the integrator to apply to `AGENTS.md` (this card does not edit it):

> 15m. **A composed region on the epoch task route opens its pack after every
> join, and the pack's duties are the parent's own owners.**
> `REGION-CREDIT-JOINS` (wave 28) let a `source.epoch.tasks.v1` entry carry
> one `error region … catch (E) quant open … { discharge …; }` whose
> `AbandonCredit` publishes the parent-local unused credit — CT22's scenario.
> `REGION-JOINS-DUTIES` (wave 29) **deliberately superseded** the duty-free
> premise this constraint recorded: the composed body also admits the task
> route's own acquisition and release — `let NAME = qcore_buffer_new(INT)?;`
> sealing a real `Destroy` duty, `drop NAME;` of a region owner erasing one —
> and the discharge destroys the remaining owners in reverse acquisition
> order, CT25's duty half. Premises that must survive refactors, each a
> refusal today: every `TrySpawn`/`Join` span of the checked task IR
> **precedes** the region (`Q600` in both directions — children live at the
> opening stays `CheckedRegionTaskCreditV1`'s territory, `REGION-JOINS-T07`);
> the region **erases into its straight-line twin** — a duty-free region
> blanks completely (emitted C, source map and reference observation
> byte-identical with and without it, the wave-28 measurement kept), and a
> duty-carrying one keeps its acquisitions and body releases at their own
> byte offsets in the task half with one `drop NAME;` per remaining duty
> written into the blanked space after them, first-fit, newlines never
> crossed, refusing when it does not fit (`REGION-JOINS-ERASURE-FIT`) — so
> the three composed checkers (the scoped-task route, `check_flat_epoch` +
> `EpochCreditRecoveryV1`, `analyze_error_regions`) stay **unedited**, each
> refusing at its own codes. **Do not re-implement a custody rule beside the
> erasure**: two simultaneously live owners refuse at the task route's
> existing custody `Q600`, because the second acquisition's `?` failure edge
> discards the first — the erased program's real leak path
> (`REGION-JOINS-MULTI-OWNER`) — and teaching the erasure control flow to
> "repair" that would invalidate the one-path premise the erasure rests on.
> Both published figures are re-derived before publication —
> `abandoned_credit` against the reader's quantity fold, `released` against
> its duty fold, over the model's own cleanup trace. A region owner's name is
> the region's own (a collision with an entry binding refuses, since the
> erasure flattens the body's block into the task scope), the sequential
> `qcore_acquire` spelling names no task-route resource and stays refused,
> and the declarative epoch route gains no error path (constraint 15j's
> shape). Zero new codes: `Q302` condition widenings, stated in §L21.
> `region-credit-joins`, `region-joins-duties`.

## 3. Where the work is done, and where it is not

| Component | Change |
|---|---|
| `crates/qcore-frontend/src/source_epoch_tasks.rs` | the reader admits the two owner shapes (`region_acquisition` token shape; body `drop` of a region owner), assigns resource identities in acquisition order, folds the duty list, scans entry bindings for collisions, plans the erasure (`plan_releases`, first fit); `ComposedRegion` gains `duties` and `erasure`; `RegionErasure` **new**; `split_composed_epoch` takes the erasure and applies keeps + insertions over verified blanks |
| `crates/qcore-verification/src/driver.rs` | `check_composed_region` parses the trace as destroys-then-abandon and cross-checks the destroys against the reader's duty fold (a second independent re-derivation beside the wave-28 credit one); `ComposedRegionEvidence` gains `released` |
| `compiler/tests/region_joins_duties.rs` | **new**: 9 end-to-end tests, every refusal an exact code vector, including the hand-written-twin byte comparison and the native counting-allocator run |
| `compiler/tests/fixtures/epoch_credit/CT25/` | **new**: the case and seven controls (§6) — deliberately outside `compiler/tests/fixtures/ct/` and outside the CT22 directory the parallel lane owns |
| `crates/qcore-frontend/tests/source_epoch_tasks.rs` | 3 new reader/split unit tests (21 total); the wave-28 split test repointed at the new signature |
| `proof/QCore/RegionJoinsDuties.lean`, `proof/AuditRegionJoinsDuties.lean`, `proof/QCore.lean` | **new** Lean module, 14 audited theorems (§7); audit count 115 → **116** |
| `docs/QCORE_SPEC_r0_3.md` §L21 | the `Q302` widening stated (§4) |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02f | the `region_epilogue` grammar grown by the two owner shapes; the erasure and boundary paragraphs rewritten |

Not touched, deliberately: `qcore_core::error_region`,
`qcore_core::epoch_accounting`, `qcore_core::epoch_credit_recovery`,
`qcore_core::region_task_credit`, `qcore_core::task_execution`, the sequential
route's `source_region.rs`, `examples/`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance*.json`, `AGENTS.md`, the CT22 fixture directory,
the C gateway, and every corpus artifact. Corpus-neutral: no gateway,
contract, audit or provider file is edited, so no arbiter run is implied.

## 4. Zero new codes, zero new words

Under the `Q-NAMESPACE-DECISION` ice box this card allocates nothing and
widens **one** code's conditions, stated in §L21 and here:

* **`Q302`** widens a third time on the composed route: the admitted-set
  condition now includes the two owner shapes, and the card's own shape
  refusals land beside the existing ones — a release of a name the region did
  not acquire (`REGION-OUTER-LIVE-RESOURCES` reached from this route), an
  owner colliding with an entry binding, one owner acquired twice, the
  erasure-fit refusal (`REGION-JOINS-ERASURE-FIT`), and the sequential
  `qcore_acquire` spelling, which names no task-route resource.
* **`Q600` is not widened.** The after-joins condition is unchanged in both
  directions, and the two-owner refusal (§6) is the task route's **existing**
  custody condition reached by the erased program — no new condition exists.

**Constraint 14 census: +0 reserved words, vacuous by construction.** Every
token of the admitted shapes — `let`, `=`, `qcore_buffer_new`, `(`, `)`, `?`,
`;`, `drop` — already parses on this route in exactly these statement kinds;
what changed is only which statements the composed region **reader** admits
inside the region body, where every shape previously refused at `Q302` with
the wave-28 admitted-set message (a measured baseline, kept as tests).

## 5. Refusals kept, refusals changed, and the one admitted

**Kept exactly** (code and message, verified by the unedited wave-28 suite
running green):

| Refusal | Code | Status |
|---|---|---|
| a spawn or join after the region, both directions | `Q600` | unchanged, message still names `REGION-CREDIT-JOINS`; re-verified with a duty in the region (`after_joins_is_still_refused_with_a_duty`) |
| the mint/abandon refusals | `Q635`/`Q636` | untouched (no epoch file edited); `both_models.qcore` still measured green in the wave-28 suite |
| the model's own pack obligations | `Q303`–`Q309`, `Q300` | unchanged codes and wording; `Q305` and `Q306` now **also** reachable with a real duty (`no_discharge_duty`, `no_open_duty` — a pack with an action and zero credit is nonempty) |
| a region in a worker / outside the task scope / a second region / second catch / passing check / uncaught raise / handler body | `Q302` | unchanged messages, wave-28 tests unedited |
| the pure declarative epoch route with a region | `Q600` | unchanged (constraint 15j's shape) |

**Changed, being the one admitted:** the wave-28 admitted-set message (the
`Q302` refusal of any body statement outside `credit/effort/check`) now lists
the two owner shapes and states that `qcore_acquire` is the sequential route's
spelling; the `let … = qcore_acquire(…)` probe still refuses at `Q302` and
still names the card (`the_composed_reader_refuses_its_own_shapes` in the
wave-28 file passes **unedited**, its `contains("REGION-JOINS-DUTIES")`
assertion satisfied by the new message).

**New refusals, each named in §4.** And one **measured boundary**:
`REGION-JOINS-MULTI-OWNER` — two simultaneously live owners
(`two_owners.qcore`) refuse `["Q600"]` at the task route's existing rule
("return/error discards … owned buffer"), because the second `?` failure edge
really does leak the first owner in the erased program. The sequenced spelling
(`drop first;` before the second acquisition) is admitted, duties `[second]`.
The discharge condition is a cleanup-aware error-path shape in the erased text
(the region body has no `match`), not a weakening of either checker.

## 6. Evidence

### Rust — 12 new tests, all green; the wave-28 suite green unedited

`compiler/tests/region_joins_duties.rs` (9), end to end through
`compile_files`:

| Test | Covers |
|---|---|
| `ct25_the_cleanup_destroys_the_owner_after_every_join` | **the CT25 duty half**: admitted; `released = ["owner"]`, `abandoned_credit = 3`; the model handed the real `Acquire`; the join precedes the region in the checked IR; the owner's allocation in the emitted C |
| `the_region_erases_into_its_straight_line_twin_byte_for_byte` | emitted C, entry, reference observation byte-identical with a **hand-written** twin; source-map positions identical row for row, at most the synthesized release's rendering apart |
| `a_body_release_erases_its_duty` | `dropped_owner.qcore`: `released = []`, `abandoned_credit = 3` — the frame rule end to end |
| `duty_order_and_the_multi_owner_boundary` | `sequenced_owners.qcore` admitted with `released = ["second"]`; `two_owners.qcore` refused `["Q600"]` by the task route's own rule |
| `the_models_own_refusals_hold_with_a_real_duty` | `["Q305"]` walk-away over a held owner; `["Q306"]` duty-alone nonempty pack |
| `the_duty_reader_refuses_its_own_shapes` | `["Q302"]` non-owner release (named), `["Q302"]` colliding owner, `Q302` `qcore_acquire` naming `qcore_buffer_new` |
| `after_joins_is_still_refused_with_a_duty` | `["Q600"]`, the wave-28 rule re-verified over a duty-carrying region |
| `the_epoch_certificates_are_unmoved_by_a_duty` | bound and credit-recovery certificates equal between the owner region and the duty-free shape; the duty-free region publishes wave-28 evidence (`released = []`) |
| `the_native_run_frees_every_owner` | Linux/macOS: emitted C under counting allocator hooks; 2 allocations, freed set = allocated set, exit success |

`crates/qcore-frontend/tests/source_epoch_tasks.rs` (3):
`the_reader_reads_an_owner_acquisition_as_a_duty` (ops, duty fold, erasure
plan: one kept span holding the acquisition's exact bytes, one `drop owner;`
insertion inside the span after it; the dropped-owner control keeps two and
inserts none), `the_split_erases_a_duty_region_into_its_straight_line_twin`
(byte length preserved, kept bytes restored, insertion written, the reparsed
task half carries no region and does carry the release),
`the_duty_reader_refuses_its_own_shapes` (non-owner release, collision,
doubled owner, and the erasure-fit refusal measured on an 84-byte owner name).

### Fixtures

`compiler/tests/fixtures/epoch_credit/CT25/`, **outside**
`compiler/tests/fixtures/ct/` (the harness fails closed when an unmapped case
owns a directory there; mapping moves the directory — the CT22/CT23
precedent) and outside the CT22 directory a parallel lane owns:

| Fixture | Role | Measured |
|---|---|---|
| `case.qcore` | **the case** | admitted; `released = ["owner"]`, `abandoned_credit = 3`; join precedes region; native run frees both buffers |
| `dropped_owner.qcore` | the frame rule | admitted; `released = []`, abandoned 3 |
| `sequenced_owners.qcore` | duty order through a real release | admitted; `released = ["second"]` |
| `two_owners.qcore` | the multi-owner boundary | refused `["Q600"]`, the task route's own custody rule |
| `no_discharge_duty.qcore` | CT21's shape with a held owner | refused `["Q305"]` |
| `no_open_duty.qcore` | nonempty-by-duty pack, zero credit | refused `["Q306"]` |
| `outer_drop.qcore` | release of a non-owner | refused `["Q302"]`, names `REGION-OUTER-LIVE-RESOURCES` |
| `colliding_owner.qcore` | owner name = entry binding | refused `["Q302"]` |

### Reference / native byte comparison — the family's standard, plus one native run

The family's standard is emission parity against the task route's own output
for the derived text (`T08-COMPOSE` §5, kept through wave 28), and this card
keeps it with the twin now **carrying the duty program**: the hand-written
twin comparison is byte-for-byte on emitted C and reference observation. On
top of the standard, this card adds the counting-allocator **native run** of
the case (Linux/macOS, `clang -std=c11 -pthread -Wall -Wextra -Werror`,
`QCORE_BUFFER_MALLOC`/`QCORE_BUFFER_FREE` hooks). **No sanitizer claim is
made** — the scoped-task family's native pattern (`owned_source_tasks.rs`)
carries none, and none is added here.

### Lean — 14 new audited theorems, no new axiom

`proof/QCore/RegionJoinsDuties.lean`, registered in
`proof/AuditRegionJoinsDuties.lean` and imported by `proof/QCore.lean`. Audit
count **115 → 116**. Every theorem's axiom surface is empty, `[propext]` or
`[propext, Quot.sound]`; no `sorry`, no placeholder, no project axiom.
`QCore.ErrorRegion`, `QCore.QuantRegionSource` and `QCore.RegionCreditJoins`
are consumed, not edited.

| Claim | Theorems |
|---|---|
| the admitted shapes as one list (four constructors where wave 28 had two), and the supersession is conservative on the duty-free sublanguage | `admitted_owner_shapes_are_exactly_these`, `the_duty_free_sublanguage_is_unchanged` |
| the pack after the joins carries the body's own duties beside the remainder, and nothing a child held can be among them | `the_pack_after_joins_carries_the_bodys_duties`, `the_pack_still_carries_the_quantity`, `duties_are_the_bodys_own` |
| **the CT25 property**: the discharge restores the baseline, destroys the owners in reverse acquisition order, abandons the remainder, consumes the capability; a walked-away duty refuses; a duty alone makes the pack nonempty | `the_cleanup_destroys_the_owners_after_the_joins`, `a_handler_that_walks_away_from_a_duty_is_refused`, `a_duty_alone_makes_the_pack_nonempty` |
| the erasure is a program and it is the flat twin; the split's releases are the model's own trace; an owner moves no credit through the wave-28 cross-check | `the_splits_releases_are_the_models_trace`, `an_owner_moves_no_credit` |
| the flagship's own figures | `the_flagship_seals_one_owner_and_abandons_three`, `a_body_release_erases_its_duty`, `a_sequenced_body_leaves_the_second_owner`, `two_owners_are_destroyed_last_first` |

The Rust/Lean correspondence is **by inspection**, as everywhere in this
family. The after-joins "no child holds anything" fact is
`QCore.RegionCreditJoins`'s own `live` machinery, consumed unchanged.

## 7. CT25 readiness — what a wire lane can and cannot claim

**`docs/reevaluation/ct_manifest.json` and `docs/reviews/r0_3/acceptance.json`
are untouched by this card**; mapping is the CT lane's (constraint 23).

CT25's scenario has two halves, and this card lands **one**:

* **"Cleanup destroys owner after joins" — expressible and accepted end to
  end.** Every word carried by a checked fact: *cleanup destroys owner* by the
  unedited model's `Destroy` trace on the discharge of the opened pack, made a
  real emitted, custody-checked, reference-executed and natively-run
  destruction by the erasure; *after joins* by the derived span rule refused
  at `Q600` when false, plus `duties_are_the_bodys_own` (no duty a child
  held). The proving test is
  `region_joins_duties.rs::ct25_the_cleanup_destroys_the_owner_after_every_join`
  on `epoch_credit/CT25/case.qcore`.
* **"and loan expiry" — NOT expressible.** Read-loan captures stay refused on
  this route (`T08-COMPOSE-LOAN`), so no loan over the owner exists to expire;
  the sequential route has loans nowhere either. The remaining gap is exactly
  one named residual: a route on which an owner can be lent, the loan's
  expiry derived, and the cleanup ordered after it.

A lane should therefore **not** map CT25 on this card's evidence unless it
decides — in its own stage-evidence record, with the reasoning published —
that the duty half alone carries the scenario's expected Accept; on the plain
reading of the case text ("joins **and** loan expiry"), the honest position is
to keep CT25 unmapped with the narrowed reason *"loan expiry has no referent
on the composed route (`T08-COMPOSE-LOAN`)"* replacing the wave-28 double
reason. CT22's readiness (`region-credit-joins` §7) is unaffected.

## 8. What this does not establish

* **No loan exists and none is claimed.** `T08-COMPOSE-LOAN` stands untouched.
* **The epoch bound is not moved.** The owner is a creator-scope buffer
  outside the epoch's allocation, exactly as any creator allocation on this
  route is; the certificates are measured equal with and without the duty
  (`REGION-JOINS-EPOCH-AXIS` unchanged for the credit).
* **No sanitizer claim** for the native run (§6).
* **The source map's synthesized-release row** renders the original file's
  line text at the release's position (inside the region statement). Positions
  are exact; only the rendering names the written source. Recorded, not
  repaired: the alternative — rendering text that exists in no file — would be
  a fabrication.
* **Rust/Lean correspondence is by inspection.**
* **Nothing is promoted.** No corpus row, CT row, contract count or version
  number moves; the card touches no audit path and no pinned artifact.

## 9. Open, named

| Id | Item |
|---|---|
| `REGION-JOINS-LOAN` | CT25's loan-expiry half: a route on which the owner can be lent, the expiry derived, and the cleanup ordered after it. Blocked behind `T08-COMPOSE-LOAN`. |
| `REGION-JOINS-MULTI-OWNER` | Two simultaneously live owners refuse at the task route's own custody rule over the erased `?` failure edge; the discharge condition is a cleanup-aware error-path shape in the erased text. |
| `REGION-JOINS-ERASURE-FIT` | The erasure must fit inside the statement it erases; refused today, reachable only with owner names longer than every blank run after the region's statements. |
| `REGION-JOINS-T07` | Unchanged from wave 28: spawning inside a region is `CheckedRegionTaskCreditV1`'s discipline and still has no source route. |
| `REGION-JOINS-EPOCH-AXIS` | Unchanged from wave 28: the parent-local credit (and now the owner's bytes) are outside the epoch allocation; charging either needs an integrator decision. |
| `CT25-CT-MAPPING` | The §7 decision — duty half alone vs. loan half required — is the CT lane's, on its own stage-evidence record. |

Items owed to the integrator rather than edited here: the constraint 15m
amendment (§2), `AGENTS.md`'s headline numbers (Lean audits 115 → 116;
workspace test totals), and `acceptance_matrix.json` follow-on reconciliation.

## 10. Standing per-change gate

Run at the card tip with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok |
| `cargo test --workspace --no-fail-fast` | **4,404 passed, 0 failed, 8 ignored**, over 216 test binaries, in one run — no constraint-16 load row tripped, despite a sibling wave-29 agent's concurrent suite (machine load ~35 on 16 cores at launch), so no isolation re-verification was needed |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 116 Lean audits** (115 → 116, `AuditRegionJoinsDuties.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 193 ran, **192 OK, 1 failure** — `test_build_matrix_sections … test_the_derived_facts_are_recomputed_rather_than_carried`, asserting the committed matrix's **derived** `validation.lean_audits` (115) equals the tree's recomputed value (116). That drift is exactly this card's new audit file; the card's boundary forbids editing the acceptance matrix, and the reconciliation is the integrator's wave-close matrix pass (the wave-28 close's own "reconcile … the matrix" step). No assertion is weakened and no other Python test moves |

This card's own rows: `compiler/tests/region_joins_duties.rs` **9 passed**
(native run included), `qcore-frontend`'s `source_epoch_tasks` integration
tests **21 passed** (18 → 21), and the wave-28 `region_credit_joins.rs` suite
**9 passed unedited**.
