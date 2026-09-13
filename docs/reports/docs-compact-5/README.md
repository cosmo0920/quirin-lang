# DOCS-COMPACT-5 — fifth compaction pass

**Date:** 2026-09-13. **Base:** main `6caea680` (wave-32 increment record
opened, on top of the wave-31 close `8bf7fd33`). **Wave 32.**
**Scope:** documentation only. The diff touches `AGENTS.md`, `CLAUDE.md`,
`README.md`, one new paragraph in `docs/development/increments/wave-32.md`,
this report directory and the worktree-private `measure/` tooling. No crate
source, no contract module, no provider file, no test, no spec, no acceptance
file, no manifest, no matrix, no frozen archive and no existing report changed.
**Corpus-neutral by construction:** nothing this card touches is read by the
gateway, the audit driver or a provider, so **this wave runs no arbiter** and
no corpus row moves.

## 0. Cadence

The cadence rule compacts `AGENTS.md`, `CLAUDE.md` and `README.md` every 2–3
waves. Precedents: DOCS-COMPACT (2026-09-10, wave 16), DOCS-COMPACT-2
(2026-09-11, after wave 23), DOCS-COMPACT-3 (2026-09-12, after wave 26),
DOCS-COMPACT-4 (2026-09-13, after wave 29). This card runs after wave 31 — two
waves on, the cadence is honored — and `CLAUDE.md` now records it as the fifth
precedent with the **next pass due around wave 34**.

Its material is what waves 30–31 appended: two wave closes, **two arbiters in
one wave** (a first), a moved corpus baseline, a docs restructure performed by
another tool (increment records now under `docs/development/increments/`), the
amendments to constraints 4, 15h and 15m, one new constraint (10b), and the new
every-2-wave `STABILITY-REEVAL` cadence in `CLAUDE.md`.

## 1. What this card did and did not do

It changed **no fact about the system**. No verdict, count, pin, boundary or
premise moved. Eight pieces of prose that had drifted from their sources were
re-read from those sources and corrected (§4); the material waves 30–31 landed
in card reports and §L21 but not in these files was synchronised in (§3); and
the restructure's seams were checked mechanically (§5).

**This pass was synchronisation-heavy again, and the files grew** — within the
single-digit band the rule names, and for the reason DOCS-COMPACT-4 recorded:
the method yields net growth whenever the closes have under-synchronised, and
its real value is the re-read and the inventory. The wave-31 close reconciled
the headline numbers but **not** `WRITE-LEND-TASKS`' explicit integrator
reconciliation list (§4.1), which is the third consecutive wave in which a
per-card condition widening reached §L21 and not `AGENTS.md`'s summary table.

## 2. Before / after

| File | Before (at `6caea680`) | After | Change |
|---|---|---|---|
| `AGENTS.md` | 1,219 lines / 104,330 bytes | **1,259 / 109,709** | +40 lines, **+5,379 bytes (+5.2%)** |
| `CLAUDE.md` | 170 / 10,711 | **176 / 11,130** | +6 lines, +419 bytes (+3.9%) |
| `README.md` | 197 / 11,711 | **198 / 11,970** | +1 line, +259 bytes (+2.2%) |
| `docs/development/increments/wave-32.md` | — | +1 increment paragraph | per the sync rule |

Where the bytes went, and what offset them:

- **Growth (deliberate, all synchronisation):** constraint 15h's
  `WRITE-LEND-TASKS` append, carried from that card's §12 proposal (~2.2 KB);
  the `Q020`/`Q021`, `Q302` and `Q600` diagnostics rows catching up to §L21's
  wave-28/30/31 widenings (~2.0 KB); constraint 14's census ledger gaining
  waves 30–31; the Q-space note gaining waves 30–31 and three named
  non-instances; the repository map's gateway module list; the CT row's
  observed/proved separation; `WRITE-LEND-TASKS-WINDOWS-RUN` in `CLAUDE.md`'s
  platform records; two `README.md` refreshes.
