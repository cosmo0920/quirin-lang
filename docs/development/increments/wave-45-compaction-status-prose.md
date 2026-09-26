# Wave 45 compaction checkpoint: status page prose (`DOCS-COMPACT-9`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Status table checkpoint](wave-45-compaction-status.md) · [Rule-page checkpoints](wave-45-compaction-checkpoints.md) · [Wave 45 records](wave-45-continuation.md)

> **Preserved checkpoint text. These blocks are not a current authority.**
> They are the verbatim text `DOCS-COMPACT-9` compacted out of `docs/STATUS.md` at
> base `8c1f15eb`, kept so that nothing the compaction removed is lost. Each
> block names its base lines; `git show 8c1f15eb:<file>` reproduces it byte for
> byte. Where a block and the current page disagree, **the current page wins**;
> where a block states something the card measured to be false, the correction
> is in [the card report](../../reports/docs-compact-9/README.md) and is
> deliberately not applied here. Relative links inside a block keep their
> original base, the page they were written in.

<a id="status-prose"></a>

## `docs/STATUS.md`: prose sections

Current page: [../../STATUS.md](../../STATUS.md).

### 1. The raw-units paragraph

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

### 2. Latest validated gate, and the provider-contract narrative

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

### 3. The follow-up chronicle

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
