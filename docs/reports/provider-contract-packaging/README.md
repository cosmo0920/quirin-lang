# Provider-owned Quirin contracts

Base: `10aafcf8`. Status: implementation and validation in progress.

The user prioritized repository structure before the remaining Cycle 18
production acceptance work. The five provider crates now own 72 Quirin contract
modules under their `contracts/` directories: ChunkIO 44 (including Fluent Bit's
`busy_state`), Libevent/Monkey 15, librdkafka 8, CMetrics 4 and CTraces 1.
Their `contracts.json` files enumerate package-relative paths and SHA-256 values.
Shared language, support-library and application policies without a provider owner remain in the shared corpus.

## Ownership and compatibility

`providers/contracts.py` is the ownership map and synchronization entry point.
`sync` copies canonical provider sources to the retained top-level corpus and
regenerates inventories; `check` writes nothing and refuses missing, stale,
changed or unowned files. Orphaned compatibility contracts require deliberate
removal and corpus pin updates; synchronization never deletes them silently.
Symlink inventories cannot redirect synchronization writes.

The [byte comparison](contract-parity.json) measures every canonical contract
against the base and its retained `contract/qcore/` snapshot: all 72 are exact.
These snapshots preserve the existing in-process routes, pinned resource
manifest, source paths and core build independence from the optional providers
workspace. There are still 96 logical corpus modules, not 168. Ownership moved;
contract predicates, declarations, clause counts and Lean semantics did not.
Existing Rust and audited Lean verification models remain applicable unchanged.

A GPT-5.6 Sol agent at medium reasoning implements package distribution and
verification in an isolated worktree. The integrator owns the canonical assets,
compatibility checks, documentation, review, complete gate and publication.

## Acceptance boundaries

Packaging contracts does not grant a provider trust or additional capabilities.
Contracts remain declared premises; the independent checker retains its existing
obligations and refusal behavior. This structural change closes none of the
production callback, cancellation, event-generation or foreign-host acceptance
gaps. The ten proposed Priority 1/2 task candidates remain the next semantic
workstreams after this packaging priority.

Cargo's own `package --list --allow-dirty --offline --manifest-path …` was run
for each artifact crate. All five exits were 0 and every owned contract plus
its inventory was included; [the counts](cargo-package-lists.json) are
44/4/1/15/8. This checks the source-package inventory, not crates.io publication
or an independently buildable Cargo archive (these crates remain unpublished
and use the existing workspace/shared-source layout).
