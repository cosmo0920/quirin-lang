# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: S3

The implementation supports scoped conditional prerequisites and exact
same-unit forwarding of declared registration obligations, with independent
certificate replay. Conditional record correlation and whole-program threaded
teardown remain unresolved; syntax alone never supplies either proof.

Preparation base: `654e2b7eed72374b02ea39ef41059f7a9149b776`; integration
base: `e8b9ff07d`; frozen analyzer/proof source: `d0a48bfd6`. Branch:
`codex/fb-lifetime-s3`, isolated at `/tmp/qcore-fb-lifetime-s3`.
Fresh corpus, system and slice measurements all exit 0. This is the frozen
schema-18 semantic checkpoint; the coordinated unpublished schema-19 event
with the detach card must close compatibility before push.

The task is the existing ledger entry `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`:
implement the user's S3 declared lifetime protocols after the access/identity
blockers. The historical census and decisions remain in
[the design report](../fb-event-loop-cross-function-lifetime/README.md).
D1/D5 selects S3; D3 restricts equalities to derivation from proven single
stores and immutable flags; D6 orders the blockers first. Removal poisoning
and loop destroy ending records already belong to the separate
RETENTION-UNMATCHED-REMOVAL-POISONS card and must be preserved.

## User direction and implementation grammar

The user supplied scoped conditions, `quant #threaded in { … }: [[local]];`
and `quant not #threaded in { … }: [[local]];`, in response to the S3
condition question. The coordinator directed implementation of this form
with an explicit read-only condition binding, such as
`quant #threaded = (#ins.is_threaded != 0);`. This is the chosen local
binding grammar; it is an implementation choice, not an extra Boolean
assumption. The first supported blocks contain conditional prerequisites and
lifetime-protocol clauses. Unsupported inner clauses reject the module.
`requires resource_contract_v1` stays unconditional. The new footprint is
`quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;`.

[LOCAL-CONDITIONS-DESIGN.md](LOCAL-CONDITIONS-DESIGN.md) preserves the design
and boundaries discussed before implementation. It records the authorized direction and the remaining evidence boundaries. Equalities remain derived-only under D3, and neither unknown
conditions nor local prerequisites are assumed true. This feature is
separate from C04.20 outer-contract static/runtime scenarios.

The production host is the existing pinned `flb_input_set_collector_time`
declaration in `fluent_bit/input_collector_registration.qv`; that declaration
already establishes the record via the checked `collector_create` helper.
The new line is inside its existing contract, beside the id-forwarding line:

```qcore
extern fn flb_input_set_collector_time(ins: ptr<mut, u8>, cb_collect: fnptr, seconds: i64, nanoseconds: i64, config: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    // Existing custody and callback-registration clauses remain here.
    quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;
};
```

This is a location fragment, not a full replacement contract. The footprint
is type-wide and checked against every registration/store in the run, including
the event and socket setters and start/resume bodies. Its provenance names the
host's revision pin and a dedicated protocol clause/evidence kind; W4 custody
evidence does not validate it. There is no standalone module-level `quant`
grammar promise and no new custody declaration for `collector_create`.

Any future conditional equality must also name the existing explicit rule-10p premise
that the threaded flag is immutable after initialization. Source has both
initialization and a `threaded` property-setter write; absence of writes is
not proved. Any writer outside that premise prevents the equality from
discharging a dependent lifetime.

## Implementation acceptance

The registered footprint is keyed by record/interior and holder path. Every
registration and store site in the run must be checked. A contradiction
withdraws the declaration; a site that cannot be checked makes its use an
explicit named declared premise. An open or empty candidate set cannot
validate the footprint. Allocation-site candidates establish candidate coverage only; runtime
identity or disjointness needs its own checked evidence. Overlapping
candidates never establish runtime identity.

