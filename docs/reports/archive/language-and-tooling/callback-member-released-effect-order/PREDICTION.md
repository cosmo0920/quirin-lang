# CALLBACK-MEMBER-RELEASED-EFFECT-ORDER: prediction

Committed before the implementation commit (Lean module, rule, report). Base
`3ca85c7a6` (main, r65). USER DECISION 2026-11-24: "launch them all".

## What was known when this was written

Read from the base:

* The transfer (`legacy_transfer.rs`) applies the call's custody contract,
  release included (`apply_contract_call` → `resource_action`), then
  `retained_by_route::apply`, then `callback_route::apply`: a candidate's
  member-path states (`member_paths`), a member reading deferred to the
  constructed state (`member_uses`, which reads `state.constructed` and then
  the same composition), the helper destroyer's own states
  (`direct_member_paths`), the arms and the registry uses.
* The release of a site `s` writes only `s`'s status, the statuses and
  `contained` entries of what `s` contains (`cascade_resource_action`), and the
  retentions `s` holds (`holder_released`). It never writes `field_links`
  (premise P-B08-SHALLOW), `constructed`, `registries` or `pointers`.
* `legacy_member_path_route::apply` releases a linked child only while it is
  live (`if single && live`) and refuses (`member-path-undecided`) otherwise.
  So composing after the release is the C order (callback, then free) except
  where a linked child is `s` or something `s` contains, and there the late
  order reads it released and refuses.
* The fixture `crates/qcore-c-gateway/tests/callback_member_released_effect_order.rs`
  (6 tests, `private/logs/fixture-4.log`, exit 0 at base) shows it: the plain
  member release, the arms form and the synchronous form compose the
  candidate's `#released` payload (`Safe`) and leave a kept payload leaked
  (`Unsafe`); a payload the op also holds in custody refuses
  `member-path-undecided` in all three forms; `o->self = o` is refused at the
  store (indirect access). The mutation that drops the liveness read
  (`evidence/mutation/`, harness `scripts/mutation.py`) turns the contained row
  `Safe` (a double free certified); restored, the fixture passes.

## Finding and planned change

The plain member form is already fail-closed: no refusal is added and no
gateway source changes. The card adds the ordering proof (new Lean module
`QCore.CallbackReleasedEffectOrder` with its audit), the fixture as the pin,
rule 74p, and the report. r60's slot refusal is kept as it stands.

## Predicted movements

No analysis source changes, so nothing can move.

* **Opt-in `current --monkey-linked`** over Fluent Bit 046accf6, base
  `3ca85c7a6` vs tip: **0 status moves, 0 content changes**; totals equal
  (r65's tip figures, 1172 / 2 / 3622 / 8, expected on both sides). Rows
  differ at most in identity digests (the revision in `evidence_version`).
* Controls: `rd_kafka_op_destroy` and `mk_thread_destroy` rows unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); predicted none on all three. The
riskiest move would be any row at all, which would mean the tip's binary
differs in behaviour from the base's although no analysis source changed.
