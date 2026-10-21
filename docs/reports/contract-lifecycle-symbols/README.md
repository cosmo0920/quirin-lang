# CONTRACT-LIFECYCLE-SYMBOLS

Base `fb9e5f6e8cc4362e3d266ff201b6c469a85677d9`; [prediction](PREDICTION.md) committed before implementation. Adds symbolic `|^+` creation/introduction and `|^-` destruction/retirement through a separate `resource_lifecycle_v1` metadata clause. C04.18.4 and G05-lifecycle are the authoritative semantics and grammar. [Checked contract example](../../../crates/qcore-contract/examples/support/contract_lifecycle.qv).

```text
requires resource_lifecycle_v1(make_node, |^+ resource(node));
requires resource_lifecycle_v1(drop_node, |^- resource(node));
requires resource_lifecycle_v1(introduce_h, |^+ hypothesis(h));
requires resource_lifecycle_v1(retire_h, |^- hypothesis(h));
```

The typed source inspection API reruns existing property-specific resource/bounds loaders or explicit declared-model graph replay, retaining entry/operation/target/module/declaration/source SHA. Hypotheses must separately be declared graph atoms, with original reference and erasure checks intact. Resource IDs are literal symbols, not parameter names/C places; inheritance preserves them even when a formal has the same spelling. All entries validate, with duplicate-entry, source-version, typed-target, placement, reference and size/count refusals. Existing flow-v1 syntax is unchanged; lifecycle operators cannot nest inside it.

This stage performs no allocation/release, liveness or ownership judgment, hypothesis truth mutation, retirement/invalidation, negation or executable scheduling. No schema/provider/production contract changes. A symbolic create/retire pair is not a checked resource lifecycle. Detachment is not destruction. Textual bounds-contract identity changes when metadata is added; canonical semantic identity and interpreted fields remain unchanged, with both aspects checked separately. No fresh corpus audit, coverage gain or native/source-safety claim belongs to this card.

Four focused Rust tests cover resource/bounds/model source views, exact interpreted/graph erasure, malformed/unsupported forms and limits, and inheritance collisions. One frontend test pins atomic tokens beside all original flow/pipe symbols. Nine registered Lean theorems model typed metadata, duplicate/oversize/missing-reference refusal and semantic/disposition neutrality. These abstract theorems are not a mechanized Rust-parser correspondence or C-source proof.

Pre-gate focused attempt1 exited101 on a test's private-field accessor; attempt2 exited101 because the model fixture omitted its existing atom attachment; attempt3 exited101 because the bounds comparison incorrectly expected unchanged textual provenance. Corrected controls retain each refusal and explicitly check changed textual hash plus complete remaining-field equality. Lean-focused attempt1 exits0. All raw focused logs remain at `private/contract-lifecycle-symbols/`. First full standing attempt at `8e7449578` completed every stage: Rust101 and Clippy101 on the new frontend unit test using an external crate name instead of `crate::`; no full Rust count is claimed for that build failure. Submodules/Clang/fmt/both reevaluation examples/358 Lean audits passed0. The test namespace and an unnecessary clone of the Copy view enum are corrected. Successful full standing gate and final committed-matrix Python remain pending.

## Follow-ons

The existing `PREMISE-FLOW-PRODUCTION-DISCHARGE` entry remains open and is amended to include checked lifecycle lowering: fresh allocation identity, source-bound create/release events, ownership/alias effects, lifetime/linear-use rules and versioned hypothesis retirement with dependency invalidation. The readable guarded branch/postcondition layer remains proposed, and is not parsed by this card. Missing evidence stays unresolved; neither retirement nor failed proof application automatically establishes negation or Unsafe.
