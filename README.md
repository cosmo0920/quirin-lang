# Quirin

Quirin is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup and concurrency, lowering to C11. It also
provides a conservative checker for existing C, exercised on a pinned Fluent Bit
corpus. Unsupported or unproven behavior stays `Unknown`.

The commands are `quirin` and `quirin-audit`; sources use `.qcore`. Rust crates,
namespaces and the emitted-C API retain the `qcore` spelling. The `.qc` extension
is retired.

## Quick start

```sh
git submodule update --init --recursive
cargo build --bin quirin
./target/debug/quirin check examples/cycle12.qcore
./target/debug/quirin emit -o generated.c examples/cycle12.qcore
```

Checking/emitting Quirin does not require Clang. Compiling emitted C needs a
C11 compiler; existing-C analysis and the full test suite need Clang. See
[build and platform setup](docs/development/build-and-test.md) and the
[complete validation gate](docs/agents/validation.md).

## What the type system tracks

Usage grades, static indices with nominal units, spendable quantitative credit,
live resource obligations, cost upper bounds, and effects with per-exit states
are distinct domains. A work bound is not a wall-clock bound.

## What source can say today

Support is defined per admitted profile in
[implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).
[Runnable examples](examples/README.md) cover threads, buffers, records,
strings, rendered output, extern calls, events, sockets and HTTP. Profile
boundaries matter: a checked model or a successful native run does not establish
full source-language support, protocol conformance or general race freedom.

## Status at a glance

**Full language conformance is not claimed.** Read [current measured status](docs/STATUS.md)
for corpus verdicts, CT mapping, proof and host-scoped native evidence. Unknowns
stay in the denominator; provisional Unsafe findings are not upstream defects.

## Documentation map

**Start at [the documentation hub](docs/README.md).** It separates current
specifications, implementation status, future work and historical evidence.

| Need | Entry point |
|---|---|
| Language and contracts | [Specifications](docs/README.md#specifications) |
| Build, architecture or contribute | [Development guide](DEVELOPMENT.md) |
| Work as an agent | [Agent rules](AGENTS.md) |
| Choose the next card | [Planning](docs/README.md#planning) |
| Find a past card or wave | [Evidence](docs/README.md#evidence-and-history) |
