# DOCS-COMPACT-4 — fourth compaction pass

**Date:** 2026-09-13. **Base:** main `30aee0b2` (wave-29 close). **Wave 30.**
**Scope:** documentation only. The diff touches `AGENTS.md`, `CLAUDE.md`,
`README.md`, one new paragraph in `DEVELOPMENT.md`'s "Increment records", this
report directory and the worktree-private `measure/` tooling. No crate source,
no contract module, no provider file, no test, no spec, no acceptance file, no
manifest, no matrix, no frozen archive and no existing report changed.
**Corpus-neutral by construction:** nothing this card touches is read by the
gateway, the audit driver or a provider, so no arbiter is engaged and no
corpus row moves.

## 0. Cadence

The cadence rule compacts `AGENTS.md`, `CLAUDE.md` and `README.md` every 2–3
waves. Precedents: DOCS-COMPACT (2026-09-10, wave 16), DOCS-COMPACT-2
(2026-09-11, after wave 23), DOCS-COMPACT-3 (2026-09-12, after wave 26). This
card runs after wave 29 — three waves on, the cadence is honored — and
`CLAUDE.md` now records it as the fourth precedent with the **next pass due
around wave 32**. Its material is exactly what DOCS-COMPACT-3 §12 predicted:
"whatever waves 27–29 append" — three closes, the six new or amended standing
constraints (12a, 15j, 15k, 15l, 15m twice-amended, 15h twice-amended), new
ledger rows, a new corpus baseline pointer, the first proof row and the
diagnostics-table growth.

## 1. What this card did and did not do

It changed **no fact about the system**. No verdict, count, pin, boundary or
premise moved. Five pieces of prose that had drifted from their sources were
re-read from those sources and corrected (§4), and the material waves 28–29
landed in card reports and §L21 but not yet in these files was synchronised in
(§3).

**This pass was synchronisation-heavy, and the files grew.** The cadence rule
says to expect single-digit percentages and that it bounds accretion rather
than promising a shrinking file; DOCS-COMPACT-3 additionally recorded that
`AGENTS.md` is at its floor for the compaction method. What three waves
appended (five new constraints and two amendments among them) had largely been
kept in sync by the closes; what had **not** been synchronised — the
diagnostics-table rows, the census ledger, the Q-space note, one platform
residual — is the growth here, offset only partly by trims.

## 2. Before / after

| File | Before (at `30aee0b2`) | After | Change |
|---|---|---|---|
| `AGENTS.md` | 1,096 lines / 92,530 bytes | **1,114 / 95,189** | +18 lines, **+2,659 bytes (+2.9%)** |
| `CLAUDE.md` | 128 / 7,752 | **133 / 8,063** | +5 lines, +311 bytes (+4.0%) |
| `README.md` | 183 / 10,441 | **190 / 11,223** | +7 lines, +782 bytes (+7.5%) — a refresh, not a compaction |
| `DEVELOPMENT.md` | — | +1 increment paragraph (wave 30) | per the sync rule |

Where the bytes went, and what offset them:

- **Growth (deliberate, all synchronisation):** the `Q051`–`Q057`, `Q077` and
  `Q302` diagnostics rows catching up to §L21's recorded wave-28/29 widenings
  (§3.1); constraint 14's census ledger gaining waves 28–29 (§3.2); the
  Q-space note gaining waves 28–29's four uses; `WRITE-LEND-WINDOWS-RUN` in
  the platform records; the README refresh (§3.4).
- **Trims:** the Q-space note's two per-wave narrations merged into one run
  (waves 26–29, eleven uses, eight bands); the corpus row's duplicated
  "authoritative artifacts" sentence removed (the ledger header two paragraphs
  above states the baseline); constraint 23's duplicated CT numerators
  replaced with a pointer at the ledger row (§4).

## 3. What changed, section by section

### 3.1 `AGENTS.md` — the diagnostics table catches up to §L21

