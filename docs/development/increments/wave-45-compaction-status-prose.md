# Wave 45 compaction checkpoint: status page, part 2 (`DOCS-COMPACT-9`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Status part 1](wave-45-compaction-status.md) · [Rule-page checkpoints](wave-45-compaction-checkpoints.md) · [Wave 45 records](wave-45-continuation.md)

> **Preserved checkpoint text. These blocks are not a current authority.**
> They are the verbatim text `DOCS-COMPACT-9` compacted out of `docs/STATUS.md` at
> base `8c1f15eb` (one status block at the merged `main`), kept so that nothing
> the compaction removed is lost. Each block names its revision and lines;
> `git show <revision>:<file>` reproduces it byte for byte. Where a block and the current page disagree, **the current page wins**;
> where a block states something the card measured to be false, the correction
> is in [the card report](../../reports/docs-compact-9/README.md) and is
> deliberately not applied here. Relative links inside a block keep their
> original base, the page they were written in.

<a id="status-prose"></a>

## `docs/STATUS.md`: later coverage rows and prose sections

Current page: [../../STATUS.md](../../STATUS.md).

### 1. Coverage row: Contract corpus

`docs/STATUS.md` lines 42–42 at `8c1f15eb`, 979 bytes. Kept: every current figure and source. Moved: the declaration-count chain.

