# FRAME-HOLDER-SHADOWED-NAME: prediction

Committed before the implementation commit. Base `3ca85c7a6` (main, r65).
USER DECISION 2026-11-24, "launch them all". Discharges
`FRAME-HOLDER-SHADOWED-NAME`, registered by CG25-IN-PLACE-REGEX (r64).

## What was known when this was written

No implementation was measured. Known from reading the base and from the
pins `crates/qcore-c-gateway/tests/frame_holder_shadowed_name.rs` run at
base (`private/card-logs/pins-at-base.log`: 2 passed, 3 failed):

* **Census.** Two callers of `one_automatic_object` lack
  `every_reference_is`: rule 12t's retention frame holder
  (`legacy_retained_by.rs`, `holder`) and the callback registry's
  `frame_registry_holder` (`legacy_callback_route.rs`). The third caller,
  `Parser::automatic_object` (r64), already asks both. Other "automatic"
  readers (`aggregate_value::automatic_object`,
  `legacy_variadics::automatic_local`, `storage_duration::automatic_ids`)
  are keyed by declaration id, not by name, so shadowing cannot reach them.
* **Reachable, fail-open.** A file-scope `struct st g` that a block-scope
  `struct st g` shadows, named by the body as `&g` at a holder position,
  passes both token censuses (every mention is a `&g` holder actual or the
  declarator) and `one_automatic_object` (one `VarDecl`). Probes F1, F2
  (a caller's object retained by the file-scope `g`), F3 (a file-scope
  record dropped by the local's reset, then the object released) and F4
  (the file-scope registry destroyed under the local's registration) are
  `Safe` at base. F5 already refuses at base.

## Planned change

Both readers also require `every_reference_is` (`Parser::automatic_object`'s
second half). A name that is not one declaration is not a frame holder: a
retention there is `retention-undecided`, a registry there is
`callback-registry-undecided`. Only `Safe -> Unknown` is possible.

## Predicted movements

None measured. A Fluent Bit or Monkey frame holder that shadows a
file-scope object the same body names is not expected; if one exists it
moves `Safe -> Unknown`, never the other way. No `Unknown`, `Pending` or
`Unsafe` row becomes `Safe`; no new `Unsafe`.

Opt-in `current` (this card's arbiter): no row moves, totals identical.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as above: no move.