The summary preserves may-registration effects and under-approximates must
removal/destroy effects on all exits. Substitution must preserve root identity;
an unnameable root lifts to a closed type footprint instead of disappearing.
Releases need exact or derived holder equality and a dominating must-ending,
or forward an explicit lifetime precondition. The run fold only demotes:
every caller discharges or forwards every precondition; an unresolved root
refuses its dependent chain. Poisoned records end only when their holder
ends. An unchecked or absent join never silently discharges a lifetime.

Target placement follows GENERALIZATION_PLAN §§2–3: portable contract/schema
data in `qcore-schemas`, parser/loader in `qcore-contract`, source-derived
summaries and adapter in `qcore-c-gateway`, run fold and dependencies in the
existing audit integration, matching proofs in `proof/QCore` and an
`Audit*.lean`. The existing `retained_by`, `legacy_retained_by`,
`callback_member_closure` and per-clause standing readers are integration
points. No compiler compatibility implementation or provider-to-core
dependency is added.

Required theorem families are summary-substitution overapproximation,
footprint-lifting coverage, admitted releases having no live registration,
unproven equality ending nothing, poisoned-record preservation until destroy,
fold demotion, validated-declaration equivalence, withdrawn-declaration
non-use and premise attribution to every dependent verdict. Shared Rust/Lean
decision and trace vectors bind the executable model to its proof. Numeric
model identifiers do not prove production canonicalization: exact typed source,
condition/member identity, formal-to-actual substitution and stable object/value
snapshots remain explicit extraction premises. The source route rechecks its
bounded evidence and refuses unsupported correspondence.

## Compatibility boundary

The running semantic checkpoint and its package artifacts use unpublished
`qcore-schemas 18.0.0`. S3 adds two public Rust struct fields:

| Field | Migration for existing construction | Absent-field wire behavior |
|---|---|---|
| `ResourceSummaryRecord.lifetime_effects: Vec<ForwardedLifetime>` | Add `Vec::new()` or use `ResourceSummaryRecord::new` | Defaults empty; omitted when empty |
| `RetainingContract.lifetime: Option<LoopLifetimeContract>` | Add `None` or use `RetainingContract::custody_only` | Defaults absent; omitted when absent |

Old wire bytes remain unchanged for declarations/summaries using no new
feature. That does not make public Rust struct additions source-compatible:
the schema README's 2.0/3.0 precedents require a major compatibility event.
This omission was found after the schema18 gate/package checkpoint and is
explicitly not waived. The coordinator selected one unpublished schema19
compatibility event covering these fields and the concurrent detach card's
new return-mode variant. Its final cumulative gates, all five package
rebuilds/verifies and production audits are required before push. S3's
schema18 results are a frozen semantic checkpoint, not the final publishable
compatibility state. No unmerged detach variant or version is inserted into
the running measurement.

## Validation checkpoint

The frozen analyzer/proof source `d0a48bfd6` passes the host-access standing
phase: **8,307 Rust passed / 0 failed / 15 ignored** (516 units, 529 summaries),
format, clippy, both reevaluation examples and **341 Lean audits** all exit 0.
The [actual commands and private paths](artifacts/final-standing-gate/commands.json)
retain the exact source and full runner summary. Post-reconciliation matrix
regeneration, all **374 root Python tests** (26.229s) and the documentation
index check also exit 0; [closeout receipts](artifacts/closeout/commands.json)
record their exact private paths.

The [provider gate](artifacts/provider-gate/commands.json) passes contract
sync/check, **133 Rust tests / 0 failed / 1 ignored** (43 summaries), format,
clippy and **25 Python tests**, package build and all five explicit verifies,
all exits 0. These are the schema18 checkpoint packages; the coordinated
schema19 event requires fresh cumulative packages before publication.

[Earlier attempts](artifacts/attempts/) remain separate: the first full build
stopped at an exhaustive test match for the new frontend variants, then clippy
style findings; the first host suite passed all 8,307 tests but exposed further
style findings. The final frozen gate supersedes those failures. Preparation
compiler/proof/fixture and provider-sync failures are retained with their
actual exits/private logs, including the misplaced initial provider line that
was corrected before pinning. No safety boundary, sanitizer setting or negative-control verdict
requirement was weakened. The sandboxed submodule config failure 128 and host retry 0 are
retained; actual native suites run with host access.

