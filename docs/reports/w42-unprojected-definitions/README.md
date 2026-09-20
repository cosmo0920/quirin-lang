# W42-UNPROJECTED-DEFINITIONS — exact leaf census

This card measures the gap between the B-series enumerated source denominator
and its projected function rows. It changes no C projection, contract, verdict,
source pin, or frozen artifact. Python measurement is supplementary evidence;
it does not close a semantic card, and no `Unknown` is promoted.

## Result

The pinned artifact contains **4,921 enumerated source definitions** and
**4,834 projected function rows**, leaving **87 names**. All 87 names have
positively verified raw function-definition syntax, and all of their definition
sites are excluded by the pinned historical build:

| definition classification | count |
|---|---:|
| pinned-build exclusion | **87** |
| raw-enumerator false positive | **0** |
| established projection defect | **0** |
| unresolved | **0** |

This is a classification of the 87-row denominator gap, not a safety result.
An excluded definition gains no projected verdict. The 4,834 projected rows
remain **221 Safe / 84 provisional Unsafe / 4,529 Unknown** at this baseline,
and the 87 exclusions do not enter that function-verdict split.

The gap spans **24 `(library, relative file)` units**. The previously stated
23 is the distinct-basename count: the two real units
`monkey-libevent/mk_core/mk_utils.c` and
`monkey-libevent/mk_server/mk_utils.c` collide as `mk_utils.c`. Collapsing that
basename loses a unit identity, so this report uses 24. The 87 rows by library
are librdkafka 67, monkey-libevent 13, chunkio 3, fluent-bit-chunk 2,
cmetrics 1, and ctraces 1.

## What “pinned-build exclusion” proves

The tool total-or-rejects the baseline `not_projected` arrays against
`source_functions - functions`, then total-or-rejects the resulting unit and
name sets against the anchor ledger. Every anchor is checked as a raw C
function-definition token form through its body opener; a call or prototype is
rejected.

For each source it reconstructs
`qcore_audit::compile_db::compiler_options_compatibility`: source-parent and
compile-database `-I`/`-isystem` paths, stable `-D` values, the selected dialect,
and `HistoricalStdin`. This is the route that made the archived baseline, not
an inferred platform configuration. A Clang branch probe emits which sibling
branch was selected. Its inserted directives restore the `<stdin>` logical line
after each branch and `#endif`; the run is accepted only when Clang's complete
preprocessed token stream is identical to an uninstrumented replay. Continued
conditional directives are refused by the tool rather than instrumented.

The same replay emits a typed AST dump. A row is a build exclusion only when
every anchored definition site has a positively selected different branch for
every matching compile-database entry and no same-name typed definition
survives. There are 25 matching entries for the 24 units (the static/shared
duplicate agrees); all 25 exclude the relevant sites. AST name survival alone
would remain `unresolved`: it would still need anchored source/body identity
and attribution through the historical projection transport before this tool
could call it a projection defect.

The machine-readable result is
[`artifacts/census.json`](artifacts/census.json), generated from the explicit
[`artifacts/anchors.json`](artifacts/anchors.json). It records every definition,
source/body line, selected sibling branch, per-entry command identity,
preprocessed and AST hashes, compiler banner, source hash, and the unit status
and reason code as separate fields.

## Unit-level Unknown reasons are a different denominator

Across all 185 units, the sibling whole-unit reason census is:

| unit reason code | units |
|---|---:|
| `unsupported-syntax` | **136** |
| `incomplete-obligation-coverage` | **1** |
| no reason code | **48** |

These counts reconcile to 185. They do not classify the 87 excluded
definitions. Of those names, 47 happen to sit in unsupported-syntax units, 30
in the one incomplete-coverage unit (`tinycthread.c`), and 10 in provisional
Unsafe units; that association is not attribution of the unit reason to a leaf
the pinned build did not compile.

## Priority consequence

None of these 87 rows is a cheap pinned-Linux Cycle 19 addition: admitting a C
form cannot project a definition absent from the configured translation unit.
Platform/profile expansion would be a separate evidence task.

