# Current status and evidence boundaries

Figures re-derived on 2026-09-27 by DOCS-COMPACT-9 from primary sources: the
Cycle 19 unit, function, diagnostic, unit-reason, uncontracted-call,
indirect-access and return-expression rows from the combined baseline
`docs/reports/w46-external-callee-contracts/integration/artifacts/`, which
W46-EXTERNAL-CALLEE-CONTRACTS's own integration arbiter reproduced with
`verdict_bearing_identical: true` (its predecessor
`docs/reports/w45-container-root-reach/integration/artifacts/`, which
W45-REFUTED-CONTRACT-WITHDRAWAL and W45-AUDIT-SUBCOMMAND-PLUGINS reproduced,
is frozen); the CT rows from the manifest; the follow-up
count from the matrix; contract counts from the tracked provider sources and
their pinned tests. Rows naming another card carry that card's census and were
not re-run here. This is the concise status authority routed by `AGENTS.md`;
reports own the underlying evidence. Update numbers from those sources, not from
an older summary. History belongs in the
[increment records](development/increments/README.md); the per-row chains this
page carried through wave 45 are in the wave-45 status checkpoints
([part 1](development/increments/wave-45-compaction-status.md),
[part 2](development/increments/wave-45-compaction-status-prose.md)), not a second authority.

## Coverage

