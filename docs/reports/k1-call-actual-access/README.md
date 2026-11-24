# K1-CALL-ACTUAL-ACCESS: call-actual access reads, re-censused; rule 75

Card K1 of round 34 (brief `private/integrator-briefs/r34-k-common.md`), base
`3ca85c7a6`. Bundle: an expressible typed access path inside a call actual
whose call the typed layer still refuses (census tags `expr:statement:call-arg`,
`expr:cond:call-arg`, `expr:return:call-arg`); owners
W44-CALL-ARGUMENT-UNTYPED-ACTUAL (primary), W44-CALL-ARGUMENT-NON-STATEMENT,
W44-CALL-ARGUMENT-UNCONTRACTED; rules 10k, 10n, 10u.

## 1. Re-census (11-07 vs 11-24)

Source: the user's `current` report written 2026-11-24 09:58 (Fluent Bit
`046accf6`, 1,173 Safe / 3,750 Unknown / 11 Unsafe rows; read-only), joined to
CURRENT-ACCESS-ALIAS-CENSUS's 11-07 leaf tags by (file, function, `-E -P`
line, family, sub) ([scripts/recensus.py](scripts/recensus.py),
[census/recensus.json](census/recensus.json)). 11,541 of 12,259 IND/COND/RET
leaf groups and 9,420 of 10,108 alias leaves join; an unjoined group carries
`new` and is never counted removable (so "freed alone" is a lower bound by up
to the 718 unjoined groups, and an upper bound in every other respect).

| Measure | 11-07 (base `7b726587b`) | 11-24 (user's run) |
|---|---:|---:|
| function rows / Unknown | 4,912 / 3,832 | 4,934 / 3,750 |
| K1 leaves | 6,718 | 6,366 (statement 4,856, condition 1,447, return 63) |
| Unknown rows touched | 1,651 | 1,489 |
| rows freed alone / distinct functions | 138 / 135 | **113 / 110** |
| freed alone by `statement` / `return` / `cond` alone | 69 / 48 / 16 | 67 / 27 / 14 |

The 135 functions of the 11-07 frozen list now: **8 Safe**
(`get_stream_path`, `LZ4_prepareTable`, `LZ4HC_clearTables`,
`rd_kafka_broker_update_reconnect_backoff`, `rd_kafka_message_leader_epoch`,
`XXH32_digest_endian`, `XXH64_digest_endian`, Monkey `header_cmp` — AG3's
rule 63 and the rounds since), **91 still freed alone**, **36 Unknown with a
changed blocker**: 19 the `rd_kafka_*interceptor_add_on_*` /
`AdminOptions_set_*` adders now carry a retention leaf (rule 71 removed their
access leaf; the retention is what remains), 8 a callback leaf, 3 an
uncontracted callee, 6 Monkey rows whose leaves moved lines in the
`--monkey-linked` text (`new`, not a semantic change).

What earlier cards refused on purpose and K1 does not re-admit (each a
registered owner, with its 11-24 count among the 110 functions freed alone):

* a custody-taking position (`*_destroy(arr[i])`, `cfl_sds_destroy(ctx->x->name)`,
  `close(cf->fd)`, `mk_mem_free(entry->key)`): 24 functions,
  W44-CALL-ARGUMENT-CUSTODY-POSITION and AG1-FORMAL-ARRAY-ELEMENTS-RELEASED (12);
* the mock API's `return rd_kafka_op_err_destroy(rd_kafka_op_req(…))`: 8,
  RK-OP-ERR-DESTROY-CONTRACT (AG1);
* several Borrowed `None` calls in one scalar statement (`append_metric_value`,
  `pack_static_labels`): AG1-TWO-BORROWED-CALLS;
* a pointer-valued `?:` return holding a call (`rd_kafka_error_string`):
  W45-RETURN-POINTER-RESULT-READS / rule 10z;
* an uncontracted callee in a condition (rule 63 (2) names it): W44-CONDITION-CALL-UNCONTRACTED.

## 2. Sub-shapes (method step 2)

**The base already reads the generic shapes.** Unit probes of the base gateway
(the generic `d_*`/`k_*` fixture contracts, [probes/](probes/README.md)) decide
`Safe` for a scalar, pointer and member-address actual in a statement, a
condition and a return; a chain (`b->next->name`); a typedef-spelled base;
a union-typed member's address (`&t->lock`); `?:` and cast actuals; a scalar
`?:` return holding a call. The leaves that remain are specific refusals.

Lexical taxonomy of the 110 functions freed alone, on the analysed `-E -P`
text at each leaf's line ([scripts/preprocess.py](scripts/preprocess.py),
[scripts/freed_taxonomy.py](scripts/freed_taxonomy.py),
[census/freed-taxonomy.json](census/freed-taxonomy.json)); primary class first:

| Primary class | Functions | Sample |
|---|---:|---|
| custody-taking position (incl. 8 mock nested destroys) | 32 | `rd_kafka_NewTopic_destroy(new_topics[i])` |
| function-pointer actual (a callback beside the access) | 17 | `rd_kafka_timer_start0(&rk->rk_timers, &rk->rk_eos.pid_tmr, …, cb, rk)` |
| nested call in an actual | 14 | `if (flb_routes_mask_get_bit(ic->routes_mask, …, ic->in->config->router) != 0)` |
| log / format macros | 10 | `rd_kafka_log0(&rk->rk_conf, rk, …, rd_kafka_err2str(err), …)` |
| conditional actual / return | 9 | `strcmp(a->dst ? a->dst : "", b->dst ? b->dst : "")` |
| call through a formal function pointer | 4 | `cmp(rl->rl_elems[i], rl->rl_elems[i + 1])` |
| cast in an actual | 4 | `print_string_ptr((unsigned char*)item->valuestring, p)` |
| parenthesised member address `&(p)->f` | 1 | `rd_atomic32_get(&(rk)->rk_terminate)` |
| `sizeof (*v)` in an actual | 1 | `memmove(…, sizeof(*errstack->errs) * errstack->cnt)` |
| other (run-defined callees, record-array decay, …) | 18 | `poll(rktrans->rktrans_pfd, …)` |

Over all 5,767 touched K1 sites (1,489 rows) the largest lexical class is
the **parenthesised member address**, `&(p)->f`: 1,691 leaf groups (2,245
leaves) on 441 rows, the expansion of librdkafka's lock, terminate and
buffer-read macros ([census/pp-classes.json](census/pp-classes.json)).

## 3. Chosen shape and why it is sound: rule 75

Probe on the base: `d_len(&(t)->n)` over a typedef-spelled `t` is refused in
a statement, a condition and a return; `d_len(&t->n)` is `Safe`. The cause is
not the typed reader (which strips the parentheses) but the never-bound
closure: rule 10o's reader of parenthesised writes seeds `p` as addressed
whole in `& ( p ) -> f`, while the plain reader exempts `& p -> f`. Every
typed position that needs the never-bound premise then refuses
(rule 10k(3)'s address and pointer actuals, rule 10n(1)'s member address),
wherever the lexical recogniser does not type the base — every typedef-spelled
librdkafka handle.

[Rule 75](../../agents/gateway.md#constraint-75)
(`crates/qcore-c-gateway/src/parenthesised_member_operand.rs`): the
never-bound closure drops exactly that group's address seed when the two
tokens after the group are `-` `>`. **Soundness**: a postfix operator binds
tighter than a unary one (C11 6.5.2, 6.5.3), so `& ( p ) -> f` is
`& ( ( p ) -> f )`, and a parenthesised expression is the expression it
encloses (6.5.1p5): the `&` takes the member place, never `p`, which can be
neither reassigned nor released through it — the plain reader's own declared
premise for `& p -> f`. The reading never exceeds the plain one
(`at_least_the_plain_reader`), misses no whole-object address
(`a_whole_address_is_seeded`, `every_whole_address_is_seeded`) and only
removes seeds (`the_closure_only_drops_seeds`). The aggregate store's
premises and `may_bind` keep rule 10o's reading byte for byte
(`the_old_reading_is_kept_without_the_arm`): the plain `may_bind` reader
binds `&p->f`, so changing it would make the parenthesised spelling the more
permissive one there.

Lean: `QCore.ParenthesisedMemberOperand` (`proof/AuditParenthesisedMemberOperand.lean`,
9 theorems; axioms `propext`, `Classical.choice`, `Quot.sound` only, no
project axiom).

Fixtures (`crates/qcore-c-gateway/tests/k1_call_actual_access.rs`, 5 tests,
every row status and full key set): admitted — statement (union and scalar
member), lock/read/unlock, condition, whole-call return, scalar `?:` return,
doubly parenthesised, a later hop; the plain spelling unchanged; refused —
root released after / before the address (`unknown_alias:t` kept), whole
address elsewhere (`g(&(t))`), incremented root, owned local (bound) and
released local, pointer member `&(t)->name`, array member `&(t)->a`,
custody-taking position, released root in a condition; loops: a walk
`t = t->next` (unchanged reading) `Safe`, a release in the body never `Safe`.
Unit tests: `parenthesised_member_operand::tests` (decision vectors; only the
closure reading drops the seed; increment, compound, whole-address, `.` and
`[` spellings keep it; rule 71 unchanged).

## 4. Refused shapes and their named reasons

Counts are the lexical taxonomy's (functions freed alone, primary class; sites
over all 1,489 touched rows from [census/lexical.json](census/lexical.json));
the base call-argument stream was not recorded (section 6).

