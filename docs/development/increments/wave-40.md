# Wave 40 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Compaction checkpoints](wave-40-compaction-checkpoints.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`DOCS-COMPACT-8` (2026-09-19).** The eighth firing of the three-file
compaction cadence. Measured first at `6dc484ad`, then **re-derived** — not
rebased — at `d6a57f66` after `STABILITY-REEVAL-4` merged, which is how the
wave-37 pass earned its trust. Every headline number was read **from its source
and never from the file being compacted**: the corpus row (2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only; 213 / 84 / 4,537 of 4,834
functions; leak 272, `unknown_alias` 4,309, `unsupported` 49,753,
`summary_conflict` 1, `double_release` 0) from
`docs/reports/w37-corpus-contracts-4/artifacts/diagnostics.json.gz`; the CT row
(47/50 mapped, unmapped CT10/CT24/CT41, sanitized-native 10, proof 4 — CT01,
CT03, CT07, CT37) from `docs/reevaluation/ct_manifest.json`; the contract corpus
(88 modules, 59 resource-v1 declarations, 33 manifest-selected) from the tree and
`contract/cycle19-resource-contracts.json`; the crate, workspace, version, Lean
and example-family counts from their own files; and the merged-gate row from
**this card's own detached runs**, which is the second arm of its own wave-37
finding `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` — the row names the run that
produced it rather than borrowing a close's. **Across the re-derivation exactly
one figure moved**: `named_followups` 234 → **258**, the sibling's +23 −1 plus
this card's two, re-derived from the committed file rather than patched. The
standing-constraint inventory is **56 numbered entries before and after** (6
hard invariants, 50 constraints), order identical, **53 of 56 byte-identical**;
the sibling moved no numbered entry, verified by hashing them at both bases.
**Nine corrections**, each measured against the tree rather than inherited.
Three came from re-measuring the merged-gate row instead of carrying it: its
summary arithmetic (240 summaries and four doubled doc-test sections, where this
card and `STABILITY-REEVAL-4` independently measure **238** and **two**), the
**cause** of that 240 — a prefix match on `test result` without its colon, which
also catches two ordinary test names beginning with `result_`, so one loose
regex produced both wrong figures — and the row's claim that all five of
constraint 16's named rows live in this workspace, where **four** do. Two are
the discharge of the sibling's `LEDGER-TSAN-BOUNDARY-SENTENCES-STALE` in the
prose this card owns: constraint 15h presented as "measured, not claimed" that
ThreadSanitizer cannot run any task-route program, citing
`TASK-ROUTE-TSAN-SEGV`, **retired in wave 37**, while
`compiler/tests/task_route_tsan.rs` runs six route-emitted subjects and
`TSAN-ROWS-2` runs that constraint's **own flagship** clean under two forced
schedules; and constraint 15o said no TSan row exists anywhere in the
repository, now replaced by the sharper measured position — CT46's case is a
subject and is clean **because it is serial by construction**, which is that
constraint's own theorem observed. `README.md` carried the same two defects and
is corrected with them, together with its CT figures (46 of 50, 3 proof stages)
and its description of CT07's leg, which is bound through the scoped-task ledger
and not the F02 relation the other three use. The last two: `AGENTS.md`'s
repository map still called the contract corpus 76 modules, stale since wave 31
through two compaction passes, and its six-module gateway inventory omitted the
four landed since then that constraints 5b, 8 and 10b rest on. The Q-space tally
(**eighteen** condition extensions on **ten** bands) was re-derived from the
enumeration plus the §L21 rows and **holds** — `AGGREGATE-OWNED-FIELD` reaches
the `Q100`/`Q103` row by vocabulary alone, with that condition not widened, so
it is not an eleventh band. The compaction fell on the blocks that had become
chronicles: constraint 14's per-card census ledger (19,638 → 6,074 bytes, every
rejected spelling, contextual rule and name-resolution exclusion kept and now
mechanically checkable against `is_reserved_source_word`'s 112 words), the
Q-space note's eighteen-instance enumeration, five ledger blocks whose own
sentences assign their narrative to the increment records, and `CLAUDE.md`'s
Windows residual family, which stated one shape seven times. Eighteen
`SCREAMING-KEBAB` names stopped appearing in the three files and **none is
lost** — each checked against the files, `docs/reports/`, `named_followups` and
the checkpoint page, and not one is a residual with an open discharge condition.
Two names are registered, both about figures nothing derives:
`GATE-ROW-SUMMARY-COUNT-UNCHECKED` and `AGENTS-MAP-FIGURES-UNDERIVED`. Nine
blocks, **47,344 bytes**, moved verbatim into
[the wave-40 compaction checkpoints](wave-40-compaction-checkpoints.md) — a
separate page rather than this one, because `DOCS-COMPACT-7`'s inline
checkpoints took `wave-37.md` to 64,613 bytes and that card flagged the split as
the close's to make. Report: `docs/reports/docs-compact-8/README.md`.

