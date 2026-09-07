# W2 Clang CMake build transport — 2026-09-07

The canonical CMetrics audit now reads CMake's unmodified compilation database
from a persistent, isolated Clang GNU17 build. It no longer needs a rewritten
command with warning/debug/PIC/assembler options removed. The source is the
same pinned Fluent Bit revision. CMake's `cmetrics-static` target builds
successfully, and both the normal and a freshly prepared build audit report one
checked function (`cmt_counter_allow_reset`), six Unknown functions, and one
provisional Unsafe raw unit. No raw unit or CT case is promoted.

## Reproduction

```sh
# For a fresh package directory only:
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit --target-dir providers/target/conformance-artifacts
python3 -B scripts/prepare_cmetrics_clang_audit.py
cargo run --quiet -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json
```

The default source input is the sibling Fluent Bit checkout; `--fluent-bit`
selects another checkout at the profile pin. `--output` selects another persistent
parent and the script prints its generated audit configuration. For example:

```sh
python3 -B scripts/prepare_cmetrics_clang_audit.py --output build/cycle19/cmetrics-clang-reproduction
cargo run --quiet -p qcore-audit -- build/cycle19/cmetrics-clang-reproduction/fluent-bit-clang-production/cmetrics-audit.json.config
```

The script creates a detached independent clone, not a borrowed Git object
store, and checks the pin plus tracked-source cleanliness. CMake writes some
files into its source tree; using the isolated clone preserves the upstream
checkout. It preserves the full generated database and records the exact
selected entry, source/cache/database/library hashes, compiler version and
configure/build commands. Existing mismatched source or compiler caches reject.
Custom Clang selection must also match `QCORE_CLANG` during audit execution.

## Frontend boundary

Four explicit `CFrontendFlag` variants carry `-Wall`, `-g`, `-fPIC` and
`-Wa,--noexecstack` through preprocessing, syntax validation and typed AST
transport. They are included in the existing ordered configuration identity,
including repetitions. No arbitrary compiler arguments are accepted. Enabling
these flags requires an explicit supported standard and an executable resolving
to the configured Clang C driver; C++ and CL driver aliases reject. Compiler
lookup does not execute the compile-database program. The original GCC 15
command (with a different default dialect) rejects without writing a report.
The pinned compatibility adapter retains its historical bounded projection.

The real Clang/GCC macro probe showed that `-fPIC` removes `__PIE__` on this
host; omitting it can change the chosen C body. New tests compare preprocessing
and AST results with direct Clang, exercise an unresolved call selected by PIC,
and reject missing dialects or a different executable. Existing source-task
identity tests now distinguish the new options even with unchanged source paths.
Unsupported native targets reject rather than dropping flags.

The assembler request is forwarded but the analysis produces no object code;
it does not establish executable-stack policy, code-generation correctness,
GCC equivalence or whole-library safety. Source filename builtins still see
stdin at the gateway. The checked counter function does not use those builtins.
This change extends trusted frontend transport, not Lean language/custody
semantics; no new semantic axiom or theorem is claimed.

## Evidence

`validation.json` records gate results and hashes. All 819 workspace tests,
45 provider tests, 43 Python tests and 20 Lean audits pass, together with the
formatting, Clippy and two reevaluation gates. Three new workspace tests and
extended identity/provider regressions cover this transport. `audit-parity.json` compares
all four full 185-unit reports against `w2-preprocessor-transport`.
`cmetrics-diff.json` records every change against the preceding configured audit;
`build-comparison.json` compares both independent CMake builds after replacing
only their source/build root paths. `build-provenance.json.gz` and its fresh
counterpart retain exact commands. Debug object/archive bytes are not claimed
to be reproducible across directory names.

The two generated commands differ by one additional normal-build include path,
`lib/yyjson-0.12.0/src`, after only source/build root substitution. Both commands
are preserved exactly as generated and produce identical raw counter-unit
diagnostics/verdicts. Their task identities remain separate. This is successful
preparation and audit of both builds, not a claim of identical CMake output or
general build equivalence; the complete difference is archived.
