# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`COMPOSED-WINDOW-REACH` (2026-09-19).** The wave's soundness-adjacent card,
against `STABILITY-REEVAL-4`'s four findings about the loan-window machinery.
**The headline is negative and is reported as one: no program constraint 15h
says must refuse is admitted.** Twenty-one probe programs were measured before
anything was repaired — every position a view can be read from, a bare render
and an inline-slice render inside a composed `for` body, a nested body, a plain
and an owned-field record beside a lend and inside a body, a scalar rebound
from a byte at a lend bound — and each refuses at the code and by the mechanism
constraint 15h names, or is admitted where the constraint says it should be
(`docs/reports/composed-window-reach/probes/verdicts.log`, shipped with the
harness). What is true is narrower. `COMPOSED-VIEW-WINDOW-INERT`'s *title* is
wrong and its content is right: `std_string::view_uses_at_any_depth` does not
return the empty vector for every input — it names a view at every reading
position, and those names are what the composed window is computed from — but
its **depth contribution** is empty for every input, so **a `for` contributes
nothing to any view's window**, pinned over every `StringEvent` constructor by
`the_depth_contribution_is_empty_for_every_event`. That is safe only because no
admitted composed body can read a view, and the card measured that this rests
on **three** mechanisms rather than the one the code claimed: the body's type
(a body holds `BufferEvent`s, none of which names a view), the `Q077`
vocabulary refusal for the *constructing* forms — and, newly named,
`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`, because a **bare** `print s;` is
deliberately not claimed by `StringUse::Render` and is refused instead by the
buffer route's own "names a `std.fmt` rendering" rule, which is a rendering
rule and not a window rule, and which a later card could remove without reading
this file. Measured beside it, by re-running wave 39's own M1 against each of
that card's 24 rows: **7 are anchored by the window function and 17 are not**,
and **none of the composed-route rows is** — the composed window is defence in
depth at depth and the deciding rule only at top level
(`W41-M9-composed-window-emptied` separates the two). The repair that moves
code is on the buffer side: `custody_names_at_any_depth` now classifies the
names it meets (`WindowName::Binding` / `WindowName::FieldPath`) and the four
field-path events of `AGGREGATE-OWNED-FIELD` **refuse by name** at the window
function's own new gate `window_unnameable_events` (`Q051`, a condition
extension of the band constraint 15h already names for the lend's window
shapes; no code allocated) instead of silently contributing nothing —
discharging `WINDOW-FIELD-ARM-UNREACHED` on its own second disjunct, with its
premise corrected: **two of the six arms it counted are reachable today**
(`Construct` and `Field`, measured admitted beside a lend and inside a body of
a lending entry) and are correctly empty. In Lean, `QCore.LoopWindows`'
`the_admitted_body_shapes_are_exactly_these` — the two-constructor tautology
`LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY` named — is **superseded** by
`every_admitted_body_event_is_windowed` over `LendEvent`, the window-relevant
shape of the Rust `BufferEvent`, which a mutation falsifies; and
`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION` is discharged on **two** of its three
disjuncts at once — the unrolling is instantiated at that lend-carrying
alphabet (`the_lend_alphabets_unrolling_is_loop_v1s_recursion` beside
`loop_v1s_unrolling_is_the_same_flattening`, the two sharing a right-hand
side), and the doc-comment link is replaced by a **differential test over
shared vectors**: the Rust test reads `proof/QCore/LoopWindows.lean` as text,
extracts all eight rows of its `agreement` table, rebuilds each spelling as a
`Vec<BufferEvent>` and asserts `last_uses` returns the Lean number, while the
Lean side proves `the_agreement_table_holds` — that every row's spelling is its
own program's rendering and every row's number is `lastUse`'s answer — so
neither side can be edited alone (`W41-M6` reddens the Rust row from the Lean
file; `W41-M7` fails the Lean build from the same edit). **Ten mutations, every
one non-vacuous**, every file restored sha256-equal with both baselines green
before and after. **Emission census: 535 programs, 146 admitted by both,
byte-identical, 0 differing, 0 verdicts moved in either direction** — the new
gate refuses nothing reachable from source today, which is exactly why the old
arm looked safe. Four retirements
(`COMPOSED-VIEW-WINDOW-INERT`, `WINDOW-FIELD-ARM-UNREACHED`,
`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`, `LOOPWINDOWS-ADMITTED-SHAPES-TAUTOLOGY`),
three registrations (`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`,
`WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED`, `LOOPWINDOWS-BINDING-IS-FINITE`).
No new `Q` code, no reserved word, no corpus count, unit status or function
status moved, raw units stay unpromoted and full language conformance stays
**false**. The card was rebased mid-flight onto `96142e49`'s documentation
restructure and its ledger edits re-routed: the follow-up figure to
`docs/STATUS.md`, constraint 15h's amendment to `docs/agents/buffers-records.md`
and the `Q051` widening plus the namespace tally to `docs/agents/diagnostics.md`
— in every case **outside** the `<!-- preserved:… -->` markers, because
`docs/reports/docs-structure-compression/verify.py` pins each preserved span's
`sha256` as a byte-exact reconstruction of the pre-restructure guides. The first
draft was written inside those markers, the verifier caught it, and **the pin
was not edited** (constraint 21); `scripts/build_docs_index.py --check` caught a
stale report index in the same pass and was regenerated. Evidence:
`docs/reports/composed-window-reach/`.

