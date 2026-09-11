# DOCS-COMPACT-2 — second compaction pass over `AGENTS.md`, `CLAUDE.md` and `README.md`

**Date:** 2026-09-11. **Base:** main `b2cf7563` (wave-23 close).
**Scope:** documentation only. The diff touches `AGENTS.md`, `CLAUDE.md`,
`README.md`, one stale cross-reference in `docs/GENERALIZATION_PLAN.md` (§6.1),
and this report directory. No code, test, spec, contract, acceptance file,
frozen archive or existing report changed.

This card is the second in the recurring series the user directed: *"Every 2–3
cycles, we needed to do."* The precedent is card DOCS-COMPACT (2026-09-10,
`docs/reports/wave16-docs-compact/README.md`), which established the sync rule
this card enforces. The cadence is now recorded as a standing working rule in
`CLAUDE.md` (§5 below).

## 1. What this card did and did not do

It changed **no fact about the system**. No verdict, count, pin, boundary or
premise moved. Three pieces of prose that had *drifted away from* their sources
were re-read from those sources and corrected (§4).

The three files had each drifted a different way since wave 16:

- `AGENTS.md` had grown from 361 lines to 584 as waves 17–23 appended
  constraints; several of them restated their card report at paragraph length,
  the repository map had accreted one row per gateway sub-module, and the
  diagnostics table carried four "second/third/fourth band of the same pass"
  rows for one owning pass.
- `CLAUDE.md` re-carried the `Current work status` blockquote, the `QCORE_CLANG`
  discovery rule, the native-threading-examples pointer, the Emacs pointer, the
  audit-setup pointer and the spec pointer — all of which also live in
  `AGENTS.md`, which `CLAUDE.md` imports with `@AGENTS.md`.
- `README.md` had fallen behind the source surface: it described the Quirin
  rename and `std.concurrency` but not the `std.buffer`, `std.string`,
  `std.event`, `std.extern`, pipe, branch or native-library profiles that waves
  17–23 landed.

## 2. Before / after

| File | Before | After | Change |
|---|---|---|---|
| `AGENTS.md` | 584 lines / 42,583 bytes | **567 lines / 41,908 bytes** | −17 lines, −675 bytes (−2.9% / −1.6%) |
| `CLAUDE.md` | 97 lines / 5,622 bytes | **93 lines / 5,448 bytes** | −4 lines, −174 bytes; ~40% of its body rewritten |
| `README.md` | 135 lines / 7,187 bytes | **156 lines / 8,188 bytes** | **+21 lines** — a refresh, not a compaction |
| `docs/GENERALIZATION_PLAN.md` | 6 lines in §6.1 | 9 lines | one stale rule corrected |
| `DEVELOPMENT.md` | 3,927 lines | 3,927 lines | untouched; nothing needed relocating |

`README.md` grew on purpose. The card's README directive was *refresh to the
current state*, and six admitted source profiles plus three composed routes had
no mention in the human summary at all. The one-table treatment (§3.3) is the
shortest honest form found for them.

`AGENTS.md`'s 2.9% is the honest number. The first compaction removed 85% of
that file because it was a duplicated wave-by-wave chronicle; what remains is a
dense reference whose only real fat was paragraph-length card restatement in
seven constraints and structural repetition in two tables. Nothing was cut to
make the percentage larger.

**`DEVELOPMENT.md` was not touched.** Unlike DOCS-COMPACT, this card moved no
paragraph into the increment records, because every fact it tightened is still
present in `AGENTS.md` in shorter words — none was removed, so none needed a
new home. `DEVELOPMENT.md`'s frozen DOCS-COMPACT section (lines 3008+), which
quotes the old `AGENTS.md` and `CLAUDE.md` by line range, describes the past
and was deliberately left alone.

## 3. What changed, section by section

### 3.1 `AGENTS.md`

