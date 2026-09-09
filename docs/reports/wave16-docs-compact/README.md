# DOCS-COMPACT — compacting `AGENTS.md` and `CLAUDE.md` to the agent-facing minimum

**Date:** 2026-09-10. **Base:** main `8069a99` (wave-15 consolidated).
**Scope:** documentation only. `git diff --stat` touches exactly `AGENTS.md`,
`CLAUDE.md`, `DEVELOPMENT.md` and this report directory. No code, test, spec,
acceptance file or existing report changed.

## 1. What this card did and did not do

`AGENTS.md` and `CLAUDE.md` had grown into near-duplicate wave-by-wave
chronicles: 113 and 101 paragraph blocks respectively, most of them the same
text, and most of that text already present a third time in `DEVELOPMENT.md`.
The repository's own convention already said per-increment status paragraphs
belong in `DEVELOPMENT.md`'s "Increment records". This card executes that
convention retroactively.

It changed **no fact**. No count, verdict, pin, boundary or premise moved. The
headline numbers published in the new `AGENTS.md` were re-verified against
their sources rather than copied from the old prose (§5).

## 2. Before / after

| File | Before | After | Change |
|---|---|---|---|
| `AGENTS.md` | 2,423 lines / 113 blocks | **361 lines** | −85.1% |
| `CLAUDE.md` | 2,150 lines / 101 blocks | **86 lines** | −96.0% |
| `DEVELOPMENT.md` | 2,079 lines | **3,001 lines** | +922 lines (moved paragraphs only) |
| `README.md` | 126 lines | 126 lines | untouched; its four pointers still resolve |

Aggregate: 4,468 deleted lines, 1,264 inserted. The two agent-facing files
together went from 4,573 lines to 447.

## 3. Method

Paragraph-level, mechanical, then checked by hand:

1. Every blank-line-delimited block of the old `AGENTS.md` and `CLAUDE.md` was
   whitespace-normalised and matched against every block of `DEVELOPMENT.md`
   (exact, then `difflib` ratio). Output: `paragraph-disposition.txt`.
2. Blocks with no `DEVELOPMENT.md` match were cross-compared between
   `AGENTS.md` and `CLAUDE.md` to find where the two files carried *different*
   wordings of the same increment — five such pairs were found, and in every
   one each side carried facts the other did not.
3. Everything unpreserved was extracted **verbatim by line range from
   `git show HEAD:`** and appended to `DEVELOPMENT.md` under one labelled
   section, with per-paragraph origin notes.
4. A verifier re-read the original files from `HEAD` and asserted, byte for
   byte, that each moved segment is a substring of the new `DEVELOPMENT.md`:
   **28 of 28 segments OK, 0 failures.**
5. A second verifier re-ran the whole block comparison against the *new*
   tree and enumerated every block not accounted for; all 19 residuals are
   structural or rewritten-in-place and are itemised in §4.3.

## 4. Disposition ledger

### 4.1 Moved into `DEVELOPMENT.md` (verbatim, with origin note)

All under the new section *"Status paragraphs relocated from AGENTS.md and
CLAUDE.md (2026-09-10, card DOCS-COMPACT)"*.

