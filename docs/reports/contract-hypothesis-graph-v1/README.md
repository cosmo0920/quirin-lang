# Evidence-only contract hypothesis graphs

Work in progress on `codex/contract-hypothesis-graph-v1`, based on
`198f6fc4349d3a47591a2378835ae9286859d918`. Design and the zero-movement
prediction were committed as `1a2e40c95` before implementation. The current
frozen implementation source is `5b5a8c0f0`; no completion or corpus-parity
claim is made until the receipts below are filled from completed commands.

The schema adds declaration-local opaque atoms, conjunction-only premises,
separate disposition and warrant, complete instance contexts and evidence
references. `qcore-schemas` moves additively from 19.0.0 to 19.1.0; existing
protocol/evidence versions and fields remain unchanged. No typed predicate is
registered. Unknown typed predicates reject instead of becoming opaque text.
The graph remains evidence-only because no separately reviewed typed consumer
exists; supported/derived reporting records cannot discharge safety obligations
or mint provider certificates.

The loader rejects missing children, premise edges/cycles, conflicting IDs,
noncanonical children/JSON, unused atoms and stale replay. Equal repeated atom
declarations coalesce. Legacy records elaborate to distinct reserved atom and
premise IDs and retain their exact erasure. Source/formal renaming preserves
opaque graph symbols. Graph composition requires both independently held
context and a freshly reconstructed expected graph, preventing an attacker
from deleting an atom and recomputing a self-consistent replacement digest.

The gateway lifts its existing actual-consultation closure, including in-unit
summary inheritance and held/withdrawn names, to atom dependencies. Audit
sidecars carry definitions, contexts, open/declared evidence standing and
counts; the null property's consulted legacy premises elaborate after its
existing fold. `erase_hypotheses` removes only additive graph publication.
Legacy standing, exact named-premise semantic readers and runtime outer
coverage remain separate. No outer envelope is widened.

The Lean model is `QCore.HypothesisGraph`, imported by `QCore.lean` and
registered in `AuditHypothesisGraph.lean`. It models flat bipartite graph
acyclicity, canonical graph validity before composition, compatible contexts,
warrant independence, erasure and refusal when an open/refuted atom is
inserted. These are model theorems, not extraction of the Rust loader or a
proof of Clang import fidelity.

## Controls

The [loader/replay controls](../../../crates/qcore-contract/tests/hypothesis.rs)
exercise exact legacy erasure, same/conflicting duplicates, malformed graphs,
canonical wire replay, all five warrants, each context coordinate, stale
content and independently held relational graph identity. Removing an atom
and recomputing a self-consistent digest still refuses against the expected
graph. Inheritance retains parent atoms and rejects conflicting replacements.
These are control families; no separate numeric mutation-campaign count is
claimed.

The [provider firewall controls](../../../crates/qcore-audit/tests/hypothesis_firewall.rs)
reject graph-shaped forged evidence and fabricated Safe results without the
private checked certificate under every warrant. Gateway tests pin the actual
call/summary dependency closure. Audit publication tests cover full erasure,
graph digests and declarations whose legal names collide with metadata field
names. Existing outer-envelope tests and the required full runtime comparison
retain the static/runtime boundary. The [Lean model](../../../proof/QCore/HypothesisGraph.lean)
and [audit registration](../../../proof/AuditHypothesisGraph.lean) retain the
model/import boundary stated above.

## Validation in progress

Private evidence root:
`/home/cosmo/.codex/worktrees/3d1b/qcore-c/private/contract-hypothesis-graph-v1/`.

The full standing gate runs in `gate-source/`, a private clone of the exact
committed card with the pinned Libevent submodule initialized. The ordinary
worktree's broader focused test hit an existing fixture assumption that
`.git/modules/third_party/libevent` exists below a directory `.git`; a managed
worktree instead has a `.git` file. The clone preserves the tested source
without changing that fixture. Its actual Clang recording and command receipts
are local to the clone's run. The final host standing gate on `5b5a8c0f05bef6aece8b57d90a2d405a459c29ea`
passes **8,364 Rust tests / 0 failed / 15 ignored**, 522 units and 535
summaries, formatting, Clippy, both reevaluation examples and **345 Lean
audits**. Rust build/capture/run took 104.037/9.406/403.036 seconds; timings
are shared-host observations. [Commands and exits](gate/standing-commands.json),
[source receipt](gate/standing-source.json), [complete runner summary](gate/rust-summary.json.gz)
and [actual Clang recording](gate/clang-version.txt) are archived here.
Private Rust unit logs are under
`gate-source/private/test-binaries/20261019T221940Z-658616/` below the private
evidence root; other standing logs are in `standing-4/`.

