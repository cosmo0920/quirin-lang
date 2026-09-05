# QCore contracts against actual Fluent Bit source — 2026-09-05

The source audit completed with exit code 0: all 185 selected C translation
units passed Clang preprocessing and syntax validation. Resource projection
classified 2 units as `Safe`, 54 as provisional `Unsafe`, and 125 as `Unknown`;
4 data-only units received syntax validation only. **This is not a successful
verification of every QCore contract predicate against Fluent Bit, nor a
whole-program safety certificate.**

## Source and execution identity

- Fluent Bit: `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`
  (`v5.1.1-127-g66910c10a`), at `/home/cosmo/Documents/GitHub/fluent-bit`.
- Fluent Bit's working tree was clean. The audit also enforced its pinned
  revision and selected-source modification gate.
- QCore-C: `6e3bc38a53d5e9ada66ac8e0f95a76b04675d697`; no compiler changes
  were made for this run.
- Toolchain: Ubuntu Clang 21.1.8, x86_64 Linux; Cargo 1.98.0.
- Used the existing Fluent Bit compilation database and existing disposable
  Monkey build with `MK_EVENT_LOOP_LIBEVENT=ON`. The runner checked the backend
  definition and byte-compared selected Monkey C files with Fluent Bit's copy.
  This run did not rebuild or execute Fluent Bit or Monkey.

The exact commands run from the QCore-C root were:

```sh
cargo test -p qcore-compiler --test qcore_contract_corpus
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  > docs/reports/fluent-bit-contracts-2026-09-05/source-audit.log 2>&1
```

The temporary Monkey build must still exist to rerun the second command;
instructions to recreate it are in the
[Cycle 19 validation record](../../cycles/CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md#reproduction).

## Contract checks

The QCore regression suite passed both tests, covering all 11 `.qcore` files,
28 contract-bearing declarations, and 145 structured clauses. These checks
parse, round-trip, compile, and pin the expected contract surface. Arbitrary
contract predicates are retained as syntax and are not semantically proved
against the C implementations by this suite.

The separate source audit passed its pinned source-pattern checks:

| Library | Checked contract patterns | Checked files | Available mechanism summaries |
| --- | ---: | ---: | --- |
| librdkafka | 3 | 3 | Reference counts, intrusive lists, typed queues, cascading destruction, thread handoff |
| CFL | 2 | 2 | Intrusive lists, arenas, cascading destruction |
| ChunkIO | 3 | 3 | Intrusive lists, cascading destruction |

These checks require expected text fragments in pinned source files; they do
not prove the semantics of each C function. CFL is covered by these source
patterns, not a separate whole-library C projection in this runner.

Monkey's adapter uses 10 recognized Cycle 18 API symbols, with 0 unrecognized
symbols. All 7 audited bundled Libevent contract files were byte-identical to
the pinned QCore Libevent reference. API recognition and source equality do
not establish event lifecycle or runtime backend safety.

## Actual C projection results

| Slice | C units | Projected / source functions | Data-only | Safe | Unsafe* | Unknown | Gateway failures |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| CMetrics | 31 | 525 / 540 | 0 | 0 | 17 | 14 | 0 |
| CTraces | 17 | 264 / 274 | 1 | 1 | 3 | 12 | 0 |
| librdkafka | 94 | 3,025 / 3,410 | 3 | 0 | 10 | 81 | 0 |
| ChunkIO | 13 | 122 / 126 | 0 | 0 | 5 | 8 | 0 |
| Fluent Bit input chunk | 1 | 64 / 66 | 0 | 0 | 1 | 0 | 0 |
| Monkey, forced Libevent | 29 | 346 / 359 | 0 | 1 | 18 | 10 | 0 |
| **Total** | **185** | **4,346 / 4,775** | **4** | **2** | **54** | **125** | **0** |

Projection coverage is 91.0% of discovered raw-source functions. It measures
availability of partial resource summaries, not proved memory-safety coverage.
The only `Safe` units are `ctraces/ctr_version.c` (1 function) and
`monkey-libevent/tls/tls_stub.c` (5 functions), under the current model.

*`Unsafe` is a provisional resource-model result, not a confirmed Fluent Bit
defect. Incomplete modeling of custody transfer, aliases, callbacks, lists,
queues, refcounts, and nested destruction prevents interpreting these counts
as an upstream bug count. The runner does not print per-obligation diagnostics
for its `Unsafe` units, so this report makes no specific leak or misuse claim.

The 125 `Unknown` units report 69 reduced-parser failures, 55 unsupported
operation results, and 1 function absent after preprocessing. The parser's
“malformed body” messages refer to limitations of QCore's reduced parser:
Clang accepted every selected translation unit. The absent-function case is
`tinycthread.c`, whose functions are excluded by this build configuration.
Function counts also reflect preprocessing and source discovery, so missing
projections should not all be interpreted as syntax failures.

`src/flb_input_chunk.c` returned provisional `Unsafe` (64/66 projected) in this
run. Its full ChunkIO lifecycle composition remains unproved. Monkey's
`mk_core/mk_event.c` returned provisional `Unsafe` (14/14 projected); a complete
source-derived event trace and deployed dispatcher/backend observation were
not produced. Neither slice receives a lifecycle/event safety certificate.

These counts reproduce the prior Cycle 19 table. The older record's prose
saying the input-chunk wrapper “stays Unknown” must not be read as the current
generic projection verdict; it describes the unresolved lifecycle boundary.

## Evidence and limits

- [Full source audit output](source-audit.log): every selected unit's result.
- [Machine-readable results](results.json): totals, exact `Unknown` reasons,
  revisions, toolchain, and SHA-256 hashes of both compilation databases.
- [Contract regression output](contract-tests.log): both passing tests.

Exit code 0 indicates completion without gateway/source-gate failure; the
runner does not fail its process for `Unsafe` or `Unknown` resource results.
Scope follows selected build commands: CMetrics/CTraces top-level units,
built librdkafka/ChunkIO units, the central input-chunk wrapper, and selected
Monkey library units. Other Fluent Bit core files, plugins, unselected build
configurations, and dependency units outside these slices are not covered.
No runtime integration, sanitizer, or Valgrind run was performed: this task
ran the static contract/source audit and changed no Fluent Bit component.
