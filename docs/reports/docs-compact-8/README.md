# `DOCS-COMPACT-8` — the wave-40 compaction pass over `AGENTS.md`, `CLAUDE.md`, `README.md`

**Base:** `6dc484ad` (confirmed equal to `origin/main` and to `main` at the
start of the card). **Scope:** `AGENTS.md`, `CLAUDE.md`, `README.md`, plus the
verbatim checkpoints and the wave record they move into, this report, and the
two tracker registrations this pass's own measurements produced.

The eighth firing of `CLAUDE.md`'s recurring compaction cadence. **Compaction is
not deletion.** No constraint, hard invariant or evidence pointer is dropped;
what is compacted out is copied **verbatim** into
[`docs/development/increments/wave-40-compaction-checkpoints.md`](../../development/increments/wave-40-compaction-checkpoints.md),
labelled as a checkpoint and stated there not to be a second current rule
authority. One current authority per rule.

**Headline.** 56 numbered entries before and after, 54 of them byte-identical.
`AGENTS.md` 191,823 → 172,076 bytes (−10.3%), and the fall is not spread thinly:
**two blocks account for 85% of it**, constraint 14's per-card keyword-census
ledger and the Q-space note's instance enumeration, both of which had become
chronicles of their own cards' reports. Eight corrections, each measured against
the tree. **Two of the eight were found only because the merged-gate row was
re-measured rather than carried** — which is the whole point of that row's second
arm, and this card's own wave-37 finding.

---

## 1. Numbers re-derived, and the source each was read from

Every figure below was read **from its source and never from the file being
compacted**, at base `6dc484ad`.

