# ADDRESS-OF-MEMBER-DESTROY-UNDECLARED — prediction, written before the AFTER run

Pre-registered on the base tree `ec5535a9` after the BEFORE run was launched
and **before** any AFTER measurement existed. Nothing below is edited after the
fact; the report's measurement section is written beside it and says where the
prediction was right and where it was wrong.

## 0. What is being declared

`flb_input_chunk_destroy`, read against its body at Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, `src/flb_input_chunk.c:2704-2807`:

```c
int flb_input_chunk_destroy(struct flb_input_chunk *ic, int del)
{
    ...
    if (flb_input_chunk_is_up(ic) == FLB_FALSE) { flb_input_chunk_set_up(ic); }
    mk_list_foreach(head, &ic->in->config->outputs) { /* size accounting only */ }
    ret = flb_input_chunk_is_up(ic);
    if (ret == FLB_FALSE) { ret = cio_chunk_up_force(ic->chunk); ... }
    ret = flb_input_chunk_get_tag(ic, &tag_buf, &tag_len);
    ...
    if (del == CIO_TRUE && tag_buf) { flb_hash_table_del_ptr(..., (void *) ic); }
#ifdef FLB_HAVE_CHUNK_TRACE
    if (ic->trace != NULL) { flb_chunk_trace_destroy(ic->trace); }
#endif
    cio_chunk_close(ic->chunk, del);
    mk_list_del(&ic->_head);
    if (ic->routes_mask != NULL) { flb_free(ic->routes_mask); ic->routes_mask = NULL; }
    flb_free(ic);
    return 0;
}
```

declared as

```
extern fn flb_input_chunk_destroy(ic: ptr<mut, u8>, del: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_released(ic, nonnull);
    ensures normal resource_borrowed(del);
    ensures normal resource_return_none();
};
```

The faithfulness argument is in the module's own header comment; the two facts
the prediction turns on are that the body has **exactly one exit** (`return 0;`
at `:2807`; there is no early `return` anywhere between `:2704` and it) on
which `flb_free(ic)` is unconditional, and that it dereferences `ic` in its
**first** statement, so `nonnull` is the only faithful nullability and
`nullable` would be unfaithful in the *permissive* direction.

## 1. The mechanism the prediction rests on

`apply_resource_action` (`crates/qcore-c-gateway/src/legacy_c.rs:6597`) is what
a `resource_released(ic, nonnull)` actual reaches. It has two arms:

* the actual resolves — one target, not `unknown`, not `may_be_null`, no
  nullable site — and the site moves `LIVE -> RELEASED`; or
* it does not, in which case it reports `UnknownAlias` and/or
  `unsupported: nullable resource ... used by a non-null contract`, calls
  `mark_unresolved_effect` and returns.

**Both arms remove the leak leaf**, and that is the single most important
sentence in this document. `report_leaks` (`:7319`) skips any site in
`state.unresolved_effect_sites`, so the refusing arm *retires* the residual
exactly as constraint 10 records for the publication refusals. The pin
therefore moves **273 -> 272 either way**, and the interesting question is not
*whether* it moves but *which arm moves it*, because the two arms mean
completely different things.

## 2. The prediction proper

**Primary prediction (B): the refusing arm, not the discharging arm.**

Card ABI-TYPEDEF-ADDRESS-OF-MEMBER's archived
`logs/probe-input-chunk-get-after.txt` shows the reaching state at the sibling
exit `L19851`:

```
L19851  n1  EscapeIfOwned ic   | return ic;
        !! unresolved resource identity `ic`
        !! unsupported: nullable resource `ic` used by a non-null contract
```

`EscapeIfOwned` (`:6289`) is `apply_resource_action` with `escape = true` and
`nullable = context.result_nullable`, i.e. the identical predicate. `n1` and
`n4` (hence `n3`, the destroy call at `L19847`) both consume the state after
`n5`, and every branch condition between them is `Unknown`, so **no narrowing
separates them**. The same predicate on the same state must report the same two
refusals. So I predict, at `L19847`:

* `unsupported: call to uncontracted function \`flb_input_chunk_destroy\`` — **removed**
* `unknown_alias: unresolved resource identity \`ic\`` — **added**
* `unsupported: nullable resource \`ic\` used by a non-null contract` — **added**
* `L19849 leak: live obligation acquired at preprocessed line 19835` — **removed**,
  by `mark_unresolved_effect`, *not* by a modelled release.