The existing B-series blocker census therefore remains the useful syntax
priority. Its highest-reach row is indirect memory access outside the resource
projection (**17,775 leaves / 3,157 functions / 288 sole-blocked functions**).
The return-expression family has the largest sole-blocked movement
(**1,440 / 1,109 / 465**), followed by the branch-condition family
(**8,750 / 2,424 / 43**). Smaller measured candidate rows are increment or
decrement (**734 / 378 / 5**), nested calls (**22 / 13 / 4**), and local
declaration shadowing (**33 / 22 / 2**). These are reach counts, not proofs of
implementation cost or semantic admissibility; any implementation still needs
Rust, audited Lean parity, and the standing gate.

## Evidence identity and reproduction

The verdict-bearing input is the B-series integration diagnostics compressed
SHA-256
`cc9e5ebc2a330191d74965b64c5c9a4acb80407f4e1ec10ab333516871960949`
(raw JSON
`2545039c8252d5ac2c313e98104b59293e701dd1743d88ee5a659c37c345732b`).
Its run identity is source commit `18b142b8d055a385985c68b392f449d60d3e2b99`
and binary SHA-256
`85c8208326f1f234cc49644f582710fcb2ba7b30bfae60ba25359f184191847a`.
The Fluent Bit and Monkey compile databases hash to
`5e6868d9af26eaec97bc8020ed5f23496f2490abf44532443239bd4d365f7e78`
and `85cc996712bbd35675340b8b863b6ebc716cb94552253f2a6ad0288db920f331`.
All observations used Ubuntu Clang 21.1.8. The census processed 11,956,614
bytes of active preprocessed text and 98,981,155 bytes of typed AST dumps in
temporary storage.

Run from the repository root (substitute relocated pinned build paths as
needed):

```sh
python3 -B scripts/w42_unprojected_census.py \
  --diagnostics docs/reports/b-series-integration/artifacts/diagnostics.json.gz \
  --run-identity docs/reports/b-series-integration/artifacts/run.json \
  --anchors docs/reports/w42-unprojected-definitions/artifacts/anchors.json \
  --compile-db fluent-bit=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json \
  --compile-db monkey=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/monkey/monkey-build/compile_commands.json \
  --output /tmp/w42-census.json
```

The final reproduction exited 0. The focused Python tests are:

```sh
python3 -m unittest scripts.tests.test_w42_unprojected_census
```

They cover branch/synthetic-branch marking, continued-directive refusal,
logical-line restoration, raw definition versus call/prototype anchoring,
historical flag reduction, and AST body detection. A real-Clang boundary test
also proves that marker instrumentation preserves active `__LINE__`/`__FILE__`
tokens and that a deliberately changed active token is rejected.

## Standing gate

The authoritative workspace run used `CARGO_BUILD_JOBS=1`,
`CARGO_PROFILE_DEV_DEBUG=0`, a disk-backed `CARGO_TARGET_DIR`, and elevated
execution so LeakSanitizer retained its ptrace checks. It passed **4,845 tests,
0 failed, 10 ignored** over 248 result summaries. Formatting and Clippy were
clean; both reevaluation examples exited 0; all **148 Lean audits** passed; and
the Python suite passed **313 tests**. Submodule initialization also exited 0.
Exact commands, exits, the host Clang recording, and the deliberately preserved
invalid sandbox attempt are recorded in [gate.json](gate.json). Private logs
are under `measure/w42-gate-22d3056e/`.

The first workspace attempt ran inside the sandbox, where LeakSanitizer could
not ptrace its child processes. Its log is retained rather than counted: it
printed 4,743 passed / 102 failed / 10 ignored across 248 summaries, and the
owning session was interrupted with exit 130. The elevated complete run has no
failed result row.

## Integrator reconciliation list

1. Use **24 path-qualified units** for this gap; retain **23** only when it is
   explicitly labelled the basename count and name the `mk_utils.c` collision.
2. Do not add the 87 exclusions to the 4,834 projected-function verdict
   denominator or attach whole-unit reason codes to them.
3. Record this as measurement only. It closes no syntax/semantic card and
   changes no Safe/Unsafe/Unknown status.
4. When choosing follow-up syntax work, use the blocker census reach and sole
   counts above; do not treat excluded-platform definitions as cheap Linux
   additions.
