# SLICE-PERMISSIONS-WRITE-LEND — the write half of CT07, and the fraction that finally exists (wave 29)

Card brief: the residual card `CT07-SLICE-PERMISSIONS` named for the half of
CT07 it could not reach — a lend of **a range of an owned buffer carrying write
authority**, so two parties mutate two disjoint ranges independently. The
residual enumerated what it needs, and every item is in this diff: *a second
value kind (a loan a store can name), index rebasing into the owner's
coordinates, an exclusive-overlap rule between two lends, and a custody shape
in which the owner's authority over the lent range is genuinely **transferred**
rather than merely excluded — the footprint fraction in a borrow node that the
read card measured was NOT needed for reads but IS needed here.*

Base: `origin/main` at `5d8b2ff1` (the wave-28 close), isolated worktree,
branch `worktree-agent-ad1178113fbab0e0c`.

**What landed.** One construct on `source.buffer.v1`, one extension of the
cycle-3 ownership IR, zero new diagnostic codes, zero new reserved words, zero
new tokens.

* **`let NAME = lend(BUFFER, LOW, HIGH);`** — lend write authority over the
  bytes `[LOW, HIGH)` of an owned buffer to a new binding. `NAME[i] = v` and
  `let x = NAME[i];` go through the loan, decided against the loan's **own**
  `[0, HIGH-LOW)` and rebased to the owner's byte `LOW + i`; `extent(NAME)`
  binds the loan's own length. The window runs from the lend to the last
  statement naming the loan.
* **The ownership IR gains the fraction.** A lend projects onto a new
  `OwnershipOp::LendRange` borrow node carrying a half-open byte interval on
  the resource's own axis, and the **IR itself** — not a route rule beside it —
  decides exclusivity, transfer, rebasing containment and the release arm
  (§3).
* **Two disjoint write lends are live at once**, each party mutating its own
  range, the owner reading both back after the windows close — the fixture a
  CT07 lane can point at, sanitized-native beside its reference (§7).

**31** end-to-end tests in `compiler/tests/write_lend.rs`, one of them a
sanitized native run with a positive leak control; **7** IR-level rows in
`crates/qcore-core/tests/unit/ownership.rs`. A new `QCore.WriteLend` with
**28 declarations, 25 audited by name** in `AuditWriteLend.lean`
(115 → **116** Lean audits). Two published fixture programs. **Corpus-neutral
by construction** (§9).

---

## 1. The headline: the blocker bit exactly where the read card said it would

The read card's §1 measured `STD-BUFFER-SLICE`'s blocker sentence — *"needs the
ownership IR's borrow nodes to carry a footprint fraction"* — to be **about the
write direction**, left the IR untouched, and named this card for the place the
fraction is real. This card is the measurement's other half, and it confirms
the prediction in both directions:

* **A route-level rule could not have done it.** The read card's exclusion is a
  window rule of the string route's own state, sound because the owner *keeps*
  the write authority and the route merely restrains its use. A write lend has
  no owner-side rule to restrain: the authority itself must move, other
  checkers of the custody ledger must see that it moved, and two *loans* must
  be able to conflict with each other — a question the per-resource
  `has_any_borrow` conflict test structurally cannot ask. So the borrow node
  gets the fraction: `BorrowKind::MutableRange { axis, low, high }`, and the
  conflict tests become interval questions.
* **The fraction is an interval, not an amount.** `Footprint` carries
  quantities per axis, and two 4-byte quantities cannot be checked disjoint;
  what disjointness needs is *which* bytes. The node therefore carries
  `[low, high)` on a named axis — `axes_overlap` is `max low < min high`, the
  same decision `std_string::ranges_overlap` publishes, restated on the IR's
  `u64` axis so the two layers cannot drift by a conversion.

## 2. The construct

```qcore
let store = bytes(8);
let left  = lend(store, 0, 4);   // write authority over [0, 4)
let right = lend(store, 4, 8);   // write authority over [4, 8)
left[0]  = 11;                   // writes store[0]
right[0] = 55;                   // writes store[4]  — both loans live
let b = right[1];                // reads through the loan (rebased)
let a = left[1];                 // right's window closed first: windows nest
let x = store[0];                // the owner's authority is back
drop store;
```

