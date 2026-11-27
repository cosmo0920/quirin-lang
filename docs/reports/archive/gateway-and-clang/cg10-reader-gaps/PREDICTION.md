# CG10-READER-GAPS: prediction

Committed before the implementation commit. Base `9b107caf1` (main). USER
DECISION 2026-11-21: "launch the reader-gap card for the indirect-memory-access
refusals". Specification: the three `named_followups` entries
CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL, CG10-CAST-VIEW-RETURN-POSITION and
CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS.

**Disclosure.** Before this file was committed: the base binary was built and
its opt-in `current` run started (its report not yet read); scratch fixtures
were run against the **base** code (`tests/reader_gaps.rs`, uncommitted), and
one debug run of the base gateway over the preprocessed
`rdkafka_interceptor.c` with the typed-path parity stream on
(`private/rg/parity1.jsonl`). The rule-71 hook (below) was written but not
built or measured. No tip binary exists.

## 1. Diagnosis on the base (method step 1)

### (a) CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL (10 rows)

The ledger's reading of the cause (a three-hop address neither reader reads)
is **not** the cause. Fixtures over the base read
`&rk->rk_conf.interceptors.on_send` (a typedef'd embedded record, an untagged
struct member, then the list) with W44's typed actual reader and name rule
12t's `retention-undecided` (rule 69), exactly as for `&conf->interceptors.on_X`.
The real rows differ by their first statements, the expansion of
`RD_KAFKA_INTERCEPTOR_REJECT_IF_SHARE(rk)`:
`do { if (((rk)->rk_type == RD_KAFKA_CONSUMER && (rk)->rk_conf.share.is_share_consumer)) return …; } while (0);`.
The parity row of the never-bound join for `rk` reads
`nested-reading-binds` / `lexical-binds-ast-never-bound` (closure reason
`e:nested`): rule 10o's reader of parenthesised writes
(`lexical_nested_assignment::parenthesised_writes`) reads the `&` of the
logical AND `&&` (tokenised `&` `&`) right before the group `(rk)` as the
unary address of `rk`. The lexical closure therefore binds `rk`, and an
address actual (rule 10n: every root never bound) is refused, so rule 69's
reading answers `an actual neither reader recognises`. A fixture with the
macro's spelling reproduces the refusal; the same body with `if ((rk)->f)`
does not.

**Reading (rule 71):** `X && (p)`, exactly two `&` tokens after an operand
end, is a logical AND, not an address (C11 6.5.13; `X & &(p)` would be a
bitwise AND with a pointer operand, forbidden by 6.5.10p2).

### (b) CG10-CAST-VIEW-RETURN-POSITION (5 rows)

All five are whole-call returns `return f(…, (T *)p->m, …)`. Rule 12i (W44 at
the return position) declines a returned call that hides a place behind an
identity-preserving view (rule 15z's guard, `returned_call_hides_a_place`),
and rule 12h's H-U has no return placement.

**Reading (rule 71a):** at rule 12i's placement, a whole-call return whose
call holds identity-preserving views of places (H-U0's set,
`hides_an_identity`) is read as its uncast call, with every such view removed
(none left hidden), by rule 12i unchanged; W44's typed actual reader matches
an uncast actual to its cast's operand node, only while this reading is
asked. A view to a union pointer type is not removed (rule 10w's design).

### (c) CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS

* `T *` for a function typedef `T` (`rd_kafka_buf_set_maker`'s `make_cb`):
  **reading (rule 71b)**: a formal whose Clang type is exactly `NAME *` with
  `NAME` a typedef of the unit every declaration of which is a function type
  is rule 10v's function-pointer name, not rule 70's retained formal.
* A formal declared `R (*f)(A)` in the W4 lexical parameter reader
  (`legacy_tokens::function_parameters`): **refusal recorded**. The reader is
  shared by the W4 derivation and validation, selection frames, recursion,
  neutral-callee and self-link readings, member release and every body's
  formal set; reading a function-pointer declarator there changes every
  function with such a formal, which is its own census (follow-up).
* `cio_set_log_callback` (an object pointer `void (*log_cb)` stored into a
  function-pointer member, an implicit `BitCast`): **refusal recorded**. ISO C
  defines no conversion between object and function pointers (C11 6.3.2.3;
  J.5.7 is an extension); rule 12h's table classes it `function-object`,
  refused. The row stays.

## 2. Predicted moves (opt-in `current --monkey-linked`, Fluent Bit 046accf6)

* Rule 71: the ten `rd_kafka_interceptor_add_on_*` rows **re-attributed**
  (the access leaf replaced by rule 12t's `retention-undecided`, through rule
  69), each still `Unknown`. Elsewhere (`&& (p)` appears in 80
  `RD_KAFKA_IS_SHARE_CONSUMER` uses and many other macros): 0 to 15
  Unknown -> Safe, 20 to 300 rows change leaves.
* Rule 71a: `print_string`, `pb_dec_bool`, `pb_enc_fixed_length_bytes`
  Unknown -> Safe **0 to 3** (each also needs its same-unit callee's
  summary: `print_string_ptr`, `pb_decode_bool`, `pb_encode_string`);
  `LZ4F_localSaveDict` stays (union pointer view kept; its callees are
  defined in `lz4.c`); `sort_by_map_elem_val_toppar_list_cnt` stays (two
  pointer declarations from member reads), its return leaf removed. 0 to 20
  more rows change leaves (other returned cast views).
* Rule 71b: no status move predicted (`rd_kafka_buf_set_maker` is already
  `Safe` and publishes no summary for its callers, section 1(c)); 0 to 5
  rows change leaves.
* **No Safe -> non-Safe move, no new Unsafe, no added leak.**

```measure-prediction
route corpus
move print_string Unknown -> Safe
move pb_dec_bool Unknown -> Safe
route system
none
route slice
none
```

Corpus and twelve-home: as above where the route analyses the unit (to be
confirmed by the integrator's `quirin-audit measure --declared`, which cannot
run on this host: no `build/cycle19` inputs or baselines); system and slice:
none.
