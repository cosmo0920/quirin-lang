# quirin-provider-fluent-bit

Library crate `quirin-provider-fluent-bit` (Rust library `quirin_provider_fluent_bit`) in the separate
`providers/` workspace. It owns the shared Quirin contract modules for
Fluent Bit application code (input chunks, routing, hash tables, logging, memory, strings, time, the storage backlog, task users, the singleplex queue, the loop-owner collector operations, the engine thread's start, stop and shutdown joins, and the library context's strictly paired start/stop lifecycle (`requires resource_lifecycle`, contract spec §C04.11: a stop only after a successful start, once; its preconditions held against every caller and its body clauses checked against the bodies by the rule 10q census), and the teardown destroyers with the member-path states rule 30 reads at a direct call (`teardown_destroy.qv`, card FB-TEARDOWN-DESTROY-DECLARATIONS; it also hosts, on `flb_config_set_program_name`, the declared `quant cell` protocol of `struct flb_input_thread_instance`'s notification channel, contract spec §C04.6.3, card CELL-PROTOCOL-FB-DECLARATIONS), and the outer contract `flb_external_plugin` on the two plugin loaders (`outer.qv`, outer contract v1, card OUTER-CONTRACT)), under `contracts/fluent_bit/` (26 modules). `storage_backlog.qcore` holds checked specifications (`requires resource_body_checked;`, contract spec §C06.3): declarations of bodies the audit analyses, in force only where the body establishes them (card W45-FB-BACKLOG-CUSTODY). Two of them take their contracts from published `quirin-provider-libc` modules by `#include` and `contract inherit` (card CONTRACT-INHERIT-ADOPTION), through the crate's Cargo dependency on that crate. These are canonical
sources, not copies: the [corpus index](../../contract/corpus.json) names this
crate's `contracts/` as a root and rejects duplicate logical names. The crate is
data only; it links no provider runtime and is not an executable package.

`CONTRACTS` lists every owned module as `(logical path, source text)` and
`contract(path)` looks one up; a unit test pins the index to the tree on disk.
[`quirin-provider-shared`](../quirin-provider-shared/README.md) re-exports this
crate as part of the complete shared set. Consumers still select modules
explicitly; see [corpus conventions](../../contract/README.md).

Fluent Bit's busy-state API is provider-owned by `qcore-provider-chunkio` (`fluent_bit/busy_state.qv`) and deliberately not duplicated here; the corpus index refuses a duplicate logical name.