The changed time-host [source-pin checks](artifacts/pin-checks/commands.json)
pass on all three prepared trees. Corpus/system outer checks pass; the two
extra strict slice outer checks return 1 because its git revision is not
recorded in `reviewed_at`, while every loader body identity is in force through
the reviewed 669/046 bytes (0 stale, 0 not in force). Those checks are recorded
as non-green, not waived mandatory gates or all-green pin checks. The fresh
slice runtime inventory must independently confirm 3 in force / 0 held.

## Measurement and completion

[PREDICTION.md](PREDICTION.md) preserves the original pre-code prediction;
later direction and integration changes are dated addenda. The immediate before
observation is the frozen lexical `913a72a91` measurement, explicitly reused,
covering corpus `66910c10`, system `046accf6` and slice
`34155955e4962a56d94eb23c03a002880899a0d1`. The final composition also contains
the user's RK envelope correction. [The exact source-boundary record](artifacts/baseline-scope.json)
retains the lexical-to-integration-base diff: no analyzer or proof change, only
the outer text/inventory/lock correction and its tests/count. Outer vocabulary
is excluded from static resource selection; runtime changes still require
per-row attribution. This is not an isolated S3 runtime experiment. Historical
`854920e3e` measurements remain cumulative evidence, not a fresh immediate base.

Final source/binary/input identities, jobs, exits, complete row changes and
private logs accompany the fresh composed routes. Every moved row is read.
No unjoined teardown reading becomes a validated upstream defect, and no
measurement claims deployments satisfy its declared premises.

The full standing gate in `docs/agents/validation.md` is required; provider
changes also require their packaging/conformance gate. Heavy work waits for
the coordinator's resource slot. The card closes only with implementation,
audited Lean, measurements, exactly one current increment paragraph and
status/ledger reconciliation. Likely shared files are the contract spec,
grammar, gateway policy, proof imports, acceptance matrix, STATUS and current
increment index/record. The coordinator integrates and pushes; this branch
does neither. Advisory wall-clock budget: 90 minutes, not a stop rule.

## Follow-ons retained

Preparation-only source census reran the historical `loop_member_census.py`
`build` function over its three prepared exports, exit 0. Worktree-private
output: `private/fb-lifetime-s3/preparation/source-census.json`. Each tree
again has 19 loop members, 17 closed candidate sets, 7 allocation sites,
51 registrations, 37 removals, 13 waits and 11 destroys. Collector scope has
two holder stores, four registration sites (start/resume, two arms each), two
matching pause removals and two destroy removals at `flb_config.evl`.
This lexical census is source observation, not protocol validation or a
verdict baseline. Submodule initialization completed exit 0 at
`d82464a277d0f42703702c4dfd9af6af38595a83`. These preparation observations
are separate from the final gates and production verdict measurements.

`FB-UNJOINED-TEARDOWN-READINGS` remains a reading under rules 10q/C5.
`W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS` is not discharged by a lifetime
protocol. Open HTTP-server member candidate sets remain open. List membership,
event-loop round ordering and release through erased record types require
their own evidence; no scope expansion is implicit.

## Implementation boundaries under validation

The local conditional path rederives its typed source/call evidence during
standalone certificate replay. It does not borrow ambient run facts. Active
named prerequisites have a supported Hoare entry path: the caller body's own
matching conditional prerequisite, exact substituted object/member/polarity,
and explicit `s3_condition_snapshot_stable` premise. Unsupported owner/thread/
lifecycle discharge and conditional type-protocol correlation remain Unknown.
The local type reader refuses narrowing getters, volatile/atomic fields and
pointer retyping, while may-effects retain physical pointer erasure separately.