| Shape | Freed-alone functions | Kept refused because | Owner |
|---|---:|---|---|
| custody-taking position (`*_destroy(arr[i])`, `cfl_sds_destroy(c->x->name)`, `close(cf->fd)`, `cf->fd = open(…)`) | 24 (654 custody-like sites) | the callee takes custody of an object the projection cannot name (rule 10u, by design) | W44-CALL-ARGUMENT-CUSTODY-POSITION, AG1-FORMAL-ARRAY-ELEMENTS-RELEASED |
| mock API nested `rd_kafka_op_err_destroy(rd_kafka_op_req(…))` return | 8 | the destroy has no contract | RK-OP-ERR-DESTROY-CONTRACT |
| function-pointer actual beside the access (`rd_list_init(&x->l, 0, rd_free)`, `rd_kafka_timer_start0(…, cb, rk)`, `…, rktp->rktp_rkt->rkt_conf.msg_order_cmp)`) | 17 | a callback handed on is an open set (rule 11p / callback routes); most callees are run-defined and uncontracted | K1-FUNCTION-POINTER-ACTUAL-BESIDE-ACCESS (new) |
| nested call in an actual, or a condition calling a run-defined callee (`flb_routes_mask_get_bit`, `rd_kafka_buf_read_NodeEndpoints`) | 14 | the callee has no contract in force (fail-closed); several Borrowed calls in one statement are rule 14a's one-call limit | W44-CALL-ARGUMENT-UNCONTRACTED, AG1-TWO-BORROWED-CALLS |
| log/format macros over `rd_kafka_log0` | 10 (1,085 sites, 514 rows) | the contract is premised (`rk_plugin_callback_closure` and two more): a caller is at best Pending; tail actuals spell `RD_KAFKAP_STR_PR` `?:` expansions | K1-LOG-MACRO-ACTUALS (new) |
| conditional actual or pointer `?:` return (`strcmp(a->dst ? a->dst : "", …)`, `return e->errstr ? e->errstr : f(e->code)`) | 9 | branching lowering of a `?:` actual is rule 10z's; a pointer `?:` return holding a call is W45's | W45-RETURN-POINTER-RESULT-READS, W44-CALL-ARGUMENT-NON-STATEMENT |
| call through a formal function pointer (`cmp(rl->rl_elems[i], …)`) | 4 | an indirect callee is an open set (rule 11p) | K1-CALL-THROUGH-FORMAL-POINTER (new) |
| cast in an actual (`(unsigned char*)item->valuestring`, `(LZ4_stream_t*)(c->p)`) | 4 (113 sites) | casts beyond rule 12h's identity views (CG08) | W44-CALL-ARGUMENT-UNTYPED-ACTUAL |
| `sizeof (*v)` in an actual | 1 (117 sites) | never folded lexically; the typed reader has no unevaluated-operand arm | W44-CALL-ARGUMENT-UNTYPED-ACTUAL |
| record-array decay (`poll(rktrans->rktrans_pfd, …)`, an array of `struct pollfd`) | 1 | rule 14c's element address is scalar elements only; a record element needs a no-pointer-member premise | K1-RECORD-ARRAY-DECAY-ACTUAL (new) |
| a typedef-spelled root bound by another event (`t_t *t = k_alloc(…); d_len(&(t)->n);`, or a formal released or handed to an uncontracted callee) | (probe) | the lexical recogniser types only `struct T *` bases; the typed reader needs the never-bound premise; rule 61's alias reading is the aggregate store's only | K1-TYPEDEF-ROOT-BOUND-MEMBER-ADDRESS (new) |
| `INT * f(x)` at the end of a return or initialiser (`return 31 * d_len(s);`) | 0 (1 site) | the lexical classifier reads `* f (` as a dereferenced call: the return is refused, and an initialiser `r = 31 * d_len(…)` is misattributed to ``call to uncontracted function `d_len` `` for a contracted callee (fail-closed, wrong text) | K1-SCALAR-PRODUCT-CALL-OPERAND (new) |

## 5. Shared-file hook points

| File | Hook |
|---|---|
| `crates/qcore-c-gateway/src/lexical_nested_assignment.rs` | `parenthesised_writes` delegates to the new `parenthesised_writes_reading(body, member_operands)`; one added conjunct in its `address` test (`!(member_operands && parenthesised_member_operand::member_operand(body, close))`) |
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | `seed_nested` calls `parenthesised_writes_reading(body, parenthesised_member_operand::ARM)` |
| `crates/qcore-c-gateway/src/lib.rs` | `mod parenthesised_member_operand;` after `mod logical_and_operand;` |
| `proof/QCore.lean` | `import QCore.ParenthesisedMemberOperand` (alphabetical) |
| `docs/agents/rules/10k.md`, `10o.md` | one "Amended by rule 75" sentence each; `gateway.md`/`README.md` regenerated |

