# Evidence-only contract hypothesis graphs

Work in progress on `codex/contract-hypothesis-graph-v1`, based on
`198f6fc4349d3a47591a2378835ae9286859d918`. Design and the zero-movement
prediction were committed as `1a2e40c95` before implementation. The current
frozen implementation source is `69f13d6b9`; no completion or corpus-parity
claim is made until the receipts below are filled from completed commands.

The schema adds declaration-local opaque atoms, conjunction-only premises,
separate disposition and warrant, complete instance contexts and evidence
references. `qcore-schemas` moves additively from19.0.0 to19.1.0; existing
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

## Validation in progress

Private evidence root:
`/home/cosmo/.codex/worktrees/3d1b/qcore-c/private/contract-hypothesis-graph-v1/`.

The full standing gate runs in `gate-source/`, a private clone of the exact
committed card with the pinned Libevent submodule initialized. The ordinary
worktree's broader focused test hit an existing fixture assumption that
`.git/modules/third_party/libevent` exists below a directory `.git`; a managed
worktree instead has a `.git` file. The clone preserves the tested source
without changing that fixture. Its actual Clang recording and command receipts
are local to the clone's run. Final Rust/provider/Python/Lean counts and corpus,
system, slice, null and runtime erasure comparisons remain pending.

Superseded attempts are retained: malformed initial custody fixture syntax
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
