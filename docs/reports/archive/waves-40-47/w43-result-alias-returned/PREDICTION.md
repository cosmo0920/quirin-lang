# W43-RESULT-ALIAS-RETURNED: prediction, frozen before implementation

Card W43-RESULT-ALIAS-RETURNED, 2026-09-25, base `f8b41209`. This file is
written and committed before any analyzer change. It is not edited after that
commit. The report compares it with the outcome.

## What the card will add

An inferred, validated **derived-result summary** for a defined function. Every
exit of the function must do one of two things:

* return a pointer derived from **one** borrowed formal, through a declared or
  inferred derived result;
* return a null pointer constant.

The summary is `ResourceReturnMode::Derived { parameter, extent, nullable,
const_dropped }`, the record a `resource_return_alias` or
`resource_return_interior` declaration loads to. Callers therefore apply the
wrapper through the unchanged W43-LIBC-RESULT-ALIAS call rules, including
`use_after_release` and `invalid_release`.

## Expected movers

Measured from the baseline `docs/reports/w43-libc-result-alias/artifacts/` and
the pinned sources.

| Function | Unit | Body | Predicted summary | Predicted verdict |
|---|---|---|---|---|
| `XXH_memcpy` | `rdxxhash.c` | `return memcpy(dest,src,size);` | `Derived { parameter: 0, extent: Exact, nullable: false, const_dropped: false }` | Unknown → **Safe** |
| `mk_string_casestr` | `mk_core/mk_string.c` | `if (!heystack \|\| !needle) { return NULL; } return strcasestr(heystack, needle);` | `Derived { parameter: 0, extent: Interior, nullable: true, const_dropped: true }` | Unknown → **Safe** |

`mk_string_casestr` has a `return NULL;` exit. The rule must therefore admit a
null-constant exit as the null alternative of a nullable derived result, and
refuse every other non-derived exit.

Each function carries exactly one leaf at the baseline, the return-expression
refusal (`rdxxhash.c:112`, `mk_core/mk_string.c:316`). Both leaves are predicted
to disappear, and no leaf is predicted to be added to either function.

## Transitive movers

* **`XXH_memcpy`'s callers** are `XXH32_update_endian` (3 call sites) and
  `XXH64_update_endian` (3 call sites), both in `rdxxhash.c`. Every call's first
  argument is a member or a cast of a member, such as
  `(BYTE*)(state->mem32) + state->memsize` or `state->mem32`. No baseline leaf
  names `XXH_memcpy`, so these statements are refused before any callee lookup.
  - **Prediction:** no verdict movement. Both functions stay Unknown; they carry
    34 and 29 leaves.
  - **Stated risk:** leaf re-attribution confined to those six lines, if a
    contract in force changes which gate refuses the statement first. The
    report attributes any such leaf.
* **`mk_string_casestr`'s callers:** none in the corpus. The pinned sources
  contain only its definition and its header prototype.
* **Other bodies returning a derived pointer through a name**
  (`p = strchr(s, c); … return p;`): not censused before implementation.
  - **Prediction:** 0 further movers. Such bodies at the baseline carry an
    `unknown_alias` on the returned name, and the card's own census will
    classify them by refused shape.

## Expected totals

| Measure | Baseline | Predicted |
|---|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 257 / 84 / 4,493 | **259 / 84 / 4,491** |
| Units | 2 / 42 / 137 / 4 | unchanged |
| `unsupported` | 48,930 | **48,928** (−2) |
| `unknown_alias` / `leak` / `summary_conflict` | 4,657 / 272 / 1 | unchanged |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 |
| Inferred-validated summaries in the two units | — | +2 |

The leak pin stays at **272**. A derived summary never discharges its base, and
neither mover acquires anything.

## Refused shapes (counts unknown before the census)

Each is refused and counted by the card's census:

* **Mixed exits:** a derived pointer on one path, and a fresh resource, an
  untracked pointer or no pointer on another.
* **Base custody:** the result is derived from a formal the body releases,
  consumes or otherwise uses at a custody-bearing position.
* **No formal base:** the result is derived from a non-formal base, such as a
  local resource, a string literal, a global or an untracked expression.
* **Several formals:** different exits derive from different formals.
* **Mutated local copy:** the derivation runs through a local copy that the
  body rebinds or offsets. It is expected to land in "no formal base" or
  "body not modeled", and a unit test pins which one.
* **Body not modeled:** anything else the ordinary validation refuses.

## Exact-alias release (`W43-RESULT-ALIAS-EXACT-RELEASE`)

**Prediction:** kept refused on a recorded ground. No corpus function reaches
the shape, so crediting it would move nothing measurable. The card also
predicts one supporting change: a derived base spelled `place ± offset` is read
as **interior** rather than exact, because `memcpy(p + 1, …)` does not return
`p`. This is predicted to move **nothing** in the corpus (0 `invalid_release`
leaves before and after).