| Section | Action |
|---|---|
| Header | `DEVELOPMENT.md`'s contents description folded in from the repository map (see below) |
| Sync rule | one clause added pointing at `CLAUDE.md`'s new compaction-cadence rule, so an agent reading only `AGENTS.md` still learns the cadence exists |
| Repository map | three `crates/qcore-c-gateway/src/*.rs` rows merged into the gateway crate's own row (full paths retained verbatim inside it); three `compiler/src/` facade rows merged into one; the `DEVELOPMENT.md` row removed, its text moved into the header sentence two paragraphs above. **25 data rows → 19** |
| Status ledger provenance | wave-23 baseline paragraph tightened from 6 lines to 5; every claim kept, including "the wave's arbiter card deliberately did **not** re-pin it" and constraint 21's frozen-archive reference |
| "Where the detail lives" | absorbed `docs/cycles/CYCLE_20.md` and the two reevaluation documents from the tail section |
| Standing constraints | 7 of 33 entries reworded (§3.2); 26 byte-identical after whitespace normalisation |
| Lean CI hygiene | same length (14 lines), reordered so the runner's properties sit with the runner; no rule dropped |
| Diagnostics namespaces | four `NativeLibrary` rows merged into one four-band row, two `Event` rows into one two-band row. Every code range is still spelled in the `Codes` column, so a search for `Q065` or `Q074` still lands. **32 data rows → 28** |
| "Where to read more" | reduced to the four pointers the ledger paragraph does not already carry |

### 3.2 The standing-constraint inventory (the load-bearing check)

**33 entries before, 33 after. Zero dropped, zero added, zero renumbered.**
Verified mechanically by `measure/inventory.py` (output in
`measure/constraint_inventory.txt`), which re-extracts the numbered entries from
`git show HEAD:AGENTS.md` and from the working tree and diffs the label sets.

| # | Subject | Disposition |
|---|---|---|
| 1, 2, 3 | §L21 code ownership; `Q020`/`Q021` separation; `DerivationIncomplete` | unchanged, byte-identical |
| 4 | W4 summary publication do-nots | unchanged, byte-identical |
| 5, 5a | typed scalar-field last premise; fixed `sizeof`/`_Alignof` operand class | unchanged, byte-identical |
| 6 | `C02-P-ALIAS-LEAK-ORDER` | unchanged, byte-identical |
| **7** | leak pin **266** | **reworded**, 2,639 → 2,316 chars. Same rule, same four moves with the same line numbers, same directions, same three residual names (`CONSTRUCTOR-RESIDUALS-LEAK-271`, `AOM-DESTROY-JOINED-PROVENANCE`, `AOM-SIBLINGS-BINDING-POISON`), same "five, not six, standing claims" and same L19544 exception. The narrative is restructured into one move-per-sentence list; the per-move detail also stands in `docs/reports/aom-destroy-siblings/README.md` §6/§6.1 and in `DEVELOPMENT.md`'s wave-23 paragraph |
| **8** | declared contract must agree with the unit's C | **reworded**, 2,143 → 2,057 chars. The type arm and the mode arm shared a "withdrawn for that unit" definition stated twice; they now share one. Every do-not survives: do not move either reader, never install the derived summary in the declaration's place, no withdrawal on an intra-duty refinement, absence of evidence is not evidence, `LOADER-ABI-LP64` |
| 9 | fixture prelude ≠ production declaration | unchanged, byte-identical |
| **10** | store into an outliving object | **reworded**, 1,406 → 1,395 chars. "reads no per-site status" and "reads no provenance" merged into one clause; both withdrawals and the invariant-1 promotion measurement kept |
| 11, 12 | compile-task identity; `#line` | unchanged, byte-identical |
| 13 | concurrency opt-in | unchanged, byte-identical |
| **14** | new keywords are a compatibility hazard | **reworded**, 1,591 → 1,495 chars. Only the parenthetical explaining *why* `extern` and `library` were already words was dropped; every count, every keyword, both route selectors and the "do not remove that exclusion" rule stand |
| 15, 15a, 15b | lock ranks; composed-task line placement; post-desugaring token reads | unchanged, byte-identical |
| **15c** | event-program stream comparison | **reworded**, 1,219 → 1,175 chars. The `strict_iff_byte_equality` degenerate-case explanation is now a cross-reference to 15a, which states it in full. Both premises kept |
| 15d | Windows `std.event` AF_UNIX capability probe | unchanged, byte-identical |
| **16** | `P02-FLAKE` | **reworded**, 838 → 834 chars, and a **drift corrected**: the entry said "four … rows" and then listed five. It now says five (§4) |
| 17–24 | `EXTERN-F1-STORE-REACH`; package rebuild; `E03-B`; P04/P09; frozen archives; `contracts.json`; CT mapping; `REASON-CODES-B3` | unchanged, byte-identical |
| **25** | acceptance-matrix `section_ownership` | **drift corrected only**, same length: `D02-B00-LEDGER-SNAPSHOT`'s "beside a live 69" now reads "beside a live 73", the measured count (§4) |
| 26, 27, 28 | provider confinement; mutex model; declared premises | unchanged, byte-identical |

