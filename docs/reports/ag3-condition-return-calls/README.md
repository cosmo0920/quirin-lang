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

## 2. Chosen shapes and why each is sound

Rule [63](../../agents/gateway.md#constraint-63), in
`crates/qcore-c-gateway/src/legacy_condition_leaf_reads.rs`, a child of
`legacy_c` reached through one hook per position (`if`, `while`, `for`,
`do`-`while`) in `parse_statement`, asked **after** `planned_condition` (rule
5h, then the elaborator 10s, then the `va_arg` route 12c) answered no plan.
Lean `QCore.ConditionLeafReads` (`proof/AuditConditionLeafReads.lean`, 16
theorems). Both readings require a condition with no brace, no opaque callee,
no tracked descriptor read as data, and direct calls only: none nested in
another's actuals, none through a member or a parenthesised expression;
`__builtin_expect(E, K)` with `K` one token of decimal digits is read through
as rule 5h reads it, any other hint refuses.

**(1) S1, a residual read beside identity calls.** Rule 5h lowers each call
of a leaf through the one application point (rule 10u) and then requires the
leaf's residual, every call replaced by `0`, to be custody-neutral as written;
a residual that reads memory (`d_len(s) > b->n`, `expected[i] !=
tolower(value[i])`) was refused although the same read alone is decided by
the typed condition read. Rule 63 decides it as the calls, then that read,
when:

- the condition is one leaf (no top-level `&&`, `||`, `?:`), with no
  assignment, compound assignment, increment or decrement, and no hint;
- each call is instantiated with use `Tested` (as rule 5h does) and **every**
  instruction they lower to is the identity on the abstract state: an
  identity contract call (every position borrowed, no container effect, a
  `None` or derived result, no produced site, no aggregate effect) or an
  actual read with no destination and no frame;
- the residual is not custody-neutral as written, and `condition_access` (the
  lexical typed condition read) answers a read with no destination and no
  frame.

*Soundness.* C leaves the order of a leaf's call and its operand reads
unspecified (C11 6.5p2-3). Identity calls change no map, so every order
reaches the same abstract state and every obligation is checked against that
same state (`every_order_reaches_the_same_state`,
`every_order_checks_the_same_obligations`,
`the_read_sees_the_same_state_in_either_order`): choosing "calls, then the
read" chooses nothing. The read keeps every dereferenced base's flow
obligation (unbound, a non-null untracked binding, or one known, non-null,
live identity; a released base is a `use_after_release` candidate). It is the
condition's branch instruction and refines neither edge
(`the_read_refines_no_edge`): the residual's `0` is not the call's value, so
no path is pruned on it. A call that is not the identity (`k_close(fd) >
b->n`), a release sequenced inside the leaf (`(k_free(b), d_len(s)) > b->n`),
an assignment, a connective and a switch keep the refusal (fixtures).

**(2) S2, the named refusal of an uncontracted call.** Otherwise, when the
condition's direct calls are, in textual order, calls the application point
instantiates followed by one the classifier refuses with exactly
``call to uncontracted function `F` `` for its own callee `F`, and `F` names a
function (no formal, no declared local, no `( * F )` declarator in the unit:
a call through a pointer object is an open set, rule 11p), the position's
refusal node becomes one plan node holding `Unknown` with the text
``call to uncontracted function `F` in branch condition`` (or `loop`,
`for-loop`, `do-while`), the condition's writes and both exits. Every probe's
state is restored. It is rule 12k's re-attribution for controlling
expressions: never an admission (`the_attribution_never_admits`,
`the_attribution_keeps_the_verdict`, `the_attribution_keeps_writes_and_edges`).
A `switch`, a held call, any other classifier text (an indirect actual's
`indirect memory access …` stays the position's text: AG1's subject), a
recursive-component or custody refusal and a condition reading a tracked
descriptor keep the position's text.

*The suffix is load-bearing.* The refused-operand hand-on rule
(`legacy_refused_operands.rs`, `refuses_a_condition`) reads whether a refusal
node is a controlling expression's from its **text**. A first implementation
that used the bare classifier text withdrew a leak accusation on a one-line
fixture (`void *p = k_alloc(4); if (ext(a) == 2) return 1; k_free(p);`: base
`Unsafe` with `leak`, bare text `Unknown`), because the node was then read as a
statement refusal and handed on its line's statement table. With the suffix
the node keeps its class (`the_named_text_keeps_the_condition_class`) and the
fixture keeps the leak beside the named refusal, exactly as beside the
generic one.

*Where the rule is asked, and why.* The first implementation put both
readings inside `legacy_condition_desugar.rs` (`leaf_plan`'s residual
decision and `planned_condition`'s last route). Two findings moved it out:
(a) that file is a pinned artifact of the libc reading-site evidence
(`docs/reports/libc-reading-site-evidence/bindings.json`, `scalar-reader`,
SHA-256 of the whole file), so any edit failed `reading_site_domain` and
`libc_reading_site_evidence` (gate attempt 2); (b) the value-connective route
(rule 10z, `legacy_value_connective.rs`) also asks `planned_condition` and
reads any plan as a decided condition, so a named refusal changed its answers
on the first corpus measurement (`rd_kafka_path_is_dir` and
`rd_kafka_racks_mismatch` exchanged their return refusal;
`rd_kafka_consumer_close` lowered the arms after the refusal, two
`unknown_alias`). Both files are back to the base bytes; the rule is asked
only at the four statement positions, after every older route, so it can
change no older admission. Its residual read is therefore one leaf only: a
connective's leaves stay rule 5h's.

## 3. Refused shapes and their named reasons

| Shape (census) | AG3-only rows | Kept refused because | Follow-up |
|---|---:|---|---|
| uncontracted callee in a condition (S2) | most of the 37 branch-call rows | the callee has no contract in force: fail-closed; now named per callee | `W44-CONDITION-CALL-UNCONTRACTED` (amended) |
| pointer-valued return read, member, chain, subscript | 15 | load vs interior address; a derived result must keep its base (W45) | `W45-RETURN-POINTER-RESULT-READS` (amended) |
| returned address of a member (`&p->f`) | 6 | an interior address escapes derived from `p`; needs a `Derived` (`Interior`/`Borrowed`) result through the derived-summary route, not `None` | `AG3-RETURN-MEMBER-ADDRESS-DERIVED` (new) |
| call through a member (`provider->init(…)`) | 2 + 1 | an open set (rule 11p) | none (by design) |
| function-pointer member test (`s->cb != NULL`) | 0 alone (5 with an indirect call) | the lexical typed model has no function-pointer field class | `AG3-FUNCTION-POINTER-MEMBER-TEST` (new) |
| `PB_RETURN_ERROR` comma store | 2 | the member store of a `?:` of the member and a literal (store rules) | `CG02-COMMA-OPERAND-RESIDUALS` |
| `errno` returned (`*__errno_location()`) | 1 | the scalar accessor route reads it only in conditions | `AG3-ERRNO-RETURN` (new) |
| `<ctype.h>` table read (`(*__ctype_b_loc())[c]`) | 3 | a table read through a call result: no route | `W44-CONDITION-CALL-REFUSED-SHAPES` |
| macros (`rd_assert`, `RD_LIST_FOREACH`, `RD_KAFKAP_STR_*`) | 7 | expand to the classes above (`RD_LIST_FOREACH` to `(elem = rd_list_elem(l, i))`, an uncontracted cross-unit callee) | S2 names them |
| an assignment beside a residual read | — | rule 10x's split owns assigned calls | none |
