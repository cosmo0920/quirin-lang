# CG26-TAIL-RESIDUAL-ACTUALS

Wave 48, round 16. Gateway rule [12g](../../agents/gateway.md#constraint-12g),
`QCore.VariadicTailActuals`. Authority: the registered follow-up
`CG26-TAIL-RESIDUAL-ACTUALS` and the
[CONTRACT-NORETURN-AND-BORROWED-TAILS report](../contract-noreturn-and-borrowed-tails/README.md).
Base: local `main` `cb3085365` (main did not move during the card). Prediction
frozen at `b2bce36f6` ([PREDICTION.md](PREDICTION.md)).

**Scope change.** Before the prediction was frozen, the integrator passed on a
user decision: a new card, CAST-FORMALIZATION (rule 12h), owns every cast
reading. This card reads plain names in functions whose declarations the
lexical reader cannot read, non-cast arithmetic and conditional actuals, and
the assignment form. An actual that contains an explicit cast is refused by
name and left to CAST-FORMALIZATION.

**Published property and scope.** Resource custody on route G, unchanged:
every acquired resource is released or handed on at every return. The card
adds one reading of a trailing actual of a declared variadic tail. It admits
the actual only as data that carries no tracked identity, or as a `Borrowed`
place of one pointer name. Nothing is claimed about format strings, values or
bounds.

**Result.**

- Corpus: **649 / 9 / 4,176, unchanged** (`450a9e54…` → `b864e7ba…`).
- 25 of the 40 tail leaves go, in 15 Unknown functions. No other leaf moves,
  and no function changes status.
- System: no status move. 2 of its 4 tail leaves go.
- Tail-pause slice: byte-identical.

Ledger 696 → 698: the entry is amended to its cast residual, and two
follow-ups are registered.

## 1. Why the residual refused

Rule 11w reads a trailing actual from the statement's tokens. T4 (a declared
scalar), T5 (a typed chain) and T8 (a plain name under a borrowed tail) need
the lexical declaration reader's facts (`FunctionTypes::is_readable`). That
reader is total-or-reject: one declaration it cannot read makes it answer
nothing for the whole function. Examples:

- an anonymous `struct { … } test_cases[] = …` (`unittest_ssl`);
- a function-pointer parameter (`unittest_msgq_order`'s
  `int (*cmp)(const void *, const void *)`);
- a function-pointer array (`rd_kafka_sticky_assignor_unittest`'s `tests[]`).

24 of the 34 statements sit in such functions. In the 10 readable ones the
token premises still refuse:

- `max_cnt * sizeof (crc)`: the type operator looks like a call to T3;
- `(ver >> 24) & 0xff` in `rd_kafka_version_str`;
- the casts and the identity conditionals.

Making the lexical reader read those declarations would change every typed
rule of those functions, not only the tail. So the card reads the actual
**from the call's Clang argument node** instead, which carries the type Clang
gave it. Rule 11w's fallback already matches that node, and its rule-10l
facts are already available.

## 2. What changed

[`variadic_tail_actuals.rs`](../../../crates/qcore-c-gateway/src/variadic_tail_actuals.rs)
is new. It reads the node into an abstract `TailExpr`. Parentheses and
implicit conversions are transparent. The operand of an arithmetic implicit
conversion is read as arithmetic.

`classify` is one recursion with three outcomes:

- **Data.** Any of:
  - a literal, a predefined identifier, an enumeration constant, or a
    constant-size `sizeof`/`_Alignof`;
  - an arithmetic object that is not an integer-carried identity;
  - a pointer or array object for which rule 10l's served never-bound
    premise holds;
  - a unary `- + ~ !`, a binary arithmetic, bitwise, shift, comparison or
    logical operator, or a `?:`, when every operand is data.
- **Identity.** One name that may carry a tracked identity.
- **A named refusal.**

`read` turns an identity into a `Borrowed` place of that name only when all
of these hold:

- it is a pointer or array object;
- the callee declares `resource_variadic_borrowed_tail()`;
- rule 10l answers the name (declared exactly once, as a formal or an
  automatic local);
- the token actual is exactly that name.

The flow then applies its liveness and alias checks to that object. Every
refused reading keeps O04-TRACE's refusal text byte for byte. The reason is
in the parity row's `tail_readings` (`cast`, `pointer-arithmetic`,
`identity-arm`, `identity-operand`, `integer-identity`,
`identity-under-data-tail`, `unanswered-root`, `unread`).

**Hooks.** Thin hooks in `legacy_variadics.rs`:

