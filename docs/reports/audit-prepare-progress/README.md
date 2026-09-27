# Audit preparation progress

Base: `2e44e963`. Preparation already reconstructs an absent Clang source/build tree from the upstream checkout. This change prints checkout/clone/Monkey stages and subprocess start/completion messages on stderr, including live log paths and elapsed time. Standard output and the prepared input identities remain unchanged. No verdict or coverage movement is expected; no analysis rule changes or new proof claim.

CMake stdout/stderr now go to files while the process runs. Clang configuration/build retain merged output. Monkey retains its historical final stdout-then-stderr ordering: during execution stderr has a separate `configure.stderr.log`, then it is appended to `configure.log` and the temporary sibling is removed. Completed logs retain strict UTF-8 and universal-newline conversion. An interruption leaves available raw subprocess output on disk; child-side buffering can still delay individual lines.

## Recovering a lost preparation tree

From the repository, run:

```sh
cargo run --release -p qcore-audit -- prepare \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision HEAD \
  --clang /usr/lib/llvm-21/bin/clang \
  --clangxx /usr/lib/llvm-21/bin/clang++
```

This creates the independent source clone, Clang CMake database/generated inputs, CMetrics static library and Monkey input tree under `build/cycle19/clang/<revision>/`. It does not build the entire Fluent Bit application. For an incompatible or partially managed output that is refused, pass `--output` naming a fresh directory; do not delete a tree another audit is using. Add `--run --jobs 4 -- --checkpoint-dir tmp/audit-checkpoints --resume` to chain checkpointed auditing. The separator forwards those last options to `current`.

A `prepare` parent waiting on a CPU-active `current` child has finished preparation. Per-file checkpoint progress belongs to the companion checkpoint card. Missing input files still refuse or remain Unknown under existing rules; progress output supplies no evidence of safety.

## Validation

The regression test blocks a fake subprocess until both live streams have been read, then releases it to exit 7. It checks failure propagation and exact completed log ordering/newline conversion. Existing preparation integration tests cover managed input reuse and refusal. `cargo test -p qcore-audit --lib prepare::tests -- --nocapture` exited 0 (4 passed) on the integrated checkpoint branch. The [combined integration gate](../audit-checkpoint-files/integration/README.md) passed every step; it records 6,059 Rust tests, 365 Python tests, 213 Lean audits, prior failures and exact private log paths.
