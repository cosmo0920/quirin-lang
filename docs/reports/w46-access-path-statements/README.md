# W46-ACCESS-PATH-STATEMENTS

P2 of the [unsupported-coverage plan](../../plans/unsupported-coverage/README.md),
based on `207cebe4cdb745f6216ff2b511c4b7fd134581b9`. The prediction was
frozen before implementation in [PREDICTION.md](PREDICTION.md). The earlier
[P1 prediction](../w46-access-path-census/PREDICTION.md) is unchanged.

## Implementation and boundary

`typed_access_path.rs` reads Clang's typed AST for the exact analyzed source,
dialect and frontend flags in each gateway unit. It uniquely matches a
complete statement by internal line and tokens, then lowers scalar paths and
whole pointer-valued path reads through `.`, `->`, `*` and `[]` to
the established indirect-access flow transfer. Each tracked root and loaded
pointer hop keeps the existing null/liveness obligation. A non-pointer store
retains the custody frame. A pointer read binds a target-free, possibly-null
borrowed alias and never transfers custody.

Missing or ambiguous AST facts, unmatched source, unions, bitfields,
casts within a path, volatile/atomic paths, pointer arithmetic, mixed
paths with a tracked root, pointer stores and call-bearing statements refuse. This card
does not serve conditions, returns or call arguments (P3). A narrow no-access
arm admits scalar arithmetic and declarators that Clang proves do not touch
memory; multi-pointer aliases require every source and destination to be
statically never-bound to tracked custody. Source-scope and certificate
re-derivation both pass the same original dialect and frontend flags.
Mixed embedded-record and indexed paths are admitted only when the root is
statically never-bound to a tracked identity; every pointer loaded from that
untracked graph is outside this resource projection. A tracked mixed path
still refuses because the existing field-link state cannot name its nested
places. A focused indexed path test checks both arms.
Each member ID must have a direct owning Clang `RecordDecl` with a complete
struct/union tag and a field type. Missing or conflicting ownership refuses;
union-owned fields and bitfields refuse even when a typedef hides `union` in
the expression's surface type. Held-out typedef-union and nested union-pointer
controls check that boundary beside a typedef-struct positive control.
`sizeof` is admitted as no access only for a childless, non-array AST type:
Clang represents evaluated VLA bounds as children, and the held-out
`sizeof(int[(close(fd), n)])` control stays `Unknown`.

The established indexed-element, scalar-member-store, scalar-field-noop,
typed scalar, chain, sub-object-store and pointer-step shape readers stay
first. With
`QCORE_TYPED_PATH_PARITY` set to an output JSONL path, each consulted
statement records both readers' complete plans, any disagreement, and the
analyzed source identity. The frontend request is identified by SHA-256 of
its retained request record rather than repeated verbatim in every row. A
companion context stream archives the exact request preimage once per digest;
the comparison checks every digest before archiving both streams. The AST
receives those same flags.
The process writer is mutex-protected, and each row has stable source,
function, line and route keys; archive scripts sort rows canonically, so
four-job audits do not depend on arrival order.
Routes with a different instruction vocabulary record every overlap as an
unresolved structural comparison, including cases both admit; they make no
parity claim. Verdict/coverage comparison remains the retirement arbiter.
No shape route is retired before verdict and coverage parity.

The matching Lean module is `proof/QCore/TypedAccessPathStatements.lean`,
registered in `proof/AuditTypedAccessPathStatements.lean`. It proves refusal
on missing/unlisted forms, per-hop discharge, non-pointer frame preservation,
the target-free borrowed binding, and the untracked declarator boundary.
The borrowed proof requires `frame = none`, matching the Rust reader's only
reachable whole-pointer-read form; a borrowed read with a write frame refuses.

## Measured corpus and system slice

The integrator's fresh unchanged-implementation BEFORE at `29b799f9`
completed the pinned 185-unit corpus arbiter with exit 0. All five primary
artifacts were byte-identical to the prior archived baseline; the diagnostic
SHA-256 is `c784c398a174a8c582821ab067e46245954df068d3897dfa7374db69bd149c69`.
Its 4,834 functions are 411 Safe, 9 provisional Unsafe and 4,414 Unknown,
with 41,086 unsupported, 3,573 unknown-alias and 34 leak leaves. The
P1 forecast was frozen on `9bce3986`; six later contract-driven Safe
functions are already present in this BEFORE and overlap none of its 45 P2
same-unit candidates. All 185 corpus units and the function denominator are
unchanged. The
`046accf6` configured system BEFORE also exited 0, with byte-identical
diagnostics, contracts, chunk contracts and source effects. Raw BEFORE runs
are under `private/w46-next/baseline/{corpus,system}` in the main checkout;
the [archived baseline](../w46-contract-access-expansion/artifacts/baseline/manifest.json)
and [corpus identity check](../w46-contract-access-expansion/census/baseline-corpus-identity.json)
are owned by the integrator.