> instructions describe their recorded dates. Current agent rules are routed by
> [AGENTS.md](../../../AGENTS.md) to the [rule reference](../../agents/README.md)
> and headline status lives in [docs/STATUS.md](../../STATUS.md); each card's
> report owns its evidence.

**`OWNED-FIELD-ROUTE-REACH` (2026-09-19).** Wave 39's five owned-field rules
were measured for **reach** before anything was changed, by a shipped script
that writes 29 whole `.qcore` programs and hands each to the built `quirin
check` (`docs/reports/owned-field-route-reach/scripts/reach.py`, paired against
the same script run on a `git archive` of the card's base `463fa6e8` by
`reach_delta.py`). Four rules — construction moves in, the partial move's hole,
the declaration-order drop-through, the leak naming a path — were already
driven by real programs. The fifth, **non-copyability, was reached by nothing**:
`OwnershipOp::Copy` was constructed at four places in the tree, every one a unit
test (`STABILITY-REEVAL-4`'s `OWNERSHIP-COPY-OP-UNPRODUCED`), so the rule was
decided by hand-built operation lists and constraint 15k's "a v1 record is a
plain copyable value" was an assertion nothing could check in either direction.
The card gives the question a source spelling: `let NAME = NAME;` — **the copy
bind** — on `source.buffer.v1`, which builds `Copy` for every name the ownership
ledger holds a binding for and **no operation at all** for every other. Seven of
the 29 probes moved and the rest are unchanged: an owned buffer, an owning
record hollow or not, a moved binding, a dropped one and a write loan now refuse
at `Q054` with the inherited checker's own words where they were a `Q051` shape
refusal, and a scalar and a record of scalars are **admitted** — the card's one
new admission, its invariant-1 argument proved rather than asserted in
`QCore.OwnedFieldCopyReach` (a place the ledger holds no binding for owns no
resource, so an admitted copy moves no duty and is the identity on the ledger;
142 Lean audits, +1). The three boundaries `STABILITY-REEVAL-4` measured as
**absence rather than refusal** now refuse **by their own names**, each at the
code, span and refused set it already had: `AGGREGATE-VIEW-FIELD` and
`AGGREGATE-NESTED` at `Q051` (the latter in either declaration order, the
classification reading the module's declaration kinds), `AGGREGATE-TASK-CAPTURE`
at `Q500` in the task route's declaration walk, with the same module minus the
record pinned admitted so the refusal is attributable. **Zero words reserved,
zero tokens, zero codes**; the nineteenth run of the `Q-NAMESPACE-DECISION`
consequence, on the existing `Q051`–`Q057` band. The constraint-14 census
(535 `.qcore`/`.qc` files) **found a collision and kept it**: the claimed token
shape occurs 29 times in the tree, every occurrence `source.socket.v1`'s
`let listener = tcp;`, so the copy bind was deliberately **not** added to
`buffer_use` — the function route selection and the composed string route both
read this route's statements through — and a socket program's admission and the
composed route's `Q068` are pinned rather than argued
(`COPY-BIND-SOCKET-SHAPE-COLLISION`). 19 end-to-end tests drive whole programs
through `compile_files`; 14 mutations, and the harness earned its keep — its
**first run reported one VACUOUS** (a totality arm the loop test did not
measure), which a second test about the loop's shadowing refusal turned into a
real measurement, both logs shipped. What stays unreachable is recorded in those
words: `AggregateAtOwnerPosition` and `NotAnAggregate` are **unreachable from
any source program**, the route's own `Q051`/`Q052` gates deciding every such
shape first by design, and they are kept as the IR's fail-closed backstop with
the two things that would reach them named
(`AGGREGATE-OWNER-POSITION-UNREACHABLE`). Tracker **257 → 259**: three retired
by exact name after each discharge was read in full (`OWNERSHIP-COPY-OP-UNPRODUCED`
on its "emit Copy from a route" arm, `OWNED-FIELD-IR-REFUSALS-UNREACHED` on its
documentation arm, `OWNED-FIELD-BOUNDARIES-ARE-ABSENCE` on its "a test that
drives the refusal and asserts the boundary's own name" arm), five registered,
and four amended in place because this card made their text false —
`OWNED-FIELD-SEQUENTIAL-UNREACHED` **stands**, re-scoped from "`Q103`'s grown
vocabulary" to `ownership`'s whole operation vocabulary beyond the four variants
the sequential route can present. Nothing is promoted: no corpus count, unit
status, function status or CT row moves, raw units stay unpromoted, and full
language conformance stays **false**.

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
proof of race freedom. Five residuals registered
(`TSAN-COHORT-TRIGGER-CATCH-COST`, `TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED`,
`TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND`,
`TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER`) and two more found while
rebasing — `TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE`, wave 39's join-edge row missing
its `ct07-case` forced-together cell under concurrent sibling load and passing it
18/18 in isolation, 0 of 20 red on the row alone and again inside the final
suite, both facts recorded and the assertion not weakened; and
`STATUS-SOURCES-CHECKPOINT-VS-LIVE-TRACKER`, the migration's frozen
`status-sources.json` going stale by exactly this card's tracker delta and by
nothing else, disclosed rather than repaired because the archive is frozen
(constraint 21) and the check is in no gate. The card was rebased twice, onto
the documentation restructure `96142e49` and then onto the integration tip
`dffa151d` carrying `CT-PROOF-LEG-3`; neither touches this card's harness, its
subjects or any emitting source, so **every measurement above stands unchanged
across both rebases** and only the standing gate was re-run. The tracker moves
**259 → 262** on that base, and the stated count moves with it in
`docs/STATUS.md`'s `Registered follow-ups` row, the restructured `AGENTS.md`
being a router that carries no counts. The card lands **no new rule**; its one
constraint amendment — the ThreadSanitizer evidence citation constraint 15h
already carries for wave 39, extended by the cohort trigger and the failure arms
— is written **outside** every `<!-- preserved:… -->` block, in a new
"Amendments after the 2026-09-19 migration checkpoint" section of
`docs/agents/buffers-records.md`, with
`docs/reports/docs-structure-compression/verify.py` exiting **0** and the page at
24,721 bytes, under the 32 KiB cap.
Evidence: `docs/reports/tsan-failure-arms/README.md`.

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