| Figure | Value found | Source, as run |
|---|---|---|
| Unit statuses | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only (185) | `diagnostics.json.gz` of `docs/reports/w37-corpus-contracts-4/artifacts/`, `Counter(u['status'])` |
| Function statuses | 4,834 total: 213 Safe / 84 Unsafe / 4,537 Unknown | same artifact, over every unit's `functions` |
| `flb_input_chunk_create` | `Safe` | same artifact |
| Diagnostic counts | leak **272**, `unknown_alias` **4,309**, `unsupported` **49,753**, `summary_conflict` **1**; `double_release` absent, i.e. **0** | same artifact's `diagnostic_counts` |
| CT mapping | **47** mapped / **3** unmapped (CT10, CT24, CT41) | `docs/reevaluation/ct_manifest.json`, `Counter(c['mapping_status'])` |
| CT sanitized-native | **10**: CT03, CT07, CT09, CT17, CT22, CT25, CT26, CT34, CT46, CT49 | same, `stages.native_sanitized.status == executed` |
| CT proof | **4**: CT01, CT03, CT07, CT37 | same, `stages.proof.status == executed` |
| Contract corpus | **88** modules | `find contract/qcore -name '*.qcore' \| wc -l` |
| resource-v1 declarations | **59**, across 33 files | `grep -rhc 'requires resource_contract_v1;' --include='*.qcore' contract/qcore` |
| Manifest-selected | **33** | `contract/cycle19-resource-contracts.json`, `len(d['modules'])` |
| Crates / workspace / providers | 9 / **10** / **6** | `ls crates/`, both `Cargo.toml` `workspace.members` |
| Versions | schemas 14.0.0; compiler, host, verification 0.4.0; ChunkIO package 0.13.0 | the four `Cargo.toml`s and `providers/qcore-provider-chunkio/provider-template.json` |
| Lean audits | **141** | `ls proof/Audit*.lean \| wc -l`, and `check_lean_audits.py`'s own "Passed 141 Lean audits" |
| Python suite | **303**, OK | `python3 -m unittest discover -s scripts/tests`, exit 0 |
| Lean pin | `leanprover/lean4:v4.34.0` | `proof/lean-toolchain` |
| Example families | **12**, and the named list matches the directories exactly | `ls -d examples/*/` |
| `named_followups` | **234** at the base (236 after this card's two registrations) | `docs/reviews/r0_3/acceptance_matrix.json`, and `dimensions.named_followups` agreeing |
| Merged gate | §6 — **measured by this card**, not carried | `docs/reports/docs-compact-8/gate.json` |

Every one of these was already correct in the ledger. **The ledger's figures are
in good health; the defects this pass found are all in the prose beside them** —
in the repository map, in a standing constraint, in `README.md`, and in the two
parts of the merged-gate row that no check covers.

### 1.1 The Q-space tally, verified rather than restated

`CLAUDE.md`'s brief flags this tally as fragile, and it is: nothing derives it,
and the wave-39 close had to correct it after it silently missed a card. It was
re-derived here from the note's own enumeration plus the §L21 table rows below
it, which is the only available derivation, and it **holds**: eighteen condition
extensions across waves 26–39, on ten bands — `Q020`–`Q021`, `Q051`–`Q057`,
`Q068`–`Q079`, `Q080`–`Q089`, `Q090`–`Q096`, `Q097`–`Q099`,
`Q300`/`Q302`–`Q309`, `Q600`, `Q610`–`Q627`, `Q630`–`Q641`.

The one near-miss worth recording: `AGGREGATE-OWNED-FIELD` (wave 39) also
reaches the `Q100`/`Q103` row, which would be an **eleventh** band. It is not
counted as one, and that is correct — its own §L21 row says `Q103`'s condition
"covers by definition" and was **not widened**, the vocabulary having grown
under an unchanged condition. The compacted note now states that explicitly, so
the next pass does not have to rediscover why ten is not eleven.

---

## 2. Standing-constraint inventory, before and after

Measured by `scripts/entry_hash.py` (shipped in this report), which splits the
`## Hard invariants` → `## Standing per-change gate` span into per-label blocks
and hashes each. Full listings:
[`artifacts/constraint-entries-before.tsv`](artifacts/constraint-entries-before.tsv),
[`artifacts/constraint-entries-after.tsv`](artifacts/constraint-entries-after.tsv).

| | before | after |
|---|---|---|
| Numbered entries (`grep -cE '^[0-9]+[a-z]?\. '`) | **56** | **56** |
| — hard invariants | 6 (`INV-1`…`INV-6`) | 6 |
| — standing constraints | 50 (1, 2, 3, 4, 5, 5a, 5b, 5c, 6–10, 10a, 10b, 11, 12, 12a, 13, 14, 15, 15a–15p, 16–28) | 50, same labels |
| Label order | — | **identical** (`diff` of the label columns is empty) |
| Bytes inside numbered entries | 124,953 | 111,454 |
| Entries whose block hashes **byte-identically** | — | **54 of 56** |

The two that moved, and no others:

| entry | before | after | why |
|---|---|---|---|
| `C-14` | 19,638 | 6,074 | compaction (§3.1) |
| `C-15h` | 15,852 | 16,917 | correction (§4.4) — it **grew**, because correcting an overstated boundary costs words |

**No entry was removed, renumbered or reordered, and no evidence pointer was
dropped.** Constraint 14's trailing evidence-pointer list in fact **gained five**
entries (`mutex-share-cross`, `write-lend-tasks-exercise`,
`aggregate-owned-field`, `ct42-cell-route`, `loops-on-composed-routes`): those
five cards had amended the constraint's body since wave 31 without ever being
added to its pointer list, each verified present under `docs/reports/`.

---

## 3. What was compacted, and what each compacted block keeps

### 3.1 Constraint 14 — the per-card keyword-census ledger (19,638 → 6,074)

The largest single accretion in the file: a card-by-card narrative of every
keyword census ever run, restating each card's own report. Three things made it
compactable without loss.

1. **The word set has a machine-readable authority.**
   `qcore_frontend::frontend::is_reserved_source_word` is the reserved-word
   table, **112 words** ([`artifacts/reserved-words.txt`](artifacts/reserved-words.txt),
   extracted from the `matches!` arms), and *each arm carries its own card's census in a source
   comment*. The chronicle in `AGENTS.md` was a second, staler copy of that.
2. **The standing decisions are separable from the narrative**, and they are
   what a refactor could undo. All of them are kept: the five rejected spellings
   (`stream`, `socket`, `record`, `struct`, `Buffer`-as-field-type) with the
   ground for each, including that `record` would falsify a frozen archive
   (constraint 21); `in` deliberately unreserved with its contract-corpus census
   site and the "do not promote it" instruction; the rename-rather-than-edit-the-pin
   shape (`work`→`effort`, `buffer`→`bytes`, `record`→`type`); every contextual
   rule (`error` only after `log`; `unroll`/`where` with the two-token head and
   the `c#0` premise; `lend` in three positions; arity-selected forms); the three
   name-resolution exclusions with **do not remove that exclusion** intact, and
   `AGGREGATE-SELECTOR-WORDS`' deliberate *non*-copy with its reason;
   `MUTEX-SHARE-CROSS`'s position-not-a-word claim.
3. **The decisions are now checkable.** The compacted entry says the five
   rejected spellings are absent from `is_reserved_source_word` — verified here
   (`lend`, `in`, `record`, `struct`, `socket`, `stream` all absent; `disjoint`,
   `for`, `bind`, `error` present), so a future census can check the claim
   mechanically instead of reading a narrative.

Kept as well: the **method finding**, which is the part a later reader most
needs — a wave-by-wave roll call of cards that *do not* engage invites a false
universal (one such list once contained a card that had run a census,
`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`), so the wave-37 and wave-39 closes
settled their entries by **measuring the front-end seam** with
`git diff --name-only <base> <tip> -- crates/qcore-frontend/src/` and attributing
each moved file with `git log --name-only`. Both closes' report pointers are
retained.

### 3.2 The Q-space note (6,644 → 3,474)

Kept: the exhaustion fact and the two spent pools; the candidates for a next
allocation; the icebox decision, recorded as **an open decision and not a rule**,
with its 2026-09-12 integrator record and its 2026-09-18 re-affirmation in the
wave-38 close's deferred decisions; the operative consequence and its
`SOCKET-EVENT-COMPOSE` / `NATIVE-LIBRARY-HEADER-DECORATION` precedent;
constraint 1 unchanged; deliberate **narrowings** named (`REGION-JOINS-LOAN`'s
`Q600`, `AGGREGATE-OWNED-FIELD`'s `Q055` as the first lift,
`LOOPS-ON-COMPOSED-ROUTES` as all-narrowings); the tally with its ten bands now
*listed* rather than merely counted; `Q-SPACE-EXTENSION-TALLY-HAND-KEPT`; **both
undercounting findings**, which are the note's most useful content; and the
cards that allocate nothing and are *not* instances, so the tally cannot be
padded.

Moved: the per-instance enumeration, which had accreted out of chronological
order (waves 26–31 in one run, then 32, then 36, then 39, then 33/34/37, then
wave 39 again at the end) because each wave appended to whichever sentence was
nearest.

**One figure was deliberately not restated.** The note said "the icebox has now
held for thirteen waves". Nothing in the note derives that, and the wave the
decision landed in is not stated anywhere I could measure, so the compacted text
says "across waves 26–39", which is exactly what its own enumeration supports.
This is the same discipline as the tally: a count with no derivation, replaced
by the span that has one.

### 3.3 Five ledger blocks

The arbiter-chain paragraph, the *Cycle 19 raw corpus* row, the *CT01–CT50* row,
the *Merged gate* row and the follow-on-count paragraph. Each kept its current
figures (all re-derived, §1) and its **standing method**, and shed the per-wave
narrative. The follow-on paragraph is the clearest case: it spelled out wave 39's
six-step registration chain and then said, in its own next sentence, that the
chain is the increment records' business.

Two improvements beyond shortening. The CT row now says the proof leg has **two
shapes, not one** — three legs bound through the F02 digest relation and CT07's
through the scoped-task ledger — which the old row stated but buried. The corpus
row now states the pre-pass reach figures as an explicit rule (*a reach figure is
a coverage fact and not a safety one*) rather than as two measurements.

### 3.4 `CLAUDE.md`'s Windows residual family (2,193 → 1,905)

Seven residuals stating one shape seven times: *the card's native test ran the
Linux arm only, the `windows_c` path is carried unchanged, no such run has
happened and none is claimed*. The shape is now stated once and the seven listed
under it. **Every name, every report pointer and every tracking status is
kept**, including the deliberate unevenness that is the actual open question:
`WRITE-LEND-WINDOWS-RUN` and `WRITE-LEND-TASKS-WINDOWS-RUN` registered in
`named_followups` since `DOCS-COMPACT-6`, `SLICE-PERMISSIONS-WINDOWS-RUN` and
`OWNED-FIELD-WINDOWS-RUN` named-here-only by the wave-39 close's decision, until
`PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` is decided. Verified by grepping
every name back out of the compacted file.

### 3.5 What was **not** compacted, and why

* **Constraints 15h (16,917), 16 (6,836), 15n (6,532), 5c (6,007), 7 (5,380).**
  Each is long because it carries a list of things that must survive refactors,
  not because it narrates. Constraint 7's leak-pin history in particular is a
  chain of *withdrawals and decisions* in which each step's direction is the
  load-bearing fact; shortening it would cost the direction.
* **The §L21 diagnostics table.** It is the summary of the spec's own ownership
  table and every row is a current rule.
* **`CLAUDE.md`'s two new conventions** — the mutation-count rule (wave-38
  close) and the integration-gate rule (wave-39 close). Both are authorities
  landed one and two waves ago; the pass compacted around them and touched
  neither.

