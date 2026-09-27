# Wave 41 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.
>
> **Split at wave 45.** The oldest paragraphs of this record (`CT-PROOF-LEG-3`)
> are in [wave-41-history.md](wave-41-history.md), moved verbatim by `DOCS-COMPACT-9` to keep
> this page below 32 KiB.

**`SOURCE-SCALAR-CALLS-V1` (2026-09-20).** Implemented explicit i32/unit source
functions, immutable i32 locals, checked arithmetic, direct acyclic calls and
value returns, plus user-requested `#import std.io; print(expr);` for an evaluated
i32 and newline. I/O effects propagate through calls and explicit empty rows
refuse them. The backend contributes fragments and an entry body to the existing
C assembler, emits one main, and prints no automatic result/fault diagnostics.
Three GPT-5.6 Sol medium subtasks supplied focused tests, documentation and an
independent implementation review. The scalar model and audited Lean cover
arithmetic, call/frame behavior, output order and effect-row rules; correspondence
remains a review premise. Final gate: 4,884 Rust passed / 0 failed / 10 ignored,
150 Lean audits, 313 Python tests, clean fmt/clippy and both probes exit 0; five
host-sensitive workspace rows also pass in isolation. The initial full Lean
build found an import after module documentation; import-only commit 62129bd2
corrected it and the complete Lean gate passed. Earlier interrupted/failing runs
are preserved. R04 remains partially repaired, 271 follow-ups are carried, and
C projection counts/provider122 are not remeasured. [Report and precise run identity](../../reports/source-scalar-calls-v1/README.md).

**`W42-PRIORITY-INTEGRATION` (2026-09-20).** Three GPT-5.6 Sol medium agents
implemented a bounded scalar-member-store rule, total Monkey callback
accounting and a positive build-exclusion census. The syntax rule retains
alias/liveness checks and preserves the projection frame on flow refusal;
Rust negative controls and the audited Lean frame/refusal model accompany it.
The combined pinned audit moves only `ctr_span_start_ts` and `ctr_span_start`
Unknown→Safe: **223 Safe / 84 provisional Unsafe / 4,527 Unknown** projected
functions, with units unchanged at **2 / 42 / 137 / 4 data-only**, unpromoted.
It removes **362 lexical refusals + 2 downstream call refusals**, with
**350 exact-coordinate alias exchanges**, seven downstream alias additions
and one old alias removal: unsupported **49,371**, unknown_alias **4,665**,
leak **272**, summary_conflict **1**, net **8** fewer diagnostic leaves.
The forecast miss is preserved. All **87** unprojected definitions are
positively excluded by the pinned historical build across **24 full-path
units / 23 basenames**; no projection defect or unresolved census row is
established. Unit reasons remain **136 unsupported-syntax / 1 incomplete
coverage / 48 without a reason code**. Monkey has **3/3 selected callbacks accounted**,
**8/8 source paths** checked by optional focused replay, **0/3 Cycle 18 composed**
and **3/3 explicit Unknown**; the ordinary no-binding audit checks **0/8**
native paths. Its sidecar changes only schema and callback accounting; three
other artifacts are byte-identical and diagnostics exactly match the corrected
isolated syntax run. The [integration report](../../reports/w42-priority-integration/README.md)
records the gate: **4,863 Rust / 149 Lean / 313 Python**, zero failures,
10 ignored Rust tests, 250 strict summaries over 248 targets, clean fmt/clippy,
both probes exit 0 and five isolated host checks green; 122 provider tests are carried.
All 15 reconciliation items are recorded. All 269 previous follow-ups remain;
two explicit certificate entries bring the ledger to **271**. The input-chunk
lifecycle/ChunkIO teardown and Monkey event-composition certificates remain
open; neither this bounded slice nor the census completes Cycle 19.

**`B01–B04-CALL-POSITIONS` (2026-09-20).** Four GPT-5.6 Sol medium agents
implemented bounded return evaluation, branch conditions, loop/switch conditions
and neutral sibling nested arguments. The merged rules retain call effects,
alias/liveness obligations and CFG exits; new return/control call extraction
requires an external declaration without a local definition, preserving the
pre-existing recursive pointer boundary. B01's direct opaque getters escape no
identity. B04 keeps effectful/deeper nesting refused. The final pinned arbiter
moves **8 functions Unknown→Safe**: **213→221 Safe / 84 provisional Unsafe /
4,537→4,529 Unknown**; units remain **2 / 42 / 137 / 4 data-only**, unpromoted.
Unsupported diagnostics fall **49,753→49,735**: **12** return leaves from B01
and **6** branch leaves from B02, with no additions or exchanges; leaks **272**,
unknown_alias **4,309** and summary_conflict **1** are unchanged. B03/B04 have
zero measured corpus movement. The combined forecast's eight mover identities
match, but its **57-leaf** prediction misses: the guard repair retains **39**
local-definition branch refusals. Initial B01/B02 forecast misses and failed
gates remain disclosed; the sibling refusal pin is deliberately replaced by a
positive certificate check and retained deep-nesting refusal. No tracker entry
retires (**269** remain), the indirect+branch pair stays **196**, and CT
mapping/proof/native counts and full-language conformance do not change.
[Measurements, validation and reconciliation](../../reports/b-series-integration/README.md).