- **Trims:** the sync rule and the retrieval section's duplicated
  "update the increment index / do not append to the root table of contents"
  instruction merged into the sync rule (§5.2); the ledger header's restatement
  of the two arbiters' per-delta counts replaced by the chaining fact plus the
  two report pointers (the corpus row two paragraphs below owns those counts);
  the retrieval section's duplicate of `CLAUDE.md`'s worktree-private log rule
  replaced by a pointer.

## 3. What changed, section by section

### 3.1 `AGENTS.md` — the diagnostics table catches up to §L21 (three rows)

The table's own header says it is "a summary of §L21, not a second
allocation". §L21 was re-read for every wave-28/30/31 card:

- **`Q020`, `Q021`:** `WRITE-LEND-TASKS` (wave 31) widens `Q020`'s
  task-parameter condition — the admitted worker parameter types grow by the
  ungraded `lend<Buffer>` range write lease, receivable only from a
  `lend PLACE LOW..HIGH` capture, the place/parameter agreement check covering
  the new pair exactly as it covers `borrow<Buffer>`, `lend` staying
  contextual. Spec §L21 line 978; **no code allocated**.
- **`Q300`, `Q302`–`Q309`:** `REGION-JOINS-LOAN` (wave 30) is `Q302`'s
  **fourth** composed-route widening — the adoption `move NAME;` beside a
  worker's counted read share, with an adopted name the entry does not bind
  and one owner taken twice through either shape as that card's own `Q302`
  conditions. Spec §L21 line 1001; **no code allocated**.
- **`Q600`:** the row carried only `CT47-COPY-CAPTURE`. It now carries all four
  movers with their directions kept distinct — `REGION-CREDIT-JOINS` (wave 28,
  a widening the wave-28 close never reached this row), `CT47-COPY-CAPTURE`
  (wave 29), `REGION-JOINS-LOAN` (wave 30, a deliberate **narrowing**,
  discharging `T08-COMPOSE-LOAN`) and `WRITE-LEND-TASKS` (wave 31, three
  widenings: the judgment's grown vocabulary, the lease's check-time bound
  decisions, and the ledger's own lease errors through the unedited
  `check_task_module`) — plus `CT24-CLEANUP-PREEMPTED` (wave 30) recorded as
  moving no code and widening no condition. Spec §L21 line 1004.

Each addition points at the constraint (15h/15m) that carries the rules, so the
row stays a summary rather than a second statement of them.

### 3.2 `AGENTS.md` — constraint 14 and the Q-space note

Constraint 14's per-card census ledger stopped at wave 29. Added, each read
from its card report rather than assumed: `WRITE-LEND-TASKS` (**+0 words, one
contextual clause word reused** — the wave-29 `lend` recognised in two further
positions that refused before the card: immediately before `<Buffer>` in a task
parameter type, and as a capture's first word inside a spawn's argument list),
and the eleven waves-30/31 cards that do not engage the constraint at all. The
existing "does not engage" list was merged into one entry rather than left as
three sentences; every card name, count, keyword, census figure, rejected
candidate and "do not" rule in the constraint is byte-identical (word-diff in
§6). Both waves closed at **zero reserved words**.

The Q-space note moves from "**Waves 26–29** ran the consequence **eleven**
times, on **eight** bands" to "**Waves 26–31** … **thirteen** times, on
**nine** bands" — wave 30's `REGION-JOINS-LOAN` (`Q302` a fourth time plus the
`Q600` narrowing) and wave 31's `WRITE-LEND-TASKS` (`Q020` and `Q600`, whose
`Q020`/`Q021` band is the ninth). The non-instance sentence gains two: the
existing `GATEWAY-C-LOCATION` is joined by `CT24-CLEANUP-PREEMPTED` (fixing
where a diagnostic is *attributed* is not a code change, constraint 1) and
`CT42-PROTOCOL-CELL` (which reaches the footprint checker's existing `Q664`
from a new deriver, allocating none and widening none).

### 3.3 `AGENTS.md` — constraint 15h gains `WRITE-LEND-TASKS`

