# AOM-DESTROY-SIBLINGS — pre-registered prediction

Written and committed on the base tree (`52e3889c`) **before any AFTER
measurement exists**, following card `wave22-aom-destroy`'s convention. Every
number below is a prediction, not a measurement; the report states where each
one landed, including where it was wrong.

Baseline measured against: `docs/reports/wave23-c-operations/artifacts/`
(185 units · 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only;
4,834 projected functions · 150 / 84 / 4,600; `leak` **272**, `unknown_alias`
**4,408**, `unsupported` **49,794**; contract corpus 72 modules,
manifest-selected 17, v1 declarations 38).

---

## 0. What the card decides

`AOM-DESTROY-SIBLINGS-UNDECLARED` names two functions. **They get different
answers**, and that is the point of reading each body on its own terms rather
than on its neighbour's name.

| function | disposition predicted | ground |
|---|---|---|
| `flb_input_chunk_destroy_direct_routes` (`src/flb_input_chunk.c:2483-2504`) | **declare** | faithful `resource_released(routes, nullable)`; 13 uncontracted-call leaves in the pinned corpus to retire |
| `flb_input_chunk_destroy_corrupted` (`:2623-2701`) | **do not declare** | the reading is faithful and is written out, but the measured payoff is **zero**: the corpus carries no `call to uncontracted function \`flb_input_chunk_destroy_corrupted\`` leaf at all. Same disposition, same rule, as `msgpack/packer.qcore` under card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF |

Counted on the frozen wave-23 artifact before the runs: **13** hits for
`flb_input_chunk_destroy_direct_routes` (12 in `flb_input_chunk_get_direct_routes`,
one in `flb_input_chunk_map`), **0** for `flb_input_chunk_destroy_corrupted`.

## 1. The declaration predicted to land

```
extern fn flb_input_chunk_destroy_direct_routes(routes: ptr<mut, u8>,
                                                route_count: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(routes, nullable);
    ensures normal resource_borrowed(route_count);
    ensures normal resource_return_none();
};
```

`nullable`, **not** the `nonnull` its already-declared sibling
`flb_input_chunk_destroy` carries — the third statement of this body is
`if (!routes) { return; }`, so a null argument is defined behaviour here and a
`nonnull` declaration would be unfaithful in the *restrictive* direction while
also refusing every call site that cannot prove non-nullness. This is exactly
the divergence the card brief warns about: the two destructors differ in the
one conjunct that decides the outcome.

## 2. Corpus movement predicted

### 2.1 `flb_input_chunk_get_direct_routes` — the seven-leaf family

Twelve call sites, all `flb_input_chunk_destroy_direct_routes(result, stored_count)`,
each immediately followed by `return -1;` or `return -2;`. `result` is bound
directly from the declared `flb_calloc` at preprocessed line **19357** and
narrowed non-null by `if (!result) { ... return -1; }` — **there is no join of
two provenances here**, which is what distinguishes this card from wave 22.

**Primary arm (A), the DISCHARGING one.** `apply_resource_action` resolves
`result` to the single site, `nullable = true` removes the `may_be_null`
conjunct entirely, `cascade_is_unambiguous` holds (the `result[index].label`
stores are refused as `unrecognized assignment destination`, so no field links
exist), and the site goes `LIVE -> RELEASED`.

| preprocessed line | predicted |
|---|---|
| 19383, 19391, 19398, 19405, 19433, 19452, 19460, 19488, 19494, 19504, 19516, 19523 | `unsupported: call to uncontracted function \`flb_input_chunk_destroy_direct_routes\`` **removed** (12) |
| 19384, 19392, **19399**, 19434, 19489, 19505 | `leak: live obligation acquired at preprocessed line 19357` **removed** (6) |
| 19544 | `leak: ... 19357` **STAYS**. This exit is the success path: `*routes = result` is refused (`store through \`*routes\` without an out-parameter contract`), so `result` is still live and no destroy call runs on that path |
| — | **no** `double_release`, **no** `release_after_escape`, **no** `custody_conflict`, **no** `unknown_alias` added at any of the twelve sites |

Consequence predicted: `flb_input_chunk_get_direct_routes` keeps `leak` = **1**
and therefore **stays provisional `Unsafe`**; its diagnostic count falls
98 -> 80. The unit `fluent-bit-chunk/flb_input_chunk.c` keeps leaks 11 -> 5 and
**stays provisional `Unsafe`**.

**Alternative arm (B), the REFUSING one — stated because it is possible.** If
`result`'s pointer is `unknown` at some site (an earlier
`mark_unresolved_effect` does *not* cause this — it writes
`unresolved_effect_sites`, not `pointers` — but an unmodelled write might),
that site reports `unknown_alias: unresolved resource identity \`result\``
instead, and `mark_unresolved_effect` retires the same leak anyway. **Under
both arms the six leaves go**; the arms differ only in whether `unknown_alias`
grows by up to 12. Arm A is predicted.

