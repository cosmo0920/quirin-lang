# Generic C source import and replay consolidation

Baseline `128d355`, 2026-09-07. W5's crate decomposition now moves six generic
implementation modules out of the compiler. Clients can import typed C paths
and check the supported contracts through the gateway and verification crates,
without depending on the compiler or a first-party provider.

| Module | Owning crate | Property and boundary |
|---|---|---|
| `typed_source` | `qcore-c-gateway` | Bounded typed paths; calls remain opaque and imports are not safety certificates |
| `source_call_use` | `qcore-c-gateway` | Def/use of call results; no claim of call success or discharge |
| `counter_contract` | `qcore-verification` | Checked unit counter updates under explicit unsigned-range/ABI premises |
| `field_destructor` | `qcore-verification` | Nullable owned-field release under explicit live/exclusive/disjoint storage premises |
| `list_unlink_contract` | `qcore-verification` | Complete helper/outer replay over the two supported neighbor-alias cases |
| `cleanup_outcome` | `qcore-verification` | Conditional cleanup field transitions, including guarded deletion; OS custody remains a premise/boundary |

The compiler modules now re-export the same types and entry points. Existing
provider adapters and public paths continue to compile; there is one production
implementation of each checker. The gateway owns `DEFAULT_SOURCE_RULES`, using
the unchanged declarative policy bytes. The compiler's default policy alias
remains for its Boolean-source compatibility adapter. The verification crate
adds a direct Serde dependency for existing serializable types; both workspace
lockfiles record that edge, with no package version updates or new workspace
edges. Layering guards pass.

`SourceSummary` retains private fields and no Deserialize implementation.
Mutable path records or JSON reports cannot manufacture an imported summary.
The constructor, reference/type checks, path limits, unsupported-operation
rejections, default rules and all replay algorithms are unchanged. Imports bind
a declared revision string; pin verification and actual checkout authority stay
with their existing callers. Neither this move nor a string revision establishes
source/runtime or layout equality.

## Test separation and generic consumers

The gateway now owns short-circuit and call-result tests, plus direct private
symbol/reference validation. Those tests do not depend on library adapters.
The existing library corpus stays in compiler compatibility tests. Redundant
private-validator calls on already imported summaries were removed from those
library tests; direct negative producer/aggregate/loop-scope checks execute in
the gateway, and all source/operation assertions remain. The loop increment test
now checks the exact exported expression instead of using a private visitor.
A certificate field assertion reads its existing serialized field, preserving
private construction without a new public mutator/accessor.

Four new `qcore-verification` integration tests import an unrelated C fixture and
exercise all four replay APIs directly. They check counter underflow and wrong
field mutations, nested release order and false predicates, both list-neighbor
alias cases and changed helper behavior, and cleanup failure field preservation.
No compiler or provider dependency supplies these tests. The actual fixture and
QCore contracts live in `crates/qcore-verification/tests/fixtures/generic_source`.
Two compile-fail documentation tests separately prevent summary construction
and JSON deserialization. Moving one loop-scope assertion into its own unit test
accounts for the remaining test-count increase: **760 → 767**.

## Parity and validation

Run `python3 -B docs/reports/w5-generic-source-consolidation/check-relocation.py`
from the repository root to compare the six production bodies with the baseline.
It removes test modules/comments, normalizes only documented module references
and import ordering, preserves exact string tokens, checks the relocated default
policy helper, and verifies unchanged policy bytes. `relocation-parity.json`
records the original/current source hashes and normalized token inventories.
This is implementation parity evidence, not a Rust semantic-refinement theorem.

All standing gates pass: **767 workspace tests**, format/Clippy, both reevaluation
probes, Lean build and all **20** axiom audits. The excluded provider workspace
passes **31 tests**, format/Clippy and locked offline dependency resolution;
all **43 Python tests** pass. There is no core or Lean semantic change, new
protocol/schema surface, CT mapping, provider witness capability or native claim.
Production provider packages remain unchanged.

The full **185-unit** audit uses persistent Monkey inputs and the existing
configured process packages. All four source/diagnostic/contract/source-effect
artifacts are byte-identical to baseline: **2 Safe /42 provisional Unsafe /
137 Unknown**, plus four data-only units; **3 checked /93 Unknown** source-effect
functions. `audit-parity.json`, `validation.json` and compressed logs/artifacts
record this comparison. No raw unit is promoted.

Specialized reference-body matching stays in the compiler: moving its private
reviewed-reference constructor across crates needs an explicit authority boundary.
Full provider source extraction, generalized lifecycle/callback import, registry
composition, compiler-driver consolidation and final compatibility removal remain
open. Subagents remain terminal at their reported usage limit; this increment
was implemented and reviewed locally.
