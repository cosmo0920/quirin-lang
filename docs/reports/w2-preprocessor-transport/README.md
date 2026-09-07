# W2 ordered preprocessing transport — 2026-09-07

The configurable audit previously converted `-isystem` into `-I` and put the
source directory into the angle-include search path. Either could select a
header that removes an unresolved call and publish a false `Safe` projection.
`red.log.gz` reproduces this through the CLI using the retained historical
adapter: expected `Unknown`, received `Safe`. The corrected CLI regression
covers both collisions and returns `Unknown`.

## Change

The gateway accepts typed, ordered `CPreprocessorOption` values for regular,
system and quote includes, definitions and undefinitions. The configured driver
resolves include paths against the compilation directory and supplies the
source parent as quote-only. It accepts joined or separate arguments and
preserves `-D`/`-U` order. Main units, configured helpers and derived native probe
units carry the same options. The legacy include/definition fields remain;
ordered options apply after them. The configured adapter uses only the ordered
sequence. `C11TranslationUnit` and `CompilerOptions` gain public Rust fields;
external struct literals must initialize them or use the unit's default.
No published schema/wire version changes.

Source-contract configuration identity binds the typed sequence and its order,
including options that leave the imported paths unchanged. Empty sequences keep
historical identity bytes. Strict commands need one source, one `-c`, at most
one output and absolute directory/source paths. Token decoding preserves empty
quoted arguments, double-quote backslash rules and escaped line continuations;
unexpanded shell expressions and unmodeled flags reject. Nothing executes a
compile-database shell command. JSON argument vectors remain literal tokens.

Eight new workspace tests exercise header precedence, quote-only source lookup,
macro ordering, malformed commands/empty arguments, the CLI false-Safe
regression and source-task identity separation. Existing provider conformance
now demonstrates `-D`, then `-U`, then `-D` changing actual helper import success.

## Evidence and limits

`validation.json` records final commands, exit codes, test counts and artifact
hashes. All 816 workspace tests, 45 provider tests, 43 Python tests and 20 Lean
audits pass, with formatting, Clippy and both reevaluation examples. `audit-parity.json` compares all four 185-unit pinned reports with
`w5-cmetrics-unit-provider`; `cmetrics-diff.json` records every change in the
separate configured counter audit. Its resource diagnostics and verdicts stay
unchanged; only configuration/task/coverage identities change. The counter
still has one checked function, six Unknown functions and a provisional Unsafe
raw unit. No raw verdict or CT case is promoted.

This is transport into the existing trusted Clang preprocessing boundary, not
a change to the Lean language or custody semantics. Existing Lean definitions,
theorems and 20 axiom audits remain in force; no theorem claims arbitrary Clang
or compiler-command equivalence. The retained Cycle19 compatibility adapter
still uses its explicitly bounded historical projection, including flattened
include classes, pending replacement parity. The strict adapter still rejects
`-Wall`, `-g`, `-fPIC`, assembler options and other unsupported flags. The actual
CMetrics CMake command therefore still needs the separately declared analysis
build documented in the preceding report. GNU17 defaulting is explicit analysis
behavior, not GCC default discovery; source filename builtins still see stdin.
Broader CMake command support and provider/lifecycle composition remain open.
