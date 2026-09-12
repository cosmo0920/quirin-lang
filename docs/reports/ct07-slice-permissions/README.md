# CT07-SLICE-PERMISSIONS — lending a range, and the blocker that was about a different operation (wave 27)

Card brief: land **sub-extent lending** — *a borrowed range of an owned buffer*
— the capability whose absence is the recorded blocker for CT07/CT08's
disjointness and part of CT25's gap.

Base: `origin/main` at `f122cc03` (the wave-26 close), isolated worktree, branch
`claude/wave-23-hatch-opening-f8b35d`.

Two records name the gap. Card `STD-BUFFER` recorded it as `STD-BUFFER-SLICE`:

> a borrowed sub-range. Needs the ownership IR's borrow nodes to carry a
> footprint *fraction*, which they do not

and the wave-26 CT lane (`ct-wire-lane` §3.1) recorded the consequence:

> *Independently* in CT07 means two parties holding disjoint permission over two
> ranges, and **no source form lends a sub-extent of a buffer to anyone**

**What landed.** One construct, one widened condition, zero new diagnostic
codes, zero new reserved words, zero new tokens.

* **`borrow_utf8(NAME, LOW, HIGH)`** — the existing `std.buffer` → `std.string`
  bridge, lending the bytes `[LOW, HIGH)`. Both bounds are check-time constants
  decided against the extent the acquisition declared.
* **The exclusion becomes range-wise.** A write whose byte lies inside a live
  loan is refused; **a write outside every live loan is admitted**. That
  admission is the card: two disjoint permissions over one buffer are now
  expressible, and the disjointness is *decided* rather than assumed.
* **The whole-extent form is that rule's degenerate case**, measured rather than
  asserted: `borrow_utf8(b)` and `borrow_utf8(b, 0, extent)` produce the same
  certificate, the same observation and byte-identical emitted C, and the Lean
  statement is an **equation** with `QCore.StringConcat.excluded` — the
  definition the route has run since card `STRING-CONCAT`.

**24** end-to-end tests in `compiler/tests/slice_permissions.rs`, one of them a
sanitized native run with a positive leak control. **39** declarations in a new
`QCore.SlicePermissions`, **36** of them audited by name. Two published fixture
programs a CT lane can point at. **Corpus-neutral by construction** (§9).

---

## 1. The headline: the recorded blocker is about an operation this card did not land

`STD-BUFFER-SLICE`'s blocker sentence is *"needs the ownership IR's borrow nodes
to carry a footprint fraction"*. **The ownership IR's borrow nodes are not
involved in this at all**, and the measurement is the whole shape of the card.

`crates/qcore-core/src/ownership.rs` has `OwnershipOp::BorrowShared` and
`BorrowMutable`, whose conflict test (`has_any_borrow`) is per **resource** and
carries no range. But `source.string.v1` **does not use them**: constraint 15h
records that *"a borrow contributes an ordinary `Read` to the cycle-3 ownership
operation list"*, and `std_string::string_custody_operations` emits exactly
`OwnershipOp::Read(buffer)` for a `borrow_utf8`. The exclusion window is the
string route's **own** rule at `Q078`, over an interval of statement indices —
not a borrow node. So giving a loan a footprint is a change to that rule's
state, and the ownership IR is untouched by this card: `ownership.rs` does not
appear in its diff.

That is the same shape card `STRING-CONCAT` reported for its own inherited
dependency ("the dependency the brief inherited is **false**"), and it is worth
stating in the same words: the soundness order the blocker describes
("fractions first, because a sub-range loan needs one") is about a **write**
lend, where two parties each hold exclusive authority over their own range. This
card lends **read** permission and leaves the write authority with the owner, so
what it needed was a range on the route's own window and nothing else.

The follow-on that does need the fraction is named in §10 as
**`SLICE-PERMISSIONS-WRITE-LEND`**, and §8 says exactly what CT07 can and cannot
claim because of the difference.

---

## 2. The construct

```qcore
let head = borrow_utf8(store, 0, 2);    // bytes [0, 2)
let tail = borrow_utf8(store, 4, 6);    // bytes [4, 6)
store[2] = 88;                          // admitted: byte 2 is in neither loan
print head;                             // "ab"
print tail;                             // "ef"
```