---

## 4. Corrections — eight, each with its evidence

### 4.1 `README.md`: the conformance corpus stood at 46 of 50

> "The conformance-case corpus stands at **46 of 50 mapped**, with 4 unmapped."

`docs/reevaluation/ct_manifest.json` says 47 mapped and 3 unmapped (CT10, CT24,
CT41). Wave 39's `CT42-CELL-ROUTE` mapped CT42 — the ledger row and the merge
message `45a33285` both say so — and `README.md` was not updated with it.
**Corrected to 47 / 3.**

### 4.2 `README.md`: three proof stages, and the wrong shape for the fourth

> "Of the 46 mapped, 10 carry a sanitized native run and 3 carry a proof stage —
> CT01, CT37 and CT03, each a scenario property proved in Lean of the checked
> artifact's reference run and its admitted renderings"

The manifest's `proof.status == executed` set is **four**: CT01, CT03, CT07,
CT37. Two defects, not one: the count, and the description. CT07's leg is **not**
of the other three's shape — `AGENTS.md`'s own CT row says it is bound through
the scoped-task ledger's lease set rather than the F02 digest relation, because
`reference_trace` refuses this run's lease events by constraint 15n's decided
boundary. Repeating "each a scenario property … and its admitted renderings"
across all four would have over-claimed CT07. **Corrected to four, with the two
shapes distinguished.**

