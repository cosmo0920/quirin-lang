# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`W32-RECONCILE-SWEEP` — the integrator's own reconciliation lists, measured and
made mechanical (2026-09-14, mid-wave 32; **no arbiter run, no corpus row
touched**). A defect in the *integrator's* process had been reported three
waves running — `DOCS-COMPACT-4-TABLE-SYNC`, then
`DOCS-COMPACT-5-RECONCILIATION-LIST`, whose concrete casualty was
`WRITE-LEND-TASKS` §12's proposed **constraint 15h append** standing unlanded
for a whole wave beside a shipped, Lean-proved task-route write lease. This
card swept the **20** card reports of waves 29–32-so-far into **77
integrator-addressed items** and gave each a verdict **measured against the
tree** — the target file grepped for the proposed text or its effect — never
inferred from the report's own claim: **37 applied, 38 noted, 1 superseded,
1 deferred, 0 declined**, recorded in
`docs/reports/w32-reconcile-sweep/reconciliation-waves-29-31.json` with seven
`no_section` answers, and the enumeration independently reproduced by the new
tool's own `--since 5d8b2ff1` selector. The three-file surface measured
**clean** — because `DOCS-COMPACT-5` had swept it days earlier — which is
itself the finding: a sweep run after a compaction pass measures the compaction
pass's reach, not the close's, and what remained lay **outside** those files.
Three unapplied items were found and repaired, all documentation-side and
unambiguous. `dimensions.named_followups` stood at **39 against a list of 40**:
a *derived* field, hand-outrun when commit `1c7d8e45` added the fortieth entry
after the wave-31 close regeneration, and no test compares it
(`RECONCILE-SWEEP-DIMENSIONS-UNPINNED`); the whole-file JSON round trip was
measured byte-identical first, so the diff is four lines. `ct_corpus.rs`'s doc
comment read *"proof is 2 of 43"* against a `PROOF_GATED_CASES` table and a
manifest that both read **three** — **the same sentence stale a second time**,
`CT03-PROOF-ALPHABET` having merged after the very lane that had just
refreshed it from its previous staleness, so the repair **records both
stalenesses and why prose beside a pinned table drifts** rather than only
fixing the number. And `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE` is discharged
on its own terms by **scoping** the increment index's clause to the record
files that carry a migrated section (measured: `wave-32.md` carries none,
`wave-29`–`31` all do). The sharpest finding is that
**`DOCS-COMPACT-4-TABLE-SYNC` and `DOCS-COMPACT-5-RECONCILIATION-LIST` were
tracked nowhere** — not `named_followups`, not `AGENTS.md`, not `CLAUDE.md`:
the follow-on saying the close loses follow-ons was itself lost by the close.
`DOCS-COMPACT-O` now carries both passes and four residual names. The mechanism
replaces advice with a check: `scripts/reconciliation_sweep.py` (27 tests;
Python suite 200 → **227**) splits each card's integrator-addressed section
into individually addressable items with stable ids and **refuses** a
wave-close record lacking a verdict for any of them, with a `no_section` map
checked in both directions so it cannot wave a card away, and `--residuals`
listing residual names no tracker mentions — it found its own occasioning pair.
Three design decisions were each made against a measured failure of the naive
version: prose blocks are items (`CT22-WIRE` put the owed half in a paragraph
*after* its file list, so a list-only read yields nine items none of which is
the one thing owed), fenced records attach to the block they follow, and a
fifth verdict `noted` keeps `applied` from stretching over 38 preambles and
suggested readings. **The tool decides nothing:** a green `--check` means every
item carries a recorded *human* verdict, never that any item was applied —
stated in the module, in the CLI's own success line, and pinned by a test.
`CLAUDE.md` gains the close step naming its two commands and its evidence file
beside each close's `gate.json`; `AGENTS.md` gains one entry-point pointer.
Everything here is **observed, not proved** (`STABILITY-REEVAL`): a verdict is
one reading of one tree at one commit, and "complete" is not "correct". Four
items need an integrator decision and are listed with options rather than
guessed — the matrix's four-wave-stale `updated` note, `STD-MODULES-O` still
listing a follow-on discharged three waves ago, whether the sync rule binds a
tooling commit that publishes under `docs/reports/`, and CT10's reason row (the
one `deferred`). Gate at the card tip, under **disclosed concurrent load** from
two sibling worktrees running their own binaries (constraint 16's regime):
**4,510 passed / 1 failed / 8 ignored across 223 binaries** — the one failure
being **constraint 16's own `RLIMIT_NPROC` row**
(`a_declared_process_limit_stops_the_fixture_from_forking`, `Io(Broken pipe)`
where it expects `Terminated`), **re-verified in isolation on a quiet machine
at 12 passed / 0 failed / exit 0**, both facts recorded and nothing weakened to
obtain either, which puts the suite at the wave-31 close's **4,511**;
fmt/clippy clean, both probes exit 0, `Passed 125 Lean audits`, **227 Python
tests OK**. `GATE-NO-FAIL-FAST` earned itself here: under cargo's default the
run would have stopped at `qcore-verifier-host` and the 223-binary total would
have been a lower bound rather than a measurement. Nothing is promoted, no
count moved, no frozen archive was edited, no
`Q` code allocated, and full language conformance stays **false**.
`docs/reports/w32-reconcile-sweep/README.md`.

`DOCS-COMPACT-5` — fifth recurring compaction of `AGENTS.md`, `CLAUDE.md` and
`README.md` (2026-09-13, after wave 31; **no arbiter run, no corpus row
touched**). The standing-constraint inventory is **45 in, 45 out, zero dropped
and zero renumbered** (DOCS-COMPACT-4's 44 plus `C-GATEWAY-WAVE`'s 10b),
verified mechanically along with the six hard invariants, every evidence-pointer
slug, every residual name and every diagnostic code; four bodies were reworded
and each word-diffed — 14 (waves 30–31 appended to the census ledger, the
"does not engage" sentences merged), 15h (a **pure insert**), 23 and 25 (stale
numerators only). Every headline number was re-read from its source and
**eight drifts** fixed: contract corpus 74 → **76**, the gateway's "four"
load-bearing modules → **six** (`legacy_neutral.rs` and
`legacy_joined_provenance.rs`, already named by constraints 4 and 10b), named
follow-ons 39 → **40**, constraint 23's numerator pointer → 7 of 43 and 3 of 43
at the wave-31 close, constraint 25's live corpus → 76, constraint 14's census
ledger and the Q-space note both stopped at wave 29 (now waves 26–31, thirteen
uses on nine bands, three non-instances named), and three diagnostics rows
behind §L21's wave-28/30/31 widenings (`Q020`/`Q021`, `Q302`, `Q600`). Those
last four share **one root cause**, recorded as
`DOCS-COMPACT-5-RECONCILIATION-LIST`: `WRITE-LEND-TASKS`' explicit integrator
reconciliation list (§12) was applied for the headline numbers only, so its
proposed **constraint 15h append** — the range write lease crossing the spawn
boundary, with `ownership::axes_overlap` called rather than re-implemented, the
three asymmetries, and `WRITE-LEND-TASKS-EXERCISE` — had no home in the three
files at all; it is now landed as an insert that changes not one existing word.
The restructure's seams were checked mechanically: all **149** path references
and every `#fragment` across the three files and `DEVELOPMENT.md` resolve, the
sync rule and the increment index agree on "one paragraph, newest first, in the
current record file the index names", the split's one duplicated instruction
(update the index / do not append to the root table of contents) was merged back
into the sync rule, and `README.md`'s pointer for the verdict vocabulary and the
hard invariants was repointed from `DEVELOPMENT.md` — now a table of contents
that carries neither — to `AGENTS.md`. The new `STABILITY-REEVAL` step produced
three findings: the CT ledger row now states CT03's **observed** side beside its
proved one (`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, the arm no test drives,
bounded by direction alone), 15h's append carries
`WRITE-LEND-TASKS-EXERCISE` in the card's own words so it cannot read as a
two-threads-mutating claim, and `DOCS-COMPACT-5-EXAMPLES-HOST-GATE` is
**flagged and deliberately not reworded** — measured, `examples_suite.rs` is
`#![cfg(any(target_os = "linux", target_os = "macos"))]`, so the unconditional
"built and run under the sanitizers" sentences are a scope omission already
covered by the `*-WINDOWS-RUN` residual family. `CLAUDE.md` gains
`WRITE-LEND-TASKS-WINDOWS-RUN` and records the next pass as due around wave 34.
Byte deltas: `AGENTS.md` +5,379 (+5.2%), `CLAUDE.md` +419 (+3.9%), `README.md`
+259 (+2.2%) — growth again, and for DOCS-COMPACT-4's recorded reason. Gate at
the card tip: **4,511 workspace tests (0 failed, 8 ignored, 223 binaries,
`--no-fail-fast`), no constraint-16 row tripped**, fmt/clippy clean, both probes
exit 0, `Passed 125 Lean audits`, 200 Python tests OK. Nothing is promoted, no
count moved, and full language conformance stays **false**.
`docs/reports/docs-compact-5/README.md`.

