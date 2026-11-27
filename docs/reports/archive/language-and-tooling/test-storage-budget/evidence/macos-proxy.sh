#!/bin/bash
# Linux proxy of the macOS CI job's builds (static, CI's commands, --no-run).
set -e
cargo test -p qcore-c-gateway --lib --no-run
cargo test -p qcore-compiler --test qrlib_linkage --test ffi_struct_import --test float_ffi --test usable_records --no-run
cargo test -p qcore-compiler --test float_types --test float_extern --test float_tasks --test float_buffer_payloads --test float_ieee_vectors --no-run
