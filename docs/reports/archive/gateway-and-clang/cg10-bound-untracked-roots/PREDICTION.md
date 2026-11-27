# CG10-BOUND-UNTRACKED-ROOTS — prediction (frozen before the premise is consulted)

Frozen at the census step (`508b9a43`, parity only: eligibility on the typed
reader's chains, the premise's facts and answer per settled chain root, and the
lexical closure's reasons per name; `chain_owner` does not consult the premise).
Base `a235ead6` (= local `main`). The B08 soundness repair
(`B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED`, rule 11c) has not landed; the AFTER is
re-taken on the repaired base when it does, and reported against that base.

CG00's `predictions/CG10.md` lists CG10-COMMON-TYPED-PLACE's functions, not this
residual's; there is no CG00 function list for `CG10-BOUND-UNTRACKED-ROOTS`, so
this prediction is re-derived from the census alone.

## BEFORE (identity)

| Route | Runs | Digest | Totals |
|---|---|---|---|
| corpus (`66910c10`) | base, census | `2fc3af8c…` both | 602 / 10 / 4,222 |
| system (`046accf6`) | base, census | `062a990d…` both | 37 / 2 / 195 |
| tail-pause slice (`90978ac6`) | base, census | `e4e4197d…` both | 26 / 1 / 242 / 1 Pending |

The base reproduces the round's baseline digest `2fc3af8c…` byte for byte; the
census binary moves nothing on any route.

## The population (corpus, `scripts/census.py`)

A *root refusal* is an `unknown_alias` leaf naming a chain root the never-bound
premise does not cover, whose settled chain does not resolve. The census reads
every typed-reader chain on such a root (rule 10t's mixed places and W46's
arrow chains), so it is wider than CG10's increment of 2,550.

- **7,455 root-refusal leaves on 1,635 non-Safe functions.** Top roots
  `rkbuf` 851, `rk` 776, `rkb` 604, `rkcg` 382, `rktp` 331, `rko` 299 (CG10's
  top roots, reproduced in kind).
- **Binding at the access (first row)**: absent 5,955, nullable 959, tracked
  423, target-free 118.
- **Premise in the BEFORE state (first row)**: path-refused/absent 5,072,
  bound/nullable 917, ineligible/absent 444, holds/absent 426, bound/tracked
  410, path-refused/target-free 112, rooted-place 13. Holds (every row of the
  key): 336 leaves on 329 functions.

**Classes** (why the lexical closure binds the root; a root may carry several):

| Class | Leaves | Functions | Freed alone |
|---|---|---|---|
| (a) non-`Borrowed` actual of a contracted callee (released 4,014 / owned 128 name-reasons) | 458 | 164 | 0 |
| (b) actual of an uncontracted callee (or beyond the declaration) | 5,973 | 1,223 | 0 |
| (c) assignment from a call (uncontracted 12,397 / none 2,951 / owned 2,070 / derived 176 name-reasons) | 1,514 | 518 | 0 |
| (d) address taken | 153 | 40 | 0 (stays refused) |
| (e) other (compound, increment, assignment from a bound name, protected formal, rule-10o nested) | 2,244 | 740 | 0 |
| bound by the Clang AST closure only (no lexical reason) | 89 | 49 | 2 (upper) |

**Functions freed alone.** Only **3** non-Safe corpus functions have root
refusals as their only leaves, whatever premise is granted:
`mk_fifo.c::mk_fifo_queue_get` (`q`) and `mk_plugin.c::mk_plugin_cap`
(`plugin`), both path-refused in the BEFORE state (the refusal on the path is
another root refusal), and `rdkafka_subscription.c::_share_invalid_topic_cb`
(`rktpar`, a protected formal, bound in some analyses). Every other function
carrying a root refusal carries another leaf: the class-(b) roots are, by
construction, in functions that call an uncontracted function (the P4
callee-closure work), and the flow reports that call.

System slice: 193 leaves on 70 functions, 2 functions with only root refusals
(`sb_find_segregated_backlog_by_output_plugin_instance`,
`flb_engine_get_chunk_backpressure_percent`, both AST-only roots). Tail-pause
slice: 341 leaves on 92 functions, 1 (`flb_input.c::get_collector`).

CG11's identity-free stores: 404 owner-root refusals keyed by statement, 6
admitted in the BEFORE state. CG04's typed call actuals are refused at
classification (rule 10k(3)), so the census has no row for them.

## The premise (what is implemented next)

Eligible root (pointer, declared once as a parameter or automatic local, never
address-taken) + absent or known non-null target-free binding in an unwidened
state + no bound place rooted at it + **no refusal or unresolved effect on any
reaching path** (`unmodeled`). The last conjunct is deliberately path-wide, not
per root: every custody-taking position receiving an untracked object and
every uncontracted call reports there, so it covers aliases, at the cost of
also ending the premise after unrelated refusals. It frees no function the
per-root form would free (a function with any refusal is Unknown), but it
lifts fewer leaves. Rule 10k(3) is widened to leave a pointer read at a
`Borrowed` or data-tail actual on an eligible chain root to the flow; an
address actual keeps the never-bound premise.

## Predicted outcome (upper bounds; AFTER on the repaired base)

| Measure | Prediction |
|---|---|
| Corpus Unknown → Safe | **2** (band 0–5): the two AST-only roots above, plus at most a callee-summary cascade |
| Corpus Safe → non-Safe | 0 (the rule only adds) |
| Provisional Unsafe added | 0–5: a path no longer poisoned by a lifted refusal may expose an exit leak the flow withheld; each is read |
| `unknown_alias` root-refusal leaves removed | 336 at least; point 700, band 336–2,000 (a lifted refusal un-poisons the rest of its path; uncontracted calls keep most paths refused) |
| `unsupported` indirect-access leaves moved by the rule 10k(3) widening | 100–1,500 (an admitted actual becomes a flow obligation: admitted, or an `unknown_alias` on a refused path) |
| CG11 stores admitted | 6–60 of 470 |
| System slice | 0–2 status moves |
| Tail-pause slice | 0–1 status moves |

A miss below the band is not a defect: the measured decision is that this
premise's yield is capped by the uncontracted calls that share its functions.