**The bounds are the loop bound's tier, deliberately.** `LOW` and `HIGH` are an
integer literal, a literal `let`, or an `extent` binding — narrower than the
index tier, because the custody projection must *state* the lend's footprint
and a bound only the checker's byte model can compute would be decided in a
second place. A bound read out of a byte refuses by name
(`WRITE-LEND-BOUND-FROM-BYTE`, the `LOOP-BOUND-FROM-BYTE` shape).

**The window is the read card's window.** From the lend to the last statement
naming the loan, exact because the route is one flat sequence — and §5 lists
the three premises that exactness is defended by.

**An empty lend holds no byte and conflicts with nothing** — a consequence of
the interval decision, measured on the route
(`an_empty_lend_holds_no_byte_and_conflicts_with_nothing`) and proved in Lean
(`an_empty_lend_is_admitted_beside_any_loan`). **Exclusivity is temporal**: a
closed window frees its range, and lending the same range twice in sequence is
a program (`sequential_lends_of_one_range_are_admitted_…`).

## 3. The custody shape: the rules live in the IR, and the route inherits them

The residual asked for authority *genuinely transferred rather than merely
excluded*, and the operational content of that phrase is **where the refusals
come from**. Every custody refusal of the lend arrives at the route through the
inherited-checker arm (`BufferError::Custody`, `Q054`); the route re-implements
none of them, on the `PIPE-CHAIN` discipline — what changed is that the
cycle-3 checker itself grew the rules, in the one semantic model where every
other custody rule already lives, with matching Lean statements (§8):

| Rule | IR decision | Refusal value |
|---|---|---|
| **Exclusive overlap between two lends** | a second `LendRange` of the same resource is admitted **exactly when its range is disjoint** from every live loan's | `OverlappingLends` |
| **Owner write inside a lent range** | a `WriteRange` through the owner refuses exactly when its interval shares a point with a live loan's; admitted outside every one | `OwnerMutationWhileBorrowed`, now range-wise |
| **Owner read inside a lent range** | same decision against the loans carrying write authority; a shared borrow blocks no read; an empty interval (an extent query) conflicts with nothing | `OwnerAccessWhileMutablyBorrowed`, now range-wise |
| **Rebasing containment** | an access through a loan must state a footprint inside the lend's own range; a rangeless one states none and refuses | `AccessOutsideLendRange` |
| **Release under a live lend** | the **existing** `has_any_borrow` drop rule, with no new rule beside it — a lend is a borrow, and what ends at a release is the allocation, not a byte | `DropWhileBorrowed`, unchanged |
| **Re-lending** | a loan is not an owner | `LendOfBorrow` |
| **A write lend against a whole-resource borrow** | any non-empty lend shares a byte with a borrow that holds every byte, in either order | `MutableBorrowWhileBorrowed` / `SharedBorrowWhileMutablyBorrowed`, unchanged |

Two accesses state their footprints through two new flat operations,
`WriteRange` and `ReadRange`. **An access the projection cannot resolve stays
rangeless and conflicts with everything** — a `Write(owner)` under any live
lend refuses — which is the read card's "not provably disjoint conflicts with
every live loan", reached from the write side and measured as
`an_owner_index_the_projection_cannot_resolve_conflicts_with_every_live_lend`
(`WRITE-LEND-COMPUTED-INDEX`: an index a `let k = b[6];` binds is known to the
checker's byte model but not to the custody projection's literal tier, so the
statement refuses even where the value is in fact outside every lend —
strictly the refusing direction).

**A lend moves no quantity.** The node adds no ledger entry, the live vector is
the owner's throughout, and `peak_live_bytes` cannot move at a lend — which is
also why an *acquisition* inside a lend window is refused rather than admitted
(§5).

**A module with no lend builds the operation list it always built.** That is a
structural branch in `custody_operations_with_gates`, not a measurement — and
it is measured anyway
(`a_module_with_no_lend_builds_the_operation_list_it_always_built`), beside the
five neighbouring suites running unmoved at their pinned counts (§6).

## 4. Diagnostics: zero codes, five widened conditions and one composed refusal

The `Q-NAMESPACE-DECISION` ice box forbids allocating a code, so every refusal
lands on a condition an existing band already names, stated in spec §L21 (both
amended rows) and here.

