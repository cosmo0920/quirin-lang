# AUDIT-IR: bounded component execution for existing-C verification

Status: implementation started; the complete IR and memory target remain open.
Base: `d03f6c864f84cfc837db09f7bdb0a2292bc32aab`.
Initial implementation/evidence: [AUDIT-IR-PLAN](../reports/audit-ir-plan/README.md).
Fragment implementation/evidence: [AUDIT-IR-FRAGMENTS](../reports/audit-ir-fragments/README.md).
Lifetime implementation/evidence: [AUDIT-IR-LIFETIMES](../reports/audit-ir-lifetimes/README.md).

## Objective and acceptance

Make a cold, complete existing-C audit practical on an 8 GB Linux runner:
target at most **6,000,000,000 bytes** during ordinary execution and a hard
acceptance ceiling of **7,000,000,000 bytes** for the entire audit job,
including concurrently running Clang/provider/worker processes. These are
decimal bytes, not GiB and not a sum of per-process lifetime RSS maxima.
Measure the peak of one dedicated cgroup, including charged file cache;
record `memory.peak`, `memory.events`, elapsed time and the exact inputs.
Budget measurement must include preparation when the advertised command
includes it. Measure build/toolchain setup separately and disclose that boundary.

The target applies to Rust-based C auditing. Lean elaboration and proof audits
are separate development validation, not part of this user's audit-job memory
budget. This card does not remove the repository's existing proof obligations.
No verdict, diagnostic, selected target, assumption, source identity, dependency,
contract standing or evidence obligation may be lost to meet the budget.

The IR is a semantic working representation, not compressed C text, a new C
compiler, an LLVM backend or a claim that arbitrary C is supported. Reuse
`LegacyCfg`, typed C readings and existing contract/evidence representations.
Keep Quirin source compilation's `TypedCfg` separate unless a later design
demonstrates the same semantics; this card initially owns the existing-C route.

## Representation and ownership

1. A unit manifest binds exact retained preprocessing artifacts, spelled input
   and source transport, compile-task identity (target, compiler record and
   flags), contracts and their retained sources, profile/trust policy, selected
   targets, checker/IR/evidence versions and analysis bounds. Existing recorded
   compiler provenance remains a premise, not executable attestation. IDs are
   local references until authenticated through this manifest.
2. Intern symbols, types, contracts, clauses and evidence objects once per
   appropriate identity domain. IR instructions and dependency edges carry IDs,
   not cloned contract graphs or repeated source strings. Preserve exact source
   attribution, integer widths/conversions, evaluation-order constraints,
   memory/alias effects, call semantics and explicit unsupported readings.
   Use dense integer IDs and vector adjacency/state tables for traversal.
   Name lookup is a construction boundary: temporary hash tables are permitted,
   with explicit sorting for deterministic IDs and output. Keep ordered maps
   where their order is an actual interface requirement; do not replace every
   `BTreeMap` with a hash table or assume a speedup without measurement.
3. Function bodies are independently loadable. A component is a strongly
   connected set of bodies under every dependency the consumer actually needs.
   Direct and supported callback edges remain included; unresolved dispatch is
   an explicit boundary, never evidence that the graph is complete.
4. The execution plan retains lightweight symbols, forward/reverse edges,
   component membership and callee-before-caller order. Recursion stays within
   one component; existing ranked/neutral validation and declared precedence
   remain unchanged. Planning alone proves no summary or safety property.
5. Load one admitted component's body facts at a time. After validation, persist
   summaries and evidence and release body ASTs, CFGs, temporary states and
   buffers that no remaining consumer needs. Define leases for readers needing
   typed facts later; do not drop facts while a certificate replay still needs
   them. A larger-than-budget component must fail explicitly with its affected
   targets retained as Unknown, never silently truncate or count as a successful
   complete audit.
6. Persisted fragments are versioned, length-bounded and content-addressed;
   validate the caller-held expected manifest, exact dependencies and complete
   selected inventory before reuse. An unavailable, stale, corrupt, partial,
   ambiguous or incompatible cache entry supplies no evidence: rebuild from
   authenticated inputs or retain Unknown. Write atomically and reject partial
   writes. The cache is disposable and cannot be its own trust authority.
7. Reverse indexes answer invalidation/impact queries. A change to source,
   configuration, contracts, profile, policy, bounds or checker identity
   invalidates every affected transitive consumer. External/unknown dependencies
   must prevent optimistic reuse. Cache eviction changes cost, never meaning.

## Ordered implementation slices

### AUDIT-IR-PLAN

Extract an interned, iterative component planner in `qcore-c-gateway`, consume
the existing direct/callback call graph and connect its exact component order
to W4 summary derivation. Retain non-body dependency names and forward/reverse
edges without treating them as analyzed bodies. Preserve parity graph output.
Release the temporary string-keyed graph after conversion. No disk format,
cache-based admission, body eviction, CLI memory flag or memory-reduction claim
is part of this slice.

Validate exact ordered SCC parity against the original algorithm for every
directed graph on up to four vertices, including self edges; test empty graphs,
external boundaries, reverse indexes and a deep chain without recursive DFS.
Keep the original algorithm as a test oracle; archive comparison results.
Existing summary, recursion, callback and premise tests exercise the production
consumer. This is representation/scheduling work; inference and validation rules
remain unchanged, with their existing audited Lean models retained.

