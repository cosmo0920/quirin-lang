# W5 generic driver and custody consolidation

2026-09-07, baseline `7cb45a0`. Source compilation with an explicit verifier
registry and the remaining generic library-custody machinery can now be used
without linking `qcore-compiler` or its first-party adapters. This is a bounded
consolidation increment; complete provider semantic extraction remains open.

| Component | Current owner | Preserved boundary |
|---|---|---|
| Reference-count, container and synchronized-custody model | `qcore-core::library_custody` | No runtime or workspace dependencies; declared evidence kinds remain model premises |
| Library mechanism and chunk operation import gates | `qcore-verification::library_import` | Explicit permitted mechanisms, revision and complete operation inventory |
| Source-bundle fragment/checksum checks | `qcore-verification::external_source` | Existing declared source/profile checks, without stronger authentication claims |
| Cycle17 aggregate checker | `qcore-verification::research_extensions` | Same inference, loop, legacy-C, weak-memory, native-platform and accounting checks |
| Sequential/scoped-task source compilation | `qcore-verification::driver` | Explicit registry; complete checked profile selection, reference execution, dispatch and C emission |
| Default registry discovery and Monkey composition | `qcore-compiler` | Existing default behavior and library-specific interpretation remain adapters |

The compiler's public entry points re-export the same types and functions.
`compile_files` still builds the default first-party registry and delegates;
`compile_files_with_registry` now resides in verification. An empty explicit
registry permits ordinary source compilation but still rejects a requested
missing provider. Unchecked source contracts still reject. No dependency edge,
language rule, diagnostic, protocol or Lean model changes.

`check-relocation.py` compares seven production components against the baseline,
including the retained default wrapper and Monkey specialization. After only
documented import/module-owner substitutions, all 14,638 tokens match. Comments
and test modules are excluded, imports are sorted, and string contents are
preserved. This is source comparison evidence, not a formal Rust equivalence
proof. Run it from the repository root:

```sh
python3 -B docs/reports/w5-generic-driver-consolidation/check-relocation.py
```

Two existing pure custody tests move into the kernel. Five new integration tests
in verification compile sequential and owned-task source without compiler
adapters, reject missing providers and unchecked contracts, and exercise generic
mechanism/revision and chunk-inventory failure gates. A compiler compatibility
test checks type identity across five facades. Existing compiler/CLI tests still
exercise default discovery and actual generated code. The owned-task profile
remains limited to Linux C11 threads.

All standing gates pass: 797 workspace tests, formatting, clippy, both
reevaluation probes, Lean build and 20 placeholder-free Lean audit modules.
The excluded provider workspace passes its 31 tests, formatting and clippy;
all 43 Python tests pass. Commands, exit statuses and compressed log hashes are
recorded in `validation.json`. No additional native sanitizer experiment or new
semantic theorem is claimed for this relocation.

The full audit uses the persistent `build/cycle19/monkey-build/compile_commands.json`:

```sh
QCORE_SOURCE_CONTRACTS="$PWD/contract/profiles/fluent-bit/source-contracts.json" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  /tmp/qcore-generic-driver-source.json /tmp/qcore-generic-driver-diagnostics.json
```

Only these generated report destinations are temporary. All four report
artifacts are archived here and byte-identical to
`docs/reports/w6-reused-address`: 185 units, 2 Safe /42 provisional Unsafe /
137 Unknown plus 4 data-only units; source-effect replay remains 3 checked /
93 Unknown. `audit-parity.json` pins sizes and hashes. No raw verdict is promoted;
58 contract modules and CT coverage of 11 mapped /39 unmapped are unchanged.

The public model profiles still declare assumptions. A model certificate does
not authenticate a checkout, establish native synchronization or bypass semantic
witness admission. Specialized provider source binding, broader lifecycle
composition, provider-owned semantic implementations and removal of compatibility
routes remain open.
