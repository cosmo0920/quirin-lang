# Current status and evidence boundaries

Re-derived on 2026-09-21 from the sources below; corpus rows re-derived on
2026-09-25 from the W43-DESCRIPTOR-RESOURCE-TRACKING integration arbiter on the combined tree
(re-verified unchanged by the W43-INDEXED-RESOURCE-COLLECTIONS and W43-DESCRIPTOR-NATIVE-CHECK
integrations), with the diagnostic and indirect-access rows re-derived by the W44-INDIRECT-CHAIN-ACCESS
card arbiter; contract-corpus rows re-derived on 2026-09-25 at the integrations; native-descriptor
rows from W43-DESCRIPTOR-NATIVE-CHECK; the follow-up count at its integration. This is the concise status
authority routed by `AGENTS.md`; reports own the underlying evidence. Update
numbers from those sources, not from an older summary. History belongs in the
[increment records](development/increments/README.md).

## Coverage

| Measure | Recorded value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **50/50**; none unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT independent artifacts | **48** distinct source-fixture programs behind the 50 mapped cases (CT01/CT12/CT14 share one); **9** reviewed byte-identical fixture groups | Same manifest, `artifact_identity`, recomputed by `ct_corpus.rs`; [disclosure](reports/ct-shared-fixture-disclosure/README.md) |
| CT case-bound proof | **5/50**: CT01, CT03, CT07, CT37, CT50 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/50, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Cycle 19 units | **3 Safe / 43 provisional Unsafe / 135 Unknown / 4 data-only**, 185 total; `cmt_time.c` moved Unknown → Safe at W43-INDIRECT-ACCESS-GENERAL | [W43 indirect-access corpus](reports/w43-indirect-access-general/README.md) |
| Projected functions | **296 Safe / 85 provisional Unsafe / 4,453 Unknown**, 4,834 total; the 37 Safe added by the three W43 libc declaration cards are relative to 78 declared libc/POSIX premises, and the 12 added by W43-INDIRECT-ACCESS-GENERAL to the typed scalar-access rule and existing premises; 2 by W43-RESULT-ALIAS-RETURNED's inferred derived-result summaries; 13 by W43-INDEX-TRAVERSAL (11 its own, 2 cleared only in combination with W43-LIBC-RESTRICT-PROTOTYPES), with no bounds claim; 7 by W43-LIBC-PRINTF-FAMILY (5 its own, 2 cleared only in combination with W43-INDEX-TRAVERSAL) | [Combined baseline](reports/w43-libc-restrict-prototypes/integration/integration-confirmation.md) |
| Corpus diagnostics | leak **271**; unknown_alias **4,420**; double_release **0**; unsupported **45,721**; summary_conflict **1**; invalid_release / use_after_release **0 / 0** (unknown_alias 4,358 and unsupported 45,848 before W44-INDIRECT-CHAIN-ACCESS, which removed 74 member-chain refusals and exchanged 50 of them for 49 root refusals) | [W44 chain baseline](reports/w44-indirect-chain-access/artifacts/), `diagnostics.json.gz`; the leak move 272 → 271 is one attributed false-leak withdrawal |
| Uncontracted-call refusals | **9,505** leaves on **2,579** Unknown functions, **406** blocked only by uncontracted calls (9,547 / 2,591 / 407 before W44-CORPUS-CALLEE-SUMMARIES); by callee body: 3,652 same-unit bodies that do not validate, 2,974 in another pinned unit (105 with a `Safe` home body), 1,257 header `static` bodies that do not validate, 950 with no pinned body, 586 glibc `__bswap_*`, 86 external-linkage `inline` | [Callee census](reports/w44-corpus-callee-summaries/census/classes-after.json) |
| Indirect-access refusals | **17,232** leaves on **3,278** Unknown functions, sole blocker of **409** (17,665 / 3,315 / 378 before W43-INDIRECT-ACCESS-GENERAL; 17,300 / 3,291 / 380 after it, before W43-LIBC-RESTRICT-PROTOTYPES; 17,281 / 3,289 / 406 before W44-INDIRECT-CHAIN-ACCESS); the typed scalar-access rule removed 1,083 leaves across five access families, and the refused shapes are sized in its [residuals](reports/w43-indirect-access-general/residuals.txt); the member-chain family is re-sized by shape in the [chain residuals](reports/w44-indirect-chain-access/residuals.txt) | [Current blocker census](reports/w44-indirect-chain-access/blocker-census.txt) |
| Indexed resource collections | **27** element operations lowered in **19** corpus functions, **0** on a resolved array (every one keeps its refusal byte for byte); **0** leaves, functions or units moved on main `d2dd61c1`; collections are heap-kind only; at the card's base census the array allocation's `sizeof(*v)` operand was refused in **224** leaves on **211** Unknown functions (4 sole-blocked) | [Collections report](reports/w43-indexed-resource-collections/README.md), [element reach](reports/w43-indexed-resource-collections/census/element-reach.txt), [census](reports/w43-indexed-resource-collections/census/census.txt) |
| Return-expression refusals | **496** leaves on **419** Unknown functions, sole blocker of **148** (1,440 / 1,109 / 465 before W43-RETURN-EXPRESSION; two returned derived results joined at W43-LIBC-RESULT-ALIAS; 146 sole at that card, 149 after W43-INDIRECT-ACCESS-GENERAL cleared other families, 148 after `rd_regex_exec` left at W43-LIBC-RESTRICT-PROTOTYPES); 941 whole-call leaves carry the call classifier's own refusal | [W43 attribution](reports/w43-return-expression/attribution-summary.json), [current blocker census](reports/w43-indirect-access-general/blocker-census.txt) |
| Unit reasons | **134 unsupported-syntax / 1 incomplete-obligation-coverage / 50 without a reason code** | [W43 indirect-access corpus row](reviews/r0_3/acceptance_matrix.json) (`corpus.cycle19_raw_units`); `cmt_time.c` lost its reason at W43-INDIRECT-ACCESS-GENERAL, and the earlier fields are in the [combined attribution](reports/w42-priority-integration/attribution.json) |
| Raw source coverage | **4,921 definitions / 4,834 projected / 87 pinned-build exclusions**, across **24 full-path units** | [Verified census](reports/w42-unprojected-definitions/README.md); combined inventory unchanged |
| Monkey callback accounting | **3/3 accounted, 0/3 Cycle 18 composed, 3 explicit Unknown**; 8 source paths; per-callback obligations **51 relevant cells, 17 discharged / 34 open** (was 5 / 46) | [Callback report](reports/w42-monkey-callback-coverage/README.md); ordinary audit checks 0/8 native paths, optional focused replay 8/8; [per-callback composition](reports/w43-monkey-callback-composition/README.md) discharges initialization reachability (3 callbacks) and natural activation (2) from the pinned compile task |
| Observed Monkey timed wait | **1 instrumented Linux epoll success run**, 11 identity records and 11 model operations; conditional replay, no coverage promotion | [Observed integration](reports/cycle18-observed-integration/README.md) |
| Cycle 18 priority-one checks | **8 failure cases** (3 fault-model Unsafe / 5 Unknown); **4/4 payload paths** under explicit authority; **15-operation timeout teardown** | [Bounded integration](reports/cycle18-priority1-integration/README.md) |
| Cycle 18 bounded aggregate | **3 joined families / 8 source paths / 7 unresolved obligations**, aggregate Unknown | [Broader integration](reports/cycle18-broader-integration/aggregate.json) |
| Callback task handoff | **12 source-derived operations**, one Linux pthread fixture; production acceptance pending | [Integrated lifetime evidence](reports/cycle18-broader-integration/task-handoff.json) |
| Current native platform matrix | **4 Linux Observed / 3 foreign-host Unknown**; generic Libevent probe only | [Bounded platform evidence](reports/cycle18-broader-integration/platform.json) |
| Contract corpus | **111** modules; **152** resource-v1 declarations; **47** manifest-selected, 4 of them by `crate:module` reference; **13** declarations inherit a libc contract | [Provider ownership](../providers/README.md#contract-ownership) (72 in five provider crates, 39 in six [`quirin-provider-*` shared crates](reports/quirin-provider-shared/README.md#crate-map)), canonical index `contract/corpus.json` (schema 2, 11 roots), [selection](../cycle19-resource-contracts.json) (schema 2), [load path](../providers/contract-load-path.json) and pinned tests; [adoption](reports/contract-inherit-adoption/README.md) |
| Multi-module declarations | **10** functions in **12** module pairs, every pair a registered checked relation: 5 projections, 1 renamed equivalence, 5 source-shape agreements, 1 lifecycle agreement; **0** textual duplicates; 1 view not implied outside a coverage premise (`CONTRACT-VIEW-CHUNK-CLOSE-BACKEND-COVERAGE`) | [View projection check](reports/contract-view-projection-check/README.md), `qcore_contract::contract_views::REGISTERED_RELATIONS`, guard `compiler/tests/contract_view_census.rs` |
| Native descriptor observation | **81 pinned Linux runs** (80 accepted, 36 with injected failures), 7,416 records of the pinned Fluent Bit build, Monkey `mk_socket.c`, ctraces `ctr_random_get` and a libc probe; 8 of 10 lifecycle properties hold on every accepted run; release-at-most-once fails in 13 runs (two source-confirmed upstream double closes) and operand validity in 37 (`close(-1)`); the landed libc descriptor, stream and directory contracts bound to **3,149** executions of 18 declared callees with **0 contradictions**; `ctr_random_get`'s descriptor-0 leak reproduced by a direct driver, not through the binary; **observation, not proof or mapping** | [W43 descriptor native check](reports/w43-descriptor-native-check/README.md), `aggregate.json`, `binding.json`, pinned `records/` |
| Chunk lifecycle results | **0 Safe / 96 Unknown** over **96** selected `flb_input_chunk.c` and `cio_chunk.c` functions; nine outstanding composition legs each; 2 carry a path-incomplete composition from their own pinned unit | [Chunk source profile](reports/w43-chunk-source-profile/README.md), `lifecycle-coverage.json`; [lifecycle certificate](reports/w42-chunk-lifecycle-cert/README.md) |
| Registered follow-ups | **358** | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |

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

**Raw units remain unpromoted.** The unit Safe count rose to 3 at
W43-INDIRECT-ACCESS-GENERAL (`cmt_time.c`, whose one projected function became
Safe); that is a function-level roll-up, not a promotion. `flb_input_chunk_create` is function-level Safe;
the production `input_chunk_get` residual remains Unknown. Function-level safety
and the chunk **lifecycle** result are separate dimensions: no selected chunk
function's lifecycle post-state is established, `flb_input_chunk_create`
included, and a checked field contract is not a lifecycle result. Provisional Unsafe
findings are not validated upstream defects. Unknowns stay in the denominator.
The baseline is `docs/reports/w44-corpus-callee-summaries/integration/artifacts/`, the
combined-tree run of that integration (its predecessor `docs/reports/w44-indirect-chain-access/integration/artifacts` is frozen); an arbiter must
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

The five provider crates own **72** canonical contract modules; **39** shared or
application-level modules live in six `providers/quirin-provider-*` crates: five per-library
data crates (libc only in `quirin-provider-libc`) and `quirin-provider-shared`, which also
holds the providers' shared runtime and bundles the five. `contract/corpus.json`
indexes all **111** without duplicate snapshots. Retained integration adapters
embed the provider-owned data directly; generic core crates retain no provider
executable dependency. Package inventories bind supplied contract bytes to their
executables; policy selection and independent checking remain explicit. This
changes source locations, with no predicate or verdict promotion. The bundle
pilot adds independently pinned data-only selection for the CMetrics policy,
retains version-1 paths and existing adapters, and models admission separately
from policy truth. Four scoped follow-ons remain in its report. Contract
modules can take a declaration's contract from another published module
(`#include` / `contract inherit NS::NAME`, [CONTRACT-INHERIT](reports/contract-inherit/README.md)).
[CONTRACT-INHERIT-ADOPTION](reports/contract-inherit-adoption/README.md) wires it
into the six `quirin-provider-*` crates through a generated, digest-pinned load
path read as data. Five Fluent Bit wrappers (`flb_malloc`, `flb_calloc`,
`flb_free`, `flb_strdup`, `flb_strndup`) inherit libc contracts, with no
verdict-bearing byte moved.
[CONTRACT-INHERIT-QCORE-PROVIDER-NAMING](reports/contract-inherit-qcore-provider-naming/README.md)
admits the second prefix `qcore-provider-` (the user's option B), puts the five
executable crates on the load path, and converts the eight librdkafka and Monkey
allocator wrappers the same way, again with no verdict-bearing byte moved;
`qcore.librdkafka` and `qcore.libevent` are the first packages with include pins. Every function declared in more than one module is a
[checked view relation](reports/contract-view-projection-check/README.md); the
one textual duplicate (`source_identity`) was removed, with no verdict-bearing
byte moved. Resource custody is now kind-indexed: a release discharges only an
obligation of its own kind, and a file descriptor is tracked as an `int` with
sentinel validity, with a single custody-carrying-value predicate guarding
every scalar route including the typed indirect one
([W43-DESCRIPTOR-RESOURCE-TRACKING](reports/w43-descriptor-resource-tracking/README.md),
Lean `QCore.KindedCustody`).

All three broader Cycle 18 follow-ups remain open; **358** follow-ups are
registered — `W42-CHUNK-LIFECYCLE-CERT` retired against its own report and two
narrower lifecycle residuals took its place; W43-RETURN-EXPRESSION registered six
refused return-expression families and the `report_leaks` premise finding, and
retired `RETURN-CALL-INCOMPLETE-LINE-455` against its census;
`MONKEY-CALLBACK-CYCLE18-COMPOSITION` was narrowed and two Cycle 18 residuals
registered; QUIRIN-PROVIDER-SHARED registered
`PROVIDER-CONFORMANCE-CORE-FIXTURE-PATHS`; the Windows F02 run and active
cross-host pin check retired three F02 follow-ups; W43-LIBC-CONTRACTS registered ten libc
residuals and retired `LOADER-ABI-RESTRICT-QUALIFIER` against its measurement; CONTRACT-INHERIT registered its
real-crate adoption, extension clauses and non-v1 consumers; W43-LIBC-RESULT-ALIAS
retired `LIBC-MEM-RESULT-ALIAS`, `W43-LIBC-RESULT-ALIAS-NAMES` and
`W43-LIBC-POINTER-QUALIFIER-READER` and registered six derived-result and reader residuals;
CONTRACT-INHERIT-ADOPTION retired the adoption and registered the `qcore-provider-*`
naming decision, alias-result parents and the Fluent Bit string-wrapper assumptions;
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING retired the naming decision and the alias-result
parents against its tests and arbiter and registered `CONTRACT-INHERIT-PERMUTED-PARAMETER-NAMES`;
CONTRACT-VIEW-PROJECTION-CHECK registered six view-relation residuals (backend
coverage, unenforced premises, the kept renamed equivalent, the unproved
small-model lemma, declared bridges and the timeout-policy native rerun) and
retired none; W43-INDIRECT-ACCESS-GENERAL retired `INDIRECT-CENSUS-FEATURE-UNBUILT`,
`BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT` and `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`
and registered twelve refused-shape, premise and census residuals;
W43-LIBC-RESTRICT-PROTOTYPES retired `W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED` and
registered `getpeername`'s transparent-union withdrawal and `dlsym`'s handle lifetime;
W43-RESULT-ALIAS-RETURNED retired `W43-RESULT-ALIAS-RETURNED` and
`W43-RESULT-ALIAS-EXACT-RELEASE` (refusal kept on a recorded ground) and registered
`W43-DERIVED-SUMMARY-CONST-DROPPED-FROM-CHAIN`;
W43-INDEX-TRAVERSAL retired `W43-INDIRECT-POINTER-INCREMENT` and registered twelve
refused-shape and premise residuals; W43-LIBC-PRINTF-FAMILY retired `LIBC-PRINTF-FAMILY`
and registered the `va_list` members, the declared-tail exchange and its nested inner callees; W43-DESCRIPTOR-RESOURCE-TRACKING retired `W43-LIBC-DESCRIPTOR-LIFECYCLE`, discharged
`CT-CTWIN-O`'s descriptor arm and amended `EXTERN-CALL-F`, and registered descriptor summaries,
refused descriptor callees, the cast-return false leak and its unexercised accusing arms;
W43-INDEXED-RESOURCE-COLLECTIONS retired nothing and registered seven collection residuals,
led by the `sizeof(*v)` allocation; W43-DESCRIPTOR-NATIVE-CHECK registered nine native-descriptor residuals and findings (coverage
gaps, two double closes, a storage-scan double free, a Monkey bind-failure leak, an unreleased
proxy handle, the observer's ordering boundary, `ctr_random_get`'s descriptor-0 witness and
the data-pipe zero guard) and retired its own Phase B binding follow-up; W44-INDIRECT-CHAIN-ACCESS
retired `W43-INDIRECT-CHAIN-ACCESS` and `W43-INDEX-TRAVERSAL-CONDITION-CHAIN` against their
discharge conditions and registered nine measured chain residuals, led by nested aggregates and the
3,564 leaves W43's chain classes counted that carry no member chain. Production callback composition, task/cancellation/event-generation
coverage and current foreign-host observations remain outstanding. Callback,
CT and full-conformance coverage are unchanged.

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
