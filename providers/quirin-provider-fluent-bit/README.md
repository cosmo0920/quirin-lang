# quirin-provider-fluent-bit

Library crate `quirin-provider-fluent-bit` (Rust library `quirin_provider_fluent_bit`) in the separate
`providers/` workspace. It owns the shared Quirin contract modules for
Fluent Bit application code (input chunks, routing, hash tables, logging, memory, strings, time), under `contracts/fluent_bit/` (13 modules). Two of them take their contracts from published `quirin-provider-libc` modules by `#include` and `contract inherit` (card CONTRACT-INHERIT-ADOPTION), through the crate's Cargo dependency on that crate. These are canonical
sources, not copies: the [corpus index](../../contract/corpus.json) names this
crate's `contracts/` as a root and rejects duplicate logical names. The crate is
data only; it links no provider runtime and is not an executable package.

`CONTRACTS` lists every owned module as `(logical path, source text)` and
`contract(path)` looks one up; a unit test pins the index to the tree on disk.
[`quirin-provider-shared`](../quirin-provider-shared/README.md) re-exports this
crate as part of the complete shared set. Consumers still select modules
explicitly; see [corpus conventions](../../contract/README.md).

Fluent Bit's busy-state API is provider-owned by `qcore-provider-chunkio` (`fluent_bit/busy_state.qcore`) and deliberately not duplicated here; the corpus index refuses a duplicate logical name.
