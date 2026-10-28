# Ordered removal summaries and cross-unit lifetime replay

The implementation is complete on `codex/event-ordered-lifetime`. Its semantic
source and native measurement tip is `d313cc177dfe666fe51bd87b4fa2135e600bfcb7`;
subsequent changes contain report/evidence records. The branch starts at
caller-removal merge `c265a31fadf7e69933fb106ce08408789944a26e`. That preceding
work landed, published and pushed to main at
`facdb5f4ec1806c38f09f953f4c5b2ae463386e5`, which supplies this comparison's
published baseline. The difference between that merge and publication is
integration documentation, not checker semantics.

## Capability and limits

Complete ordered traces retain at most 4096 primitive register, remove, destroy
and release steps. Calls substitute exact source-qualified paths and replay the
trace against incoming registrations. A later matching removal cannot clear
wrong-holder poison or an earlier release obligation. Only identical traces on
all exits retain ordered discharge. Missing/open traces, unsupported control
flow, lifted actuals and unstable holder snapshots retain the conservative
may/must summary. Same-unit ordered transport requires internal linkage.

The actual C caller route additionally infers removal-only helpers. Their
complete CFG must be one acyclic single-successor path of removal calls and
exits, with exact formal operands, borrowed/none custody and typed parameter
slots without volatile/atomic qualifiers. Extra calls, writes, branches, aliases,
releases and mixed effect classes refuse. Calls require a static same-unit
helper or an independently replayed external home. Ordinary W4 publication
rejects removals; the richer channel preserves their order. A removal-only
helper can receive member pointer values without receiving the caller's whole
record. Borrowed custody alone supplies no heap frame for a whole-record handoff.

`ResourceContracts.lifetime_imports` is separate from ordinary borrowed imports
and locally inferred summaries. Cross-unit admission independently reconstructs
the selected home source, scope, request/environment identities, link premise,
Safe row and complete effects before caller substitution. Missing, changed,
duplicate or chained homes and pre-seeded local inference refuse. Removal traces
are rederived from the home CFG. Unaware lookup/frame/custody routes withhold
these imports. The schema moves from 22.0.0 to 23.0.0 so an older reader cannot
silently interpret the new effect-bearing record as ordinary borrowed custody;
all path dependencies/locks and all five provider packages were checked. No
provider-owned `.qv` contract changes in this card.

The registered `OrderedLifetimeReplay` Lean audit checks 16 theorems covering
sequential composition, sticky refusal, exact/poisoned/unmatched removals,
independently expected home/effect identity, missing/empty imports, identical
joins, complete bounds, ordinary-borrow exclusion, mixed-vector refusal,
unresolved externals and supplied local inference. Source extraction, declared
API semantics, link selection and model/Rust correspondence remain assumptions.
These are bounded model and selected-source checks, not automatic proof of
arbitrary backend bodies or whole-C memory safety.

## Native observation

The [brief](BRIEF.md) and [forecast](PREDICTION.md) precede semantic edits.
The forecast is zero resource/NULL verdict movement on all three live routes.
The official `quirin-audit measure --declared` comparison exited 0;
all three declared routes match, with no unmatched rows or conflicts. Measurement uses the published `facdb5f4e` baseline and exact
source `d313cc177`, unchanged input receipts, pinned route configurations and
`SOURCE_DATE_EPOCH=1767225600`. Exact commands and inputs are retained with the
[evidence receipt](evidence/native-receipt.json).

| Route | Resource rows | Safe / Unknown / Pending / provisional Unsafe | NULL rows | Safe / Unknown / provisional Unsafe |
|---|---:|---|---:|---|
| corpus | 4834 | 1013 / 3811 / 0 / 10 | 4921 | 2549 / 2355 / 17 |
| system | 448 | 101 / 343 / 0 / 4 | 454 | 211 / 243 / 0 |
| slice | 404 | 57 / 342 / 4 / 1 | 406 | 190 / 216 / 0 |

These are the existing published resource/NULL properties, semantic profiles,
selected scopes and bounds; the full verdict contexts remain in the archived
artifacts. Unsafe entries are provisional projection observations, not claims
of upstream defects. No native safety-coverage gain is claimed. Unknown and
Pending remain unresolved.

The complete comparison covers 25 selected artifacts per side. Corpus and
slice have no non-provenance payload delta. System has 69 field deltas accounting
for six already-Unknown scheduler rows (three functions in two configurations).
`flb_sched_timer_cb_disable` is removal-bearing and no longer publishes an
ordinary W4 borrow summary; the affected rows drop that summary and its
transitive `mk_event_timeout_destroy` dependency. The resulting diagnostics and
unit/component aggregates change (`unsupported` +8, `unknown_alias` -2), and
the declaration dependency ledger loses exactly those six row contexts.
Component assumptions remove only `summary:flb_sched_timer_cb_disable`.
The complete reviewed before/after patch is retained. No verdict changes.

All NULL carriers (4,921/454/406 rows), runtime reports, source/Boolean effect
payloads, graph/atom bindings, bound-reading multisets and captured effective
environments remain equal after the checked provenance renewals. The corpus
check independently reconstructs cwd-dependent build/configuration identities
from 1,403 compile-entry preimages and checks six before/after canonical source
tasks, including their import, task, obligation, bounds and coverage identity
chains. These checks confer no new premise truth or whole-source proof.

