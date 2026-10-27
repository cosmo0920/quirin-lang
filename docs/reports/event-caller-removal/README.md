# Event caller removal contract

The canonical Monkey contract now states the registration removal performed by
`mk_event_timeout_disable`. Its complete body forwards directly to
`mk_event_del`. A caller can therefore end the exact unpoisoned registration
across this declared API boundary before releasing its object. The call does
not close a descriptor or destroy the loop. The wrapper is a pinned declared
assumption; the body-standing machinery does not automatically prove its
removal clause.

The generated `monkey/event_loop.qv` inventory deliberately grows from 9
functions / 51 clauses to 10 / 56. `providers/contracts.py sync` and
`quirin-audit pin` generate the package inventory and reviewed source locks;
no lock is edited by hand. [Source review](source-review.json) records the
complete body at corpus `66910c10`, system `046accf6` and slice `34155955`.
The slice's historical directory label `90978ac6` is not its preparation
record's source revision and is not used as a reviewed pin identity here.

Two canonical-contract Rust tests cover four caller scenarios: exact removal,
wrong-holder removal followed by a matching disable, disable-before-release,
and release-before-disable. Every Safe certificate is independently replayed.
All four scenarios passed; the negative cases remain Unknown, with
`retention-removal-poisoned` or `retention-released-while-held`. The registered
`QCore.EventRemovalWrapper` Lean model has three audited theorems: disable ends
an exact fresh registration, keeps an already-poisoned one, and does not
destroy its holder. The focused audit passed with only allowed axioms.

## Remaining caller-composition capability

| Boundary | Current support | Still needed |
| --- | --- | --- |
| Direct declared call | Exact object/holder removal, with poison preserved | Automatic proof of removal clauses against arbitrary bodies |
| Same-unit call chain | Bounded S3 forwarding, with retained source and certificate replay | More caller shapes and ordered removal effects through richer summaries |
| Cross-unit call chain | Lifetime-bearing imports deliberately refused | Retain and independently replay complete home/effect evidence; preserve outstanding obligations in import and certificate tables |
| Collector id | Callback registration records the returned id/list | Resolve later start/pause/resume/delete calls to that same collector and its lifetime record |
| Differently spelled loop paths | No equality from spelling or candidate overlap | Derive exact per-record equality from checked stores and stable conditions |
| Threaded teardown | Unproved ordering remains Unknown | Establish join and loop destruction before collector/object release on every feasible path |
| Event descriptor | No new descriptor claim | Member-held/out-slot descriptor identities and their lifetime model |

These residuals remain tracked under
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`,
`RETENTION-DERIVED-LOOP-PATH-EQUALITIES`,
`RETENTION-PATH-LOOP-DESTROY-ADMISSION` and `MK-EVENT-DESCRIPTOR-CUSTODY`.
This increment does not close the complete Fluent Bit collector lifetime chain.

## Validation in progress

Focused Rust: 2 passed, 0 failed. Focused Lean: 1 audit / 3 theorems, exit 0.
Targeted source pin checks at all three prepared roots: exit 0.
The first inventory sync exited 1 because the new declaration had no pin;
the initial pin attempt exited 2 while the load-path regeneration was still
in progress. Both are retained under `private/event-caller-removal/`.
The subsequent inventory sync and all three pin writes/checks exited 0.
The sandbox submodule attempt failed DNS resolution; host initialization
succeeded at the required Libevent revision. The standing card gate and
paired three-route attribution remain pending; no native coverage gain is
claimed from these focused controls.
