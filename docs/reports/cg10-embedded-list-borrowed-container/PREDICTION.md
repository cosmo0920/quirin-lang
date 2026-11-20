# CG10-EMBEDDED-LIST-BORROWED-CONTAINER: prediction

Committed before the implementation commit. Base `0111ddf38` (card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3's tip, which merged main
`b7f86a0de`). USER DECISION (2026-11-20, on round 3's decision class 1,
question 1, verbatim through the integrator's brief): "Read embedded lists as
borrowed container operations".

**Disclosure.** Before this file was committed, a synthetic probe
(`crates/qcore-c-gateway/tests/embedded_list_borrowed_container.rs`, then a
print-only test) was run on the base: every fixture call handing
`&c->list`, `&c->in.on_new` or `&k->conf.in.on_new` to a registry or
retaining helper keeps the gate's refusal, and a helper whose every position
is `Borrowed` with no container clause (`el_count(&c->in.on_new)`) is
already `Safe`. The rule's module was drafted but neither compiled nor run
over Fluent Bit. The predictions below come from round 3's census and the
base census streams.

## 1. Census on the base (method step 1)

Route: the opt-in `quirin-audit current --monkey-linked` over Fluent Bit
046accf6, round 3's measured tip run (`run-tip2`, binary `cdcbf730…` of
`1379d031a`; the later round-3 commits are a lint fix and fixture updates, no
semantic change, so it is this card's base):
**1,140 Safe / 2 Pending / 3,655 Unknown / 7 Unsafe**; 2,190 Unknown rows
carry the access refusal, **240** nothing else, 9,149 access leaves.

The 23 rows round 3's `refine.py` classes as `embedded-list-address` alone
(`census/rows.md` will list each leaf), with what each helper does per its
contract in force and what the base census stream (`call-arguments.jsonl`,
rule W44's own reading of each actual, base binary) says decides the
statement once the access is read:

| row(s) | helper and embedded list | contract (provider) | W44's classification with the actual read |
|---|---|---|---|
| `rd_kafka_ConfigResource_add_ConfigEntry` | `rd_list_add(&config->config, entry)` | `resource_borrowed(rl)`, `resource_element_added(elem, rl)`: custody of `entry` passes only under a releasing registration in force (rules 13w, 13x) | `the registry holder `& config - > config` is not one this body names (callback-registry-undecided)` |
| `rd_kafka_brokers_wait_state_change_async` | `rd_list_add(&rk->rk_broker_state_change_waiters, eonce)` | as above | the same refusal for its holder |
| `rd_kafka_metadata_cache_wait_state_change_async` | `rd_list_add(&rk->rk_metadata_cache.rkmc_observers, eonce)` | as above | (not in the streams' unit set; same shape) |
| `rd_kafka_assignors_term` | `rd_list_destroy(&rk->rk_conf.partition_assignors)` | registry invoked over the elements, elements dropped, unregistered, `resource_released_if_constructed`: **releases the container's elements** | `callback-registry-undecided` |
| `rd_kafka_interceptors_destroy` (15 leaves) | `rd_list_destroy(&conf->interceptors.on_*)`, `.config` | as above | `callback-registry-undecided`, every leaf |
| `rd_kafka_mock_clear_requests`, `_start_request_tracking`, `_stop_request_tracking` | `rd_list_clear(&mcluster->request_list)` | registry invoked, elements dropped: **releases the elements** | `callback-registry-undecided` |
| 4 `rd_kafka_conf_interceptor_add_on_conf_*`/`_on_new`, 10 `rd_kafka_interceptor_add_on_*` | `return rd_kafka_interceptor_method_add(&conf->interceptors.on_X, ic_name, (void *)on_X, ic_opaque)` (10 through `&rk->rk_conf.interceptors.on_X`) | all `Borrowed`, `resource_retained_by(func, list)`, `resource_retained_by(ic_opaque, list)`: two **caller objects retained by the caller's container** (rule 12t) | a whole-call `return`: not in the call-argument stream |
| `pack_static_labels` | `mpack_start_array(writer, cfl_list_size(&cmt->static_labels->list))` | `cfl_list_size`: `Borrowed`, `None` result (no container clause) | the inner call is a **nested call** inside the outer actual: refused `call` |

## 2. The reading (method step 3)

Rule 69: where the gate refused a statement (or a whole-call `return`) whose
first call has an actual `& R -> M …` / `& R . M …` with `R` a formal, and
the statement classified again with its actuals read (rule W44's reading) is
refused by a **container custody** refusal of that call about that actual
(`callback-registry-undecided`, `callback-element-custody-undecided`,
`retention-undecided`), the gate's node keeps its writes and takes that
description. Never an admission: a releasing helper is not a no-op, an added
element does not pass to a container whose registration the body does not
hold, a retention in the caller's container stays undecided. Rule 46
(element destroyers' unlink member) is neither read nor amended.

## 3. Predicted moves (opt-in `current`)

* **No verdict moves**: no Unknown -> Safe, no Safe -> non-Safe, no new
  Unsafe, no Pending change. The rule changes only the description of a node
  that stays `Unknown` with the same writes.
* Of the 23 rows: the **8** `rd_list_add`/`rd_list_destroy`/`rd_list_clear`
  rows are re-attributed to `callback-registry-undecided` (every leaf); the
  **14** interceptor rows are re-attributed to `retention-undecided` (range
  0-14: the whole-call return form and the `(void *)on_X` actual are not in
  the base streams); `pack_static_labels` keeps the access refusal (the
  container actual is an actual of a nested call, not of the first call;
  AG1-TWO-BORROWED-CALLS' nested half).
* Access-alone rows **240 -> 218** (range 218-232). Access leaves removed:
  the 23 rows' 37 leaves at most, plus leaves of the same shapes on rows with
  other blockers (predicted 10-80 more, every such row staying `Unknown`).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

Pinned routes: `quirin-audit measure --declared` cannot run on this host (no
`build/cycle19` inputs or baselines). For the integrator: no verdict move on
corpus, twelve-home, system or slice; leaf descriptions change on corpus rows
of librdkafka that hand a formal's embedded `rd_list_t` to `rd_list_*` or
`rd_kafka_interceptor_method_add`.
