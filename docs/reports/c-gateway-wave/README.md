# C gateway refinements after the worktree sweep

Base: `c05b2c4b81c6d99352aa23fa401bb7504e7da863` (main, 2026-09-13).
The user selected AOM-POISON-NOOP-INCREMENT, AOM-POISON-CONDITION-FRAME,
and AOM-DESTROY-JOINED-PROVENANCE, and requested a worktree sweep before
implementation, followed by commit and push after validation.

## Worktree sweep

[worktree-sweep.json](worktree-sweep.json) records the fresh inventory.
The preceding sweep had already removed six obsolete checkouts. All four
remaining secondary worktrees contain unique, unmerged implementation work;
one also contains uncommitted corpus evidence. None is obsolete. They, their
branches, the corpus lock, and `.sweep-backups` are retained. The four existing
corpus recovery archives/patches were verified against their recorded hashes.
No pending implementation was merged or deleted by this change. This wave
runs in the clean primary checkout, independently of the wave-31 recovery
plan's four pending cards.

## 1. Scalar increments and decrements

A standalone prefix/postfix `++` or `--` of a declared scalar, including a
parenthesized operand, now emits `ForgetScalar`. It removes that scalar's old
binding fact without changing resource obligations. Thus `n = 0; n++; if (n)`
cannot discard the nonzero branch using the old zero. An absent scalar fact
permits both branches and does not turn a scalar return into a pointer escape.

Nested increments, increments in conditions/returns, calls carrying increments,
and pointer increments remain refused by this bounded model. Custody-neutral
scalar assignments, including `flag = !0`, enum assignments, compound writes
and scalar-field reads, also forget their old branch facts. Multiple scalar
writes in one statement and assignments inside conditions/returns are refused. These increment/decrement expressions are
never silently replaced by `NoOp`. This card does not model arbitrary C
arithmetic values or overflow. Fixed-query operand markers remain classifications, never
computed values. The old scalar-only `n++` Safe control remains Safe; the
new leak-bearing control becomes visible through the real Clang gateway.

## 2. Complete-expression refusal frames

The parser now derives a `RefusedWrites` frame from each complete `if`,
`while`, `for` condition, `do/while`, `switch`, `case`, and return expression.
The corresponding CFG refusal uses that frame. Unsupported diagnostics and
the unmodeled-path flag are retained. Unknown/malformed control flow, unresolved
jumps, and stray labels still use the blanket poison.

The three existing positive premises are unchanged: a plain automatic name,
whose address is never taken, with no write position in the refused text.
The write reader now also recognises prefix mutation through parentheses,
such as `++(p)`; otherwise the new condition frame would wrongly spare `p`.
Assignments, mutation operands, address-taken locals, static locals and globals
are covered by refusing controls. Other bindings can survive an unrelated
refused condition so that subsequent releases and repeated releases remain
visible. Resource states are not modified by the framing rule itself.

## 3. Joined lookup/acquisition provenance

Nullness and resource identity are separate facts. A null test of an unknown
binding now narrows that binding on its two edges while retaining the
UnknownAlias diagnostic and the unmodeled path. The null edge has no pointer
target; the nonnull edge keeps the unknown identity. Neither edge removes a
resource, refines another alias, or invents caller custody.

`legacy_joined_provenance.rs` adds a separate pre-pass over **acyclic** CFGs.
Names must be unshadowed automatic locals whose addresses are never taken;
unreadable declarations and contextual recursion combinations derive no names.
For each locally created site it records names that denote that site on every
path **on which the site exists**. A predecessor where the site is absent is
neutral; two present predecessors intersect the names. Assignments invalidate
relations; aliases copy them; null edges exclude conditionally named sites.
Calls forget prior relations, with only a fresh borrowed-parameter constructor
establishing its returned identity. Cyclic CFGs derive no facts. Nothing is
added to `AbstractState` and the pre-pass does not alter worklist scheduling.