| # | Paragraph | Origin | Note |
|---|---|---|---|
| 1 | Cycle status ledger (Cycles 1–16/17/18/19/20, Reevaluation, Active direction W0–W6) | `AGENTS.md` 60–231 | the long Cycle 19 teardown-observation run and the whole W0–W6 history |
| 2 | Cycle 19 nested-teardown boundary bullet | `CLAUDE.md` 36–79 | Claude-side observation bullet |
| 3 | R02 terminal task state, adversarial verification | `AGENTS.md` 841–860 **and** `CLAUDE.md` 615–627 | two wordings; AGENTS' is the superset, CLAUDE's kept verbatim anyway |
| 4 | Compile-task identity | `AGENTS.md` 862–887 | byte-identical in `CLAUDE.md` 629–654 |
| 5 | Windows Clang discovery and test portability | `AGENTS.md` 929–938 | byte-identical in `CLAUDE.md` 696–705 |
| 6 | Diagnostic attribution and ownership | `AGENTS.md` 964–986 **and** `CLAUDE.md` 731–752 | genuinely different: AGENTS has the V04/V07 occurrence detail and the spec reconciliation, CLAUDE has the file paths, the empty-`contract { }` asymmetry and the `DerivationIncomplete` rule |
| 7 | Configured second source set | `AGENTS.md` 988–1007 | byte-identical in `CLAUDE.md` 754–773 |
| 8 | Replay mutation matrix | `AGENTS.md` 1008–1040 | byte-identical in `CLAUDE.md` 774–806 |
| 9 | Source-filename-sensitive transport | `AGENTS.md` 1041–1067 **and** `CLAUDE.md` 807–824 | different: CLAUDE's carries the `CSourceInput` do-not rules and the `HistoricalStdin` re-measurement rule |
| 10 | Checked mutex invariants (T02) | `AGENTS.md` 1068–1101 **and** `CLAUDE.md` 825–851 | different framings, both kept |
| 11 | Typed scalar-field access classification | `AGENTS.md` 1103–1144 | byte-identical in `CLAUDE.md` 853–894 |
| 12 | Remaining-work wave accounting | `AGENTS.md` 1146–1179 **and** `CLAUDE.md` 896–906 | AGENTS' carries the milestone-criteria list and the named follow-ups |
| 13 | Delegated increment (five cards) + its `Nothing is promoted` | `AGENTS.md` 1181–1246 | byte-identical in `CLAUDE.md` 908–973 |
| 14 | Second delegated increment (seven cards) + its `Nothing is promoted` | `AGENTS.md` 1248–1349 | byte-identical in `CLAUDE.md` 975–1076 |
| 15 | Six short pointer bullets (W2 profiles, W3 evidence-v2, W2 H6, Lean CI hygiene, W7 Lean naming, `qcore-audit` driver) | `CLAUDE.md` 122–124, 134–140, 142–144, 146–149, 151–153, 163–165 | each moved separately with its own origin line |
| 16 | Claude-specific working bullets incl. the W4 custody-pass execution record | `CLAUDE.md` 81–109 | imperative halves also lifted into the new constraints section |

**Byte-verbatim check: 28/28 segments confirmed present in the new
`DEVELOPMENT.md`.**

### 4.2 Cut as already-preserved duplicates

Verified present in `DEVELOPMENT.md` before this card, so removed without a
move (paragraph-level exact or near-exact match; see
`paragraph-disposition.txt` for the per-block evidence):

- **AGENTS.md:** 37 chronicle paragraphs matched `DEVELOPMENT.md` exactly
  (Lean build performance; Reopen failure evidence; W2 profile migration;
  Frontend gates; W2 H6 discovery; Boolean field routing; W5 configured audit;
  Monkey typed profiles; Revision 0.3 review; W5 kernel/contract boundaries;
  W5 packaging; CLI package lifecycle; Trusted verification adapter; W4 output
  forwarding; generic reactor; generic callback records; source-derived field
  provider; ChunkIO statement execution; contextual recursion; source task
  foundations; schema compatibility; native source tasks; nullable removal;
  configured source-contract dispatch; configured frame helpers; W7 diagnostic
  attribution; owned source tasks; configured process audit; unsigned
  countdown; generic source consolidation; handler rethrow; worker-local
  allocation; typed body comparison; completed-child custody; immutable output
  aliases; reused-address; generic driver consolidation; public source-profile
  validation; CMetrics unit provider; ordered preprocessing transport;
  persistent Clang CMake audit; moving Fluent Bit revisions; Cycle19 command
  correction; Cycle19 explicit Clang; enforced process limits; third delegated
  increment + integration notes + closing; Windows native concurrency; Windows
  second-source-set fixture; Windows Clang fixture repair; concurrency prelude;
  final observation increment; and waves 5 through 15 in full, each with its
  `Nothing is promoted` closing paragraph).
