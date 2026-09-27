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

The fixed-source corpus AFTER and its complete comparison are pending at the
four-job arbiter slot. The preliminary binary at `a95be942` completed a
four-job corpus child with exit 0, but its wrapper was interrupted with 130
after completion. It lacked the positive field-owner check and is superseded.
Its 45 `Unknown`→`Safe` moves exposed nine P1 predicted-Safe misses and 16
unforecast gains, which are investigated against the fixed run below.

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

TBD after the standing gate: exact commands, exits, totals and private logs.

## For the integrator

TBD after measurements: precise STATUS/matrix changes, named followups and
the increment paragraph. The integrator owns those shared files.
