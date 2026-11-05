# MEMBER-ACCESS-PROJECTION prediction

Committed before any gateway change, on base `96a3efd3b` (integrate/r3). The
census is [README.md](README.md) section 1. Rows are relisted at
`baseline:c8c7edf21` (`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`
in the main checkout, reports `eight-home/corpus/corpus`,
`fluent-bit-system/system` and `fluent-bit-tail-pause/slice`), the newest
published baseline; integrate/r3's own cards (release-window frames,
cfl-mpack and libc-gap declarations) are not in it and touch none of the rows
below.

> **USER REQUEST 2026-11-05** (verbatim): "We're still needed to handle tons
> of unsupported instructions on our corpus..."; then "Member access first".

## What the card will change

One reading defect of the census (README §1.3): a declaration with no
initializer whose specifier is a typedef name, or whose array bound is a
constant expression rather than a literal, is refused by the lexical
indirect-access gate (`rd_segment_t *seg, *tmp;` reads the second `*` as a
dereference; `uint32_t op[32];` and `char buf[1024 * 3];` never reach the
declaration check). The fix: where Clang's AST shows the statement is exactly
one declaration of automatic objects, none initialized and none variably
modified, the gate takes the keyword-specifier declaration branch, whose
lowering (`NoOp`) is unchanged. Nothing else moves; the member-custody
readings stop on a user choice (README §3).

## Rows (baseline `c8c7edf21`), found by `scripts/predict.py`

**Corpus.** 123 rows carry such a leaf (132 leaves); each loses it.

- **Two rows have no other diagnostic and are predicted Unknown -> Safe:**
  - `crc32c.c::crc32c_zeros_op` (`uint32_t odd[32];` is its only leaf; the
    rest of the body is scalar array stores and calls to the Safe
    `gf2_matrix_square`);
  - `rdkafka_sticky_assignor.c::ut_testConflictingPreviousAssignments`
    (`rd_kafka_group_member_t members[2];` is its only leaf: the body returns
    at `RD_UT_PASS()` before anything else).
- **One row is predicted to follow through the in-unit summary:**
  `crc32c.c::crc32c_zeros`, whose other leaf is the call to
  `crc32c_zeros_op` (its own `uint32_t op[32];` goes too). Its caller
  `crc32c_init_hw` passes file-scope arrays and is predicted to stay Unknown;
  a move of it to Safe is to be explained, not assumed.
- **The other 120 rows stay Unknown** on their other blockers.

**System.** `src/flb_engine.c::handle_output_events` (two configurations)
loses `uint64_t values[FLB_ENGINE_OUTPUT_EVENT_BATCH_SIZE];` and keeps its
other leaf. No move.

**Slice.** `src/flb_lib.c::flb_lib_config_file` (two configurations) loses
`char tmp[PATH_MAX + 1];` and keeps twelve other leaves. No move.

```measure-prediction
route corpus
move crc32c.c::crc32c_zeros_op Unknown -> Safe
move rdkafka_sticky_assignor.c::ut_testConflictingPreviousAssignments Unknown -> Safe
move crc32c.c::crc32c_zeros Unknown -> Safe
delta Safe +3
route system
none
route slice
none
```

No row may move to `Unsafe`. A move to `Safe` other than these three is a
finding to explain before landing.

## The user's `current` run (Fluent Bit 046accf6)

- **Indirect-access leaves removed: 141** (126 functions), the census's
  uninitialized declarations; 9,996 -> about 9,855.
- **Functions newly Safe: 2-3**, the same three as the corpus where the
  units are the same (`crc32c_zeros_op`, `ut_testConflictingPreviousAssignments`,
  `crc32c_zeros`).
- **Functions moved to another blocker:** none newly "indirect only"; the
  12 functions whose every indirect leaf is such a declaration keep their
  other families (alias, uncontracted calls) except the three above.
- **No unit changes status**: every unit holding these functions has other
  Unknown functions.
