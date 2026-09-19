# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status are routed by [AGENTS.md](../../../AGENTS.md) to the
> [rule reference](../../agents/README.md) and [status page](../../STATUS.md);
> each card's report owns its evidence.

**`TSAN-FAILURE-ARMS` (2026-09-19).** The ThreadSanitizer forcing harness gains
a **release trigger the creator's join does not own**, and the task programs'
**failure arms** get rows for the first time — the order `STABILITY-REEVAL-4`
measured, in which the harness breaks before the arms can be reached. Wave 39's
gate opened only at `QCORE_TASK_JOIN`, so a mutation deleting the join deleted
the release with it and hid every race (`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`,
0 of 20 forced against 20 of 20 unforced).
`tests/support/thread_sanitizer.rs`'s `SCHEDULE_FORCING` now also opens its gate
**inside the create hook**, when the row's own per-program cohort count
(`QCORE_FORCED_COHORT`) of workers stands parked, before the creator reaches any
join; the join trigger is **retained beside it**, and `0` keeps wave 39's
join-only behaviour reachable, which is how that card's own 0-of-20 survives as
the control arm of `a_join_deleted_is_caught_by_the_cohort_trigger` rather than
as a memory. Re-measured at 20 runs per cell: the same gate-closed mutation is
now reported **20 of 20** under both forced schedules, and every wave-39 cell
reproduces exactly at `cohort 0`, including the `halt_on_error=0` race counts
16 / 6 / 6. A **cost is published rather than elided**
(`TSAN-COHORT-TRIGGER-CATCH-COST`): at `cohort 2` the *join-edge-removal*
mutation loses catches — 0/20 on both subjects under `forced-together`, 18/20 on
`shared_readers` under `forced-reverse` — because the workers then finish long
before the creator's post-join access and ThreadSanitizer reports a thread leak
instead, so the second trigger is an addition for the arms that need it and the
two wave-39 mutation rows stay at wave 39's trigger. A third `-include` header
drives each subject onto a real failure arm — CT07's own refusing second
`QCORE_TASK_CREATE`, and a NULL-returning `QCORE_BUFFER_MALLOC` — and the twelve
cells (write-lend flagship and CT07's `case.qcore` × two arms × three schedules)
are each **20/20 clean, 20/20 arm-named and 20/20 byte-identical to the
reference execution scripted for the same outcomes**, with the arm that ran
asserted **from the program's own stdout** (`QCORE_TASK_CREATE_FAILED 0` with the
first lease present and the second absent; `QCORE_BUFFER_ALLOC_FAILED 0` with no
lease at all) rather than from the harness's intent, and a control row showing
the same subject taking the success arm when nothing is injected. The
allocation-failure arm creates **no thread** on either subject, so its clean
verdict is about a single-threaded recovery path — said in those words and
registered (`TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED`). The forcing header's
invisibility to ThreadSanitizer is now **enforced rather than documented**,
discharging `TSAN-HEADER-INVISIBILITY-UNENFORCED` on **both** branches of its
condition: a textual tripwire over the shipped header (`no_sanitize("thread")`
on every `qcore_forced_` function, no pthread or C11 synchronization primitive
named) and the `hook_visible_to_tsan` reproduction promoted to a workspace test
that is **not** `#[ignore]`d — it rebuilds the visible-synchronization header
from the shipped text, every substitution asserting its anchor appears exactly
once, and measures the overlap mutation going unreported **0 of 20 under both
forced schedules** where the shipped header reports it 20 of 20, which is
broader than wave 39 recorded. Twelve mutations, **eight of them moving the rule
rather than the test**, all `RED-AS-REQUIRED`, baseline green before and after,
every file restored by rewrite and checked sha256-equal. **No emitted byte
moves**: the diff over every emitting source against the base is empty and the
whole card's `git diff --name-only` is two test files plus its report; the seam
is `-include`/`-D` only, and the emitted C still carries both hook defaults.
Nothing is promoted — no corpus count, unit status, function status, CT stage,
contract count or verdict moves; raw units stay unpromoted; full language
conformance stays **false**. No `Q` code is allocated and no band's condition is
widened (nothing here is a checker rule); constraint 14 is **+0 words and +0
tokens**, run not assumed (`is_reserved_source_word` holds 113 literals, all
distinct, and `crates/qcore-frontend/src/frontend.rs` is unchanged); no Lean
moves, because no semantic surface does. **Linux only, and no Windows or Apple
claim is made** (`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` stands), and every row is
still a pthread-bound build (`TASK-ROUTE-TSAN-PTHREAD-BINDING` stands).
Retirements, each verified registered by id and met word for word:
`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`, `TSAN-FORCED-SCHEDULE-FAILURE-ARMS`,
`TSAN-HEADER-INVISIBILITY-UNENFORCED`. `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`
is **amended, not retired** — it is a rule about any future forcing harness and
the behavioural half is one mutation rather than a quantifier over all visible
designs. `TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`' **named remaining half is met**
(the failure arms) and the residual is nonetheless **kept**, its disposition
becoming the first branch of its own discharge — stated permanently as a
measured boundary — because what remains of it is the permanent statement that a
clean ThreadSanitizer run is evidence about the paths that executed and not a
proof of race freedom. Four residuals registered
(`TSAN-COHORT-TRIGGER-CATCH-COST`, `TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED`,
`TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND`,
`TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER`), so the tracker moves
**257 → 258**. Evidence: `docs/reports/tsan-failure-arms/README.md`.

