# quirin-provider-libc

Library crate `quirin-provider-libc` (Rust library `quirin_provider_libc`) in the separate
`providers/` workspace. It owns the shared Quirin contract modules for
the C standard library and POSIX (`errno`, heap memory, `pthread_mutex`), under `contracts/libc/` (3 modules). These are canonical
sources, not copies: the [corpus index](../../contract/corpus.json) names this
crate's `contracts/` as a root and rejects duplicate logical names. The crate is
data only; it links no provider runtime and is not an executable package.

`CONTRACTS` lists every owned module as `(logical path, source text)` and
`contract(path)` looks one up; a unit test pins the index to the tree on disk.
[`quirin-provider-shared`](../quirin-provider-shared/README.md) re-exports this
crate as part of the complete shared set. Consumers still select modules
explicitly; see [corpus conventions](../../contract/README.md).

This crate is the **only** home for libc contracts: a new libc module is added under `contracts/libc/` here and registered in `CONTRACTS`, never under `fluent_bit/` or another corpus module. It has no dependency and is usable on its own, without the `quirin-provider-shared` bundle.
