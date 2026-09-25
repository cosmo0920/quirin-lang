# W44-LIBC-RESIDUALS: header-inline helpers, `va_list`, literal tails and nested tail callees

Card W44-LIBC-RESIDUALS, 2026-09-25, on base `c1e69bb5` (main after "Open the
wave-44 increment record"). It works the four libc-side residuals beside the
largest remaining libc leaf masses, measuring each before building anything:

1. glibc's header-inline `__bswap_16/32/64` (the two largest single
   uncontracted callees) and the other header-inline scalar helpers the census
   finds;
2. `LIBC-PRINTF-VA-LIST`: the `va_list` printf members;
3. `LIBC-PRINTF-TAIL-ACTUALS` and the `arguments beyond the declared
   parameters` family;
4. `LIBC-PRINTF-NESTED-TAIL-CALLEES`.

A [prediction](PREDICTION.md) was frozen and committed (`dca6f959`) before any
reader, tail-rule or contract edit and before any staged run.
[§7](#7-prediction-against-outcome) compares it with the outcome.

> **Paused for a route change on item 1 (2026-09-25).** W44-CORPUS-CALLEE-SUMMARIES
> measured that glibc's `__bswap_*` header bodies already validate under W4 as
> `Borrowed -> None` and are held back only by its callee-scope route's
> reserved-name filter (`W44-CALLEE-SCOPE-RESERVED-BODIES`). This card will take
> that route (a validated body is stronger than a trusted declaration), drop
> `libc/byteswap.qcore`, and re-run the arbiter on main once that card is
> integrated. Everything below describes the declared route as measured, and
> §8 (validation) has not been run on it: the gate was stopped.

## Headline

* **Item 1, declared, and said to be header-inline.** Six helpers
  (`__bswap_16/32/64`, `__uint16/32/64_identity`) in the new
  `libc/byteswap.qcore`, each the neutral element. They are `static __inline`
  bodies in glibc headers, outside the enumerated main-source denominator
  (E01-B2), so they are declared rather than projected: a trusted assumption in
  place of a body the unit carries. The census finds **one** identical
  signature and body per helper in all **174** defining units, each pure
  integer arithmetic over its one scalar parameter. `__bswap_64` needed a
  reader change: `__extension__`, which carries no type, is now discarded.
  **3 functions move Unknown → Safe** (`ut_put_be16/32/64`).
* **Item 2, read and declared.** The reader projects `__builtin_va_list` at a
  parameter to a new loader spelling, `va_list`, with Lean parity
  (`QCore.GnuBuiltinReader`). It is borrowed-only, a parameter type only, and
  target-independent. All four members agree in all **168** units;
  `vsnprintf` and `vprintf` are declared (the other two are never called). 26
  leaves go, 1 comes back, and no function moves: 24 of the 26 callers keep
  `__builtin_va_start`/`__builtin_va_end` leaves.
* **Item 3, a sound narrow widening, built.** **T6** admits a tail actual that
  is only literals and C11 6.4.2.2 predefined identifiers, whatever the
  function's typed declarations or C02-P's closure say (Lean
  `QCore.LiteralTailActual`). Measured first: **no function** could move. It
  removes **974** of the 1,200 tail refusals and adds none.
* **Item 4, measured, one libc callee declared.** 89 printf-nested refusals at
  this base. `strerror` is declared (2 leaves go). The other 87 are corpus
  callees, inner calls inside a larger expression, or Cyrus SASL, and none of
  them blocks a function alone.
* **Arbiter.** BEFORE reproduces the W43-INDEXED-RESOURCE-COLLECTIONS
  integration diagnostics **byte for byte**. The reader stage alone is
  byte-identical to BEFORE in all five artifacts. End to end, **1,629 leaves
  are removed and 6 added**, every one attributed per stage, and the stages
  compose exactly ([compose.json](attribution/compose.json)). Units do not move.
  Leak stays **271**. **Violation candidates: none.**
* **Follow-ups 345 → 345.** Four retired, four registered, each with its size.

| Measure | BEFORE (combined wave-43 baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | **297 / 85 / 4,452** | +3 / 0 / −3 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `unsupported` | 45,848 | **44,225** | −1,623 |
| `unknown_alias` | 4,358 | 4,358 | 0 |
| `leak` / `summary_conflict` / `double_release` | 271 / 1 / 0 | 271 / 1 / 0 | 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole (Unknown functions) | 9,547 / 2,591 / 407 | 8,911 / 2,577 / 404 | −636 / −14 / −3 |
| Unknown functions blocked only by one uncontracted callee | 302 (231 callees) | 299 (228 callees) | −3 |
| tail refusals leaves / functions / sole (Unknown functions) | 1,014 / 246 / 1 | 213 / 125 / 1 | −801 / −121 / 0 |
| nested-call refusals leaves / functions / sole (Unknown functions) | 136 / 91 / 5 | 134 / 89 / 5 | −2 / −2 / 0 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 111 / 47 / 152 | **112 / 48 / 161** | +1 / +1 / +9 |

The census rows count Unknown functions, as
[blocker-census.txt](blocker-census.txt) does; the baseline's rows are the same
script over the baseline artifacts. The brief's 279 + 247 `__bswap_32`/`__bswap_16`
leaves and its 1,014 / 246 tail figure are these Unknown-function counts; over
all functions the families are 599 and 1,200 leaves.

## 1. The census, before anything was built

Four scripts ran over BEFORE and a private capture of the 185 units (tee
wrapper, base binary; 180 paired `-E -P` texts, the same 5 unpaired units as
every wave-43 card):

* [leaf_census.py](scripts/leaf_census.py) counts each family per callee and
  bounds its function yield: the non-Safe functions whose **every** leaf the
  family's admission would remove
  ([census-leaves-before.json.gz](census-leaves-before.json.gz));
* [header_inline_census.py](scripts/header_inline_census.py) reads every
  `static`/`extern` inline definition in a `/usr/include` header and every
  main-source call of one
  ([census-header-inline.json](census-header-inline.json));
* [tail_nested_lines.py](scripts/tail_nested_lines.py) reads every tail and
  nested refusal against its analysed line
  ([census-tail-nested-before.json.gz](census-tail-nested-before.json.gz));
* the production `declared_abi_census` over the measurement modules in
  [measurement/](measurement/) and the whole manifest
  ([abi-evidence.json](abi-evidence.json)).

| Family at BEFORE | Leaves | Functions | Bound: non-Safe functions whose every leaf it holds |
|---|---:|---:|---:|
| header-inline callees (`__bswap_32` 286, `__bswap_16` 252, `__bswap_64` 60, `__uint64_identity` 1) | 599 | 99 | 3 (`ut_put_be16/32/64`) |
| `vsnprintf` 25 + `vprintf` 1 callee leaves | 26 | 26 | 0 |
| tail refusals (`fprintf` 1,093, `snprintf` 78, `mk_print` 21, `printf` 8) | 1,200 | 263 | 1 (`cb_debug_chunk_complete`) |
| printf-family nested refusals | 89 | 62 | 0 |

The one tail-bound function, monkey's `cb_debug_chunk_complete`, prints
`buf`, a live `mk_mem_alloc` allocation. Admitting it takes a tail admission of
a tracked identity, which is not the narrow widening the ledger names, so T6
was forecast to move no function. It moved none.

## 2. Item 1: header-inline helpers

### 2.1 The decision

The brief offered three ways: declare them in `quirin-provider-libc` as the
neutral element and say they are header-inline bodies, let the analyzer
project their bodies, or refuse them. **They are declared.**

* **Projecting the bodies** would cross E01-B2: the function denominator is
  the main-source definitions, and a definition spelled in a header is outside
  it by design. Deriving W4 summaries for header bodies without counting them
  is an analyzer-level change of its own, beside W44-CORPUS-CALLEE-SUMMARIES'
  territory. It is registered as one discharge route of
  `W44-HEADER-INLINE-BODY-UNCHECKED`, not taken here.
* **Refusing them** would keep 599 leaves for bodies that are, measurably,
  scalar arithmetic.
* **Declaring them** is a trusted assumption (trust boundary 28), and the
  module says so. The census makes it small. Every one of the 174 units that
  defines these helpers carries exactly one (signature, body) pair per helper,
  from `/usr/include/x86_64-linux-gnu/bits/byteswap.h` and
  `bits/uintn-identity.h`:

| Helper | Body (tokens, all 174 units) | Called in (units / main-source sites) |
|---|---|---:|
| `__bswap_16` | `return ((__uint16_t) ((((__bsx) >> 8) & 0xff) \| (((__bsx) & 0xff) << 8)));` | 13 / 253 |
| `__bswap_32` | four masks and shifts of `__bsx` | 16 / 287 |
| `__bswap_64` | eight masks and shifts of `__bsx` (`__extension__ static __inline`) | 8 / 80 |
| `__uint16_identity`, `__uint32_identity`, `__uint64_identity` | `return __x;` | 1 / 1, 2 / 3, 1 / 1 |

No body calls anything, takes an address or touches an object other than its
parameter. The census is a measurement of the pinned corpus, not a gate: the
declaration is applied without comparing a unit's body with it, and that is
registered as `W44-HEADER-INLINE-BODY-UNCHECKED`.

### 2.2 `__extension__`

The declared-ABI reader reads a definition as the C declaration, so the gate
works for these bodies. Five of the six agreed in 174/174 units at the base.
`__bswap_64` was **withdrawn in all 174**: its prefix is
`__extension__ static __inline __uint64_t`, and `__extension__` was not a
discarded specifier, so the core `__extension__ __uint64_t` projected to
nothing. `__extension__` is a GNU keyword that suppresses pedantic diagnostics
and carries no type; the typed-field and return-call readers already discard
it. It is now in `DISCARDED_SPECIFIERS`, and all six agree in 174/174
([abi-evidence.json](abi-evidence.json), `residuals8-reader`).

### 2.3 Effect (stage 2)

[attribution/stage2.json.gz](attribution/stage2.json.gz), by
W43-LIBC-CONTRACTS' [`attribute.py`](../w43-libc-contracts/scripts/attribute.py)
with `libc/byteswap.qcore` as its module list; `attribution_complete: true`.

| Class | Removed | Added |
|---|---:|---:|
| declared-callee: `__bswap_32` 286 (87 functions), `__bswap_16` 252 (84), `__bswap_64` 60 (33), `__uint64_identity` 1 | 599 | — |
| newly-safe-callee: `ut_put_be32` 15, `ut_put_be64` 8, `ut_put_be16` 4, in `ut_build_valid_msgset_v2` (12), `ut_write_msgset_v2_header` (12) and three `unittest_msgset_*` (1 each) | 27 | — |
| same-line: `nested call __uint16_identity` in `emit_copy_less_than64`, exchanged for `resource call with a formal embedded in an unmodeled expression` | 1 | 1 |
| same-line: `resource call with a formal embedded in an unmodeled expression` at `(&x)->len = (int16_t)__bswap_16 (_v);` in `rd_kafka_mock_handle_ConsumerGroupHeartbeat`, `…ShareGroupHeartbeat` and `rd_kafka_buf_read_NodeEndpoints` (2) | — | 4 |

The forecast exchange did not happen. The 412 `*_vp = __bswap_32 (_v);` and
`*_vp = (int16_t)__bswap_16 (_v);` sites in librdkafka's buffer-read macros are
stores through a pointer to a local scalar, which W43-INDIRECT-ACCESS-GENERAL's
typed scalar-access rule already admits once the call is contracted, and so are
most member stores. Only the four length stores above refuse, on a formal
inside the destination expression.

**The three functions that move Unknown → Safe**, each relative to the
declared premises (invariant 2's reading, not a proof of the C):

| Function | Unit | Its only BEFORE blocker |
|---|---|---|
| `ut_put_be16` | `rdunittest_msgset_errors.c` | `int16_t be = __bswap_16 (v);` (`htobe16`), then `memcpy(p, &be, sizeof(be))` |
| `ut_put_be32` | same | `__bswap_32`, same shape |
| `ut_put_be64` | same | `__bswap_64`, same shape |

Their callers lose the newly-safe-callee leaves above and stay Unknown.

## 3. Item 2: `va_list`

### 3.1 The reader and the spelling

glibc's four members end in `__gnuc_va_list __arg`, and
`typedef __builtin_va_list __gnuc_va_list;` is all the unit carries: the
builtin is the compiler's. On x86-64 it is `__va_list_tag[1]`, adjusted to an
object pointer at a parameter; on AArch64 it is a structure passed by value.

The ledger allowed a target-bound pointer spelling. This card spells the
**type** instead: `VA_LIST_ABI_TYPE`, `va_list`, in the loader's closed
vocabulary (contract spec §C04.1), beside `fnptr` and on the same terms.

* It is not a `ptr` spelling, so the loader's two custody gates make
  `resource_borrowed` the only writable mode.
* It is a parameter type only: `-> va_list` does not load, and the reader
  never reads the builtin at a result.
* The comparison is exact in both directions: `va_list` is refused for every
  other C type, and every other spelling (`ptr<mut, u8>`, `usize`) is refused
  for the builtin. No target representation is assumed beyond the reader's
  existing LP64 caveat.

The reader change is one arm in `project_scalar`
(`crates/qcore-c-gateway/src/legacy_declared_abi.rs`, `BUILTIN_VA_LIST`): the
name `__builtin_va_list`, reached directly or through any typedef chain, reads
as `va_list` at a parameter of pointer depth zero, **unless the unit itself
`typedef`s that name**, in which case its own typedef is followed. A pointer to
it, an array of it and a result of it still refuse. Ten new rows in
`crates/qcore-c-gateway/tests/loader_abi.rs` (one for `__extension__`, nine
for `va_list`) and a result-position test pin all of this. `compiler/tests/contract_loader.rs`
pins the spelling: borrowed loads, every custody mode and a `va_list` result
are rejected.

### 3.2 Lean parity

`proof/QCore/GnuBuiltinReader.lean`, registered in
`proof/AuditGnuBuiltinReader.lean` (28 theorems), with `AbiType` gaining
`vaList` in `QCore.DeclaredAbiAgreement`:

* **conservativity** of both widenings: `the_extension_widening_only_adds`,
  `a_prefix_without_extension_is_unchanged`, `extension_is_invisible`,
  `the_va_list_widening_only_adds`, `resolution_is_conservative` (a chain that
  does not reach the builtin reads exactly as `TypedefProjection.resolve` did);
* **exactness**: `only_va_list_is_admitted_for_the_builtin`,
  `no_earlier_reading_admits_va_list`,
  `the_builtin_is_not_its_x86_64_representation`;
* **what still refuses**: `a_va_list_result_refuses`,
  `a_pointer_to_a_va_list_refuses`,
  `a_typedef_pointer_to_the_builtin_stays_unresolved`, and shadowing,
  `a_unit_typedef_of_the_builtin_name_is_followed`;
* **custody**: `borrowed_is_the_only_mode_on_a_va_list`,
  `an_owned_result_of_a_va_list_is_rejected`,
  `the_result_vocabulary_lost_only_va_list`;
* **non-vacuity** with the measured witnesses:
  `the_bswap64_result_was_unreadable_and_is_now_read`,
  `the_glibc_chain_reaches_the_builtin`,
  `vsnprintf_agrees_now_and_was_withdrawn_as_unreadable`,
  `a_pointer_spelling_of_the_va_list_disagrees`.

`QCore.NativeLibraryHeader`'s manifest vocabulary excludes `vaList`, as the
Rust manifest table (`qcore_core::extern_call::AbiType`) does; its theorem
`the_manifest_vocabulary_excludes_exactly_three_spellings` is renamed
`…_exactly_the_unspellable_spellings` and now lists four. That header reader
has no builtin arm and never produces `vaList`; the change keeps the model's
vocabulary equal to the Rust one rather than calling `va_list` spellable by
default. The Rust–Lean correspondence is by review, not proof (trust boundary
28). No placeholder, no project axiom.

### 3.3 Census and declarations

The census reads all four members as agreeing in all 168 units
(`valist4-reader`). `vsnprintf(s, maxlen, format, arg)` and `vprintf(format,
arg)` are appended to `libc/stdio.qcore`, every parameter borrowed and
`resource_return_none()`. The `va_list` is the caller's; the call reads the
arguments it reaches as text and frees or retains none of them, the tail
premise stated for the list that carries them. The list is used up (C11
7.16p3), which is not custody. `vfprintf` and `vsprintf` agree but are never
called, so they are not declared (the `dprintf` precedent).

### 3.4 Effect (stage 3)

[attribution/stage3.json.gz](attribution/stage3.json.gz): the 26 callee leaves
go (`vsnprintf` 25, `vprintf` 1). One comes back as a same-line
`resource call with a formal embedded in an unmodeled expression` at
`strsz = vsnprintf(((void*)0), 0, fmt, ap2) + 1;` in `rd_kafka_error_new_v`.
No function moves. What still blocks the callers is in
[attribution/valist-callers.json](attribution/valist-callers.json): 24 of the 26
carry 50 `__builtin_va_start`/`__builtin_va_end` leaves, and
`rd_kafka_set_thread_name` now has exactly those two leaves and nothing else.
Registered as `W44-VA-START-END-BUILTINS`: 127 such leaves in 51 functions
across the corpus.

## 4. Item 3: T6, literal tail actuals

### 4.1 The rule, and why it is sound

`variadic_nested::tail_arguments_are_data_with_nested` refused a literal on
two premises, not one:

* **readability**: when a function's typed declaration facts are unreadable,
  every tail refuses, a literal included;
* **the closure**: T4 admits an identifier only if C02-P's closure says the
  body never binds it, and `seed_call_arguments` marks as risky every root
  name passed to an uncontracted call. The tokenizer delivers every string
  literal as the identifier-shaped `__qcore_literal__`, so one literal passed
  to an uncontracted call made every string literal of the function risky.
  librdkafka's `RD_UT_ASSERT` does exactly that:
  `__assert_fail ("…", "<stdin>", 1349, __PRETTY_FUNCTION__)`.

Both premises protect the **names** a tail may carry. A literal and a
predefined identifier are not names any declaration or binding reaches: a
literal denotes a value or a static array, and `__func__` a static array (C11
6.4.2.2p1), neither of which is a resource identity. Clang lexes `__func__`,
`__FUNCTION__` and `__PRETTY_FUNCTION__` as keywords, so none can be declared.

T6 (`is_literal_or_predefined_actual`) therefore admits an actual whose every
token is `__qcore_literal__`, a numeric literal (a digit then alphanumerics) or
one of the three predefined identifiers. It applies whether or not the
declarations are read and whatever the closure says. Where the declarations are
unread, nothing else is admitted. Where they are read, every other actual still
needs T1–T5. Still refused: any other name, a sign, a cast, a floating literal
(it carries `.`), a prefixed string (`L"…"`), an empty actual, and a statement
with a nested call.

### 4.2 Lean parity

`proof/QCore/LiteralTailActual.lean`, registered in
`proof/AuditLiteralTailActual.lean` (16 theorems), over
`QCore.VariadicNested`'s `admitsTail`:

* `the_widening_only_adds`, and `a_read_site_without_literal_actuals_is_unchanged`;
* `the_declaration_is_still_required`;
* **soundness over every binding assignment**:
  `a_literal_actual_carries_no_identity_under_any_declarations`,
  `every_actual_admitted_unread_carries_no_identity`, and
  `every_actual_admitted_read_is_literal_or_data`;
* **what still refuses**: `an_unread_name_still_refuses`,
  `an_unread_punctuator_still_refuses`, `an_empty_actual_still_refuses`,
  `a_risky_name_beside_it_is_still_refused`;
* **non-vacuity**, both halves with the measured `RDUT:` shape:
  `the_rdut_tail_was_refused_and_is_admitted` (unread) and
  `the_closure_risky_literal_tail_was_refused_and_is_admitted` (read).

The gateway unit test `an_unreadable_function_admits_no_tail` pinned the old
behaviour (a literal `1` refused in an unreadable function). It is re-pinned
deliberately as `an_unreadable_function_admits_only_a_literal_tail`, which
also pins nine refused shapes, and
`a_literal_tail_is_data_even_where_the_closure_marked_literals_risky` pins the
closure half.

### 4.3 Measured before building, and the first form

The census bounded T6's function yield at **0** before it was written.

T6 was first built for unreadable functions only (commit `1aebc27f`). Its
stage run ([runs/stage1-narrow-run.json](runs/stage1-narrow-run.json),
[attribution/stage1.json.gz](attribution/stage1.json.gz)) removed **86**
leaves in 21 functions, against a forecast of 850–1,091.
[`tail_residual.py`](scripts/tail_residual.py) then found 523 remaining leaves
on lines whose only calls had literal-only tails, for example
`unittest_rd_crc32c`'s
`fprintf(stderr, "RDUT: INFO: %s:%d: %s: ", "<stdin>", 422, __FUNCTION__);`.
Those functions are readable, and the closure had marked the literal. The
closure half was added (`0a75d66e`), which is what the ledger's
"readability-independent" wording asked for. The narrow run is kept as a
disclosed intermediate. It is not the stage the arbiter composes.

### 4.4 Effect (stage 1)

[attribution/stage1b.json.gz](attribution/stage1b.json.gz), by
[`attribute_tail.py`](scripts/attribute_tail.py) (W43's `attribute.py` with a
T6 first class; `attribution_complete: true`):

| Class | Removed | What it is |
|---|---:|---|
| `t6-literal-tail` | 967 | a tail refusal whose line carries a call of the callee with a literal-only tail |
| `t6-mixed-tail` | 6 | a literal actual beside a declared scalar (`snprintf(tmp, …, "%*s", off, "")` in ctraces `format_array`/`format_event`) |
| `t6-tail-statement-spans-lines` | 1 | `rd_kafka_broker_set_error`, whose statement starts on the line before its call |
| **total** | **974** | `fprintf` 962, `snprintf` 9, `mk_print` 3; 86 in 21 unreadable functions, 888 in readable ones |

Nothing is added. No function moves (the largest per-function removals are
`unittest_msgset_all_error_types_with_valid` 37,
`unittest_share_acknowledge` 24, `unittest_idempotent_producer` 23). By library:
librdkafka 965, ctraces 6, monkey 3.

**What is left** ([attribution/tail-residual-after.json.gz](attribution/tail-residual-after.json.gz)):
226 tail refusals in 132 functions (`fprintf` 131, `snprintf` 69, `mk_print`
18, `printf` 8). 223 sit on lines whose calls carry a named tail actual, and 3
are statements that span lines. A name is exactly what T1–T5 must decide.
Registered as `W44-TAIL-NAME-ACTUALS`, bounded at one function
(`cb_debug_chunk_complete`, the tracked `buf`). `MK-PRINT-TAIL-ACTUALS`' 18
remaining `mk_print` sites are among the 226, and that entry stays open.

## 5. Item 4: nested tail callees

At this base, 89 nested-call refusals have a printf-family outer call (62
functions), not W43-LIBC-PRINTF-FAMILY's 100. Read against their lines
([census-tail-nested-before.json.gz](census-tail-nested-before.json.gz)):

| Premise that refuses | Leaves | Inner callees |
|---|---:|---|
| N3, the inner callee has no contract in force, and it is **libc** | 2 | `strerror(r)`, `strerror(-r)` |
| N3, the inner callee is **corpus-defined** | 58 | `rd_kafka_err2str` 42, `rd_kafka_error_string` 7, `rd_kafka_msgq_len` 3, `rd_kafka_broker_name` 2, `rd_kafka_err2name`, `rd_kafka_event_name`, `rd_kafka_txn_state2str`, `rd_sockaddr2str` 1 each |
| N4, the call is only part of the argument | 28 | `!(rd_kafka_msgq_len(…) == …)` 9, `rd_list_cnt` 9, `rd_buf_len` 3, `rd_kafka_msgq_size` 2, `(double)(rd_clock() …)` 2, `(long) getpid()` 2, `path[strlen(path) - 1]` 1 |
| N3, outside libc | 1 | `sasl_errstring` (Cyrus SASL) |

The brief admits a nested callee only where it is contracted with a scalar or
borrowed result. The existing N3/N6/N7 premises already do that for any
callee declared `resource_return_none()` with borrowed parameters, so no
premise is widened here. The one libc callee a tail passes whole is declared:

* **`strerror`**, appended to `libc/errno.qcore` beside the `errno` accessor,
  `errnum` borrowed and `resource_return_none()`. glibc returns a message in
  static storage or in a thread-local buffer the next call may overwrite, which
  the caller must not free and which points into no argument. That is the
  `gai_strerror`/`dlerror`/`__errno_location` precedent. It agrees in 181 units.
  It is **not** put in `libc/string.qcore`, which is PUBLISHED and whose
  SHA-256 every inheriting contract's identity carries
  (`providers/contract-load-path.json`): appending there would move that
  identity for a declaration no inheriting contract uses.
* **`getpid`** agrees in 137 units, but both of its sites are
  `(long) getpid()`, which N4 refuses. A declaration would decide nothing, so it
  is not declared.

Stage 4 ([attribution/stage4.json.gz](attribution/stage4.json.gz)) removes the 2
`strerror` leaves (`rd_kafka_transport_new`, and `rd_kafka_snappy_java_uncompress`,
which stays provisional `Unsafe` on its other leaves) and adds none. The 58
corpus callees are W44-CORPUS-CALLEE-SUMMARIES' bodies. A validated W4 summary
or a read declaration of one with a `none` result and borrowed parameters is
admitted by the unchanged premises. The residual is registered as
`W44-PRINTF-NESTED-INNER-CALLEES`.

## 6. Arbiter

All runs use
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py)
(`PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19`, `QCORE_CLANG=/usr/bin/clang`, the wave-20
ChunkIO package), through [`scripts/stage_run.py`](scripts/stage_run.py). That
wrapper records the SHA-256 of each stage's contract files before and after the
run and refuses a stage whose files changed while it ran. Every stage
recorded `unchanged_during_run: true`. The stages are cumulative.

| Run | Binary (sha256) | Code | Contracts | Exit | Window (UTC, 2026-09-25) | Record |
|---|---|---|---|---:|---|---|
| BEFORE | `d41624f0…` | `c1e69bb5` | base | 0 | 00:02–00:13 | [runs/before-run.json](runs/before-run.json) |
| stage 0, reader | `146810b9…` | `c3a53240` | base | 0 | 00:29–00:39 | [runs/stage0-reader-run.json](runs/stage0-reader-run.json) |
| stage 1 (narrow T6, disclosed) | `4172bf9b…` | `1aebc27f` | base | 0 | 00:41–00:51 | [runs/stage1-narrow-run.json](runs/stage1-narrow-run.json) |
| stage 1, T6 | `e49b314d…` | `0a75d66e` | base | 0 | 00:56–01:07 | [runs/stage1b-tail-run.json](runs/stage1b-tail-run.json) |
| stage 2 | `e49b314d…` | `0a75d66e` | + `libc/byteswap.qcore` | 0 | 01:08–01:19 | [runs/stage2-inline-run.json](runs/stage2-inline-run.json) |
| stage 3 | `e49b314d…` | `0a75d66e` | + `vsnprintf`, `vprintf` | 0 | 01:20–01:31 | [runs/stage3-valist-run.json](runs/stage3-valist-run.json) |
| stage 4 = AFTER | `e49b314d…` | `0a75d66e` | + `strerror` = committed `390ce29d` | 0 | 01:32–01:44 | [runs/stage4-after-run.json](runs/stage4-after-run.json), [artifacts/](artifacts/) |

The full digests are in each record. `quirin-provider-libc` is not linked into
the audit, and `390ce29d` changes only contract data and tests over
`0a75d66e`, so the stage-4 binary is the committed tree's analyzer. The stage-4
contract files hash exactly as committed ([runs/stage4-after-files.json](runs/stage4-after-files.json)).

**BEFORE vs the baseline**
([runs/before-identity.txt](runs/before-identity.txt), `baseline_identity.py`
with this worktree aliased to the integrator's root): diagnostics,
chunk-contracts and source **byte-identical**. `contracts` is identical once the
checkout root is aliased. `source-effects` differs only in the
`resource_contracts:<digest>` key and the `build_digest`, `configuration_digest`,
`task_digest`, `obligation_id` and `coverage[0]` values that bind it, the
legitimate checkout-path differences the brief lists.

**Stage 0 vs BEFORE:** all five artifacts byte-identical
([runs/stage0-vs-before.json](runs/stage0-vs-before.json)). This is the reader
change alone. The manifest census moves 0 of its 13,101 (unit, name) rows
([abi-evidence.json](abi-evidence.json), `reader_change_alone`).

**AFTER vs BEFORE** ([runs/after-vs-before.json](runs/after-vs-before.json),
[runs/contracts-diff.txt](runs/contracts-diff.txt)): chunk-contracts,
source-effects and source byte-identical. `contracts` differs in the manifest
text, the source text of `stdio.qcore` and `errno.qcore`, and one added source,
`byteswap.qcore`.

**Composition.** [`compose_stages.py`](scripts/compose_stages.py)
([attribution/compose.json](attribution/compose.json)) recomputes the
BEFORE → AFTER leaf delta and the signed sum of the four stages' attributed
rows. They are equal: **1,629 removed, 6 added**. Every stage exits with
`attribution_complete: true`. Every unit whose fields move changes only its
`diagnostic_counts` and count-summary `reason`, each by exactly its own
functions' leaf delta. No top-level field other than the counts moves.

| Stage | Removed | Added | Function moves | Leak |
|---|---:|---:|---|---:|
| 0 reader | 0 | 0 | none | 271 |
| 1 T6 | 974 | 0 | none | 271 |
| 2 header-inline | 627 | 5 | `ut_put_be16/32/64` Unknown → Safe | 271 |
| 3 `va_list` | 26 | 1 | none | 271 |
| 4 `strerror` | 2 | 0 | none | 271 |
| **BEFORE → AFTER** | **1,629** | **6** | **3 Unknown → Safe** | **271** |

**Leak pin (constraint 7).** Unmoved at **271**, with no leak leaf added or
withdrawn at any stage. T6 removed 974 refusals in 209 functions, and
none of them released a poisoned binding into a new leak.
Every declaration borrows every argument and releases nothing. The new
baseline is [artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json));
`docs/reports/w43-indexed-resource-collections/integration/artifacts/` is its
frozen predecessor and is not edited.

**Violation candidates: none.** `double_release`, `invalid_release` and
`use_after_release` are 0 before and after. No provisional `Unsafe` is added,
and none is presented as an upstream defect. Sole-blocker set movement
([sole-moves.json](sole-moves.json)): the three movers leave it and none enters.

## 7. Prediction against outcome

| Item | Predicted | Measured | |
|---|---|---|---|
| P0 decisions (what is built, declared, refused) | as frozen | as frozen, except `strerror` lands in `errno.qcore`, not `string.qcore` (published digest) | ✔ |
| P1 reader alone | 5 artifacts identical; 0 of 13,101 census rows | identical; 0 of 13,101 | ✔ |
| P2 T6 removed | 1,050 (850–1,091) | 974 (86 in the first, unreadable-only form) | ✔ final, ✘ first form |
| P2 T6 added / moves / leak | 40 (0–200) / 0 / 271 | 0 / 0 / 271 | ✔ |
| P3 header-inline census | 6 × 174 agree | 6 × 174 agree | ✔ |
| P3 removed | 600 | 627 (599 + 27 newly-safe-callee + 1 nested) | ✔ within the spirit, 27 more than the point |
| **P3 added** | **520 (400–600)** | **5** | ✘ |
| P3 moves | `ut_put_be16/32/64` (2–6) | exactly those three | ✔ |
| P4 `va_list` | 168/168; −26; +12 (4–26); 0 moves | 168/168; −26; +1; 0 moves | ✔ removed and moves, ✘ added below range |
| P5 `strerror` | 181 units; −2; +0–2; 0 moves | 181; −2; +0; 0 | ✔ |
| P6 functions | 297 / 85 / 4,452 (+2 to +8 Safe) | 297 / 85 / 4,452 | ✔ |
| P6 `unsupported` | ~44,700 (44,300–45,200) | 44,225 | ✘ just below range |
| P6 units, `unknown_alias`, leak, candidates | unchanged, ±30, 271, 0 | unchanged, 0, 271, 0 | ✔ |

**Where the forecast failed.**

* **The tail estimate read a line, not a call.** One `RDUT:` macro line carries
  several `fprintf` calls, and the census counted a leaf removable when its
  line held any literal-only call. It did not see the closure premise at all.
  The first form of T6, built on the readability premise the W43 probe had
  named, removed 86. The closure premise was found by reading what remained, and
  the rule was completed in the direction the ledger had already named.
* **The header-inline exchange assumed the store would refuse.** Most
  `*_vp = __bswap_32 (_v);` stores were already admissible once the call was
  contracted, through W43-INDIRECT-ACCESS-GENERAL's typed scalar-access rule.
  The prediction did not check that rule against these shapes.

## 8. Validation

GATE-TABLE-PLACEHOLDER

## 9. Ledger and status

* **Retired**, each preserved verbatim with its ground in
  [retired-followups.json](retired-followups.json):
  * `W43-LIBC-HEADER-INLINE-BODIES` (its "recorded decision" arm);
  * `LIBC-PRINTF-VA-LIST`;
  * `LIBC-PRINTF-TAIL-ACTUALS` (its arm (a));
  * `LIBC-PRINTF-NESTED-TAIL-CALLEES` (the libc half discharged, the rest
    re-registered below with its premises).
* **Registered** ([followups.json](followups.json)):
  * `W44-TAIL-NAME-ACTUALS`: 226 leaves, 132 functions, bound 1;
  * `W44-PRINTF-NESTED-INNER-CALLEES`: 87 leaves, 60 functions, bound 0;
  * `W44-VA-START-END-BUILTINS`: 127 leaves, 51 functions, one function
    blocked by nothing else;
  * `W44-HEADER-INLINE-BODY-UNCHECKED`: the trusted assumption, with its two
    discharge routes.
* **Follow-ups 345 → 345.** `corpus.cycle19_raw_units` points at this
  report's artifacts; its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* `docs/STATUS.md`'s projected-function, corpus-diagnostic, indirect-access,
  contract-corpus and follow-up rows are updated from this arbiter, and
  `docs/reports/INDEX.md` is regenerated.
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.1 gains the `va_list` vocabulary and
  the `__extension__` reading.

## 10. For the integrator

* **Constraint 7:** W44-LIBC-RESIDUALS leaves the pin at 271. It makes 9
  declarations, adds one analyzer admission (T6) and one reader widening, and
  moves 3 functions.
* **Constraint 8:** the reader paragraph can record that `__extension__` is a
  discarded specifier and that `__builtin_va_list` reads as the parameter-only
  `va_list` spelling (with the shadowing rule). Rule text is left to the
  integrator, as predecessors did.
* **`W43-LIBC-ABI-READER-SHAPES`** stays open. Its third shape, `va_list`, is
  discharged here; its transparent-union and enum-typedef shapes are unchanged.
  The entry's text was not edited.
* **`MK-PRINT-TAIL-ACTUALS`** stays open with 18 `mk_print` sites (21 at the
  base), all inside `W44-TAIL-NAME-ACTUALS`.
* **Composition with the concurrent wave-44 cards:**
  * W44-CORPUS-CALLEE-SUMMARIES: a validated summary for `rd_kafka_err2str` and
    its neighbours is admitted by the unchanged N3/N6/N7. The 58 whole-argument
    leaves are the expected exchange.
  * W44-INDIRECT-CALL-ARGUMENT-ACCESS: T6 admits no indirect read. It is one
    disjunct in `tail_arguments_are_data_with_nested`, and every other
    disjunct is unchanged.
  * No new `LegacyInstruction` variant is introduced, so
    `descriptor_custody::admits_descriptor_read` is untouched. A T6-admitted
    actual is a literal, which never carries a descriptor.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/variadic_nested.rs`: one disjunct and one early
    return in `tail_arguments_are_data_with_nested`, the T6 helper, and a
    re-pinned unit test;
  * `crates/qcore-c-gateway/src/legacy_declared_abi.rs`: one
    `DISCARDED_SPECIFIERS` entry, one `project_scalar` arm, the
    `BUILTIN_VA_LIST` constant and the import;
  * `crates/qcore-contract/src/contract_loader.rs`: `VA_LIST_ABI_TYPE` and the
    two vocabulary iterators;
  * `proof/QCore.lean` (two imports), `proof/QCore/DeclaredAbiAgreement.lean`
    (`AbiType.vaList`), `proof/QCore/NativeLibraryHeader.lean` and its audit
    (the renamed theorem);
  * pins that compose additively: `compiler/tests/qcore_contract_corpus.rs`
    (corpus 111 → 112, eleven clause pins),
    `compiler/tests/contract_loader.rs` ((47, 152, 216) → (48, 161, 216),
    modules 111 → 112, two source-assumption pins, nine parity entries),
    `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs` (47 → 48),
    `scripts/tests/test_contract_corpus.py` (111 → 112, shared crates 39 → 40);
  * `cycle19-resource-contracts.json` (one line after `dirent`),
    `providers/contract-load-path.json` (regenerate with
    `python3 providers/contracts.py sync`, never by hand),
    `providers/quirin-provider-libc/src/lib.rs` (`CONTRACTS` index),
    `providers/quirin-provider-libc/contracts/libc/{stdio,errno}.qcore`
    (appended sections), the READMEs' module counts;
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/reports/INDEX.md`, `docs/development/increments/wave-44.md`.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/before c1e69bb5
sh docs/reports/w44-libc-residuals/scripts/identity.sh measure/before docs/reports/w43-indexed-resource-collections/integration/artifacts measure/before-identity.txt
python3 docs/reports/w43-libc-printf-family/scripts/capture_run.py measure/capture-run
python3 docs/reports/w43-libc-result-alias/scripts/strip.py
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py measure/before/resource-diagnostics.json measure/capture/ppP measure/unit-texts
python3 docs/reports/w44-libc-residuals/scripts/leaf_census.py measure/before/resource-diagnostics.json <out> <names>
python3 docs/reports/w44-libc-residuals/scripts/header_inline_census.py measure/capture/pp <out>
python3 docs/reports/w44-libc-residuals/scripts/tail_nested_lines.py measure/before/resource-diagnostics.json measure/unit-texts <out>
cargo build --release -p qcore-c-gateway --example declared_abi_census
target/release/examples/declared_abi_census cycle19-resource-contracts.json measure/capture/plain/*.i
target/release/examples/declared_abi_census docs/reports/w44-libc-residuals/measurement/residuals8.qcore -- measure/capture/plain/*.i
# (PREDICTION.md frozen and committed here; then, per stage, the code commit's release build)
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/stage0-reader c3a53240
python3 docs/reports/w44-libc-residuals/scripts/stage_run.py measure/stage1b-tail 0a75d66e <contract files>
python3 docs/reports/w44-libc-residuals/scripts/stage_run.py measure/stage2-inline … ; … stage3-valist … ; … stage4-after …
python3 docs/reports/w44-libc-residuals/scripts/cmp_runs.py measure/before measure/stage0-reader
python3 docs/reports/w44-libc-residuals/scripts/attribute_tail.py measure/stage0-reader measure/stage1b-tail measure/unit-texts <out>
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/stage1b-tail measure/stage2-inline measure/unit-texts <out> providers/quirin-provider-libc/contracts/libc/byteswap.qcore
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/stage2-inline measure/stage3-valist measure/unit-texts <out> docs/reports/w44-libc-residuals/measurement/stage3-names.qcore
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/stage3-valist measure/stage4-after measure/unit-texts <out> docs/reports/w44-libc-residuals/measurement/stage4-names.qcore
python3 docs/reports/w44-libc-residuals/scripts/compose_stages.py measure/before measure/stage4-after <out> <stage1b> <stage2> <stage3> <stage4>
python3 docs/reports/w44-libc-residuals/scripts/tail_residual.py measure/stage1b-tail/resource-diagnostics.json measure/unit-texts <out>
python3 docs/reports/w44-libc-residuals/scripts/valist_callers.py measure/before/resource-diagnostics.json measure/stage4-after/resource-diagnostics.json <out>
python3 docs/reports/w44-libc-residuals/scripts/abi_summary.py docs/reports/w44-libc-residuals/abi-evidence.json manifest-base=… manifest-reader=… …
python3 docs/reports/w43-libc-contracts/scripts/archive.py measure/stage4-after docs/reports/w44-libc-residuals/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w44-libc-residuals/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w44-libc-residuals/artifacts 30
python3 docs/reports/w43-libc-printf-family/scripts/newly_sole.py docs/reports/w43-indexed-resource-collections/integration/artifacts docs/reports/w44-libc-residuals/artifacts <out>
python3 docs/reports/w44-libc-residuals/scripts/publish_evidence.py
python3 docs/reports/w44-libc-residuals/scripts/update_ledger.py
python3 docs/reports/w44-libc-residuals/scripts/run_gate.py measure/gate <steps>
```

The private raw outputs are in this worktree's `measure/`.