The fixed-source corpus AFTER completed with exit 0 at four jobs from
`753cde34` (exact command, binary SHA, frontend environment and times in
[run.json](artifacts/after/run.json)). It retains all 185 units and 4,834
functions. The 411 Safe / 9 provisional Unsafe / 4,414 Unknown BEFORE becomes
451 / 9 / 4,374: exactly 40 `Unknown`→`Safe`, with no `Safe` correction or
new provisional violation. `librdkafka/rdcrc32.c` is the one newly Safe unit;
unit totals become 6 Safe / 8 provisional Unsafe / 167 Unknown / 4 data-only.
Among the 167 Unknown units, 166 retain an unsupported leaf and one retains
an incomplete-summary reason; 18 of the 185 units have no unsupported leaf.
Unsupported leaves fall 41,086→38,729, unknown-alias rises 3,573→5,584,
and leak stays 34. The [corpus comparison](artifacts/after/comparison.json)
names all 40 moving functions and all 1,097 functions with any changed
diagnostic; the [manifest](artifacts/after/manifest.json) binds the complete
sorted parity stream, exact frontend request preimages and primary outputs.

Against P1's frozen 45 same-unit candidates, 29 of 38 predicted Safe become
Safe; nine stay Unknown. All seven predicted Unknown stay Unknown. Thus 36/45
predicted verdicts match; the comparator's narrower `p2_hits=34` requires a
changed diagnostic multiset as well, and two of the seven Unknown rows have
none. Eleven Safe gains were outside P1's candidate set (five cmetrics, four
librdkafka and two Monkey); every name is in the comparison's `moves` and
`p2_forecast` records. The three P1 cross-unit candidates were excluded from
this 45-row prediction and have no gained verdict here.

The nine predicted-Safe misses have source-form causes:

| Function(s) | Observed residual |
| --- | --- |
| `flb_input_chunk_get_name`, `flb_input_chunk_is_storage_overlimit` | Initial pointer casts on `ic->chunk` and `i->storage`; casts in paths remain refused. The latter also reads in conditions (P3). |
| `rd_kafka_sw_str_sanitize_inplace` | `char *s = str, *d = str` cannot prove its multi-alias declarator untracked; pointer increments remain outside this path grammar. |
| `rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic(s)` | `rd_tmpabuf_add_alloc` arguments carry the access/`sizeof` arithmetic; call arguments are P3. |
| `ut_testConflictingPreviousAssignments` | Indexed aggregate member pointer stores and call arguments require the P5/P3 rules. |
| `clear_unit_test_data` | Writes fields of the global `unit_test_data`; the local-root never-bound premise does not apply. |
| `XXH32_reset`, `XXH64_reset` | Local aggregate `state` has its address handed to `memset`/`memcpy`, so the untracked-root premise cannot be established for its field stores. |

The preliminary `a95be942` binary lacked the positive union field-owner
check. Its corpus child completed exit 0 (the PTY wrapper was interrupted
after completion and reported 130), showing 45 apparent Safe gains. Source
review found five unsound admissions: `LZ4_favorDecompressionSpeed` and
`LZ4_setCompressionLevel` access the typedef-hidden
`union LZ4_streamHC_u`'s `internal_donotuse` field; the three
`calculate_consumer_commit_latency_max`,
`calculate_consumer_rebalance_latency_max` and
`calculate_share_time_between_poll_max` functions write `.int_value` in the typedef-hidden
`rd_kafka_telemetry_metric_value_t` union. All five are Unknown in the fixed
run. The preliminary run and system artifacts remain in
`private/w46-access-path-statements/after/{corpus,system}` only; no claim
uses them as the AFTER result.

