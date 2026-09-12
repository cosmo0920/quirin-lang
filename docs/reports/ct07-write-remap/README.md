# CT07-WRITE-REMAP — CT07's evidence upgraded to the two-write-authorities scenario (wave 30)

**Card:** `CT07-WRITE-REMAP` (wave 30's CT lane). **Constraints engaged:** 21
(the superseded record's bytes are untouched), 23 (a remap is not a mapping —
no count moves), 25 (no matrix section is hand-clobbered; the CT reasons map is
derived and CT07 is mapped, so it has no row there), and the amended 15h (the
write lend is the uncomposed buffer route's, decided by the lend-extended
ownership IR — nothing here re-implements or widens it).

**Headline.** CT07 — *"Two disjoint array halves mutated independently →
Accept with bounds/disjointness proof"* — stays **mapped**; what changes is
the evidence and the honesty statement. The wave-28 mapping
(`CT-WIRE-LANE-2`) rested on the read-loan program and stated its boundary
exactly: *one of the two disjoint permissions is a READ loan and the other is
the owner's retained write authority — NOT two parties each holding write
authority over their own range.* Card `SLICE-PERMISSIONS-WRITE-LEND`
(wave 29) landed exactly that missing half and its §7 named this lane. The
case's artifact is now that card's own published program — **two write
authorities over disjoint ranges of one buffer, both held by one sequential
entry's bindings** — and the boundary sentence that replaces the old one is
the remaining true one: **not two threads** (`WRITE-LEND-TASKS` is the
residual). **No count moves: mapped stays 42/50, unmapped 8,
harness-gated sanitized-native stays 7 of 42, proof stays 1 of 42 (CT01 only,
untouched). This card is evidence quality, not coverage.**

## 1. The semantic profile, before and after

| | Before (wave 28) | After (this card) |
|---|---|---|
| Route/profile | `sequential` / `StandardStringV1` | `sequential` / `StandardBufferV1` (unchanged route, the buffer surface) |
| Case artifact | read-loan program: `borrow_utf8` views of `[0, 2)` and `[4, 6)` of a six-byte buffer, the owner writing bytes 2–3 between them | write-lend program: `lend(store, 0, 4)` and `lend(store, 4, 8)` of an eight-byte buffer, four interleaved stores through the loans, read-backs through the loans and then by the owner |
| The two permissions | one READ loan + the owner's retained write authority | **two WRITE authorities**, each party mutating its own range while both loans are live |
| Where exclusivity lives | a route-side window rule (the owner *keeps* the authority; the route restrains its use) | **the custody model**: the borrow node carries the footprint fraction, and the IR itself decides overlap, owner exclusion and containment |
| Boundary stated | "not two parties each holding write authority" (`SLICE-PERMISSIONS-WRITE-LEND` residual) | "both parties are bindings of **one sequential entry** — not two threads" (`WRITE-LEND-TASKS` residual) |
| Bounds proof | `Q077` (string-route bound) | `Q053` in **both** layers: the lend's own range, and the loan's own index (the latter beside the IR's `Q054` containment refusal) |
| Disjointness proof | `Q078` (write inside a live read loan) | `Q054` (two write lends sharing one byte refuse by the IR's exclusivity rule) |

## 2. The fixture set (`compiler/tests/fixtures/ct/CT07/`)

The write-lend fixtures are **moved**, not copied — the
CT23/`epoch_credit_error_path.rs` precedent — so one program is the evidence
in both places: `compiler/tests/write_lend.rs` repoints its two `include_str!`
constants at the CT07 directory and every one of its 31 tests still runs on
the same bytes (the fixtures' comment headers are the only edit).

| File | Role | Measured |
|---|---|---|
| `case.qcore` | the case | was `fixtures/write_lend/disjoint_write_halves.qcore`; **admitted**, `StandardBufferV1`, observation `22, 66, 11, 55`, sanitized native run of its own emitted C |
| `overlapping_lends.qcore` | disjointness control | was `fixtures/write_lend/overlapping_lends.qcore`; one bound moved 4 → 3, one shared byte: **refused**, exact cascade `[Q054]`, *"two write lends of `store` would share a byte"* |
| `out_of_extent_lend.qcore` | bounds control (the lend's range) | new, the case with one bound moved 8 → 9: **refused**, `[Q053]`, *"not a range of `store`, whose declared extent is 8 bytes"* — check time, not a run-time trap |
| `loan_index_out_of_extent.qcore` | bounds control (the loan's own index) | new, the case with one store's index moved 1 → 4 (one past the `[4, 8)` loan's 4-byte extent): **refused**, `[Q054, Q053]` — the IR's rebased containment refusal beside the route's own bound, two halves of one law, each in the layer that owns it |
| `read_loan.qcore` | accept control — the superseded configuration | was CT07's `case.qcore` at the wave-28 mapping: **admitted**, `StandardStringV1` — the weaker disjointness shape (read loan beside retained owner write) stays admitted beside the stronger one, so the remap *extends* the family of admitted disjointness shapes rather than replacing it |

**Dropped, and why they no longer earn rows here:** the wave-28
`overlapping.qcore` (`Q078`) was CT08's own case shape and CT08 still measures
that exact program as its case, so keeping a copy in CT07's directory measured
nothing the corpus does not already pin; `out_of_extent.qcore` (`Q077`) was
the *read program's* bounds control, and the case's bounds proof is now the
lend surface's own `Q053`, measured in both of its layers by the two new
controls. Nothing is lost from the corpus: both refusals remain pinned where
they are load-bearing (CT08's case; `string_slice`/`write_lend` tests).

## 3. Evidence

The verdict mode and runner are unchanged: `sequential_sanitized`, via
`compiler/tests/ct_corpus.rs::run_sequential_sanitized` — parse, elaborate,
whole-program check, the route's per-stream observation compared exactly, C11
emission, then **one binary** built from the case's own emitted C under
`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0 -fsanitize=address,undefined
-fno-omit-frame-pointer`, run once under
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` (Linux; `detect_leaks=0` on the
macOS arm — the standing platform boundary) and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, both streams compared byte
for byte with the checked observation.

ASan's load changes direction with the remap and gets **stronger**: the read
case's hazard was an out-of-bounds *read* through a displaced pointer; a write
lend emits a displaced `const` pointer and every store through it is a real
memory write at `owner + low + i`, so an off-by-one in the displacement, the
length or the rebasing would be an out-of-bounds **write** — the construct's
worst failure mode, and precisely what ASan traps. The leak control stays
positive (the program allocates).

## 4. The record mechanics (constraints 21, 23, 25)

* `docs/reports/ct07-write-remap/ct_stage_evidence.json`'s CT07 entry declares
  `"supersedes": "docs/reports/ct-wire-lane-2/ct_stage_evidence.json"` — the
  record that held CT07 — exactly the mechanism `CT01-PROOF-LEG` used against
  card T00's archive. The wave-28 record **keeps its rows and its bytes**;
  `merge_evidence` resolves CT07 to this record and CT08/CT09/CT21/CT23 stay
  resolved to that one.
* The record path is appended to **both** `EVIDENCE_FILES` lists —
  `compiler/tests/ct_corpus.rs` (the pinned array grows `[&str; 14]` →
  `[&str; 15]`, deliberately) and `scripts/ct_stage_summary.py` — and to
  `acceptance.json`'s hand-grown `ct_evidence.derived_from`, the third record
  that ties the two derivations to one file set.
* `python3 scripts/ct_stage_summary.py . --write` then `--check`: exit 0,
  `mapped 42 / unmapped 8`, no drift. The stage statuses, executors and
  observation counts are unchanged from the superseded entry — what changed is
  *which program* is the case's artifact, not how it is executed — so no
  generated field of `acceptance.json` moves.
* No `proof` row, `PROOF_GATED_CASES` or CT01 evidence is touched. No
  `unmapped_reasons` and no `retired_unmapped_reasons` row is published: CT07
  was already mapped and stays mapped.
* Two stale cross-references in **live** (non-archived) CT08 metadata — its
  `expected.json` notes and manifest profile calling the boundary control
  "CT07's own case shape" — are updated to say *at the wave-28 mapping*, since
  CT07's case is now the write-lend program. The archived wave-28 record and
  README keep saying it, correctly, of their own time.

## 5. What this card does not claim

* **No coverage claim.** A remap is not a mapping (constraint 23): the
  denominator did not grow and the numerator did not move.
* **No concurrency claim.** Both write authorities are bindings of one
  sequential entry; the scenario's two-threads reading is `WRITE-LEND-TASKS`
  (`docs/reports/slice-permissions-write-lend/README.md` §13), where the
  interval moves onto the disjoint-footprint checker's derived footprints.
* **No proof claim.** CT07's `proof` stage stays `unassessed`; the capability
  card's Lean (`QCore.WriteLend`, 25 audited declarations) proves the IR's
  exclusivity/containment laws, not this fixture.
* **No new surface.** Zero new Q codes, zero reserved words, and no crate
  source change: the only Rust edits are the `ct_corpus.rs` pinned
  `EVIDENCE_FILES` growth plus comment accuracy, and `write_lend.rs`'s
  `include_str!` repointing.

## 6. Follow-ons

* `WRITE-LEND-TASKS` — unchanged, now CT07's stated boundary rather than a
  side note.
* `WRITE-LEND-WINDOWS-RUN` — unchanged; the CT07 sanitized leg keeps the
  Windows arm's emit-only behaviour every `sequential_sanitized` case has.
* Observed in passing, not repaired here (out of this card's scope): the doc
  comment on `ct_corpus.rs::ct_mapping_progress_is_pinned_and_counted` still
  reads "proof stays 0 of 42" from before `CT01-PROOF-LEG` landed proof
  1 of 42; the ledger and the derived records are correct.

## 7. Gate (measured at this card's close, on this worktree)

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | 220 targets (doc-tests included): **4,444 passed / 1 failed / 8 ignored**. The one failure is `qcore-verifier-host/tests/enforced_process_limits.rs::a_declared_process_limit_stops_the_fixture_from_forking` with the `Io(Broken pipe)` shape — one of constraint 16's five load-sensitive rows — **re-verified in isolation immediately after: 12 passed / 0 failed** in its own binary. `ct_corpus` (5) and `write_lend` (31) both green in the full run |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | `Passed 118 Lean audits`, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | 193 tests, OK |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, `mapped 42 / unmapped 8` |