- `tail_readings` reads the nodes once per statement, inside rule 11w's
  fallback and after its promoted-type premise;
- `tail_reading_admits` is a fourth alternative of `tail_widened`, asked only
  after T1-T8 refuse the position.

Shared items, named:

- `Parser::variadic_tail_readings` (`legacy_c.rs`), set only around the
  fallback's re-classification;
- `Parser::position_answered` (`legacy_typed_roots.rs`), rule 10l's
  "declared once" premise.

No older admission changes code (hard invariant 3).

**Casts.** An actual that contains an explicit cast anywhere is refused as
`cast`. No cast is read.

**Pointer arithmetic.** It is refused as well: `p + 1`'s root is not a place
the projection identifies (the token actual has no canonical place), so a
borrowed reading would check nothing.

**Conditionals.** A `?:` alone in the tail is decided first by rule 10z's
value-connective lowering. Each arm becomes its own call. Rule 12g decides a
`?:` only when that lowering leaves the statement refused, for example when a
sibling name needs this reading, as in `unittest_msgq_order`.

## 3. Reproduced claims

| Claim (registered) | At this base | Standing |
|---|---|---|
| corpus 40 tail leaves in 23 functions (`fprintf` 27 / 14, `snprintf` 11 / 8, `printf` 2 / 2), none blocked by them alone | 40 / 23 (27, 11, 2), none alone | reproduces |
| 34 statements: 19 plain names in unreadable functions, 14 cast/arithmetic/conditional, 1 assignment form | 34 statements with a `variadic-tail-statement` row, 24 in unreadable functions; 6 of the 40 leaves (two `rdhttp.c` tests) have **no row** | the statement split is the predecessor's classification; the 6 unmatched leaves are new (`CG26-TAIL-UNMATCHED-STATEMENTS`) |
| system 4 leaves in 2 functions: `flb_coro_destroy`'s pointer arithmetic and `flb_storage_quarantine_chunk`'s `(void *) src` | 4 leaves: `flb_sched_timer_cb_create` 2 (`type`) and `flb_storage_quarantine_chunk` 2 | **correction**: `flb_coro_destroy` (a header inline body) has no report row; its 2 statements are refused but carry no leaf |
| slice none | none (1 refused `flb_coro_destroy` statement, no leaf) | reproduces |

BEFORE (release `quirin-audit` `39953f7d…`):

| Route | Totals | Digest |
|---|---|---|
| corpus | 649 / 9 / 4,176 | `450a9e54…`, equal to the brief |
| system | 44 / 2 / 188 | normalised `96e77cdb…` |
| slice | 29 / 1 / 238 / 2 Pending | `f8efd8cb…` |

([totals](artifacts/totals-before.txt),
[census](artifacts/residuals-before-corpus.txt).)

## 4. Prediction against outcome

| Prediction (frozen) | Outcome |
|---|---|
| Corpus: no move; no Safe → non-Safe; at most 2 Unknown → Unsafe, none expected | **no move** (AFTER-2) |
| Tail leaves 40 → 15; the 25 gone in the named 15 functions; the 15 kept named (6 casts, 3 identity conditionals, 6 unmatched) | **exactly** at AFTER-2. The first AFTER (`after`, binary `639d2501…`) left 17: `j` in `rd_kafka_sticky_assignor_unittest` and `rd_kafka_range_assignor_unittest` was read `unread`. Its type is `rd_kafka_assignor_ut_rack_config_t`, a typedef of an anonymous enumeration. Clang gives it no desugared spelling. The fix (`225cd9ce8`, after the freeze): an arithmetic implicit conversion's operand is arithmetic (the integral promotion Clang applied is the evidence), with a unit test and a control. AFTER-2 (binary `d161cee2…`) then matched the prediction |
| Other leaves of the 15 functions may change (unresolved identities on borrowed names) | **none changed**: no leaf was added anywhere |
| System: no move; its 4 tail leaves stay (both casts) | no move. **Missed**: `flb_sched_timer_cb_create`'s 2 leaves go (its formal `int type` is data). The frozen census followed the registered attribution, corrected in §3 |
| Slice byte-identical | byte-identical |
| Tip probe unchanged | unchanged (§5) |
| Lean audits 282 → 283 | 283 (§6) |

## 5. Measurement and attribution

Corpus arbiter `66910c10` `--jobs 4`; system route and tail-pause slice
([`scripts/measure.sh`](scripts/measure.sh)). Each run went into a fresh
`private/cg26r/<run>` directory.