An unconditional exact self-record footprint can forward a caller-owned
registration under `loop-protocol-declared:HOST:DIGEST:INTERIOR|->HOLDER`.
This is a named DeclaredAssumption, not proof of a global closure or of all
callers meeting release obligations. The local CFG retains the outstanding
record and poison bit; settled exits export it, with the exact formal/member
paths and declaration identity, into the function and inference records.
Forwarding proof joins require proof on every arm where a record is present;
an unproved new registration cannot reuse an ended record's old evidence.
Source/configuration-qualified run roots and caller graphs demote unresolved
roots/callers; no source census substitutes for complete must-ending evidence.

Bounded same-unit summary transport carries custody and lifetime effects as
one separately routed record, rederived during certificate replay. Ordinary
borrow-only W4 records cannot erase these effects. Cross-unit retaining import
remains refused until its home/effect evidence is retained and independently
replayed. Unjoined worker teardown, unknown actuals and unproved holder
identities retain their obligations. Model controls preserve may-release
ordering: destroy-before-release can discharge an exact concrete holder;
release-before-destroy and lifted-footprint prefixes cannot.

New lifetime members and qualified guarded/protocol assumptions are dependency
and runtime outer facts. S3 adds no envelope coverage. The composed tree
separately removes the four unrelated Fluent Bit promises from the RK envelope,
as directed by the user. Protocol identifiers
containing source digests are not expressible by current `#premise NAME` and
therefore remain runtime Unknown; a follow-on must define a reviewable outer
identity/envelope surface before these facts gain runtime coverage. This is
`S3-RUNTIME-PROTOCOL-ENVELOPE-IDENTITY`, not a build/tracking exception.

The source summary withholds both must endings and release prefixes across
member stores, unread call frames, or incomplete control flow. Custody-only
contracts do not establish heap frames. Direct, typed, complete no-store
traces retain their exact ending evidence; the symbolic model assumes stable
object/value snapshots, whose source correspondence remains an explicit
extraction obligation. A protocol assumption supplies no arbitrary equality
or stability.

Condition-bearing declarations live in an exclusive guarded schema table.
Raw retaining-table prepasses cannot read them. Pin withholding/restoration,
withdrawal, source replay, inheritance, and ABI/dependency enumeration preserve
that partition. Forged placement or missing identity rejects; an ordinary
overlay cannot replace the guarded interface. Structured
callback/reference/handoff mixtures remain refused at the S3 use gate until
guard-aware body/effect consumers exist (`LOCAL-CONDITION-STRUCTURED-EFFECT-CONSUMERS`).

Declared guarded interface names remain identity metadata after withdrawal or
a pin hold, preventing a bare W4/import/ordinary overlay from erasing that
interface. They grant no entry premise or effect. Only an active declaration
can supply its body's conditional requires. The publication policy has shared
Rust/Lean vectors, and three-function controls check both missing and matching
guarded caller contexts.

## Composed production measurement

The retained binary SHA-256 is
`f5ca84194827defe0c5dc36f59d7424ccf1e91eb252e324e0e2d5c458eedd974`.
Invocations record documentation checkpoint `14dfc6aac`; analyzer/proof source
is frozen at `d0a48bfd6`. Exact commands, source/configuration/link-map hashes,
times and all primary artifacts are in [measurement.json](artifacts/measurement.json)
and each route's `before`/`after` manifests. The baseline remains lexical
`913a72a91`, with its original RK envelope. This measures the composed S3 and
RK scope correction, using the source-equivalence argument above; it is not
an isolated S3 runtime experiment.

| Route | Rows | Safe before → after | Unknown before → after | Other statuses | Audit |
| --- | ---: | ---: | ---: | --- | --- |
| Corpus | 4,834 | 963 → 961 | 3,862 → 3,864 | 9 provisional Unsafe unchanged | exit 0, 51m 27s |
| System | 448 configurations | 99 → 99 | 345 → 345 | 4 provisional Unsafe unchanged | exit 0, 6m 43s |
| Slice | 404 configurations | 53 → 53 | 346 → 346 | 4 Pending, 1 Unsafe unchanged | exit 0, 5m 39s |