**Six hard invariants:** extracted before and after, identical label set
(`Fail-closed`, `Verdict contract`, `Parity before deletion`, `Gate ordering`,
`Pinned expectations are intentional`, `Lean parity`) and byte-identical bodies.

**Evidence pointers:** every backticked token in `AGENTS.md` that names a
directory under `docs/reports/` was collected before and after — **67 before, 67
after, zero lost**, and zero card-shaped tokens in the new file fail to resolve
to a real directory.

**Diagnostics codes:** 63 distinct `Q…`/`QP…` codes before, 63 after, none lost,
none gained; every owning-pass name preserved.

**Status vocabulary:** the Safe/Unsafe/Unknown verdict words, the "unpromoted"
phrasing and both forms of the "a provisional `Unsafe` is never presented as an
upstream defect" sentence are present and unedited in `AGENTS.md` invariant 2,
`CLAUDE.md`'s status-language rule and `README.md`'s status section.

### 3.3 `CLAUDE.md`

Rewritten in place around the sync rule: `AGENTS.md` is the source of truth and
`CLAUDE.md` carries only what is in neither `AGENTS.md` nor `DEVELOPMENT.md`.
The `@AGENTS.md` import survives on line 3, as required.

Removed as duplicates — each verified still present in `AGENTS.md`, which
`CLAUDE.md` imports, so nothing is lost to a reader of either file:

| Removed from `CLAUDE.md` | Still stated in |
|---|---|
| the `Current work status` blockquote | `AGENTS.md` lines 3–6 (imported) |
| the enumeration of what `AGENTS.md` contains | shortened, not removed; now one sentence |
| `QCORE_CLANG` overrides discovery / invalid value is an error | `AGENTS.md` "Standing per-change gate" |
| the native threading examples / `build-windows.ps1 -Run` bullet | `AGENTS.md` "Key entry points" |
| `docs/CYCLE19_BUILD_INPUTS.md` pointer | `AGENTS.md` "Where to read more" |
| normative-spec pointer | `AGENTS.md` "What this project is" |
| Emacs `qcore-mode` pointer, including the "not a change to conformance" caveat | `AGENTS.md` "Key entry points" ("Highlighting does not imply executable or verified support") |

Kept, because they are operational guidance that lives nowhere else: the six
working rules (fail-closed gates and pins, status language, running the gate
honestly, landing code in the target crate, closing a card with Rust plus
audited Lean, one home for new status text), all three worktree/measurement
hygiene bullets with their incident evidence, and all four platform records
including the user-validated 2026-09-11 cross-platform and ARM64 Windows
observations with their candidate residual names.

Added: the cadence rule (§5). The `Platform records` section now opens with one
sentence naming it as the home for per-platform observations, with `AGENTS.md`
keeping the toolchain requirements that are part of the gate — the "one home
each" split the card asked for.