None of the six files the brief names as shared (legacy_c.rs,
legacy_cfg_builder.rs, legacy_transfer.rs, legacy_classify.rs,
typed_access_path.rs, typed_field_access.rs) is touched.

## 6. Measurement

No measurement of an implementation build is part of this card. The
unit-scoped census of the 124 touched units (base and tip in one `audit` slot,
[scripts/run_current_census.py](scripts/run_current_census.py) with its
`OUT.then` companion, [scripts/census-run.sh](scripts/census-run.sh)) waited
4 h 25 min for a slot (pinned corpus runs and two queued Rust suites ahead of
it) and was stopped (exit 143) a few minutes after it was admitted, before any stream or report was written, when the coordinator relayed the instruction below; only this card's PIDs were killed (`private/k1/base-touched.stopped`, receipt `private/k1/base-touched.receipt.json`). On the user's instruction of 2026-11-24 the
integrator merges K1, K3, K5, K7 and SP10 onto origin/main and measures that
one tip (corpus eight-home, twelve-home, system, slice, then `current`);
[PREDICTION.md](PREDICTION.md) is the forecast it is read against. The
fixture and probe evidence is in section 3 and [probes/](probes/README.md).

## 7. Gate

No card gate (user instruction 2026-11-24, relayed by the coordinator: "We
don't need to run the each of K-related gates. We must run the merged
K-related tip's gate."). A scoped card gate had been started
(`docs/reports/k1-call-actual-access/scripts/gate.sh 1`, i.e.
`python3 scripts/gate_scope.py --base 3ca85c7a6 --out private/gate-1`, mode
full because the diff reaches `qcore-c-gateway`); submodules, fmt and Clippy
had passed (`private/gate-1/fmt.log`, `private/gate-1/clippy.log`) and the
Rust suite was queued at position 2 for the `rust-suite` slot when it was
stopped (exit 143) by killing only this card's own PIDs (1075491, 1067357,
1067353); `private/k1/gate-1.stopped` records it.

Targeted checks run on the final code (`be1f3c8c5`'s analyzer bytes; later
commits change records only), each in a `build` or `lean` slot:

| Check | Command | Exit | Result |
|---|---|---:|---|
| fmt | `cargo fmt --all -- --check` | 0 | |
| fixtures | `cargo test -p qcore-c-gateway --test k1_call_actual_access` | 0 | 5 passed |
| unit tests | `cargo test -p qcore-c-gateway --lib` filtered to `parenthesised_member_operand lexical_nested_assignment logical_and_operand borrowed_actual` | 0 | 44 passed (the module's 2 included) |
| Lean | `lake build QCore.ParenthesisedMemberOperand` | 0 | |
| Lean audit | `lake env lean AuditParenthesisedMemberOperand.lean` | 0 | 9 theorems; `propext`, `Classical.choice`, `Quot.sound` only |
| rule docs | `python3 scripts/gen_rule_docs.py` | 0 | `gateway.md`, `README.md` regenerated |
| amendments | `python3 scripts/apply_amendments.py docs/reports/k1-call-actual-access/integration-amendments.json --check` | 0 | valid (6 amends, 6 registers) |

Logs: `private/k1/fixture-tests.log`, `private/k1/lib-tests.log` ([scripts/targeted-tests.sh](scripts/targeted-tests.sh)), builds `private/k1/fixture-build2.log`, `private/k1/lib-build2.log`,
`private/k1/lean-2.log`, `private/k1/lean-audit-1.log`,
`private/k1/gen-rule-docs.log`, `private/k1/amendments-check.log`.

## 8. Follow-ups

Registered or amended in [integration-amendments.json](integration-amendments.json):
the three owners (W44-CALL-ARGUMENT-UNTYPED-ACTUAL, -NON-STATEMENT,
-UNCONTRACTED) amended with the 11-24 figures and rule 75; W44-CALL-ARGUMENT-CUSTODY-POSITION,
RK-OP-ERR-DESTROY-CONTRACT and AG1-FORMAL-ARRAY-ELEMENTS-RELEASED amended with
their 11-24 counts; new: K1-FUNCTION-POINTER-ACTUAL-BESIDE-ACCESS,
K1-LOG-MACRO-ACTUALS, K1-CALL-THROUGH-FORMAL-POINTER, K1-RECORD-ARRAY-DECAY-ACTUAL,
K1-TYPEDEF-ROOT-BOUND-MEMBER-ADDRESS, K1-SCALAR-PRODUCT-CALL-OPERAND.
