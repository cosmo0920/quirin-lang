# quirin-provider-libc

Library crate `quirin-provider-libc` (Rust library `quirin_provider_libc`) in the separate
`providers/` workspace. It owns the shared Quirin contract modules for
the C standard library and POSIX, under `contracts/libc/` (18 modules: `errno`, heap memory, `pthread_mutex`, card W47-CONDVAR-JOIN-PUBLICATION's `pthread_sync` (condition variables and join, each under a thread-synchronization obligation only the census can discharge), card W43-LIBC-CONTRACTS' `dlfcn`, `math`, `process`, `socket`, `stdio`, `stdlib`, `string`, `sys_stat`, `threads`, `time`, `unistd`, card W43-LIBC-RESTRICT-PROTOTYPES' `regex`, and card W43-DESCRIPTOR-RESOURCE-TRACKING's `dirent`; card CELL-GUARD-RESULT-RANGE-CHECKED's `result_sets`, the crate's bounds view: the C library's documented result sets of `pipe`, `epoll_ctl` and `open` as flexible bounds contract v1 value sets under `libc-c-standard-abi`, stating no custody). Every declaration is a trusted assumption about the C library, read from the prototype the pinned units declare; the modules' comments record what each declines and why. These are canonical
sources, not copies: the [corpus index](../../contract/corpus.json) names this
crate's `contracts/` as a root and rejects duplicate logical names. The crate is
data only; it links no provider runtime and is not an executable package.

`CONTRACTS` lists every owned module as `(logical path, source text)` and
`contract(path)` looks one up; a unit test pins the index to the tree on disk.
[`quirin-provider-shared`](../quirin-provider-shared/README.md) re-exports this
crate as part of the complete shared set. Consumers still select modules
explicitly; see [corpus conventions](../../contract/README.md).

This crate is the **only** home for libc contracts: a new libc module is added under `contracts/libc/` here and registered in `CONTRACTS`, never under `fluent_bit/` or another corpus module. It has no dependency and is usable on its own, without the `quirin-provider-shared` bundle.

`libc-names.txt` is the generated list of libc/POSIX function names visible to the pinned corpus (every function a `libc6-dev` or `linux-libc-dev` header declares at file scope in the 185 pinned units; generator `docs/reports/w43-libc-contracts/scripts/libc_names.py`). `scripts/tests/test_contract_corpus.py`, run by the standing gate, fails if any contract module outside this crate — in the canonical corpus or anywhere under `providers/` — declares one of those names or a name this crate declares.