The table's own header says it is "a summary of §L21, not a second
allocation". §L21 was re-read for every wave-28/29 card: it records
`SLICE-PERMISSIONS-WRITE-LEND`'s widenings on the `Q051`–`Q057` and
`Q077`–`Q079` rows, `REGION-CREDIT-JOINS`' second and `REGION-JOINS-DUTIES`'
third widening of `Q302`, and `CT47-COPY-CAPTURE`'s widening of `Q600`. Of
those, only the `Q600` row in `AGENTS.md` had been updated by its card. The
other three rows now carry their widenings:

- **`Q051`–`Q057`:** the write lend — `Q051`–`Q055` widened per the card
  report's own per-code table (window shapes; the loan value kind; the lend's
  range; `Q054` from the four listed refusals to *any refusal of the
  lend-extended cycle-3 checker*; a loan at a scalar/index/rendering
  position), with `Q056` once more explicitly **not** widened.
- **`Q068`–`Q073`, `Q075`–`Q079`:** `Q077`'s one further widening — a
  `lend(…)` in a composed string/buffer entry refuses by name
  (`SLICE-PERMISSIONS-COMPOSED-LEND`).
- **`Q300`, `Q302`–`Q309`:** `Q302`'s second and third composed-route
  widenings named with their cards, pointing at constraint 15m and §L21.

Each addition points at the constraint (15h/15m) that carries the rules, so
the row stays a summary rather than a second statement of them.

### 3.2 `AGENTS.md` — constraint 14 and the Q-space note

Constraint 14's per-card census ledger stopped at wave 27. Added, each read
from its card report rather than assumed: `REGION-CREDIT-JOINS` and
`REGION-JOINS-DUTIES` (+0, vacuous by construction — every word of the
composed form already reserved and the shape previously refused);
`SLICE-PERMISSIONS-WRITE-LEND` (+0 words, one **contextual** clause word
`lend`, with `let lend = 5; print dec lend;` still a program, pinned; the read
card's census had already recorded the word free and reserved it for this
card); and the five wave-28/29 cards that do not engage the constraint
(`EXAMPLES-AGGREGATES`, `CT47-COPY-CAPTURE`, `CT22-WIRE`, `CT01-PROOF-LEG`,
`W29-CORPUS-CONTRACTS`). The pointer list gains the three census reports,
which constraints 15h and 15m already cited.

The Q-space note's two separate wave narrations ("Wave 26 ran the consequence
three times… Wave 27 ran it four more times…") merged into one run covering
waves 26–29: **eleven uses on eight bands**, every card and every code kept,
plus wave 28's one use (`REGION-CREDIT-JOINS`, `Q302`) and wave 29's three
(`CT47-COPY-CAPTURE` `Q600`; `REGION-JOINS-DUTIES` `Q302`;
`SLICE-PERMISSIONS-WRITE-LEND` `Q051`–`Q055`, `Q077`). The
`GATEWAY-C-LOCATION` non-instance sentence is unchanged.

### 3.3 The standing-constraint inventory (the load-bearing check)

**44 entries before, 44 after. Zero dropped, zero renumbered.** Verified
mechanically by `measure/inventory.py` (output archived at
`measure/constraint_inventory.txt`, worktree-private per the hygiene rule),
which extracts the numbered entries, the hard invariants, every
backticked evidence-pointer slug, every backticked residual name and every
`Q…`/`QP…` code from `git show 30aee0b2:` and from the working tree and diffs
the label sets:

```
standing constraints: 44 before, 44 after   lost: none   gained: none
  identical bodies: 41      reworded: ['14', '23', '25']
hard invariants:      6 before, 6 after   lost: none   gained: none   identical: 6
pointer slugs AGENTS.md: 102 before, 102 after   lost: none   gained: none
residual names AGENTS.md: 70 before, 70 after   lost: none   gained: none
residual names CLAUDE.md: 11 before, 12 after   gained: ['WRITE-LEND-WINDOWS-RUN']
diagnostics codes:    112 before, 112 after   lost: none   gained: none
RESULT: nothing dropped
```

