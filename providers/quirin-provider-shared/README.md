# quirin-provider-shared

Library crate `quirin-provider-shared` (Rust library `quirin_provider_shared`)
in the separate `providers/` workspace. It holds what the five first-party
provider executables and the conformance fixtures share, and bundles the shared
Quirin contracts. It is not an executable provider and is not packaged by
`providers/package.py`. No core workspace crate may depend on it or on any
other `quirin-provider-*` crate (`compiler/tests/workspace_layering.rs`).

## Rust modules (`src/`)

`runtime` is the bounded protocol transport every provider binary runs. The
schemas-only capability evaluators are `source_effect`, `counter_effect`,
`teardown_effect`, `call_use_effect`, `cleanup_replay_effect`,
`composition_effect` and `reactor_callbacks`. A provider's `main` names exactly
the evaluators it chains into its v2 proposal, so linking this library gives no
binary a capability it does not call. The library depends only on
`qcore-schemas`, serialization and the five data-only contract crates below.
`fixtures/` holds the gateway projection fixtures that its unit tests and the
conformance tests read.

## Shared contract assets

This crate's own `contracts/` owns the four provider-owner-less policies:
`language/quantified_indices.qv`, `library/custody_evidence.qv`,
`library/descriptor_native_observation.qv` (the native descriptor
observation policy, card W43-DESCRIPTOR-NATIVE-CHECK) and
`library/source_projection.qv`. The per-library modules live in their own
crates, which this crate re-exports as `libc`, `cfl`, `mpack`, `msgpack` and
`fluent_bit`:

| Crate | Modules |
|---|---|
| [`quirin-provider-libc`](../quirin-provider-libc/README.md) | `libc/` (14); the only home for libc contracts, usable without this bundle |
| [`quirin-provider-cfl`](../quirin-provider-cfl/README.md) | `cfl/` (6) |
| [`quirin-provider-mpack`](../quirin-provider-mpack/README.md) | `mpack/` (1) |
| [`quirin-provider-msgpack`](../quirin-provider-msgpack/README.md) | `msgpack/` (1) |
| [`quirin-provider-fluent-bit`](../quirin-provider-fluent-bit/README.md) | `fluent_bit/` (12) |

`CONTRACTS` lists this crate's own modules; `CONTRACT_SETS`, `shared_contracts()`
and `shared_contract(path)` cover the complete shared set of 24. These are
canonical sources, not copies of provider-crate contracts. The
[corpus index](../../contract/corpus.json) names each crate's `contracts/` as a
root and rejects duplicate logical names.

Nothing here loads or selects a module implicitly. Consumers select the required
`.qcore` modules explicitly. The repository's
[resource manifest](../../cycle19-resource-contracts.json) selects the resource-v1
subset together with the provider-specific modules. The five executable package
inventories describe their own 72 assets; they do not implicitly select or
install these shared modules. See [corpus conventions](../../contract/README.md).
