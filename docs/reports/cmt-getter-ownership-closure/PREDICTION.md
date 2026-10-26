# Prediction: full CMetrics getter ownership chain

Semantic BASE is `3e3d707bf7d8d12fb29d6cf6b0e690561ef80d64`.
The fresh corpus and bottom-up planner both exited 0 with stable binary, input,
source and planner identities before this prediction. The corpus has 4,834
functions: 1,013 Safe / 3,812 Unknown / 9 provisional Unsafe. The planner has
2,189 bodies, baseline same/cross closure 6/8 and conditional ceiling 384. The
complete identities and timings are in the [baseline receipt](baseline-receipt.json).
No semantic Rust, Lean, `.qv` or tracked-tool change precedes this commit.

The full-chain source review found a reachable ownership violation, rather than
a remaining premise under which the root can become Safe. The NUL-free eight-byte
string `42 d5 68 e1 38 d7 27 ff` has both one-shot and streaming XXH3-64 digest
zero in the pinned CFL implementation. It is a valid CMetrics metric name when
namespace and subsystem are empty; one empty label leaves the exact same stream.
The public histogram path then calls the map getter with one label and write mode.
`metric_hash_lookup` selects the embedded metric because the digest is zero,
`metric_prepare_storage` allocates its histogram storage, and neither the dynamic
primary list nor `metric_static_set` is changed. `cmt_map_destroy` releases
embedded storage only when `metric_static_set` is true. The recorded native
witness observes allocated embedded storage, `metric_static_set == 0`, an empty
dynamic list, and no free of that allocation during destruction.

This is the full root counterexample: constructor representation, valid input,
vendored hash state/effect, embedded selection, optional storage allocation,
primary-versus-secondary state, returned map borrow, and teardown all take part.
The implementation will not replace unresolved branches with an assumed neutral
summary. It will add a validated lifecycle-violation diagnostic only after a
generic source extractor establishes every edge of this graph and the exact
hash model establishes reachability. All prior diagnostics remain in the row;
under the verdict join, the validated violation decides the root as Unsafe while
retaining those Unknown facts as evidence.

Ex ante forecast:

- `cmt_map_metric_get` changes from Unknown to provisional Unsafe in corpus,
  system and slice. Corpus is predicted to become 1,013 Safe / 3,811 Unknown /
  10 provisional Unsafe. System is predicted to become 101 Safe / 342 Unknown /
  5 provisional Unsafe. Slice is predicted to become 57 Safe / 341 Unknown /
  4 Pending / 2 provisional Unsafe. Any additional movement requires complete
  attribution; no existing Safe result may regress.
- The internal helper rows remain Unknown because the implementation preserves
  every pre-existing unsupported/alias diagnostic. Their fresh planner leaves
  remain 6 (`metric_prepare_storage`), 4 (`metric_index_add`), 14
  (`map_metric_create`), 20 (`map_metric_get_unlocked`), 21 (`cmt_map_create`),
  9 (`cmt_map_destroy`), 3 (`cmt_map_metrics_expire`) and 1
  (`cmt_map_metric_get`) before the new lifecycle diagnostic. The decisive
  counterexample does not manufacture Safe summaries for those bodies.
- A generic ownership-lifecycle graph in the gateway will distinguish primary
  owned children from secondary index/cache/backpointer aliases, embedded
  storage guarded by a representation flag, committed optional allocation,
  borrowed results and destructor coverage. A source adapter may instantiate
  the graph only after checking the complete relevant bodies and call edges;
  changed branches, fields, callees, conditions, list roles, hash APIs or cleanup
  paths decline to the old result.
- Matching placeholder-free Lean will prove the graph invariants, borrow
  invalidation, exact XXH3 short-input zero witness, and the reachable
  destruction leak. Canonical CFL and CMetrics `.qv` files will state the
  mutating hash effects and the now-concrete nonzero-digest safety precondition.
  They remain declared models unless an independently body-checked interpreted
  interface is established; model parsing alone grants no call permission.
- NULL results, unrelated resource rows, runtime output and contract dependency
  sets are expected unchanged except for real consultations and the new root
  evidence. Hash/API source identities, negative source controls, flow controls,
  allocation/free interception and model vectors must all reproduce.

System and slice BASE measurements continue against the immutable BASE binary
and 314-file snapshot. They cannot observe later implementation edits. Paired
TIP measurements will use the same snapshot. This preserves fresh corpus
BASE/planner → this committed prediction → semantic implementation ordering.

Acceptance remains the full brief: Rust, registered audited Lean and truthful
`.qv` parity; the complete source and native counterexample; fail-closed negative
controls; standing root/provider/package gates; full paired attribution; exactly
one increment and current status/tracking amendments. The result is a validated
root Unsafe for the pinned ownership property, not a whole-Fluent-Bit safety
claim or an assertion that every other helper path is modeled.