- **CLAUDE.md:** the same set, matched independently.
- **macOS native concurrency** (`AGENTS.md` 548–559 / `CLAUDE.md` 322–333) —
  `DEVELOPMENT.md` line 703 carries a reworded version with the same facts;
  compared sentence by sentence, nothing is lost.
- **macOS Cycle19 compiler-option transport** (`AGENTS.md` 1563–1571 /
  `CLAUDE.md` 1290–1298) — `DEVELOPMENT.md` line 1216 carries a **fuller**
  version (it additionally records the compile-task identity binding).

### 4.3 Kept, rewritten in place, or structural

Nineteen residual blocks that the mechanical check flags but that are not
lost content:

| Block | Disposition |
|---|---|
| `AGENTS.md` 8–13 (sync sentence) | **Superseded deliberately** by the new, explicit sync rule (requirement 5) |
| `AGENTS.md` 29–56 (repository map) | **Rewritten current** — two stale facts corrected (see §5) |
| `AGENTS.md` 259–272 (Lean CI hygiene) | **Kept as rules, not history**; the "hosted CI awaits a push" status line survives in `DEVELOPMENT.md` via the moved CLAUDE bullet |
| `AGENTS.md` 304–325 (key entry points + diagnostics) | **Kept and expanded** — the diagnostics table is now the full §L21 summary |
| `AGENTS.md` 329–336 (where to read more) | **Kept**; all ten pointers preserved, three added |
| `AGENTS.md` 338 / `CLAUDE.md` 111 (`## Reopen failure evidence` heading) | Heading only; body moved/preserved |
| `AGENTS.md` 988–1101 / `CLAUDE.md` 754–851 | These are **four paragraphs run together without blank lines**; each was moved separately, so the joined block does not match. All four verified byte-verbatim |
| `CLAUDE.md` 19–21, 23–29, 31 | Content preserved in the new `CLAUDE.md` "Pointers" and the new sync rule; `docs/CYCLE19_BUILD_INPUTS.md` explicitly retained |
| `CLAUDE.md` 33–79 | Line 33–35 (threading examples) kept in the new "Platform records"; 36–79 moved |
| `CLAUDE.md` 81–109 | Moved verbatim; imperatives also lifted to constraints |

## 5. Facts re-verified rather than carried forward

The old ledger prose had drifted. Every headline number in the new `AGENTS.md`
was read from its source:

| Claim | Source read | Result |
|---|---|---|
| Contract corpus **67** modules | `compiler/tests/qcore_contract_corpus.rs:464` (`assert_eq!(sources.len(), 67)`) | old map said "60 files" — **corrected** |
| `qcore-schemas` **12.0.0** | `crates/qcore-schemas/Cargo.toml` | old map said "Rust API 3.0.0" — **corrected** |
| `qcore-compiler` 0.4.0, host/verification 0.4.0 | the three `Cargo.toml` files | confirmed |
| 185 units 2 / 42 / 137 / 4 | `docs/reports/wave15-corpus/README.md` headline | confirmed |
| CT **32/50 mapped**, 18 unmapped, sanitized-native 4/32, proof 0/32 | `acceptance_matrix.json` `ct_conformance.totals` | old Reevaluation bullet still said "39/50 unmapped" — **corrected** |
| `named_followups` = 25 entries | `acceptance_matrix.json` | confirmed |
| Diagnostic bands | `docs/QCORE_SPEC_r0_3.md` §L21 read directly | the brief's list was correct; `Q000`, `Q001`/`Q016`, `Q010`–`Q015`, `Q1xx`–`Q5xx` and `Q700`–`Q703` added |

These are corrections of stale documentation, not verdict movement. Nothing is
promoted.

## 6. Carried-constraints inventory

