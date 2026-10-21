# W48 bottom-up batch5: scalar read identity

The actual pinned `cio_chunk_is_up` body now keeps the complete existing typed
plan for `type = ch->st->type`. The earlier opaque-field route manufactured a
pointer identity for the scalar local and reported `UnknownAlias type` at its
zero comparison. The replacement preserves all alias, liveness and per-hop
reads, clears only the scalar result, and retains both possible branch outcomes.
Pointer destinations, store frames, readback, casts and unsupported effects keep
their existing refusal boundaries.

[Prediction](PREDICTION.md) was committed at `d5d881126` before implementation,
on exact semantic base `d08dcca8b77b35417a999474b608cedfe821cfdb`.
[Readiness](readiness.json) binds the actual source to the repository fixture.
Implementation source ends at `9728ff745`; the shared measured integration is
`4d0cd81ff6c1f611bcc538311e92c12324a224f3`. Its report/controller HEAD is
`9ed46cc4e`. Actual embedded evidence versions are independently held BASE build
`d5d881126` and TIP build `4d0cd81f`, with exact binary receipts.

| Route | Custody rows | Safe | Unknown | Pending | Provisional Unsafe |
|---|---:|---:|---:|---:|---:|
| Conditional eight-home corpus | 4,834 | 973 → 975 | 3,852 → 3,850 | 0 | 9 |
| Configured system | 448 | 99 | 345 | 0 | 4 |
| Configured tail/pause slice | 404 | 53 → 57 | 346 → 342 | 4 | 1 |

The corpus gains are `cio_chunk_is_up` and adjacent `cio_chunk_is_file`.
The slice gains are two scoped copies each of
`flb_input_rate_gate_can_resume` and `flb_input_rate_gate_is_limited`, whose
`window_size = ins->rate_window_size` has the same scalar pattern. Six further
Unknown rows retain fewer alias/callee diagnostics across the three audits.
No prior Safe row regresses and no Unsafe is added. These are scoped custody
observations under the existing source/link/type assumptions.

The existing declared `cio_chunk_is_up` contract gains native `W4` and `entry`
standing for `resource_borrowed(#0)` and `resource_return_none()`. No declared
contract, provider, profile, source pin, hypothesis graph or source-binding
truth is added. Header/configuration replay multiplicity is retained separately
from distinct native definitions. All Unsafe labels remain provisional.

All 5,781 nested null dictionaries are identical. Whole envelopes retain their
actual changes: the slice gains two named candidate read premises, the corpus
three; slice combined Safe grows 31 → 35, while the corpus's combined tally
stays unchanged. Runtime custody tallies reflect the measured custody gains.
Complete production null-tally, custody/null joins, outer source/pin/reach and
static/runtime JSON replay exactly on both sides.

[Native comparison](native-comparison.json), [artifact manifests](artifact-manifests.json)
and [semantic review](semantic-review-summary.json) retain every raw-field
count, exact binary/input identity, command exit and private artifact reference.
All 169 semantic remainder leaves are explicitly accounted for, with zero
unexplained leaves and no erased fields. Complete original canonical captures,
receipts, source modules, tasks and witnesses remain private.

Historical corpus build coordinates are reproduced from all 185 complete Rust
`Debug(unit, cwd, Clang)` tuples against all 4,834 rows on each side. The sole
coordinate change is the independently held invocation directory. Three entire
source-effect records also reproduce after exact configuration, recursive
serialized identity, typed task, obligation and witness digest recomputation.
Three empty-summary `__TIME__` key changes reproduce from exact source/configuration
preimages with wrong-time negatives. [Producer provenance](producer-provenance.json)
records these boundaries.

Every original BASE and TIP reading receipt replays against its complete actual
canonical environment and unchanged graph. Two `cio_chunk_open` observations
retain real inferred-table changes for source-Safe internal `cio_chunk_is_file`
and already-Safe `cio_version` at different inference phases. Their complete
payloads remain semantic differences; no function entry or source atom is
removed to manufacture parity. Source binding stays **Open**, with no truth
transport. Faithful extraction and Rust–Lean correspondence remain premise 28.

The accepted focused set is 60 distinct Rust tests (3 new unit, 6 new source,
51 existing descriptor/alias/chain controls), Clippy/fmt exit0 and ten audited
Lean theorems. [Focused receipts](focused-receipts.json) retain all attempts.
The [shared frozen gate](shared-gate.json) passes all eleven stages: 8,475 Rust
passed, 15 ignored, zero failed; 362 Lean theorems, no placeholders or project
axioms; fmt, Clippy, probes, examples and release exit0. This is the joint
integration gate. The integrator's regenerated committed matrix and final
Python receipt remain the final acceptance step.

[Retained attempts](attribution-attempts.json) include strict historical/canonical
mismatch checks and report-helper preparation/compilation failures before their
exact producer/schema corrections. [Earlier validation attempts](validation-attempts.json)
retain two corrected new-fixture expectations, the excluded host-ENOSPC corpus
attempt (analyzer signal exit−2, wrapper 254), and a queued focus job terminated
143 before compilation. The successful strengthened corpus retry records every
24,991-file archive input, both external databases and complete package files
before/after. Earlier completed configured routes keep their narrower initial
per-file guards and honestly timed [archive postcheck](runtime-input-postcheck.json).

[Refreshed ranking](ranking-receipt.json) keeps 2,215 candidate bodies, the
conditional ceiling of 362 and baseline closure 6/6. Positive priorities stay
unchanged; `mk_stop` has one fewer own alias leaf, with score/freed callers still
zero. Logger score3 and scheduler score2 remain blocked on actual callback,
ownership and TLS transport; cJSON still needs string membership and Removed
result transport. Planning ceilings are conditional, not promised Safe gains.
Keep `W48-BOTTOM-UP-RESIDUAL-PLAN` open and close only the scalar-local identity
leaf. Pointer/status carriers and unsupported reads/casts/stores/effects remain
separate obligations.

[Integration amendments](integration-amendments.json) provide exact proposed
STATUS, part11 increment and matrix wording. Root owns those shared ledgers,
report index, final Python acceptance and main/remote integration.