The [corpus leaf attribution](artifacts/after/attribution.json) records every
changed leaf and its source/parity relationship. Of 2,371 removed unsupported
leaves, 2,288 coincide with old-refusal/new-admission statement lines; 73
are now-resolved uncontracted-callee summary leaves, and ten are other
downstream forms. The 14 added unsupported leaves are independent call,
return, nullable-contract or indirect-access refusals surfaced in affected
functions, not newly admitted claims. Of 2,026 added unknown-alias leaves,
1,930 arise at the newly admitted statement and 92 later in the same
function when its borrowed or tracked root still lacks a unique identity;
four are call/condition cascades in functions without a direct statement row.
Fifteen earlier unknown-alias leaves disappear. The source-reviewed unmatched
families include `set_span`'s call argument to newly Safe
`otel_span_set_kind`, `rd_kafka_anyconf_dump`'s condition now reaching an
unresolved `conf`, and `rd_kafka_lwtopic_new`'s member store now exposing
unresolved `lrkt`. The residual nested-call diagnostics in `snappy.c`, the
return expression in `rdkafka_conf.c`, and the remaining indirect accesses
are still Unknown. [Every changed leaf](artifacts/after/changed-leaf-attribution.jsonl.gz)
retains its origin, before/after function verdict and any matched analyzed
source SHA; no changed family was discarded.

The corpus side-by-side stream has 28,386 rows: 13,550 old-refusal/new
admission, 14,683 established-route rows and 153 chain rows. Only three
scalar rows and 144 chain rows have identical typed plans; other rows are
recorded disagreements or distinct-vocabulary comparisons. No established
route was retired.

The fixed-source `046accf6` system slice completed with exit 0 at four jobs
from source revision `753cde34` (exact binary SHA, command, flags and times
in [run.json](artifacts/system-after/run.json)). Across 448 configured rows,
54 Safe / 4 provisional Unsafe / 390 Unknown became 56 / 4 / 388.
The two moves are the same `flb_engine_get_chunk_backpressure_percent`
function in two configurations; across 234 unique functions, 28 Safe / 2
provisional Unsafe / 204 Unknown became 29 / 2 / 203. Unsupported leaves
fell 3,303→3,081 and unknown-alias leaves rose 713→893; counted-user leaves
stayed at four. No leak or provisional Unsafe was added. The
[system comparison](artifacts/system-after/comparison.json) records both
moving rows and all 90 rows whose diagnostic multiset changed. The
[archive manifest](artifacts/system-after/manifest.json) binds complete
side-by-side parity, frontend preimages and primary diagnostics.
Its 6,298 parity rows comprise 1,648 old-refusal/new-admission rows,
4,450 established routes with a different plan vocabulary (all marked
structurally unresolved), and 200 comparable chain rows (120 matching,
80 disagreeing). They are observations, not grounds for deleting a route.

The [system leaf attribution](artifacts/system-after/attribution.json) joins
every changed diagnostic to the side-by-side source coordinate. Of 222
removed unsupported leaves, 220 are at old-refusal/new-admission statement
lines. The other two are the call-summary leaf at
`flb_engine_adaptive_flush_update` after its callee becomes Safe. Of 184 new
unknown-alias leaves, 148 are at those admitted lines; the other 36 are later
uses in seven functions that first acquire a borrowed pointer, such as
`i_ins = ic->in` followed by a condition on `i_ins->storage_type` in
`flb_input_chunk_place_new_chunk`. Four earlier unknown-alias leaves are
removed in `flb_input_chunk_write_header_v2` (two configurations each):
`label_length` at `src/flb_input_chunk.c:1950` and `plugin_length` at 1978
come from scalar reads of `routes[index].label_length` and
`routes[index].plugin_name_length` at 1947–1948 before their guards. The
matched analyzed source digest is
`4fce97f43c687bfa82e771cff913ae89a52809da103281f2c63274549a366f52`.
The new scalar path read clears those unresolved facts. The other downstream
cases remain `Unknown` where the alias identity
cannot be discharged. The complete per-leaf rows, including all origins and
matching source hashes, are in
[changed-leaf-attribution.jsonl.gz](artifacts/system-after/changed-leaf-attribution.jsonl.gz).
The contract and source-effect inputs are unchanged: chunk contracts and
source effects are byte-identical to BEFORE after decompression, while the system contract record
differs only by the checkout's absolute root path and is identical after
normalizing that prefix.

## Validation

