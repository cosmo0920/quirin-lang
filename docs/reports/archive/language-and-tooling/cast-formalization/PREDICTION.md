# CAST-FORMALIZATION: prediction (frozen before implementation code)

Frozen at the commit that adds this file, after the BEFORE census and before
any hook that can change a verdict. The conversion classifier
(`cast_semantics.rs`) and the parity-only census hook were committed first
(`2598f6e4e`) as the census instrument: they publish nothing, and the BEFORE
corpus run with them is checked byte-identical to main's recorded
`resource-diagnostics.json` (`450a9e54…`).

## BEFORE (main `cb3085365` + the census instrument)

Corpus arbiter (`66910c10`, `--jobs 4`, binary `49439047…`):
**649 / 9 / 4,176**, `resource-diagnostics.json` `450a9e54…` (main's recorded
digest, byte for byte: the instrument publishes nothing); leak 40, summary
conflict 2, unknown_alias 11,482, unsupported 28,737.

Census ([census-before-corpus](artifacts/census-before-corpus.txt)): 16,743
explicit casts in 3,225 function bodies. 10,392 leaves in 2,273 functions sit
on a line holding a cast or starting a statement that does; 285 Unknown
functions have every leaf on such a line. The join is by line, so a
macro-expanded line holding many statements (librdkafka's `rd_kafka_buf_*`
macros) associates every leaf on it with every cast on it: the counts are an
upper bound on "a refusal whose statement holds a cast", not on "a refusal a
cast causes". `NULL` is itself a `null-pointer` cast: that class is 7,169 of the 16,743
casts and 5,447 joined leaves (2,671 at call actuals alone).

By class × position, the cells this card's hooks address (casts / leaves /
Unknown functions; the family most of the leaves carry):

| Hook | Cells | Casts | Leaves | Unknown fns |
|---|---|---|---|---|
| H-V access views | typed-view, byte-view, qualifier at member root, dereference, subscript base | 131 | 75 | 43 |
| H-V refused by name | punned-view at member root, dereference | 38 | 34 | 12 |
| H-U value positions | byte-view, typed-view, punned-view, qualifier at call actual, pointer step, initializer, assignment value | 1,735 | 1,290 | 771 |
| H-U variadic tails | scalar / byte-view at variadic tail | 691 | 2,081 | 204 |

(The H-U leaf counts are dominated by the line-join inflation: most of their
leaves are `indirect memory access`, `unresolved resource identity` and
uncontracted calls of the same macro lines.)

## What the card will implement

One conversion table (`cast_semantics::CONVERSION_TABLE`), refined from rule
11b's kind table by the operand's and the result's type classes, and four
consultations of it in route G, each asked only where an older route refused
(hard invariant 3), except the soundness repair:

1. **H-SR5, soundness, asked first** (`W45-SR5-SCALAR-READ-STAR-THROUGH-CAST`).
   Rule 5j's token reader counts a `*` written before a cast group that
   precedes a may-bind member or subscript root (`*(T *)q->f`): the cast is
   transparent to the dereference count. Only refuses more.
2. **H-V, access positions** (member root `((T *)p)->f`, dereference
   `*(T *)p`, subscript base `((T *)p)[i]`, in the typed access-path reader).
   The view is decided by the table instead of CG12's `void *`-only test: a
   `typed-view` (from `void *` *or a character pointer*) and a `byte-view`
   (to a character pointer) are views, read only over an untracked root or a
   never-bound load exactly as CG12's view (rule 11j's premises unchanged;
   `P-EFFECTIVE-TYPE` for a typed view); a `qualifier` conversion (the same
   pointee type) is transparent (the same object at the same type, so no view
   premise); a `punned-view`, a function/object conversion and every other
   class stay refused.
3. **H-U, value positions** (call actuals, variadic tails, pointer-step
   operands, initializers, assignment values, conditions). A refused
   statement whose explicit casts of a plain object name include
   identity-preserving pointer conversions (`byte-view`, `typed-view`,
   `punned-view`, `qualifier`) at a value position, or a `scalar` conversion
   of a non-descriptor scalar name in a variadic tail, is lowered as the
   uncast statement by the older routes, exactly as rule 11b(3) lowers a
   top-level pointer-destination cast, and only if they admit it. A cast at
   an access position is never removed (its type is the access's).
4. **H-T, the existing readings consult the table** (no verdict change
   intended): the elaborator's object-pointer `BitCast`/`NoOp` arm asks the
   class and refuses a non-preserving class by name; rule 11b(3)'s uncast
   operand and (4)'s integer return read the class.

## Prediction

Corpus, AFTER against BEFORE `450a9e54…`:

* **Safe → non-Safe: none.** H-SR5 refuses more only for the four textual
  `return *(T *)X->F` / `X[I]` shapes W45-SR5 registered, none on a Safe
  path (that card measured no verdict move at its base). H-V, H-U and H-T
  only add admissions to positions that refused, behind the older routes.
  Should one appear, it is a named correction, read and attributed.
* **Unknown → Safe: 2 to 12**, most likely about 5. Few cast statements are
  a function's only blocker (the 285 "sole" functions are an upper bound over
  the inflated join, and most of them hold the same macro lines' other
  refusals); a freed statement exposes later readings of its names (rule
  10a), so `unknown_alias` leaves grow where `unsupported` ones fall.
* **Leaves:** unsupported falls by 100 to 800; unknown_alias rises by up to
  half of that. New leaks: at most 3, each read and classified (a freed
  statement can expose an existing failure-arm leak, as CG08's H4 did in
  `mk_sched_add_connection`); a provisional Unsafe is never presented as an
  upstream defect.
* **System route** (44 / 2 / 188 at its last record): 0 to 2 Unknown → Safe;
  `flb_storage_quarantine_chunk`'s `(void *) src` tail actual (CG26's system
  cast leaf) is admitted by H-U; `flb_coro_destroy`'s
  `((char *) coro) + sizeof(struct flb_coro)` (arithmetic over a cast, not a
  plain name) stays refused.
* **Tail-pause slice** (29 / 1 / 239 / 1 Pending): no status move; leaves may
  change.
* **`mk_vhost_set` at the tip** (the shared probe): unchanged. Its three
  remaining refusals are a `va_arg` loop condition, a `va_arg` value store and
  an uncontracted callee, none a cast; `mk_vhost_set_property` keeps its
  blockers (`mk_string_dup`, unresolved `ha`, indirect stores).

Census AFTER: the cells H-V and H-U address lose leaves; the `punned-view`
access cells keep theirs (refused by name), as do `function-object`,
`integer-to-pointer` and every cast over arithmetic or a call.