| Run | Corpus | System | Slice |
|---|---|---|---|
| BEFORE (`39953f7d…`) | 649 / 9 / 4,176 `450a9e54…` | 44 / 2 / 188 (normalised `96e77cdb…`) | 29 / 1 / 238 / 2 P `f8efd8cb…` |
| AFTER-1 (`639d2501…`, retained) | 649 / 9 / 4,176 `bc4cb9e3…` | 44 / 2 / 188 (`eda4ea86…`) | identical |
| **AFTER-2** (`d161cee2…`, `225cd9ce8`) | **649 / 9 / 4,176 `b864e7ba…`** | 44 / 2 / 188 (`eda4ea86…`) | identical |

([totals](artifacts/totals-after2.txt).)

The first BEFORE script ran all three routes. A second invocation for system
and slice, started by mistake with the same binary bytes, was stopped through
its task handle before it ran anything. Its outputs never replaced the first
run's.

**Every moving function** (`scripts/compare.py`;
[corpus](artifacts/compare-after2-corpus.txt),
[system](artifacts/compare-after2-system.txt),
[slice](artifacts/compare-after2-slice.txt)):

- Corpus, 15 functions, all Unknown → Unknown. The only change is the removal
  of 25 tail leaves: `fprintf` 18, `snprintf` 5, `printf` 2.
  - `rd_kafka_sticky_assignor_unittest` 4, `unittest_ssl` 4,
    `rd_kafka_range_assignor_unittest` 3, `rd_string_render` 3, and 1 each in
    `do_unittest_erase`, `do_unittest_write_read_payload_correctness`,
    `mk_server_info`, `rd_kafka_topic_new0`, `rd_kafka_version_str`,
    `rd_msghdr_print`, `rd_unittest`, `unit_test_telemetry`,
    `unittest_fetcher_share_filter_forward`, `unittest_msgq_order`,
    `ut_string_split`.
  - Nothing is added: no leak, violation or unresolved identity.
  - No Safe → non-Safe move, so there is no correction to name.
  - No provisional Unsafe is added.
- System, 2 rows: `flb_sched_timer_cb_create` (two configurations),
  `flb_log_print`'s tail leaf.
- Slice: none.

**What the readings were** ([AFTER-2 census](artifacts/residuals-after2-corpus.txt);
admitted rows carry their `tail_readings` in the parity stream):

- Pointer names became `borrowed:` places:
  - `errstr` (a `char[256]`), `desc`, `expected`, `result`;
  - `topic`, `used_conf_str`, `t`, `tmpkey`;
  - `tmp`, `what` (`rd_msghdr_print`), `match`.
- Scalars became data: `i`, `j`, `expsz`, `r`, `len`, `tof`, `fails`,
  `metrics_payload_size`.
- Expressions became data:
  - `i + 1`;
  - `max_cnt * sizeof (crc)`;
  - `fifo ? "FIFO" : "LIFO"`;
  - `(ver >> 24) & 0xff` in the assignment form.

**Left** (15 corpus leaves, all refused by name):

