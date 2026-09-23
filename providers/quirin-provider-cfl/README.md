# quirin-provider-cfl

Library crate `quirin-provider-cfl` (Rust library `quirin_provider_cfl`) in the separate
`providers/` workspace. It owns the shared Quirin contract modules for
the CFL support library (arena and SDS lifetimes, intrusive lists, time), under `contracts/cfl/` (4 modules). These are canonical
sources, not copies: the [corpus index](../../contract/corpus.json) names this
crate's `contracts/` as a root and rejects duplicate logical names. The crate is
data only; it links no provider runtime and is not an executable package.

`CONTRACTS` lists every owned module as `(logical path, source text)` and
`contract(path)` looks one up; a unit test pins the index to the tree on disk.
[`quirin-provider-shared`](../quirin-provider-shared/README.md) re-exports this
crate as part of the complete shared set. Consumers still select modules
explicitly; see [corpus conventions](../../contract/README.md).