`docs/reports/write-lend-tasks/README.md` §12 proposed a one-paragraph append
to constraint 15h, explicitly noting that the card "supersedes nothing in 15h;
it discharges the *sibling* residual the wave-29 card listed". The wave-31
close did not land it, so the constraint's wave-29 sentence — the write lend
"on the **uncomposed buffer route only**" — stood beside a shipped, Lean-proved
task-route lease with no rule text anywhere in the three files.

The append is carried, condensed, with every "must survive refactors" rule and
every residual name kept: the construct and its window (`lend PLACE LOW..HIGH`,
`BUFFER_PAYLOAD_BYTES = 4`, exactly spawn..join, decided by the lease-extended
scoped-task ledger's `ResourceState::WriteLeased` rather than by a route rule);
the "do not re-implement, do not move it out of the IR" rule on
`ownership::axes_overlap` called by `scoped_tasks::LeaseRange::overlaps`; the
three asymmetries (no whole-payload owner authority including reads while any
lease is out; an **empty** lease refuses where the sequential empty lend admits;
only the creator's join reclaims); `TransferMode::LendWrite` kept distinct from
the still-refused `write` capture; the audited
`QCore.WriteLendTasks.no_byte_has_two_write_authorities`; and
`WRITE-LEND-TASKS-EXERCISE` stated in the card's own honest terms (§7, F2).

### 3.4 `AGENTS.md` — the repository map and the ledger

The gateway row's "**four** load-bearing modules" became **six**:
`legacy_neutral.rs` and `legacy_joined_provenance.rs` were already named as
load-bearing by constraints 4 and 10b but missing from the map. The row is also
shortened by factoring the repeated `crates/qcore-c-gateway/src/` prefix.

The ledger header's narration of the two arbiters' deltas (five `unsupported`
leaves, six leak rows, five function moves) was removed: the corpus row two
paragraphs below states all of them with their attribution, and each arbiter's
report is now pointed at by path. What the header keeps is the fact only it
carries — **the chain**: `C-GATEWAY-WAVE`'s BEFORE reproduced wave 30's
`w4-recursive-summaries` archives byte for byte, and the second arbiter's fresh
BEFORE reproduces `c-gateway-wave`'s own `diagnostics.json` and
`chunk-contracts.json` byte for byte, which is what isolates each delta. The
frozen-archive list (constraint 21) is untouched.

### 3.5 `CLAUDE.md` and `README.md`

`CLAUDE.md`: the cadence rule records this card and the next due wave (34); the
Windows platform records gain `WRITE-LEND-TASKS-WINDOWS-RUN` beside its wave-27
and wave-29 siblings, on the same footing and with the same no-claim scope,
read from that card's §9, and record that wave 30's `CT07-WRITE-REMAP` left
`WRITE-LEND-WINDOWS-RUN` unchanged. Nothing else moved. The whole
`STABILITY-REEVAL` rule, landed at `1c7d8e45`, was re-read and left byte-identical.

`README.md`, refreshed as a human summary in the ledger vocabulary:

- `std.buffer`'s row gains the wave-31 language head — with `std.concurrency`, a
  lease of a byte range crosses the spawn boundary to one worker for exactly
  spawn..join — carrying the boundary in the same sentence ("held and enforced
  there; no worker operation exercises it yet").
- The "where to read more" sentence is repointed (§5.4).
- The status numbers were already current (43/50, 7 unmapped, 7 sanitized-native,
  3 proof, CT01/CT37/CT03, 2/42/137 + 4 data-only, twelve families) — the
  wave-31 close had refreshed them; re-verified, not re-written.

## 4. Drift found and fixed (eight items)

Every headline number was re-read from its source. The wave-31 close's ledger
is right everywhere this card looked; the eight drifts are all in text the
close does not own.

| # | Drift | Source re-read | Fix |
|---|---|---|---|
| 1 | Repository map said contract corpus **74 modules** | `find contract/qcore -name '*.qcore' \| wc -l` → **76**; `w31-corpus-contracts-2/README.md` "74 → 76"; ledger row already 76 | map row → 76 |
| 2 | Repository map said the gateway's "**four** load-bearing modules" | `ls crates/qcore-c-gateway/src/`; constraints 4 and 10b already name `legacy_neutral.rs` and `legacy_joined_provenance.rs`; `w4-recursive-summaries` §2.2, `c-gateway-wave` §3 | → **six**, each with the wave and constraint that made it load-bearing |
| 3 | Ledger said named follow-ons **39 entries** | `acceptance_matrix.json` `named_followups` → **40** | → 40 |
| 4 | Constraint 23 carried "7 of 42 and 1 of 42 at the wave-29 close" as the ledger pointer | `ct_manifest.json` tally → 43 mapped / 7 unmapped, `native_sanitized` executed **7**, `proof` executed **3** (CT01, CT03, CT37) | → "7 of 43 and 3 of 43 at the wave-31 close" |
| 5 | Constraint 25 said "a live corpus (**74** at wave 29)" | same as #1 | → "(76 at wave 31)" |
| 6 | Constraint 14's census ledger stopped at wave 29 | `write-lend-tasks` §10; the eleven other wave-30/31 card reports | waves 30–31 appended (§3.2) |
| 7 | Q-space note stopped at wave 29 ("eleven times, on eight bands") | §L21 lines 978, 1001, 1004; card reports | → waves 26–31, thirteen times, nine bands, three non-instances (§3.2) |
| 8 | Diagnostics rows `Q020`/`Q021`, `Q302`, `Q600` behind §L21's wave-28/30/31 widenings | `docs/QCORE_SPEC_r0_3.md` §L21 rows 978/1001/1004; card reports | rows carry the widenings (§3.1) |

### 4.1 The unapplied integrator reconciliation list

Drifts 6, 7 and 8, plus §3.3's constraint append, are **one root cause**:
`docs/reports/write-lend-tasks/README.md` §12 is an explicit, card-authored
reconciliation list naming three `AGENTS.md` items (the 15h append; the
`Q020`/`Q021` and `Q600` rows; the Lean-audit headline). The wave-31 close
applied the headline numbers and left the other two. This is
`DOCS-COMPACT-4-TABLE-SYNC` recurring for a **third consecutive wave**, and it
is re-raised here as `DOCS-COMPACT-5-RECONCILIATION-LIST` (§8) because the
earlier follow-on's suggested remedy — a sentence in the next wave's card
template — was not enough: the cards *did* write the reconciliation lists; the
close did not work through them.

### 4.2 Numbers re-read and found **in agreement** (no drift)

| Number | Source re-read | Result |
|---|---|---|
| Workspace 4,511 · provider 122 · Python 200 · probes 0/0 · fmt/clippy clean | `docs/reports/wave31-close/gate.json` | ✓ |
| Lean audits 125 | `find proof -name 'Audit*.lean' \| wc -l` | **125** ✓ |
| Corpus 185 units 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only; 4,834 functions 146/85/4,603; leak **272**; `unknown_alias` **4,208**; `unsupported` **50,150**; `double_release` **1**; baseline `w31-corpus-contracts-2/artifacts/` | `docs/reports/w31-corpus-contracts-2/README.md` headline; `docs/reports/c-gateway-wave/README.md` | ✓ |
| CT 43/50 mapped, 7 unmapped; sanitized-native 7; proof 3 (CT01, CT03, CT37) | `docs/reevaluation/ct_manifest.json` tally | ✓ |
| Contract corpus 76; resource-v1 41 → 43; manifest-selected 19 → 21 | corpus `find`; `w31-corpus-contracts-2` README | ✓ |
| `qcore-schemas` 14.0.0 · compiler / host / verification 0.4.0 · ChunkIO **package** 0.13.0 | the four `Cargo.toml`s; `providers/qcore-provider-chunkio/provider-template.json` `package_version` | ✓ (the *crate* is 0.5.0; the ledger's number is the package manifest's, correctly) |
| Nine crates, ten workspace members, six-member providers workspace | `ls crates/`, `Cargo.toml` members, `providers/Cargo.toml` | ✓ |
| Twelve example families | `ls -d examples/*/` | **12** ✓ |
| Cycle records `CYCLE_2` … `CYCLE_20` | `ls docs/cycles/` | ✓ |

The matrix was **not** regenerated and no number disagreed with it.

## 5. Restructure seams — checked specifically

The `DEVELOPMENT.md` split was performed by a different tool (`c05b2c4b`,
"docs: split development guide and integrate compact retrieval strategy"). Four
checks, all mechanical where a mechanism exists.

### 5.1 Every pointer resolves

`measure/links.py` extracts every markdown link target and every backticked
repository path from `AGENTS.md`, `CLAUDE.md`, `README.md` **and**
`DEVELOPMENT.md`, resolves it on disk, and for `.md#fragment` targets checks
the fragment against the target file's own headings under GitHub's slug rule
(which replaces each space with one hyphen and does **not** collapse runs — the
first version of the checker collapsed them and produced one false positive on
`#quant-delimiter-alias---2026-09-09`).

```
checked 149 path references across 4 files
unresolved paths: none
unresolved fragments: none
```

**Nothing in the three files describes the pre-split layout.** Every
`DEVELOPMENT.md` reference in them is to the table of contents or one of its
anchors; every increment reference goes through
`docs/development/increments/README.md`.

### 5.2 The sync rule and the retrieval section had duplicated one instruction

The split left two homes for "update the increment index when a new wave file
opens; do not append narratives to the root table of contents" — the sync rule
(under the file header) and the new "Documentation retrieval and compact
context" section. Merged into the sync rule, which is where the "one paragraph,
newest first, in the current record file" sentence already lives; the retrieval
section keeps the navigation half ("the increment index names the current
record file and every historical wave"). This is the pass's clearest
compaction: one rule, one authority.

### 5.3 The sync rule and the increment index agree

Four statements of the same convention were read side by side and **agree**:

| Where | What it says |
|---|---|
| `AGENTS.md` sync rule | one paragraph, newest first, to the **current record file named by** `docs/development/increments/README.md`; update that index when a wave file opens; headline numbers here |
| `CLAUDE.md` "New status text goes in one place" | one paragraph, newest first, in the current record file named by the index, plus `AGENTS.md`'s headline numbers; update the index when a new wave file opens |
| `DEVELOPMENT.md#increment-records` | the index "names the **current record file**, every wave, and the early and relocated archives"; add one paragraph to that current file, newest first |
| `docs/development/increments/README.md#adding-a-record` | "Current record file: `wave-32.md`. Prepend one paragraph per new increment…" |

### 5.4 Two seams found

- **Fixed (`README.md`):** "Verdict vocabulary, invariants, per-increment
  evidence and the full cycle narratives are in DEVELOPMENT.md and
  `docs/reports/`." Post-split, `DEVELOPMENT.md` is a table of contents and
  neither the verdict vocabulary nor the hard invariants live under it or under
  `docs/development/` at all — both are `AGENTS.md`'s. Repointed; the
  per-increment and cycle halves still go through the table of contents.
- **Reported, not edited (`DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`):**
  `docs/development/increments/README.md#adding-a-record` says to prepend
  "below the checkpoint notice and **above the migrated section**". The
  `<!-- migrated-section -->` markers exist on `wave-30.md` and earlier but
  **not** on `wave-31.md` or `wave-32.md`, so that clause no longer describes
  the current record file. It is harmless (it still describes the older pages)
  and the file is outside this card's three, so it is named rather than edited.

## 6. The standing-constraint inventory (the load-bearing check)

**45 entries before, 45 after. Zero dropped, zero renumbered.** Verified
mechanically by `measure/inventory.py` (output archived at
`measure/constraint_inventory.txt`, worktree-private per the hygiene rule),
which extracts the numbered entries, the hard invariants, every backticked
evidence-pointer slug, every backticked residual name and every `Q…`/`QP…`/`L…`
code from `git show 6caea680:` and from the working tree, then diffs the label
sets:

```
standing constraints: 45 before, 45 after   lost: none   gained: none
  identical bodies: 41      reworded: ['14', '15h', '23', '25']
hard invariants:      6 before, 6 after   lost: none   gained: none   identical: 6
slugs AGENTS.md:    106 before, 107 after   lost: none   gained: ['write-lend-tasks']
residuals AGENTS.md: 77 before, 84 after   lost: none
  gained: ['CT24-CLEANUP-PREEMPTED', 'CT42-PROTOCOL-CELL', 'DOCS-COMPACT-4',
           'STABILITY-REEVAL', 'TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED',
           'W4-RECURSIVE-SUMMARIES', 'WRITE-LEND-TASKS-EXERCISE']
codes AGENTS.md:    115 before, 116 after   lost: none   gained: ['Q664']
residuals CLAUDE.md: 14 before, 17 after   lost: none
  gained: ['CT07-WRITE-REMAP', 'STRING-CONCAT-WINDOWS-RUN',
           'WRITE-LEND-TASKS-WINDOWS-RUN']
README.md: slugs/residuals/codes unchanged
RESULT: nothing dropped
```

(The 45 are DOCS-COMPACT-4's 44 plus **10b**, added by `C-GATEWAY-WAVE` in
wave 31: "Non-nullness does not establish lookup identity".) The four reworded
bodies, each word-diffed by `measure/reworded.py`:

| # | Subject | Disposition |
|---|---|---|
| **14** | keyword census ledger | **Extended, then merged**: waves 30–31 appended per §3.2, and the three sentences listing cards that do not engage the constraint merged into one bulleted entry. Every existing count, keyword, census figure, rejected candidate (`stream`, `socket`, `record`), "do not" rule and evidence pointer is byte-identical. The one reworded existing clause is "the first two touch no crate source" → "the documentation cards touch no crate source", which names the same two cards now that the list is longer |
| **15h** | concat / borrow / lend | **Pure insert plus one pointer** (the word-diff shows a single `[insert]` opcode and `write-lend-tasks` appended to the evidence list). Not one existing word changed; see §3.3 |
| **23** | CT mapping vs conformance | Numerator pointer refreshed only: "7 of 42 and 1 of 42 at the wave-29 close" → "7 of 43 and 3 of 43 at the wave-31 close". The rule half is byte-identical |
| **25** | matrix generator ownership | "a live corpus (74 at wave 29)" → "(76 at wave 31)". Nothing else moved |

**Constraints 4, 15m and 10b — the amendments and the new entry the brief
named — needed no reconciliation.** Constraint 4's wave-30 amendment ("above
the neutral element", with both load-bearing premises and their Lean names) and
constraint 15m's two amendments (`REGION-JOINS-DUTIES` wave 29,
`REGION-JOINS-LOAN` wave 30) were read in full against their card reports and
are already correct and complete; 10b was landed by `C-GATEWAY-WAVE` with its
production-residual caveat intact. None was edited. The one superseded premise
found anywhere was 15h's, and that is §3.3.

## 7. `STABILITY-REEVAL`: observed stated as settled

The new cadence rule (user direction, 2026-09-13) adds one step to this
method: flag any sentence in the three files that states as settled something
the evidence only *observes*. Three findings; two were repaired by carrying the
evidence's own words, one is flagged and deliberately **not** reworded.

**F1 — the CT ledger row gave CT03's proved side without its observed side.
Repaired.** The row said CT03's alphabet growth was "proved **conservative** so
that a write-free trace relates exactly as before and no existing digest
moved", which is true and is the *write* half. The wave-31 record states the
other half plainly and the ledger did not: the same integration decision's
sibling arm — the two lease events, still refused — is unexercised, because
`reference_trace` has no caller outside its own module and no test drives a
lease-carrying reference run into it, so the two green suites are evidence
about the paths they took. The row now carries both halves with the residual
name `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, the bounded-by-direction
statement (the arm refuses rather than admits), and the pointer to the cadence
it occasioned. This is method step 2 — separate the proved side from the
observed side of a boundary — applied to the case that produced the rule.

**F2 — constraint 15h's append could have read as a two-threads-mutating
claim. Repaired at the source.** The append carries
`WRITE-LEND-TASKS-EXERCISE` in the card's own honest terms: **no operation
admits a lease in a worker body**, so the authority is *held and enforced*, not
exercised, and the sentence says so before it says anything about
exclusivity — even though
`QCore.WriteLendTasks.no_byte_has_two_write_authorities` is a real theorem over
every reachable point of every interleaving.

**F3 — the examples sentences read unconditionally. Flagged, not reworded
(`DOCS-COMPACT-5-EXAMPLES-HOST-GATE`).** `AGENTS.md`'s examples bullet says
every program "is byte-compared against its reference execution by a workspace
test", and `README.md` says "its emitted C is built and run under the
sanitizers". Measured rather than inferred:
`compiler/tests/examples_suite.rs` carries
`#![cfg(any(target_os = "linux", target_os = "macos"))]` at line 39 and compiles
with `-fsanitize=address,undefined` at line 190 — so both sentences hold on
Linux and macOS, and the entire suite compiles out on Windows. That is a
**scope omission rather than an unproved claim**, it is already covered by the
`*-WINDOWS-RUN` residual family in `CLAUDE.md`'s platform records, and per the
rule's own instruction ("do not silently reword a claim whose status you have
not measured" — and, where postponement is the outcome, record it as one)
neither sentence was touched. Handed to the first `STABILITY-REEVAL` card, due
after wave 33.

## 8. Follow-ons

- **`DOCS-COMPACT-5-RECONCILIATION-LIST`** (§4.1). Three consecutive waves have
  now closed with a per-card integrator reconciliation list only partly worked
  through, and `WRITE-LEND-TASKS` §12 is the first to have been written as an
  explicit numbered list and still left. Discharge condition: a wave close whose
  own record states, per card, that every item of that card's reconciliation
  list was applied or explicitly declined.
- **`DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`** (§5.4): the increment index's
  "above the migrated section" clause no longer describes the current record
  file. Discharge: drop or scope the clause when that file is next edited.
- **`DOCS-COMPACT-5-EXAMPLES-HOST-GATE`** (§7 F3), for the first
  `STABILITY-REEVAL` card.
- The next pass (due around wave **34**) inherits DOCS-COMPACT-3's floor
  observation and DOCS-COMPACT-4's growth observation unchanged, with this pass
  as a third data point: two of the five passes have now been net-growth
  synchronisation passes, and the value delivered was eight drifts, one
  unlanded constraint amendment, two restructure seams and three
  observed-vs-settled findings — not bytes removed.

## 9. Gate

Run at the card tip on Linux with `$HOME/.elan/bin` on `PATH`. The gate command
is `cargo test --workspace --no-fail-fast` (`GATE-NO-FAIL-FAST`), and the Lean
step is read from its **text** line per that script's own behaviour.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,511 passed, 0 failed, 8 ignored** across 223 test binaries, exit 0 — the wave-31 close number exactly; **no constraint-16 load row tripped**, so no row needed isolation |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | exit 0 — **`Passed 125 Lean audits`** |
| `python3 -m unittest discover -s scripts/tests` | **200 tests, OK** (exit 0) |

No provider rebuild: `providers/package.py` inputs untouched, `qcore-schemas`
stays 14.0.0, constraint 18 not engaged; the provider row carries at 122.
No arbiter was run and no corpus artifact was produced or read.

## 10. Measurement hygiene

`measure/inventory.py`, `measure/reworded.py` and `measure/links.py`, and their
archived outputs `measure/constraint_inventory.txt` and `measure/links.txt`,
are worktree-private under `measure/` (gitignored) — never under `target/` and
never in the shared session scratchpad.
