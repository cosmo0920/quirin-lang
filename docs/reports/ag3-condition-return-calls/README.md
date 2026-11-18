# AG3-CONDITION-RETURN-CALLS: calls, aggregates and indirect access in conditions and returns

Card AG3 of round 33 (brief `private/integrator-briefs/r33-ag3-condition-return-calls.md`),
base `a9c6dcf44`. Gap: the refusal `unmodeled call, aggregate or indirect
access in {branch condition, loop condition, for-loop condition, do-while
condition, switch expression, return expression}`.

## 1. Sub-shape census

Source: the user's `current --monkey-linked` report of 2026-11-18 (Fluent Bit
`046accf6`, read-only, never written), streamed by
[`census/scripts/ag3_census.py`](census/scripts/ag3_census.py); each site's
text is read from the prepared source tree at its `origin` and classified
lexically by [`census/scripts/classify.py`](census/scripts/classify.py)
(all sites: [`census/ag3-sites.json.gz`](census/ag3-sites.json.gz)).

**Rows.** 1,370 function rows carry at least one AG3 leaf; **119** carry
nothing else (the integrator's "blocked by these alone" figure, 97, counts per
position; a row with a branch and a return leaf counts once here).

| Position | Rows mentioning | Rows with only AG3 leaves |
|---|---:|---:|
| branch condition | 1,134 | 64 |
| for-loop condition | 191 | 3 |
| return expression | 155 | 62 |
| loop condition | 82 | 8 |
| do-while condition | 18 | 3 |
| switch expression | 3 | 1 |

**By primary sub-shape** ([`census/shapes.txt`](census/shapes.txt); the
primary shape is the first of macro, call through a member, direct call,
builtin, assignment, increment, address-of, member chain, subscript, member,
dereference, connective that the expression shows; "only" counts rows whose
AG3 leaves all carry this one shape and nothing else blocks them, i.e. the
rows this shape alone would unblock):

| Position | Primary shape | Sites | Rows | AG3-only rows | Shape alone |
|---|---|---:|---:|---:|---:|
| branch | direct call | 2,379 | 884 | 49 | 37 |
| return | direct call (inside a value) | 38 | 38 | 22 | 19 |
| return | member chain (`a->b->c`, `a->b.c`) | 23 | 21 | 12 | 10 |
| loop | direct call | 77 | 62 | 7 | 6 |
| return | address of a member (`&p->f`, `&p->a[i]`) | 15 | 15 | 6 | 6 |
| return | single member (`p->f`) | 22 | 20 | 4 | 3 |
| return | macro (nanopb `PB_RETURN_ERROR`) | 70 | 35 | 8 | 2 |
| branch | macro (`rd_assert`, `RD_KAFKAP_STR_*`, `RD_MAP_GET`) | 576 | 226 | 5 | 2 |
| return | call through a member (`provider->init(…)`) | 12 | 11 | 5 | 2 |
| return | subscript (`rl->rl_elems[i]`) | 9 | 7 | 5 | 2 |
| branch | call through a member (`stream->callback(…)`) | 16 | 16 | 4 | 1 |
| branch | member / member chain | 257 | 160 | 6 | 0 |
| for-loop | macro (`RD_LIST_FOREACH`, `TAILQ_FOREACH`) | 216 | 168 | 3 | 0 |
| do-while | (condition at loop end) | 19 | 18 | 3 | 0 |
| other | 22 smaller classes | 59 | — | 6 | 6 |

**Named lexical sub-shapes** ([`census/named-shapes.txt`](census/named-shapes.txt)),
sites / rows / AG3-only rows: call through a member 28 / 26 / 9;
`PB_RETURN_ERROR` 67 / 32 / 7; returned address of a member 15 / 15 / 6;
`<ctype.h>` classification 14 / 9 / 3; `rd_assert` 58 / 50 / 3;
`RD_LIST_FOREACH` 147 / 115 / 3; `likely`/`unlikely` 95 / 76 / 3;
`strcmp` family 59 / 34 / 2; `errno` 1 / 1 / 1; `__builtin_ctz`/`clz` 4 / 1 / 1.

**Candidates measured for this card** ([`census/candidates.txt`](census/candidates.txt)):

- **S1, a call-bearing leaf whose residual reads memory** (`d_len(s) > b->n`,
  `f(x) == p->len`): 177 sites, 129 rows, 5 AG3-only rows
  (`buffer_skip_whitespace`, `header_cmp`, `rd_kafka_init_wait`,
  `rd_kafka_message_leader_epoch`, `rd_kafka_share_find_ack_batch_by_id`).
