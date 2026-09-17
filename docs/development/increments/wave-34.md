# Wave 34 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

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
