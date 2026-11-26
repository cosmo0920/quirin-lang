# CALLBACK-TABLE-MEMBER-PATH-STATES: prediction

Committed after the probe fixture (`3836b3a40`) and before the semantic
commit. Base `43410b230` (main). USER DECISION 2026-11-25: "lanich four of
queued cards", with CALLBACK-TABLE-MEMBER-PATH chosen as the fourth.

## What was known when this was written

* `legacy_callback_route::callback_operands` collects the candidates'
  member-path states (`member_path_route::operands`) for a synchronous
  invocation (`CallbackClause::Invoked`) and a member invocation, but not for
  a table dispatch (`CallbackClause::Table`). `classified_in` reads a
  candidate stating member-path states as its custody contract, so at a table
  dispatch its states are dropped.
* The fixture `crates/qcore-c-gateway/tests/callback_table_member_path_states.rs`
  ([`evidence/probe-base.log`](evidence/probe-base.log), exit 101, 2 passed
  and 4 failed) shows it at base. A table candidate releasing the linked
  payload, followed by a free of the payload, reads **`Safe`**; the same body
  through the synchronous form reads `Unsafe`. A stale candidate reads `Safe`
  in the table form; the synchronous form refuses it (`member-path-stale`).
  A table candidate releasing the payload with no later free reads `Unsafe`
  (the item leaks); the synchronous form reads `Safe`.
* The shipped table clauses are exactly `cmt_mpack_unpack_map` and
  `ctr_mpack_unpack_map`. Neither provider states a member path, and their
  handlers are static functions of the decoding units, read from in-unit
  summaries, which state none.

## Planned change

At a table dispatch, compose the candidates' member-path states exactly as
the synchronous branch does (the same `member_path_route::operands` call,
joined over the candidates by `member_path::join`, which refuses candidates
that disagree). Rules 13v and 30 are amended; the Lean model
`QCore.CallbackReleasedEffectOrder`'s family gains the table composition.

## Predicted movements

* **Opt-in `current --monkey-linked`** over Fluent Bit 046accf6, base
  `43410b230` against the tip: **0 status moves and 0 content changes**, with
  equal totals on both sides. No shipped table candidate states a member
  path, so `member_path_route::operands` returns nothing at either shipped
  table dispatch.
* Controls: the rows naming `cmt_mpack_unpack_map` or `ctr_mpack_unpack_map`
  are unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); none is predicted on any of the
three. The riskiest move would be a cmetrics or ctraces decoder row changing,
which would mean a handler of a shipped table is governed by a member-path
declaration after all.
