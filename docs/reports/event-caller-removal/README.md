# Event caller removal contract

The canonical Monkey contract now states the registration removal performed by
`mk_event_timeout_disable`. Its complete body forwards directly to
`mk_event_del`. A caller can therefore end the exact unpoisoned registration
across this declared API boundary before releasing its object. The wrapper
adds no close call and does not destroy the loop; descriptor effects of the
delegated backend are outside this contract. The wrapper is a pinned declared
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
| Conditional host flag | Per-record correlation remains a declared assumption | Prove record.instance equals the hosting actual before using a conditional guard |
| Threaded teardown | Unproved ordering remains Unknown | Establish join and loop destruction before collector/object release on every feasible path |
| Event descriptor | No new descriptor claim | Member-held/out-slot descriptor identities and their lifetime model |

These residuals remain tracked under
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`,
`RETENTION-DERIVED-LOOP-PATH-EQUALITIES`,
`RETENTION-PATH-LOOP-DESTROY-ADMISSION` and `MK-EVENT-DESCRIPTOR-CUSTODY`.
This increment does not close the complete Fluent Bit collector lifetime chain.

## Measured caller increment

The paired arbiter compares immutable merged-source BASE `b403cd4de` with
semantic TIP `58e80090d`, using `--declared MEASURE_DECLARATION.md`. It exits 0:
all three routes match the zero-movement forecast, with no verdict moves,
unmatched rows, composition conflicts or diagnostic-leaf changes. Corpus stays
1,013 Safe / 3,811 Unknown / 10 provisional Unsafe; system stays 101 / 343 / 4;
slice stays 57 Safe / 342 Unknown / 4 Pending / 1 provisional Unsafe.

The already-Safe `mk_event_timeout_disable` row adds its own declaration to
its dependency list, alongside `mk_event_del`. Its new declaration is added
to `contract_body_evidence.unvalidated`; no removal-clause body proof is gained.
All NULL property dictionaries, graphs, custody joins and other carrier fields
are unchanged across 4,921 corpus, 454 system and 406 slice rows after the
recorded context renewal. All three outer documents are identical.

The attribution checks every changed unit/function field. It allows only the
new exact removal entry and synthetic retained-call entry, the reviewed module
text, the recorded environment/link/configuration/context hashes, the new
unvalidated declaration and that single dependency addition. Compile commands,
imported source/home identities and other environment payloads are unchanged.
All 148 reading-site binding IDs are independently recomputed; receipt multisets
match after the recorded environment/context renewal, preserving every checked
predicate, source span, atom, graph and binding. Receipt ordering can change
with the renewed IDs. These are local checker observations, not new C execution
or universal source proofs.

[Native summary](evidence/native-summary.json),
[receipt and raw manifests](evidence/native-receipt.json),
[complete paired comparison](evidence/native-comparison.json.gz) and
[complete attribution](evidence/native-attribution.json.gz) retain this evidence.
The 31 BASE artifacts are bound at main records commit
`9348f813444a25ef710ad6096cb88469913176fa`; their exact paths/hashes, including
shared archives, are in the receipt. The 31 complete TIP route artifacts plus
run and slot receipts are archived here. The preliminary TIP audit compared
against published `d4c43b5ac` and therefore also saw the already-owned CMetrics
move; the final paired comparison above isolates this card.

The native version is exactly `58e80090d`, not the later source-validation tip.
Later changes repair deliberate census expectations and correct descriptor
wording; generated module hashes renew. The formal timeout-disable declaration
is byte-identical, as recorded in [source review](source-review.json). The
wrapper inherits `mk_event_del`'s declared removal assumption; no automatic
proof of backend failure/cleanup paths or descriptor lifecycle is claimed.

## Source validation

The scoped card gate selects full root Rust, Lean and provider validation on
`6398ddd7c9911c3b38bf651fb3394d414cd1fb53`. Rust: 8,834 passed / 0 failed / 15 ignored
(572 units / 591 summaries; 24 filtered). Provider Rust:
133 passed / 0 failed / 1 ignored (43 summaries). Lean: 389 audits.
Python: 554 tests / 0 skips. Every command exits 0, including both
reevaluation probes, root/provider formatting and Clippy, and build/verification
of all five provider packages. [Compact gate](gate.json) and
[exact executed commands, exits, counts and private log paths](evidence/gate-steps.json)
record the run; the Clang banner was captured in this worktree when this gate
started. Source pin checks at all three roots and the focused two Rust tests /
three Lean theorems also passed. Records-only completion is committed after
regenerating the matrix/index/contributions and checking the amendments; the
post-commit Python rerun is logged at `private/event-caller-removal/records-python.log`
with its separately retained exit.

The first full attempt is preserved in `evidence/gate-attempt-1.json`: Rust
8,833 passed / 1 failed / 15 ignored (572 units / 591 summaries). The failed
loader census deliberately needed 497 -> 498 interpreted declarations; a
focused rerun exposed the companion 106 -> 107 withheld-structured-effect
expectation, then passed after both corrections. Provider Rust was terminated
while still queued (exit -15, zero summaries) to avoid closing admission ahead
of main publication. That attempt is not a passing gate. Its other steps,
including 389 Lean audits, 554 Python tests and five package verifications,
exited 0. The clean final gate above is authoritative.

Earlier setup attempts remain recorded: sandbox submodule DNS failed before
host initialization succeeded; the first inventory sync exited 1 for the
missing new pin; an initial pin command exited 2 during load-path regeneration.
Subsequent inventory sync and all three pin writes/checks exited 0. The private
provenance-reader prototypes initially rejected optional environment maps,
the new unvalidated entry, renewed receipt ordering and NULL context metadata;
the completed strict reader exited 0, retaining the raw pairs and all actual
changes.

The report's [capability review](evidence/capabilities.json) pins the remaining
implementation refusals. The proposed [integration amendments](integration-amendments.json)
keep the existing caller-lifetime follow-up open. This card is ready for review
on its separate branch; it does not land or publish itself.