### 4.3 `README.md`: "no ThreadSanitizer row exists anywhere in this repository"

False since wave 37. `tests/support/thread_sanitizer.rs` exists (25,400 bytes),
introduced by `TASK-ROUTE-TSAN`; wave 39's `TSAN-ROWS-2` runs six subjects under
two forced schedules (`docs/reports/tsan-rows-2/README.md` §5's table). The
sentence's *purpose* — that the mutex crossing witnesses no contention — is still
true, and for a sharper reason the card itself measured: **CT46 is serial by
construction**, so its forcing header attaches to nothing and its clean row
witnesses nothing about contention. **Corrected to say that**, with the boundary
that a clean detector is evidence about the schedules realised, not a proof of
race freedom, and that the rows are Linux only.

### 4.4 Constraint 15h: the same claim, from the other direction

> "Three boundaries are **measured, not claimed**: ThreadSanitizer cannot run any
> program this route emits, lease or not (`TASK-ROUTE-TSAN-SEGV`)"

Wave 36 measured that and read the fault as the route's. Wave 37's
`TASK-ROUTE-TSAN` measured that it is **not** the route's — TSan ships no
`thrd_create` interceptor, so an emitted thread reaches `start_thread`
unregistered — **retired `TASK-ROUTE-TSAN-SEGV`** (wave-37 record,
`named_followups` 171 → 174), and bound `thrd_create` to `pthread_create` through
one `-include`, under which TSan runs these programs. Sharper still: `TSAN-ROWS-2`
runs **this constraint's own flagship**,
`fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore`, as the first row of
its subject table — clean, unforced and under both forced cohort schedules.

The constraint carried a **retired residual as a live boundary for three waves**.
**Corrected**, and deliberately without weakening anything: the exclusivity claim
still rests on C11 3.14, the ASan/UBSan/LSan run and the allocation identity,
with the TSan rows stated as beside them and not in their place, and the
"clean detector ≠ proof of race freedom" boundary written in. This is why the
entry grew by 1,065 bytes.

### 4.5 Repository map: `contract/qcore` as 76 modules

The map said **76**; the ledger row two sections above it says **88**, and the
tree has 88. Stale since wave 31 — through waves 36 and 37 adding six modules
each, and through two compaction passes. **Corrected to 88, with a pointer to
the ledger row as the one current figure** (and registered, §5).

### 4.6 Repository map: the gateway's "six load-bearing modules"

The row named six and said they were the ones "added by the waves their
constraints name". Four more have landed since, each under a standing constraint
that names its card: `legacy_declared_abi.rs` and `legacy_declared_modes.rs`
(constraint 8's two readers, which it forbids moving), `legacy_return_call.rs`
(wave 34, constraint 5b's modeled call in return position, added by `c8fd58f1`)
and `provenance_rounds.rs` (wave 37, constraint 10b's proved round bound, added
by `114694bc`). `crates/qcore-c-gateway/src/` holds **36** modules.
**Corrected to ten, with the derivation stated** — the list is derived from the
constraints, not from the directory, and a module the constraints do not name is
not thereby unimportant.

### 4.7 Merged-gate row: 240 result summaries, four doubled sections

> "240 result summaries over 236 test targets (227 `Running` plus 9 doc-test
> sections; four of those sections print two summaries each)"

This card's own run, on code identical to the wave-39 close's, measures **238**
summaries over the same 236 sections, with exactly **two** sections printing two
summaries each — `Doc-tests qcore_core` and `Doc-tests qcore_verifier_host`
(`scripts/summary_shape.py`, output in
[`artifacts/gate-summary-shape.txt`](artifacts/gate-summary-shape.txt)).
`grep -c '^test result:'` on the log agrees at 238. The passed/failed/ignored
triple reproduces **exactly** (4,727 / 0 / 10), which is why nothing caught this:
the figure that drifted is the one no check covers. **Corrected, and registered**
as `GATE-ROW-SUMMARY-COUNT-UNCHECKED` (§5).

### 4.8 Merged-gate row: "all five named rows that live in this workspace"

Constraint 16 names five rows. **Four** live in this workspace —
`a_declared_process_limit_stops_the_fixture_from_forking` (the `RLIMIT_NPROC`
row, `crates/qcore-verifier-host/tests/enforced_process_limits.rs`),
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths` and
`memory_growth_beyond_the_declared_limit_is_bounded`. The fifth,
`real_packages_preserve_unknown_and_reject_mutations`, is in
`providers/conformance/tests/packages.rs`, which the row itself says in its next
sentence. **Corrected to four**, so the row's claim and its own exclusion agree.

---

## 5. Names registered — two, both about figures nothing derives

`named_followups` **234 → 236**, added to the carried section and the matrix then
regenerated with this card's `gate.json`, so `dimensions.named_followups` is
derived and the two agree on the committed file (constraint 25). **No name was
retired**, and no other card's entry was touched.

| id | what it records | discharge |
|---|---|---|
| `GATE-ROW-SUMMARY-COUNT-UNCHECKED` | §4.7. The first arm of `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` made `lean_audits` and `clang` checked keys; the summary arithmetic beside them is still unchecked, and drifted for one wave with nothing to catch it — the same family as `GATE-SUMMARY-PARSER-OK-ONLY`. | the gate file carries the summary and target counts as keys `build_matrix.py --gate` derives from the run's own log, or refuses |
| `AGENTS-MAP-FIGURES-UNDERIVED` | §4.5 and §4.6. The repository map restates figures the ledger derives; a second copy of a number is a second thing to go stale, and two of them did — one for five waves. | the map stops carrying countable figures and points at the ledger row, or a test derives them the way `qcore_contract_corpus.rs` pins the corpus itself |

Two candidate registrations were **considered and not made**, because an existing
name already covers them: the Q-space tally (`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`)
and the Windows residual family's uneven tracking
(`PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED`). Registering a second name for
either would inflate the tracker without adding a claim.

---

## 6. The gate — this card's own measurement

Full accounting in [`gate.json`](gate.json); per-command exit files and the
Clang recording in [`gate/`](gate/).

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0, `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | **exit 0** — 4,727 passed / 0 failed / 10 ignored |
| `cargo fmt --all -- --check` | exit 0, empty diff |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | exit 0 — "Passed 141 Lean audits"; 141 `Audit*.lean` on disk |
| `python3 -m unittest discover -s scripts/tests` | exit 0 — 303 tests, **before and after** this card's edits and its matrix regeneration |
| `clang --version` | exit 0 — Ubuntu clang 21.1.8, **run in this worktree** and recorded at `gate/clang-version.txt` |

**How it was run.** One detached run, `setsid -f measure/gate/run.sh < /dev/null`,
**no `trap '' TERM`**; every step's exit written to its own file with **no pipe
in the command** (`GATE-EXIT-SWALLOWED-BY-PIPE`); polled by a bounded foreground
loop on its own `ALLDONE` marker, never by ending a turn on a notification. Logs
are worktree-private under `measure/`, per `CLAUDE.md`'s hygiene rule; only the
exits and the Clang recording are shipped.

**Why a documentation card may carry the merged-gate row.** The run started at
`624bab70` (this report's skeleton) and finished before `c188d3ab`;
`git diff --name-only 624bab70 c188d3ab` names **six files and all six are
documentation**. No crate source, no fixture, no artifact, no matrix. And no test
in this tree reads any of the three compacted files: the only `include_str!` of a
markdown file anywhere in `crates/` or `compiler/` names an
`examples/*/README.md`, and every `AGENTS.md` mention in a `.rs` file is a
comment. The figures therefore hold at the card's tip, and are offered as **one
measurement at one tip**, which is what that row says of itself.

**The summary figures were summed the safe way.** `scripts/sum_tests.py` sums
**every** `test result:` line, `FAILED` ones included — the shape
`GATE-SUMMARY-PARSER-OK-ONLY` names. All 238 read `ok`.

### 6.1 Which load-sensitive rows this gate saw

Constraint 16 requires a gate report to state which of its rows it saw. Four of
its five live in this workspace and **all four were green**:
`a_declared_process_limit_stops_the_fixture_from_forking`,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths`,
`memory_growth_beyond_the_declared_limit_is_bounded` — as were
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` and
`a_forced_override_never_weakens_an_execution`. The fifth,
`real_packages_preserve_unknown_and_reject_mutations`, lives in the separate
`providers/` workspace and was not run here. **Nothing flaked**, so no isolated
re-verification was required and none is claimed. This is a full-suite run on a
machine carrying its own build and is **not** the quiet-host isolated number
`HOST-ROW-STARVATION-DESCRIPTION` still asks for.

**Provider 122 is carried, not re-measured**: the card's whole diff is
documentation, `qcore-schemas` is still 14.0.0, so constraint 18 is not engaged.
It is also a Unix figure (`PROVIDER-CONFORMANCE-UNIX-ONLY`).

### 6.2 A check working

`build_matrix.py --gate` **refused** this card's first regeneration:

> `--gate docs/reports/docs-compact-8/gate.json claims clang = '…' but no
> clang-version.txt is recorded beside it … a presence claim with no recording
> is refused (CLANG-GUARDED-GREEN-FAMILY-750)`

That is the wave-38 close's required-key rule doing exactly what it was landed
for, against this card. The recording was copied beside the gate and the
regeneration then exited 0.

---

## 7. Byte counts, split compaction vs correction

| File | before | after | delta |
|---|---|---|---|
| `AGENTS.md` | 191,823 | **172,076** | **−19,747 (−10.3%)** |
| `CLAUDE.md` | 25,692 | **25,417** | −275 (−1.1%) |
| `README.md` | 13,178 | **13,796** | **+618 (+4.7%)** |
| total | 230,693 | **211,289** | −19,404 (−8.4%) |

Per-block, by `scripts/split_delta.py`
([`artifacts/byte-delta-split.txt`](artifacts/byte-delta-split.txt)):

| block | kind | delta |
|---|---|---|
| constraint 14, per-card census ledger | compaction | −13,564 |
| Q-space note | compaction | −3,170 |
| ledger: CT01–CT50 row | compaction | −1,778 |
| ledger: follow-on count paragraph | compaction | −546 |
| ledger: Cycle 19 raw corpus row | compaction | −409 |
| `CLAUDE.md` Windows residual family | compaction | −288 |
| ledger: headline and arbiter chain | compaction | −185 |
| `CLAUDE.md` cadence roster | compaction | +13 |
| ledger: merged gate row | correction + re-measurement | −1,494 |
| constraint 15h, the TSan boundary | correction | +1,065 |
| `README.md` the TSan claim | correction | +448 |
| repo map, gateway modules | correction | +422 |
| `README.md` conformance figures | correction | +166 |
| repo map, contract corpus count | correction | +49 |
| | **compaction** | **−19,927** |
| | **correction** | **+2,150** |
| | **gate row (both)** | **−1,494** |

Accounted −19,271 against an actual −19,404; the −133 residue is block-boundary
newlines, the blocks being measured without their trailing separators.

**The corrections cost 2,150 bytes and that is the rule working as written.**
`CLAUDE.md` says a pass may legitimately grow a file, and `README.md` grew by
4.7% here for exactly the reason `DOCS-COMPACT-6` recorded: correcting an
overstatement costs words. Two of the four `README.md`/15h corrections replaced a
short false sentence with a longer true one.

---

## 8. What moved verbatim into the increment history

Eight blocks, **46,603 bytes**, in
[`docs/development/increments/wave-40-compaction-checkpoints.md`](../../development/increments/wave-40-compaction-checkpoints.md):

| # | block | bytes |
|---|---|---|
| 1 | `AGENTS.md` constraint 14, the per-card census ledger | 19,750 |
| 2 | `AGENTS.md` Q-space note | 6,644 |
| 3 | `AGENTS.md` ledger, *CT01–CT50 conformance* row | 5,278 |
| 4 | `AGENTS.md` ledger, *Merged gate* row | 4,996 |
| 5 | `AGENTS.md` ledger, *Cycle 19 raw corpus* row | 3,065 |
| 6 | `AGENTS.md` ledger, headline and arbiter chain | 2,680 |
| 7 | `CLAUDE.md` platform records, Windows open-residual list | 2,201 |
| 8 | `AGENTS.md` ledger, follow-on count paragraph | 1,989 |

The page states in terms that these are **not** a second current rule authority,
that where a block and the current file disagree the current file wins, and that
blocks stating something this card measured to be false are **not** corrected in
place — the correction is §4, because editing a verbatim copy defeats it.

**The page is separate from `wave-40.md`, deliberately.** `DOCS-COMPACT-7` put
its checkpoints inline and took `wave-37.md` to 64,613 bytes, against the
retrieval guidance that a new history page stay below 32 KiB where a complete
record boundary permits it; that card flagged the split as the close's call to
make. This card makes it, for its own page only: `wave-40.md` stays a record
page, and the appendix is indexed from it, from the checkpoint page's own header
and from `docs/development/increments/README.md`'s wave table.

### 8.1 Nothing named was lost — measured, not asserted

Every `SCREAMING-KEBAB` name in the three files was extracted before and after
(`scripts/check_names.py`). **Eighteen** stopped appearing:
[`artifacts/names-no-longer-in-the-three-files.txt`](artifacts/names-no-longer-in-the-three-files.txt).
For each, the check asks whether it still lives in the three files under any
case, in a `docs/reports/` directory, in `named_followups`, or in the verbatim
checkpoint page —
[`artifacts/names-still-live-where.txt`](artifacts/names-still-live-where.txt).

**Lost in all four: none.** All eighteen are in the checkpoint page; fifteen are
also in a report directory or still in the files (`W29-CORPUS-CONTRACTS`,
`W31-CORPUS-CONTRACTS-2` and `W4-RECURSIVE-SUMMARIES` only changed case, the
arbiter chain naming them by report directory now). The three thinnest —
`OR-AND-LOWERING`, `T03-GUARD-JOIN`, `WAVE-34-CLOSE` — were each verified by
hand to live in the increment history as well (`wave-19.md`, `wave-05.md` and
`wave-32.md`, `wave-35.md` respectively, plus `docs/reports/wave34-close/`).
**Not one of the eighteen is a residual with an open discharge condition**; all
are card names cited as precedent inside the compacted narratives.

---

## 9. For the integrator

Every item below is addressed to the close and needs a recorded verdict
(`CLAUDE.md`'s reconciliation-sweep rule).

1. **The merged-gate row now names `DOCS-COMPACT-8`'s run at `c188d3ab`.** The
   close will re-measure at its own tip and re-own the row; that is expected, and
   the second arm of `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` asks only that the row
   name **whatever run produced its figures**. Please keep that sentence form.
2. **The summary arithmetic correction (§4.7) is the one figure to carry
   forward**: 238 summaries over 236 targets, two doubled doc-test sections. If
   the close's own run measures otherwise on unchanged code, that is a finding
   about the counting and should be reported as one, not silently reconciled.
3. **`GATE-ROW-SUMMARY-COUNT-UNCHECKED` proposes a mechanical discharge** (two
   more `--gate` keys derived from the run's log). That is generator work, not
   this card's; the residual is registered either way.
4. **`AGENTS-MAP-FIGURES-UNDERIVED` proposes the repository map stop carrying
   countable figures.** This card corrected the two stale ones but deliberately
   did not restructure the map — that is a decision, not a compaction.
5. **Constraint 14's evidence-pointer list gained five card names** it should
   have had since wave 32. Worth a glance that the convention is "every card that
   amends the constraint appears in its pointer list"; if it is not, the five
   should come back out.
6. **`CLAUDE.md`'s cadence bullet now says the ninth pass is due around wave 42
   or 43.** That is this card reading its own 2–3-wave rule, not a schedule
   decision; overrule it freely.
7. **The checkpoint-page split (§8) is a precedent.** If the close prefers
   checkpoints inline, say so and the next pass will follow — but `wave-37.md` is
   the reason this one did not.
8. **`STABILITY-REEVAL-4` runs this wave and owns the tracker.** This card
   registered two names and retired none; if the sibling's own registrations
   collide, its text wins and this card's entries should be merged, not
   duplicated.
9. **Not measured, and flagged rather than assumed:** whether the "icebox has
   held for thirteen waves" figure the Q-space note carried is right. It was
   replaced by the span its own enumeration supports (§3.2) rather than
   re-derived, because nothing in the tree records which wave the decision landed
   in. If the close knows, the sentence can carry the count again.

---

## 10. What the rebase re-derivation changed

_To be completed when `STABILITY-REEVAL-4` merges and main's new sha is known.
Per the brief, every number that moved will be **re-derived from its source**
rather than rebased, keeping this report and its scripts._