~~~~~text
| Contract corpus | **114** modules; **174** resource-v1 declarations (162 before W45-FB-BACKLOG-CUSTODY's twelve checked specifications, `fluent_bit/storage_backlog.qcore`, pinned at `046accf6` and `66910c10`; 155 before W45-FB-RETRY-LIFETIME's two modules and `mk_list_del`; 152 before W44-LIBC-RESIDUALS' `vsnprintf`, `vprintf`, `strerror`); **50** manifest-selected, 4 of them by `crate:module` reference, and exactly the eleven crates' checked resource views (W44-PROVIDER-RESOURCE-VIEW); **13** declarations inherit a libc contract | [Provider ownership](../providers/README.md#contract-ownership) (72 in five provider crates, 42 in six [`quirin-provider-*` shared crates](reports/quirin-provider-shared/README.md#crate-map)), canonical index `contract/corpus.json` (schema 2, 11 roots), [selection](../cycle19-resource-contracts.json) (schema 2), [load path](../providers/contract-load-path.json) and pinned tests; [adoption](reports/contract-inherit-adoption/README.md) |
~~~~~

### 2. Coverage row: Native descriptor observation

`docs/STATUS.md` lines 44–44 at `8c1f15eb`, 797 bytes. Corrected, not compacted (`NATIVE-OBSERVATION-UPSTREAM-WORDING`): 'two source-confirmed upstream double closes' now reads as local observations, provisional candidates, not upstream defect claims.

~~~~~text
| Native descriptor observation | **81 pinned Linux runs** (80 accepted, 36 with injected failures), 7,416 records of the pinned Fluent Bit build, Monkey `mk_socket.c`, ctraces `ctr_random_get` and a libc probe; 8 of 10 lifecycle properties hold on every accepted run; release-at-most-once fails in 13 runs (two source-confirmed upstream double closes) and operand validity in 37 (`close(-1)`); the landed libc descriptor, stream and directory contracts bound to **3,149** executions of 18 declared callees with **0 contradictions**; `ctr_random_get`'s descriptor-0 leak reproduced by a direct driver, not through the binary; **observation, not proof or mapping** | [W43 descriptor native check](reports/w43-descriptor-native-check/README.md), `aggregate.json`, `binding.json`, pinned `records/` |
~~~~~

### 3. Coverage row: Fluent Bit system slice

`docs/STATUS.md` lines 45–45 at `8c1f15eb`, 2,085 bytes. Kept: the current figures, the refuted OR-1/OR-7 decisions, the held callers and the OR-2/OR-6 and OB-1 … OB-10 status. Moved: the per-card slice chain.

~~~~~text
| Fluent Bit system slice (current build) | **8 files / 15 units: 0 Safe / 2 provisional Unsafe / 13 Unknown**; **237 functions: 28 Safe / 2 provisional Unsafe / 204 Unknown / 3 not projected**, at `046accf6` (re-pinned by the user) through `--selection contract/profiles/fluent-bit/system-selection.json`; W45-REFUTED-CONTRACT-WITHDRAWAL moved no status: its refutation table is OR-7 (`flb_output_task_flush`) and OR-1 (`flb_output_task_singleplex_enqueue`), both refuted by their own bodies and still provisional Unsafe on their own rows, and every caller call site (10 rows in `flb_engine_dispatch_retry`, `tasks_start`, `flb_output_task_queue_flush_one`, all already Unknown) is now held Unknown (`contract-refuted-by-body`), so no caller applies either declaration ([report](reports/w45-refuted-contract-withdrawal/README.md)); W45-CONTAINER-ROOT-REACH moved no status (18 leaves removed, 3 added: the foreign unlinks of six functions admitted, OB-3/OB-10 lose that blocker and stay Unknown, the OR table identical; [report](reports/w45-container-root-reach/README.md) §6–7); after W45-FB-BACKLOG-CUSTODY: every `sb.c` iteration and element refusal withdrawn, four list traversals Unknown → Safe, backlog obligations OB-1 … OB-10 all Unknown with two custody legs held by pinned checked specifications ([report](reports/w45-fb-backlog-custody/README.md) §4–5); 24 / 2 / 208 / 3 before it, after W45-FB-RETRY-LIFETIME: both publication leaks withdrawn, the obligations OR-1 and OR-7 checked as Fluent Bit provider contracts and each decided **refuted** (provisional Unsafe candidates, never upstream defects; OR-1 under the synchronous-proxy premise), OR-6 not stated, OR-2 not reached; 0 / 3 / 12 units and 25 / 2 / 207 / 3 functions before it at `046accf6` (source change from `864c3d97`'s 0 / 3 / 12 and 25 / 2 / 206 / 3: one new function); a separate audit scope, not counted in the Cycle 19 rows above, and no promotion | [W45-FB-RETRY-LIFETIME](reports/w45-fb-retry-lifetime/README.md) §4–5, [W45-FB-SYSTEM-SELECTION](reports/w45-fb-system-selection/README.md) |
~~~~~

### 4. Coverage row: Registered follow-ups

`docs/STATUS.md` lines 48–48 at `8c1f15eb`, 2,935 bytes. Kept: the count and the last moving card. Moved: every card's registration list, which each report and wave record carries. The version STABILITY-REEVAL-5 left on `main` is the next block.

~~~~~text
| Registered follow-ups | **472** (W45-AUDIT-SUBCOMMAND-PLUGINS added four: `W45-SUBCOMMAND-CERTIFICATE-ROUTE`, `W45-SUBCOMMAND-CONFINEMENT`, `W45-SUBCOMMAND-NON-UNIX`, `W45-JSON-TEXT-DUPLICATE`, and its integration added `W45-SUBCOMMAND-HOST-REPORT-TRUST`; 467 before it; W45-FB-ENGINE-PROGRESS added eight: `W45-FB-ENGINE-PROGRESS-MEASURE`, `W45-FB-BACKLOG-TRANSIENT-STICKY-ERROR`, `W45-FB-RETRY-UNSPENT-RESCHEDULE`, `W45-FB-RING-BUFFER-DRAIN-UNBOUNDED`, `W45-FB-SCHED-RANDOM-RANGE`, `W45-FB-ENGINE-REVISITS-UNEXERCISED`, `W45-FB-CIO-MAP-FAILED-STATE`, `W45-FB-BACKLOG-EXIT-WRAPPERS`; 459 before it; W45-TEST-HYGIENE added `W45-PRODUCTION-WALKS-READ-WORKING-TREE`, the production and validation readers its tracked-input rule leaves on the working tree; 458 before it; W45-REFUTED-CONTRACT-WITHDRAWAL retired `W45-FB-SINGLEPLEX-CUSTODY-DECLARED`, re-scoped `W45-FB-CROSS-UNIT-USER-PREMISES` and added three: `W45-CONTAINER-CLAUSE-BODY-CHECK`, `W45-UNCHECKED-DECLARATIONS-TRUSTED`, `W45-REFUTED-HOLD-UNIT-REASON`; 456 before it; W45-CONTAINER-ROOT-REACH retired `W45-FB-BACKLOG-FOREIGN-UNLINK` and added `W45-FORMAL-ENTRY-UNLINK`, `W45-FOREIGN-UNLINK-SUMMARY` and `W45-FOREIGN-UNLINK-RESIDUAL-SHAPES`; 454 before it; W45-USER-CONTRACT-PLUGINS added seven: `W45-CHECKPOINT-COMPOSE-SEMIGROUP`, `W45-PLUGIN-CERTIFICATE-CHECKER`, `W45-PLUGIN-COUNTEREXAMPLE-CHECKER`, `W45-PLUGIN-ROUTE-COVERAGE`, `W45-PLUGIN-LINK-CFG-PREMISE`, `W45-PLUGIN-OBLIGATION-PROPERTY`, `W45-PLUGIN-EXECUTION-PROFILE`; 442 before it; W45-FB-BACKLOG-CUSTODY added eight: `W45-FB-BACKLOG-REFERENCE-VOCABULARY`, `-MAP-ARMS`, `-REACHABLE-RELEASE`, `-CIO-RETURN-CODES`, `-PUBLICATION-BLOCKED`, `-FOREIGN-UNLINK`, `W45-ILI-PREMISES`, `W45-MK-HTTP-ERROR-FD-LEAK`; 434 before it; W45-CONTRACT-REVISION-MODE added `W45-REVISION-PIN-ROUTES`, `W45-REVISION-LOCK-NOT-PACKAGED`, `W45-REVISION-SCANNER-LEXICAL`, `W45-REVISION-HOLD-REPORTING`, `W45-REVIEW-PREFIX-COLLISION-UNPINNED` and `W45-REVISION-COMPARE-MATCHED-IDENTITY`; W45-PARENTHESISED-CALLEE added `W45-FOR-CLAUSE-ORIGINAL-SPELLING`, `W45-CALLEE-EXPRESSION-FORMS` and `W45-CALLEE-CAST-LEXICAL-PREMISE`; W45-RETURN-EXPRESSION-FAMILIES retired `W43-RETURN-CALL-UNDER-CAST` and `W43-RETURN-TYPE-OPERATOR` and added `W45-RETURN-POINTER-RESULT-READS` and `W45-RETURN-BOUND-BASE-POSITIONS`; W44-PROVIDER-RESOURCE-VIEW retired `W44-PROVIDER-RESOURCE-VIEW` and added `W44-CONTRACT-MANIFEST-ORDER-DIGEST` and `W44-RESOURCE-VIEW-REFUSAL-UNCLASSIFIED`; W44-QUIRIN-AUDIT-PROVIDERS added `W44-PROVIDER-RESOURCE-VIEW`, W44-AUDIT-PARALLEL-UNITS added `W44-AUDIT-CRITICAL-UNIT`, W44-C11-GENERIC-SELECTION `W44-BSEARCH-UNDECLARED`, W44-TEST-SUITE-WALL-TIME three, W44-INDIRECT-CALL-ARGUMENT-ACCESS eight, retiring `W43-INDIRECT-CALL-ARGUMENT-ACCESS`, W44-COLLECTION-ALLOCATION three, retiring `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`) | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |
~~~~~

### 5. Coverage row: Registered follow-ups, as STABILITY-REEVAL-5 left it

`docs/STATUS.md` lines 48–48 at `200fdcbe`, 3,308 bytes. The row on `main` `200fdcbe`, re-derived at the merge rather than union-merged.

~~~~~text
| Registered follow-ups | **512** (STABILITY-REEVAL-5 added forty, retired none and amended `GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT` with its recurrence; among them `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`, a measured `Safe` over a double member release, and `HELD-PREFIX-SOURCE-FORGES-PENDING`; the full list is in its [report](reports/stability-reeval-5/README.md) and `scripts/register.py`; 472 before it; W45-AUDIT-SUBCOMMAND-PLUGINS added four: `W45-SUBCOMMAND-CERTIFICATE-ROUTE`, `W45-SUBCOMMAND-CONFINEMENT`, `W45-SUBCOMMAND-NON-UNIX`, `W45-JSON-TEXT-DUPLICATE`, and its integration added `W45-SUBCOMMAND-HOST-REPORT-TRUST`; 467 before it; W45-FB-ENGINE-PROGRESS added eight: `W45-FB-ENGINE-PROGRESS-MEASURE`, `W45-FB-BACKLOG-TRANSIENT-STICKY-ERROR`, `W45-FB-RETRY-UNSPENT-RESCHEDULE`, `W45-FB-RING-BUFFER-DRAIN-UNBOUNDED`, `W45-FB-SCHED-RANDOM-RANGE`, `W45-FB-ENGINE-REVISITS-UNEXERCISED`, `W45-FB-CIO-MAP-FAILED-STATE`, `W45-FB-BACKLOG-EXIT-WRAPPERS`; 459 before it; W45-TEST-HYGIENE added `W45-PRODUCTION-WALKS-READ-WORKING-TREE`, the production and validation readers its tracked-input rule leaves on the working tree; 458 before it; W45-REFUTED-CONTRACT-WITHDRAWAL retired `W45-FB-SINGLEPLEX-CUSTODY-DECLARED`, re-scoped `W45-FB-CROSS-UNIT-USER-PREMISES` and added three: `W45-CONTAINER-CLAUSE-BODY-CHECK`, `W45-UNCHECKED-DECLARATIONS-TRUSTED`, `W45-REFUTED-HOLD-UNIT-REASON`; 456 before it; W45-CONTAINER-ROOT-REACH retired `W45-FB-BACKLOG-FOREIGN-UNLINK` and added `W45-FORMAL-ENTRY-UNLINK`, `W45-FOREIGN-UNLINK-SUMMARY` and `W45-FOREIGN-UNLINK-RESIDUAL-SHAPES`; 454 before it; W45-USER-CONTRACT-PLUGINS added seven: `W45-CHECKPOINT-COMPOSE-SEMIGROUP`, `W45-PLUGIN-CERTIFICATE-CHECKER`, `W45-PLUGIN-COUNTEREXAMPLE-CHECKER`, `W45-PLUGIN-ROUTE-COVERAGE`, `W45-PLUGIN-LINK-CFG-PREMISE`, `W45-PLUGIN-OBLIGATION-PROPERTY`, `W45-PLUGIN-EXECUTION-PROFILE`; 442 before it; W45-FB-BACKLOG-CUSTODY added eight: `W45-FB-BACKLOG-REFERENCE-VOCABULARY`, `-MAP-ARMS`, `-REACHABLE-RELEASE`, `-CIO-RETURN-CODES`, `-PUBLICATION-BLOCKED`, `-FOREIGN-UNLINK`, `W45-ILI-PREMISES`, `W45-MK-HTTP-ERROR-FD-LEAK`; 434 before it; W45-CONTRACT-REVISION-MODE added `W45-REVISION-PIN-ROUTES`, `W45-REVISION-LOCK-NOT-PACKAGED`, `W45-REVISION-SCANNER-LEXICAL`, `W45-REVISION-HOLD-REPORTING`, `W45-REVIEW-PREFIX-COLLISION-UNPINNED` and `W45-REVISION-COMPARE-MATCHED-IDENTITY`; W45-PARENTHESISED-CALLEE added `W45-FOR-CLAUSE-ORIGINAL-SPELLING`, `W45-CALLEE-EXPRESSION-FORMS` and `W45-CALLEE-CAST-LEXICAL-PREMISE`; W45-RETURN-EXPRESSION-FAMILIES retired `W43-RETURN-CALL-UNDER-CAST` and `W43-RETURN-TYPE-OPERATOR` and added `W45-RETURN-POINTER-RESULT-READS` and `W45-RETURN-BOUND-BASE-POSITIONS`; W44-PROVIDER-RESOURCE-VIEW retired `W44-PROVIDER-RESOURCE-VIEW` and added `W44-CONTRACT-MANIFEST-ORDER-DIGEST` and `W44-RESOURCE-VIEW-REFUSAL-UNCLASSIFIED`; W44-QUIRIN-AUDIT-PROVIDERS added `W44-PROVIDER-RESOURCE-VIEW`, W44-AUDIT-PARALLEL-UNITS added `W44-AUDIT-CRITICAL-UNIT`, W44-C11-GENERIC-SELECTION `W44-BSEARCH-UNDECLARED`, W44-TEST-SUITE-WALL-TIME three, W44-INDIRECT-CALL-ARGUMENT-ACCESS eight, retiring `W43-INDIRECT-CALL-ARGUMENT-ACCESS`, W44-COLLECTION-ALLOCATION three, retiring `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`) | [Matrix](reviews/r0_3/acceptance_matrix.json), carried entries and derived dimension agree |
~~~~~

### 6. The raw-units paragraph

`docs/STATUS.md` lines 65–78 at `8c1f15eb`, 1,143 bytes. Corrected: the unit Safe count is 5, not 3, and `flb_input_chunk_create` has been Unknown since W45-FB-RETRY-LIFETIME.

~~~~~text
**Raw units remain unpromoted.** The unit Safe count rose to 3 at
W43-INDIRECT-ACCESS-GENERAL (`cmt_time.c`, whose one projected function became
Safe); that is a function-level roll-up, not a promotion. `flb_input_chunk_create` is function-level Safe;
the production `input_chunk_get` residual remains Unknown. Function-level safety
and the chunk **lifecycle** result are separate dimensions: no selected chunk
function's lifecycle post-state is established, `flb_input_chunk_create`
included, and a checked field contract is not a lifecycle result. Provisional Unsafe
findings are not validated upstream defects. Unknowns stay in the denominator.
The baseline is `docs/reports/w45-container-root-reach/integration/artifacts/`, the
combined-tree run of that integration (its predecessor `docs/reports/w45-fb-backlog-custody/integration/artifacts` is frozen); an arbiter must
reproduce its predecessor's verdict-bearing artifacts byte-for-byte and
attribute every delta. The chain and leaf identities are preserved in the
individual reports, [wave history](development/increments/README.md), and
[gateway rules](agents/gateway.md#constraint-7).
~~~~~

### 7. Latest validated gate, and the provider-contract narrative

`docs/STATUS.md` lines 97–139 at `8c1f15eb`, 3,328 bytes. Corrected: the gate named was the task-context bundle's (5,043 Rust / 165 Lean / 336 Python); the latest merged gate is W45-AUDIT-SUBCOMMAND-PLUGINS' integration gate. The provider paragraph said 39 shared modules and 111 indexed; the tree has 42 and 114. The contract-inheritance chronicle moved here.

~~~~~text
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
~~~~~

### 8. The follow-up chronicle

`docs/STATUS.md` lines 141–189 at `8c1f15eb`, 4,172 bytes. Corrected: it said 391 follow-ups; the matrix carries 472. Kept: the three open Cycle 18 follow-ups, now by name, and the outstanding coverage. Moved: the card-by-card registration chain from W43 to W44.

~~~~~text
All three broader Cycle 18 follow-ups remain open; **391** follow-ups are
registered — W44-LIBC-RESIDUALS retired `W43-LIBC-HEADER-INLINE-BODIES`,
`W44-CALLEE-SCOPE-RESERVED-BODIES`, `LIBC-PRINTF-VA-LIST`,
`LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES` and registered
three measured residuals; `W42-CHUNK-LIFECYCLE-CERT` retired against its own report and two
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
3,564 leaves W43's chain classes counted that carry no member chain;
W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE retired nothing (its own name is registered only on the
call-argument branch) and registered the refused sub-object stores, the aggregate premise-3
member-name reading and the `==`-first statements. Production callback composition, task/cancellation/event-generation
coverage and current foreign-host observations remain outstanding. Callback,
CT and full-conformance coverage are unchanged.
~~~~~