**`CT41-MAP` (2026-09-19).** A **second, independent** CT41 mapping was built
from the same base and reached the same row; `CT41-PUBLICATION-MAPPING` landed
and this card is what survives **reduction onto it**. It claims **no coverage**:
CT41's row, stage evidence and bucket retirement are the landed card's, and this
one adds no `ct_stage_evidence.json`. The agreement is the substance — both
cards measured, separately, that the wave-9 attribution rule's clause (b) cannot
be met literally for this case. They differed only in the exit: the landed card
**supersedes** the clause for CT41 alone and says so in a routed constraint page,
while this one read "the construct the scenario names" at the **mechanism**
level and claimed the clause as written. **The superseding form is the more
honest of the two and is what the project should carry** — a supersession is a
visible, bounded, per-case declaration, whereas a mechanism-level reading is a
silent general reinterpretation of the word "exactly" that any later multi-site
control could lean on without saying so; whether clause (b) is read at token
level with per-case supersessions or at construct level generally is left as a
decision for the constraint page (`CT41-ATTRIBUTION-CLAUSE-SUPERSEDED-FOR-ONE-CASE`).
Kept: **two controls** the landed fixture does not carry, each re-measured
against the **landed** route (whose qualifier arms and memory-order reader both
differ from the route this card first measured) and each derived from the
accepting control by one declaration substitution, which the runner now asserts.
`qualified_atomic.c` (`volatile _Atomic int`) refuses `UnsupportedAtomicFlag` —
the flag-type arm neither qualifier refusal reaches, and a live regression pin on
the landed exact match, since widening it to `contains("_Atomic")` **issues a
happens-before certificate** for a flag this route does not model.
`volatile_atomic_ops.c` is **not valid C11** — Clang refuses `address argument to
atomic operation must be a pointer to _Atomic type` — so it gets its own
`language_boundary` block rather than riding a loop that requires every artifact
to be admissible C; it is the measurement that makes the landed supersession
checkable rather than assertable, since the one-token alternative does not exist
in the language. Six mutations, all red through assertions, every file restored
sha256-identical. Dropped as duplicates: the manifest row, the STATUS coverage
rows, `MAPPED_CASES`/stage-vocabulary edits, three controls, and **all ten Python
repairs** — main's `unmap_ct41()` reached the same fix independently. Kept from
that work is the **finding** plus one hazard main does not record: on a tree
where the mapping card's retirement was the only one,
`test_retiring_a_still_unmapped_case_fails_closed` could pass through the *wrong
rule*, a test reaching its refusal not thereby testing its own
(`CT-UNMAPPED-LABEL-MACHINERY-HAS-NO-LIVE-INSTANCE`). Seven residuals registered
and two dropped by name after re-verification against main — including
`CT41-PROOF-NOT-BOUND-TO-THE-CASE-ARTIFACT`, whose content the merge **changed**:
the landed `non_atomic_completion_has_no_payload_path` is a theorem about CT41's
own case class, so the gap is no longer a missing theorem but the artifact
binding, and a CT41 proof leg is now a realistic next card. Follow-ups
**262 → 269**; coverage, proof and sanitized-native counts all **unmoved**.
[Evidence](../../reports/ct41-map/README.md).

**`CT41-PUBLICATION-MAPPING` (2026-09-19).** CT41's original volatile
completion case now executes through the bounded C publication route: volatile
and plain refuse, release/acquire checks, and either relaxed arm refuses.
The explicit protocol-replacement attribution decision preserves qualifier
invariance. Source-shape checks precede attribution; Lean proves the missing
payload ordering path for non-atomic flags. Evaluated enum values now prevent
misleading names from conferring release/acquire ordering. **50/50 mapped**, **5/50 case-bound
proof**, **10/50 gated sanitized-native** on Linux/macOS; mapping is not full
conformance or production safety. The CT41 blocker retires: **263 → 262**
follow-ups. Standing gate: **4,812 Rust passed / 0 failed / 10 ignored;
145 Lean audits; 305 Python tests**, all required exits 0.
[Decision, bounds and measured evidence](../../reports/ct41-publication-mapping/README.md).

**`CT-INTEGRATION-REBASE` (2026-09-19).** Rebased the published CT branch onto
main `026314c8`, preserving reviewed CT conflict resolutions and main’s TSAN
evidence. Code, fixtures, scripts and Lean proofs are unchanged from `a0584a50`;
original checkpoint measurements remain intact. Fresh standing gate: **4,808
Rust passed / 0 failed / 10 ignored; 145 Lean audits; 305 Python tests**, all
required exits 0. Coverage and 263 follow-ups are unchanged.
[Rebase identities and evidence](../../reports/ct-remaining-implementation/README.md#rebase-onto-main).

**`CT-REMAINING-IMPLEMENTATION` (2026-09-19).** Six Sol medium agents in two
waves implemented and reviewed CT10 field-footprint checking, CT24 real
cleanup/task-ledger composition, and CT41's bounded atomic-publication
prerequisite. The combined corpus maps **49/50**, leaving CT41 unmapped;
case-bound proof stays **5/49**, gated sanitized-native **10/49** on Linux/macOS.
Declared layout, successful task windows and dynamic invocation bounds remain
explicit premises, not production safety claims. Two CT10 entries retire and
`CT10-TASK-FAILURE-LIFETIME` is registered: **264 → 263** follow-ups. Compact
rule/spec amendments and the Fluent Bit transfer queue preserve these scopes.
Combined standing gate: **4,808 Rust passed / 0 failed / 10 ignored; 145 Lean
audits; 305 Python tests**, all required exits 0. [Integration evidence and reconciliation](../../reports/ct-remaining-implementation/README.md).

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