**`CT-PROOF-LEG-3` (2026-09-19).** The fifth executed `proof` row, and the
measurement that the cheap route to a sixth is spent. The card first **re-ran
the survey rather than inheriting it**: every one of the 47 mapped cases' own
fixtures compiled through `compile_files`, every task-route module executed by
`execute_task_module` and handed to `AdmittedCausalOrder::derive`, on the
default script and on five failure scripts, with the raw output shipped
(`docs/reports/ct-proof-leg-3/probes/survey.tsv`). Eleven mapped cases derive on
the default script; four were already proved; CT12's and CT14's runs are **event
for event CT01's**; CT13, CT15, CT16, CT17 and CT20 derive but their default runs
do not exhibit their scenarios, whose defining arms are refused as
`UnattributableEvent` (creation failure) or carry an `Error` outcome the Lean
alphabet has no constructor for (allocation failure, and CT16's default run).
`STABILITY-REEVAL-4`'s claim that **CT50 is the only remaining F02-shaped
candidate** therefore holds, verified rather than inherited — and this card spent
it, so the set is now empty (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`, registered;
`CT-PROOF-FOURTH-CASE-UNTRACKED` retired). CT50's leg is
`proof/QCore/Ct50LifetimeIdentity.lean` with `proof/AuditCt50LifetimeIdentity.lean`
(Lean audits **141 → 142**) and the host-unconditional
`compiler/tests/ct50_proof_leg.rs`; proof **4 of 47 → 5 of 47**, which is
evidence about one case's artifact and never conformance (constraint 23). Two
things about it are worth the record. Its **closure is the degenerate case and
is proved, not assumed**: the run has one actor, so a relation demanding
agreement on every actor's projection over the same multiset admits nothing but
the run itself (`an_admitted_rendering_is_the_reference_run`), which the Rust
leg re-measures on the artifact by enumerating `admitted_linearizations` and
requiring the singleton. And its **scope is the capability half of the
scenario**: CT50 is named after a numeric *address* reuse that an alphabet
carrying generations cannot see, so what is proved is disjoint generation
lifetimes and that no fact of the run uses the old generation, while address
identity stays native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`,
registered). Beside the leg, `compiler/tests/proof_leg_digests.rs` closes
`PROOF-LEG-DIGESTS-UNCROSSCHECKED` (retired): every `theorem ct<N>…digest` in
`proof/QCore/` is compared with every corresponding `compiler/tests/` constant,
total or reject in both directions, and the sweep found **six** Rust pins where
the residual's own inventory named five — CT01's is written twice. Its own
premise is registered (`PROOF-LEG-DIGEST-CROSSCHECK-TEXTUAL`). **The non-vacuity
harness found two defects in this card's own claims and both were repaired
rather than reworded**: the read-by-a-second-actor mutant fails *two* clauses so
it isolates neither (replaced by a result-claimed-by-a-second-actor mutant that
does, with the old one kept and named as non-isolating), and the payload clause
is *implied* by the read-window chain together with the read count — now the Lean
theorem `the_payload_clause_is_implied_by_the_read_window_and_the_read_count`
rather than a silent weak spot, and its mutation row is expected green with that
ground. Twenty mutations, every restore sha256-equal, baseline green before and
after (`mutations.log`). Three carried entries disposed: `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`'s
discharge text repaired to name the mechanism that now exists (the entry stays
open; this card deliberately did not apply it to another card's Lean module),
and `CT-PROOF-CONTROL-ARM-TIER` and `CT-PROOF-CREATION-FAILURE-ALPHABET`
restated as measured boundaries with wave-41 measurements — the second sharpened,
because the cases it lists need **two** alphabet extensions and the entry named
one. No `Q` code, no reserved word, no lexer change, no checker or emitter
change; the diff over `crates/` and `compiler/src/` is empty.
`named_followups` **257 → 259** (+4 −2). The card was **rebased onto the documentation
restructure** (`96142e49`) before merge, so its ledger edits land in
`docs/STATUS.md` — the CT case-bound-proof row, the registered-follow-ups row
and the paragraph naming which cases bind through F02 — and **not** in the
former `AGENTS.md` ledger, which that commit replaced with a router. No
standing constraint text moved, so no `docs/agents/*.md` preserved block is
touched.
