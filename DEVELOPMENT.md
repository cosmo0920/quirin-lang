# QCore-C development notes

This is the development table of contents. Open the page for the task at hand;
historical checkpoint narratives are loaded only when their evidence is needed.
[README.md](README.md) gives the human overview. [AGENTS.md](AGENTS.md) owns
current agent rules and headline status. Each card's report remains the
authority for that card's counts, boundaries and premises.
The [development directory index](docs/development/README.md) is a second
entry point to the same topic pages.

## Table of contents

- [Specifications and records](#specifications-and-records)
- [Repository layout](#repository-layout)
- [Project status](#project-status)
- [Build and test](#build-and-test), including [Windows](#windows)
- [Increment records](#increment-records): current record location and wave index
- [Historical section links](#historical-section-links): original fragment compatibility

## Specifications and records

[Development references](docs/development/references.md) links the normative
specifications, acceptance records, audit setup and editor support.

## Repository layout

[Repository structure and entry points](docs/development/repository-layout.md)
maps crates, contracts, proofs and compiler APIs.

## Project status

[Cycle status and recorded evidence](docs/development/cycle-status.md) retains
the former guide's detailed checkpoint narratives. For the latest headline
measurements, read the [agent status ledger](AGENTS.md#status-ledger-do-not-overstate).
Historical counts are not new conformance or current aggregate gate results.

## Build and test

[Building and validating QCore-C](docs/development/build-and-test.md) contains
toolchain setup, Clang requirements, Lean invocation and platform instructions.
Run the complete [standing per-change gate](AGENTS.md#standing-per-change-gate)
before claiming a change complete; `--no-fail-fast` remains required.

### Windows

[Windows setup and compiler discovery](docs/development/build-and-test.md#windows).

## Increment records

The [increment history index](docs/development/increments/README.md) names the
**current record file**, every wave, and the early and relocated archives.
Add one paragraph to that current file, newest first, rather than growing this
table of contents. Update headline numbers in `AGENTS.md` when they change.

| Read | Purpose |
|---|---|
| [Current record location](docs/development/increments/README.md#adding-a-record) | Where to add the next increment |
| [Wave index](docs/development/increments/README.md#waves) | Individual wave pages, newest first |
| [Early source and platform records](docs/development/increments/README.md#earlier-source-and-platform-records) | Original integration narratives |
| [Relocated guidance archive](docs/development/increments/README.md#relocated-historical-guidance) | Older status and operational records; historical authority only |

## Historical section links

The original headings below keep existing fragment URLs usable. Follow their
links for the preserved text; the full history is in the index above.

### Quant delimiter alias `[| |]` (2026-09-09)

[Preserved section](docs/development/increments/source-and-platform-notes.md#quant-delimiter-alias---2026-09-09).

### Sequential C emission indentation (2026-09-08)

[Preserved section](docs/development/increments/source-and-platform-notes.md#sequential-c-emission-indentation-2026-09-08).

### Core concurrency prelude (2026-09-08)

[Preserved section](docs/development/increments/source-and-platform-notes.md#core-concurrency-prelude-2026-09-08).

### Reopen failure evidence (2026-09-07)

[Preserved section](docs/development/increments/early-integration.md#reopen-failure-evidence-2026-09-07).

### Status paragraphs relocated from AGENTS.md and CLAUDE.md (2026-09-10, card DOCS-COMPACT)

[Preserved section](docs/development/increments/legacy/cycles-and-teardown.md#status-paragraphs-relocated-from-agentsmd-and-claudemd-2026-09-10-card-docs-compact).

#### Cycle status ledger as carried in AGENTS.md through wave 15

[Preserved section](docs/development/increments/legacy/cycles-and-teardown.md#cycle-status-ledger-as-carried-in-agentsmd-through-wave-15).

#### Cycle 19 nested-teardown boundary as carried in CLAUDE.md through wave 15

[Preserved section](docs/development/increments/legacy/cycles-and-teardown.md#cycle-19-nested-teardown-boundary-as-carried-in-claudemd-through-wave-15).

#### R02 terminal task state, adversarial verification (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#r02-terminal-task-state-adversarial-verification-2026-09-07).

#### Compile-task identity (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#compile-task-identity-2026-09-07).

#### Windows Clang discovery and test portability (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#windows-clang-discovery-and-test-portability-2026-09-07).

#### Diagnostic attribution and ownership (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#diagnostic-attribution-and-ownership-2026-09-07).

#### Configured second source set (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#configured-second-source-set-2026-09-07).

#### Replay mutation matrix (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#replay-mutation-matrix-2026-09-07).

#### Source-filename-sensitive transport (2026-09-07)

[Preserved section](docs/development/increments/legacy/identity-and-diagnostics.md#source-filename-sensitive-transport-2026-09-07).

#### Checked mutex invariants (2026-09-07)

[Preserved section](docs/development/increments/legacy/mutex-and-scalars.md#checked-mutex-invariants-2026-09-07).

#### Typed scalar-field access classification (2026-09-07)

[Preserved section](docs/development/increments/legacy/mutex-and-scalars.md#typed-scalar-field-access-classification-2026-09-07).

#### Remaining-work wave accounting (2026-09-07)

[Preserved section](docs/development/increments/legacy/remaining-work.md#remaining-work-wave-accounting-2026-09-07).

#### Delegated increment (2026-09-08)

[Preserved section](docs/development/increments/legacy/remaining-work.md#delegated-increment-2026-09-08).

#### Second delegated increment (2026-09-08)

[Preserved section](docs/development/increments/legacy/remaining-work.md#second-delegated-increment-2026-09-08).

#### Short pointer bullets carried in CLAUDE.md through wave 15

[Preserved section](docs/development/increments/legacy/operational-notes.md#short-pointer-bullets-carried-in-claudemd-through-wave-15).

#### Claude-specific working bullets carried in CLAUDE.md through wave 15

[Preserved section](docs/development/increments/legacy/operational-notes.md#claude-specific-working-bullets-carried-in-claudemd-through-wave-15).