A plain, shallow declared release may record that conditional acquisition as
Released only when its incoming state has exactly one known target plus an
unknown alternative, the binding is nonnull, the site is Live and nonnullable,
there is no earlier unresolved effect or child/reallocation relation, and the
call's only failure is that source's unresolved identity. Other effects keep
the ordinary refusal. The UnknownAlias stays, other uncertain resources stay
uncertain, and the function cannot certify. The controls distinguish this
from overwriting a resource that already exists on the lookup branch.

**The production residual is not closed.** The fresh `input_chunk_get` probe
still has Mixed custody at L19847: its address-taken `ic` binding crosses
refused stores/calls and conditions, including the uncontracted
`flb_input_chunk_get_projected_write_size`. The new conditional relation
supplies a previously missing capability, but does not pretend these other
premises hold. The real function and its lookup alternative remain Unknown;
its declarations were not replaced by a reduced fixture. Follow-ons:
`JOINED-PROVENANCE-CALL-FRAMES` (prove preservation across calls),
`JOINED-PROVENANCE-CYCLES` (a loop invariant instead of an acyclic pre-pass),
and the remaining production `AOM-DESTROY-JOINED-PROVENANCE` integration.

## Proof boundary

`QCore.CGatewayRefinements` and `AuditCGatewayRefinements` prove scalar
forgetting, the complete-expression frame premises, conditional identity over
path unions, the absent-predecessor rule, null-guard exclusion, and release of
every present local acquisition without certifying the unknown lookup.
Faithful C token/type extraction, the C object model under no undefined
behaviour, and trusted release contracts remain premises. These are abstract
rule theorems, not extraction of the Rust analyzer or whole-C correctness.

## Measurement and validation

The first corpus review exposed a second stale-value shape: the production
`decode_histogram_points` writes `static_metric_detected = !0`, which the
older scalar whitelist left as `NoOp`. The complete-expression frame made
that stale zero visible. The final scalar-write hardening repairs this before
publication; the intermediate stages are attribution evidence, not accepted
checkpoints. The added regression covers `!0`, enum values, scalar-field
reads, compound writes and assignment expressions.
The published stages' raw logs and captured audit binaries live in the ignored
`measure/c-gateway-wave/`. `evidence.py` archives all five artifacts from each
stage and itemizes every diagnostic and verdict change. Previous archives stay
frozen. The source transport remains HistoricalStdin and every diagnostic
comparison uses the internal preprocessed coordinate.


## Reproduction and evidence interpretation

`run_audit.py` records the final executable hash, the 1,415-entry compile
database hash, the unchanged verified ChunkIO 0.13.0 package, and the
HistoricalStdin transport. `audit-runs.json` records completion of each stage.
The initial BEFORE reproduces the wave-30 diagnostic and chunk-contract
archives byte for byte. The final `artifacts/` is the new baseline; each
`artifacts-<stage>/` is an intermediate measurement. All five artifacts are
archived at every stage, and `diff-*.json.gz` contains the canonical itemizer's
raw and normalized comparisons. `leaves-*.json` enumerates diagnostic
occurrences by library, file, function, internal line, category and text;
its status lists are per-entity moves, not just histogram differences.

The diagnostic evidence is deliberately provisional. In particular, the
`rd_kafka_jwt_b64_decode_payload` repeated-release row runs through a
preprocessed `do { if (converted_src) rd_free(converted_src); } while (0)`
macro. The existing CFG abstracts that constant loop condition as a `NoOp`
with a back edge. Preserving `converted_src` makes the repeated abstract
release visible; this is not a validated upstream double free. A named
follow-on, `C-GATEWAY-CONSTANT-DO-WHILE`, must model the constant exit before
that diagnostic could support an upstream claim. This change leaves the
existing worklist schedule and its provisional classification unchanged.


## Final measured result

The final audit exits 0. [headlines.json](headlines.json) and
[artifact-manifest.json](artifact-manifest.json) own the counts and byte hashes.
The last scalar-write review restores the four intermediate missing leak leaves;
**no baseline leak leaf is removed**. Leak **266 → 272** is six newly exposed
provisional rows, each listed below. There are **no unit status moves and no
Safe promotions**. The 185-unit histogram remains **2 Safe / 42 provisional
Unsafe / 137 Unknown / 4 data-only**. The 4,834-function histogram becomes
**146 Safe / 85 provisional Unsafe / 4,603 Unknown**, with exactly five moves:
the four `cio_chunk_counter_total_{add,sub,up_add,up_sub}` helpers withdraw
Safe → Unknown because their field increments are now explicitly refused;
`rd_kafka_jwt_b64_decode_payload` moves Unknown → provisional Unsafe for the
constant-loop abstraction described above. `flb_input_chunk_create` stays Safe.

