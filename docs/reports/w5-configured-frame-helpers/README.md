# Configured Boolean frame helpers and W7 attribution integration

Recorded 2026-09-07 against main baseline `edba69e`. The Boolean source-contract
route now selects helper source, function, record and policy from configuration.
The gateway imports generic declared records and forbids constant metadata
helpers from writing any modeled field on that record. A different C codebase
executes a separate-TU helper through the existing schemas-only process and
trusted replay. This closes this retained helper selection route; general
lifecycle import and full W5 provider extraction remain open.

## Configuration and checked imports

`source-contracts.json` bindings accept optional `frame_helpers` entries with
`source`, `function`, `record` and `contract`. Source paths are relative to the
selected library root; policy paths resolve from the configuration directory.
The service snapshots all policies, including helper policies, within the same
32 MiB aggregate budget. It permits at most 256 distinct helper function names
per binding, each function/record identifier at most 128 ASCII bytes.

`import_configured_frame_helpers` checks canonical source containment and exactly
one matching compile entry, normalizes the selected file, requires an absolute
compile directory, and rejects compiler options the gateway cannot preserve.
Source reads have an 8 MiB bound and retained imported helper source has a
32 MiB per-call cap. These do not add a transient Clang-output bound.
The explicitly named `_compatibility` entry point is used only by the existing
pinned adapter; its historical option projection remains an outer-audit premise.
There is no automatic fallback from strict parsing.

`import_chunk_frame_helper_with_profile` captures the declared model library,
record, full profile identity and protected-field inventory privately in its
opaque helper object. It checks a complete `void(struct Record *)` body containing
only direct constant i32 assignments to ordinary metadata fields. These writes
must be disjoint from every Boolean field modeled for the record across the
loaded profile. Permissive policy predicates cannot override the intrinsic
protection, prototype, declaration, layout-vector or uniqueness checks.

Caller composition checks record identity, exact parameter binding, external
prototype, source revision, dialect, compiler identity, rule/profile selection,
and ordered declaration fields/types. It rechecks all protected fields even for
legacy helper objects. Existing record and input-identity serialization is
unchanged. The provider still executes the existing `FrameWrites` statement;
no schema, witness representation or production provider binary changes.

`evaluate_boolean_frame_contract` requires the selected nominal record borrow,
unit result, explicit live/exclusive premises and frame predicates. Empty and
modeled-field writes reject intrinsically. Both generic and legacy frame policy
consumers reject writes omitted from the policy, including policies that negate
the footprint predicate. A raw schema evaluation remains declared-model evidence;
the service supplies the additional opaque source-import boundary.

The generic production policy is `contract/qcore/chunkio/error_frame.qcore`.
Retained specialized lifecycle consumers still use `error_state.qcore`.
The corpus now pins 58 modules: 19 resource-loader declarations in five modules,
with 143 other declarations rejected by that consumer and interpreted separately.

## Independent source/process and proof evidence

The existing sample codebase now has `sample_prepare` calling `sample_reset` in
a separate source file. The helper writes metadata while preserving the declared
`sample_record.ready` field. Its actual compile entry, generic policy and source
import feed the configured registry and real child process. Four Boolean input
combinations produce a checked function certificate, and three helper policy
postconditions check. No expected paths are supplied to the provider.

Negative tests cover omitted helpers, wrong layouts, false policies, changed
helper bodies, protected-field writes, missing source, missing/duplicate compile
entries, `-include`, `-U` and response-file options. A relative compile-file entry
with a quoted local header resolves correctly. Gateway tests additionally cover
all-profile field protection, legacy helper reuse under a different model,
wrong record/prototype/parameter identity, changed profile/revision/rules/dialect,
nonconstant bodies and policy attempts to bypass mandatory checks.

Ten additional Lean theorems cover disjoint modeled fields and unrelated object
identities, constant prestate independence, ordered frame writes and unchanged
Boolean/control observations. The [proof note](proof-note.md) identifies the
correspondence premises. Ordered top-level field/type vectors do **not** prove
recursive nested C layouts: archived Clang probes show equal vectors with
metadata offsets 12 and 8. Actual cross-unit layout and linker identity remain
external premises. This increment does not establish caller liveness/exclusivity,
full C memory safety or production reachability.

## Production parity and reporting

The first complete configured 185-unit run reproduced all four baseline files
byte-for-byte. Its actual outputs and empty diffs remain under
`pre-policy-metadata/`. A second full run verifies the final reporting format:

| Artifact | Final comparison with edba69e |
|---|---|
| Source | byte-identical |
| Diagnostics | byte-identical |
| Source effects | byte-identical |
| Chunk contracts | exactly one added helper-policy inventory; removing its 773-byte serialized span reproduces the baseline byte-for-byte |

`configured_frame_helper_policies` records the immutable selected source,
function, record and policy bytes. It is selection metadata, not a claim that
every configured helper was executed. The prior `frame_helper_contract` field
remains the actual policy input of retained specialized lifecycle adapters.
`audit-parity.json` binds exact commands, input/package hashes, all outputs and
diffs; `configured-policy-inventory.json` checks the new metadata against the
configuration and actual policy bytes. The durable Monkey database/source/build
and existing verified ChunkIO0.3.0 package were reused without rebuilding.

Counts remain **185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown**, plus
four data-only units; source effects remain **3 checked function contracts / 93
Unknown**. No raw unit is promoted. Provisional Unsafe is not an upstream defect
claim, and helper function contracts are not whole-unit lifecycle certificates.

## Combined standing gates

All gates passed: **737 workspace tests, 19 provider tests, 43 Python tests**,
workspace/provider formatting and Clippy, both reevaluation probes, Lean build
and 20 axiom audit modules. CT coverage remains 7 mapped /43 unmapped. Logs and
source hashes are archived in `validation.json` and referenced gzip logs.
The five new gateway tests, five host attribution tests, two service configuration
tests and three generic policy tests account for the workspace increase;
two new process tests account for the provider increase.

The independent [W7 attribution repair](../w7-diagnostic-attribution/README.md)
requires the exact selected plugin ID followed by one nonempty 1–128-byte ASCII
local code in both response paths. Nested-provider, empty/control/dotted suffix
codes now reject with QP007 before verdict handling. This fixes diagnostic
attribution; the reproduced cases did not bypass Safe-admission gates. Manifest
and wire shapes, valid local codes, v1 Safe refusal and v2 trusted replay remain.

Owned task execution, shared-memory source support, general lifecycle extraction,
arbitrary helper effects and complete W5 consolidation remain outstanding.