Requirement 4 was the card's central obligation. Both files were swept
sentence-by-sentence with a regular expression over imperative and
standing-constraint language (`do not`, `don't`, `must not`, `must stay`,
`when touching`, `never`, `deliberately not`, `not weakened`, `load-bearing`,
`reserved word`, `hazard`, `tripwire`, `pending hygiene`, `flaky`, `stash`,
`scratchpad`, `prefer … over`, `stay separate`, `requires updating`), yielding
87 candidate sentences (45 from `AGENTS.md`, 42 from `CLAUDE.md`), plus a
targeted second pass for `§L21 process`, `opt-in`, `unavailable by default`,
`denominator is mapping` and premise language.

Deduplicated, these became **25 numbered constraints** in the new
`AGENTS.md` § *Standing constraints when touching specific subsystems*, in
seven groups. Each carries the card that established it.

| # | Constraint (abbreviated) | Carried from |
|---|---|---|
| 1 | §L21 is the canonical per-code ownership table; a pass owns a range; `source_diagnostic` is the authoritative map; a code may not be reused; a code change is semantic, attribution is not; new codes go through the §L21 process | AGENTS 315–325, CLAUDE 741, spec §L21 |
| 2 | `Q020`/`Q021` must stay separate until a source-contract consumer lands — the task gate refuses an empty `contract { }` and the sequential gate accepts it; `source.extern-call.v1` is not that consumer | AGENTS 322–325 & 975, CLAUDE 743–747, spec §L21 |
| 3 | Do not add a `category` field to `DerivationIncomplete` without regenerating every task digest, provider package and witness; `environment-unavailable` must never be relabeled a source defect | CLAUDE 749–752, spec §L21.1 |
| 4 | W4: a summary must be validated by re-analysis before publication; do not seed entry custody the body did not demand; do not accept an assume-guarantee fixed point for a recursive component; declared contracts always win (`SummaryConflict`, never a silent override) | CLAUDE 85–99, AGENTS 218–228 |
| 5 | The typed scalar-field whitelist's "base never binds a resource identity" premise was added because dropping it made a read through a released pointer `Safe`; the pin was not weakened | AGENTS 1112–1114 / CLAUDE 862–864 |
| 6 | C02-P-ALIAS-LEAK-ORDER: `report_leaks` scheduling sensitivity is recorded, not repaired — do not disturb it | AGENTS 2107–2110, 2238, 2312 |
| 7 | Leak diagnostics are pinned at 271; movement is a finding to attribute | AGENTS 1916 context, corpus-repin |
| 8 | Do not normalize the spelled input path; do not let a declared identity survive onto derived text; do not move the Cycle19 route off `HistoricalStdin` without re-measuring; keep `source_input` in `CompileTaskIdentity` | CLAUDE 813–819 |
| 9 | A `#line` directive must not move the audited scope; `Safe` requires full analysed coverage at the certificate gate | AGENTS 2371–2378 (E01-TAIL) |
| 10 | Concurrency is opt-in per file (`import std.concurrency;`); imports do not leak across compilation inputs (`Q014`/`Q015`); the task profile keeps its separate `concurrency checked;` marker | AGENTS 1497–1503 |
| 11 | New reserved words are a compatibility hazard checked against every fixture (T03-F five; T08-EPOCH-SRC eight, `work`→`effort` rather than editing the pinned golden surface; `succeeds`) | AGENTS 1621, 1651, 1797 |
| 12 | Lock ranks and protection growth are opt-in; unranked modules stay byte-identical; a rank is a declared position, not an assumption | AGENTS 2018–2021, 2174–2177 |
| 13 | P02-FLAKE: the `RLIMIT_NPROC` assertion is byte-for-byte unchanged with a deterministic companion — do not weaken it | AGENTS 1243–1246, 1813 |
| 14 | EXTERN-F1-STORE-REACH: the abandonment gate refuses a second live acquisition and was deliberately not weakened | AGENTS 2035–2037 |
| 15 | Provider executables embed their manifest template; all five packages must be rebuilt and verified after a template or schemas-major change; `providers/package.py --locked` is in CI | AGENTS 1626–1636, 1695, 1813 |
| 16 | E03-B deliberately supersedes E03's `QP012` unequal-premise refusal with the premise union (values still refuse) | AGENTS 1601–1611 |
| 17 | P04's in-process counter route was deliberately not removed (live callers); P09-A..F retained with named gaps; removal needs invariant-3 parity | AGENTS 1413, 1831–1843 |
| 18 | The frozen T00 CT archive was mutated in place by a later lane (pending hygiene) — publish provenance beside it, do not edit it | AGENTS 2353–2362, 2067–2072 |
| 19 | DIFF-TOOL-CONTRACTS: verify contract-inventory growth by direct comparison, not the diff summary | AGENTS 2331–2338 |
| 20 | CT coverage grows by mapping, not conformance; a stage reads `executed` only where a named workspace test runs it on the case's own artifact | AGENTS 1657–1662, 1846–1850, T00-CT-SCHEMA |
| 21 | REASON-CODES-B3's tripwire test is flipped at the corpus re-pin per its own design, not silently | AGENTS 1968 |
| 22 | The acceptance-matrix generator's `section_ownership` block: whole-file regeneration must not clobber later reconciliations | AGENTS 1955–1963 |
| 23 | Process providers have containment + opt-in `linux-rlimit-v1`/`linux-confined-v1`, **not an OS sandbox**; default stays `unenforced-legacy` | AGENTS 170–171, 940–962, 1236–1243, 1806–1815 |
| 24 | The mutex model supports initially-unlocked non-recursive mutexes only; the excluded forms are represented refusals, not omissions | AGENTS 171–172, 1083–1085 |
| 25 | Declared profiles, `observations_trusted`, `GuardFootprintAuthority`, `SynchronizedCustodyEdges`, `DeclaredFootprintShape`, `InvariantPredicateDeclared` and C layout/linker equalities are **premises**; witness admission conveys no provider authority | AGENTS 774–776, 1028–1032, 1098–1101 |