| Refusal | Code | Widening |
|---|---|---|
| a `for` in an entry that lends; crossed windows; an acquisition inside a window; a malformed `lend(…)`; a shadowing lend binding | `Q051` | **yes** — the flat-shape condition covers the lend's three window premises |
| a lend of anything but an owned buffer this entry acquired (a scalar, a loan — `WRITE-LEND-RELEND`); a `drop` of a loan | `Q052` | **yes** — the name condition covers the loan value kind |
| a lend range violating `0 <= low <= high <= extent`; an index through a loan outside its own `[0, len)`; a bound outside the lowering-visible tier | `Q053` | **yes** — from index/byte/extent/field-count ranges to those and the lend's |
| every custody refusal of §3's table | `Q054` | **yes** — from the four listed refusals of the unedited checker to *any refusal of the lend-extended cycle-3 checker*; the route still inherits rather than re-implements |
| a loan at a scalar, index or rendering position | `Q055` | **yes** — a lend beside the buffer |
| the certificate recheck (`lends` compared field-for-field) | `Q056` | no — one more compared field |
| a lend in a **composed string/buffer** entry | `Q077` | **yes** — refused by name until the cross-model overlap rule with `borrow_utf8`'s read loans exists (`SLICE-PERMISSIONS-COMPOSED-LEND`) |

