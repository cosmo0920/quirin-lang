# Wave 35 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`C-GATEWAY-CONSTANT-DO-WHILE` (2026-09-17), wave 35's first arbiter, modelling
the constant `do … while (0)` exit and withdrawing the corpus's only
`double_release` row. `do S while (0)` executes `S` exactly once (C11
§6.8.5.2), so the CFG without a back edge is the construct's **exact**
semantics rather than an approximation; the builder had given every `do`-`while`
header both `body_entry` and `continuation` as successors whatever its
condition. The census the card was asked to re-derive is the finding that sizes
the problem: on the **preprocessed** text the 185 units carry **15,053**
`while (0)` occurrences (librdkafka 13,361, cmetrics 841, ctraces 435,
monkey-libevent 286, fluent-bit-chunk 130, chunkio **0**), against the 266 a
source-level grep of two directories sees — a factor of about 57, because the
idiom is a macro body and macro expansion is where it lives; 185 units
measured, 0 missing from the compile database, 0 preprocessor failures, and the
brief's `librdkafka-2.10.0` is **2.15.0** in the pin, its 121 reproducing
exactly. Because removing an edge is the **admitting** direction, the rule is
keyed on the single **literal token** `0` and on nothing else (constraint 5a —
never an evaluated expression): `while ((0))`, `while (1 - 1)`, `while (0 == 0)`
and `while (0 && 0)` each keep the CFG they had, pinned by a test, and the
header node and its **exit** edge are never removed, so no leaf coordinate
moves for an unrelated reason. `QCore.ConstantDoWhile` (Lean audits **129 →
130**) states both halves: `constant_do_while_is_its_body` is the `≡ S`
equality, `removing_the_back_edge_only_removes_paths` and
`the_successor_list_never_grows` are the conservativity — the new CFG's paths
are contained in the old one's **for every condition**, so this change can
withdraw a refusal and can never manufacture one — and
`the_repeated_release_is_a_path_the_c_does_not_have` names the parting witness
as the corpus's own row, one release under the C against two under the back
edge. The re-pin is surgical and the BEFORE run reproduces
`c-gateway-return-call-w4`'s archives byte for byte: **four of the five
artifacts are byte-identical between BEFORE and AFTER**, only `diagnostics.json`
moves, and within it exactly **one leaf removed and zero added** —
`double_release` **1 → 0**, the `rd_kafka_jwt_b64_decode_payload` row at
internal L21340 (origin `rdkafka_sasl_oauthbearer_oidc.c:192`, acquisition
`:136`, reached through `RD_IF_FREE`'s own `do { … } while (0)`). It is
**withdrawn, not discharged**: the function keeps its other 18 leaves and goes
provisional `Unsafe` → **`Unknown`**, so functions read **186 Safe / 84
provisional Unsafe / 4,564 Unknown** with **Safe unmoved and nothing promoted**,
the unit stays `Unsafe` on two sibling leak-carrying functions, and there are
**zero unit status moves** — the wave-31-through-35 record holds. Leak **272**,
`unknown_alias` **4,214** and `unsupported` **50,106** are all **unmoved**
(constraint 7 has nothing to attribute), and the 32 `unmodeled … do-while
condition` leaves are a different question and measured **unmoved** at 32. The
gap between 15,053 expansions and one leaf is published rather than glossed: a
spurious back edge is only observable where the body releases or acquires a
tracked identity that survives to the header, which in a corpus with 4,564
`Unknown` functions is rare — so the rule's measured reach today is one leaf and
its forward value, foreclosing the same false accusation in every expansion a
later card promotes, is left explicitly unquantified
(`CONSTANT-DO-WHILE-LATENT-REACH`). Two findings came out of the campaign rather
than the plan. All six mutations were caught, **including the control** that
swapped successor *order* on the **unchanged** arm — so successor order is
observable in the worklist schedule, recorded not repaired, the same
sensitivity constraint 6 carries for `report_leaks`
(`CONSTANT-DO-WHILE-SUCCESSOR-ORDER`). And a neighbouring route was found and
**measured pre-existing on the unmodified base**: where a scalar binding's value
is modelled precisely, a `do`-`while` on it was already decided from the value,
answering conservatively on all four probed shapes (`flag = 0` loses the edge;
`= 1`, a tracked reassignment to 1, and an unmodelled initializer each keep it),
so it is not a false-`Safe` source, it is pinned by a test so a refactor cannot
move it silently, and it is why the card still had work — a macro's `while (0)`
is a token and no binding (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`).
Wave 35's **second arbiter gets a measured hand-off rather than an assurance**, and it is the largest thing this card does: `legacy_joined_provenance::derive` returns **no** conditional-provenance facts at all for a cyclic CFG (constraint 10b), so removing a back edge can flip a whole function into deriving them. An instrumented pair of corpus runs — the probe placed in `legacy_c.rs` and never in the sibling's file, reverted on every exit path — measures **654** of the 4,771 analysed functions moving **cyclic → acyclic**, 34% of the 1,926 that were cyclic, with **0** moving the other way; and the measured diagnostic effect of all 654 is **zero** — `unknown_alias` unmoved, no leaf added anywhere. So that pre-pass's reach grew by 654 functions before the second arbiter changed anything, its coverage figures must be quoted against this tree rather than `a300a39d`, and the 1,272 that stay cyclic are closer to the genuine loops now that the macro idiom's spurious cycles are gone (`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`). Follow-ons also name the unclaimed literal-zero spellings
(`0u`, `0L`, `0x0`, `'\0'` — `CONSTANT-DO-WHILE-ZERO-SPELLINGS`) and the
deliberately untaken `while (1)` / `for (;;)` generalisation, whose direction is
the opposite one: modelling *those* exactly would remove an **exit** edge, which
is accusing and needs its own Lean statement and re-pin
(`CONSTANT-DO-WHILE-OTHER-CONSTANTS`).
**`CT-CTWIN-LANE` (wave 35) — CT46 mapped, 45/50 → 46/50; CT41, CT42, CT10 and
CT24 re-measured and deliberately left unmapped.** The lane was briefed to map
**CT41** on the C-twin route as its primary deliverable, and it did not: the
brief's premise is contradicted by a measurement already archived on this tree,
and re-measuring confirmed it rather than overturning it. CT41 asks for "no
synchronization proof" from a `volatile` completion flag, and clause (b) of the
wave-9 attribution rule requires a control in which removing *the construct the
scenario names* moves the verdict — while the decided content of card
`CTWIN-PUBLISH` §5 is precisely that removing `volatile` moves **nothing**, C11
`volatile` being an access-ordering qualifier and not a synchronization
primitive. That obstruction is **structural, not a gap**: no measurement can
retire it, because the thing a mapping would need is the thing the project has
decided is false. Five shapes were re-measured here anyway, and one half of the
recorded blocker did move: the wave-17 reason's clause *"no harness path runs
it"* is **retired** (that path has existed since that very lane, and this lane
ran it), while clause (a) fails *more* sharply than wave 17 recorded — the
scenario's own consumer shape, `while (!g_done) { } q_free(p);`, is answered
**Safe with no diagnostic at all**, with and without the qualifier, so a mapping
would record a certification against a case demanding a non-certification. The
typed-source arm named in the archived blocker was examined for the first time
and cannot carry the case either, for a reason about the rule's *shape*: it
refuses `volatile`, `_Atomic` and `restrict` in one branch with one message, so
it treats a real synchronization primitive exactly like the qualifier that is
not one — the opposite of the distinction CT41 exists to make
(`CT41-TYPED-SOURCE-UNDISCRIMINATING`). **CT46 was mapped in its place**, on
card `MUTEX-SHARE-CROSS`'s wave-32 composition, under a new
`checked_mutex_crossing` verdict mode on the `mutex_guard` route — the corpus's
first **admitted** case that deliberately stops at the checker. `compile_files`
accepts the module and returns `c_source: None`, and `interpret_mutex_program`
refuses its event list with `UnsupportedEvent { event: 1 }`, event 1 being the
`Spawn` the crossing introduces; that is `MUTEX-SHARE-CROSS-EMISSION` measured
rather than inherited, and it is why four stages read `unassessed` and none
reads `executed`. The emission boundary is pinned in **both** directions — one
control is the same route, payload and generation with the clause removed, and
it *does* render and *does* interpret to its pinned observation — so the case's
empty emission is attributable to the crossing rather than to the route. The
accept half is the lowered shape (a real `Spawn{0,1}`, the child's own `Lock`, a
real `Join{0,1}`, and the complete guarded-reach vector rebuilt whole from the
event list); the "only" half is four controls refused by four distinct rules,
including a crossing that names a **real** generation guarding a *different*
payload, which is what keeps the mapping from meaning "naming any lock will do".
Two of CT46's own words are published as untranslatable rather than rested on:
there is no `wrapper` type at all and no `Send`/`Sync` trait, both carried
instead by the transferability judgment's `TransferKind::ProtectedResource`,
asserted against `MUTEX_SHARE_CROSSING` itself rather than against fixture
prose. **CT42** stays unmapped with **both** arms measured where earlier lanes
measured one: the C-twin arm's loader still refuses an int-typed owned result
verbatim (`CT-CTWIN-DESCRIPTOR` unchanged), and the socket arm that wave 31
landed at model level is **unreachable from source** — `derive_socket_cells` and
`footprint_program` are called from one unit test and from nothing in the
frontend, the checker or `compile_files` — so no `.qcore` fixture can reach the
`Q664` refusal and a row there would name evidence the harness cannot execute
(`CT42-CELL-UNROUTED`). **CT10** stays unmapped on half a scenario (the
concurrency half exists at byte granularity since wave 31; no source syntax
declares a bit-field, and the gateway refuses a bit-field's whole enclosing
function), and **CT24** stays withdrawn and untouched per constraint 25. Six
mutations of the new runner were each shown to fail the suite before it was
believed, and removing the lane's own retirement declaration was measured to
turn both derivations red. Discharging `CT-SHARED-FIXTURE-DISCLOSURE`'s duty —
hashing all six new files against every fixture in the corpus, which found no
collision — incidentally surfaced **two undisclosed** byte-identical pairs of a
different kind: `CT02/case.qcore` ≡ `CT14/read_before_join.qcore` and
`CT03/case.qcore` ≡ `CT04/write_after_joins.qcore`, each a case's own artifact
doubling as another case's *control*. Classified as a **disclosure gap, not a
defect** — such a control is arguably a strong one — but editing either case's
artifact silently changes another case's control, with nothing anywhere saying
so. Registered as `CT-FIXTURE-CROSS-CASE-DUPLICATES` and deliberately not
repaired: the repair touches other lanes' fixtures. No C gateway source was touched (two arbiters own
that crate this wave), no checker rule, diagnostic code, lowering, model, Lean
theorem or reserved word was added, and the only Rust is harness wiring.
Sanitized-native stays **9** and proof stays **3** — denominators only, 9 of 46
and 3 of 46 — and a grown denominator is mapping, not conformance.
`docs/reports/ct-ctwin-lane/README.md`.
**`VERIFIER-HOST-CONTROL-BUDGET` (2026-09-17)** dispositioned
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` by finding its mechanism, and the
mechanism was a defect rather than the host property two waves had assumed. The
transport loop's backoff charged a **fixed 1 ms quantum for an event taking
~5 µs**; with this host's measured pipe capacity of **8,192 bytes** (two pages,
where `PIPE_DEF_BUFFERS` gives 65,536 — unexplained, `HOST-PIPE-CAPACITY-UNEXPLAINED`)
that is a ceiling of 8 KiB/ms, so the control's 8 MiB request needs ≥1,024
quanta and consumes a 1,000 ms budget whole — **the same arithmetic wave 33
repaired, surviving on the write side**, that card having removed only the
*unconditional* sleep. The inversion `docs-compact-6` recorded is explained:
the quantum is charged only when the host comes back around before its peer is
scheduled, so a **quiet** machine pays it and a contended one does not. CPU
frequency scaling, the brief's leading candidate, is **falsified** — the passing
heavy-load arm ran at 2,954–3,082 MHz against up to 3,654 MHz in the failing
ambient arm. Landed as disposition (c): `backoff_until_ready` waits for pipe
readiness via `poll(2)`, bounded by the same quantum and the same deadline, with
every assertion, deadline, cap and refusal arm untouched and **the test file not
edited at all**. Paired and interleaved under one ambient load, the old backoff
was 4/8 red with phase 4 taking 350–1,000 ms and the new one 8/8 green at
42–112 ms; per-wait cost fell from 1,057 µs to 5.5 µs, a factor of 190. Also
measured, and **not** repaired: the row's *first* arm carries an opposite
sensitivity, its fixed one-second guard over a 40 ms deadline failing 3 of 6
repetitions at load ≈100 (`VERIFIER-HOST-EARLY-ARM-OVERLOAD`, constraint 16's
shape, amendment text proposed to the integrator). The residual's
"deterministic 3/3 in isolation" **does not reproduce**: 22 pre-repair isolated
repetitions gave 7 pass / 15 fail. Invariant 6 checked rather than inherited —
the only two Lean modules naming `ProcessProvider` model the construction
discipline and the declared-limit-kind plan, neither the transport loop. Two
backoff arms are recorded **defensive, not verified**
(`HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN`); Linux only, no macOS run claimed, and
no Windows sibling exists since the non-Unix arms refuse before any transport.
`docs/reports/verifier-host-control-budget/README.md`.
`SWEEP-BLIND-SPOTS` (2026-09-17) repaired the four measured blind spots
`STABILITY-REEVAL` #1 found in `scripts/reconciliation_sweep.py` and then
pointed the mechanism at the wave it was built in and skipped in.
`--residuals` now scans the card reports, `docs/development/increments/` and
the standing-constraints text of `AGENTS.md`/`CLAUDE.md` rather than
`docs/reports/` alone (`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`); `TRACKERS` is
narrowed to the matrix's `named_followups` **alone**, so a name living only in
a constraint paragraph is reported **untracked with where it was found** — a
constraint states a rule that must survive refactors, the ledger states work
that is open, and a name can be in the first and untracked as the second
(`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`); `--since` became
inclusive of its own base with `--range A..B` and `--uncarded` beside it, the
last returning **exactly one commit** on the wave-33 seam, `37bc9004`
(`WAVE-BASE-UNSWEPT`); and a close's `deferred_decisions` are re-listed at
every later close until each carries `resolved` or `withdrawn` with a ground,
the shape and vocabulary **read from the wave-33 close rather than invented**,
so that record is valid unchanged and was not edited (constraint 21,
`WAVE-33-DEFERRED-DECISIONS-UNTRACKED`). The tool's rule is unchanged and
restated in its own docstring: **it decides nothing**. Tests 27 → **45**, each
new one built from a fixture the old behaviour would have missed and **each
falsified by mutating the arm it covers** — six mutations, every one caught,
the module restored byte-identically. `docs/reports/wave32-close/reconciliation.json`
is written as a **retro-sweep performed in wave 35**: `--range
8bf7fd33..474f27af`, all **seven** wave-32 cards, **67 items**, every verdict
measured against the tree rather than taken from a report's claim — 27 `noted`,
18 `applied`, 16 `deferred`, 4 `superseded`, 2 `declined` —
discharging `WAVE-32-CLOSE-UNSWEPT` without editing that close's `gate.json`.
Three findings came out of the measurement, none a code defect and all the
class of loss an unswept close produces: **`MUTEX-SHARE-CROSS`'s entire
integrator list is unlanded** — `AGENTS.md` contains that string **zero times**
and its `Q600` row still reads "Four later cards move this row", while the
authority that row summarises, spec §L21, *does* carry the widening
(`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`); constraint 14 now claims
"**every card of waves 32, 33 and 34**" over an enumeration omitting
`MUTEX-SHARE-CROSS`, `PROOF-LEG-BATCH` and `DOCS-COMPACT-5`
(`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`); and a recorded `applied` verdict
is **false in half its evidence** — `W32-RECONCILE-SWEEP`'s own `DOCS-COMPACT-O`
amendment was resolved away at its own merge `3bbec4d0` (**−2,208 bytes**, the
only `named_followups` row to change across it), so `DOCS-COMPACT-4-TABLE-SYNC`
reaches no tracker at all and the verdict citing it still reads `applied`
(`W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`). The general lesson is recorded as a
boundary rather than repaired: **a green `--check` says a verdict was recorded,
never that its "where" still holds.** Surfaced and deliberately not taken:
`docs/reports/wave34-close/` **does not exist**, so wave 34's three cards' 25
integrator items have no verdict anywhere — the same defect two waves after the
card that found it (`WAVE-34-CLOSE-UNSWEPT`). Six residuals registered,
`named_followups` **80 → 86** with `dimensions` moved by the generator
(`docs/reports/remaining-d02/build_matrix.py`, `--gate` re-supplied so
`measurement_provenance` was not downgraded). No crate source, no frozen
archive, no `AGENTS.md` or `CLAUDE.md` edit; the proposed constraint text and
three `AGENTS.md` rows are carried to the integrator **in the words the cards
proposed**. `docs/reports/sweep-blind-spots/README.md`.