- `cast` (6, CAST-FORMALIZATION's):
  - `ut_quantile`;
  - `rd_kafka_anyconf_get0`;
  - `rd_kafka_snappy_java_uncompress`;
  - `rd_kafka_conf_set_ssl_cert`;
  - `unittest_untyped_map` ×2.
- `identity-operand` (3):
  - `rd_kafka_ssl_error0` ×2: `data` is address-taken by
    `ERR_get_error_all`;
  - `unittest_untyped_map`'s `val2 ? val2 : "NULL"`: `val2` holds
    `rd_map_get`'s result.
- No row (6): `unittest_http_get` 4 and `unittest_http_get_params_append` 2.

In the system route, `flb_storage_quarantine_chunk`'s `(void *) src` is a
`cast`. `flb_coro_destroy`'s `coro` is an `unanswered-root`, because rule 10l
does not answer a header inline body. Its pointer arithmetic is a `cast`.

**Round-12 probe** ([`scripts/tip-probe.sh`](scripts/tip-probe.sh) on a
private copy of the integrator's probe tree;
[rows](artifacts/tip-probe-rows.json), [`scripts/tip_rows.py`](scripts/tip_rows.py)):

| Copy | Function | BEFORE | AFTER-2 |
|---|---|---|---|
| tip (`21a2c190`) | `mk_vhost_set` | Unknown: uncontracted `mk_vhost_set_property` | the same leaves |
| tip | `mk_vhost_set_property` | Unknown: two indirect accesses | the same |
| master | `mk_vhost_set` | Unknown: five `va_end` protocol refusals and `mk_vhost_set_property` | the same |
| master | `mk_vhost_set_property` | Unknown: two indirect accesses | the same |

Neither body has a declared-tail call with a residual actual.

## 6. Controls and Lean

**Integration controls.**
[`tests/cg26_tail_residual_actuals.rs`](../../../crates/qcore-c-gateway/tests/cg26_tail_residual_actuals.rs)
has 5 tests over the production `printf` family, loaded byte for byte beside
the CG00 fixture contracts. Each body sits in a function whose formal list
the lexical reader refuses (`int (*cb)(int)`):

- **Accepted, and `Safe`:**
  - a tracked pointer;
  - an array;
  - `i + 1`, a scalar and `sizeof(b) * 2`;
  - a data conditional beside a name;
  - operators;
  - an anonymous-enum object;
  - the assignment form.

  Each is also `Safe` in a readable function, where T4 and T8 decide it.
- **Refused:**
  - a released pointer in the tail is never `Safe`, and a dropped one leaks;
  - casts (`(long) i`, `(float) i / 1000.0`, `(void *) p`), pointer
    arithmetic, an identity arm, an identity condition beside a name, and a
    pointer under `sprintf`'s data tail all keep the tail refusal;
  - a shadowed pointer name is unanswered and stays refused.
- **Rule 10z.** A lone `p ? "set" : "none"` is decided by rule 10z, readable
  or not. This is recorded, not claimed for this card.

**Unit tests.**
[`tests/unit/variadic_tail_actuals.rs`](../../../crates/qcore-c-gateway/tests/unit/variadic_tail_actuals.rs):

- the node shapes (the anonymous-enum promotion, a VLA `sizeof`, `&`, `++`,
  a function, a function pointer, a record);
- the shared vectors. The Lean `readVectors` are parsed and compared with
  `READ_VECTORS` row by row, and each row is re-run through the Rust `read`.

**Lean.** [`QCore.VariadicTailActuals`](../../../proof/QCore/VariadicTailActuals.lean)
and [`AuditVariadicTailActuals.lean`](../../../proof/AuditVariadicTailActuals.lean)
(282 → 283 audits; no `sorry`, no project axiom; `propext` and `Quot.sound`
only):

- `data_iff_nothing_carried`: over the arithmetic fragment, an actual is data
  exactly when no operand carries an identity.
- `every_data_reading_carries_nothing`.
- `borrowed_is_the_whole_root`: a borrowed reading is one carrying, answered
  pointer name under a borrowed tail.
- `the_data_tail_is_never_widened`.
- `a_cast_is_never_read` and `a_cast_is_refused_by_name`.
- `pointer_arithmetic_is_never_read`.
- `an_identity_arm_is_refused`, `an_identity_condition_is_refused`,
  `an_unanswered_root_is_refused`, `an_integer_identity_is_refused`.
- `the_reading_is_custody_neutral`, through
  `QCore.Variadics.borrowed_tail_is_the_identity`.
- `read_vectors_hold`.

## 7. Gate

GATE_SECTION

## 8. Follow-ups

Ledger 696 → 698 ([`scripts/update_ledger.py`](scripts/update_ledger.py)).

**Amended: `CG26-TAIL-RESIDUAL-ACTUALS`.** It is not retired. Its DISCHARGE
named cast readings, which the scope change gives to CAST-FORMALIZATION. The
entry now holds only the cast residual, re-measured:

- corpus: 6 leaves, 6 statements, 5 functions;
- system: 2 leaves;
- `flb_coro_destroy`'s 2 (system) and 1 (slice) leafless statements;
- the system-attribution correction.

Its new DISCHARGE is CAST-FORMALIZATION's reading consulted for these trailing
actuals.

**Registered:**

- `CG26-TAIL-IDENTITY-CONDITIONALS`: 3 leaves, 2 functions. A conditional
  whose condition or arm may carry an identity, read per arm, or accepted as
  a measured boundary.
- `CG26-TAIL-UNMATCHED-STATEMENTS`: 6 leaves, 2 `rdhttp.c` tests. Their
  statements reach no rule-11w row. Rule 10l's AST reader does not answer
  either function, though the lexical reader reads both. The cause is not
  diagnosed further.

**Prediction discipline.** A first draft of the module, its unit test and
the Lean model was written before `PREDICTION.md` was committed. It was moved
out of the tree, neither compiled nor measured, until the prediction was
frozen at `b2bce36f6`. It is disclosed here.