Six further imperatives are working-practice rather than subsystem rules and
stayed in `CLAUDE.md` (§7): never weaken a fail-closed gate / update pins
deliberately; keep the Safe/Unsafe/Unknown and "unpromoted" vocabulary; run the
standing gate and report exit codes honestly; land code in the target crate,
not the grab-bag; prefer WIP commits over `git stash`; keep measurement outputs
in worktree-private directories. The card-completion standard (Rust plus
audited Lean) was added there from the project memory convention.

### One deliberate reconciliation

The brief's item 3 lists the W4 do-not rules as `CLAUDE.md` content, while
item 4 calls for consolidating *all* surviving constraints into `AGENTS.md`.
Duplicating them would recreate exactly what this card removes. Resolution:
**`AGENTS.md` carries the complete subsystem-constraint inventory** (it is the
source of truth for agents, and item 4 is named the central obligation);
`CLAUDE.md` carries the working-practice rules and points at that section by
name. The W4 execution record itself is preserved verbatim in `DEVELOPMENT.md`
(move #16).

## 7. New file shapes

**`AGENTS.md` (361 lines):** status banner · explicit sync rule · what this
project is · repository map (current) · short status ledger (headline table +
one paragraph of cycle state + where the detail lives) · the six hard
invariants **verbatim** · standing constraints (25, seven groups) · standing
per-change gate · Lean CI hygiene as rules · key entry points · diagnostics
namespace table derived from §L21 · where to read more.

**`CLAUDE.md` (86 lines):** status banner · `@AGENTS.md` · a pointer telling
the reader the constraints live in `AGENTS.md` · working rules (6) · worktree
and measurement hygiene (3) · platform records (Windows / macOS / Linux +
threading examples) · pointers (`CYCLE19_BUILD_INPUTS.md`, normative spec,
Emacs mode).

**`DEVELOPMENT.md` (3,001 lines):** unchanged except for the appended
relocation section. No existing line was edited.

## 8. Gate

The orchestrator directed this documentation-only card to the cheap
verification path rather than the full standing gate: confirm the diff scope,
and grep the suite for any test that pins `AGENTS.md`/`CLAUDE.md` content,
running only that target if one exists.

| Check | Result |
|---|---|
| `git diff --stat` | **`AGENTS.md`, `CLAUDE.md`, `DEVELOPMENT.md` only** (+ this new untracked report directory). No code, test, spec, acceptance file or existing report touched |
| Tests pinning `AGENTS.md`/`CLAUDE.md` content | **none** — see §8.1. No target to run, nothing to reconcile |
| `cargo fmt --all -- --check` | clean (run before the directive) |
| `python3 scripts/check_lean_audits.py` | **67/67 audits passed** (run before the directive, with `~/.elan/bin` on `PATH`); no proof file changed by this card |
| `cargo test --workspace` | **not required for this card** and not re-run after the directive; observations from the runs made before it are recorded in §8.2 for honesty |

### 8.1 Test reconciliation — none needed

**No test pins `AGENTS.md` or `CLAUDE.md` content.** This was checked, not
assumed. No Rust source `include_str!`s or otherwise reads any `.md` file. The
only occurrences of either filename in `crates/` or `compiler/` are three
**doc comments in `src/`, not tests**:

- `crates/qcore-c-gateway/src/borrow_liveness.rs:95` — cites "AGENTS.md hard
  invariant" for the unproven-call-effect rule;
- `crates/qcore-c-gateway/src/legacy_enumeration.rs:322` — cites "AGENTS.md
  invariants 1 and 2";
- `crates/qcore-schemas/src/reason.rs:142` — cites "AGENTS.md invariant 2".

All three cite invariants *by number*. The hard-invariants section is preserved
**verbatim and in the same order**, so invariants 1 and 2 still say what those
comments claim. One further occurrence is a string literal in
`docs/reports/remaining-d02/update_acceptance.py`, which this card did not
touch.

All 39 file-path references in the new `AGENTS.md` and all 14 in the new
`CLAUDE.md` were additionally resolved against the working tree; every one
exists.

### 8.2 Observations from the pre-directive test runs

Recorded honestly, though the gate is no longer required for this card. Two
full `cargo test --workspace` runs were started before the directive arrived.
Neither failure is attributable to this documentation change, and both are
environmental:

1. **`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`** failed once on
   `assertion failed: elapsed < Duration::from_millis(500)`
   (`compiler/tests/cycle20_verifier_plugins.rs:794`) while a Lean build was
   competing for CPU on the same host. It **passes in isolation**
   (`1 passed; 0 failed`). This is a wall-clock assertion under load, the same
   class as the recorded P02-FLAKE hazard. The assertion was **not** touched
   or weakened.
2. **`libevent_submodule_matches_the_audited_revision`** failed on the second
   run because this fresh worktree had no submodule checked out — the standing
   gate's own first line. After
   `git submodule update --init --recursive` the submodule checks out at the
   pinned `d82464a277d0f42703702c4dfd9af6af38595a83` and the pin test's
   precondition is satisfied.

The third run was stopped mid-flight on the directive and is not reported as a
result.

## 9. Boundaries

- This card moved and reorganised prose. It establishes nothing, promotes
  nothing, and re-measured nothing: the 185-unit corpus, the CT manifest, the
  contract corpus and every pin are untouched.
- The preservation claim is **paragraph-level and byte-verbatim for the moved
  set** (28/28 checked mechanically). For the 37+ paragraphs cut as duplicates,
  the claim is that a whitespace-normalised exact match already existed in
  `DEVELOPMENT.md`; the two macOS paragraphs are near-matches read and compared
  by hand (§4.2), and `DEVELOPMENT.md`'s version is at least as complete in
  both cases.
- The constraint sweep is regex-driven plus a manual second pass. It is
  thorough, not provably exhaustive: a standing rule phrased without any of the
  swept markers could in principle have been missed. The complete original text
  remains in git history and, for every unpreserved paragraph, verbatim in
  `DEVELOPMENT.md`, so nothing is unrecoverable.
- `docs/reports/wave16-docs-compact/paragraph-disposition.txt` is the raw
  machine output backing §4; it lists all 214 original blocks with their match
  status.
