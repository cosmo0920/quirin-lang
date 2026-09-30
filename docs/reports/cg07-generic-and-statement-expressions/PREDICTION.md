# CG07: frozen prediction (re-frozen from CG00's, before any AFTER run)

Frozen on base `c374de96` after the BEFORE arbiter reproduced the baseline
(608 / 10 / 4,216, `resource-diagnostics.json` `3a25c4e9…`) and before any
AFTER measurement. CG00's [prediction](../cg00-clang-gap-baseline/predictions/CG07.md)
(corpus `ae17152b…`) is the starting point; its figures are upper bounds from a
static attribution that reads a whole line's constructs, not the statements a
lowering would expose. Each amendment below gives its reason, read from the
preprocessed source of the function's blocking line.

## Route and design, as frozen

- **E19, route G.** A statement expression is lowered only in a discarded,
  initializer, assigned-to-a-named-object or returned position (the enclosing
  full expression puts nothing unsequenced beside it), under a name-hygiene,
  storage and (where the value is used) escape premise; its inner statements
  are parsed from the stream by the ordinary statement walker, so every inner
  statement meets exactly the rules it meets in an ordinary block. An inner
  refusal keeps its own reason.
- **E17, route G.** Clang's selected association is a fallback after the
  selection-invariance route (W44). Every corpus `_Generic` is glibc's
  selection-invariant `__glibc_const_generic` shape, which route 1 already
  lowers, so route 2 is expected to lower **no corpus selection**.

## Amended function list (CG00's 18 "freed alone")

| Function | CG00 | Amended | Reason |
|---|---|---|---|
| `rd_kafka_log_syslog`, `rd_kafka_cgrp_clear_wait_resp`, `rd_kafka_cgrp_set_wait_resp`, `rd_kafka_confval_get_int`, `validate_subscription`, `rd_kafka_oidc_assertion_get_algo_label`, `rd_list_set_cnt`, `rd_list_set_int32`, `rwlock_rdlock`, `rwlock_rdunlock`, `rwlock_wrlock`, `rwlock_wrunlock`, `isFullyBalanced0` (13) | Safe | **Unknown** | Each blocking statement expression is glibc 2.43's `assert` (`((void) sizeof (__assert_single_arg (E)), __extension__ ({ if (E) ; else __assert_fail (…); }))`, `isFullyBalanced0`'s inside `RD_UT_ASSERT`). The block lowers, and its `else` arm is a call to `__assert_fail`, which no contract declares (it does not return: `W43-LIBC-NORETURN`, card CG21). The leaf moves from the braced-construct refusal to `call to uncontracted function __assert_fail`; the function stays Unknown. |
| `get_hash_table` | Unknown (alias) | **Unknown** | The same `assert` family, plus its alias leaf. |
| `rd_http_req_get_content_type` | Unknown (alias) | **Unknown** | The statement expression is libcurl's `curl_easy_getinfo` type check inside an `if` condition: a refused position. |
| `mk_handler_param_get` | Safe | **Unknown** | `return mk_list_entry (…)`: a `container_of` statement expression in a returned position; its inner `__typeof__` declaration and `offsetof` pointer arithmetic are refused by the older routes (E18, CG16), so the inner leaves remain. |
| `mk_mimetype_lookup` | Unknown (alias) | **Unknown** | `container_of` in an initializer, as above, plus its alias leaf. |
| `rd_kafka_conf_validate_single` | Safe (E17) | **Unknown** | Its `_Generic`s are selection-invariant and already lowered by route 1; the blocking refusal is the lowered `return !strchr (…) && !strchr (…)`, whose tokens no longer match its Clang node, so the value-connective route (CG06) cannot read it. Route 2 does not change that (a correction to the input: no corpus E17 leaf is a refusal of the selection itself). |

## Predicted verdicts

- **Safe: 0** from CG00's list (upper bound 18 → 0), and **at most 3**
  elsewhere: a function outside the list becomes Safe only if every
  statement expression blocking it is admitted and every inner statement is
  one the older routes already decide (not expected for the `assert`,
  `container_of` or `typeof`-temporary families).
- **Unknown → Unsafe: 0.** A new leak or violation may appear only where an
  inner statement newly reads an acquisition the braced refusal used to hide;
  each would be classified by reading.
- **Safe → non-Safe: 0.**
- **Leaves.** The braced-construct refusal (`braced initializer, expression,
  or declaration is not modeled`, 2,000 leaves at BEFORE, initializer lists
  included) falls by the admitted statement expressions; `call to
  uncontracted function __assert_fail` and the inner families' own refusals
  (pointer arithmetic, `typeof` declarations, indirect access) rise.
- **E17: 0 moves** in the corpus.
