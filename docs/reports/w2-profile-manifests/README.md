# W2: bounded source-profile migration

Date: 2026-09-07. This implements a bounded subset of the approved
[W2 design](../../design/W2_PROFILE_MANIFEST_SCHEMA.md). W2 remains open for
provider registration, typed import tables and configured audit orchestration.
Profiles are declared trusted assumptions, never verified upstream behavior.

## Implemented scope

- Published schema v1 in `qcore-schemas::profile`; total-or-reject loading,
  provider manifest consistency, deterministic ordering, pin/clean-tree checks,
  mandatory SHA-256 file identities and source-profile assumption records in
  `qcore-contract::profile_loader`.
- H1 source inventories and construct witnesses are declared in Fluent Bit and
  Libevent profile directories. The public source-summary derivation rechecks
  each digest independently, including directly constructed source bundles.
- H2 replaces `AuditedLibrary`, per-library permitted mechanism sets and the
  operation-to-summary name table with explicit profile data. Missing operation
  entries reject an import; they do not erase requirements. Monkey callback
  and composition tables remain in their existing bounded provider model.
- H3 moves the Libevent revision, twelve audited files, construct witnesses,
  API disposition table and build matrix into data. The event checker takes
  explicit accepted-build selectors. Unknown symbols, empty matrices and
  unrecognized forbidden build features cannot become accepted.
- H7 uses declared pin/clean-path/source inventories for the existing audit.
  Reports print profile identity/trust assumptions, and the optional contract
  sidecar includes the source-profile identity.

`QCORE_PROFILES` overrides the in-tree profile root; a supplied value never
falls back to compiled tables. Provider manifests are validated and loaded as
schema records, but registration still uses the existing first-party literals.

## Deliberate remaining work

H4 typed Monkey import tables and callback summaries, H5 Boolean field importer
routing and its revision constant, H6 first-party registry literals, and the
H7 source selection/orchestration driver remain. H5/H6 equality guards prevent
declared data drifting from the retained code. Monkey binding/observation
contract text is retained under design Q5. This is not W2 or W5 completion,
and does not claim third-party providers are yet standalone processes.

The schema records optional resource-contract and provider bindings because
some audited libraries have neither today; inventing bindings would be false.
The design's alleged 48-entry Libevent API table actually has **45 entries:
37 Accepted and 8 Unknown** in baseline commit `43040e7`. The exact old/new
symbol map is archived in `api-disposition-parity.json`; no symbol was added.
All 60 backend/thread/OpenSSL/debug combinations preserve the baseline matrix.

## Parity before deletion

Baseline: `43040e7d7ed006bc8a98b8987d7e0f2d3974c929`, reconstructed from a clean
`git archive` in `/tmp/qcore-w2-baseline-43040e7`, with the existing pinned
Libevent checkout made available. Both baseline and W2 ran:

```sh
cargo run -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/w2-<baseline-or-after>-source.json \
  /tmp/w2-<baseline-or-after>-diagnostics.json
```

Both exit **0**, covering all **185 translation units** including the 29-unit
forced-Libevent Monkey leg. Entire diagnostic JSON and Monkey source JSON are
**byte-identical**. Counts: **3 Safe / 98 provisional Unsafe / 80 Unknown /
4 data-only**, with zero verdict or diagnostic movement. The extra Safe
relative to the original Cycle 19 triage was already present in the W4
baseline; W2 promotes nothing. All raw production units remain unpromoted
under the Cycle 19 acceptance gates.

`parity.json` records hashes, counts and zero deltas; compressed original and
new diagnostic/source JSON and audit logs accompany it. The before/after
Libevent source audit also exits 0 with the same revision and twelve checked
paths; its only output addition is explicit profile assumptions. Digest checks
strengthen identity validation; the three public-header witnesses retain
substring needles with rationales.

## Verification

All required gates passed on this branch:

- recursive pinned submodule initialization;
- `cargo test --workspace`: **454 tests passed**;
- `cargo fmt --all -- --check`;
- `cargo clippy --all-targets -- -D warnings`;
- both reevaluation examples: exit 0;
- `proof/` `lake build`: 52 jobs, success.

Logs are archived here. Tests additionally reject a directly fabricated source
bundle (the draft had incorrectly trusted loader provenance), unknown schema
fields/versions, missing files/pins/providers, digest drift, missing rationale,
operation conflicts and unknown build exclusions. Semantic kernel transition
rules are unchanged; Lean remains unchanged and builds cleanly.