All 5,781 static null-property dictionaries are identical. No production row
is promoted by a lifetime assumption. The corpus loses two admissions:
`rd_kafka_event_destroy` and `ut_op_destroy_free`, direct wrappers around
`rd_kafka_op_destroy`. New lifetime tokens `event`, `evl` and
`flb_input_collector` expand the existing member pre-pass. Corpus reads change
from 987/0 unread to 1,059/36 unread over 1,444 units; system from 966/0 to
1,038/36 over 1,419, and slice from 968/0 to 1,040/36 over 1,421. Every route
takes 2 → 4 rounds and retains 100 → 12 scope exclusions, with 88 dropped:
77 cite `minilua.c` naming `event`, 11 cite `flb_config.h` naming `event`/`evl`.
The 36 unread entries comprise two C++ units and 34 preprocessing failures,
including missing generated files/headers. Existing run-global unread policy
therefore opens every callback closure, including `rd_kafka_op_s.rko_op_cb`.
This is conservative source-read exposure, not a lifetime violation.
`prepass-review.json` retains all unread reasons, exclusions, changed closures
and the existing exercised Rust/Lean refusal controls. Restoring precision
requires independently proved graph isolation or readable inputs under
`S3-PREPASS-UNREAD-GRAPH-ISOLATION`; unread input cannot be ignored.

Every changed row is retained and reviewed: corpus has 89 diagnostic changes,
all librdkafka closure consequences, and seven additional dependency-only rows
after `ut_op_destroy_free` loses its inferred summary. Both demoted source
bodies were read. The configured routes retain every custody diagnostic;
their four/eight raw row changes are review-hold checkout paths. Slice adds
the new protocol clause with an **empty** body-evidence list in both
`flb_input.c` configurations. W4 does not validate that clause. The run's
protocol remains a named DeclaredAssumption with explicitly unchecked sites,
not a validated global protocol. Supported same-unit forwarding, independent
replay and conditional prerequisite positives/negatives are exercised by the
green Rust/Lean gate; unresolved production callers stay unresolved.

The five artifact families are accounted separately. Source wrappers and
chunk contracts are unchanged; the sole resource module text change is the
collector registration declaration. All 185 corpus cross-unit resolution
changes are environment hashes, with identical functions, home paths and
raw/preprocessed sources. Three source-effect results have only 17 SHA-256
scalar changes and derived obligation-ID changes; all other normalized values
and structure agree. Full raw differences and comparison-only normalized
reviews remain archived. The first slice postprocessing check failed because
it expected no body-evidence additions; its log is retained, and the corrected
check explicitly requires the new clause's empty evidence rather than
silently discarding it.

RK scope attribution matches the historical four-key dependency census:
under both all-loaded and RK-only, null Pending → Unknown is **1/2/16** rows
and combined Pending → Unknown **1/2/0**, corpus/system/slice. Fluent Bit-only
null and combined projections are unchanged by that envelope correction.
Every affected null reason explicitly says RK lacks the corresponding
`#kept` line. Corpus additionally removes the two demoted bodies' custody and
combined Pending moves in every scenario. All-loaded corpus custody is
**931 Safe / 30 Pending / 3,864 Unknown / 9 provisional Unsafe**; null is
**2,386 / 162 / 2,355 / 18**, combined **808 / 30 / 4,056 / 27** in
Safe/Pending/Unknown/Unsafe order. Digest-only RK reason changes are retained
separately in each `runtime-attribution.json`. Every fresh projection has two
outer contracts in force and all three loaders without a pin hold, including
slice's reviewed-body identity reuse; the extra strict slice outer CLI
preflights still exit 1 as disclosed above.

The original committed prediction is preserved. Its lack of predicted
production promotions holds, but the two callback-closure demotions were not
predicted release obligations and are an explicit precision residual. No
unjoined teardown reading is validated and no deployment claim follows.
