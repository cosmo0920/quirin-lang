# Current status and evidence boundaries

Re-derived on 2026-09-21 from the sources below. This is the concise status
authority routed by `AGENTS.md`; reports own the underlying evidence. Update
numbers from those sources, not from an older summary. History belongs in the
[increment records](development/increments/README.md).

## Coverage

| Measure | Recorded value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **50/50**; none unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT case-bound proof | **5/50**: CT01, CT03, CT07, CT37, CT50 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/50, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Cycle 19 units | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**, 185 total | [W42 priority corpus](reports/w42-priority-integration/README.md) |
| Projected functions | **223 Safe / 84 provisional Unsafe / 4,527 Unknown**, 4,834 total | Same baseline |
| Corpus diagnostics | leak **272**; unknown_alias **4,665**; double_release **0**; unsupported **49,371**; summary_conflict **1** | Same baseline, `artifacts/diagnostics.json.gz` |
| Unit reasons | **136 unsupported-syntax / 1 incomplete-obligation-coverage / 48 without a reason code** | [Combined attribution](reports/w42-priority-integration/attribution.json) |
| Raw source coverage | **4,921 definitions / 4,834 projected / 87 pinned-build exclusions**, across **24 full-path units** | [Verified census](reports/w42-unprojected-definitions/README.md); combined inventory unchanged |
| Monkey callback accounting | **3/3 accounted, 0/3 Cycle 18 composed, 3 explicit Unknown**; 8 source paths | [Callback report](reports/w42-monkey-callback-coverage/README.md); ordinary audit checks 0/8 native paths, optional focused replay 8/8 |
| Observed Monkey timed wait | **1 instrumented Linux epoll success run**, 11 identity records and 11 model operations; conditional replay, no coverage promotion | [Observed integration](reports/cycle18-observed-integration/README.md) |
| Cycle 18 priority-one checks | **8 failure cases** (3 fault-model Unsafe / 5 Unknown); **4/4 payload paths** under explicit authority; **15-operation timeout teardown** | [Bounded integration](reports/cycle18-priority1-integration/README.md) |
| Cycle 18 bounded aggregate | **3 joined families / 8 source paths / 7 unresolved obligations**, aggregate Unknown | [Broader integration](reports/cycle18-broader-integration/aggregate.json) |
| Callback task handoff | **12 source-derived operations**, one Linux pthread fixture; production acceptance pending | [Integrated lifetime evidence](reports/cycle18-broader-integration/task-handoff.json) |
| Current native platform matrix | **4 Linux Observed / 3 foreign-host Unknown**; generic Libevent probe only | [Bounded platform evidence](reports/cycle18-broader-integration/platform.json) |
| Contract corpus | **96** modules; **59** resource-v1 declarations; **33** manifest-selected | [Provider ownership](../providers/README.md#contract-ownership), canonical index `contract/corpus.json`, [selection](../cycle19-resource-contracts.json) and pinned tests |
| Chunk lifecycle results | **0 Safe / 96 Unknown** over **96** selected `flb_input_chunk.c` and `cio_chunk.c` functions; nine outstanding composition legs each; 2 carry a path-incomplete composition from their own pinned unit | [Chunk source profile](reports/w43-chunk-source-profile/README.md), `lifecycle-coverage.json`; [lifecycle certificate](reports/w42-chunk-lifecycle-cert/README.md) |
| Registered follow-ups | **277** | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |

**Mapping is not conformance.** CT01/CT03/CT37/CT50 bind their model proofs
through the F02 trace/digest relation; CT07 binds through the scoped-task lease
ledger, not F02, whose lease refusal remains intentional. CT50's binding grew
no alphabet, and its closure over admitted renderings is the degenerate case —
a single-actor run, proved to admit nothing but itself rather than assumed to.
Every proof row is evidence about one case's artifact; **CT50's covers the
capability half of its scenario only**, because the numeric address reuse that
case is named after is invisible to an alphabet carrying generations and stays
native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`). No F02-shaped
candidate remains among the mapped cases (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`,
measured). Since wave 41 each leg's pinned digest is compared with the value its
Lean side decides, total-or-reject both ways. Native execution is observed,
not proved. The sanitizer count carries its host condition; emission assertions
can run on other hosts without implying sanitizer execution there.