The complete host provider gate passes **133 Rust tests / 0 failed / 1 ignored
doctest**, **25 Python tests**, check, formatting, Clippy, contract checks and
all five rebuilt/verified packages. [Provider commands and exits](gate/provider-commands.json)
name the complete logs in `provider-4/`. Only root post-matrix Python remains pending.

Superseded attempts are retained in [validation-attempts.json](validation-attempts.json): malformed initial custody fixture syntax
(four failing new tests, subsequently corrected); an inheritance ID/formal
collision (five passed/one failed before graph-symbol substitution was fixed);
initial shared Git metadata permission failures (authorized retries passed);
a gate-wrapper relative-path setup error; standing attempt1 cancelled130
before producing a suite count under saturated host memory; provider attempt1
Rust101/fmt1 under moving source, cancelled130 during Clippy; and a concurrent
contract-check probe cancelled130. The first baseline archive build could
inherit the enclosing card HEAD in build.rs and was excluded; it was rebuilt
with explicit base `QCORE_AUDIT_BUILD_COMMIT` before measurement. The cancelled
standing build's logs were preserved before reclaiming only its own29.7GiB
of safe build output. These attempts are not substituted for final evidence.

## Measured erasure

Fresh base/tip system runs both exit 0: **99 Safe / 345 Unknown / 4
provisional Unsafe**, 448 custody functions and 454 null rows. Every invariant
in [the full comparison](measurements/system/comparison.json) passes: complete
static rows/diagnostics/reasons/dependencies/standing, legacy premises,
contract revision, full null records and summary, and runtime projection.
Raw differences and the standard five-artifact comparison are retained beside
both raw archived artifact sets.

After graph erasure and collision-checked analyzer-root normalization, the
only four differing leaves are one empty lifetime-summary identity remap,
published twice (evidence and prepass). The
[exact attribution](measurements/system/provenance-attribution.json) and
[replay harness](scripts/attribute_time_identity.py) reproduce both SHA-256
keys by changing only Monkey's `MONKEY_BUILT` `__TIME__` expansion,
`08:01:09` to `08:12:24` on 2026-10-20 Asia/Tokyo. Both semantic summaries
are `{}`; all other erased normalized fields are equal. This attribution
does not rewrite or suppress those raw differences.

Fresh base/tip tail-pause runs both exit 0: **53 Safe / 346 Unknown / 4
Pending / 1 provisional Unsafe**, 404 custody functions and 406 null rows.
[Every invariant check passes](measurements/slice/comparison.json).
The same exact timestamp-only replay accounts for its four residual leaves,
`08:21:32` to `08:29:10`
([attribution](measurements/slice/provenance-attribution.json)). Graph publication
covers every measured custody/null row and every declaration in each dependency
section: [system counts](measurements/system/publication-counts.json),
[slice counts](measurements/slice/publication-counts.json). These are metadata
counts, not new safety coverage.

Fresh base/tip corpus runs both exit 0: **963 Safe / 3,862 Unknown / 9
provisional Unsafe**, 185 units, 4,834 custody functions and 4,921 null rows.
[Every invariant passes](measurements/corpus/comparison.json); all 5,781 null
rows across the three routes retain their complete old dictionaries after
graph erasure, and every full runtime projection is equal. Corpus graph
publication covers all rows and all 440 dependency declarations
([counts](measurements/corpus/publication-counts.json)).

Corpus retains 1,116 changed `home_environment_sha256` fields: six imports in
each of 185 unit records and six in the sidecar. The
[serialized environments](provenance/corpus-environment/attribution.json)
reproduce both recorded SHA-256 values exactly. Replacing only the analyzer
root prefix makes their bytes identical (79 occurrences); the applied and
published refutation tables are empty in both. The comparer permits this exact
mapping only after hashing both archived environments and checking root-only
byte equality; raw and erased differences remain retained. Its four remaining
empty-summary leaves replay from `08:36:51` to `09:38:50`
([timestamp attribution](measurements/corpus/provenance-attribution.json)).
There is zero unexplained verdict, diagnostic, reason, premise, standing, null
or runtime-projection movement. The corpus comparison's first attempt assumed
the configured route's top-level premise map; the repaired harness reads the
pinned route's per-declaration premises. That failed comparison and partial
output are retained; no analyzer measurement was replaced.

## Follow-on boundary

The graph deliberately remains evidence-only. A typed family needs a separately
registered semantic-consumer card with its closed predicate registry, independent
replay, exact context, Rust/Lean rule and refusing/positive controls before it
can affect a verdict. Opaque text, provider agreement and LLM proposals grant
no authority.

Integrator items pending final reconciliation: preserve the design commit and
frozen source provenance; retain all failed/cancelled attempts; merge shared
status/matrix/index/increment records serially with the sibling card; keep the
private validation clone and build outputs until final evidence is archived.