**The arity is one or three and nothing between.** A range needs both of its
ends, so a two-argument spelling has no reading and keeps refusing by shape
exactly where `STRING-CONCAT` pinned it — that card's
`a_malformed_bridge_refuses_by_shape` is untouched and green, and this card adds
`a_two_argument_bridge_still_refuses_by_shape` beside it with the message the
new parser writes.

**The bounds are the slice rule's bounds.** `LOW` and `HIGH` are resolved by the
same `Evaluation::operand` a slice bound is: an integer literal, or a name this
entry bound to a constant. A bound naming nothing refuses at `Q013` before this
route sees it; a buffer or a string at a bound position refuses at `Q072`, where
every other scalar position on this route refuses one. Nothing about the operand
rule is new, and both directions are pinned.

**The event carries `range: Option<(StringOperand, StringOperand)>`.** `None` is
the pre-card spelling, which is what keeps an existing program's *parsed model*
identical too: it gains a field whose value is `None` and nothing else.

---

## 3. The exclusion rule, and the two asymmetries that are deliberate

### 3.1 The decision

A write `b[i] = v` enters as the one-byte range `[i, i + 1)`, and conflicts with
a loan exactly when the two ranges share a byte:

```rust
pub fn ranges_overlap(low_a: u32, high_a: u32, low_b: u32, high_b: u32) -> bool {
    low_a.max(low_b) < high_a.min(high_b)
}
```

**This is an equality with the set-theoretic question, not an approximation of
it.** `QCore.SlicePermissions.overlaps_iff_a_shared_byte` proves
`overlaps a b = true ↔ ∃ i, a.contains i ∧ b.contains i`. Every bound on this
route is a check-time constant, so "provably disjoint" and "disjoint" are one
predicate and the fail-closed clause — *anything not provably disjoint refuses*
— has no third case to cover. That is why this card can admit something without
weakening anything: it is not trading refusal for precision, it is deciding a
question that was previously not asked.

An **empty** range shares no byte with anything, which falls out of the decision
rather than being a case beside it (`an_empty_range_overlaps_nothing`), and
adjacency is disjointness (`adjacent_ranges_are_disjoint`), which is what makes
"the two halves of a buffer" expressible without an off-by-one.

### 3.2 The first asymmetry: a release is **not** range-wise

A release ends the storage every loan addresses, so it is refused for any live
loan however narrow — including an empty one. Making that arm range-wise would
let a program free the bytes a live view reads, which is the one thing the
window exists to prevent. Two separate definitions in Lean
(`excludedStore`, `excludedRelease`) and two arms in Rust, with
`releasing_a_buffer_refuses_for_a_loan_however_narrow` measuring a one-byte loan
of a four-byte buffer.

### 3.3 The second asymmetry: an index the route cannot decide conflicts with everything

The exclusion decision runs **before** `source.buffer.v1` judges the statement —
that ordering is card `STRING-CONCAT`'s, so that a statement refused for writing
through a live borrow says so rather than reporting a byte it never wrote. An
index this route cannot resolve to a non-negative constant is therefore *not
provably disjoint* from anything and conflicts with every live loan of that
buffer. `buffer_constant` returns `Option<u32>` and every caller reads `None`
that way; `an_index_this_route_cannot_decide_conflicts_with_every_live_loan`
pins it. The alternative — falling through to the buffer model — would have made
an unresolvable index an *admission* at this gate, and the fact that the buffer
model refuses it a moment later is not a reason to write a fail-open line.

### 3.4 What did **not** become a refusal, measured

The brief asked for *"overlapping second borrow refused"*. **It is not
implemented, and the measurement is the argument.**

Two overlapping `borrow_utf8` views conflict on nothing: both are immutable, and
neither can write. Refusing an overlap would also refuse the mechanism
constraint 15h rests on — *"a slice of a borrow is itself a borrow of the same
buffer"* — because a slice of a loan is a **fully overlapping second loan of the
same buffer by construction**, which is how a derived view extends the window at
all (`STRING-CONCAT`'s `a_slice_of_a_borrow_extends_the_window_the_borrow_excludes_over`,
still green). And it would break this card's own degenerate-case requirement:
two whole-extent borrows overlap totally, and `let s = borrow_utf8(b); let t =
borrow_utf8(b);` with both live is a program `STRING-CONCAT` admits.

