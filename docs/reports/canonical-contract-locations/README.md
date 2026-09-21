# Canonical contract locations

The user requested complete removal of `contract/qcore` after provider ownership
landed, then requested that shared contracts also live under providers.
This migration supersedes the compatibility-copy arrangement recorded in
`provider-contract-packaging`; that historical report remains unchanged.

## Source ownership and boundaries

All 96 logical modules have one canonical source: 72 in the five executable
provider crates and 24 in `providers/shared/contracts`. The shared directory is
a data bundle, not a new executable provider. `contract/corpus.json` selects the
six roots for validation, rejecting duplicate logical names. The 96-module and
per-declaration/clause pins remain intact. The resource selector still chooses
33 modules / 59 declarations, and the other 207 declarations remain rejected by
the resource interpreter rather than silently skipped.

The [migration ledger](migration.json) compares every source against main at
`deb4035340205b5eb1e44595256d3cc11bbd0d3f`, the base before this task. Each entry
records the old path, final canonical path and unchanged SHA-256. No Quirin
predicate or Lean definition changed. No analysis route was deleted. The old
directory contains neither files nor compatibility symlinks; inventory checks
reject its reintroduction. `providers/contracts.py sync` updates only provider
inventories. The five provider package inventories continue to contain 72 files;
shared modules are explicitly selected data, not implicitly installed by every
executable package.

The aggregate `cycle19-resource-contracts.json` moved to the repository root so
all 33 selected paths remain relative without traversal. The runtime loader's
strict path checks were not weakened. Profile bindings, compile-time includes,
native-observation scripts, CI path filters and corpus discovery now point at
the canonical assets. Missing inputs still refuse loading/building.

Retained integration adapters in `qcore-verification` and `compiler` now embed
provider-owned data directly. This is a deliberate layout boundary change from
the prior full-workspace “zero providers present” requirement: these adapters
and their integration tests require the data, but add no Cargo dependency on
provider executable crates. Five generic production libraries (schemas, core,
frontend, contract and verifier host) build with no provider or contract
directories. The C gateway additionally requires the shared source-projection
policy; it builds with only the shared bundle and no executable provider crates. This does not claim that their integration tests are
asset-independent or that individual Cargo source packages are self-contained.

## Identity and evidence

Contract source bytes and logical corpus pins are unchanged. Physical source
keys, resource selection text, profile digests and dependent certificate/report
identities may legitimately change. Raw identities are not rewritten to fake
byte parity. The artifact comparison tool maps canonical shared/provider paths
into its existing `qcore/<logical>` comparison namespace, while retaining raw
keys and collision refusal. Historical artifact comparisons preserve their old
normalization behavior; frozen reports and artifacts remain unchanged.

Validation and the five-artifact corpus comparison are recorded below when
complete. Incomplete first measurements were stopped by their identified
process groups after the user's shared-location correction; their logs remain
under `private/canonical-contract-locations/{core-gate,provider-gate,corpus}` and
are excluded from final counts. The earlier focused compile failure (obsolete
test-local root variables) and comparison-test assertion failures are retained
beside their successful corrected runs. The initial six-library no-assets
check failed on the gateway shared policy; the corrected five-library no-assets
and gateway shared-only checks establish the narrower boundary. None is presented as a passing gate.

Two GPT-5.6 Sol agents at medium reasoning independently reviewed consumers and
implemented inventory migration. Root integrated the canonical layout, source
identity comparison, documentation and final measurements.

## Integrator reconciliation list

1. Keep provider inventories authoritative, reject a recreated `contract/qcore`,
   and retain all 96 logical module/clause pins through the canonical index.
2. Preserve strict resource-manifest traversal rejection by placing the aggregate
   selector at the repository root and updating profiles, rather than widening
   the loader's accepted path language.
3. Document the integration-adapter data dependency separately from generic
   production-library independence; do not claim the full workspace tests with
   provider assets absent.
4. Record source-byte and verdict parity separately from path/digest identity
   changes, preserve frozen evidence, and leave broader Cycle 18 follow-ups open.

## Corpus and package parity

The pinned 185-unit run exits 0. Its resource diagnostics are **byte-identical**
to the frozen W42 baseline: no function or unit verdict, coverage count or
resource diagnostic changes. The other four artifacts retain their raw identity
changes. The [exhaustive diff](corpus-diff.json), [field summary](corpus-identity-summary.json)
and [bounded attribution checks](corpus-parity.json) distinguish these:

- Source and chunk-contract artifacts differ only in profile digests (19 and 3
  changed leaves respectively).
- The contract artifact retains exactly 33 identical source texts; its 68 raw
  differences are 33 removed/added physical keys, manifest paths and one profile
  digest.
- Source-effects retains 93 exact results; three results change only identity
  fields and bound obligation identifiers. All bounds, premises, verdicts and
  non-identity evidence agree. There are 33 raw changed leaves, explicitly
  enumerated rather than discarded wholesale.

`python3 docs/reports/canonical-contract-locations/verify_parity.py
private/canonical-contract-locations/corpus-final` checks those precise
boundaries. This is measured equivalence under the pinned inputs, not a
source-extraction or Lean-to-Rust correspondence proof. The frozen baseline is
unchanged; later provider/cycle integrations had already measured it byte-exact.
All five rebuilt package identity records, including executable, manifest,
contract inventory and archive hashes, match the previous root-checkout build
([package parity](package-parity.json)).
