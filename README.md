# QCore-C

> **Handoff, 2026-09-07:** managed subagents were halted at the user's request;
> do not restart them without a new user instruction. Full text and the current
> implementation checkpoint: [DEVELOPMENT.md](DEVELOPMENT.md#handoff-2026-09-07)
> and [the handoff record](docs/HANDOFF_2026-09-07.md).

QCore-C is a research language for **quantitative types**, **owned
resources**, **typed recoverable errors**, **checked cleanup**, and **checked
concurrency**, lowering to portable C11. Alongside the language, the project
builds a conservative, fail-closed checker for *existing* C source — the
"unrestricted-C gateway" — currently exercised against a pinned Fluent Bit
corpus.

Two things live here, and they are useful to keep apart:

- **The language.** A supported executable subset compiles through `qcorec`
  to C11 with a tagged-error ABI and explicit cleanup.
- **The gateway.** A checker that reads unmodified C, models what it can, and
  answers `Unknown` for everything it cannot — it never guesses in the
  optimistic direction.

## What the type system tracks

QCore-C deliberately keeps separate quantitative domains separate instead of
overloading one number:

- **Usage grades** — how many times a value is demanded, as intervals.
- **Static indices and dimensions** — numeric type parameters with nominal
  units (`Bytes`, `Count<Fd>`, `Steps`, …); cross-unit arithmetic is rejected.
- **Quantitative credit** — linear, spendable capacity.
- **Live resource obligations** — what must be released, rolled back, or
  transferred; never weakened.
- **Cost summaries** — upper bounds, not tokens.
- **Effect rows and per-exit post-states** — what a function may do and what
  remains at each exit, including typed error exits.

The supported executable source subset lowers to C11 with a tagged-error ABI
and explicit cleanup. Translation checks and native differential runs cover
that reduced model; broader structural syntax and separately checked semantic
models do not imply complete source-language support.

The Rust kernel exposes its checked concurrency models through
`qcore_core::prelude::concurrency`, with a matching bounded Lean proof entry point,
`QCore.ConcurrencyPrelude`. The [thread examples](examples/threads/README.md#mapping-source-examples-to-the-core-prelude)
show how source opts in with `import std.concurrency;`; concurrency is unavailable
without that file-local import. General task/mutex composition remains open.

## Status at a glance

This is a research project that advances in bounded, individually accepted
**cycles** ([`docs/cycles/`](docs/cycles/)). Results are scoped claims with
recorded evidence, not general guarantees.

| | |
|---|---|
| Language | Cycles 1–17 complete at their stated milestones. **Full language conformance is not claimed**; revision 0.2 remains historical. |
| Event reactor (Cycle 18) | Bounded implementation recorded; aggregate acceptance pending. |
| Existing-C gateway (Cycle 19) | In progress against a pinned Fluent Bit corpus. |
| Verifier plugins (Cycle 20) | Active: versioned protocol, fail-closed registry, bounded process providers. |

The latest raw Cycle 19 measurement over 185 translation units is
**2 Safe / 42 provisional Unsafe / 137 Unknown**, plus four data-only units.
**Raw units remain unpromoted**: a translation unit is not promoted to `Safe`
until per-path identity imports and real observation evidence satisfy the
cycle's fail-closed gates. A provisional `Unsafe` is never presented as an
upstream defect. Coverage is not safety coverage, and unknowns stay in the
denominator.

Verdict vocabulary, invariants, per-increment evidence and the full cycle
narratives are in [DEVELOPMENT.md](DEVELOPMENT.md) and
[`docs/reports/`](docs/reports/).

## Quick start

The Rust compiler and `qcorec check` / `qcorec emit` do not require Clang.
Compiling emitted C requires a C11 compiler. Native scoped task and checked
mutex emission runs on Linux C11 threads, macOS pthreads, and Windows through
an emitted winpthreads-derived compatibility subset. Emitted C is self-contained;
Windows builds need no pthread library. See the [Windows thread guide](docs/reports/windows-c11-threads/README.md). The full test suite also
exercises existing-C analysis, which needs Clang for preprocessing, syntax
validation, and its JSON AST. See the [Windows Clang fixture validation](docs/reports/windows-clang-fixtures/README.md)
for target types, pinned source bytes and platform test evidence.

```sh
git submodule update --init --recursive
cargo test --workspace
python3 scripts/check_lean_audits.py
```

Try the CLI:

```sh
cargo build --bin qcorec
./target/debug/qcorec check examples/cycle12.qc
./target/debug/qcorec emit -o generated.c examples/cycle12.qc
```

For native multithreading, see the [runnable examples](examples/threads/README.md).
From Visual Studio Developer PowerShell at the repository root, build and run
all three with:

```powershell
.\examples\threads\build-windows.ps1 -Run
```

The standing per-change gate — the full command list every change must pass,
including formatting, Clippy and both reevaluation probes — is in
[`AGENTS.md`](AGENTS.md). Windows setup, Clang discovery order and CI job
layout are in [DEVELOPMENT.md](DEVELOPMENT.md#build-and-test).

## Documentation map

| Read this | For |
|---|---|
| [`AGENTS.md`](AGENTS.md) | Working here as an AI agent: compressed status ledger, hard invariants, standing per-change gate |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Implementation and status detail: repository layout, cycle records, per-increment evidence, full build/validation instructions |
| [`docs/QCORE_SPEC_r0_3.md`](docs/QCORE_SPEC_r0_3.md) | The normative specification, with [contract semantics](docs/QCORE_CONTRACT_SPEC_r0_3.md) and [implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) |
| [`docs/GENERALIZATION_PLAN.md`](docs/GENERALIZATION_PLAN.md) | Where the project is going: generalizing the checker away from pinned source sets, decomposing the crate |
| [`docs/reviews/r0_3/REVIEW.md`](docs/reviews/r0_3/REVIEW.md) | Implementation acceptance ledger for revision 0.3 |
| [`docs/cycles/`](docs/cycles/) and [`docs/reports/`](docs/reports/) | Per-cycle requirements and archived evidence packages |
| [`editors/emacs/`](editors/emacs/README.md) | `qcore-mode` syntax highlighting for `.qcore` / `.qc` (lexical support only) |

Further reading: [module modes and index syntax](docs/QCORE_MODULE_MODES.md),
[QTT alignment](docs/QTT_ALIGNMENT.md),
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md),
[reevaluation](docs/REEVALUATION_SPEC.md) and its
[follow-up](docs/REEVALUATION_FOLLOWUP.md).