So the conflict this route decides is between a **write** and a loan, which is
where the conflict actually is. What the brief wanted from that rule — CT08's
*"overlapping array ranges → reject"* — is delivered by exactly that decision,
and §8 says so in the lane's own terms. A rule that refuses two overlapping
*write* lends belongs to `SLICE-PERMISSIONS-WRITE-LEND`, where two parties do
conflict.

### 3.5 The messages

A **whole-extent** loan reports the sentence this route reported before ranges
existed, byte for byte — `a_whole_extent_loan_refuses_a_write_with_the_sentence_it_always_refused_it_with`
asserts both that the old text is there and that no range clause was appended. A
narrower loan appends its footprint, because the useful next question for its
reader is *which* write would have been admitted:

```
… (statement 7 of this entry); `head` borrows bytes [0, 2) of `store` and this
write is inside that range — a write outside every live borrow's range is
admitted
```

and a release of a narrow loan says the opposite thing, for the reason in §3.2.

---

## 4. The degenerate case, proved twice

The claim "the pre-card construct is this one at `[0, extent)`" is measured on
both sides.

**In Rust**, `a_whole_extent_range_and_the_whole_extent_spelling_certify_identically`
compiles the two spellings of one program and asserts equality of
`certificate.borrows`, `certificate.views`, `certificate.lines`, the emitted C
source and the reference observation. Not one of those is a proxy: the borrow
records carry the new `low`/`high` fields, so their equality *is* the statement
that the whole-extent form records `[0, 4)`.

**In Lean**, `the_whole_extent_decision_is_the_shipped_one` is an equation
between this module's `excludedStore` and `QCore.StringConcat.excluded` — the
definition the route has been running since card `STRING-CONCAT`, imported
rather than restated — for a loan of the whole extent at every statement index
and every in-bounds byte. `the_release_decision_is_the_shipped_one` is the same
equation for the release arm, and needs no side condition at all. Nothing was
replaced; the old rule is the new one at its degenerate bound.

**In emission**, the displacement is written only when there is one:

```c
qcore_string_view v_head = { b_store, (size_t) 2u };        /* [0, 2) */
qcore_string_view v_tail = { b_store + 4u, (size_t) 2u };   /* [4, 6) */
```

so a whole-extent borrow emits the byte-identical initialiser it always emitted
(`a_sub_extent_loan_emits_a_displaced_pointer_and_a_whole_one_emits_none`), and
no program written before this card changes by a character.

**And the four suites that own the surrounding routes are unmoved at their
pinned counts**: `string_concat` 53, `std_string` 62, `string_slice_io` 37,
`std_buffer` 52, `source_loop` 39 — run before and after, all green, none
edited. The certificate gained two fields and the recheck compares them with the
rest; no test was deleted, weakened or re-pinned.

---

## 5. The loop, measured rather than assumed

Follow-on `STRING-CONCAT-LOOP-WINDOW` asked what a `for` beside a live borrow
does. Re-measured for this card with a **range** loan and a body writing
**outside** it:

```
Q068: `source.string.v1` models one flat statement sequence;
      this statement opens a nested block
```

**The range does not rescue the loop, and the reason is that they are about
different things.** The window is an interval of *statement indices*, exact only
because the route is one flat sequence; a range narrows **which** writes
conflict and says nothing about **when** a statement runs. A loop body makes one
index run many times, so a store inside a body can precede — in index order — a
read that happens after it in time, whatever range the store names. The refusal
is unchanged, at the earliest point rather than the latest, and the model's own
`Repeat` guard behind it is untouched.
`a_loop_beside_a_range_loan_still_refuses_and_the_range_does_not_rescue_it` is
the measurement. `STRING-CONCAT-LOOP-WINDOW` therefore **stands unchanged**:
what it needs is still a window over iterations rather than over indices.

---

## 6. Diagnostics: zero codes, one widened condition

The `Q-NAMESPACE-DECISION` ice box forbids allocating a new code, so every
refusal lands on a condition the `Q077`–`Q079` band already names. The widening
is stated in `docs/QCORE_SPEC_r0_3.md` §L21 and here rather than taken silently.

