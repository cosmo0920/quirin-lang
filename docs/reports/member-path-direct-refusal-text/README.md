# MEMBER-PATH-DIRECT-REFUSAL-TEXT

Base main `0f6e17118`. Ledger entry `MEMBER-PATH-DIRECT-REFUSAL-TEXT`
(recorded by SCHEDULER-DESTROY-MEMBER-STATES).

## Prediction

Committed before any code change; nothing had been built or measured from
this card.

The card changes only the text of the `member-path-direct` refusal of a
direct call (`callback_operands` in
`crates/qcore-c-gateway/src/legacy_callback_route.rs`) so that it names the
clause that makes the declaration non-plain. When the refusal fires, its code
and every verdict stay as they are.

- No status moves on any route (system, slice, corpus).
- Corpus: exactly one diagnostic's text changes, in `mk_http_thread_destroy`'s
  own row (`mk_server/mk_http_thread.c`, origin `mk_http_thread.c:269`, call
  to `mk_thread_destroy`). Its text changes from "beside a hand-off, which only
  a callback invocation composes" to text that names the member invocations
  of the formal. The code `member-path-direct`, the category `unsupported`,
  the status (Unknown) and every other diagnostic in that row stay the same.
  Hash-like fields that are derived from the binary (`build_identity`) may
  differ by rebuild.
- System and slice: no change. Neither has a Monkey scheduler unit, and no
  other declaration in force reaches this refusal: every other member-path
  declaration in force is plain or stands beside a hand-off, and the text for
  a hand-off is unchanged.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
