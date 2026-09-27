# W44-LIBC-RESIDUALS: header-inline helpers, `va_list`, literal tails and nested tail callees

Card W44-LIBC-RESIDUALS, 2026-09-25. It started on base `c1e69bb5` and was
re-measured on main `5355998d`, after merging W44-INDIRECT-CHAIN-ACCESS and
W44-CORPUS-CALLEE-SUMMARIES. It works the four libc-side residuals beside the
largest remaining libc leaf masses, measuring each before building anything:

1. glibc's header-inline `__bswap_16/32/64` (the two largest single
   uncontracted callees) and the other header-inline scalar helpers the census
   finds;
2. `LIBC-PRINTF-VA-LIST`: the `va_list` printf members;
3. `LIBC-PRINTF-TAIL-ACTUALS` and the `arguments beyond the declared
   parameters` family;
4. `LIBC-PRINTF-NESTED-TAIL-CALLEES`.

A [prediction](PREDICTION.md) was frozen and committed (`dca6f959`) before any
edit and any staged run. Its addendum for the re-measured route was committed
before any stage output of that route was read.
[§7](#7-prediction-against-outcome) compares both with the outcome.

**Two routes for item 1, one shipped.** The card first *declared* the six
helpers in a `libc/byteswap.qcore` (route a, arbiter on `c1e69bb5`). Card
W44-CORPUS-CALLEE-SUMMARIES then measured that their `static __inline` bodies
already validate under W4 and were held back only by its reserved-name filter.
A validated body is a check of what the unit runs; a declaration is an
assumption about it. So the shipped route (b) drops the declaration and admits
the bodies. Route (a)'s arbiter and attribution are kept under
[route-a/](route-a/), but its artifacts were never a baseline and are not
kept. Its numbers were identical to route (b)'s, leaf for leaf.

## Headline

* **Item 1: checked, not declared.** `legacy_callee_scope.rs` no longer
  excludes reserved names. A reserved-name `static` definition the unit
  carries enters W4's derivation scope on the linkage premise alone, and a
  name the unit only declares never does (Lean `QCore.CalleeSummaryScope`).
  The six helpers (`__bswap_16/32/64`, `__uint16/32/64_identity`) are one
  identical pure-arithmetic body each in all 174 defining units, and they
  validate as `Borrowed -> None`. **3 functions move Unknown → Safe**
  (`ut_put_be16/32/64`). No libc declaration of any of the six is shipped.
* **Item 2: read and declared.** The reader projects `__builtin_va_list` at a
  parameter to a new loader spelling, `va_list`, with Lean parity
  (`QCore.GnuBuiltinReader`). It is borrowed-only, a parameter type only, and
  target-independent. All four members agree in all **168** units;
  `vsnprintf` and `vprintf` are declared (the other two are never called). 26
  leaves go, 1 comes back, and no function moves: 24 of the 26 callers keep
  `__builtin_va_start`/`__builtin_va_end` leaves.
* **Item 3: a sound narrow widening, built.** **T6** admits a tail actual that
  is only literals and C11 6.4.2.2 predefined identifiers, whatever the
  function's typed declarations or C02-P's closure say (Lean
  `QCore.LiteralTailActual`). Measured first: **no function** could move. It
  removes **974** of the 1,200 tail refusals and adds none.
* **Item 4: measured, one libc callee declared.** There are 89 printf-nested
  refusals at the base. `strerror` is declared, and its 2 leaves go. The other
  87 are corpus callees, inner calls inside a larger expression, or Cyrus SASL,
  and none of them blocks a function alone.
* **Arbiter on main `5355998d`.** BEFORE reproduces the
  W44-CORPUS-CALLEE-SUMMARIES integration diagnostics **byte for byte**. The
  reader stage alone is byte-identical to BEFORE in all five artifacts. End to
  end, **1,629 leaves are removed and 6 added**. Every one is attributed per
  stage, and the stages compose exactly
  ([compose.json](attribution/compose.json)). Units do not move. Leak stays
  **271**. **Violation candidates: none.**
* **Follow-ups 358 → 356.** Five retired, three registered, each with its size.

| Measure | BEFORE (W44-CORPUS-CALLEE-SUMMARIES integration) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 296 / 85 / 4,453 | **299 / 85 / 4,450** | +3 / 0 / −3 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 43 / 135 / 4 | 0 |
| `unsupported` | 45,721 | **44,098** | −1,623 |
| `unknown_alias` | 4,420 | 4,420 | 0 |
| `leak` / `summary_conflict` / `double_release` | 271 / 1 / 0 | 271 / 1 / 0 | 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole (Unknown functions) | 9,505 / 2,579 / 406 | 8,869 / 2,565 / 403 | −636 / −14 / −3 |
| Unknown functions blocked only by one uncontracted callee | 300 (229 callees) | 297 (226 callees) | −3 |
| tail refusals leaves / functions / sole (Unknown functions) | 1,014 / 246 / 1 | 213 / 125 / 1 | −801 / −121 / 0 |
| nested-call refusals leaves / functions / sole (Unknown functions) | 136 / 91 / 5 | 134 / 89 / 5 | −2 / −2 / 0 |
| indirect-access refusals leaves / functions / sole (Unknown functions) | 17,232 / 3,278 / 409 | 17,232 / 3,278 / 410 | 0 / 0 / +1 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 111 / 47 / 152 | 111 / 47 / **155** | 0 / 0 / +3 |

The census rows count Unknown functions, as
[blocker-census.txt](blocker-census.txt) does. The baseline's rows come from the
same script run over the baseline artifacts. The brief's 279 + 247
`__bswap_32`/`__bswap_16` leaves and its 1,014 / 246 tail figure are these
Unknown-function counts; over all functions the families are 599 and 1,200
leaves.

## 1. The census, before anything was built

Four scripts ran over the first BEFORE (`c1e69bb5`) and over a private capture
of the 185 units: a tee wrapper around the base binary, giving 180 paired
`-E -P` texts and the same 5 unpaired units as every wave-43 card. The family
figures below were recomputed on the `5355998d` BEFORE and are unchanged
([census-leaves-before.json.gz](census-leaves-before.json.gz),
[census-tail-nested-before.json.gz](census-tail-nested-before.json.gz)).

* [leaf_census.py](scripts/leaf_census.py) counts each family per callee and
  bounds its function yield: the non-Safe functions whose **every** leaf the
  family's admission would remove.
* [header_inline_census.py](scripts/header_inline_census.py) reads every
  `static`/`extern` inline definition in a `/usr/include` header and every
  main-source call of one
  ([census-header-inline.json](census-header-inline.json)).
* [tail_nested_lines.py](scripts/tail_nested_lines.py) reads every tail and
  nested refusal against its analysed line.
* The production `declared_abi_census` runs over the measurement modules in
  [measurement/](measurement/) and over the whole manifest
  ([abi-evidence.json](abi-evidence.json)).

| Family at BEFORE | Leaves | Functions | Bound: non-Safe functions whose every leaf it holds |
|---|---:|---:|---:|
| header-inline callees (`__bswap_32` 286, `__bswap_16` 252, `__bswap_64` 60, `__uint64_identity` 1) | 599 | 99 | 3 (`ut_put_be16/32/64`) |
| `vsnprintf` 25 + `vprintf` 1 callee leaves | 26 | 26 | 0 |
| tail refusals (`fprintf` 1,093, `snprintf` 78, `mk_print` 21, `printf` 8) | 1,200 | 263 | 1 (`cb_debug_chunk_complete`) |
| printf-family nested refusals | 89 | 62 | 0 |

The one tail-bound function is monkey's `cb_debug_chunk_complete`. It prints
`buf`, a live `mk_mem_alloc` allocation, so admitting it takes a tail admission
of a tracked identity. That is not the narrow widening the ledger names, so T6
was forecast to move no function, and it moved none.

## 2. Item 1: header-inline helpers

### 2.1 The decision

The brief offered three ways: declare the helpers in `quirin-provider-libc` as
the neutral element and say they are header-inline bodies, let the analyzer
project their bodies, or refuse them. **The bodies are checked.**

* **What the census found.** Each of the 174 units that defines these helpers
  carries exactly one (signature, body) pair per helper, from
  `/usr/include/x86_64-linux-gnu/bits/byteswap.h` and `bits/uintn-identity.h`.
  None calls anything, takes an address, or touches an object other than its
  parameter.

| Helper | Body (tokens, all 174 units) | Called in (units / main-source sites) |
|---|---|---:|
| `__bswap_16` | `return ((__uint16_t) ((((__bsx) >> 8) & 0xff) \| (((__bsx) & 0xff) << 8)));` | 13 / 253 |
| `__bswap_32` | four masks and shifts of `__bsx` | 16 / 287 |
| `__bswap_64` | eight masks and shifts of `__bsx` (`__extension__ static __inline`) | 8 / 80 |
| `__uint16_identity`, `__uint32_identity`, `__uint64_identity` | `return __x;` | 1 / 1, 2 / 3, 1 / 1 |

* **Why checked rather than declared.** Projecting the bodies as reported
  functions would cross E01-B2: the function denominator is the main-source
  definitions. W44-CORPUS-CALLEE-SUMMARIES had already built the route that
  keeps the denominator and widens only W4's *derivation* scope, and its census
  showed these bodies validate there. Its reserved-name filter held them out
  for ownership reasons only ("the libc provider's decision"). This card is
  that owner. A validated summary is a statement about the body the unit
  carries and every call runs. A declaration would have been an assumption
  about it, and it would have won over the validated body ("declared contracts
  still win").
* **Route (a), measured and dropped.** The card first declared the six in a
  `libc/byteswap.qcore`, with a reader change to discard `__extension__` for
  `__bswap_64`. Its arbiter on `c1e69bb5` ([route-a/](route-a/)) decided the
  same 627 leaves and moved the same 3 functions. The module, its manifest
  line, its `CONTRACTS` entry and its pins are removed. So is the `__extension__`
  widening and its Lean section, which nothing else measurably needed.

### 2.2 The rule, and why it is sound

`derivation_scope` admits an unselected definition that a selected one
reaches, and that has **internal linkage** read off its own definition
(`static` present, `extern` absent). C11 6.2.2p3 then makes every call to it in
the unit denote this body. The reserved-name conjunct is dropped: reservation
(C11 7.1.3) constrains who may *declare* a name, and it changes nothing about
which body a call to a `static` definition runs.

The lift is **only where the unit carries the body.** A candidate is a
definition in the unit's own token stream. A reserved name the unit only
declares or calls is never a candidate, so its calls stay uncontracted. The
card did not restrict the lift to system-header bodies. The token stream the
certificate checker re-reads has no header marks, and the linkage premise
already carries the soundness argument. Everything downstream is unchanged:
`observations_match_contract` publishes a summary only after re-analysing the
body; a declared contract still wins; the declared-mode gate still compares.
`quirin-provider-libc` declares none of the six names, so a declaration and a
derived summary never both apply to one of them.

Pinned by two re-pinned unit tests in
`crates/qcore-c-gateway/tests/unit/legacy_callee_scope.rs`:

* `a_reserved_static_definition_is_admitted_only_where_the_unit_carries_it`:
  `__bswap_32` and `_Upper` are admitted. A declaration-only
  `__declared_only` and an `extern inline` `__external_inline` are not.
* `a_reserved_helper_is_checked_where_the_unit_carries_it`: a `static`
  `__release` body discharges the acquisition. With only a declaration, the
  call stays uncontracted.

**Lean parity** (`proof/QCore/CalleeSummaryScope.lean`, audited in
`proof/AuditCalleeSummaryScope.lean`):

* `candidate` drops `!d.reserved`;
* `reservation_is_not_read` states it as an equation;
* `an_undefined_name_is_never_in_scope` is the carried-body condition;
* the witness now admits a reached static `__bswap_32` and refuses a
  reserved name the unit only calls;
* `every_scope_member_is_selected_or_internal`,
  `an_admitted_callee_runs_only_its_analysed_body` and the installation
  theorems are unchanged in statement, reproved over the wider candidate set;
* `an_unselected_reserved_definition_is_never_in_scope` is removed, because it
  stated exactly what this card reverses.

### 2.3 Effect (stage 2)

[attribution/stage2.json.gz](attribution/stage2.json.gz) is produced by
[`attribute_scope.py`](scripts/attribute_scope.py): W43's `attribute.py`, with
the decided set read from
[measurement/header-inline-names.txt](measurement/header-inline-names.txt). It
reports `attribution_complete: true`.

| Class | Removed | Added |
|---|---:|---:|
| summarised-callee: `__bswap_32` 286 (87 functions), `__bswap_16` 252 (84), `__bswap_64` 60 (33), `__uint64_identity` 1 | 599 | — |
| newly-safe-callee: `ut_put_be32` 15, `ut_put_be64` 8, `ut_put_be16` 4, in `ut_build_valid_msgset_v2` (12), `ut_write_msgset_v2_header` (12) and three `unittest_msgset_*` (1 each) | 27 | — |
| same-line: `nested call __uint16_identity` in `emit_copy_less_than64`, exchanged for `resource call with a formal embedded in an unmodeled expression` | 1 | 1 |
| same-line: `resource call with a formal embedded in an unmodeled expression` at `(&x)->len = (int16_t)__bswap_16 (_v);` in `rd_kafka_mock_handle_ConsumerGroupHeartbeat`, `…ShareGroupHeartbeat` and `rd_kafka_buf_read_NodeEndpoints` (2) | — | 4 |

No uncontracted-callee leaf of any other reserved name moved, so no other
reserved-name body decided a leaf.

The route (a) forecast of about 520 exchanged leaves did not happen. The 412
`*_vp = __bswap_32 (_v);` and `*_vp = (int16_t)__bswap_16 (_v);` sites in
librdkafka's buffer-read macros store through a pointer to a local scalar.
W43-INDIRECT-ACCESS-GENERAL's typed scalar-access rule already admits those
once the call has a contract or summary.

**The three functions that move Unknown → Safe.** Their callees are now
*validated summaries* of the unit's own header bodies, not declared premises:

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
it, an array of it and a result of it still refuse. Nine new rows in
`crates/qcore-c-gateway/tests/loader_abi.rs` and a result-position test
(`a_va_list_result_has_no_spelling`) pin all of this. `compiler/tests/contract_loader.rs`
pins the spelling: borrowed loads, every custody mode and a `va_list` result
are rejected.

### 3.2 Lean parity

`proof/QCore/GnuBuiltinReader.lean`, registered in
`proof/AuditGnuBuiltinReader.lean` (20 theorems), with `AbiType` gaining
`vaList` in `QCore.DeclaredAbiAgreement`:

* **conservativity**: `the_va_list_widening_only_adds`, `resolution_is_conservative` (a chain that
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
stage run on the first base, `c1e69bb5`
([route-a/runs/stage1-narrow-run.json](route-a/runs/stage1-narrow-run.json),
[route-a/attribution/stage1.json.gz](route-a/attribution/stage1.json.gz)), removed **86**
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

[attribution/stage1.json.gz](attribution/stage1.json.gz), by
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
corpus callees are unchanged by merging W44-CORPUS-CALLEE-SUMMARIES: they are
external-linkage definitions (`rd_kafka_err2str` is librdkafka's public API),
which that card's derivation scope does not admit. A validated W4 summary or a
read declaration of one with a `none` result and borrowed parameters is
admitted by the unchanged premises. The residual is registered as
`W44-PRINTF-NESTED-INNER-CALLEES`.


## 6. Arbiter

Each stage runs a saved release binary through
[`scripts/run_binary.py`](scripts/run_binary.py), or through
[`scripts/run_stages_v2.py`](scripts/run_stages_v2.py) when several run
concurrently. The command shape and environment are those of
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py):
`PIN=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19`,
`QCORE_CLANG=/usr/bin/clang` and the wave-20 ChunkIO package. The one change
is that the binary is named, so each stage's binary could be built once and
saved.

Each run records the SHA-256 of the manifest, the load path and the two
changed libc modules, before and after the run. A stage whose files changed
while it ran is refused. Every stage recorded `unchanged_during_run: true`.

The stage binaries were built from the merged tree, with the stage's later
analyzer files checked out from main and then restored from git:

* BEFORE: main's `legacy_declared_abi.rs`, `contract_loader.rs`,
  `variadic_nested.rs` and `legacy_callee_scope.rs`;
* stage 0: the first two from HEAD;
* stage 1: the first three from HEAD;
* stages 2–4: all four from HEAD.

With those four files from main, the analyzer sources of the BEFORE build
equalled main's exactly (`git diff main -- crates compiler/src` showed test
files only). Stages 0–2 read main's contract files; stage 3 adds this card's
`stdio.qcore`, and stage 4 its `errno.qcore`.

| Run | Binary (sha256) | Code | Contracts | Exit | Window (UTC, 2026-09-25) | Record |
|---|---|---|---|---:|---|---|
| BEFORE | `153ba825…` | main `5355998d` | main | 0 | 03:07–03:18 | [runs/before-run.json](runs/before-run.json) |
| stage 0, `va_list` reader | `f0f3ade9…` | + reader and loader | main | 0 | 03:07–03:18 | [runs/stage0-run.json](runs/stage0-run.json) |
| stage 1, T6 | `a3a0a02f…` | + tail rule | main | 0 | 03:07–03:18 | [runs/stage1-run.json](runs/stage1-run.json) |
| stage 2, header bodies | `5eda648d…` | + scope lift = HEAD | main | 0 | 03:07–03:18 | [runs/stage2-run.json](runs/stage2-run.json) |
| stage 3, `va_list` members | `5eda648d…` | HEAD | + `stdio.qcore` | 0 | 03:19–03:28 | [runs/stage3-run.json](runs/stage3-run.json) |
| stage 4 = AFTER | `5eda648d…` | HEAD | + `errno.qcore` = committed | 0 | 03:28–03:37 | [runs/stage4-run.json](runs/stage4-run.json), [artifacts/](artifacts/) |

BEFORE and stages 0–2 ran concurrently, from four separate binaries against
the same on-disk contracts.

**Source labels, disclosed.** Stages 3 and 4 record their source as
`09e21484`. That was a commit made while the runs were in flight which wrongly
included the stages' temporary main contract texts. It was undone at once and
recommitted as `db1205fa`, and it is on no branch. The analyzer sources of
`09e21484`, `db1205fa` and `32f6c3e9` are identical, which is why stages 2–4
share one binary digest. Each stage's actual contract bytes are the digests
in its `runs/*-files.json`, and stage 4's equal the committed files.

**BEFORE vs the baseline**
([runs/before-identity.txt](runs/before-identity.txt), `baseline_identity.py`
with this worktree aliased to the integrator's root):

* diagnostics, chunk-contracts and source are **byte-identical**;
* `contracts` is identical once the checkout root is aliased;
* `source-effects` differs only in the `resource_contracts:<digest>` key and in
  the `build_digest`, `configuration_digest`, `task_digest`, `obligation_id`
  and `coverage[0]` values that bind it. These are the legitimate
  checkout-path differences the brief lists.

**Stage 0 vs BEFORE:** all five artifacts are byte-identical
([runs/stage0-vs-before.json](runs/stage0-vs-before.json)). This is the reader
change alone. The manifest census moves 0 of its 13,101 (unit, name) rows
([abi-evidence.json](abi-evidence.json), `reader_change_alone`).

**AFTER vs BEFORE** ([runs/after-vs-before.json](runs/after-vs-before.json),
[runs/contracts-diff.txt](runs/contracts-diff.txt)):

* chunk-contracts, source-effects and source are byte-identical;
* `contracts` differs in the source text of `stdio.qcore` and `errno.qcore`
  only, and the manifest is unchanged.

**Composition.** [`compose_stages.py`](scripts/compose_stages.py)
([attribution/compose.json](attribution/compose.json)) recomputes the
BEFORE → AFTER leaf delta and the signed sum of the four stages' attributed
rows. They are equal: **1,629 removed, 6 added**.

* Every stage reports `attribution_complete: true`.
* Every unit whose fields move changes only its `diagnostic_counts` and its
  count-summary `reason`, each by exactly its own functions' leaf delta.
* No top-level field other than the counts moves.

| Stage | Removed | Added | Function moves | Leak |
|---|---:|---:|---|---:|
| 0 `va_list` reader | 0 | 0 | none | 271 |
| 1 T6 | 974 | 0 | none | 271 |
| 2 header bodies | 627 | 5 | `ut_put_be16/32/64` Unknown → Safe | 271 |
| 3 `va_list` members | 26 | 1 | none | 271 |
| 4 `strerror` | 2 | 0 | none | 271 |
| **BEFORE → AFTER** | **1,629** | **6** | **3 Unknown → Safe** | **271** |

**Leak pin (constraint 7).** Unmoved at **271**, with no leak leaf added or
withdrawn at any stage:

* T6 removed 974 refusals in 209 functions, and none of them released a
  poisoned binding into a new leak;
* the admitted header bodies are `Borrowed -> None`;
* every declaration borrows every argument and releases nothing.

The new baseline is [artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json)).
`docs/reports/w44-corpus-callee-summaries/integration/artifacts/` is its frozen
predecessor and is not edited.

**Violation candidates: none.** `double_release`, `invalid_release` and
`use_after_release` are 0 before and after. No provisional `Unsafe` is added,
and none is presented as an upstream defect. In the sole-blocker set
([sole-moves.json](sole-moves.json)), the three movers leave and none enters.

## 7. Prediction against outcome

**The first prediction** (route a, base `c1e69bb5`, compared with the route-a
arbiter in [route-a/](route-a/)):

| Item | Predicted | Measured | |
|---|---|---|---|
| Reader alone | 5 artifacts identical; 0 of 13,101 census rows | identical; 0 of 13,101 | ✔ |
| T6 removed | 1,050 (850–1,091) | 974 (86 in the first, unreadable-only form) | ✔ final, ✘ first form |
| T6 added / moves / leak | 40 (0–200) / 0 / 271 | 0 / 0 / 271 | ✔ |
| Header-inline census | 6 × 174 agree | 6 × 174 agree | ✔ |
| Header-inline removed | 600 | 627 (599 + 27 newly-safe-callee + 1 nested) | ✔ within the spirit, 27 more than the point |
| **Header-inline added** | **520 (400–600)** | **5** | ✘ |
| Header-inline moves | `ut_put_be16/32/64` (2–6) | exactly those three | ✔ |
| `va_list` | 168/168; −26; +12 (4–26); 0 moves | 168/168; −26; +1; 0 moves | ✔ removed and moves, ✘ added below range |
| `strerror` | 181 units; −2; +0–2; 0 moves | 181; −2; +0; 0 | ✔ |
| Functions | 297 / 85 / 4,452 | 297 / 85 / 4,452 | ✔ |
| `unsupported` | ~44,700 (44,300–45,200) | 44,225 | ✘ just below range |
| Units, `unknown_alias`, leak, candidates | unchanged, ±30, 271, 0 | unchanged, 0, 271, 0 | ✔ |

**The route (b) addendum** (main `5355998d`): every row held. BEFORE was
byte-identical. Stage 0 was identical. T6 gave −974/+0 (range 950–990).
Stage 2 gave −599 helper leaves, −27 newly-safe-callee, −1 nested and +5, with
exactly `ut_put_be16/32/64` moving. Stages 3 and 4 gave −26/+1 and −2. The
functions are 299 / 85 / 4,450, and units, leak and `unknown_alias` are
unchanged. The addendum was written after route (a) had measured the same
exchange, so its accuracy is not independent evidence.

**Where the first forecast failed.**

* **The tail estimate read a line, not a call.** One `RDUT:` macro line carries
  several `fprintf` calls, and the census counted a leaf as removable when its
  line held any literal-only call. It did not see the closure premise at all.
  The first form of T6, built on the readability premise the W43 probe had
  named, removed 86. The closure premise was found by reading what remained,
  and the rule was completed in the direction the ledger had already named.
* **The header-inline exchange assumed the stores would refuse.** Most
  `*_vp = __bswap_32 (_v);` stores were already admissible once the call was
  contracted, through W43-INDIRECT-ACCESS-GENERAL's typed scalar-access rule.
  The prediction did not check that rule against these shapes.

## 8. Validation

Measured in this worktree (`agent-a12a2b7db791da414`) by
[`scripts/run_gate.py`](scripts/run_gate.py). That is W43-LIBC-RESULT-ALIAS'
runner with two additions: `CARGO_INCREMENTAL=0` and a `docsindex` step. It
sets `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`, turns debug info off, and
captures every exit to a file, never through a pipe.

The core `cargo test` step ran on `db1205fa`, which is route (b)'s code, pins
and Lean. `cargo clean --profile dev` followed once it was recorded (6.3 GiB
removed). The remaining core steps and the whole provider gate ran after
`c9337006`, which changed only documents, the ledger, report scripts and
evidence. The generator input is [gate.json](gate.json), with the
[Clang recording](clang-version.txt) taken in this checkout beside it. Logs are
worktree-private under `measure/gate2/` and `measure/provider-gate2/`. At least
40G was free before each long build: 182G before the stage builds, 197G after
the clean.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate2/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate2/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,435 passed / 0 failed / 11 ignored**, 288 summaries | `measure/gate2/test.log` |
| `cargo clean --profile dev` | 0 | 2,278 files, 6.3 GiB | `measure/logs/clean-dev.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate2/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate2/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate2/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate2/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **181** audits | `measure/gate2/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w44-libc-residuals/gate.json` | 0 | 356 follow-ups, 111 contract modules, 181 Lean audits | `measure/logs/build-matrix.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/logs/docs-index-check.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests | `measure/logs/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate2/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate2/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate2/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate2/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | inventories and load path unchanged against main | `measure/provider-gate2/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate2/packages` | 0 | 5 packages | `measure/provider-gate2/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5: chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 ×5 | | `measure/provider-gate2/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 13 tests | `measure/provider-gate2/ppython.log` |

**Counts against main.** Main's W44-CORPUS-CALLEE-SUMMARIES integration gate
records 5,432 core tests, 124 provider tests, 330 Python tests and 179 Lean
audits.

* **Core tests:** this card adds two test functions,
  `a_va_list_result_has_no_spelling` (`loader_abi.rs`) and
  `a_literal_tail_is_data_even_where_the_closure_marked_literals_risky`
  (`variadic_nested`). It renames four re-pinned ones, which count unchanged.
  The third test of the +3 difference was not traced.
* **Lean audits:** it adds two, `AuditGnuBuiltinReader` and
  `AuditLiteralTailActual`.
* **Load-sensitive tests:** no failure occurred.
  `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` passed in the full
  run, so no isolated rerun was needed.

**Attempts that are not the gate, disclosed.**

* **Route (a)'s gate** (`measure/gate-stopped-route-b/`) was stopped by hand
  during its `cargo test` step, when the item-1 route changed. It has no
  recorded exit and is not evidence.
* **Focused runs** before the gate are measurements, not gate steps. Each
  passed: `legacy_callee_scope` (13), `loader_abi` (15), `contract_loader`
  (10), `qcore_contract_corpus` (4), the re-pinned gateway tests, and
  `test_contract_corpus` (7).
* **Stage binaries:** four were built by checking later analyzer files out
  from main, and each file was restored from git afterwards. §6 records this.

## 9. Ledger and status

* **Retired**, each preserved verbatim with its ground in
  [retired-followups.json](retired-followups.json):
  * `W43-LIBC-HEADER-INLINE-BODIES`: the bodies are derived by W4;
  * `W44-CALLEE-SCOPE-RESERVED-BODIES`: W44-CORPUS-CALLEE-SUMMARIES' hand-off,
    decided and lifted;
  * `LIBC-PRINTF-VA-LIST`;
  * `LIBC-PRINTF-TAIL-ACTUALS`, by its arm (a);
  * `LIBC-PRINTF-NESTED-TAIL-CALLEES`: the libc half is discharged, and the
    rest is re-registered below with its premises.
* **Registered** ([followups.json](followups.json)), sized on the AFTER run:
  * `W44-TAIL-NAME-ACTUALS`: 226 leaves in 132 functions, bound 1;
  * `W44-PRINTF-NESTED-INNER-CALLEES`: 87 leaves in 60 functions, bound 0;
  * `W44-VA-START-END-BUILTINS`: 127 leaves in 51 functions; one function
    (`rd_kafka_set_thread_name`) has no other blocker.
* **Follow-ups 358 → 356.** `corpus.cycle19_raw_units` points at this
  report's artifacts, and its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* `docs/STATUS.md`'s projected-function, corpus-diagnostic, indirect-access,
  contract-corpus and follow-up rows are updated from this arbiter, and
  `docs/reports/INDEX.md` is regenerated.
* `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.1 gains the `va_list` vocabulary.
* `providers/quirin-provider-libc/README.md` and `providers/README.md` now
  count 16 libc modules. They said 15, which omitted
  W43-DESCRIPTOR-RESOURCE-TRACKING's `dirent`.

## 10. For the integrator

* **Constraint 7:** the pin stays at 271. The card makes 3 declarations, one
  scope admission (reserved-name header bodies), one tail admission (T6) and one
  reader widening (`va_list`), and moves 3 functions.
* **Constraint 8:** the reader paragraph can record that `__builtin_va_list`
  reads as the parameter-only `va_list` spelling, with the shadowing rule. The
  rule text is left to the integrator, as predecessors did.
* **Carried forward, not edited:**
  * `W43-LIBC-ABI-READER-SHAPES` stays open. Its third shape, `va_list`, is
    discharged here; its transparent-union and enum-typedef shapes are
    unchanged.
  * `MK-PRINT-TAIL-ACTUALS` stays open, with 18 `mk_print` sites (21 at the
    base), all inside `W44-TAIL-NAME-ACTUALS`.
* **W44-CORPUS-CALLEE-SUMMARIES' report** still describes the reserved filter
  as in force. It is frozen evidence of that card and is not edited here. The
  current rule is in `legacy_callee_scope.rs` and `QCore.CalleeSummaryScope`.
* **Composition:**
  * **W44-INDIRECT-CALL-ARGUMENT-ACCESS:** T6 admits no indirect read. It is
    one disjunct in `tail_arguments_are_data_with_nested`, and every other
    disjunct is unchanged.
  * **Descriptor custody:** no new `LegacyInstruction` variant is introduced,
    so `descriptor_custody::admits_descriptor_read` is untouched. A
    T6-admitted actual is a literal, which never carries a descriptor, and the
    admitted header bodies are scalar-only.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_callee_scope.rs`: the candidate filter,
    the module doc, and the removed `is_reserved_identifier`. Its unit tests
    are in `crates/qcore-c-gateway/tests/unit/legacy_callee_scope.rs` (two
    re-pinned).
  * `crates/qcore-c-gateway/src/variadic_nested.rs`: one disjunct and one early
    return in `tail_arguments_are_data_with_nested`, the T6 helper, and a
    re-pinned unit test.
  * `crates/qcore-c-gateway/src/legacy_declared_abi.rs`: one `project_scalar`
    arm, the `BUILTIN_VA_LIST` constant and the import.
  * `crates/qcore-contract/src/contract_loader.rs`: `VA_LIST_ABI_TYPE` and the
    two vocabulary iterators.
  * Lean:
    * `proof/QCore.lean` (two imports);
    * `proof/QCore/DeclaredAbiAgreement.lean` (`AbiType.vaList`);
    * `proof/QCore/NativeLibraryHeader.lean` and its audit (a renamed theorem);
    * `proof/QCore/CalleeSummaryScope.lean` and its audit.
  * Pins:
    * `compiler/tests/qcore_contract_corpus.rs`: five clause pins;
    * `compiler/tests/contract_loader.rs`: (47, 152, 216) → (47, 155, 216),
      three parity entries, and the `va_list` vocabulary checks.
  * Contract modules: `providers/quirin-provider-libc/contracts/libc/{stdio,errno}.qcore`
    (appended sections).
  * Records: `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/reports/INDEX.md` and `docs/development/increments/wave-44.md`.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
# census on the first base (c1e69bb5), as §1
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/before c1e69bb5
python3 docs/reports/w43-libc-printf-family/scripts/capture_run.py measure/capture-run
python3 docs/reports/w43-libc-result-alias/scripts/strip.py
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py measure/before/resource-diagnostics.json measure/capture/ppP measure/unit-texts
python3 docs/reports/w44-libc-residuals/scripts/header_inline_census.py measure/capture/pp <out>
cargo build --release -p qcore-c-gateway --example declared_abi_census
target/release/examples/declared_abi_census cycle19-resource-contracts.json measure/capture/plain/*.i
target/release/examples/declared_abi_census docs/reports/w44-libc-residuals/measurement/valist4.qv -- measure/capture/plain/*.i
# stage binaries: build with the stage's later analyzer files checked out from main, save, restore
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit   # ×4, see §6
# BEFORE and stages 0-2 with main's stdio.qcore and errno.qcore on disk, then 3, then 4
python3 docs/reports/w44-libc-residuals/scripts/run_stages_v2.py measure/v2 before=5355998d=<main bin> stage0=…=<s0 bin> stage1=…=<s1 bin> stage2=…=<head bin>
python3 docs/reports/w44-libc-residuals/scripts/run_stages_v2.py measure/v2 stage3=…=<head bin>
python3 docs/reports/w44-libc-residuals/scripts/run_stages_v2.py measure/v2 stage4=…=<head bin>
sh docs/reports/w44-libc-residuals/scripts/identity.sh measure/v2/before docs/reports/w44-corpus-callee-summaries/integration/artifacts measure/v2/before-identity.txt
python3 docs/reports/w44-libc-residuals/scripts/cmp_runs.py measure/v2/before measure/v2/stage0
python3 docs/reports/w44-libc-residuals/scripts/attribute_tail.py measure/v2/stage0 measure/v2/stage1 measure/unit-texts <out>
python3 docs/reports/w44-libc-residuals/scripts/attribute_scope.py measure/v2/stage1 measure/v2/stage2 measure/unit-texts <out> docs/reports/w44-libc-residuals/measurement/header-inline-names.txt
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/v2/stage2 measure/v2/stage3 measure/unit-texts <out> docs/reports/w44-libc-residuals/measurement/stage3-names.qv
python3 docs/reports/w43-libc-contracts/scripts/attribute.py measure/v2/stage3 measure/v2/stage4 measure/unit-texts <out> docs/reports/w44-libc-residuals/measurement/stage4-names.qv
python3 docs/reports/w44-libc-residuals/scripts/compose_stages.py measure/v2/before measure/v2/stage4 <out> <stage1> <stage2> <stage3> <stage4>
python3 docs/reports/w44-libc-residuals/scripts/leaf_census.py measure/v2/stage4/resource-diagnostics.json <out> <names>
python3 docs/reports/w44-libc-residuals/scripts/tail_nested_lines.py measure/v2/stage4/resource-diagnostics.json measure/unit-texts <out>
python3 docs/reports/w44-libc-residuals/scripts/tail_residual.py measure/v2/stage4/resource-diagnostics.json measure/unit-texts <out>
python3 docs/reports/w44-libc-residuals/scripts/valist_callers.py measure/v2/before/resource-diagnostics.json measure/v2/stage4/resource-diagnostics.json <out>
python3 docs/reports/w44-libc-residuals/scripts/abi_summary.py docs/reports/w44-libc-residuals/abi-evidence.json manifest-base=… manifest-reader=… …
python3 docs/reports/w43-libc-contracts/scripts/archive.py measure/v2/stage4 docs/reports/w44-libc-residuals/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w44-libc-residuals/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w44-libc-residuals/artifacts 30
python3 docs/reports/w43-libc-printf-family/scripts/newly_sole.py docs/reports/w44-corpus-callee-summaries/integration/artifacts docs/reports/w44-libc-residuals/artifacts <out>
python3 docs/reports/w44-libc-residuals/scripts/publish_evidence.py
python3 docs/reports/w44-libc-residuals/scripts/update_ledger.py
python3 docs/reports/w44-libc-residuals/scripts/run_gate.py measure/gate2 <steps>
```

The private raw outputs are in this worktree's `measure/` (`v2/`, `attr2/`,
`census/`, `gate2/`; route (a)'s under `before/`, `stage*/`, `attr/`).
