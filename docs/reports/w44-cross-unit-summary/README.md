# W44-CALLEE-CROSS-UNIT-SUMMARY

Card base `fa04aab0fd43b242d2a996da2c6c507f1eb2a113`; prediction
[`PREDICTION.md`](PREDICTION.md) committed first at `d92349a56`.

## Route boundary found before implementation

The W4 inference (`qcore-c-gateway::legacy_summary`) derives validated
summaries inside one preprocessed translation unit. Its `ResourceSummaryRecord`
has a function, trust label and contract, but no home unit or source identity.
`LegacySafetyCertificate` re-derives that same unit only. The audit's binding
adds a compile-task and loaded-environment identity, again for one unit.

Transporting a record by name and source digest would be unsound. A C external
call can resolve to an interposed definition; a C99 external `inline` text can
be bypassed; and multiple configurations can define the same name. The
current compile database is a set of units, not a linked-program symbol table.
The run must therefore take **an explicit conditional premise** identifying
the one external definition that each call resolves to. That premise must be
retained in the verdict's assumptions and in the certificate's expected
context. Absent, ambiguous, static-only and external-inline-only identities
must remain uncontracted.

The second boundary is certificate containment: candidate home functions can
be Safe while their translation unit is Unknown, so a whole-unit Safe
certificate cannot authenticate them. A route needs per-function home evidence
holding the preprocessed source, request and loaded environment, and must
re-run W4 over that home unit. The caller certificate must then independently
re-derive the home summary before it re-derives the caller under its imported
copy. `ResourceContracts.functions` cannot carry that copy: source-assumption
replay checks it exactly against the `.qv` declarations. Imports need their
own channel and identity.

The W48 BEFORE/AFTER calibration is 6 call-only corpus functions at the route
ceiling and 10 with held position calls; system and slice 0. It is a dated
upper bound, not this card's current-base measurement.

## Implemented conditional route

The opt-in `QCORE_CROSS_UNIT_LINK_MAP` selects a versioned JSON link claim.
Its exact bytes are SHA-256 pinned. With no map, the original analysis path
remains in force. The audit first settles its refutation table, then reads
every selected configuration's main-source definitions. It exports a named
home only if its W4 row and derived summary are Safe, no declaration exists,
no second selected configuration defines the same name, and no file-scope
declaration gives it static or inline semantics. A failed or incomplete
second phase, or a changed refutation table, leaves the first-phase results
in force. Checkpoint composition refuses this opt-in mode.

Imports live in a separate inferred channel, with the map digest in the
environment. The caller environment also carries independently expected
home source/request and environment digests; its audit trust-context hash
therefore binds the home identity separately from the certificate's retained
home witness. Caller-local definitions take precedence. Before issuing a Safe
certificate, the gateway reconstructs the home's raw, line-marked and
preprocessed source scope and its request digest, checks the loaded home
environment digest, re-runs W4 over the home, and compares the exact derived
contract. Certificate replay does the same before re-deriving the caller.
The published sidecar and each pinned unit report retain the named link
premise and map digest; configured current reports retain them in the
assumptions sidecar. Lean `QCore.CrossUnitSummary` models the conditional
resolution, uniqueness, declaration precedence and fail-closed identity
checks. The [corpus map](link-map.corpus.json) names five Safe homes serving
three call-only Unknown callers at the exact base; the
[refusal control](link-map-refusal.json) names an absent home.

The map is a **premise**, not evidence that a linker chose those definitions.
It must describe the actual linked program, compatible call/definition types
and calling conventions, and interposition environment for any conditional
Safe result to apply. Source-only presence or uniqueness cannot establish
those facts.

## Measurement

The read-only BEFORE artifact is the detached `fa04aab0f` worktree's
`/tmp/qcore-rk-outer-base/private/rk-outer/measure-base/`. Its `head` records
`fa04aab0fd43b242d2a996da2c6c507f1eb2a113`; `corpus/run.json` records
pinned source `66910c10`, jobs 4, `/usr/bin/clang`, binary SHA-256
`336609c835dad7762e3729bfc699af561a61d1704f318e2db0f6fbb57e145ff7`.
All three route exits are 0. The system and slice invocation sidecars name
the same checkouts, compile databases and selections used by this card's
`scripts/measure.sh`.

