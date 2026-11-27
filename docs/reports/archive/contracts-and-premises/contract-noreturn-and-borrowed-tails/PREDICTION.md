# CONTRACT-NORETURN-AND-BORROWED-TAILS: prediction (frozen before any contract edit)

Base: local `main` `ca3576dd4`. Frozen after the BEFORE runs and the census,
before any provider contract was edited. The only code in the tree at this
point is the control test file (`crates/qcore-c-gateway/tests/contract_noreturn_borrowed_tails.rs`,
its fixture `%n` control) and this report's scripts.

## BEFORE (this card's own runs, `scripts/measure.sh before`)

| Route | Totals | Digest |
|---|---|---|
| corpus `66910c10` | **648 / 9 / 4,177** | `bcc98d2d…` (equals the brief's) |
| system | 44 / 2 / 188 | raw `f67dd631…`, worktree-normalised `36e40330…` |
| tail-pause slice | 29 / 1 / 238 / 2 Pending | raw `791ef8b0…`, normalised `17b1cfa1…` |

Merged totals by `scripts/merged_totals.py`; the system and slice reports
embed the worktree path, so their raw digests differ from earlier cards'.

## Census (`scripts/census.py`, `scripts/near.py`; `artifacts/*-before-*`)

**No-return callees.** The corpus parity stream's `call-exit-census` rows
resolve exactly one uncontracted callee `NoReturn`: `rd_kafka_crash`, 92
statements, **50 leaves on 40 functions (all Unknown), 0 blocked by it
alone** (CG21's figures reproduce). The system and slice streams resolve
none. No other callee is in scope of part 1.

**Tail refusals** (leaves / functions / blocked alone; statements by the
shape of their trailing actuals: T8-shaped = every trailing actual a plain
name or a literal):

| Route | Callee | Leaves | Functions | Alone | Statements: T8-shaped / chain / other / assignment form |
|---|---|---|---|---|---|
| corpus | `fprintf` | 124 | 77 (75 Unknown, 2 Unsafe) | 0 | 120: 110 / 2 / 8 / 0 |
| corpus | `snprintf` | 55 | 33 (32 Unknown, 1 Unsafe) | 0 | 55: 41 / 1 / 4 / 9 |
| corpus | `mk_print` | 15 | 8 | 0 | 20: 20 / 0 / 0 / 0 |
| corpus | `printf` | 3 | 3 | **1** (`cb_debug_chunk_complete`) | 4: 4 / 0 / 0 / 0 |
| system | `flb_log_print` | 16 | 7 | 0 | 10: 6 / 2 / 2 / 0 |
| system | `snprintf` | 2 | 1 | 0 | 1: 0 / 0 / 1 / 0 |
| slice | `flb_log_print` | 2 | 1 | 0 | 2: 1 / 0 / 1 / 0 |
| slice | `fprintf` | 6 | 1 | 0 | 3: 3 / 0 / 0 / 0 |

CG26's registered sizes (at `b6a3dc42`: `fprintf` 126 / 78, `mk_print` 17 /
9, `snprintf` 55 / 33, `printf` 3 / 3, system `flb_log_print` 10 and
`snprintf` 1 statements) are re-measured here: `fprintf` and `mk_print` are
smaller at this base (later cards removed functions' other leaves or the
statements); the statement counts reproduce. No other callee with a declared
tail has a tail leaf except `fcntl`, whose one statement is already admitted
(T7). `sprintf` has no tail leaf and keeps its data tail.

**Jointly** (functions whose every leaf is one of the card's: the
`rd_kafka_crash` call or a tail refusal of the five callees): round 1, **1**
(`mk_server/mk_http_parser.c::cb_debug_chunk_complete`); round 2 (callers
whose remaining leaves are calls to a same-unit function freed in round 1),
**0**. System and slice: 0 and 0.

**One other leaf** (corpus; nothing in system or slice has fewer than 3):
`rd_kafka_version_str` (an `unsupported assignment expression`, and its tail
statement is an assignment form with arithmetic actuals) and
`mk_config_print_error_msg` (`unresolved resource identity `path``: the body
releases its formal `path` after the `mk_err`).

## Predictions

1. **Corpus status moves: exactly one**, `cb_debug_chunk_complete` Unknown →
   Safe (`printf("…%zu…%s…", in_len, buf)` with `buf` a live `mk_mem_alloc`
   buffer released after the call: T8 reads `buf` as a borrowed place).
   Allowance: up to 2 further Unknown → Safe from the path end (an assertion
   arm that no longer joins), each named. **No Safe → non-Safe move**, and no
   Unknown → Unsafe.
2. **`rd_kafka_crash`**: all 50 leaves go; every one of the 92 statements is
   a path end resolved `NoReturn` (parity `call-exit-statement` rows), none
   refused as `no-return contract … not established`. Other leaves of the 40
   functions may go where the crash arm weakened a fact at a join (CG21 saw
   36 such); none is added except, possibly, a caller-side refusal of a
   released `rk` (none expected). The function's own row stays Unknown
   (`rd_kafka_dump0` uncontracted); the declaration is not refuted (a body
   with no normal exit publishes no summary, rule 10g leaves it unchecked).
3. **Tail leaves (corpus)**: of 197, at least 140 and at most 185 go (the
   T8-shaped and chain-shaped statements, unless the enclosing function's
   declarations are unreadable, which turns T8 off); the 12 `other` and the 9
   assignment-form statements keep theirs. In their place some `unresolved
   resource identity` leaves may appear (a plain name the projection cannot
   resolve, now checked as a borrowed place): not predicted in number; each
   function gaining one is attributed.
4. **No leak or violation added** by the tail, except a released-identity
   candidate where a body prints a pointer after releasing it; any such is
   read and classified (C04.2's caller-side rule makes it an unresolved
   result, not a violation claim, so none is expected as `Unsafe`).
5. **System**: no status move; the 6 T8-shaped and 2 chain-shaped
   `flb_log_print` statements lose their tail leaf (`flb_coro_destroy`'s
   pointer arithmetic and `flb_engine_reschedule_retries`' / `flb_task_running_print`'s
   member chains decide the rest); `snprintf`'s `(void *) src` cast stays.
6. **Slice**: no status move; `flb_lib_config_file`'s three `fprintf` and
   `do_start`'s `flb_log_print` lose their tail leaves; `flb_coro_destroy`
   keeps its.
7. **Round-12 tip probe** (`mk_vhost_set` at Monkey `21a2c190`): unchanged.
   BEFORE: tip `mk_vhost_set` Unknown (`call to uncontracted function
   mk_vhost_set_property` only), `mk_vhost_set_property` Unknown (two indirect
   accesses); master `mk_vhost_set` Unknown (five `va_end` protocol refusals and
   the uncontracted `mk_vhost_set_property`). Neither body calls a callee this
   card declares.

## `%n` (decided before the edit)

`%n` stores the count written so far through a trailing `int *` (C11
7.21.6.1p8): a write through a pointer, not a custody change. The borrowed
tail reads that actual as a `Borrowed` place, exactly as `snprintf`'s
written-through `s` and every borrowed destination are read: nothing is
acquired, released or transferred. The only way a borrowed tail could certify
something false through `%n` is a custody decision resting on the rewritten
count. The fixture control `a_percent_n_write_through_an_aliased_scalar_is_never_certified`
(a synthetic borrowed-tail callee, run on this base) shows none is: `&x` (T7)
forgets `x` after the call (the leak on the `x != 0` arm is reported), `px =
&x` is itself refused by the projection, and `*cnt` of a pointer formal is not
a fact the projection holds. The same control runs against the production
`printf`, `fprintf` and `snprintf` after the edit. No `%n` conversion occurs in
the audited printf-family call sites (`src`, `plugins`, `lib/librdkafka-2.15.0`,
`lib/monkey` at `66910c10`, searched by pattern). So the libc change is not
refused on `%n`.