### 2.2 `flb_input_chunk_map` — one call site, direction uncertain

`flb_input_chunk_destroy_direct_routes(direct_routes, direct_count)` at
preprocessed **18929**. `direct_routes` is filled through `&direct_routes` by
an uncontracted out-parameter, so the projection has no identity for it.

* `unsupported: call to uncontracted function ...` at 18929 **removed** (1).
* `unknown_alias: unresolved resource identity \`direct_routes\`` at 18929
  **added** — predicted, but *uncertain*: if `direct_routes = NULL;` left a
  resolved zero-target pointer, the nullable release is a silent no-op and
  nothing is added. Predicted **+1**; **0** is the other admissible outcome.
* `flb_input_chunk_map` carries **no leak** at the baseline, so
  `mark_unresolved_effect`'s `pointer.unknown` branch (RIPPLE-UNKNOWN-CASCADE,
  wave 22 §2) has nothing to retire there. Predicted retired leaks: **0**.
* **Named uncertainty, RIPPLE-AGGREGATE-OWNER:** marking every site unresolved
  at 18929 makes `resolved_aggregate_owner` return `None` for those sites
  afterwards, so an aggregate access admitted downstream of 18929 could become
  refused. Predicted **0 to a small number** of *added* `unsupported` leaves,
  in the refusing direction, in this function only. If more than a handful
  appear they are attributed leaf by leaf.
* `flb_input_chunk_map` stays `Unknown`. No status move.

### 2.3 Everywhere else

**Zero.** The callee is called from no other unit of the 185 (upstream's other
callers are `src/flb_task.c` and `tests/`, both outside the audited scope), and
the 13 baseline leaves are all in this one unit.

### 2.4 Headline numbers predicted

| measure | baseline | predicted |
|---|---|---|
| `leak` | 272 | **266** (−6) |
| `unknown_alias` | 4,408 | **4,409** (+1; 4,408 admissible, see 2.2) |
| `unsupported` | 49,794 | **49,781** (−13; ± the RIPPLE-AGGREGATE-OWNER additions) |
| unit verdicts | 2 / 42 / 137 / 4 | **unchanged** |
| function histogram | 150 / 84 / 4,600 | **unchanged** — no function status move predicted |
| units with changed diagnostics | — | **1** |
| `summary_conflict` | 0 | **0** — the callee's own body carries 5 diagnostics, so W4 derives no summary for it and constraint 8's duty gate is never reached |
| LOADER-ABI type withdrawals | 0 | **0** — header `void f(struct flb_chunk_direct_route *, int)` against `ptr<mut,u8>, i32 -> unit` |
| contract corpus | 72 / 17 / 38 | **73 / 18 / 39**, rejected unchanged at 145 |
| Lean audits | 95 | **95** — no Rust under `crates/*/src` or `compiler/src` is predicted to change, so there is no semantic change for a Lean module to track |

## 3. The leak pin, and what moving it costs

Constraint 7: the pin is **272** and has moved three times, each deliberately.
This card predicts a **fourth deliberate move, 272 -> 266**, the largest so far,
and it is a **withdrawal of provisional claims by discharge**, not a promotion:
six `Unsafe`-class leaves disappear because the release is now modelled and
*seen to happen*, which is what a faithful release declaration exists for.
Nothing moves toward `Safe` at unit or function level.

`CONSTRUCTOR-RESIDUALS-LEAK-271` (`flb_input_chunk_get_direct_routes`, L19399)
is predicted **DECIDED — withdrawn**. Its own card recorded the cost as
"withdrawing six standing claims". The prediction refines that: of the seven
leaves, **six** retire (five of the six standing claims, plus the wave-17 leaf
itself) and the **seventh, L19544, stands**, because it is on the success path
where no destroy call runs. If that is right, the recorded cost was one claim
too pessimistic, and the function stays `Unsafe` on the surviving leaf.

If instead all seven retire, `flb_input_chunk_get_direct_routes` moves
`Unsafe -> Unknown` (a function status move, 150 / 84 / 4,600 -> 150 / 83 /
4,601) and the leak pin goes to 265. That is predicted **not** to happen and is
recorded here so that it cannot be claimed afterwards as if it had been.

## 4. What is not attempted

`AOM-DESTROY-JOINED-PROVENANCE` is untouched. This card installs no model for
the lookup/acquisition join, manufactures no entry custody, and does not revisit
`input_chunk_get`'s `L19847`/`L19851` refusals. Those two leaves are predicted
**byte-identical** before and after.