Every selected raw before/after artifact is retained as deterministic gzip with
its uncompressed SHA256/length and a decompression round-trip check. The receipt
covers 68 logical files in 53 unique gzip artifacts (74,301,831 compressed bytes),
with focused attempt archives retained separately. The
[evidence receipt](evidence/native-receipt.json) names exact pairs and receipts.
[Independent attribution](evidence/native-attribution.json.gz) compares every
payload field; it normalizes exact expected evidence revisions, recomputes reading-site IDs,
and compares those ID-sorted receipt collections as multisets. Corpus cwd-derived
build/configuration identities are recomputed from the same raw unit/options/Clang
description under both recorded working directories; the dependent import/task/
obligation IDs are recomputed too. Their exact preimages are archived. This is
identity checking, not an attestation of Clang or discharge of source assumptions.
Environment bytes are SHA256 checked, and complete non-provenance deltas are
pinned with before/after values in the reviewed ledger. No diagnostics, graphs,
atoms, NULL carriers or summary fields are silently suppressed.

Recheck the archive from the repository root:

```sh
python3 docs/reports/event-ordered-lifetime/evidence/validate-native.py \
  --receipt docs/reports/event-ordered-lifetime/evidence/native-receipt.json \
  --reviewed-deltas docs/reports/event-ordered-lifetime/evidence/reviewed-deltas.json.gz
```

Private native logs are under `private/event-ordered-native-20261028/`; the
exact command receipt is
`private/codex-event-ordered-worktree/private/ordered-lifetime/native-command.json`.

## Final standing gate

The full source/provider gate ran on the same semantic tip `d313cc177` and all
commands exited 0. [Exact argv, exits, counts and private log paths](evidence/gate-steps.json)
are authoritative. The full selection was regenerated from the final source at
`d313cc177`. The final run reruns every required source/provider command; no
superseded partial step is credited here. Clang was captured in this worktree for this run.

| Command group | Result (all exits 0) |
|---|---|
| Submodule status | One initialized, pinned submodule; no movement |
| Root fmt and Clippy | Clean; all targets, warnings denied |
| Full `run_test_binaries.py --jobs 16` | 8,842 passed / 0 failed / 15 ignored; 573 units, 592 summaries, 24 filtered |
| Both reevaluation examples | Both passed |
| `check_lean_audits.py` | 390 audits, no placeholders/project axioms |
| Provider package build + all-package verification | All five packages passed; libevent verification also passed separately |
| Provider workspace `cargo test --workspace --locked --offline` | 133 passed / 0 failed / 1 ignored; 43 summaries |
| Provider fmt and Clippy | Clean; warnings denied |
| `python3 -m unittest discover -s scripts/tests` | 554 passed, no skips |

Focused final controls also pass: 12 gateway tests and the audit link-selector
home-to-caller certificate test. Safe reduced C outcomes independently replay
their certificates; altered home/effect, early release, wrong-holder poison,
conditional/unresolved calls, volatile slots and supplied local inference stay
refused. Private full gate logs:
`private/codex-event-ordered-worktree/private/ordered-lifetime/final-gate/`.
Records-only Python admission after the completion commit is retained privately
at `private/codex-event-ordered-worktree/private/ordered-lifetime/records-python.log`
with its exit file; that postcommit result is a separate records checkpoint rather than part of the semantic-source gate.

## Earlier attempts and remaining work

[Focused attempt receipts and logs](evidence/attempts.json) preserve initial
failures. The first Lean build used a reserved binder and nonexistent theorem
accessor; corrected runs pass without weakening statements. The first Rust
control incorrectly expected a whole-record `starts(c)` handoff followed by
member removal to be Safe; the existing holder snapshot rule correctly refused
it. The new expectation was corrected to explicit refusal, with no existing pin
weakened. A removal-carrier batch then passed 10 tests and failed one member-
actual admission; extending the explicit effect-aware argument reader corrected
it. The final 12 gateway controls include the later linkage, volatile-slot and
local-seed guards. Initial carrier compilation also failed on module paths and
optional formal inventory handling; corrected compilation/Clippy pass. The first strict attribution check also refused renewed corpus build identities;
the retained preimage checks were added before admitting them as provenance.
Ad hoc provenance helper builds encountered incompatible cached rlib/serde
variants; an isolated Cargo graph resolves those dependencies. Their private
`provenance-build*`, `provenance-run*` and `validator-candidate.log` checkpoints
are separate from the source gate. Private
`private/ordered-lifetime/removal-carrier-check*.log`, `removal-clippy*.log` and
superseded partial directories `full-gate/` and `pre-native-gate/` retain those
checkpoints. Sandbox submodule initialization failed on DNS; host initialization
succeeded at pinned `d82464a277d0f42703702c4dfd9af6af38595a83`.

Whole-record holder frames, collector-by-id identity, conditional per-record
host/loop correlation, derived path equality, all-path threaded join/destruction,
descriptor custody and automatic backend contract-body validation remain open.
Mixed registration/removal C import vectors also remain refused; the separate
ordered abstract trace does not supply a C heap frame. The proposed
[integration amendments](integration-amendments.json) append
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` rather than retiring it. This follow-on
branch has not been merged into main.

Likely integration conflicts are the shared acceptance matrix, report index and
contributions page; schema path dependencies and three lockfiles; generated
gateway guidance and Lean roots; and the legacy summary/cross-unit source
modules. Regenerate generated files after reconciliation. Check the proposed
amendments with `python3 scripts/apply_amendments.py
docs/reports/event-ordered-lifetime/integration-amendments.json --check`; their
application belongs to a later integration of this branch.