The `Pointers` section collapsed to a single paragraph carrying its one
non-duplicate fact: *any supplied archive review is older than the merged W2–W5
implementations.*

### 3.4 `README.md`

Refreshed as a human summary, same register, no marketing.

- New section **"What source can say today"**: a six-row table of the `std`
  module family (`std.concurrency`, `std.fmt`/`std.io`/`std.logger`,
  `std.extern` plus `std.extern.unsafe`, `std.buffer`, `std.string`,
  `std.event`), then one paragraph naming the pipe operator, `if`/`else`
  branching, `extern library NAME;` binding and the three composed routes. Every
  import spelling was read out of
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`, not from memory.
- The honesty framing moved into that section rather than being dropped:
  "Everything outside an admitted profile is refused, not guessed"; "Broader
  structural syntax and separately checked semantic models do **not** imply
  complete source-language support, and general task/mutex composition remains
  open."
- The concurrency-prelude paragraph and the thread-examples link were folded
  into the same section instead of floating after the type-system list.
- Status table, the raw Cycle 19 measurement (2 Safe / 42 provisional Unsafe /
  137 Unknown plus four data-only), the unpromoted paragraph and the whole
  documentation map are unchanged; the `AGENTS.md` row gained the words
  "standing subsystem constraints", which it had been missing since wave 16.
- One genuine duplicate removed: "The supported executable source subset lowers
  to C11 with a tagged-error ABI and explicit cleanup" repeated, nine lines
  later, the "Two things live here" bullet that already says "A supported
  executable subset compiles through `quirin` to C11 with a tagged-error ABI and
  explicit cleanup."
- Naming blockquote and Quick start tightened by a few lines each; every link
  in both still resolves.

Checked: every `.md` path and link target in all three files resolves to a real
file (`measure/inventory.py`, final section — `CLAUDE.md` and `README.md` report
zero unresolved; `AGENTS.md`'s six are bare basenames used in prose, such as
`coverage.json` and `AuditAll.lean`, which were bare before this card too).

## 4. Drift corrected (three items), and why each is a correction

Following the DOCS-COMPACT precedent of re-reading headline numbers from their
source rather than carrying the prose:

1. **Constraint 16 said "four … rows" and listed five.** The fifth was appended
   at wave 20 with the words "and — fifth, wave 20 —" without the count being
   updated. Now "five", with the wave-20 row dated in place.
2. **Constraint 25 said the frozen B00 ledger's 59-module checkpoint sits
   "beside a live 69".** Measured: `find contract/qcore -name '*.qcore'` returns
   **73**, which is also what the status ledger and the repository map say. Now
   "beside a live 73". The frozen archive itself was **not** touched — that is
   the repair constraint 21 forbids, and the `D02-B00-LEDGER-SNAPSHOT` residual
   stands unchanged.
3. **`docs/GENERALIZATION_PLAN.md` §6.1 still carried the pre-DOCS-COMPACT
   documentation rule**: "A workstream that changes project status … updates
   `AGENTS.md` and `README.md` in the same change." That contradicts the sync
   rule in force since 2026-09-10, under which a status change adds one
   paragraph to `DEVELOPMENT.md` and updates `AGENTS.md`'s headline numbers and
   **leaves `README.md` alone**. Updated deliberately, marked as superseded, and
   it is the only file outside the three targets and this report that the card
   touches.

None of the three changes a verdict, a count in the ledger, a pin or a premise.

## 5. The cadence rule (the user directive this card records)

Added to `CLAUDE.md`'s working rules:

> **Compact these three files every 2–3 waves.** `AGENTS.md`, `CLAUDE.md` and
> `README.md` accrete: constraints restate their card reports, sections drift
> apart, README falls behind the ledger. A recurring compaction card
> (precedent: DOCS-COMPACT 2026-09-10, DOCS-COMPACT-2 2026-09-11) re-reads every
> headline number from its source, inventories the standing constraints before
> and after, and merges redundancy. Compaction is **not** deletion: no
> constraint, hard invariant or evidence pointer may be dropped, and a fact that
> lives only in these files moves to `DEVELOPMENT.md` verbatim rather than
> disappearing.

`AGENTS.md`'s sync-rule paragraph gained one clause pointing at it, so the
cadence is discoverable from the agent source of truth without being stated
twice.

## 6. Census (what was checked before moving anything)

The card's precondition was to find anything that reads or pins the content of
these three files. Searched: every `*.rs`, `*.py`, `*.sh`, `*.yml`, `*.yaml`,
`*.toml`, `*.ps1` in the tree outside `target/`, plus `scripts/`,
`scripts/tests/` and `.github/workflows/`.

| Finding | Disposition |
|---|---|
| `compiler/tests/thread_examples.rs:77` does `include_str!("../../examples/threads/README.md")` and extracts its `Reading the output` section | **Not** the top-level `README.md`. Untouched by this card. |
| `scripts/tests/test_ct_stage_summary.py`, `scripts/tests/test_sidecar_reason_inventory.py` mention `README.md` | Both are string literals naming `docs/reports/**/README.md` paths inside fixture data, not reads of these files. |
| Six Rust source files cite `AGENTS.md` invariants/constraints by **number** in doc comments (`borrow_liveness.rs`, `address_of_member.rs`, `legacy_enumeration.rs`, `reason.rs`, `qcore_contract_corpus.rs`) | No constraint was renumbered, so every citation still resolves. This is why renumbering was ruled out even where merging entries would have shortened the file. |
| `DEVELOPMENT.md` quotes old `AGENTS.md`/`CLAUDE.md` line ranges throughout its DOCS-COMPACT section | Frozen quotes describing the past. **Not edited.** |
| `docs/GENERALIZATION_PLAN.md` §6.1 documentation-split rule | Stale; corrected (§4.3). |
| `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` cites "the standing instruction in `CLAUDE.md` — *do not accept an assume-guarantee fixed point*" | That instruction now lives in `AGENTS.md` constraint 4 and was **not** moved by this card (it moved at DOCS-COMPACT). Left alone: it is a historical design note, and re-pointing it is outside this card's scope. Recorded as follow-on `DOCS-COMPACT-2-W4-DESIGN-POINTER`. |

**Nothing the census found blocked a planned compaction.** One thing the census
*did* prevent: merging the numbered constraint list into fewer, topic-grouped
entries, which would have renumbered citations in five Rust files.

## 7. Gate

Run at the card tip on Linux, with `$HOME/.elan/bin` on `PATH`.

| Step | Result |
|---|---|
| `cargo test --workspace` | **3,776 passed, 0 failed, 8 ignored** |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **`Passed 99 Lean audits`**, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |

Unchanged from the wave-23 close, as a documentation-only card must be. No
provider rebuild was required: `providers/package.py` inputs are untouched
(constraint 18).

## 8. Boundaries and follow-ons

- **`AGENTS.md` is near its floor.** After two compaction passes the file is a
  reference, not a chronicle; a third pass should expect single-digit
  percentages unless waves 24+ add long constraints again. The cadence rule
  exists to keep that accretion bounded, not to promise a shrinking file.
- **The constraint sweep is a label check, not a semantic one.** The verifier
  proves 33 labels in and 33 labels out with 26 bodies byte-identical; that the
  seven reworded bodies preserve their content is argued in §3.2 and rests on
  reading, as at DOCS-COMPACT. The pre-card text remains in git history at
  `b2cf7563`.
- `DOCS-COMPACT-2-W4-DESIGN-POINTER` (§6): `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`
  attributes constraint 4's assume-guarantee rule to `CLAUDE.md`, where it has
  not lived since 2026-09-10.
- `README.md` grew. If a future pass wants it shorter, the source-surface table
  is the candidate to move behind the implemented-grammars link — but it is the
  only place a human learns what the language can express today.
