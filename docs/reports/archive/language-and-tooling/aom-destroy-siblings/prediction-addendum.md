# AOM-DESTROY-SIBLINGS — prediction addendum: the primary arm is revised

Written **after** [`prediction.md`](prediction.md) was committed (`38ec8e0c`),
**while the BEFORE run was in flight**, and **before any AFTER measurement
exists**. It is a separate file and a separate commit on purpose: the original
prediction is not edited, so the record shows what was predicted first, what was
revised, and on what evidence.

## What changed and why

`prediction.md` §2.1 named the **discharging** arm (A) primary and the
**refusing** arm (B) as the stated alternative, on the ground that `result` is
bound directly from `flb_calloc` with no joined provenance. Reading
`crates/qcore-c-gateway/src/legacy_c.rs` further — not measuring anything —
shows that ground is insufficient, and **arm B is now the primary prediction**.

The missing fact is at `legacy_c.rs:6483-6486`. Every `LegacyInstruction::Unknown`
— which is what *any* refused statement lowers to, including
`indirect memory access is outside the resource projection` and
`unrecognized assignment destination` — ends with

```rust
for pointer in state.pointers.values_mut() {
    pointer.unknown = true;
    pointer.may_be_null = true;
}
```

It poisons the **pointer bindings**, while leaving `state.resources` and
`unresolved_effect_sites` alone. That is exactly why the leak leaves in
`flb_input_chunk_get_direct_routes` can still name their acquisition site
(`preprocessed line 19357`) although the body is full of refusals: the *site* is
untouched, only the *name* loses its binding.

Between `result = flb_calloc(...)` at 19357 and the first destroy call at 19383
the body runs the identifier loop, whose statements are refused at 19365, 19371
and 19379. So `result`'s pointer is already `unknown` at **every one** of the
twelve call sites, and `apply_resource_action`'s first conjunct
(`pointer.unknown`) fires regardless of `nullable`. The same reading explains the
BEFORE artifact's own `unresolved resource identity \`label_lengths\``
(19404, 19432, …) at `flb_free` calls on locals this body acquired: `flb_free`
is a declared nullable release and still refuses there, for this reason and no
other.

## The revised prediction

| measure | `prediction.md` | **revised** |
|---|---|---|
| arm at the twelve sites | **A**, discharging | **B**, refusing — `unknown_alias: unresolved resource identity \`result\`` at each |
| `leak` | 272 -> 266 (−6) | **unchanged: 272 -> 266 (−6)** |
| `unknown_alias` | 4,408 -> 4,409 (+1) | 4,408 -> **4,421** (+13: twelve sites plus `flb_input_chunk_map`'s one) |
| `unsupported` | 49,794 -> 49,781 (−13) | **unchanged: 49,794 -> 49,781 (−13)** |
| leaves added / removed | 1 / 19 | **13 / 19** |
| `flb_input_chunk_get_direct_routes` | 98 -> 80 diagnostics, stays `Unsafe` | **98 -> 92** diagnostics, leak 7 -> 1, **stays `Unsafe`** |
| `flb_input_chunk_map` | 99 -> 98 or 99 | **99 -> 99** (−1 `unsupported`, +1 `unknown_alias`) |
| unit / function status moves | none | **none** |

**The leak movement, the seventh surviving leaf, the unit and function verdicts,
the two declarations' dispositions and the contract-corpus pins are all
unrevised.** What is revised is the *mechanism* of the movement, and therefore
its character: the six leaves are **withdrawn by refusal** — `apply_resource_action`
reports the unresolved actual, calls `mark_unresolved_effect`, and `report_leaks`
skips the site — which is the **same mechanism card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED
measured one wave ago**, not the discharge `prediction.md` expected. That makes
this card's leak movement a withdrawal of provisional claims in the same sense as
wave 22's, and it means the declaration's *discharging* arm is still unexercised
anywhere in the pinned corpus.

## What this does not change

RIPPLE-AGGREGATE-OWNER (`prediction.md` §2.2) is now predicted to produce
**nothing**: `resolved_aggregate_owner` already returns `None` whenever
`pointer.unknown` (`legacy_c.rs:6610`), and the pointers at and after 18929 are
already unknown from the refusals at 18922 and 18926, so adding those sites to
`unresolved_effect_sites` changes no access decision. Predicted added
`unsupported` leaves from that effect: **0**.

`prediction.md` §3's recorded not-expected outcome stands unchanged: all seven
leaves retiring, `flb_input_chunk_get_direct_routes` moving `Unsafe -> Unknown`
and the pin reaching 265 is predicted **not** to happen.
