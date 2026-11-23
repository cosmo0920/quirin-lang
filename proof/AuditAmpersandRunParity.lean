import QCore.AmpersandRunParity

/-! Card **AMPERSAND-RUN-PARITY-OTHER-READERS**: every reader that decides
address-taking from the unfused token stream reads a run of `&` tokens by its
parity (gateway rule 74e), extending rule 74d's frame reading
(`QCore.NullableLazyConstructor.the_run_ends_in_an_address_iff_odd`, not
restated here).

Four directions are audited.

**The reading.** The parity is maximal munch; an even run is exactly a logical
AND among the non-empty runs.

**The unsound readers.** A run of exactly one `&` missed `p && &x`; the
parity keeps every address that reading saw, so the fix only adds addresses.

**The fail-closed and admitting readers.** Any run differs from the parity
exactly on the even runs, and the parity adds nothing to it.

**The census.** Every listed reader reads the parity at the tip; the call
index calls a callee after any `&` run.

The reader list, each reader's call site and the faithfulness of the Rust
(`ampersand_runs.rs`, bound to `DECISION_VECTORS` and counted against
`READERS`) are trust boundary 28's, pinned by
`crates/qcore-c-gateway/tests/ampersand_run_parity.rs` and the loop-owner
fixtures in `crates/quirin-source-analysis/tests`. -/

-- The reading.
#print axioms QCore.AmpersandRunParity.parity_is_munch
#print axioms QCore.AmpersandRunParity.logical_and_is_the_even_complement
#print axioms QCore.AmpersandRunParity.decision_vectors_hold

-- The unsound readers.
#print axioms QCore.AmpersandRunParity.exactly_one_misses_an_odd_run
#print axioms QCore.AmpersandRunParity.the_parity_keeps_every_single_address
#print axioms QCore.AmpersandRunParity.an_unsound_fix_only_adds

-- The fail-closed and admitting readers.
#print axioms QCore.AmpersandRunParity.the_parity_drops_only_even_runs
#print axioms QCore.AmpersandRunParity.the_parity_is_within_any_run
#print axioms QCore.AmpersandRunParity.a_fail_closed_change_only_drops_even_runs
#print axioms QCore.AmpersandRunParity.a_fail_closed_change_adds_nothing

-- The census.
#print axioms QCore.AmpersandRunParity.every_reader_reads_the_parity
#print axioms QCore.AmpersandRunParity.a_callee_after_an_ampersand_is_called
#print axioms QCore.AmpersandRunParity.the_call_index_only_adds
