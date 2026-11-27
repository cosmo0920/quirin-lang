# CG30 type-identity fixture correction

`routes_g_and_t_read_the_identity_clang_prints` failed on Clang 21.1.8:
route G returned `CTypeId("unsigned long")`, while the lexical ABI reader
returned `None`. The failing fixture used Clang's predefined `__SIZE_TYPE__`
macro in the typedef chain and expected the lexical token reader to expand it.
That reader intentionally has no preprocessor environment. The CG30 card's
recorded agreement claim uses the explicit chain `my_size` → `size_t` →
`long unsigned int`; the fixture now spells that chain and its second formal
explicitly. No production analysis, contract, proof, verdict or admission rule
changes. `cargo fmt --all` also normalized seven already-unformatted files at
the current main tip; those edits are mechanical only.

The focused test reproduced the original assertion failure (exit 101) and
passes after the fixture edit (exit 0, one test). The first standing Rust run
under the sandbox exited 101 with 8,243 passed / 108 failed / 15 ignored;
its retained per-unit logs include LeakSanitizer's ptrace restriction and
process-confinement failures. The unchanged host-access rerun passed with
**520 units / 533 summaries, 8,351 passed / 0 failed / 15 ignored**. The first
format check exited 1 on the preexisting formatting; after normalization,
`cargo fmt --all -- --check` exits 0. Clippy, both reevaluation examples,
344 Lean audits and 374 Python tests exit 0; the Python suite also passes
after the report-index update. Exact commands, exits and private logs are under
`private/cg30-type-identity-fixture/{gate-1,gate-host-2,closeout-1}/`;
the workspace runner's per-unit logs are under the paths printed in each
`rust.log`. No corpus audit is claimed for this test-only correction.
