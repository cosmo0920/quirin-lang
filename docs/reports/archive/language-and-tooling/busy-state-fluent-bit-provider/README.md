# BUSY-STATE-FLUENT-BIT-PROVIDER: Fluent Bit's busy-state contract moves to the Fluent Bit provider

User decision 2026-11-17: `fluent_bit/busy_state.qv` is a Fluent Bit contract
and belongs to `providers/quirin-provider-fluent-bit`, not to
`qcore-provider-chunkio`. This is an ownership move only: no contract text,
clause, analyzer crate, rule, specification section or Lean module changes.

## The move

- `git mv providers/qcore-provider-chunkio/contracts/fluent_bit/busy_state.qv
  providers/quirin-provider-fluent-bit/contracts/fluent_bit/busy_state.qv`
  (`1eb180a09`).
- `providers/contracts.py`: `qcore-provider-chunkio` now owns `("chunkio",)`
  only; `providers/contracts.py sync` regenerated
  `providers/contract-load-path.json`, both crates' `contract-inventory.lock`
  and the chunkio `contracts.json`. The entry moved byte-identically
  (sha256 `d5312a86…`, 1 declaration, 7 clauses): the chunkio lock loses it
  and the Fluent Bit lock gains it.
- The Fluent Bit data crate's `CONTRACTS` index (`src/lib.rs`) lists the file,
  so `tests::index_is_exactly_the_contract_tree` holds (`e0e85ba4a`).
- Path readers updated: `contract/profiles/fluent-bit/source-contracts.json`,
  `crates/qcore-contract/tests/boolean_conditions.rs`,
  `crates/quirin-source-analysis/tests/unit/chunk_contract.rs`, the chunkio
  provider's module doc, `providers/README.md`, the Fluent Bit provider README
  and the Fluent Bit coverage plan.
- Frozen reports and `docs/cycles/CYCLE_19.md` keep the old path as their
  checkpoint claim.

## Verdict effect

None by construction: the contract load path is the union of the provider
crates' trees, the declaration text and its digest are unchanged, and only the
owning crate differs. The full landing gate below passed with no pinned figure
changed.

## Landing

Integration record: [integration/gate.json](integration/gate.json) (raw scoper
record `integration/gate-scope.json`).
