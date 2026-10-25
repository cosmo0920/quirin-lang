# Prediction: checked initializer child-slot frames

Semantic BASE `47fc52a32abe8b5e56d711ff4773343ac6537167`.
Fresh corpus and bottom-up planner both exited 0, with stable input, binary and
planner identities, before this prediction. The corpus has 4,834 rows:
1,013 Safe / 3,812 Unknown / 9 provisional Unsafe. The planner has 2,189
candidate bodies, baseline same/cross closure 6/8 and conditional ceiling 384.
The getter's conditional 11-caller ceiling is planning evidence, not a gain.
See the [receipt](baseline-receipt.json) and [complete planner](base-plan.json.gz).
No semantic Rust, Lean, contract or tracked-tool changes precede this commit.

The bounded feature in [DESIGN.md](DESIGN.md) will preserve existing child pointer
slots proved separate from the initializer writes by source-bound Clang layout
and the existing core footprint checker. It will preserve the complete resource
ledger, field links and partial links. Other possible member facts are invalidated
conservatively; different allocation IDs supply no disjointness or freshness
premise. The original default-frame route and exact old fallback remain.

The first source subset closes pointer assignments against historical retyping:
same-type plain copies, direct contracted Owned results or known NULL, with
ordinary typed member destinations. Pointer-valued slot addresses and declared
Out/Stored/Removed parameter effects decline this optional proof. Current owner
identity aliases need compatible source evidence; the older aggregate-store
alias refusal remains. Declined probes restore reading/consultation state.

Ex ante forecast:

- Complete generic callers with disjoint child slots may become custody Safe.
  Positive root/embedded source and flow controls must reach the new arm.
  Aliased source stores keep the older refusal; alias preservation is exercised
  separately in explicit flow states.
- Predicted production Safe gain is **0** on corpus, system and slice. The six
  selected getter-chain functions are expected to remain Unknown. Their fresh
  own leaves are 6 (`metric_prepare_storage`), 4 (`metric_index_add`), 14
  (`map_metric_create`), 20 (`map_metric_get_unlocked`), 21 (`cmt_map_create`)
  and 1 (`cmt_map_metric_get`). Loaded-pointer assignments and other residuals
  remain outside the source/frame subset. Any actual movement requires complete
  attribution, including movements this forecast did not predict.
- No prior Safe result should regress. Missing/ambiguous layout, overlap,
  retyping, unsupported custody frames, non-Heap or summary identities,
  nullable/mixed/dead children, malformed partial links, caller custody and out
  slots refuse. A shallow parent release must not erase a child leak; partial
  escape and later lifetime violations must remain refused or diagnosed.
- No ordinary/callback/exported summary or interpreted resource-v1 initializer
  permission is added. Matching audited Lean and the canonical CFL `.qv` model
  will describe this exact frame boundary. Clause/inventory pins change
  deliberately; existing map/opts/setter model adoption stays Open.
- Independent NULL semantic results are expected unchanged. Accepted source
  proofs may add real layout/contract consultations and CFG metadata; complete
  resource, NULL, dependency, reading and runtime records must be retained and
  differences attributed, without arbitrary normalization.

System and slice BASE measurements continue against the frozen BASE binary and
314-file snapshot. They cannot observe later implementation edits. Paired TIP
measurements use the same snapshot; the changed canonical model is disclosed
and validated separately. This preserves corpus BASE/planner → this committed
prediction → semantic implementation ordering.

Acceptance requires matching Rust, registered placeholder-free Lean and `.qv`
semantics; actual source, flow, deep-reader, overlap, alias/rebinding, partial,
leak/lifetime, guarded-interface, declined-probe and certificate/export controls;
the full standing/provider gates and all five packages; complete paired native
attribution; and the prescribed increment/status/tracking amendment. Neither
the broader disjoint-frame follow-up nor the CMetrics getter chain closes.
Clang extraction, C embedding/bounds/no-undefined-behaviour and Rust/model
correspondence remain declared trust boundaries, not a C refinement proof.
