# CG26-VARIADICS: prediction (frozen before the AFTER measurement)

Frozen at the card's branch before the AFTER release binary was built. The
implementation (protocol, tail widening, borrowed-tail clause, variadic
parameter reader) was written before the census-backed figures below were
available; this file was written from the BEFORE census and committed before
any AFTER binary or measurement existed. That ordering is recorded, not
hidden.

## BEFORE (measured, main `b6a3dc42`, corpus `66910c10`, `--jobs 4`)

- Corpus **629 / 10 / 4,195**, `resource-diagnostics.json` sha256 `3d53a5d0…`
  (the brief's baseline reproduces).
- `call to uncontracted function __builtin_va_*`: **124 leaves in 50
  functions** (`va_end` 55, `va_start` 50, `va_arg` 17, `va_copy` 2). The
  ledger's W44 figure (127 in 51) does not reproduce at this base: a
  correction to the input's size, not to its claim.
- Tail refusals `arguments beyond the declared parameters of X`: **209
  leaves in 127 functions** (`fprintf` 129/81, `snprintf` 57/34, `mk_print`
  17/9, `printf` 5/4, `fcntl` 1/1).
- Functions blocked by one family alone: va-builtin 1
  (`rd_kafka_set_thread_name`), tail 2 (`rd_kafka_open_cb_generic`: `fcntl`'s
  `&on`; `cb_debug_chunk_complete`: `printf("…%s…", in_len, buf)` with `buf`
  a live tracked identity).

## Mechanisms and predicted movement

1. **`va_list` protocol** (K09, W44-VA-START-END-BUILTINS). The straight
   `va_start … vsnprintf/va_arg … va_end` shape dominates the 50 functions.
   Predicted: most of the 124 leaves disappear; the remainder are replaced by
   named `va_list protocol refused:` / `va_list operation refused:` leaves
   (pointer `va_arg` values, e.g. `rd_kafka_producev`'s argument loop; any
   list event under a conditional or in an unread form). Upper bound 124
   removed; predicted removed ≥ 90.
2. **Variadic parameter reader.** Every variadic definition's declarations
   become readable; typed routes may now decide statements they refused
   wholesale (unresolved identities and indirect accesses in those ~50
   bodies). Predicted: leaf decreases in some of the 50 functions; no new
   leak or violation expected, but any one is classified by reading.
3. **T7** (W46-FCNTL-SETFD-ADDRESS-TAIL). Exactly one corpus site:
   `rd_kafka_open_cb_generic` loses its only leaf.
4. **T8** under `resource_variadic_borrowed_tail()`. No provider declares the
   new clause in this card, so **no corpus movement** from T8;
   `cb_debug_chunk_complete` stays Unknown (its `printf` tail is a data tail).

Predicted corpus: **631 / 10 / 4,193** — exactly two Unknown → Safe
(`rd_kafka_set_thread_name`, `rd_kafka_open_cb_generic`), with an allowance
of up to +3 more Unknown → Safe among the 50 va-builtin functions if the
reader change removes their remaining unresolved-identity leaves (none of the
low-blocker rows shows that shape, so 0 is expected). **No Safe → non-Safe
move** and no new Unsafe.

System route and tail-pause slice (Fluent Bit): the same mechanisms reach
`flb_*` variadic definitions and `flb_error`-family tails; predicted no
status move (their va-builtin functions carry other blockers), leaf
decreases allowed.

## Not claimed

Format agreement (K10) — out of scope; no format checker is built. Route T
is unchanged (K08/K09's T half stays refused).
