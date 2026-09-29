# W47-PREMISES-HOLDER-REGION: frozen prediction

Frozen after BEFORE was measured and before any AFTER census, facts
derivation, fixture run or Lean change. Disclosure: a first draft of the two
Rust modules (`loop_owner_region_macros.rs`, `loop_owner_holder_opaque.rs`),
their hooks and the `region_macro_facts` example exists and compiles; it has
not been run on the export, the previous test or any fixture.

Base: local `main` at `fe603d49` (the worktree's HEAD).

## BEFORE (measured)

Base binaries (`private/hr/bin-before/`: `loop_owner_census` `62e74e04…`,
`typed_dispatch_facts` `4b7655a4…`, `predefined_macro_facts` `9923a0cc…`),
facts regenerated: all six census readings are byte-identical to the
W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS archive — `90978ac6` configured
`719a7d0d…` (11 / 130 / 7), raw `983c74b9…`, configured control `4955a12d…`;
`~1` configured `418a9251…` (8 / 136 / 7), raw `d05ebc0e…`, control
`a5edf1ed…`; predefined facts `9dd8bb73…` (both).

## Mechanism (planned)

1. **Region macros.** A new facts derivation runs each list-head region
   file's recorded command (the test as the extra unit, `src/flb_lib.c`,
   `src/flb_config.c`, `src/flb_input.c`) under Clang's token dump and records
   every outermost macro invocation of the unit's own file with its whole
   expansion (each token marked replacement-list or argument) and the unit's
   own function definitions. Step B reads every invocation on the region's
   lines from that expansion: names are resolved in the unit (`TEST_CHECK` →
   `acutest_check_` of `tests/lib/acutest/acutest.h`, never `lib/monkey`'s);
   a name that reaches a hazard, an unresolvable invocation, a replacement
   list's jump, field dispatch or listed-member use refuses. The region's own
   functions' typed indirect calls become unmodelled calls (they can hide a
   macro-made call). Without the facts the reading is unchanged.
2. **P-HOLDER-OPAQUE.** Every read of `struct flb_input_instance::data` (typed;
   lexical in a function without a typed reading) is followed through locals,
   direct calls and field dispatch into registrants' parameters; a carrier
   outside the test's file may only be compared, read through or passed on.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| region facts | every selected unit derives with the recorded Clang; the test's `TEST_CHECK`/`TEST_MSG` resolve to `tests/lib/acutest/acutest.h` | 80% |
| region macros at `90978ac6` configured | no invocation names or carries a hazard-reaching function; no region-own indirect call; the fixed test's 3 sites stay **discharged** | 65% |
| if the region refuses | through `flb_error`/`flb_errno` (`flb_log_print`) reaching a hazard in the over-approximated reach, or an unmodelled call reaching the escaped functions | 25% |
| P-HOLDER-OPAQUE | derived: readers `flb_input_instance_init`, `input_thread` (the `cb_init` dispatch), `cb_emitter_init`, `cb_emitter_pause`, the test's two callbacks; every `cb_init` registrant leaves its `data` parameter unwritten | 60% |
| if P-HOLDER-OPAQUE fails | on a `cb_init` registrant that passes or stores its `data` parameter (a classifier refusal to inspect, not a finding by itself) | 35% |
| old test (`~1`), configured | 8 sites **not discharged**, no site moves | 97% |
| other sites, both revisions | no change | 85% |
| raw readings | byte-identical but the instance flag's reason text (holder note) | 80% |
| edges / functions / seeds | unchanged (no graph change) | 90% |
| corpus / system / slice | byte-identical sidecars (the census is on no audit route) | 95% |
| follow-up `W47-LOOP-OWNER-REGION-MACRO-CALLS` | amended, not retired: the region is derived, the census graph's macro-made calls stay a named residual | 60% |