### AUDIT-IR-FRAGMENTS

Specify a versioned unit manifest and lossless body envelope around existing
readings/CFGs, including facts presently consulted outside the CFG. First prove
round-trip identity and bounded/total-or-reject loading on fixtures, then attach
an opt-in artifact writer/reader to the real audit path. Do not claim a CFG alone
replaces source/typed facts. Preserve complete evidence and refusal records.

The initial implementation is observational. For a configured JSON audit,
`QCORE_AUDIT_IR_DIR=DIR quirin-audit audit.json` captures unit context,
construction tokens/environments and typed CFG objects under `DIR/objects/OBJECT_ID.qc`,
with completed unit inventories under `DIR/units/UNIT_ID/INDEX_ID.qc`. All persisted IR files use the
`.qc` extension, including temporary publication files. This first version uses
canonical typed JSON encoding inside `.qc` files. The storage
directory is not an analysis input identity. Existing reports and verdict
paths remain unchanged; unset means no capture. This selection currently
wraps configured units, not every independent CLI source route.

Format `quirin-audit-ir-cfg/1` uses exact typed serde JSON encoding, lowercase
SHA-256 IDs, at most 64 MiB per object and at most 32 nested IR instructions.
JSON readers keep serde's default depth limit. These are artifact limits,
not changes to C grammar, Clang transports or analysis bounds. Every CFG
read checks its unit/construction/token/environment references and requires
equality with independently fresh lowering. File publication uses a synced
temporary file and an atomic no-overwrite hard link; filesystems that cannot
provide that operation refuse capture. A failed capture aborts the opted-in
audit and publishes no new completed unit index. A successful unit inventory
names missing targets and refusals explicitly, and claims no safety coverage.

Typed source facts and run-scoped facts outside the CFG still require the
original readers. This is not source-free replay or a speed/memory mode;
body eviction and independently authenticated cache admission remain later work.

### AUDIT-IR-LIFETIMES

Initial implementation bounds Clang reuse to each configured audit unit,
including its certificate rechecks, and to each configured source-contract
unit. Clear optional whole-unit AST, record-layout and filtered-AST caches on
entry and exit, including errors and unwinding. Active readers keep explicit
`Rc`/`Arc` leases; eviction never invalidates them. After global declared-body
and dependency evidence is recorded, discard unselected contextual CFGs and
move each selected contextual CFG into its final row analysis instead of
retaining another full copy. Inferred contract summaries remain live. The three contract-settlement routes
keep one borrowed-or-owned active environment: each successor replaces and
releases its obsolete owned predecessor, preserving the original caller input
and the ABI/guarded/slot/detach/reference/callback order.
This does not yet release the CFGs held by final function summaries or support
source-free certificate replay. Other audit routes and per-component AST
loading remain to be addressed and measured.

Introduce component leases and transactional summary/evidence publication;
release all obsolete full-unit representations. Preserve replay inputs on disk.
Measure bytes and lifetimes separately for contracts, AST/Clang output, CFGs,
analysis states, caches and final report products. Demonstrate cold-cache and
warm-cache equality and fault/cancellation behavior before default adoption.

### AUDIT-IR-BUDGET

Add a user-visible audit memory budget with bounded concurrency and bounded
caches; account for child processes and oversize components. Establish the
6 GB target / 7 GB ceiling with fresh cgroup measurements on pinned corpus,
configured system and slice selections, both cold and warm. Record elapsed
time to expose a slowdown bought by serial execution. Only then add a standard
runner regression job; do not make Lean CI a prerequisite of user audits.

## Compatibility, verification and evidence

Prediction before implementation: AUDIT-IR-PLAN changes zero component/member
ordering, recursive classifications, verdicts, diagnostics, selected inventories,
assumptions, dependency records or evidence versions. No contract or pin edits.

Run the [standing gate](../agents/validation.md#standing-per-change-gate), keeping
each command's actual exit, counts and private log path. Add provider gates only
when provider-owned inputs or interfaces change. Semantic changes require
matching audited Lean; no placeholder or project axiom is acceptable.
Before replacing a source-analysis route or defaulting persisted IR, archive
pinned corpus/system/slice full-record comparisons, including null, static,
runtime and consultation records. Projection equality alone is insufficient.
No implementation slice closes the parent card until the full target is met.

Related open work: `W45-CHECKPOINT-JOBS-MEMORY` remains registered; this card's
plan extraction does not discharge memory-aware admission. The
[audit-memory-performance follow-ons](../reports/audit-memory-performance/README.md#remaining-work)
cover buffered Clang output, repeated typed readings and retained final reports;
carry those mechanisms into the lifetime census rather than assuming contracts
are the dominant allocation. AST transport/depth and certificate-scope rules
remain in force. No whole-C extraction theorem is claimed.

Work stays in the current checkout; no delegated worktree, merge or push is
authorized by this card. Keep implementation in the gateway/audit layers and
published portable data in schemas only when a versioned interface is needed.
Report each slice's evidence and open successors under `docs/reports/` and add
one increment paragraph. A blocked measurement is a recorded limitation, not
evidence of the memory target. The parent remains open until all acceptance
conditions above have measured evidence.
