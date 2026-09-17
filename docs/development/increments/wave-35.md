# Wave 35 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

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