**`STABILITY-REEVAL-4` (2026-09-19).** The cadence's fourth firing, over waves
38 and 39, at base `6dc484ad`. An audit: no Rust, no Lean, no gate weakened, no
frozen archive edited, no verdict rule changed, no semantic change — **23
residuals registered and one retired**. Firing #3's fourteen tally **8
discharged / 2 partial / 4 open**, and for the first time in the cadence **no
name went quiet**: all seven that left the matrix carry a recorded discharge or
retirement whose mechanism was verified present on disk. The pattern finding is
a **correction of firing #3's own**: #3 measured 0 of 18 mechanism-needing
residuals discharged and offered the class as the explanation; this firing
measures mechanism **5 of 11** and sentence **3 of 3**, and every discharged
name plus both partials appears in the wave-38 close's item table or in its
§14 lines written into the wave-39 briefs, while the three names reaching **no
close document at all** are all open — two of them with a sentence-satisfiable
arm nobody took. The discriminator is **brief-reach**, not the class of work.
The one retirement is `W36-W37-MUTATION-HARNESSES-UNSHIPPED`, measured
discharged in full (four dated appended restatements plus `CLAUDE.md`'s
convention line) and unread by any close for two waves — the mirror image of
the three that are registered and unread. **Vacuous greenness**, measured by
reachability rather than read off diffs: `OwnershipOp::Copy` is constructed at
four places, all unit tests, so `AGGREGATE-OWNED-FIELD`'s non-copyability rule
and the older plain-owner rule alike are reachable from **no route**
(`OWNERSHIP-COPY-OP-UNPRODUCED`); that card's three named boundaries are
**absence, not refusal** — two collapse into a message naming a *different*
residual and one is unreachable by dispatch order, so zero refuse by their own
name (`OWNED-FIELD-BOUNDARIES-ARE-ABSENCE`); `QCore.LoopWindows` is a
**transcription** linked to `std_buffer::last_uses` by a doc comment, its one
`LOOP-V1` theorem instantiated at an alphabet with **no lend constructor**
(`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`); `std_string::view_uses_at_any_depth`
returns the empty vector **for every input**, while constraint 15h names it as
the composed route's window function (`COMPOSED-VIEW-WINDOW-INERT`); and
`custody_names_at_any_depth`'s six aggregate arms are unreachable behind two
refusals whose lifting would make an untested no-op load-bearing in the
**admitting** direction (`WINDOW-FIELD-ARM-UNREACHED`). CT42's two fallback
arms are now `Err` and **driven only through a published bypass**, unreachable
on-route because two hand-written walks agree and nothing tests the agreement;
`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN` is honestly pinned in both directions
while `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY` has **no mechanism** and is
guarded only by coincidence; CT07's proof leg is non-vacuous under wave 38's
liveness check, has a discrimination control and **no re-transcription
control** — the whole four-leg family pinning a Lean digest as a bare Rust
literal nothing compares, which also shows an existing entry
(`LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`) prescribing a mechanism that does not
exist. **Semantic edits without a card:** the three already registered are
verdicted (`GATE-SUMMARY-PARSER-OK-ONLY` accurate, `MUTATION-HARNESS-STALE-ARTIFACTS`
sufficient, `INTEGRATION-GATE-CLANG-CLAIM-BORROWED` **under-scoped** — the
generator's `clang` key is satisfied by any sibling text file, with the exit
arm optional), and four were not registered: three wave-39 merges hand-edited
the **Lean count inside a row labelled with another card's run**
(`MERGED-GATE-ROW-LEAN-HAND-EDIT`, `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`'s exact
shape inside the wave that made the key required, repaired by the close at
`24505b83`); `--uncarded` **excludes every merge by construction**, which is
where that prose lives; all five cited integration tips are **non-ancestors of
`main`**, `AGENTS.md`'s own row among them, and two merges published no
integration gate; and the ledger's "240 result summaries … four of those
sections print two summaries each" does not reproduce — measured **236 targets,
238 strict, 240 loose, two doubled sections**, the loose-grep artifact firing
#3 had already named. Step 4 found **two standing-constraint sentences that are
simply false**: `AGENTS.md:1252-1253` presents as "measured, not claimed" that
ThreadSanitizer cannot run any task-route program, citing a residual retired in
wave 37, while `task_route_tsan.rs` runs six route-emitted subjects today; and
`:1635` still says no TSan row exists anywhere
(`LEDGER-TSAN-BOUNDARY-SENTENCES-STALE`). Neither was repaired here —
`DOCS-COMPACT-8` owns that prose — and `CT-COMPOSED-NATIVE-ROWS` was measured
discharged and **deliberately not retired**, recorded as a postponement with
its ground. `named_followups` **234 → 256** (+23 −1) with `recorded_by` on
every entry added. Gate detached at `bd5eb8fd` and polled by a bounded
foreground loop: **4,727 / 0 / 10**, `test result: FAILED` 0 times,
reproducing the wave-39 close exactly; fmt and clippy clean; both probes 0;
**141** Lean audits; Python **303** OK before and after the regeneration; Clang
**21.1.8 recorded by this card in its own worktree**, not borrowed; all five
constraint-16 rows green plus the unnamed seventh. Nothing promoted: contract
corpus 88, CT 47/50 with sanitized-native 10 and proof 4, raw units
unpromoted, full language conformance **false**.
Report: `docs/reports/stability-reeval-4/README.md`.
