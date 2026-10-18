# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: S3 implementation brief

Preparation base: `654e2b7eed72374b02ea39ef41059f7a9149b776`.
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

## Required decision

D4 is still unanswered at preparation time. The coordinator has asked whether
to approve both the new registered clause and integer member guards, only the
registered clause, or a revision. The proposed clause is:

```qcore
quant registered #coll: flb_input_collector.event |-> flb_input_collector.evl;
```

The member guard spelling is `when #coll.instance.is_threaded == 0`.
Approving that spelling does not authorize declared loop equalities. The
historical `quant live` equality example is superseded by D3: the equality
must be derived from stores and an immutable flag. The existing outer-contract
`#registered` value (C04.20) describes loaded-code callback envelopes and has
different syntax and meaning; the S3 clause does not reuse its semantics.

No dependent parser, contract or semantic implementation starts before the
coordinator forwards the user's explicit D4 answer. Gate/push authorization
is not a syntax answer.

The later user proposal for `quant #threaded in { … }: [[local]]` and its
`quant not` complement is addressed in
[LOCAL-CONDITIONS-DESIGN.md](LOCAL-CONDITIONS-DESIGN.md). It proposes explicit
typed condition binding and scoped conditional prerequisites, and records the
remaining choices; it does not treat that design question as syntax approval.

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
validate the footprint. Allocation-site candidates establish coverage and
disjointness only: overlapping candidates never establish runtime identity.

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

[PREDICTION.md](PREDICTION.md) is committed before implementation. Capture
fresh corpus `66910c10`, system `046accf6` and slice `90978ac6` baselines and
comparisons with exact binary/source identities, jobs, exits and worktree-private
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
