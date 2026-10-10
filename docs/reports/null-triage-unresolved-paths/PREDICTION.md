# NULL-TRIAGE-UNRESOLVED-PATHS: prediction

Frozen before any implementation code. Base: local `main` at `c77efedbe`
(NULL-CANDIDATE-TRIAGE landed). BEFORE: binary `private/ntu/bin/before`, run
`b0` (corpus, system, slice; null property and parity selected).

## Design (what the code will do)

1. **Priority 0: a call's integer result is never free.** Today the path
   search (rule 14r) gives a non-pointer call result a fresh atom over its C
   type's range and takes either arm of any comparison with it
   (`Condition::Free`). That widens what the callee may return
   (`copy_label_values`: `cfl_list_size() < 0`). After the change a call
   result's range is only a **bound**: the C type's range (an unsigned or
   `size_t` result is never negative from its type alone), narrowed by a
   checked result-range contract when one is in force and not refuted by an
   analysed body. A comparison the bound decides takes its one arm; any other
   comparison over a call result is **inexact**: the search may still explore
   both arms (an over-approximation, sound for proving infeasibility) but a
   path that took an inexact decision is never a witness. Such a path that
   reaches the dereference NULL leaves the candidate unresolved.
2. **Inexact exploration for every undecided condition.** The same rule for
   the conditions that today end a path unresolved: a value the search does
   not follow, a correlated condition (a memory value re-read after a call),
   a correlated pointer test, an ordered pointer comparison, a switch over an
   unfollowed or correlated value. Both arms are explored, the path is
   marked inexact, and only exact paths are findings. A candidate whose every
   path was explored (no loop bound, budget, unread form or nested case
   label hit) and none reaches the dereference NULL is **infeasible**, and
   an infeasible candidate is **removed** (its check is proved by the search,
   `path-search:infeasible`, resting on every declaration the search read).
3. **Exact relational facts.** `atom + k` / `atom - k` (no wrap over the
   atom's whole range) is followed as an offset of the atom; a comparison of
   two distinct unconstrained atoms (or an atom and an offset of another)
   takes an arm only when the difference interval admits it (today both arms
   are taken whatever the ranges: a widening, corrected here). Floating
   values stay inexact.
4. **A result-range contract** `ensures normal null_result_range(LO, HI);`
   (bounds contract v1, null property only) for `cfl_list_size`
   (`quirin-provider-cfl`, a new bounds view), pinned at both revisions and
   checked against every analysed body (a monotone counter from a
   non-negative start); a refuting body withdraws it for the run.
5. **Memory cache soundness.** A tracked variable's assignment forgets the
   cached memory values whose spelling names it (today `p->n` keeps its
   value across `p = q`).
6. Loops: the existing budget (64 iterations) is kept; beyond it the path is
   unresolved, as CG27's budgets require. Nested case labels stay unresolved.

## Predicted outcome (corpus unless named)

| Quantity | BEFORE (NULL-CANDIDATE-TRIAGE) | Predicted AFTER |
| --- | ---: | --- |
| Custody, all three routes | 915 / 9 / 3,910 `ded14632…` | byte-identical |
| Strong-source candidates searched | 139 | 139 |
| Findings | 33 | 27–32 (`copy_label_values` removed: a correction) |
| `null-on-allocation-failure` | 30 | 26–30 |
| `null-on-normal-path` | 3 | 1–2 |
| Null provisional Unsafe rows | 22 | 18–21 |
| Infeasible (removed) | 2 (kept) | 10–50, every one removed |
| Unresolved | 104 | 50–95 |
| — value not followed | 47 | 15–45 |
| — correlated condition | 32 | 5–30 |
| — loop iteration bound | 14 | 12–20 (exploration reaches more loops) |
| — switch over unfollowed value | 4 | 0–4 |
| — nested case label | 3 | 3 |
| — ordered pointer comparison | 3 | 0–3 |
| — correlated pointer test | 1 | 0–1 |
| — inexact path reaches NULL (new class) | 0 | 5–40 |
| Null Safe rows (uncond. + cond.) | 2,536 | 2,536–2,560 (removals only) |
| System, slice | no finding | no finding; slice's one unresolved unchanged or infeasible |
| Wall time | 476 s corpus | +0–10 % |

`copy_label_values`: predicted **infeasible** once the `cfl_list_size`
range contract is in force (the `s < 0` arm contradicts `>= 0`; the
`s > 0` arm reaches the dereference only through the checked `malloc`), and
**unresolved** without it.
