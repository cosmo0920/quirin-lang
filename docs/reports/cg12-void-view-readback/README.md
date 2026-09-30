# CG12-VOID-VIEW-READBACK: `void *` views and readbacks

Wave 48, round 7. Gap entry M05 (evidence Q18), route G. Gateway rule
[11j](../../agents/gateway.md#constraint-11j). Absorbs
`W47-FIELDS-BY-MEMBER-VOID-ESCAPES`.

## 1. The input, reproduced and corrected

The gap analysis (static, no Rust run) says: storage into a `void *` member is
supported, and a read back from a tracked `void *` member is refused because
the pointee type is not known. CG00 turned that into a BEFORE control
(`m05_readback_from_a_void_field_is_refused`) and attributed 76 corpus leaves
on 33 Unknown functions to M05.

| Claim | Status |
|---|---|
| a tracked pointer stored into a `void *` member of an admitted store base is admitted (ALIAS-AGGREGATE) | **reproduced** (a record the reader reads whole) |
| its readback is refused | **reproduced**, as P2's target-free borrowed alias: `char *c = s->v;` is admitted as a read with no identity, and `q_free(c)` is `UnknownAlias { variable: "c" }` |
| CG00's control refuses at the readback | **correction**: over the control's `struct S` (a bit-field, arrays, a function pointer) the `indirect memory access` refusal is the **store** `s->v = q_alloc()`, which ALIAS-AGGREGATE's reader does not admit for a record it did not read whole; the readback is never reached |
| the 76 corpus leaves are readbacks of tracked members | **correction**: CG00's attribution (`attribute.py`, cause `cast-from-void`) takes any cast whose operand is `void *` on a leaf's path. Read line by line, every one of the 76 is a **view** of an untracked `void *` (a comparator's `const void *` formal, nanopb's `field->pData`, cmetrics' `void *instance`) or a view over a call result or arithmetic; none is a readback of a tracked member |
| `W47-FIELDS-BY-MEMBER-VOID-ESCAPES`: 80 escapes | **correction** (§6): 62 of the 80 are `(void) x;` discards (Clang `ToVoid`), which leave no pointer value; 18 carry one |

So the card is two rules, both in `void_view.rs`: the **view** (what the
corpus needs) and the **readback** (what the entry names).
