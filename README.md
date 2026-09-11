# Quirin

**Quirin** is a research language for **quantitative types**, **owned
resources**, **typed recoverable errors**, **checked cleanup**, and **checked
concurrency**, lowering to portable C11. Alongside the language, the project
builds a conservative, fail-closed checker for *existing* C source — the
"unrestricted-C gateway" — currently exercised against a pinned Fluent Bit
corpus.

> **Naming.** The language is **Quirin**; its command is `quirin` (with
> `quirin-audit` for configured audits) and its sources are `.qcore` files. The
> core component keeps the `qcore` spelling: Rust crates and namespaces
> (`qcore-compiler`, `qcore_core`, …), the emitted-C surface (`QCORE_IO_WRITE`,
> `qcore_task_*`, `qcore_buffer_*`), the diagnostics codes and the `.qcore`
> extension are all unchanged, and the repository is still `qcore-c`. Only the
> product name and the installed commands moved; the `.qc` alternative source
> extension is retired.

Two things live here, and they are useful to keep apart:

- **The language.** A supported executable subset compiles through `quirin`
  to C11 with a tagged-error ABI and explicit cleanup.
- **The gateway.** A checker that reads unmodified C, models what it can, and
  answers `Unknown` for everything it cannot — it never guesses in the
  optimistic direction.

## What the type system tracks

Quirin deliberately keeps separate quantitative domains separate instead of
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

## What source can say today

The executable source subset grows one **admitted profile** at a time, each
with its own grammar in
[implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) and its own
diagnostics band. Everything outside an admitted profile is refused, not
guessed. Capabilities are opt-in per file through the `std` module family:

| Import | Gives the file |
|---|---|
| `std.concurrency` | scoped tasks, checked mutexes, epoch bounds, disjoint footprints |
| `std.fmt` / `std.io` / `std.logger` | rendered `print`, `log`, `flush` lines and declared log levels |
| `std.extern` (plus `std.extern.unsafe`) | bounded declared extern calls; adding the second import is explicit unchecked linkage, scalars only, every pointer refused |
| `std.buffer` | the owned, extent-carrying byte buffer |
| `std.string` | immutable UTF-8 `{ptr, len}` string views |
| `std.event` | the checked event/reactor surface over descriptor custody |

Also admitted at source level: the `|>` pipe operator, `if`/`else` branching
with a decidable condition tier, `extern library NAME;` native-library binding,
and several composed profiles (extern × rendered lines, tasks × rendered lines,
events × rendered lines). Concurrency is unavailable without its file-local
import, and the checked concurrency models are exposed from the Rust kernel as
`qcore_core::prelude::concurrency` with a matching bounded Lean entry point,
`QCore.ConcurrencyPrelude` — see the
[thread examples](examples/threads/README.md#mapping-source-examples-to-the-core-prelude).

Translation checks and native differential runs cover this reduced model.
Broader structural syntax and separately checked semantic models do **not**
imply complete source-language support, and general task/mutex composition
remains open.

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

`quirin check` / `quirin emit` do not require Clang; compiling emitted C
requires a C11 compiler. Native scoped task and checked mutex emission runs on
Linux C11 threads, macOS pthreads, and Windows through an emitted
winpthreads-derived compatibility subset. Emitted C is self-contained, so
Windows builds need no pthread library — see the
[Windows thread guide](docs/reports/windows-c11-threads/README.md). The full
test suite also exercises existing-C analysis, which needs Clang for
preprocessing, syntax validation and its JSON AST
([Windows Clang fixture validation](docs/reports/windows-clang-fixtures/README.md)).
On macOS, the configured audit preserves Apple Clang architecture and
undefined-prefix warning options across all three gateway phases
([macOS Cycle19 compiler-option replay](docs/reports/macos-cycle19-compiler-options/README.md)).

```sh
git submodule update --init --recursive
cargo test --workspace
python3 scripts/check_lean_audits.py
```

Try the CLI:

```sh
cargo build --bin quirin
./target/debug/quirin check examples/cycle12.qcore
./target/debug/quirin emit -o generated.c examples/cycle12.qcore
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
| [`AGENTS.md`](AGENTS.md) | Working here as an AI agent: compressed status ledger, hard invariants, standing subsystem constraints, per-change gate |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Implementation and status detail: repository layout, cycle records, per-increment evidence, full build/validation instructions |
| [`docs/QCORE_SPEC_r0_3.md`](docs/QCORE_SPEC_r0_3.md) | The normative specification, with [contract semantics](docs/QCORE_CONTRACT_SPEC_r0_3.md) and [implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) |
| [`docs/GENERALIZATION_PLAN.md`](docs/GENERALIZATION_PLAN.md) | Where the project is going: generalizing the checker away from pinned source sets, decomposing the crate |
| [`docs/reviews/r0_3/REVIEW.md`](docs/reviews/r0_3/REVIEW.md) | Implementation acceptance ledger for revision 0.3 |
| [`docs/cycles/`](docs/cycles/) and [`docs/reports/`](docs/reports/) | Per-cycle requirements and archived evidence packages |
| [`editors/emacs/`](editors/emacs/README.md) | `qcore-mode` syntax highlighting for `.qcore` (lexical support only) |

Further reading: [module modes and index syntax](docs/QCORE_MODULE_MODES.md),
[QTT alignment](docs/QTT_ALIGNMENT.md),
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md),
[reevaluation](docs/REEVALUATION_SPEC.md) and its
[follow-up](docs/REEVALUATION_FOLLOWUP.md).
