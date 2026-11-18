# AG3-CONDITION-RETURN-CALLS: prediction

Frozen before the rule-63 commit and before any measurement of an
implementation build. The integrator measures with base `rev:a9c6dcf44`. The
forecast reads the census of the user's `current` run ([README](README.md)
§1) and unit probes of the rule on the private probe harness; the base corpus
run of this card had not finished when this file was committed.

## Mechanism and what it can move

* **Rule 63 (1), the residual read beside identity calls**, decides a
  call-bearing condition leaf whose residual reads memory. A row moves only
  when every remaining leaf of it is such a leaf. In `current` the lexical
  class holds 177 condition sites over 129 rows, 5 of them AG3-only; of those
  five, four keep a leaf the rule does not read (`buffer_skip_whitespace`:
  the `can_access_at_index` macro is call-free pointer arithmetic;
  `rd_kafka_init_wait`: `cnd_timedwait_abs` with address actuals;
  `rd_kafka_share_find_ack_batch_by_id`: a nested call;
  `rd_kafka_message_leader_epoch`: a macro mixture). The fifth,
  Monkey's `header_cmp` (`if (expected[i] != tolower(value[i]))`, the only
  leaf left, `tolower` a borrowed `None` libc declaration), is `Safe` on the
  probe harness with the rule and refused without it.
* **Rule 63 (2), the named refusal**, re-attributes a refusal and moves no
  status by construction (`QCore.ConditionLeafReads.the_attribution_keeps_the_verdict`;
  the position suffix keeps the refused-operand hand-on rule's reading, so
  no leak is withdrawn or added).

## Predicted moves

| Route | Function | Today | Predicted |
|---|---|---|---|
| corpus (eight-home) | `header_cmp` (Monkey `mk_http_parser.c`) | Unknown | Safe |
| twelve-home | `header_cmp` | Unknown | Safe (twelve-home reads the corpus inputs) |
| system, slice | — | — | no move |
| `current --monkey-linked` | `header_cmp` | Unknown | Safe |

No `Safe -> non-Safe` move and no new `Unsafe` row on any route. A further
corpus move of the same class (a function whose only remaining leaves are
residual reads beside identity calls) is possible but not forecast by name.

## Predicted leaf changes

* **Rule 63 (1)**: removes condition leaves (branch, loop, for-loop,
  do-while) where the leaf's calls are identities and the residual's read is
  decided: forecast 20 to 120 removed leaves on the corpus, each possibly
  exchanged for an `unknown_alias` at a read of a bound base (the read's flow
  obligation, a refusal, never an accusation) or for a later leaf's own
  refusal on the same line.
* **Rule 63 (2)**: exchanges, one for one on the same node, the generic
  condition text for ``call to uncontracted function `F` in … condition``.
  Upper bound on `current`: 437 condition sites over 285 rows whose first
  direct call names a callee refused elsewhere as uncontracted (top:
  `curl_easy_setopt` 50, `rd_kafka_conf_is_modified` 48, `rd_list_find` 17,
  `rd_kafka_topic_partition_list_find` 17, `rd_list_elem` 13); nanopb's
  `pb_readbyte`-style local callees add to it. Forecast on the corpus: 300 to
  900 exchanged leaves, the generic condition families falling by the same
  number, and zero change of `leak`, `double_release`, `use_after_release` or
  `summary_conflict` counts.
* On `current`, the AG3-only rows whose only condition refusal is an
  uncontracted call (at least the 17 lexical ones) change their sole reason
  to that callee, which the bottom-up plan reads as "waiting on one callee".

```measure-prediction
route corpus
move header_cmp Unknown -> Safe
delta Safe +1
route system
none
route slice
none
```