- **S2, a condition refused at an uncontracted call**: the first direct call
  of 437 condition sites (285 rows, 17 AG3-only rows) names a callee the same
  report refuses elsewhere as `call to uncontracted function`
  ([`census/scripts/attrib_forecast.py`](census/scripts/attrib_forecast.py),
  an upper bound for those callees; callees refused only inside conditions,
  such as nanopb's `pb_readbyte`, are not counted by it). Top callees:
  `curl_easy_setopt` 50, `rd_kafka_conf_is_modified` 48, `rd_list_find` 17,
  `rd_kafka_topic_partition_list_find` 17, `rd_list_elem` 13.
- **S4, a function-pointer member truth / NULL test** (`stream->callback !=
  NULL`, `provider && provider->init`): 58 sites, 35 rows, 5 AG3-only rows,
  every one of which also calls through that member (an open set, rule 11p).

**Reading of the AG3-only rows** ([`census/sole-sites.txt`](census/sole-sites.txt),
every site with its source line). Of the 37 rows the branch "direct call"
shape alone blocks, the call is, in almost every case, a callee with no
contract in force (a body in the same unit that is itself Unknown, such as
nanopb's `pb_readbyte`/`pb_decode_varint`, `rd_kafka_conf_is_modified`,
`flb_routes_mask_get_bit`; a body in another unit; or a libc function no
provider declares, such as `seteuid`, `setegid`, `strptime`, `getentropy`),
a call with an indirect actual (AG1's subject), or a `<ctype.h>` macro
(`(*__ctype_b_loc())[c]`). Clearing the position refusal for these would
exchange it for `call to uncontracted function`, as RETURN-POSITION-SUBSHAPE-B
measured for whole-call returns, and decide nothing.

### 1.1 What earlier cards refused on purpose

Nothing below is re-admitted by this card.

- **Pointer-valued return reads** (`return rk->rk_name;`, `return
  &rk->rk_conf;`): W45-RETURN-EXPRESSION-FAMILIES admitted only scalar-result
  reads, because a pointer read is a load or an interior address (array
  decay, `&B->F`) and an interior address must keep its derivation from `B`;
  rule 10n (1) "an address is never returned (rule 5j)". Open as
  `W45-RETURN-POINTER-RESULT-READS`. The 21 AG3-only return rows of shapes
  member chain, single member, subscript and address-of are this class.
- **Uncontracted callees** stay refused by design (fail-closed;
  `W44-CONDITION-CALL-UNCONTRACTED`, `CG03-RETURN-OPERAND-RESIDUALS`).
- **Calls through a member or expression** are open sets (rule 11p,
  CG22-INDIRECT-CALLS-AND-CALLBACKS).
- **The comma macro `PB_RETURN_ERROR`**: rule 10s splits it; its member store
  operand (`stream->errmsg = stream->errmsg ? stream->errmsg : "msg"`) is
  refused by the store rules (`CG02-COMMA-OPERAND-RESIDUALS`).
- **A call or assignment in a return** under rule 10n (4): a scope statement
  of W47, not a soundness refusal; rules 10u and 10y already read scalar
  returns holding calls (probes: `return d_len(s) == 0;`, `return
  !d_chr(s, ',') && !d_chr(s, ' ');` and `return d_len(s) > b->n;` are
  `Safe` on the base).
- **Two effectful calls in one leaf, custody results, recursive-component
  callees** (rule 5h): unchanged.

### 1.2 Unit reproduction of the candidates on the base

A private probe (the gateway on one function under the generic `d_*`/`k_*`
fixture contracts) on base `a9c6dcf44`:

| Probe | Base |
|---|---|
| `if (d_len(s) > b->n)` | refused: branch condition |
| `while (d_len(s) > b->n)` | refused: loop condition |
| `if (ext(a) == 2)`, `ext` undeclared | refused: branch condition (generic text) |
| `if (b->cb != (void *)0)`, `if (b && b->cb)` | refused: branch condition |
| `return &b->n;` | refused: return expression |
| `return (*__errno_location ());` | refused: return expression |
| `return (s->errmsg = s->errmsg ? s->errmsg : "x"), 0;` | refused: return expression |
| `return (long) d_len(s) - b->n;` | refused: return expression |
| `return d_len(s) > b->n;` | Safe |