Two deliberate double-reports are pinned rather than hidden, both on refused
programs: an out-of-lend index is refused by the value half (`Q053`, in the
loan's own coordinates) **and** by the IR's containment (`Q054`) — two halves
of one law, each in the layer that owns it — and a lend of a scalar cascades
into the collected-errors model's true follow-up. A lend with an out-of-range
*bound* reports once: the value half refuses at `Q053` and registers the loan
with clamped bounds (the refused-acquisition precedent), and the custody
projection clamps identically, so the two layers still agree.

From source, name resolution's `Q012`/`Q013` still fire first where they always
did (a duplicate binding, an unbound bound name); the model-level shadow rule
behind `Q012` is defence in depth and is measured at both layers.

## 5. The window's three premises, each defended by refusal

The window is an interval of statement indices, exact because the route is one
flat sequence — the read card's premise, and a write lend leans on it harder,
because what the window now bounds is *mutation authority*.

1. **A `for` beside a lend refuses** (`Q051`,
   `SLICE-PERMISSIONS-LOOP-WINDOW`): a loop makes one index run many times, so
   a store in a body can precede — in index order — a statement that runs
   before it in time. The same wall `STRING-CONCAT-LOOP-WINDOW` names, reached
   from the write side, and its discharge is the same window-over-iterations.
2. **Crossed windows refuse** (`Q051`, `SLICE-PERMISSIONS-CROSSED-WINDOWS`):
   the ownership IR's borrow nodes are lexical, so two lends either nest or
   stand apart, and a crossed pair is refused by name rather than approximated
   — the `SOCKET-EVENT-CROSSED-SPANS` precedent. The refusal is about the
   *encoding*, not the bytes: the test's crossed pair has disjoint ranges. The
   practical cost is an ordering discipline (close the later lend first — the
   fixture's `let b = right[1]; let a = left[1];`), and the discharge is a
   non-lexical loan representation.
3. **An acquisition inside a window refuses** (`Q051`,
   `WRITE-LEND-NESTED-ACQUIRE`): the one-pass peak walker steps top-level
   operations, so a footprint step inside a borrow node's body could make the
   published `peak_live_bytes` an under-measurement. Refused rather than
   under-measured; the discharge is a peak walker that recurses into loan
   bodies.

## 6. Compatibility, measured

* The five suites that own the surrounding routes are **unmoved at their
  pinned counts**: `std_buffer` 52, `slice_permissions` 25, `string_concat`
  53, `std_string` 62, `string_slice_io` 37, `source_loop` 39, `source_record`
  56 — run before and after, all green, none edited.
* `lend` is **contextual**: `let lend = 5; print dec lend;` is a program
  (pinned), route selection claims only the `lend(` shape, and the census (§10)
  found the bare word only in one comment.
* The certificate gained one field (`lends`) and the recheck compares it with
  the rest; no test was deleted, weakened or re-pinned.
* Emission: a lend emits one `unsigned char *const l_NAME = b_OWNER + LOWu;`
  (no displacement at `LOW == 0`), and every loan access is written in the
  loan's own coordinates against the loan's own length — so the emitted bounds
  witness is the rebased rule, and ASan is the load-bearing sanitizer: an
  off-by-one in displacement, length or rebasing would be an out-of-bounds
  **write**. No program written before this card changes by a byte: every new
  emission path is behind the new event.

## 7. CT07 / CT10 readiness — exactly what a lane can now claim

**This card maps nothing** — `docs/reevaluation/ct_manifest.json` and the
acceptance matrix are untouched. What it supplies:

| Fixture | What it measures |
|---|---|
| `compiler/tests/fixtures/write_lend/disjoint_write_halves.qcore` | two write lends of `[0, 4)` and `[4, 8)` of one eight-byte buffer, **both live across four interleaved stores**, read back through the loans and then by the owner — **admitted**, observation `22, 66, 11, 55`, sanitized-native beside its reference |
| `compiler/tests/fixtures/write_lend/overlapping_lends.qcore` | the identical program with one bound moved from 4 to 3 — one shared byte — **refused at `Q054`** by the exclusivity rule, naming the overlap |

* **CT07** — *"Two disjoint array halves mutated independently → Accept with
  bounds/disjointness proof."* The half the wave-28 mapping recorded as its
  boundary — *"NOT two parties each holding write authority over their own
  range"* — **is now expressible**, on the same sequential route the existing
  mapping uses. A lane re-reading CT07 can either extend the existing mapping's
  semantic profile to the write fixture or remap onto it: the bounds proof is
  `Q053` (both the lend's range and the loan's own index bound, check-time),
  the disjointness proof is the `Q054` exclusivity refusal of the control
  fixture, and the acceptance is the fixture itself with its native run. The
  remaining honest boundary for the profile text: both parties are bindings of
  **one sequential entry** — disjoint write authority, interleaved statements,
  not two threads. Task-crossing is `DISJOINT_FOOTPRINT_PROFILE.md` §8 F1's
  remainder.
* **CT10** — *"Adjacent bit-fields in one memory location → Reject independent
  concurrent writes."* **Not mappable on this card, and a lane should say
  why with the sharper vocabulary it now has.** The rejection *shape* CT10
  wants — two independent write authorities over one memory location refuse —
  is now a measured behaviour (`OverlappingLends` at byte granularity: two
  lends sharing one byte refuse, adjacent byte ranges are disjoint and
  admitted). What CT10 additionally needs is **sub-byte granularity** (two
  bit-fields of one byte are one memory location; this route's atom is the
  byte, so the conservative encoding is that both bit-fields lend the same
  byte — which refuses, the right answer, but by an encoding a lane would have
  to declare in its profile) and **concurrency** (its scenario says
  *concurrent*; this route is sequential). Candidate discharge:
  `WRITE-LEND-TASKS` (below) plus a bit-field-to-byte encoding note, or the
  disjoint-footprint checker's F1 capture.

## 8. Lean

`proof/QCore/WriteLend.lean` (registered in `proof/QCore.lean`, so
`AuditAll.lean` includes it in the whole-project axiom closure) and
`proof/AuditWriteLend.lean` — **28 declarations, 25 audited by name**,
placeholder-free with no project axiom. Every closure is a subset of
`{propext, Classical.choice, Quot.sound}`; ten depend on no axiom at all.
`python3 scripts/check_lean_audits.py` reports **Passed 116 Lean audits**
(115 → 116), no warning, exit 0.

What is proved, in the order this report argues:

* **The exclusivity rule is the disjointness question** —
  `a_second_lend_is_admitted_iff_disjoint` is an equation with
  `!overlaps`, the read card's decision **imported rather than restated**, so
  `overlaps_iff_a_shared_byte` gives "no byte is lent twice"
  (`two_admitted_lends_share_no_byte`) for free. The empty lend, and the
  whole-borrow conflict, in both directions.
* **The transfer, as an equation** —
  `the_owner_is_excluded_exactly_where_the_loan_holds`: an owner byte write is
  refused exactly where the lend holds the byte. With two disjoint lends,
  `every_byte_has_exactly_one_write_authority`: the owner's exclusion is
  exactly the union of the two loans, and no byte is in both — the partition
  CT07's "independently" means. The extent query
  (`an_extent_query_is_admitted_beside_any_loan`) and the shared-borrow read.
* **Rebasing** — an in-bounds loan access stays inside its lend
  (`a_rebased_access_stays_inside_its_lend`), is a byte the buffer has
  (composing with the read card's own `admitsRange` bound), and
  `disjoint_lends_rebase_to_distinct_bytes`: two parties writing through their
  own loans can never write one byte, whatever in-bounds indices each names.
* **The release asymmetry** — `a_lend_blocks_the_owners_drop`, however narrow,
  the read card's rule inherited by the write one; and `openLend` takes no
  ledger, which is the model-level statement that a lend moves no quantity.
* **The unlent bytes survive the window** —
  `a_confined_window_preserves_every_unlent_byte`, the mirror image of the
  read card's soundness statement over the same `Step`/`Contents` machinery:
  there, stores *outside* a read loan were shown to preserve the lent bytes;
  here, stores *inside* a write lend preserve every byte outside it. Together
  they are the two halves of one partition, and
  `a_rebased_loan_store_is_confined` ties the arithmetic to the window without
  a second definition of either.
* **What was added**, as one list (`the_added_operation_is_exactly_one`).

What is **not** proved, carried from the family's boundary: that the Rust
lowering extracts events faithfully from tokens (tested), the last-use window
computation (tested), and the composed route's total refusal (a one-arm Rust
boundary).

## 9. Corpus neutrality

By construction. The diff touches `crates/qcore-core` (`ownership.rs`,
`std_buffer.rs`, `std_buffer_c_emit.rs`, `std_string.rs`,
`std_string_c_emit.rs`), `crates/qcore-frontend` (`source_std_buffer.rs`,
`frontend.rs`, `whole_program.rs`), two Lean files plus the `QCore.lean`
import line, one test file plus two fixtures and the unit-test rows, four
documents (spec §L21, two design docs, `DEVELOPMENT.md`), and this report with
its artifacts.

No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`crates/qcore-contract/`, `contract/`, `providers/`, `examples/` or any frozen
archive appears in the diff. The pinned 185-unit artifacts, the leak count of
**266**, the 73-module contract pin and the CT manifest are main's — untouched
and unread. No provider, schema, package template or protocol changed, so
constraint 18's rebuild is not owed and the 122-test provider checkpoint is
carried, not rerun.

## 10. Keyword census (standing constraint 14)

Archived at `artifacts/keyword_census.txt` with reproduction commands. Zero
new reserved words, zero new tokens, one new **contextual** clause word:
`lend`, recognised only as `lend` followed by `(` in a `let`'s value position
(`source_std_buffer::is_lend_clause_word`, consumed by the same
reference-collection site as the string route's three words). `lend(` occurs
**zero** times across 469 `.qcore`/`.qc` files outside this card's own
fixtures; the bare word occurs once, in a comment. The read card's census had
already recorded `lend` free and reserved it for this card — this census
re-ran the measurement rather than assuming it. Rejected and recorded: reusing
`bytes` at arity three, because one word meaning an acquisition or a loan by
arity would make the `Alloc` derivation a function of an argument's kind.

## 11. Gate

Run in this worktree on main `5d8b2ff1` plus this card, with `$HOME/.elan/bin`
on `PATH` for the Lean row and the submodule initialised before the workspace
run.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the workspace run |
| `cargo test --workspace --no-fail-fast` | **4,430 passed, 0 failed, 8 ignored** across **214** result lines, exit **0** — §11.1 |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 116 Lean audits** (115 → 116: `AuditWriteLend.lean`), no warning, exit **0** — read from the text line, per the standing environment note |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, **1 expected failure** — §11.2 |
| `compiler/tests/write_lend.rs` | **31 passed, 0 failed** |
| `compiler/tests/std_buffer.rs` | **52 passed** (unmoved) |
| `compiler/tests/slice_permissions.rs` | **25 passed** (unmoved) |
| `compiler/tests/string_concat.rs` | **53 passed** (unmoved) |
| `compiler/tests/std_string.rs` | **62 passed** (unmoved) |
| `compiler/tests/string_slice_io.rs` | **37 passed** (unmoved) |
| `compiler/tests/source_loop.rs` | **39 passed** (unmoved) |
| `compiler/tests/source_record.rs` | **56 passed** (unmoved) |

### 11.1 The workspace row

The wave-28 close measured 4,392 (8 ignored); the difference is this card's
own 38 rows — 31 in `write_lend.rs` and 7 IR-level unit rows — and nothing
else. Constraint 16's load-sensitive rows: see the gate log
(worktree-private `measure/workspace_tests.log`); any row that tripped under
the concurrent suite was re-verified in isolation and both facts are recorded
here.

### 11.2 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`lean_audits: 115 != 116` — the derived-value tripwire working as designed:
this card adds one audited Lean module and may not edit the acceptance matrix.
The same one-line reconciliation every wave's new audit file receives
(constraint 25). The other 192 Python tests pass.

### 11.3 Measurement hygiene

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`, `README.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Five things are the integrator's.

1. **The proposed amendment to standing constraint 15h** — the card was
   briefed to supersede 15h's "There is **no write lend**" sentence
   deliberately and to propose the text rather than edit it:

   > **15h (amended, card `SLICE-PERMISSIONS-WRITE-LEND`).** The sentence
   > "There is **no write lend**" is superseded — by the card that sentence's
   > own residual named. A write lend now exists, **on the uncomposed buffer
   > route only**: `let NAME = lend(BUFFER, LOW, HIGH);` transfers write
   > authority over `[LOW, HIGH)` of an owned buffer for the loan's lexical
   > window (the lend to its last use), decided by the **lend-extended
   > cycle-3 ownership IR** rather than by a route rule — a `LendRange` borrow
   > node carries the half-open byte interval `STD-BUFFER-SLICE` predicted,
   > and the IR decides that two live lends are admitted exactly when their
   > ranges are disjoint (`max low < min high`, the read card's own decision
   > on the IR's axis), that an owner access is refused exactly where its
   > stated footprint meets a live lend and admitted outside every one (reads
   > included; an extent query's empty footprint meets nothing), that an
   > access through a loan must state a rebased footprint inside the lend
   > (rangeless conflicts with everything — an index the projection cannot
   > resolve from the literal tier refuses even where its value is outside
   > every lend, `WRITE-LEND-COMPUTED-INDEX`), and that a release under any
   > live lend refuses however narrow, by the **existing** drop rule with no
   > new rule beside it. Do not move these rules out of the IR into a route,
   > and do not give the release arm a range. On the **composed string route
   > every sentence of this constraint stands unchanged**: a `lend(…)` beside
   > `borrow_utf8`'s read loans refuses by name at `Q077` until the
   > cross-model overlap rule exists (`SLICE-PERMISSIONS-COMPOSED-LEND`), so
   > the window-exactness argument and the `for` refusal at `Q068` are
   > untouched there. On the buffer route the same exactness premise is
   > defended by three refusals that must survive refactors: a `for` in an
   > entry that lends (`SLICE-PERMISSIONS-LOOP-WINDOW`), a crossed pair of
   > lend windows — borrow nodes are lexical, so lends nest or stand apart
   > (`SLICE-PERMISSIONS-CROSSED-WINDOWS`) — and an acquisition inside a
   > window, which would hide a footprint step from the one-pass peak walker
   > (`WRITE-LEND-NESTED-ACQUIRE`). The lend's bound tier is the loop bound's
   > (a bound from a byte refuses, `WRITE-LEND-BOUND-FROM-BYTE`); a loan is
   > never released and never re-lent (`WRITE-LEND-RELEND`); and a module
   > with no lend builds the custody operation list it always built, by a
   > structural branch. `slice-permissions-write-lend`.

2. **The `AGENTS.md` namespace table**: the `Q051`–`Q057` `StandardBuffer` row
   should record the five condition widenings of §4 (authority: spec §L21,
   updated by this card), and the `Q068`–`Q079` `StandardString` row the one
   `Q077` widening (a composed lend refuses by name).

3. **The derived `lean_audits` count moves by one** (115 → 116) — §11.2's
   tripwire, the usual one-line matrix reconciliation (constraint 25).

4. **Public API changes**, all in `qcore-core` and `qcore-frontend`:
   * `qcore_core::ownership`: `OwnershipOp` gains `LendRange`, `WriteRange`,
     `ReadRange`; `OwnershipError` gains `OverlappingLends`, `LendOfBorrow`,
     `AccessOutsideLendRange`. A downstream exhaustive `match` on either needs
     updating (`whole_program.rs`'s identity collector was the only one).
   * `qcore_core::std_buffer`: `BufferEvent` gains `Lend`; `BufferError` gains
     `Lend` carrying the new public `LendRefusal`; `BufferCertificate` and
     `BufferRecheckFailure` gain `lends`/`Lends`; `BufferStep` gains `lend`;
     `BufferEvaluation` gains `loans` (public `LoanShape`); `LendRecord`,
     `LendWindow`, `lend_windows`, `crossed_lend_windows` are new and public.
   * `qcore_core::std_buffer_c_emit`: `BufferEmissionRefusal` gains
     `UnemittableLend`; `lend_name` is new.
   * `qcore_frontend::source_std_buffer`: `LEND_KEYWORD`,
     `is_lend_clause_word`, `BufferUse::Lend` are new.

5. **CT07's existing mapping** may be extended per §7 by a lane, never by this
   card.

## 13. Named follow-ons

| Name | What is missing | Discharge condition |
|---|---|---|
| `SLICE-PERMISSIONS-COMPOSED-LEND` | a write lend beside `borrow_utf8`'s read loans on the composed string route — refused by name today, which trivially refuses every overlapping pair | the cross-model overlap rule: a lend overlapping a live read loan refused, a disjoint one admitted, with the read window and the lend window decided against one statement index space |
| `SLICE-PERMISSIONS-CROSSED-WINDOWS` | two lends whose windows cross (disjoint ranges included) — the borrow-node encoding is lexical | a non-lexical loan representation in the IR, or open/close loan operations with the nesting restriction proved unnecessary |
| `SLICE-PERMISSIONS-LOOP-WINDOW` | a `for` in an entry that lends | a window over iterations rather than statement indices — the same discharge `STRING-CONCAT-LOOP-WINDOW` names |
| `WRITE-LEND-BOUND-FROM-BYTE` | a lend bound read out of a byte or a field | either a custody projection that reads the checker's byte model, or the relational domain `SLICE-PERMISSIONS-VARYING-RANGE` names |
| `WRITE-LEND-COMPUTED-INDEX` | an owner or loan access whose index the literal tier cannot resolve, under a live lend — refused even where the value is disjoint | the same projection upgrade; until then the rangeless refusal is the fail-closed floor |
| `WRITE-LEND-NESTED-ACQUIRE` | an acquisition inside a lend window | a peak walker that recurses into borrow-node bodies, measured against the one-pass number |
| `WRITE-LEND-RELEND` | `lend(LOAN, …)` — no transitive loans | a sub-lend that narrows (the read card's slice-of-a-borrow shape, with authority) |
| `WRITE-LEND-WINDOWS-RUN` | this card ran the **Linux** native arm only; the test file carries the `windows_c` path unchanged | the `STRING-CONCAT-WINDOWS-RUN` precedent: an MSVC run of `write_lend.rs`'s native test |
| `WRITE-LEND-TASKS` | both parties are bindings of one sequential entry | a capture lending two loans to two children — `DISJOINT_FOOTPRINT_PROFILE.md` §8 F1's remainder, where the interval moves onto that checker's derived footprints |
| `SLICE-PERMISSIONS-VARYING-RANGE` | *(unchanged, the read card's)* a range the compiler cannot evaluate | a relational domain over indices and a run-time failure arm |

## 14. Artifacts

- `artifacts/keyword_census.txt` — the constraint-14 census, with reproduction
  commands and the rejected `bytes`-arity alternative recorded.
- `artifacts/disjoint_write_halves.c` — the emitted C11 for the CT07-shaped
  fixture. The card is readable in it without the certificate: two `const`
  pointers displaced into one `calloc`ed block (`l_left = b_store`,
  `l_right = b_store + 4u`), four interleaved stores each bounded by **its own
  loan's** length, and the owner's reads standing after the loans' last uses —
  which is the transfer, in C.
