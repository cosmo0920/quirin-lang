# W48-BU-RK-RESULT-CONSTRUCTORS prediction (rule 37)

Recorded before any active Rust, Lean, provider or contract semantic change.
Launch BASE: `8991b7b58fcbd33da082bbb44e819d47a0fb2561`. The held BASE
binary was built at doc-only `1e99901c`; all tracked non-report files remain
BASE-identical. Its SHA-256 is
`01a0402f62e9edd1187e644febe132c44c138072d291a2f11aedfe00d4e25fd0`.

The own fresh corpus run completed 2026-10-23 19:26 UTC, exit 0; the
before/after input comparison also exited 0. Static custody: **999 Safe,
3,826 Unknown, 9 provisional Unsafe, 4,834 replay rows in 185 units**.
These are selected custody rows, not whole-program memory-safety coverage.
Unknowns remain in the denominator. Configured BASE system and slice runs
are continuing with the same held binary; their data remains frozen.
[Exact input/planner receipts](BASE-PLANNER-RECEIPT.json) retain hashes and
private paths. Failed setup attempts remain in [setup-attempts.json](setup-attempts.json).
No historical W47 or batch5 figures supply this baseline.

The planner was rerun on this corpus (exit 0) and its relevant rows inspected:
2,200 callee bodies, 230 positive, baseline closure 6 same-unit / 6 cross-unit,
383 ceiling callers. Its scores are hypothetical ceilings, not Safe gains.

| Callee | Own leaves | Same / cross caller ceiling |
| --- | ---: | ---: |
| ConsumerGroupListing_new | 6 | 2 / 2 |
| topic_partition_new | 4 | 1 / 2 |
| ListOffsetsResultInfo_new | 4 | 2 / 2 |
| topic_partition_result_new | 5 | 2 / 2 |
| ListConsumerGroupsResult_new | 6 (4 uncontracted list calls) | 2 / 2 |
| topic_partition_list_add0 | 13 | 2 / 2 |
| oauthbearer_set_token0 | 21 | 0 / 2 |
| MetadataRequest0 | 61 | 1 / 2 |

## Proposed change and assumptions

Reuse the existing checked non-VLA `sizeof` operand reader in aggregate base
escape and allocation-argument insulation scans. It supplies neither an identity
nor non-nullness. Keep the fresh, unique, live, non-null owner and all alias,
member, overwrite, shallow-release and call-effect gates.

Replace only `rd_calloc` / `rd_strdup` nullable inheritance with explicit
`resource_body_checked` non-null specifications and the named
`asserted_allocation` premise: the deployed build preserves the analyzed
assertion and its declared terminating failure call; allocation failure has no
normal return. Review/pin all three source revisions listed in
[allocator-source-review.json](allocator-source-review.json). A missing body,
disabled/unread assertion or unproved failure-call effect must withdraw the
specification. The premise remains an assumption, not a proved deployment fact.
Leave `rd_malloc`, `rd_strndup`, libc and the eight-home link map unchanged.

The independent null reader must not turn a checked custody declaration into an
unchecked non-null fact. Keep independent null source summaries usable; bind
relevant checked-result metadata in its resource digest. Matching Lean will
model the occurrence exemption, owner gate, normal-return assertion semantics,
child obligations and null-fact filter, retaining extraction/transcription premises.

## Forecast to test

I expect the two simple allocating bodies `ConsumerGroupListing_new` and
`topic_partition_new` to move Unknown -> Safe under the named abort premise.
I also expect their same-unit waiting `ConsumerGroupListing_copy`,
`ConsumerGroupListing_copy_opaque` and `topic_partition_new_with_id_and_name`
to follow: **five primary target rows forecast**. This is a prediction, not a
promotion; newly exposed field/argument or call obligations can falsify it.

`ListOffsetsResultInfo_new` and its two copy wrappers are a secondary potential
three-row gain if their nested copy obligations discharge. The nested stores
and cross-unit home in `topic_partition_result_new`, and the four uncontracted
list calls in `ListConsumerGroupsResult_new`, may remain blockers. Do not sum
single-body planner ceilings. No Safe result is forecast for list_add0,
oauthbearer_set_token0 or MetadataRequest0 from this narrow change.

Other constructors/callers can move because the rule is generic. No exact global
net total is promised. In particular, withholding unestablished checked facts
may conservatively move existing results to Unknown. Attribute every changed
body, status and diagnostic-leaf trade; retain all raw identity/provenance,
null and outer differences. No new Unsafe label is an upstream defect claim.
Run the full standing and provider gates, then fresh TIP corpus/system/slice
with the fixed map/profiles and compare complete retained artifacts. Amend W47
alias/identity tracking only from those measured figures; leave remaining
allocator and call-effect boundaries open.