Why the actual does not resolve: `ic` at that point is the **join** of two
provenances — the `flb_input_chunk_create` acquisition at `L19835`, narrowed
non-null by `if (!ic) return NULL;`, and the `flb_hash_table_get` lookup path,
which reaches the same program point without ever passing that guard and whose
`ic` the projection never resolves. The join keeps `site0` in `nullable_sites`
and `may_be_null` set. That is exactly what wave 20 wrote for `L19851` ("this
body's guard does not narrow it on that path"), and it is a fact about
`input_chunk_get`'s control flow, not about the declaration.

**Alternative (A), not excluded:** if the reaching state at `n3` does resolve
`ic` to the single narrowed site — i.e. if `EscapeIfOwned`'s refusal at `L19851`
comes from something `n3` does not share — then the release is admitted, the
exit becomes `Discharged`, the leak leaf is removed by a *modelled release*, and
`L19847` gains no leaf at all. That is the outcome a faithful release contract
exists to produce, and constraint 7 governs it: a **discharging** move of the
leak pin is precisely what this card is scoped to obtain, needs no accusing
justification, and is attributed the same way.

I do **not** promise the pin holds. I predict it moves to **272**, and I state
in advance that under (B) the movement is a *withdrawal of a provisional claim*
rather than a discharge, and must be reported as such.

**Excluded:** the pin staying at 273. Under both arms the leaf goes. If it
survives, my reading of `report_leaks`/`mark_unresolved_effect` is wrong and
the report says so.

## 3. Function and unit status

* `input_chunk_get`: provisional `Unsafe` -> **`Unknown`**. Its `Unsafe` rests
  on the single leak leaf; every other leaf it carries is `Unknown`-class. This
  is a **withdrawal**, not a promotion, under either arm. Nothing may become
  `Safe`.
* `flb_input_chunk_destroy` itself: **`Unknown`, unchanged**. A declaration
  fixes what callers may assume; it does not stop the unit analysing the body,
  and that body's own uncontracted calls are untouched by this card.
* Unit `fluent-bit-chunk/flb_input_chunk.c`: provisional `Unsafe`,
  **unchanged** — it carries 40+ other refusals and at least one other leak
  (`flb_input_chunk_get_direct_routes` L19399, `CONSTRUCTOR-RESIDUALS-LEAK-271`,
  which this card does not touch and does not withdraw).
* Function histogram 150 / 85 / 4,599 -> **150 / 84 / 4,600**. No `Safe` moves.

## 4. The explicit candidate set — every place a leaf may move

`flb_input_chunk_destroy` is called from **five** sites inside the one corpus
unit that defines it, and from nowhere else in the 185-unit corpus (`flb_task.c`,
`flb_storage.c`, `flb_input_log.c` and the `tests/` callers are outside it; the
`fluent-bit-chunk` scope holds exactly one unit). Source lines at the pin:

| site | enclosing function |
|---|---|
| `:718` | `flb_input_chunk_release_space` |
| `:2907` | `input_chunk_get` — the leak-273 leaf |
| `:3184` | `memrb_input_chunk_release_space` |
| `:3454`, `:3485` | `input_chunk_append_raw` |

Every leaf that may move is at one of those five statements or is consequential
on one, **plus** one class I name in advance because it is the ripple wave 20
warned about:

**RIPPLE-UNKNOWN-CASCADE.** At a site whose actual is wholly unresolved,
`mark_unresolved_effect` takes the `pointer.unknown` branch and marks
**every site in `state.resources`** unresolved, not just the actual's. At those
sites the declaration can therefore retire leaks *other than* the one it was
written for, anywhere downstream in the same function. I predict this is
possible at `:718`, `:3184`, `:3454` and `:3485`; I do not predict how many, and
every one that appears will be attributed by function and line rather than
absorbed. Any such retirement is a **withdrawal in the `Unknown` direction**,
never a promotion.

**RIPPLE-W4.** Modelling these five statements removes `Unknown` instructions
from four function bodies, which may make W4 able to derive and validate
summaries it could not derive before, which may in turn surface findings in
*their* callers — the exact pattern by which wave 20 produced the leak this card
decides. Bounded to this unit, since these four functions' callers within the
corpus are in this unit.

**Predicted diagnostic-count direction:** `unsupported` down by at least 5 (the
five uncontracted-call leaves), `unknown_alias` up by up to 5, `leak` down by 1
or more. I predict **no** `double_release`, **no** `release_after_escape` and
**no** `custody_conflict` appears: those need a resolved actual on a site that
is already released or escaped, and under (B) no actual resolves at all.

## 5. The two loader gates

* **LOADER-ABI type agreement** (constraint 8, first half). The unit's own C
  declaration is `int flb_input_chunk_destroy(struct flb_input_chunk *ic, int del);`
  (`include/fluent-bit/flb_input_chunk.h:111`), matching the definition. Declared
  `ptr<mut, u8>` / `i32` / `i32` projects to pointer / scalar-4 / scalar-4 under
  LP64. **Predicted: agrees, zero withdrawals of any class, as in the BEFORE run.**
* **LOADER-ABI-MODES duty gate** (constraint 8, second half). The body **is**
  in-unit, so unlike `msgpack_packer_init` the gate is reachable in principle.
  But the body calls `flb_input_chunk_is_up`, `flb_input_chunk_set_up`,
  `flb_input_chunk_get_real_size`, `flb_routes_mask_get_bit`,
  `flb_input_chunk_output_size_subtract`, `flb_input_chunk_get_tag`,
  `flb_hash_table_del_ptr`, `flb_chunk_trace_destroy` and `mk_list_del`, none of
  them contracted, so its own analysis carries `Unknown` instructions and
  **I predict W4 derives no summary for it** — whereupon constraint 8's "absence
  of evidence is not evidence" governs and nothing is withdrawn.
  **Predicted `summary_conflict` count: 0, unchanged.** If a summary *is* derived
  and contradicts the declared duty, the declaration is withdrawn for this unit
  and this card lands nothing; that outcome is reported, not worked around.

## 6. What decides whether the module is landed

PROD-CONTRACTS' rule, unchanged: a premise with no measured payoff is not
landed. Under both (A) and (B) the measured payoff is non-zero — a leak leaf and
a function verdict — so the module lands. It does **not** land if the corpus
shows zero movement, if the ABI gate withdraws it, or if the duty gate convicts
it.

## 7. Lean

The admission relation is not changed by this card: no Rust under `crates/` or
`compiler/src/` is touched, only contract text, its manifest and the pins that
count it. **Predicted: no Lean module needs adding or amending**, and
`check_lean_audits.py` reports the same audit count as the base tree.
