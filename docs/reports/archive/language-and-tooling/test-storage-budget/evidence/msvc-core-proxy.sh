#!/bin/bash
# Linux proxy of the Windows msvc-core CI leg's builds (static, CARGO_INCREMENTAL=0
# as the job now sets, the leg's commands with --no-run), clippy included.
set -e
export CARGO_INCREMENTAL=0
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p qcore-verifier-host --test execution_quota --no-run
cargo test --test toolchain_compatibility --test module_modes --test quant_index_contracts --test qcore_contract_corpus --test source_tasks --test windows_threads --test checked_mutex_guards --test std_socket --no-run
cargo test -p qcorec --test cli --no-run
cargo test -p qcore-c-gateway --lib --no-run
cargo test -p qcore-compiler --test qrlib_linkage --no-run
cargo test --workspace --lib --no-run
