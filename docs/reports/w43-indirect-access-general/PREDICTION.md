# W43-INDIRECT-ACCESS-GENERAL — frozen forecast

Frozen **before** any admission rule was written. The analyzer tree at the
time of freezing is base `d3cd8d81` plus census instrumentation only (the
`indirect-census` feature and a declaration-reader side table no rule reads);
every census run reproduced the baseline diagnostics byte for byte. Later
corrections go beside this file in the report, never into it.

## 1. What the census says (baseline `docs/reports/w43-libc-result-alias/artifacts/`)

Measured by `scripts/shape_census.py` over the census run `census-base6`
(binary `9e8a8c84…`, statement stream `34da9251…`, indirect stream
`1fea4329…`). Leaves counted in Unknown functions, as the blocker census counts
them.

| Family | Leaves | Unknown functions | Sole-blocked |
|---|---:|---:|---:|
| indirect memory access is outside the resource projection | 17,665 | 3,315 | 378 |
| unsupported assignment expression | 1,683 | 589 | 4 |
| unrecognized assignment destination | 901 | 434 | 3 |
| unmodeled increment or decrement expression | 734 | 378 | 5 |
| store through `X` without an out-parameter contract | 656 | 253 | 2 |

The brief's own example, `*p = '\0'`, is **not** in the indirect family: a
single dereference destination is named `*p` and lands in the store-through or
unsupported-assignment family, and `p[i]++`/`(*p)++`/`p->f += e` land in the
increment and unrecognized-destination families. The census therefore covers
all five families, statement by statement.

By syntactic shape, the indirect family is dominated by accesses inside call
arguments (8,128 leaves, 2,131 functions), chain stores (2,862), single member
stores (2,602) and chain reads (2,262). Admitting a syntactic shape *wholesale*
would decide 81, 22, 19 and 57 functions respectively — an upper bound that no
typed rule reaches, because 72% of access chains end at a member or base the
declaration reader does not classify (the finding of
`C-GATEWAY-INDIRECT-CENSUS-2` and `BASE-TYPE-READER-CENSUS`, reproduced here).

## 2. The admission chosen

One rule, **a typed scalar access statement**, reported by sub-shape. It is
consulted only for a statement the existing classifier already refused with
one of the four descriptions above, or lowered to an unsanctioned `*p` store.
Every admitted statement carries no call, no `&`, no comma, no nested
assignment and no increment except its own, and every access in it is one of

* `* B`, where `B` is declared `SCALAR * B`;
* `B [ I ]`, where `B` is declared `SCALAR * B` or `SCALAR B [ N ]` and `I` is
  a scalar-only index (declared scalars, enumeration constants, literals);
* `B -> F`, where `B` is declared `struct T *` and `F` a declared scalar member
  of a completely read `struct T`;
* `S . F`, where `S` is declared `struct T` (an object, not a pointer) and `F`
  a declared scalar member;

and the statement is one of `ACCESS = E`, `ACCESS op= E`, `++ACCESS`,
`ACCESS++` (never `*p++`, which increments the pointer), `X = E`, `X op= E` and
`SCALAR_TYPE X = E` with `X` a declared scalar, where `E` is built from
literals, declared scalars, enumeration constants, scalar casts and such
accesses. The destination is scalar and the value is scalar, so the statement
can neither acquire, release, transfer nor rebind a tracked identity.

**Flow obligation.** Every dereferenced base (`*B`, `B[I]`, `B->F`) must, at
the statement, be either **unbound on every reaching path**, a
**non-null binding with no tracked target** (an untracked or derived-untracked
pointer after its null test), or **exactly one known, non-null, live identity**
with no nullable or unresolved-effect marker — derived bindings included, which
dereference their base's allocation. Anything else refuses with `UnknownAlias`
and keeps the frame. A base settled on one released identity is additionally a
`use_after_release` candidate, read from the settled states.

**Refused shapes, kept refused:** pointer-valued stores (`p->f = q`,
`*pp = q`), chains of depth two or more, accesses inside call arguments,
`*p++`, subscripts whose index carries an access, unclassified members and
bases, casts to non-scalar types.

## 3. Forecast

From `scripts/predict.py`, the Python mirror (`scripts/typed_scalar.py`) of
the rule above applied to every census statement.

**Leaves leaving their family** (the lexical half; each is either removed or
exchanged for a flow `UnknownAlias`):

| Family | Unknown functions | All statuses |
|---|---:|---:|
| store through `X` without an out-parameter contract | 426 | 446 |
| indirect memory access | 273 | 293 |
| unrecognized assignment destination | 129 | 144 |
| unmodeled increment or decrement expression | 56 | 64 |
| unsupported assignment expression | 38 | 38 |
| **total** | **922** | **985** |

By sub-shape (Unknown functions): `store:*p` 482, `store:p[i]` 129,
`read-into-scalar` 97, `compound-store:p->f` 61, `store:s.f` 51,
`increment:p->f` 35, `store:p->f` 25, `compound-into-scalar` 18,
`compound-store:*p` 16, `compound-store:p[i]` 4, `increment:*p` 2,
`compound-store:s.f` 2.

**Exchanges.** Of the 915 dereferenced base occurrences in admitted
statements of Unknown functions, 848 are labelled neither owned nor derived
(formals and locals; forecast **admitted** by the unbound or untracked arm), 61
are assigned an owned result and 6 a derived result somewhere in their body
(forecast admitted only where a null test dominates). Point forecast: at most
**70** leaves exchanged for `UnknownAlias`, the remainder removed.

**Function moves: 11 Unknown → Safe, 0 to provisional Unsafe.** Ten are
decided directly, each inspected in the pinned source:

| Function | Shape | Flow forecast |
|---|---|---|
| `cio_chunk.c` `cio_chunk_counter_total_add` / `_sub` / `_up_add` / `_up_sub` | `ctx->n++` | `ctx` unbound: admitted |
| `cmt_encode_opentelemetry_batch.c` `set_result` | `*result = value` under `result != NULL` | unbound: admitted |
| `flb_input_chunk.c` `flb_input_chunk_output_would_exceed_limit` | two `*available_space = …` | unbound: admitted |
| `rdkafka_admin.c` `rd_kafka_ConsumerGroupDescription_authorized_operations`, `rd_kafka_TopicDescription_authorized_operations` | `*cntp = RD_MAX(grpdesc->cnt, 0)` | unbound: admitted |
| `rdmap.c` `rd_bytes_hash` | `hash = … + bytes[i]` | unbound: admitted |
| `mk_string.c` `mk_string_dup` | `p[len] = '\0'` after `p = mk_mem_alloc(…); if (!p) return NULL;` | one live identity: admitted |

The eleventh is transitive (constraint 4): `cmt_encode_opentelemetry_batch.c`
`empty_batches_create`, whose remaining leaves are calls to `set_result`,
forecast to gain a validated summary.

**Units: 0 moves. Leak pin: 272, unchanged. Violation candidates: 0** — no
census statement dereferences a base after a release on its own path, so the
accusing arm is forecast to reach no corpus function.

**Hazard named in advance.** Removing a refusal removes its binding poison
(`POISON-UNMASK-ON-ADMISSION`). Any leak or double release that appears, and
any refusal that disappears at a coordinate other than an admitted statement's,
is attributed separately from the rule.

## 4. What this forecast is not

The mirror reads the census labels, i.e. the declaration reader's own answers;
it does not re-read declarations. Its one disclosed gap: a record-pointer
base's member class is read from the census chain for that base. The forecast
is a count of statements the rule admits and of functions whose every leaf
would then clear; `Safe` additionally needs every obligation discharged, which
only the arbiter measures.