**Raw units remain unpromoted.** `flb_input_chunk_create` is function-level Safe;
the production `input_chunk_get` residual remains Unknown. Function-level safety
and the chunk **lifecycle** result are separate dimensions: no selected chunk
function's lifecycle post-state is established, `flb_input_chunk_create`
included, and a checked field contract is not a lifecycle result. Provisional Unsafe
findings are not validated upstream defects. Unknowns stay in the denominator.
The baseline is `docs/reports/w42-priority-integration/artifacts/`; an arbiter must
reproduce its predecessor's verdict-bearing artifacts byte-for-byte and
attribute every delta. The chain and leaf identities are preserved in the
individual reports, [wave history](development/increments/README.md), and
[gateway rules](agents/gateway.md#constraint-7).

## Milestones and trust

Cycles 1–17 are complete only at their bounded milestones. Cycle 18 has bounded
implementation with aggregate acceptance pending. Cycle 19 is in progress;
per-path identity imports and observation evidence gate promotion. Cycle 20 is
active: all v1 Safe proposals are refused, including empty scopes; unchecked
Unsafe proposals downgrade to Unknown (`QP010`).

Lean gateway theorems are abstract rule theorems, not extraction of the Rust
analyzer or whole-C correctness. Faithful C extraction and Rust–Lean
correspondence remain premises maintained by review. See
[trust boundary 28](agents/evidence.md#constraint-28). Report model proof,
artifact binding, source coverage, native observations and host conditions
separately.

## Latest validated gate

The [task context and contract bundle gate](reports/task-context-contract-bundle/gate.json)
records **5,043 core Rust passed, 0 failed, 11 ignored; 165 Lean audits; 336 root
Python tests; 123 provider Rust tests**. Core/provider fmt and clippy, both probes,
five isolated host controls, the isolated package mutation control, five package
builds/verifications and five package Python tests pass. Clang was measured in
this checkout. [Commands, identities, exits and logs](reports/task-context-contract-bundle/README.md#validation)
preserve failed attempts and their corrected reruns. All 96 contract sources and
five package identities match the preceding checkpoint. The focused production
CMetrics replay preserves report bytes with two attributed provenance changes;
it does not rerun or promote the 185-unit baseline.

The five provider crates own **72** canonical contract modules; **24** shared or
application-level modules live in `providers/shared/contracts`. `contract/corpus.json`
indexes all **96** without duplicate snapshots. Retained integration adapters
embed the provider-owned data directly; generic core crates retain no provider
executable dependency. Package inventories bind supplied contract bytes to their
executables; policy selection and independent checking remain explicit. This
changes source locations, with no predicate or verdict promotion. The bundle
pilot adds independently pinned data-only selection for the CMetrics policy,
retains version-1 paths and existing adapters, and models admission separately
from policy truth. Four scoped follow-ons remain in its report.

All three broader Cycle 18 follow-ups remain open; **282** follow-ups are
registered — `W42-CHUNK-LIFECYCLE-CERT` retired against its own report and two
narrower lifecycle residuals took its place. Production callback composition, task/cancellation/event-generation
coverage and current foreign-host observations remain outstanding. Callback,
CT and full-conformance coverage are unchanged.

The ignored F02 Windows pin-agreement test is red by design on `--ignored`;
Linux green does not repair it. Host/load-sensitive tests and their reporting
requirements remain in [rule 16](agents/providers.md#constraint-16).
The summary-count and Clang recording mechanisms still have their disclosed
limitations; see [validation policy](agents/validation.md). This integration
does not discharge those measurement-mechanism limitations.

## Where to go next

[Planning and registered work](README.md#planning) ·
[Rule router](agents/README.md) · [Platform evidence](development/platform-evidence.md).
The pre-restructure ledger narrative is preserved as a
[historical checkpoint](development/increments/structure-checkpoints/status.md),
not a second current status authority.