| Measure | Recorded value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **50/50**; none unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT independent artifacts | **48** distinct source-fixture programs behind the 50 mapped cases (CT01/CT12/CT14 share one); **9** reviewed byte-identical fixture groups | Same manifest, `artifact_identity`, recomputed by `ct_corpus.rs`; [disclosure](reports/ct-shared-fixture-disclosure/README.md) |
| CT case-bound proof | **5/50**: CT01, CT03, CT07, CT37, CT50 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/50, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Verdict vocabulary | **Safe < Pending < Unknown < Unsafe**, composed by the join (four values since W45-USER-CONTRACT-PLUGINS, approved 2026-09-26): **Pending** = not decided, awaiting a named human act (`awaiting-human-review`, `contract-pin-mismatch`, `plugin-claim-uncertified`), never Safe and always in the denominator, its held reason only ever tool-minted and a Pending unit subject to every Safe scope gate (gateway 10i, W45-SR5-SOUNDNESS); the Cycle 19 corpus has **0 Pending units / 0 Pending functions** (no deferred contract, every pin in force, no checker plugin), so the rows below are unchanged | [Spec §L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition), [report](reports/w45-user-contract-plugins/README.md) |
| External audit subcommands | `quirin-audit NAME` runs `quirin-audit-NAME` from the declared plugin path only (never `PATH`, never a built-in's name), out of process with a cleared environment and quotas; **0** subcommand plugins can reach `Safe` or `Unsafe` from the command line (no checker accepts their certificates: an uncertified claim is `Pending`, every failure `Unknown`); **1** shipped plugin, `quirin-audit libevent prepare`, byte-identical to the retired script on every prepared file (147 of 152 same-path files identical, the 5 others CMake/make logs with named differences); corpus verdicts unmoved | [Contract spec C11.2](QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins), [report](reports/w45-audit-subcommand-plugins/README.md) |
| Cycle 19 units | **5 Safe / 8 provisional Unsafe / 168 Unknown / 4 data-only**, 185 total; unmoved since W45-FB-BACKLOG-CUSTODY, which moved `cio_stats.c` Unknown → Safe, `mk_server/mk_http.c` Unknown → provisional Unsafe (a descriptor leak its declaration reader exposes, `W45-MK-HTTP-ERROR-FD-LEAK`, not an upstream defect claim) and `mk_core/mk_rconf.c`, `mk_server/mk_lib.c`, `mk_server/mk_vhost.c` provisional Unsafe → Unknown (their `MK-LIST-ADD-BORROWED-PUBLICATION` leaks withdrawn by its refused-call hand-on repair, not discharged); earlier moves are in each card's report | Baseline `diagnostics.json.gz`; [W45-FB-BACKLOG-CUSTODY](reports/w45-fb-backlog-custody/README.md#3-corpus-arbiter-185-units) |
| Projected functions | **405 Safe / 9 provisional Unsafe / 4,420 Unknown**, 4,834 total, at W46-EXTERNAL-CALLEE-CONTRACTS's AFTER (7 Unknown → Safe relative to nine declared neutral external-callee premises: `fcntl` at `F_SETFD`, `mk_utils_libc_error`, `rd_atomic32_get`, `cfl_sds_len`, five mpack writer primitives; no correction); before it 398 / 9 / 4,427, unmoved since W45-FB-BACKLOG-CUSTODY (9 read-only list traversals Unknown → Safe relative to gateway 5l's stated premises; 4 provisional Unsafe → Unknown, the `MK-LIST-ADD-BORROWED-PUBLICATION` leaks withdrawn, not discharged; `mk_http_error` Unknown → provisional Unsafe, not an upstream defect claim). Each `Safe` is relative to the premises its card names: 78 declared libc/POSIX premises (W43 libc cards), the typed scalar-access rule (W43-INDIRECT-ACCESS-GENERAL), inferred derived-result summaries (W43-RESULT-ALIAS-RETURNED), index traversal **with no bounds claim** (W43-INDEX-TRAVERSAL), the declared printf family (W43-LIBC-PRINTF-FAMILY), W4-validated glibc header bodies (W44-LIBC-RESIDUALS), the declared `memchr` (W44-C11-GENERIC-SELECTION), the declared result type and C02-P's untracked-base premise (W45-RETURN-EXPRESSION-FAMILIES), the declared release premises of gateway 5k (W45-CALLEE-SUMMARY-CASCADE), the sub-object store rule (W45-SUB-OBJECT-STORES) and gateway 5l (W45-FB-BACKLOG-CUSTODY). Soundness repairs moved `LZ4_write16`/`LZ4_write32` (W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE) and `flb_input_chunk_create` (W45-FB-RETRY-LIFETIME: its Safe rested on reading `mk_list_add` as a borrow) Safe → Unknown | Baseline `diagnostics.json.gz`; [combined baseline method](reports/w43-libc-restrict-prototypes/integration/integration-confirmation.md) |
| Corpus diagnostics | leak **34**; unknown_alias **3,572**; double_release **0**; unsupported **41,239**; summary_conflict **1**; invalid_release / use_after_release / counted_user **0 / 0 / 0**; last moved at W46-EXTERNAL-CALLEE-CONTRACTS (unsupported 41,314 → 41,239, unknown_alias 3,571 → 3,572, every leaf attributed in its [attribution](reports/w46-external-callee-contracts/artifacts/attribution/arbiter.json)); before it at W45-CONTAINER-ROOT-REACH (unknown_alias 3,570 → 3,571, unsupported 41,325 → 41,314: its foreign unlink removed 11 `mk_list_del` refusals and one element release and added 2 member null tests); every earlier move is attributed leaf by leaf in its card's report | Baseline `diagnostics.json.gz`, `diagnostic_counts`; [attribution](reports/w45-container-root-reach/artifacts/attribution/corpus.json) |
| Leak classification | of the 271 baseline leaves on the card's base `c1e69bb5`: **139** validated (a), **31** projection artifacts (b), **101** premise or approximation gaps (c), **0** unclassifiable; **252** name their object in a refused statement before the exit and are withdrawn to Unknown with the cast-return leaf, **128** of them validated (a) by reading (`REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`). These classify the card's 271 leaves, not the current 34 (row above); provisional candidates, never upstream defect claims | [Classification](reports/w44-leak-pin-audit/classification.md), [hand-on overlay](reports/w44-leak-pin-audit/handoff-classification.md), [class (a) table](reports/w44-leak-pin-audit/class-a.md) |
| Uncontracted-call refusals | **9,084** leaves on **2,609** Unknown functions, **429** blocked only by uncontracted calls at W46-EXTERNAL-CALLEE-CONTRACTS's AFTER (9,157 / 2,619 / 436 before it); 376 external callees (no definition in any unit) carry 2,206 sites and block **30** functions alone, none more than 3 ([ranking](reports/w46-external-callee-contracts/artifacts/attribution/ranking-after.txt)); on the earlier baseline: **0** to glibc `__bswap_*`, **0** to `_Generic`; by callee body on the baseline: 3,719 same-unit bodies that do not validate (25 of them in provisional-Unsafe functions), 3,154 in another pinned unit (149 with a `Safe` home body), 1,237 header `static` bodies that do not validate, 961 with no pinned body, 86 external-linkage `inline` | `callee_census.py` of [W44-CORPUS-CALLEE-SUMMARIES](reports/w44-corpus-callee-summaries/README.md) run on the baseline with its `definition-sites.json.gz` |
| Condition-call refusals | at W44-BRANCH-CONDITION-CALLS: **292** condition leaves lowered by desugaring (branch 8,287 → 8,018 on Unknown functions); **4,479** call-bearing condition leaves stayed refused, by blocker: indirect call argument 1,686, call-free operand 1,211, uncontracted callee 1,000, type operator 259, assignment 196, other shapes 74, recursive local callee 31, errno read 22; 0 function moves. The branch-condition refusal now stands at **7,678** leaves on 2,161 Unknown functions (70 sole) | [W44 report](reports/w44-branch-condition-calls/README.md), [residuals](reports/w44-branch-condition-calls/residuals.txt); current figure by `blocker_census.py` of [W45-SUB-OBJECT-STORES](reports/w45-sub-object-stores/scripts/blocker_census.py) on the baseline |
| Indirect-access refusals | **16,989** leaves on **3,266** Unknown functions, sole blocker of **419**; the refused shapes are sized in the [typed scalar-access residuals](reports/w43-indirect-access-general/residuals.txt), the [chain residuals](reports/w44-indirect-chain-access/residuals.txt), the [call-argument residuals](reports/w44-indirect-call-argument-access/residuals.txt) and the [sub-object store census](reports/w45-sub-object-stores/census/residual-after.txt) | `blocker_census.py` on the baseline; [W45-SUB-OBJECT-STORES](reports/w45-sub-object-stores/README.md) |
| Indexed resource collections | **27** element operations lowered in **19** corpus functions, **0** on a resolved array (every one keeps its refusal byte for byte); **0** leaves, functions or units moved on main `d2dd61c1`; collections are heap-kind only; at the card's base census the array allocation's `sizeof(*v)` operand was refused in **224** leaves on **211** Unknown functions (4 sole-blocked); W44-COLLECTION-ALLOCATION admits it (204 leaves withdrawn) and no longer refuses 9 of 14 `T **` allocations (7 bind), yet **0** element operations decide under the collection rules on main `70ce6b44` | [Collections report](reports/w43-indexed-resource-collections/README.md), [element reach](reports/w43-indexed-resource-collections/census/element-reach.txt), [census](reports/w43-indexed-resource-collections/census/census.txt) |
| Return-expression refusals | **304** leaves on **248** Unknown functions, sole blocker of **71**; since W45-RETURN-EXPRESSION-FAMILIES no cast-call or type-operator leaf remains, the rest sized in its [residuals](reports/w45-return-expression-families/census/residual-sizing.txt) (pointer-result reads, scalar expressions with a call, comma stores, bound-base or aggregate-result reads); whole-call leaves carry the call classifier's own refusal | `blocker_census.py` on the baseline; [W45 report](reports/w45-return-expression-families/README.md) |
| Typed access-path census (measurement, no verdict) | every one of the **26,028** `IND` / `COND` / `RET` leaves bound to its Clang-typed constructs (0 unmatched); **69,611** paths, **63,698** expressible, every step typed; refusals: pointer-valued store 2,553 leaves, union 1,641, aggregate value 335, call 240, indirect call 185; **1,845** leaves carry no typed access. Frozen prediction: P2 (statements) **38 Safe / 7 Unknown**, P3 (plus conditions, returns, call arguments) **83 Safe / 137 Unknown**, 0 provisional Unsafe; the callee closure adds 1 Unknown (3 Safe with a cross-unit summary route) | [W46-ACCESS-PATH-CENSUS](reports/w46-access-path-census/README.md), [PREDICTION.md](reports/w46-access-path-census/PREDICTION.md) |
| Unit reasons | **167 unsupported-syntax / 1 incomplete-obligation-coverage / 17 without a reason code** | Baseline `diagnostics.json.gz`, `reason_code`; earlier fields in the [combined attribution](reports/w42-priority-integration/attribution.json) |
| Raw source coverage | **4,921 definitions / 4,834 projected / 87 pinned-build exclusions**, across **24 full-path units** | [Verified census](reports/w42-unprojected-definitions/README.md); combined inventory unchanged |
| Monkey callback accounting | **3/3 accounted, 0/3 Cycle 18 composed, 3 explicit Unknown**; 8 source paths; per-callback obligations **51 relevant cells, 17 discharged / 34 open** (was 5 / 46) | [Callback report](reports/w42-monkey-callback-coverage/README.md); ordinary audit checks 0/8 native paths, optional focused replay 8/8; [per-callback composition](reports/w43-monkey-callback-composition/README.md) discharges initialization reachability (3 callbacks) and natural activation (2) from the pinned compile task |
| Observed Monkey timed wait | **1 instrumented Linux epoll success run**, 11 identity records and 11 model operations; conditional replay, no coverage promotion | [Observed integration](reports/cycle18-observed-integration/README.md) |
| Cycle 18 priority-one checks | **8 failure cases** (3 fault-model Unsafe / 5 Unknown); **4/4 payload paths** under explicit authority; **15-operation timeout teardown** | [Bounded integration](reports/cycle18-priority1-integration/README.md) |
| Cycle 18 bounded aggregate | **3 joined families / 8 source paths / 7 unresolved obligations**, aggregate Unknown | [Broader integration](reports/cycle18-broader-integration/aggregate.json) |
| Callback task handoff | **12 source-derived operations**, one Linux pthread fixture; production acceptance pending | [Integrated lifetime evidence](reports/cycle18-broader-integration/task-handoff.json) |
| Current native platform matrix | **4 Linux Observed / 3 foreign-host Unknown**; generic Libevent probe only | [Bounded platform evidence](reports/cycle18-broader-integration/platform.json) |
| Contract corpus | **114** modules; **183** resource-v1 declarations (elaborated, pinned by `compiler/tests/contract_loader.rs`; 174 → 183 by W46-EXTERNAL-CALLEE-CONTRACTS, nine appended to selected modules); **50** manifest-selected, 4 of them by `crate:module` reference, and exactly the eleven crates' checked resource views (W44-PROVIDER-RESOURCE-VIEW); **13** declarations inherit a libc contract | [Provider ownership](../providers/README.md#contract-ownership) (72 in five provider crates, 42 in six [`quirin-provider-*` shared crates](reports/quirin-provider-shared/README.md#crate-map)), canonical index `contract/corpus.json` (schema 2, 11 roots), [selection](../cycle19-resource-contracts.json) (schema 2), [load path](../providers/contract-load-path.json) and pinned tests; [adoption](reports/contract-inherit-adoption/README.md) |
| Multi-module declarations | **10** functions in **12** module pairs, every pair a registered checked relation: 5 projections, 1 renamed equivalence, 5 source-shape agreements, 1 lifecycle agreement; **0** textual duplicates; 1 view not implied outside a coverage premise (`CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`) | [View projection check](reports/contract-view-projection-check/README.md), `qcore_contract::contract_views::REGISTERED_RELATIONS`, guard `compiler/tests/contract_view_census.rs` |
| Native descriptor observation | **81 pinned Linux runs** (80 accepted, 36 with injected failures), 7,416 records of the pinned Fluent Bit build, Monkey `mk_socket.c`, ctraces `ctr_random_get` and a libc probe; 8 of 10 lifecycle properties hold on every accepted run; release-at-most-once fails in 13 runs (two double closes read against the Fluent Bit source: local observations, provisional candidates, not upstream defect claims) and operand validity in 37 (`close(-1)`); the landed libc descriptor, stream and directory contracts bound to **3,149** executions of 18 declared callees with **0 contradictions**; `ctr_random_get`'s descriptor-0 leak reproduced by a direct driver, not through the binary; **observation, not proof or mapping** | [W43 descriptor native check](reports/w43-descriptor-native-check/README.md), `aggregate.json`, `binding.json`, pinned `records/` |
| Fluent Bit system slice (current build) | **8 files / 15 units: 0 Safe / 2 provisional Unsafe / 13 Unknown**; **237 functions: 28 Safe / 2 provisional Unsafe / 204 Unknown / 3 not projected**, at `046accf6` (re-pinned by the user) through `--selection contract/profiles/fluent-bit/system-selection.json`; unmoved since W45-FB-BACKLOG-CUSTODY. The obligations OR-1 (`flb_output_task_singleplex_enqueue`, under the synchronous-proxy premise) and OR-7 (`flb_output_task_flush`) are checked Fluent Bit provider contracts, each decided **refuted** by its own body (provisional Unsafe candidates, never upstream defects), and every caller call site is held Unknown (`contract-refuted-by-body`, W45-REFUTED-CONTRACT-WITHDRAWAL); OR-6 is not stated and OR-2 not reached; backlog obligations OB-1 … OB-10 are all Unknown, two custody legs held by pinned checked specifications. A separate audit scope, not counted in the Cycle 19 rows above, and no promotion | [W45-FB-BACKLOG-CUSTODY](reports/w45-fb-backlog-custody/README.md) §4–5, [W45-FB-RETRY-LIFETIME](reports/w45-fb-retry-lifetime/README.md) §4–5, [W45-REFUTED-CONTRACT-WITHDRAWAL](reports/w45-refuted-contract-withdrawal/README.md), [W45-CONTAINER-ROOT-REACH](reports/w45-container-root-reach/README.md) §6–7, [W45-FB-SYSTEM-SELECTION](reports/w45-fb-system-selection/README.md) |
| Fluent Bit engine loop census (measurement, no verdict) | **121 loops** in 72 functions of the eight system-selection files at `046accf6`: **115 bounded / 2 decreasing / 1 yielding / 3 unresolved** under named premises (unresolved: `cb_queue_chunks` sb.c:611, the ring-buffer drain flb_input_chunk.c:3746, `random_uniform` flb_scheduler.c:106), plus 1 recursion cycle and 5 cross-invocation revisit sites; no progress verdict (Quirin proves resource safety, not progress), 112 loops in Unknown and 9 in resource-Safe functions. Native observations under injected premises, each with a control, not upstream defect claims: the backlog collector does not yield under a persistent `CIO_ERROR` (E1) or after one transient `fstat` failure (E2), and a retry whose chunk cannot be brought up is rescheduled without spending an attempt (E3); the implementation card `W45-FB-ENGINE-PROGRESS-MEASURE` is registered, not cut | [W45-FB-ENGINE-PROGRESS](reports/w45-fb-engine-progress/README.md), [census](reports/w45-fb-engine-progress/census/census.json) |
| Chunk lifecycle results | **0 Safe / 96 Unknown** over **96** selected `flb_input_chunk.c` and `cio_chunk.c` functions; nine outstanding composition legs each; 2 carry a path-incomplete composition from their own pinned unit | [Chunk source profile](reports/w43-chunk-source-profile/README.md), `lifecycle-coverage.json`; [lifecycle certificate](reports/w42-chunk-lifecycle-cert/README.md) |
| Registered follow-ups | **511**; last moved by W46-ACCESS-PATH-CENSUS (registered two residuals with their sizes, amended eleven, retired none), 509 after W46-EXTERNAL-CALLEE-CONTRACTS (registered four residuals with their sizes, amended four, retired none), 505 after W45-SR5-SOUNDNESS (retired ten STABILITY-REEVAL-5 findings and `W45-REVIEW-PREFIX-COLLISION-UNPINNED` with their evidence, registered three residuals), 513 before it after DOCS-COMPACT-9 (registered two, retired `INTEGRATION-GATE-PATH-RULE-DRIFT`), 512 after STABILITY-REEVAL-5 registered forty. Which card registered or retired which name is in its report and wave record | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |

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