| Stage | Leak | Unknown alias | Unsupported | Double release |
|---|---:|---:|---:|---:|
| BEFORE (wave-30 bytes reproduced) | 266 | 4,344 | 49,760 | 0 |
| Scalar increments | 266 | 4,339 | 50,160 | 0 |
| Complete expression frames (intermediate) | 264 | 4,167 | 50,047 | 1 |
| Joined provenance/nullness (intermediate) | 268 | 4,163 | 50,046 | 1 |
| AFTER, scalar assignments hardened | **272** | **4,208** | **50,155** | **1** |

The final diagnostic diff is **813 additions / 547 removals**: additions are
798 unsupported, eight unknown-alias, six leak and one double-release rows;
removals are 403 unsupported and 144 unknown-alias rows. Many are a refusal's
changed classification, not a newly decided statement. The scalar stage alone
replaces 390 unsupported rows while adding 790, and changing W4 availability
also changes callers' uncontracted-function rows. The frame stage preserves
bindings and scalar facts across complete expressions, while the final scalar
hardening prevents those facts from surviving a write. The intermediate
Unsafe → Unknown → Unsafe movement of `decode_histogram_points` and its unit
is recorded explicitly, not hidden by the final histogram.

| New provisional leak | Internal exit line | Acquisition origin | First exposed by |
|---|---:|---|---|
| cmetrics `decode_exponential_histogram_data_point` | 8095 | `cmt_decode_opentelemetry.c:1314` | Expression frame |
| monkey-libevent `mk_server_listen_init` | 6324 | `mk_server/mk_server.c:205` | Expression frame |
| cmetrics `decode_histogram_data_point` | 7914 | `cmt_decode_opentelemetry.c:1048` | Nullness refinement |
| cmetrics `decode_summary_data_point` | 7787 | `cmt_decode_opentelemetry.c:899` | Nullness refinement |
| cmetrics `decode_histogram_points` | 6903 | `cmt_decode_prometheus_remote_write.c:513` | Nullness refinement |
| librdkafka `rd_kafka_snappy_java_uncompress` | 20499 | `snappy.c:1705` | Nullness refinement |

The frame exposes identities at the first two exits; the nullness rule retains
resource obligations while narrowing only the tested binding, exposing the
remaining four. These are analyzer findings under incomplete coverage, not
validated upstream leaks. The four older `decode_histogram_points` leaves at
L6840, L6853, L6880 and L6901 all stand in AFTER: the intermediate frame lost
them while an old zero survived `static_metric_detected = !0`, and the final
write rule restores them. Its success exit L6903 is a separate added row.
[probes/commands.json](probes/commands.json) and the compressed per-function
probe logs bind these observations to the actual pinned declarations.

All four non-diagnostic artifacts are byte-identical across all five stages:
contracts, source, source-effects and chunk-contracts. The manifest keeps 19
selected modules, 19 raw keys and no normalization collision. Sidecar coverage
remains three checked function contracts / 93 Unknown. Contracts remain 74;
CT mapping remains 42/50, proof 2 of 42, sanitized-native 7 of 42.

[gate.json](gate.json) supplies the acceptance-matrix gate;
[gate-details.json](gate-details.json) and [pre-final-gates.json](pre-final-gates.json)
retain its command, ignored count and earlier attempts. The final source passes
**4,484 workspace tests (0 failed, 8 ignored), 200 Python tests, 123 Lean audits,
fmt, Clippy and both reevaluation probes**. The 122 provider tests are carried
from wave 30 because providers, schemas and packages were unchanged. No tests
or sanitizer checks were disabled. The separate green workspace run before
the last scalar-write review is not counted twice. Full language conformance
remains false.
