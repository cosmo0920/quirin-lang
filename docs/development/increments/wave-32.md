# Wave 32 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Wave 32, card `TRACE-ALPHABET-LEASE-BOUNDARY` (2026-09-14): the first card run
under the new `STABILITY-REEVAL` discipline, paying back the honesty debt that
discipline's first method step was written around. Wave 31's integration had
resolved a semantic conflict between `WRITE-LEND-TASKS` and
`CT03-PROOF-ALPHABET` inside `reference_trace` as a union — the Buffer write
encoded, the two range-write-lease events still refused — and reported it
verified on two green suites that never drove the arm; the residual
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` recorded that as **vacuous
greenness** and named three alternative discharges. Measured first, chosen
after: the arm is **reachable today** — card `WRITE-LEND-TASKS`' own admitted
flagship produces a fourteen-event reference run whose index 2 is a
`BufferLeased` — so discharge (c), *show it unreachable*, is **false**;
`derive` is the only route by which a lease could reach an
`AdmittedCausalOrder`, the other two public routes (`parse_observed_trace`,
`declare`) being unable to express one, the first by its tag table and the
second at the type level; and — the finding the residual did not anticipate —
**dropping the lease events instead of refusing them would have been an
*admitting* mistake, not a lossy one**: the flagship's ten-event lease-free
remainder is a trace `AdmittedCausalOrder::declare` **accepts**, as a causal
order for a run in which two workers held outstanding write authority the order
never mentions. The card therefore took (a) and (b) **together**.
`proof/QCore/TaskTraceLeaseBoundary.lean` (new, registered in
`AuditTaskTraceLeaseBoundary.lean`, **125 → 126** audits, no project axiom)
proves the boundary on the *observation* alphabet where CT03's theorems cover
only the observable one: `succeeds_iff_no_event_is_refusable` (exact in both
directions — no lease slips through on any shape of run, and the refusal blocks
nothing else), `encoding_is_positionwise` with `encoding_preserves_length` (the
formal content of "never silently dropped"), `refusal_points_at_a_refused_event`
with `refusal_is_the_first` (the Rust pin `index: 2` is an instance, not a
coincidence), and `the_dropped_encoding_would_have_been_admitted`.
`compiler/tests/trace_alphabet_lease_boundary.rs` (new, five tests) drives a
real compiled lease-carrying reference run into the arm, with a read-share twin
as the control; each test was shown non-vacuous by **mutating the arm** three
ways — admit-as-loan, silent drop, wrong index — catching 3, 3 and 2 of 5. The
production diff is **documentation only**: the arm's behaviour is
byte-identical, no refusal is weakened, no `Q` code allocated, no word
reserved, and **no arbiter ran** (no gateway, contract, audit or provider file
is touched, so the corpus rows are carried). AGENTS.md gains standing
constraint **15n**. The residual is **fully discharged and retirable**; its
forward-looking half is replaced by `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` —
whether the observable alphabet should grow to carry leases is **undecided**,
because CT03's method needs a rule shown non-vacuous in both directions and
that half is unavailable while a worker *holds* but does not *exercise* its
lease (`WRITE-LEND-TASKS-EXERCISE`), with the operative consequence recorded as
a measured boundary rather than a defect: write-lend programs sit outside F02
relational comparison by construction, their native evidence being sanitizer
and allocation-count. Evidence:
`docs/reports/trace-alphabet-lease-boundary/README.md`.