**Raw units remain unpromoted.** The unit Safe count is 5; each is a
function-level roll-up, not a promotion. `flb_input_chunk_create` is
function-level Unknown since W45-FB-RETRY-LIFETIME, and the production
`input_chunk_get` residual remains Unknown. Function-level safety and the chunk
**lifecycle** result are separate dimensions: no selected chunk function's
lifecycle post-state is established, and a checked field contract is not a
lifecycle result. Provisional Unsafe findings are not validated upstream
defects. Unknowns stay in the denominator. The baseline is
`docs/reports/w46-external-callee-contracts/integration/artifacts/`, the
combined-tree run of that integration (its predecessor
`docs/reports/w45-container-root-reach/integration/artifacts` is frozen); an
arbiter must reproduce its predecessor's verdict-bearing artifacts
byte-for-byte and attribute every delta. The chain and leaf identities are
preserved in the individual reports, [wave history](development/increments/README.md),
and [gateway rules](agents/gateway.md#constraint-7).

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

The latest merged gate is the
[W45-AUDIT-SUBCOMMAND-PLUGINS integration gate](reports/w45-audit-subcommand-plugins/integration/gate.json)
on merge `658717fb`: **5,995 core Rust passed, 0 failed, 11 ignored over 331
summaries (333 until `33558c67` corrected the count); 129 provider Rust passed, 0 failed, 1 ignored over 43 summaries;
207 Lean audits; 364 root Python tests; 15 package Python tests**. Core and
provider fmt and clippy, both reevaluation examples, the docs index, the
contract check, the isolated package mutation control and all five package
builds and verifications exit 0; Clang 21.1.8 was measured in that worktree.
Its arbiter reproduced the baseline (`verdict_bearing_identical: true`).

The five provider crates own **72** canonical contract modules; **42** shared or
application-level modules live in six `providers/quirin-provider-*` crates: five
per-library data crates (libc only in `quirin-provider-libc`) and
`quirin-provider-shared`, which also holds the providers' shared runtime and
bundles the five. `contract/corpus.json` indexes all **114** without duplicate
snapshots. Retained integration adapters embed the provider-owned data directly;
generic core crates retain no provider executable dependency;
package inventories bind supplied contract bytes to their executables, and
policy selection and independent checking remain explicit. Contract modules can
take a declaration's contract from another published module (`#include` /
`contract inherit NS::NAME`, [CONTRACT-INHERIT](reports/contract-inherit/README.md)),
wired into the provider crates through a generated, digest-pinned load path
([adoption](reports/contract-inherit-adoption/README.md),
[naming](reports/contract-inherit-qcore-provider-naming/README.md)). Every
function declared in more than one module is a
[checked view relation](reports/contract-view-projection-check/README.md).
Resource custody is kind-indexed: a release discharges only an obligation of its
own kind, and a file descriptor is tracked as an `int` with sentinel validity
([W43-DESCRIPTOR-RESOURCE-TRACKING](reports/w43-descriptor-resource-tracking/README.md),
Lean `QCore.KindedCustody`).

The three broader Cycle 18 follow-ups remain registered
(`CYCLE18-CALLBACK-TASK-HANDOFF`, `CYCLE18-NATIVE-PLATFORM-MATRIX`,
`CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`). Production callback composition,
task/cancellation/event-generation coverage and current foreign-host
observations remain outstanding. Callback, CT and full-conformance coverage are
unchanged.

The F02 Windows native run and the host-unconditional pin-agreement test now
pass with the same two emitted-C digests. Host/load-sensitive tests and their reporting
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
