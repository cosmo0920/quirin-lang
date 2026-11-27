# GAPO-PARITY-DECISIONS-NONDETERMINISTIC: prediction (frozen before implementation)

Base: local `main` at `0ebe77f86`. Frozen before the first implementation
commit; the BEFORE runs were still in flight when it was written.

## The mechanism read

`typed_place::DECISIONS` (thread-local) is pushed by every typed access-path
reader that decides a place (`typed_place::tracked_chain`, rule 10t, and
`record_object_place::record`, rule 12j), whoever built the reader. Only the
recording consultations take it (`legacy_c::typed_path_plan` and the four
`legacy_typed_positions` hooks), and they clear it only after their own node
matched (`StatementIndex::statement_with` after the unique match,
`argument` after `argument_node`, `expression_node` at its start). Readers
built by consumers that record nothing (`legacy_aggregate_value` x3,
`legacy_elaboration`, `legacy_atomic_route` x2, the residual-shape probes of
`typed_position_shapes`, the second `argument`/`returned`/`condition` asks of
`legacy_typed_positions`) leave their decisions behind. A later recording
consultation whose node does not match (no clear) then writes a `typed-place`
row made of those leftovers; which leftovers a worker thread holds depends on
the units the scheduler gave it, so the row varies between runs.

`pointer_store::DECISIONS` has the same shape but one producer
(`pointer_store::read_store`, reached only from `statement_with`'s reader) and
one consumer that always takes it right after (`typed_path_plan`).

## Predictions

1. **Verdicts and published diagnostics: byte-identical to main.** Both
   thread-local lists are written only when the parity stream is on and read
   only by the two `record_decisions` functions, which write parity rows and
   return nothing. Corpus **756 / 9 / 4,069**, `resource-diagnostics.json`
   `ce57d194…`, every sidecar identical; system **85 / 4 / 359** and the
   tail-pause slice identical (files and totals) to the BEFORE runs. No
   Safe/Unsafe/Unknown/Pending movement anywhere.
2. **BEFORE nondeterminism reproduces:** across five BEFORE corpus runs the
   parity multisets differ in `typed-place` rows only (expected: the
   `cio_file_st_get_meta_len` line 2223 row, as measured by the optimization
   card; possibly others).
3. **AFTER determinism:** five AFTER corpus runs give identical parity
   multisets (no row set apart); system and slice twice each, identical.
4. **AFTER against BEFORE, parity stream:** removals only, all in
   `position: "typed-place"` rows: rows written by a recording consultation
   whose own node did not match (statement or call-argument position) and
   that carried only leftovers disappear. Every row a recording consultation
   writes after its own clear is unchanged in content. Every non-typed-place
   row (including `pointer-store` and `call-instantiation`) is unchanged.
   Expected count of removed row kinds: 1 to 50.
5. **Timing:** no measurable change (within run-to-run noise, about 2 %).
6. **Lean:** none owed; the change moves parity-only bookkeeping and touches
   no decision table, transfer or verdict.

A verdict move of any kind would falsify (1) and would be attributed by
reading and named; a Safe → non-Safe move would be a correction.
7. **The audit's other fixes** (thread-local and cell state found leaking
   in `crates/qcore-c-gateway`): each predicted verdict-neutral and
   parity-neutral on the measured routes; any that moves a row is reported
   with its attribution.
