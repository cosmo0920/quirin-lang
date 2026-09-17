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