(The 44 are DOCS-COMPACT-3's 39 plus the five waves 27–29 added: 12a, 15j,
15k, 15l, 15m.) The three reworded bodies, and why each rewording preserves
its content:

| # | Subject | Disposition |
|---|---|---|
| **14** | keyword census ledger | **Extended, not compressed**: waves 28–29 appended per §3.2. Every existing count, keyword, census figure, rejected candidate (`stream`, `socket`, `record`), "do not" rule and evidence pointer is byte-identical; the only edit inside existing text is the sentence joining the vacuous-census cards |
| **23** | CT mapping vs conformance | The duplicated numerators "sanitized-native is 5 of 36 and proof 0 of 36" — wave-26 numbers presented as current — replaced by "the sanitized-native and proof counts are the ledger row's (7 of 42 and 1 of 42 at the wave-29 close — read them there, not here)", plus the ledger's own "a grown proof numerator is evidence about one case's artifact". The rule half (a stage may read `executed` only where a named workspace test runs it on the case's own artifact; retirement/withdrawal machinery) is untouched |
| **25** | matrix generator ownership | "beside a live 73" → "beside a live corpus (74 at wave 29)". Nothing else moved |

**Constraints 15h and 15m — the two superseded-and-amended entries the brief
named — needed no further reconciliation.** Searched all three files for the
superseded premises: the only occurrences of "no write lend" and "duty-free"
are *inside* the amendments themselves, correctly phrased as superseded
history ("the old 'no write lend' sentence's own residual";
"a duty-free region blanks completely", which is a statement about a region
carrying no duties, not the old premise). Neither amendment was weakened;
neither was edited at all.

### 3.4 `CLAUDE.md` and `README.md`

`CLAUDE.md`: the cadence rule records this card and the next due date
(wave 32); the Windows platform records gain `WRITE-LEND-WINDOWS-RUN` beside
its wave-27 sibling `SLICE-PERMISSIONS-WINDOWS-RUN`, on the same footing and
with the same no-claim scope, read from the write-lend card report §13.
Nothing else moved.

`README.md`, refreshed as a human summary in the ledger vocabulary:

- `std.buffer`'s row gains the **record** types (`type Name { field: i32, … }`)
  and the range **write lend** with its disjointness rule; `std.string`'s row
  gains the ranged `borrow_utf8`.
- The source-level prose gains the quantitative **error region** (erasing to
  its byte-identical straight-line twin; alone and composed onto the epoch
  task route with real duties discharged after every join) and the declared
  error arm over epoch credits with its conservation equation.