| Refusal | Code | Widened? |
|---|---|---|
| a range bound violating `0 <= low <= high <= extent` | `Q077` | **no** — the band's condition is already "the shape **or the bound** of a construct this band adds" |
| a two-argument `borrow_utf8(…)` | `Q077` | no — the pre-card shape refusal, unmoved |
| a bound that is not a check-time constant | `Q069` / `Q013` | no |
| a buffer or string at a bound position | `Q072` | no |
| a write **into the bytes of** a live loan | `Q078` | **yes** — from "a store into … a buffer inside a live borrow's window" to "a store into the **bytes** of a live loan" |
| a release of a buffer any live loan holds | `Q078` | no |
| a range whose bytes are not well-formed UTF-8, **including one whose end splits a code point** | `Q079` | **no** — that condition was always "the bytes a `borrow_utf8` would view" |

`Q079` not being widened is the one worth reading twice. The condition is stated
about the bytes the borrow views, so narrowing the view narrows the bytes the
decision is made on, and a range ending between `0xC3` and `0xA9` refuses there
by the rule that was already written — `a_range_that_splits_a_code_point_refuses_at_the_utf8_decision`
measures both that refusal and the admitted range that stops before the code
point, which is what makes it a statement about bytes rather than about ranges.
It is deliberately **not** `Q071`, whose condition is a bound on a *view*
decided against `len(s)`; this is a bound on an owned **buffer** decided against
its declared extent, and the `Q076`-rather-than-`Q069` and
`Q079`-rather-than-`Q070` precedents both govern.

---

## 7. Keyword census (standing constraint 14): zero new words, zero new tokens

Archived at `artifacts/keyword_census.txt`. `borrow_utf8` has been a contextual
clause word since `STRING-CONCAT`; `,` and `)` are existing tokens. The
load-bearing measurement is that **no `.qcore`/`.qc` file on disk spells the new
arity** — 0 occurrences of `borrow_utf8( … , … )` across 435 files — so no
shipped program's meaning changes. Route selection is untouched in both
directions: the condition is still `has_source_string_bridge`, which reads
`borrow_utf8` followed by `(` and knows nothing about arity.

Recorded rather than merely avoided: **`part`** occurs in *binding* position in a
shipped fixture (`compiler/tests/fixtures/std_string/silent.qcore`), so
reserving it would refuse that file at `Q013` — the `buffer` → `bytes` and
`join` precedent. `lend`, `range`, `low`, `high`, `sub` and `window` are free at
zero source-position occurrences and are recorded for a later card.

---

## 8. CT07 / CT08 readiness — exactly what a lane can now claim

**This card maps nothing.** `docs/reevaluation/ct_manifest.json` and the
acceptance matrix are untouched; a lane card maps, and this one enables. What it
supplies is two published fixture programs and the tests that hold them up.

| Fixture | What it measures |
|---|---|
| `compiler/tests/fixtures/slice_permissions/disjoint_halves.qcore` | two loans of `[0, 2)` and `[4, 6)` of one six-byte buffer, live at the same time, with the owner writing bytes 2 and 3 between them — **admitted**, with the observation `ab`, `ef`, `XY` proving each view read its own bytes and that the owner's writes landed |
| `compiler/tests/fixtures/slice_permissions/overlapping_write.qcore` | the identical program with one byte of the write moved to byte 1, inside `[0, 2)` — **refused at `Q078`**, naming the footprint that decided it |

Held up by `two_disjoint_loans_are_live_at_once_and_a_write_between_them_is_admitted`,
`the_same_program_with_one_byte_moved_into_a_lent_range_refuses` and
`the_disjoint_permissions_program_runs_natively_and_matches_its_reference` (the
emitted C built under `-Wall -Wextra -Werror -pedantic` and run under
ASan/UBSan/LSan, compared per stream with the checked reference).

**What a lane can claim, stated as narrowly as it is true.**

* **CT08** — *"Overlapping array ranges → Reject or analyzer potential/unknown"*.
  This is mappable now on the second fixture. Two ranges of one array overlap on
  exactly one byte, the overlap is decided at check time by
  `max low < min high`, and the program is **rejected** with the code and the
  message naming the range. The boundary control is the first fixture: the same
  program with the byte moved outside is admitted, so the refusal is keyed to the
  overlap rather than to writing a borrowed buffer at all. It is a distinct case
  from CT11, which is the whole-extent alias with no range in it.
