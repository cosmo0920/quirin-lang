# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: S3 implementation brief

Preparation base: `654e2b7eed72374b02ea39ef41059f7a9149b776`; implementation rebased onto `854920e3e`.
Branch: `codex/fb-lifetime-s3`, isolated at `/tmp/qcore-fb-lifetime-s3`.
This preparation is not a completed implementation or a measured safety gain.

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

The proposed host is the existing pinned `flb_input_set_collector_time`
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

The conditional equality also names the existing explicit rule-10p premise
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
decision and trace vectors bind the executable model to its proof.

## Measurement and completion

[PREDICTION.md](PREDICTION.md) is committed before implementation. The coordinator’s exact historical `854920e3e` baseline artifacts cover
corpus `66910c10`, system `046accf6` and slice `34155955e4962a56d94eb23c03a002880899a0d1` ; the immediate S3 baseline will be the merged lexical integration, with comparisons recording exact binary/source identities, jobs, exits and worktree-private
logs. Read every moved row. Null-property parity and no unexplained Safe
regressions are required. Do not turn the unjoined teardown readings into
validated upstream defects or claim deployments satisfy declared premises.

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
`d82464a277d0f42703702c4dfd9af6af38595a83`. No gate has run on this
preparation branch; no standing-gate completion is claimed.

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
and runtime outer facts. Existing envelopes are unchanged. Protocol identifiers
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