The delegated worktree's Libevent submodule was initialized from the local
cache before building (exit 0). The focused gateway integration test ran
seven controls and passed 7/7; the field-owner unit test passed, including a
typedef-hidden union, a union reached through a struct pointer, and a
typedef-struct positive control. The VLA `sizeof` side-effect fixture and
pointer-store refusal fixture stay Unknown. `lake build
QCore.TypedAccessPathStatements` exited 0, and
`python3 scripts/check_lean_audits.py` passed all 213 registered audits with
no project axiom or placeholder. Focused gateway Clippy, `cargo fmt --all --
--check`, and Python compilation of the measurement scripts exited 0. The
fixed release build exited 0 before both AFTER audits.

The integrator's central full standing gate began against committed source
`597ef096` in `private/w46-next-full-gate`. Its first test-runner and Clippy
compiles found an omitted `borrowed_destination` destructure in
`compiler/examples/alias_aggregate_probe.rs` (E0027); the example now prints
that fact when present, committed `fbec0ec7`. The documentation-index check
also failed stale and was regenerated by `python3 scripts/build_docs_index.py`
(exit 0), committed `c5e8c182`. These are the only first-gate failures so
far; the integrator is completing the remaining first run, then rerunning
the full root test runner, Clippy, fmt and index checks against the repaired
tip. Their final commands, exits, counts and private log paths will be
recorded here after completion. No audit binary or semantic gateway source
changed in these repairs.

The preliminary corpus audit at `a95be942` is a recorded failed soundness
attempt, not AFTER evidence. Its child exited 0 and the surrounding PTY
reported 130 after an interrupt at completion. The field-owner repair forced
the five apparent gains above back to Unknown. A release build initially
launched without the dedicated target directory was interrupted with 130
before measurement; the explicit-target rebuild succeeded. Development
fmt/Clippy/focused-test and Lean proof errors were repaired before the fixed
measurement and full gate. The complete preliminary and fixed run logs are
under `private/w46-access-path-statements/{after,final}/{corpus,system}`;
the fixed run logs are also archived with their manifests.

## For the integrator

The integrator owns shared STATUS, hand-maintained matrix fields and increment
history. Update the isolated P2 corpus row to 451 Safe / 9 provisional Unsafe
/ 4,374 Unknown over 4,834 functions and the unit row to 6 Safe / 8
provisional Unsafe / 167 Unknown / 4 data-only over 185 units. Record
unsupported 38,729, unknown-alias 5,584 and leak 34. Update the isolated
system slice to 56 / 4 / 388 configured rows, or 29 / 2 / 203 unique
functions, with unsupported 3,081, alias 893, counted-user 4. The combined
contract-plus-access figures must come from the integrator's combined arbiter,
not from adding isolated gains. The derived matrix already registers Lean
audit 213; preserve the carried gate provenance and hand-maintained ledger.

Proposed named followups are `W46-ACCESS-PATH-PARITY` (compare distinct
legacy/typed plan vocabularies and the scalar/chain disagreements before
retiring any route), `W46-ACCESS-PATH-TRACKED-MIXED-PLACES` (model tracked
embedded/indexed places instead of admitting them under a never-bound root),
and `W46-ACCESS-PATH-POSITIONS` (P3 conditions, returns and call arguments).
Carry `W43-INDIRECT-POINTER-VALUE-STORE`/the plan's P5 for pointer stores,
and `W46-UNION-MEMBER-PATH` for unions; do not weaken either refusal. A
separate residual for multi-pointer alias declarators whose custody cannot
be proved untracked may be merged with `W43-INDIRECT-UNTYPED-SINGLE-ACCESS`
followup, rather than credited to P2.

Proposed increment paragraph: “W46-ACCESS-PATH-STATEMENTS introduced a
Clang-typed, fail-closed statement path reader and matching Lean model while
retaining every shape route. Exact-unit corpus BEFORE/AFTER moved 40 of
4,834 functions from Unknown to Safe and `rdcrc32.c` to a Safe unit; the
system slice moved one unique function to Safe. Positive field-owner facts
withdrew five typedef-hidden union admissions seen in a preliminary run.
The P1 frozen prediction matched 36/45 verdicts, with nine Safe misses and
11 unforecast gains; all diagnostic and parity differences, frontend
requests, and remaining P3/P5/tracked-place refusals are archived under
`docs/reports/w46-access-path-statements/`. The full standing gate [insert
final result and log path].”
