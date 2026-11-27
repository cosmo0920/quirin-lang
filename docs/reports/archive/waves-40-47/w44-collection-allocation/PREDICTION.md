# W44-COLLECTION-ALLOCATION: frozen prediction

Frozen before any analyzer change, on base `c1e69bb5`, after the BEFORE run and
the `sizeof` census. This file is not edited after it is committed; the
outcome is compared with it in the report.

## Inputs

* **BEFORE** (`private/runs/before`, base binary `6379af44…`): diagnostics are
  byte-identical to `docs/reports/w43-indexed-resource-collections/integration/artifacts/`
  ([before-identity.json](before-identity.json)); functions 294 / 85 / 4,455,
  units 3 / 43 / 135 / 4, leak 271, unknown_alias 4,358, unsupported 45,848.
* **Census** ([census/sizeof-census.txt](census/sizeof-census.txt),
  [`scripts/sizeof_census.py`](scripts/sizeof_census.py)), over 180 paired
  units:
  * `sizeof` operand shapes: `(NAME)` 4,513, other 2,262, `(typedef)` 313,
    `(*NAME)` **288**, `(struct T)` 186, `(scalar type)` 123, member access
    122, `(*member access)` 100, `(scalar type *)` 62, `(typedef *)` 43,
    string literal 31, `(**NAME)` **4**, `(struct T *)` 3, bare `NAME` 3;
    `*NAME` unparenthesised **0**, `NAME[LIT]` / `(NAME[LIT])` **0**.
  * Owned-result allocations carrying a `sizeof`: `(*NAME)` naming the
    destination itself 190 (175 into a `T *`, 12 into a `T **`, 1 `(**NAME)`
    into a `T **`), `(struct T)` 120, `(typedef)` 107, `(*member access)` 21.
  * `indirect memory access` leaves whose line's only access is a
    dereference operand: **226** (224 on **211** Unknown functions, 2 on
    Unsafe), **4** Unknown functions blocked by nothing else
    (`rd_kafka_mock_request_copy`, `XXH32_copyState`, `XXH64_copyState`,
    `clear_env`). This reproduces the W43 card's 224 / 211 / 4.
  * By callee on the line: `rd_calloc` 122, `rd_malloc` 41, `memset` 29,
    `memcpy` 4 (all contracted); uncontracted `rd_tmpabuf_alloc0` 7,
    `LZ4_initStreamHC` 6, `__builtin_alloca` 4, `LZ4_initStream` 2,
    `rd_tmpabuf_new` 2, `qsort` 2, `LZ4F_malloc` 1, `rd_tmpabuf_write0` 1;
    no call 4, `_Generic` 1.
  * `C operator \`sizeof\` requires type/operand analysis`: 465 leaves (447 on
    117 Unknown functions). 410 of the Unknown ones sit on lines whose operand
    is a parenthesised name, overwhelmingly librdkafka's
    `size_t __len2 = (size_t)(sizeof(_v));` macro expansion: a cast around a
    scalar query, not a dereference operand. Only 1 Unknown function is
    blocked by this family alone.

## The planned rule

Only the lexical access gate of a statement whose first call is an ordinary
callee (not a type operator) changes. A `sizeof` operand `*NAME`, `**NAME`
(parenthesised or not) or `NAME[LITERAL]` is **unevaluated** (C11 6.5.3.4p2)
when every declaration of `NAME` in the function is a pointer declarator with
at least that many `*`, no array dimension, and a specifier list that resolves
to a type that cannot be variably modified. Such an operand is replaced by a
scalar marker for the gate only. Everything after the gate is the existing
classifier: the call's contract, its borrowed actuals, the nested-call rule and
the flow analysis decide. No transfer rule changes, and no `LegacyInstruction`
variant is added. The `C operator` family and the other shapes are not in
scope.

## Forecast

| Quantity | Forecast |
|---|---|
| The 226 reachable `indirect memory access` leaves | about 190 withdrawn outright (contracted `rd_calloc`/`rd_malloc`/`memset`/`memcpy`), about 25 replaced at the same line by `call to uncontracted function`, about 5 kept (no call, `_Generic`) |
| `unsupported` | falls by **150 to 224** (withdrawals net of substitutions and of refusals newly reached downstream) |
| `unknown_alias` | falls by **0 to 300**: an allocation that now binds its destination no longer poisons it, so later releases and borrows of it resolve |
| `leak` | **271 to 290**. Any rise is the accusing direction (a newly modelled allocation reaching an exit that does not release it) and is attributed leaf by leaf |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0, possibly 0–2 each |
| Functions Unknown → Safe | **2 to 4** (the four sole-blocked functions), none through a collection element node |
| Functions Unknown → provisional Unsafe | **0 to 6** (exposed leaks) |
| Functions Safe → other | **0** |
| Units | **0** moves (0–1) |
| Arrays that become resolved collections (an element operation off its fallback) | **0 to 3** (candidates: `rd_kafka_ListGroups_resp_cb`'s `grps`, `rd_kafka_mock_sharegroup_get_member_ids`'s `member_ids`, `rd_string_split`'s `arr`) |
| Element operations moving under the collection rules | **0 to 6** |
| Functions Safe through collection element nodes | **0** |
| Violation candidates | 0 to 6, all leaks, each provisional |
