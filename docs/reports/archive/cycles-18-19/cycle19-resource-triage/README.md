# Cycle 19: resource diagnostic inventory and importer recovery

The 2026-09-06 rerun uses Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and the same selected compilation
databases. It retains **342 additional function projections**, with no new
whole-unit safety certificate. Cycle 19 remains in progress.

| Measure | Before | After |
| --- | ---: | ---: |
| Selected translation units | 185 | 185 |
| Raw source functions | 4,775 | 4,775 |
| Projected functions | 4,346 | 4,688 |
| Projection coverage | 91.0% | 98.2% |
| Safe units | 2 | 2 |
| Provisional Unsafe units | 54 | 98 |
| Unknown units | 125 | 81 |
| Data-only units | 4 | 4 |
| Gateway failures | 0 | 0 |

The reduction in Unknown units is mostly a classification change: retaining
previously unprojected functions allows the existing aggregate verdict to
report provisional leaks or release conflicts. Specifically, 47 units change
from Unknown to Unsafe and three change from Unsafe to Unknown. The new
inventory still exposes unresolved operations inside Unsafe units. These are
not 44 newly proved-safe units or confirmed upstream defects.

| Slice | Projected before → after | Safe | Provisional Unsafe | Unknown |
| --- | ---: | ---: | ---: | ---: |
| CMetrics | 525 → 539 | 0 | 19 | 12 |
| CTraces | 264 → 273 | 1 | 2 | 13 |
| librdkafka | 3,025 → 3,343 | 0 | 53 | 38 |
| ChunkIO | 122 → 123 | 0 | 5 | 8 |
| Fluent Bit input chunk | 64 → 64 | 0 | 1 | 0 |
| Monkey, forced Libevent | 346 → 346 | 1 | 18 | 10 |

## Changes

- The audit prints diagnostic category counts and three examples per unit.
  An optional fourth positional argument writes every function verdict and
  diagnostic, including unresolved aliases inside Unsafe units. It also lists
  raw definitions absent from the projection and preserves gateway/projection
  failure reasons. Coordinates refer to Clang's `-E -P` output, not original
  file lines; zero denotes a synthetic location. An absent raw definition can
  be excluded by the build, so it is not automatically a parser failure.
- The generic Unknown reason distinguishes unsupported operations from
  unresolved aliases. `sizeof` and related type operators require operand/type
  analysis rather than being described as missing function contracts. They
  remain Unknown: VLA operands can have evaluated effects.
- The statement scanner balances parentheses, brackets, and braces. Local
  aggregate initializers, compound literals, local type definitions, and GNU
  statement expressions retain the rest of the function and its cleanup.
  Their unmodeled effects emit an explicit Unknown operation. Braced return
  and control expressions also remain unresolved; no aggregate custody is
  inferred from lexical structure.
- Null refinement recognizes complete parentheses, negation, either ordering
  of equality/inequality, canonical resource places, and the preprocessed
  `((void *)0)` constant. Parenthesized aliases are retained. Nonzero casts,
  pointer arithmetic, and compound conditions are not treated as null tests.
  Overwriting an allocation with null still leaves a live ownership obligation.

The four functions whose local resource summaries become Safe are three
CMetrics nullable-allocation/return helpers and CTraces' `destroy_id`. This is
only the existing resource projection: it does not establish allocation-size
arithmetic, arbitrary caller provenance, or whole-source memory safety.
No library contract was added or relaxed. The two native chunk field contracts
and three callback source-path imports still check separately; no new
lifecycle or source/runtime composition certificate is issued.

## Evidence and validation

- [Pinned source audit](source-audit.log)
- [Before/after totals, transitions, and ranked diagnostics](summary.json)
- [Complete baseline inventory, gzip JSON](diagnostics-before.json.gz)
- [Complete current inventory, gzip JSON](diagnostics-after.json.gz)
- [Source/build/tool and artifact hashes](evidence.json)
- [Workspace tests](tests.log): 312 pass, including five new importer regressions.
- [Audit-report tests](report-tests.log): two pass, covering unresolved
  diagnostics inside Unsafe units and absent partial summaries.
- [Formatting](fmt.log), [Clippy with warnings denied](clippy.log), and
  [reevaluation controls](reevaluation.json): pass.
- [Lean build](lean-build.log) and [Cycle 19 dependency audit](lean-audit.log):
  pass, with no placeholders or project axioms. No Lean semantics changed;
  these results do not constitute a proof of the Rust parser normalization.

Native harnesses were not rerun for this importer/reporting change. The pinned
audit reran the existing typed chunk and callback import checks. Prior native
execution evidence remains in the callback-source report.

## Reproduction and remaining work

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-callbacks.json \
  /tmp/cycle19-resource-diagnostics.json

cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run --quiet -p qcore-compiler --example reevaluation_probe
(cd proof && lake build && lake env lean AuditCycle19.lean)

# Inspect the retained diagnostic inventory without rerunning the audit.
gzip -dc docs/reports/cycle19-resource-triage/diagnostics-after.json.gz
```

The checked-in baseline was measured after adding diagnostic reporting and
before changing importer semantics. Both runs keep the original source/build
selection and resource contract manifest. Recreate a missing forced-Libevent
build using the cycle validation document.

Next work follows the inventory rather than a goal of making the log say Safe:

1. Add typed expression and evaluated-call import for assignments and control
   conditions, including type operators and nested calls. Do not infer neutral
   effects for logging, memory, or synchronization calls from their names.
2. Bind aggregate and container expressions to actual source identities and
   19B list/queue/refcount/arena/destructor contracts. Balanced braces supply
   visibility, not the semantics required for that proof.
3. Extend 19C source imports through the selected ChunkIO/wrapper lifecycle,
   failure, and nested teardown paths.
4. Complete 19D callback payload/map/context/fired-slot/runtime bindings and
   allocation/registration failure imports. Monkey's unit counts are unchanged.
5. Resolve the remaining source-selection gaps separately from semantic gaps;
   the 87 raw functions without projections include build-excluded definitions
   and the existing tinycthread source-name selection failure.