* **CT07** — *"Two disjoint array halves mutated independently → Accept with
  bounds/disjointness proof"*. **Half of it is mappable and half is not, and a
  lane must say which.** The bounds proof and the **disjointness** proof are both
  here, decided rather than assumed, and two disjoint permissions over two ranges
  of one array now coexist. But one of the two permissions is a **read** loan and
  the other is the owner's retained write authority: the fixture has one range
  written while the other is borrowed, which is disjoint permission but not *two
  parties mutating*. A lane that maps CT07 on this fixture must record that
  boundary in its semantic profile in the CT06/CT11 style, or it is claiming more
  than the tree does. The missing half is `SLICE-PERMISSIONS-WRITE-LEND`.
* **CT25** — unchanged by this card. Its gap is loan expiry *with joins*, and
  this loan lives on the sequential route, which has neither joins nor cleanup —
  the same reason the wave-26 lane gave.

The wave-26 lane moved CT07 and CT08 to the bucket
`blocked-on-slice-permission-surface (bounds decided, disjointness not)`. **That
bucket's reason is now retired**: the slice-permission surface exists and
disjointness is decided. A lane re-reading them should say so in its own
stage-evidence record (constraint 23's `retired_unmapped_reasons`), never by
editing the wave-26 record.

---

## 9. Corpus neutrality

By construction. **Fifteen files**, and the list is short enough to read whole:
three in `crates/qcore-core` and `crates/qcore-frontend` (the model, the emitter,
the parser), five normative or design documents, two Lean files plus the one
import line in `proof/QCore.lean`, one test file, two fixtures, and this report
with its two artifacts.

No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`crates/qcore-contract/`, `contract/`, `providers/`, `examples/` or any frozen
archive appears in the diff. `crates/qcore-core/src/ownership.rs` does not
either (§1). The pinned 185-unit artifacts, the leak count of **266**, the
73-module contract pin and the CT manifest are main's — untouched and unread. No
provider, schema, package template or protocol changed, so constraint 18's
rebuild is not owed and the 122-test provider checkpoint is **carried, not
rerun**.

---

## 10. Lean

`proof/QCore/SlicePermissions.lean` (registered in `proof/QCore.lean`, so
`AuditAll.lean` includes it in the whole-project axiom closure) and
`proof/AuditSlicePermissions.lean` — **39 declarations, 36 audited by name**,
placeholder-free with no project axiom. Every closure is a subset of
`{propext, Classical.choice, Quot.sound}`; eight depend on **no axiom at all**,
including the whole added vocabulary, and `Classical.choice` appears where the
decidability lemmas `simp` uses bring it in and in the contrapositive, which is
`Classical.byContradiction`. `python3 scripts/check_lean_audits.py` reports
**Passed 111 Lean audits** (110 → 111), no warning, exit **0**.

What is proved, in the order this report argues:

* **The decision is the question.** `overlaps_iff_a_shared_byte`, with symmetry,
  the empty range, adjacency, and the disjointness form the soundness argument
  uses.
* **A write is a one-byte range** — `a_write_conflicts_exactly_when_its_byte_is_lent`,
  which is what makes "is this write inside that loan" and "do these two loans
  overlap" one question decided by one function.
* **The exclusion, in both directions** — refused inside, admitted outside,
  admitted after the window, constraining no other buffer; and the release arm
  refused for any live loan however narrow.
* **Two disjoint loans partition their writes**, and a write outside both is
  admitted by both.
* **The degenerate case as an equation** with the shipped definition (§4).
* **Soundness, byte by byte** — `an_excluded_range_preserves_its_bytes`: over a
  window in which no statement stores inside the lent range and none releases,
  every byte the loan's view addresses is the byte that was there when its UTF-8
  decision was made. This is the range-wise generalisation of `STRING-CONCAT`'s
  `an_excluded_window_preserves_the_buffers_bytes` and is **strictly stronger in
  the useful direction**: it survives stores the old hypothesis forbade, which is
  exactly the programs this card admits. Its contrapositive is the form the
  checker runs.
* **The bound** — an admitted range lends only bytes that exist; an inverted one
  and one past the extent are refused rather than silently emptied; the whole
  extent is admitted.
* **What was added**, as one list (`the_added_operation_is_exactly_one`), so a
  widening changes a theorem rather than slipping past — and in particular there
  is **no write lend**.

What is **not** proved, carried over from the family's boundary: that the Rust
lowering extracts these events faithfully from source tokens (a tested Rust
boundary), and anything about the UTF-8 model beyond what `QCore.StringConcat`
already states.

One Lean note worth recording because it cost a cycle: unfolding a definition
**inside** a `simp` set rewrites the proposition under `decide` without
rewriting `decide`'s own instance argument, and the goal stops being
type-correct. `a_write_conflicts_exactly_when_its_byte_is_lent` restates its goal
with `show` for that reason, and the comment says so.

---

## 11. Gate

Run in this worktree on main `f122cc03` plus this card, with `$HOME/.elan/bin`
on `PATH`.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree **before** the workspace run |
| `cargo test --workspace --no-fail-fast` | **GATE-WORKSPACE** |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **GATE-PROBE** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **GATE-FOLLOWUP** |
| `python3 scripts/check_lean_audits.py` | **Passed 111 Lean audits** (110 → 111: `AuditSlicePermissions.lean`), no project axiom, exit **0** |
| `python3 -m unittest discover -s scripts/tests` | **GATE-PYTHON** |
| `compiler/tests/slice_permissions.rs` | **24 passed, 0 failed** |
| `compiler/tests/string_concat.rs` | **53 passed, 0 failed** (unmoved) |
| `compiler/tests/std_string.rs` | **62 passed, 0 failed** (unmoved) |
| `compiler/tests/string_slice_io.rs` | **37 passed, 0 failed** (unmoved) |
| `compiler/tests/std_buffer.rs` | **52 passed, 0 failed** (unmoved) |
| `compiler/tests/source_loop.rs` | **39 passed, 0 failed** (unmoved) |

### 11.1 The Lean toolchain is not on the default `PATH`

`lean`, `lake` and `elan` live at `$HOME/.elan/bin` and are **not** on this
worktree's default `PATH`; `scripts/check_lean_audits.py` reports a missing
`lake` **and exits 0** in that state, so the row above is read from the
"Passed 111 Lean audits" **text** and not from the exit code alone. That is the
environment note cards `STRING-SLICE`, `STRING-SLICE-IO` and `STRING-CONCAT` each
recorded, and it recurred; the worktree-private `measure/lean_gate.sh` exists
only to export that directory.

### 11.2 Measurement hygiene

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad. The workspace run is driven
from a detached script writing a sentinel, for the reason card `STRING-CONCAT`
recorded: a run piped through the harness's own background machinery has been
terminated mid-suite on this repository before.

---

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`, `DEVELOPMENT.md`, `README.md` or
`docs/reviews/r0_3/acceptance_matrix.json`. Four things are the integrator's.

1. **No new diagnostic code**, but one **widened condition** to carry into
   `AGENTS.md`'s namespace table: the `Q068`–`Q073`, `Q075`–`Q079`
   `StandardString` row should record that `Q078`'s condition is widened from "a
   store into or a release of a buffer inside a live borrow's window" to "a store
   into the **bytes** of a live loan, or a release of a buffer any live loan
   holds", and that `Q077` now also covers a sub-extent borrow's **bound**. The
   authority is spec §L21, which this card updated; the sentence is there.

2. **A proposed amendment to standing constraint 15h**, offered because it is
   exactly the kind of rule that inventory exists for — an *amendment* rather
   than a new number, since it narrows a sentence 15h already carries:

   > **15h (amended, card `CT07-SLICE-PERMISSIONS`).** … a `borrow_utf8` view is
   > the buffer's own, and since this card it is a view of a **range** of it.
   > `borrow_utf8(NAME, LOW, HIGH)` lends the bytes `[LOW, HIGH)` and the
   > whole-extent spelling is that form at `[0, extent)` — an equation, proved
   > against the shipped definition, not an abbreviation. The window is
   > unchanged; what changed is what it excludes: **a write is refused when its
   > byte lies inside a live loan and admitted when it lies outside every one of
   > them**, with disjointness decided by `max low < min high`, which is the
   > set-theoretic question itself because every bound is a check-time constant.
   > Three asymmetries must survive refactors. A **release** is refused for any
   > live loan however narrow, because it ends the storage all of them address —
   > do not make that arm range-wise. An index the route cannot resolve to a
   > constant is **not provably disjoint** and conflicts with every live loan,
   > because the exclusion is decided *before* `source.buffer.v1` judges the
   > statement. And two overlapping **borrows** are deliberately **not** refused:
   > both are immutable, they conflict on nothing, and a slice of a loan is a
   > fully overlapping second loan of the same buffer by construction, so
   > refusing overlap would refuse the mechanism this constraint already rests
   > on. There is **no write lend** — the owner keeps the write authority over
   > every byte it did not lend — which is why the window's exactness argument
   > survives and a `for` beside a loan still refuses
   > (`STRING-CONCAT-LOOP-WINDOW`, re-measured). `ct07-slice-permissions`.

3. **The derived `lean_audits` count moves by one** for this card's new
   `Audit*.lean`, so `scripts/tests`'s derived-value tripwire reports the stale
   matrix count until the derived section is regenerated — the same one-line
   reconciliation every wave's new audit file receives (constraint 25).

4. **Public API changes**, all in `qcore-core` and `qcore-frontend`:

   * `qcore_core::std_string`: `StringEvent::Borrow` gains a `range` field;
     `BorrowRecord` gains `low` and `high`; `StringError` gains `BorrowRange` and
     `StringError::BorrowExclusion` gains `low`, `high` and `whole`;
     `ranges_overlap` is new and public. A downstream `match` on either enum or a
     construction of either struct needs updating.
   * `qcore_frontend::source_std_string`: no new item; the borrow reader accepts
     the three-argument form.

---

## 13. Named follow-ons

| Name | What is missing |
|---|---|
| `SLICE-PERMISSIONS-WRITE-LEND` | **The half of CT07 this card does not reach.** A lend that carries *write* authority over its range, so two parties mutate two disjoint ranges independently. Needs a second value kind (a loan a store can name), index rebasing into the owner's coordinates, an exclusive-overlap rule between two lends, and — here the `STD-BUFFER-SLICE` blocker finally bites — a custody shape in which the owner's authority over a range is genuinely *transferred* rather than merely excluded, which is what a footprint fraction in a borrow node would be for. §1, §8. |
| `SLICE-PERMISSIONS-VARYING-RANGE` | A range whose bounds the compiler cannot evaluate. Needs a relational domain over indices and a run-time failure arm — the same wall `STD-BUFFER-VARYING-INDEX`, `STRING-SLICE-VARYING-INDEX` and `STRING-SLICE-BORROW-VARYING` name, reached from a third direction. |
| `SLICE-PERMISSIONS-WINDOWS-RUN` | This card ran the **Linux** arm only. The emitted difference is one pointer displacement inside an existing initialiser, and the test file carries the Windows `windows_c` path unchanged, but no Windows or Apple run was performed and none is claimed. The `STRING-CONCAT-WINDOWS-RUN` precedent, and it applies to this card's one native test. |
| `STRING-CONCAT-LOOP-WINDOW` | *(unchanged, and re-measured here — §5.)* A window over **iterations** rather than over statement indices. The range logic does not compose with a loop body and does not claim to; the composed route still refuses a `for` at `Q068`. |
| `STD-BUFFER-SLICE` | **retired in part.** The read sub-range exists. The blocker sentence it carried was measured to be about the write direction, which is `SLICE-PERMISSIONS-WRITE-LEND`. |
| `STRING-SLICE-FREEZE`, `STRING-SLICE-CONCAT-RUNTIME`, `STRING-SLICE-BORROW-VARYING`, `STRING-SLICE-PARSE`, `STRING-SLICE-ABI` | *(unchanged, earlier cards')* |

---

## 14. Artifacts

- `artifacts/keyword_census.txt` — the constraint-14 census, with its
  reproduction commands and the `part` collision recorded for a later card.
- `artifacts/disjoint_halves.c` — the emitted C11 for the CT07-shaped fixture.
  One thing in it is the whole card, readable without the certificate: the three
  views are `{ b_store, 2u }`, `{ b_store + 4u, 2u }` and `{ b_store + 2u, 2u }`
  — three windows onto one `calloc`ed block — and the two `qcore_bytes_store`
  calls for bytes 2 and 3 stand **between** the first two views and their
  renders, which is the admission this card exists for.