The BEFORE report SHA-256 values are corpus
`fd731251a789cde595ff8f5d7b66bbe9cdf9424c7bb1c64bc7bbff51eeec1701`,
system `1483b2aaad5b8eeec49a288998f46139b988e61581eac3b8a8aee650c76fc38b`,
and slice `9c0ae577e27984b7b63339ca0dee1e8a8b7e256c3419232f3cd6da455e399afa`.
The BEFORE counts from those reports are:

| route | Safe | Pending | provisional Unsafe | Unknown | functions |
| --- | ---: | ---: | ---: | ---: | ---: |
| corpus | 941 | 0 | 9 | 3,884 | 4,834 |
| system | 97 | 0 | 4 | 347 | 448 |
| slice | 52 | 4 | 1 | 347 | 404 |

The exact-base corpus has three Unknown callers whose *only* diagnostic
leaves are uncontracted calls to Safe other-unit home rows:
`cio_chunk_hash` (`cio_file_hash`), `ctxTypeID_to_size`
(`LZ4_sizeofState`, `LZ4_sizeofStateHC`) and
`rd_kafka_share_consumer_closed_err` (`rd_kafka_error_code`,
`rd_kafka_error_destroy`). These are candidates, not predicted guaranteed
promotions: export and caller replay can still refuse them. The report has
10,249 `call to uncontracted function` leaves, but no typed parity stream,
so the complete held-position-call census remains to be measured.

The first configured AFTER corpus run (`private/w44xu/after/`, binary SHA-256
`dc4a9da815462af7d474343af4a166c3acef9d39d060bf78dd7356bf945c3eba`)
exited 0 on 2026-10-17 after 40m 50s. Its report SHA-256 was
`9ef7c3799f095aa60b8a3614215302f48e117da940c16f6afa5e369935814242`;
the typed parity stream held 3,472,780 records, SHA-256
`f98df7162ce1c052c2b6433dc5b767fad762d4946180d0c85b81cbd57738b2a6`.
The exact-base comparison (`private/w44xu/after-v1-diff.json`) aligned all
4,834 functions and found **zero verdict or diagnostic-leaf changes**. The
report did record all five home identities and the link premise, so admission
and certificate replay completed; the zero gain was an implementation defect,
not a link-map refusal.

The defect was an environment rebuild in the gateway's ABI withdrawal pass:
when an unrelated declared contract was withdrawn against a C declaration,
it copied ordinary declarations but lost the separate inferred import channel
and the caller-expected home/link identities. The checked-specification and
two recursive W4 environments had the same preservation gap. Commit
`f2e235135` carries all three pieces through each rebuild. A focused fixture
now provokes an unrelated ABI withdrawal and replays both the imported home
and Safe caller; focused recursive-environment checks cover both W4 routes.
An isolated actual-source probe with the `cio_file.c` home and `cio_chunk.c`
caller resolves `cio_chunk_hash` to Safe when the import survives, whereas the
first corpus pass kept its uncontracted-call leaf. This is a route regression
diagnosis, not published corpus coverage.

The first run's 40m 50s and 2.2 GiB parity output are a material performance
cost of the opt-in route's full second unit pass and home replay. The 11 GiB private
debug target cache was preserved at the ignored
`/home/cosmo/Documents/GitHub/qcore-c/private/w44xu-target` to free `/tmp`;
remaining gate commands set `CARGO_TARGET_DIR` to that path.

The isolated AFTER binary build ran
`cargo build --release -p qcore-audit --bin quirin-audit -j 4`, exit 0
(`private/w44xu/release-build.log`); binary SHA-256
`dc4a9da815462af7d474343af4a166c3acef9d39d060bf78dd7356bf945c3eba`.

The corrected release build (`private/w44xu/release-build-v2.log`) exited 0;
its binary SHA-256 is
`bb5d47fc875bca4f4afa40c17fc8c4bd1c0c3632dd1bc87532f10ad8d4a6f866`.
The configured AFTER corpus (`private/w44xu/after-v2/`,
`private/w44xu/after-v2-parity.jsonl`) exited 0 with pinned source
`66910c10`, four jobs and `/usr/bin/clang`, from 09:00:17 to 09:43:19 UTC
on 2026-10-17 (43m 02s). The report SHA-256 is
`84b26545b3fcc25c483467ea0b64bf0ad6ce941f530376aca61e80e94de6d04d`;
3,472,478 typed parity records have SHA-256
`b9a6932b35798c29878bd69175bff3bb552a88b2816ca11a56fae55d1153f35e`.
The exact-base diff is `private/w44xu/after-v2-diff.json` (compare exit 0).
All 4,834 functions align:

| BEFORE → AFTER | Functions |
| --- | ---: |
| Safe → Safe | 941 |
| Unknown → Safe | 3 |
| Unknown → Unknown | 3,881 |
| provisional Unsafe → provisional Unsafe | 9 |

The three promotions are `librdkafka/lz4frame.c::ctxTypeID_to_size`
(its `LZ4_sizeofState` and `LZ4_sizeofStateHC` call leaves clear),
`librdkafka/rdkafka.c::rd_kafka_share_consumer_closed_err`
(`rd_kafka_error_code` and `rd_kafka_error_destroy`), and
`chunkio/cio_chunk.c::cio_chunk_hash` (`cio_file_hash`). Their AFTER rows
have no diagnostics and name their imported summaries among their contract
dependencies. There is no Safe or provisional Unsafe regression.

In all, 41 rows change diagnostic leaves: the three promotions and 38
still-Unknown rows. The latter expose deeper caller-local obligations once
a pinned callee effect is available. Across the corpus, 56 uncontracted-call
leaves clear. For example,
`librdkafka/rdkafka_admin.c::rd_kafka_DescribeConsumerGroups_response_merge`
replaces its `rd_kafka_error_destroy` call leaf with unresolved `error`
identity and a nullable-resource/non-null-contract leaf; and
`librdkafka/rdkafka_admin.c::rd_kafka_AclBinding_destroy` replaces the call
leaf with indirect memory access outside the resource projection.
`librdkafka/lz4frame.c::LZ4F_compressBegin_usingCDict` loses two calls to
`ctxTypeID_to_size` yet remains Unknown on its other leaves. Aggregate added
leaves are 23 unresolved `error` identities, 14 indirect accesses, one
nullable-resource/non-null-contract and three unmodeled formal-resource
mutations. Every such row remains Unknown; clearing a callee leaf neither
suppresses those obligations nor promotes the row.

The selected link map's raw SHA-256 is
`ddd0576cde87a5782f1fe0b9c4ce4efad4042b1ee54a483a471a3576f0698e7e`.
Each AFTER unit's `cross_unit_link_resolution` retains that digest, the
explicit linked-program resolution/type/calling-convention/interposition
premise and the five imported home identities (raw source, preprocessed
source, loaded environment). The home units are `librdkafka/lz4.c`,
`librdkafka/lz4hc.c`, `librdkafka/rdkafka_error.c` and
`chunkio/cio_file.c`. The premise remains conditional on the actual linked
program; this source-only run does not independently verify linkage.

The configured AFTER system and slice routes both exited 0 under the same
map and release binary. The system report (`private/w44xu/after-v2-system/`)
has SHA-256
`9a2355ab25cb55bd395cf0b8f8e9a13a2f3584ada82b9b7189dabe390a76e4b7`;
the slice report (`private/w44xu/after-v2-slice/`) has SHA-256
`dfd963d0bd9767fb4f75f0eba5afb7fae52d06547eb364edaa95b92d8aebe25d`.
The respective typed parity streams have SHA-256
`a601a3447f0eb2bf591e5de2e839ed67f213a293e0539cf98c2c55501c403b4e`
and `555abf393f24905c30f5e2010f2f616342c22bfe303e2e1d9265d1544632f2b6`.
Exact-base comparisons in `private/w44xu/after-v2-system-diff.json` and
`private/w44xu/after-v2-slice-diff.json` align 448 and 404 function rows.
System remains 97 Safe / 0 Pending / 4 provisional Unsafe / 347 Unknown;
slice remains 52 / 4 / 1 / 347. Neither route has a verdict or diagnostic-leaf
change. The selected-command order is pinned; the comparator uses its
occurrence index for duplicate source paths, since the import trust context
changes the full compile-configuration blob between BEFORE and AFTER.
No requested home was admitted in the system selection, so it did not install
a link premise. The slice console records `cross-unit import phase refused:
incomplete definition census`; it kept the phase-one result without a link
premise. Both are explicit fail-closed no-gain cases.