- The gateway bullet gains one clause: diagnostics name the original
  pre-preprocessing `file:line` (wave 27's presentation fact).
- The status numbers were already current (42/50, 7 sanitized-native, 1 proof,
  the CT01 sentence) — the wave-29 close had refreshed them; re-verified, not
  re-written.

Every `.md` link and backticked doc path in the three files resolves to a real
file (checked mechanically; the only non-resolving string is the intentional
`docs/reports/<card>/README.md` placeholder).

## 4. Drift found and fixed (five items)

Every headline number was re-read from its source. The wave-29 close's ledger
is right everywhere this card looked; the five drifts are all in text the
close does not own:

| # | Drift | Source re-read | Fix |
|---|---|---|---|
| 1 | Repository map still said contract corpus **73 modules** | `find contract/qcore -name '*.qcore' \| wc -l` → **74**; ledger row already 74 | map row → 74 |
| 2 | Constraint 25 still said "beside a live 73" | same | → "a live corpus (74 at wave 29)" |
| 3 | Constraint 23 carried "sanitized-native is 5 of 36 and proof 0 of 36" as current | `ct_manifest.json` tally → 42 mapped / 8 unmapped, native_sanitized 7, proof 1 | numerators now cited from the ledger row (§3.3) |
| 4 | Diagnostics rows `Q051`–`Q057` / `Q077` / `Q302` behind §L21's wave-28/29 widenings | `docs/QCORE_SPEC_r0_3.md` §L21 rows 988/995/1001; card reports | rows carry the widenings (§3.1) |
| 5 | Constraint 14's ledger and the Q-space note stopped at wave 27; `CLAUDE.md` lacked `WRITE-LEND-WINDOWS-RUN` | card reports (census sections; write-lend §13) | appended (§3.2, §3.4) |

Numbers re-read and found **in agreement** (no drift):

| Number | Source re-read | Result |
|---|---|---|
| Contract corpus 74; resource-v1 39 → 41; manifest-selected 18 → 19 | corpus `find`; `w29-corpus-contracts` README | **74** ✓ |
| Lean audits 118 | `find proof -name 'Audit*.lean' \| wc -l` | **118** ✓ |
| Workspace 4,445 · provider 122 · Python 193 · probes 0/0 · fmt/clippy clean | `docs/reports/wave29-close/gate.json` | ✓ |
| CT 42/50 mapped, 8 unmapped; sanitized-native 7; proof 1 | `docs/reevaluation/ct_manifest.json` tally | **42/8/7/1** ✓ |
| leak 266 · `unknown_alias` 4,344 · `unsupported` 49,760; 13 leaves itemized; baseline `w29-corpus-contracts/artifacts/` | `docs/reports/w29-corpus-contracts/README.md` §0/§5 | ✓ |
| Named follow-ons 39 | `acceptance_matrix.json` `named_followups` | **39** ✓ |
| Nine crates, ten workspace members, six-member providers workspace | `ls crates/`, `Cargo.toml` members, `providers/Cargo.toml` | ✓ |
| Twelve example families | `ls -d examples/*/` | **12** ✓ |
| Cycle records `CYCLE_2` … `CYCLE_20` | `ls docs/cycles/` | ✓ |

The matrix was **not** regenerated and no number disagreed with it.

## 5. What was deliberately not done

- **Constraints 15e–15i beyond 15h's cross-reference check.** DOCS-COMPACT-3
  §3.4's standing reason holds: they are rule text whose only other home is
  their card reports. The brief's condition for touching them — a later card
  superseding one — was met for 15h and 15m, and both were found already
  reconciled by their amending cards (§3.3); no edit was needed and none was
  made.
- **`EXAMPLES-STYLE-SLICE-FANOUT`** remains an open user decision, flagged in
  `examples/README.md`, untouched, undecided.
- **No frozen archive, spec line, manifest, matrix or code file** was edited
  (constraint 21; the card brief). The two carried DOCS-COMPACT residuals
  from the third pass — `DOCS-COMPACT-2-W4-DESIGN-POINTER` and
  `DOCS-COMPACT-3-WORKTREE-SUBMODULE` — were both discharged before this card
  (`de8273ed`, `e552e59c`) and needed nothing here.

## 6. Gate

Run at the card tip on Linux with `$HOME/.elan/bin` on `PATH`. The gate
command is `cargo test --workspace --no-fail-fast` (`GATE-NO-FAIL-FAST`), and
the Lean step is read from its **text** line per that script's own behaviour.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,445 passed, 0 failed, 8 ignored**, exit 0 |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **`Passed 118 Lean audits`** |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** |

No provider rebuild: `providers/package.py` inputs untouched, `qcore-schemas`
stays 14.0.0, constraint 18 not engaged; the provider row carries at 122.

## 7. Follow-ons

- **`DOCS-COMPACT-4-TABLE-SYNC`:** three of the four wave-28/29 condition
  widenings reached §L21 but not `AGENTS.md`'s summary table until this pass
  (only `CT47-COPY-CAPTURE` updated its row). The per-card habit the earlier
  waves had — state the widening in §L21 *and* the summary row — slipped for
  three cards in a row; worth a sentence in the next wave's card template, not
  a rule change.
- The next pass (due around wave 32) inherits DOCS-COMPACT-3's floor
  observation unchanged, with this pass as further evidence: the compaction
  method now yields net growth whenever the closes have under-synchronised,
  and its real value is the re-read (five drifts) and the inventory (zero
  dropped).
