# Wave 34 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Wave 34 close (2026-09-18): **three cards merged and one arbiter ran**. The
reconciliation swept **25** integrator-addressed items and gave every one a
verdict **measured against the tree** rather than inferred from the report's own
claim that it had landed — 11 `applied`, 3 `superseded`, 5 `deferred`, 6
`noted`, **0 `declined`** — and that distinction earned its keep. **One item was
unapplied**: `C-GATEWAY-RETURN-CALL-W4` asked for four residuals by name, and
`DOCS-COMPACT-6`, merging afterwards, moved `named_followups` 74 → 80 without
them (its six additions are platform and constraint names), so all four reached
no tracker — the sweep's own `--residuals` pass names them independently, which
is the same defect's other half. They are registered here, `named_followups`
**80 → 84**, every `why` written from the card's own §10 discharge condition.
**One amendment is recorded `superseded` rather than `applied` because it did
not land in the words proposed**: the arbiter's §11 item 3 transcribed "236
(70.5%)" against a 643-leaf denominator, which is the brief's pre-landing
236/335 figure and is internally inconsistent (236/643 = 36.7%), where the tree
carries the card's **own** §3.3 measurement, 379/643 (58.9%); the
two-orders-of-magnitude conclusion is identical either way and nothing was
softened. A **hand sweep** covered the tool's two known blind spots and found
six platform residuals `CLAUDE.md` still calls open reaching no tracker while
`DOCS-COMPACT-6` registered three siblings, plus wave-32's
`MUTEX-SHARE-CROSS-MULTI` — both reported and neither registered, completing a
partial registration being a disposition rather than a reconciliation. The gate
**reproduces the `DOCS-COMPACT-6` tip figures exactly and independently** —
4,559 passed / 0 failed / 8 ignored over 217 test executables / 228 result
summaries, fmt and clippy clean, both probes exit 0, **129** Lean audits, **229**
Python — so `AGENTS.md`'s Merged gate row is confirmed rather than carried and
this close does not edit it; the matrix was then regenerated carrying this
close's **own** `--gate`, a byte-identical fixed point with `validation` the only
section moved. The contested host row
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` was
**green here**, the second full-suite observation agreeing with
`DOCS-COMPACT-6`'s against `STABILITY-REEVAL` #1's 3/3 red **in isolation** — the
inverse of constraint 16's five load-sensitive rows, stated rather than treated
as settling anything, and not re-measured in isolation by this close. Two
obstacles are recorded rather than hidden: every `cargo` invocation first failed
with `ENOSPC` on a 100%-full host filesystem, and the first completed gate
attempt was SIGTERMed by the session's own ten-minute timeout at 78 of 228
summaries — a partial suite is not a measurement and no figure here comes from
it. The close registered **one finding of its own**,
`PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW` (`named_followups` 84 → **85**): one run
of the Python evidence suite reported `FAILED (failures=1)` and six subsequent
runs were green — four isolated, two deliberately concurrent — so the row is not
reliably reproducible in either direction and *which* row failed is unknown, that
run's per-test output not having been captured; it is registered because
`AGENTS.md` publishes "229 Python" as a property while constraint 16's five
load-sensitive rows are all Rust and nothing records a Python one. Seven
decisions are carried forward undecided, four of them wave 33's own. **Nothing is
promoted**: no unit status moved at the wave-34 arbiter, raw units stay
unpromoted, contract corpus unchanged at 76, CT coverage unchanged at 45/50
mapped with sanitized-native 9 and proof 3 — the wave mapped nothing — and full
language conformance stays **false**. Evidence:
[the close record](../../reports/wave34-close/README.md), its
[reconciliation record](../../reports/wave34-close/reconciliation.json) and
[gate](../../reports/wave34-close/gate.json).

`DOCS-COMPACT-6` (2026-09-17), the sixth three-file compaction pass, and the
first whose finding is that the three files **understated** rather than
accreted. Every headline number was re-read from its source and never from the
file being compacted; **four were stale**: the corpus row's two *headline*
figures (`179 Safe / 4,570 Unknown` and `unsupported` **50,117**, each
contradicted by the narration in its own table cell, which already ended at the
right value — the row appends each arbiter's delta without re-stating its
opening number), Lean audits **128 → 129** and Python **227 → 229**. Verified
unchanged: units 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only,
functions **186 / 85 / 4,563** of 4,834, leak **272**, `unknown_alias`
**4,214**, `double_release` **1**, `unsupported` **50,106**, CT **45/50** with
sanitized-native 9 and proof 3, contract corpus **76**, twelve example
families. **Constraint inventory 47 in, 47 out** — identical labels, identical
order, none dropped or renumbered, the six hard invariants byte-identical.
`README.md` is **unchanged, measured rather than assumed**: every figure it
states is current. Seven text residuals `STABILITY-REEVAL` #1 raised are
discharged in the ledger's own words — the CT row's lease arm no longer calls
`TRACE-ALPHABET-LEASE-BOUNDARY`'s discharged case "observed and not proved"
(`LEDGER-CT-ROW-LEASE-ARM-STALE`); constraint 15n says **three of five** tests
were shown non-vacuous, not five, publishing the per-mutation counts and naming
the two that exercise neighbouring routes by construction
(`LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`); constraint 5b says its theorem is
**conditional on `TrackedBasesAreBound`**, in the Lean module's own words, so
the `may_bind` half reads as the premise it is
(`RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`); constraint 28 gains the **two
largest premises in the gateway** — faithful C token/type extraction and the
Lean-model-to-Rust correspondence — with the repository map's `proof/` row
carrying the same disclaimer, since **no extraction, refinement or simulation
theorem exists in `proof/`** (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`). All 25
theorem-shaped citations in the three files resolve against `proof/`, **none
dangling**, and none of the Lean 4.34 sweep's 13 removals is cited there; the
two frozen reports that do cite the removed
`QCore.BranchJoin.evaluation_is_state_free` are **named in the superseding
report** rather than edited (constraint 21) — one of them, `wave19-if-statement`,
was already listed, correcting the residual by half. Fifteen residual-shaped
names reached no tracker: five registered as open work
(`WRITE-LEND-WINDOWS-RUN`, `WRITE-LEND-TASKS-WINDOWS-RUN`,
`SLICE-PERMISSIONS-COMPOSED-LEND`, `QUIRIN-ARM64-GATE`, `QUIRIN-ARM64-RUN`),
nine classified as **rules the constraints impose rather than work anyone
owes**, one already discharged — `named_followups` **74 → 80** through the
generator with `--gate` re-supplied so `validation.measurement_provenance` was
not downgraded, `dimensions.named_followups` derived to match. The wave-33
record's **fused paragraph is re-separated with no word changed**, and its two
"227 binaries" labels corrected to "216 test executables / 227 result
summaries"; the four sites in the frozen wave-33 close take the residual's
other branch, a provenance record published **beside** them
(`binary-count-provenance.md`) tabulating what each figure counts, since
216 + 11 doc-test summaries = 227 exactly. The merged-gate row and the
arbiter-chain paragraph were **merged** — the chain stated one rule four times —
and every fact removed was verified to survive in this history or in the
matrix. Gate at this tip, submodule initialised first: **4,559 passed /
0 failed / 8 ignored, 217 test executables / 228 result summaries**, 229 Python,
**129** Lean audits, both probes exit 0, fmt/clippy clean. **The contested host
row `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
was GREEN here**, against this card's own briefed expectation of a red row, and
that is reported rather than smoothed: `STABILITY-REEVAL` #1 measured it red
3/3 **in isolation**, so a row that fails alone and passes under load is the
**inverse** of constraint 16's five and cannot be dispositioned by re-running
it. The residual stays open and is recorded **beside** those five, carrying both
measurements, with the consequence neither run alone supports — *neither a
`0 failed` nor a `1 failed` settles it, so a gate report states which it saw*.
Provider **122 carried and now qualified Unix** (`PROVIDER-CONFORMANCE-UNIX-ONLY`).
Documentation only: no crate, proof, contract or script source changed, no
frozen archive edited, no verdict, leaf or count moved. Evidence:
[the compaction report](../../reports/docs-compact-6/README.md).

`C-GATEWAY-RETURN-CALL-W4` (2026-09-17), wave 34's **arbiter**, landing the
sub-shape wave 33's `C-GATEWAY-RETURN-POSITION` withdrew and **re-deriving the
measurement that scheduled it, against the card**. The withdrawal rested on a
yield ground (12 of 335 callees declared) and a **soundness** ground (applying a
callee's parameter custody at a return position would let a `Consumed`/`Released`
mode discharge a live caller obligation, on a route whose `observe_exit` has no
arm for a call result). Ground 2 is answered by a **rule**, not set aside: the
admission is a *desugaring* — `return f(a);` lowers to the pair
`PLACE = f(a); return PLACE;` the analyzer has decided since Cycle 9 — so no new
`LegacyInstruction`, no new `ReturnedValue` constructor and no change to
`observe_exit`, the summary validator or the certificate recheck. The result
mode decides the rest, and the vocabulary is closed at three: `None` escapes
nothing (accusing — `report_leaks` skips an *unmodeled* exit, so the blanket
`Unknown` was hiding leaks), `Owned` escapes a site drawn fresh from the parser's
monotone counter and so **discharges nothing prior**, and `Reallocated` is
**refused by name** because its result carries the replaced parameter's
obligation out of the exit — the case ground 2 was right about. `QCore.
ReturnCallCustody` proves it, including the refusal's non-vacuity in constraint
15n's shape (`a_reallocated_result_would_have_discharged_its_parameter`) and
`only_reallocation_is_refused`, which closes the vocabulary to "admitted ⟺
discharges nothing prior". Ground 1, re-derived rather than inherited (wave 33's
own reach log used a classifier that read the original line for a refusal
recorded at the preprocessed one; this one reads `origin`, wave 27's derived
coordinate, and is **checked** against the leaves the run actually withdrew —
catching one bug of its own before any number was read): the brief's correction
that **236 of 335 (70.5%)** callees are "W4-derivable" measures **definedness,
not derivability**. Constraint 4's second route publishes a summary only after
`observations_match_contract` finds the callee's body fully modeled and
defect-free; over the 643 `return CALL(...)` leaves of the pinned corpus that
holds for **4 (0.6%)**, with 366 (56.9%) having a callee whose own body is
`Unknown`. Of the **14** leaves withdrawn, **13 were decided by a declared
`.qcore` contract and exactly 1 by the W4 route** — one of the four
`destroy_data_point` fan-outs the brief named as exemplars. So wave 33's ground 1
was substantially right and is recorded as such; the card is not the 236-leaf
card its brief anticipated, and `RETURN-CALL-W4-REACH-BLOCKED` records that no
such card is available on this route until those bodies are modeled. Corpus
re-pin (BEFORE reproduces the baseline byte for byte in both verdict-bearing
artifacts): **7 functions `Unknown` → `Safe`** (179 → 186), none withdrawn from
`Safe`, none made provisional `Unsafe`, **zero unit moves**, leak pin
**unmoved at 272** and `double_release` 1 unmoved — the first gateway admission
card in this chain to move no leak leaf in either direction, the admitted exits
being allocator wrappers that retain nothing. `unsupported` 50,117 → **50,106**
(17 leaves withdrawn, of which 3 are second-order `call to uncontracted
function` decisions); `unknown_alias` 4,208 → **4,214**, all twelve added leaves
`Unknown`-class and in the accusing direction. Over the 478 functions blocked
only by a return leaf the split is **7 leaf-free / 0 exchanged / 1 partial / 470
unmoved** — wave 33's bucket (2) empty by construction, since a callee with no
contract in force is not admitted at all. One pinned expectation moved
**deliberately** and with its control written into the test:
`pointer_arithmetic_in_a_call_actual_stays_unknown` becomes
`…agrees_with_the_statement_position`, because the *statement* form of that
identical C was measured **already `Safe` before this card** — the `Unknown` it
pinned was the return position's extra strictness, not a custody judgment.
Evidence: 14 end-to-end rows whose central property is an **equivalence** with
the hand-written desugaring (an equivalence cannot be satisfied by refusing),
one white-box site-counter row, and **7 of 7 mutations caught** — the seventh
only after the initially uncaught whole-expression premise was investigated
rather than filed as an unexercised arm, which found the shape that reaches past
`classify_simple`'s own guard and measured it producing a **false `Safe`**.
Residuals: `RETURN-CALL-W4-REACH-BLOCKED`,
`RETURN-CALL-SYNTHETIC-PLACE-PRESENTED`, `RETURN-CALL-NULLABLE-WRAPPER`,
`RETURN-CALL-INCOMPLETE-LINE-455`.
`docs/reports/c-gateway-return-call-w4/README.md`.

`STABILITY-REEVAL` #1 (2026-09-17), the **first firing of the recurring
cadence** `CLAUDE.md` established by user direction on 2026-09-13, run over
waves 32 and 33 from the wave-33 close `d3225f91`. It is an audit: no gate
weakened, no frozen archive edited, no verdict rule touched, nothing promoted,
and **no Rust or Lean changed**. The cadence found defects in the two waves that
were told to apply it, and the findings are reported as findings. **A live
deterministic test failure stands behind a published "0 failed"**:
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
fails **3/3 in isolation** on the unmodified close tip at
`standalone_host.rs:212` — the admitting control the wave-33 repair card itself
added — while the ledger publishes 4,544 / 0 failed and calls the defect
"repaired"; the repair *is* present (`4aa24e28` is an ancestor of HEAD), and
raising the deadline 1,000 → 8,000 ms passes, so it is a **wall-clock
throughput shortfall, measured, not a hang**, corroborated by the Fluent Bit
plan's independent run (`FB-PLAN-GATE-HOST-TIMEOUT`, itself untracked). The
repair's "assertion not weakened" claim was separately **verified true**.
`MUTEX-SHARE-CROSS`, under a section headed "the `STABILITY-REEVAL`
discipline", asserts "there are no unexercised arms in the card's own rules";
**four are undriven and two are provably unreachable**, verified against the
transferability judgment's own only-pair documentation. `AGENTS.md` **contradicts
itself on the cadence's occasioning case** — line 131 still calls the F02 lease
arm "observed and not proved: no test drives a lease-carrying reference run into
it" where constraint 15n in the same file records the discharge, and the arm was
re-measured **genuinely driven** — and 15n separately **overstates** its
mutation evidence as five of five where the measurement is **three of five**,
the other two tests being structurally incapable of falsification by an arm
mutation. All three trace to one skipped step: **wave 32's close wrote no
`reconciliation.json`** (only `gate.json` exists) and **six of its seven merged
cards were never swept**, including the card whose report is headed "fully
discharged, retire it". Two structural blind spots explain why nothing caught
it: `--residuals` scans **only `docs/reports/`**, and its tracker definition
counts an `AGENTS.md` constraint mention as tracking — which is why **both
wave-33 vacuity residuals had no ledger entry at all**. Commit `37bc9004` is an
**uncarded, unindexed** semantic edit in the wave-32/33 seam (invisible to both
the increment records and the sweep's exclusive base) that moved two pinned F02
digests and removed four tests from the Windows provider gate; its digest move
is **exonerated** on the merits — it repairs a stale Windows twin of a wave-31
authorized re-pin that had drifted a whole wave unseen. On the proved/observed
axis: CT01, CT12 and CT14 are **one byte-identical fixture** (45 mapped cases
rest on **43 distinct artifacts**, the proof numerator's three being pairwise
distinct); the gateway's Lean modules are **abstract rule theorems, not
extraction**, a disclaimer in every module and card report and in **neither**
authoritative file; and constraint 5b's "sound … **because** … proved as
`an_admitted_chain_names_no_tracked_place`" **elides that the theorem takes
`TrackedBasesAreBound` as an explicit hypothesis** the module declines to prove.
`RETURN-POSITION-ESCAPE-UNREACHABLE` was re-measured far more broadly than its
originating card (985 + 63 tests, both probes, and the **full 185-unit corpus
audit byte-identical under the mutation**) and is **accurate and terminal** — no
test can ever distinguish the arm — until `FB-BACKLOG-CUSTODY` reopens it.
**Measured negatives are recorded too**: every other headline number reconciles
to its cited artifact (185 units, 4,834 functions 179/85/4,570, leak **272**,
`unsupported` **50,117**, CT 45/50, corpus **76**), all 18 theorem-shaped
identifiers in `AGENTS.md`/`CLAUDE.md` resolve to real theorems, the Lean 4.34
sweep's 13 removals were **all verified vacuous** with two genuine
assumed-conclusion defects repaired rather than deleted and the axiom closure
intact, and 200 stat'ed report citations found **zero** missing committed paths.
`named_followups` **47 → 74**, appended and then **regenerated through the
generator to a byte-identical fixed point** (constraint 25, never by hand).
The card adds **two Python tests** (227 → **229**) discharging the first half of
`D02-DIMENSIONS-FOLLOWUP-COUNT`, which fired twice and was caught by a person
both times because the existing tests compare generator output with generator
output; both were shown **non-vacuous by mutation** (the exact 47 → 45 drift, and
an `indent=2` reserialisation) with the matrix restored byte-identically after.
Gate: fmt and clippy clean, both probes exit 0, **128 Lean audits** (unmoved
across the 4.34 upgrade), **229 Python**, and
`cargo test --workspace --no-fail-fast` exiting **101** at **4,543 passed /
1 failed / 8 ignored over 227 result summaries** — which **reproduces the Fluent
Bit plan card's run exactly**, so with three isolated runs beside it the failure
is neither a load nor an isolation artifact, and 4,543 + 1 = the ledger's own
4,544, putting the whole disagreement in the pass/fail split rather than the
denominator. The failure is reported rather than absorbed: repairing it would
have meant a measurement card editing `crates/`, the `CARD-SCOPE-SELF-MERGE`
breach wave 33 registered. CT
coverage unchanged at 45/50 mapped, sanitized-native 9 of 45, proof 3 of 45;
corpus, contract and provider rows carried. Nothing is promoted; full language
conformance stays **false**.
[Report](../../reports/stability-reeval-1/README.md).
