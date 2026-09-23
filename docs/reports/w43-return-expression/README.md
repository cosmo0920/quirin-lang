# W43-RETURN-EXPRESSION — the refused return expressions, measured and re-attributed

Card W43-RETURN-EXPRESSION, 2026-09-23, on base `ffdddbbb` (contains
`d7b39fac`). The W42 blocker census reported that the return-expression refusal
reaches 1,109 of the 4,527 Unknown projected functions and is the only distinct
blocker of 465 — the largest sole-blocked lever in that census. This card
measures what those refusals actually are, lands the one family whose return
semantics can be made sound, and refuses the rest by name.

## Headline

* **The lever is mostly not a return-position question.** Measured in the
  analyzer's own token view, **952 of the 1,440** refused return leaves on
  Unknown functions (732 functions) are one call spanning the whole
  expression to a callee with **no contract in force**; another 290 are call-free
  accesses refused for their operands' binding or typing. Of the 465
  sole-blocked functions, 327 return an uncontracted call and 75 read through
  an operand the typed-access rules refuse. No return-position rule can make
  those functions `Safe`: the call needs a contract or validated summary
  (constraint 4) and the operands need the indirect-access and typing cards.
* **Admitted family (re-attribution, not promotion).** A return expression that
  is exactly one call `NAME ( … )`, whose synthetic assignment
  `PLACE = NAME ( … );` the ordinary statement classifier **refuses**, is
  lowered to that refusal plus a `NoOp` exit — the same pair
  `T = NAME(…); return T;` produces. The published reason now names the real
  blocker; the node stays a refusal, so nothing is admitted, escaped,
  discharged or certified.
* **Arbiter.** BEFORE reproduces the W42 diagnostics **byte for byte**. AFTER
  exchanges **941** return-position leaves for **941** classifier refusals at the
  same internal coordinate (671 `call to uncontracted function`, 267 lexical
  `indirect memory access`, 2 `braced initializer`, 1 `nested call`), across 726
  Unknown functions. **0 function and 0 unit status moves**; every category count
  is unchanged (unsupported 49,371, unknown_alias 4,665, leak 272,
  summary_conflict 1). Attribution is complete and mechanical.
* **Blocker census after.** The return refusal now reaches **422** Unknown
  functions (499 leaves) and is the sole blocker of **145**. Of the 465
  previously sole-blocked functions, 317 lose it: 237 are now blocked only by
  uncontracted calls, 77 only by the lexical indirect-access gate, 3 by both or
  a braced initializer. All 465 stay `Unknown`. Functions whose only blocker is
  one named uncontracted callee grow from **88 to 315** — a contract-card
  worklist ([sole-callees.txt](sole-callees.txt)).
* **Finding.** `report_leaks` runs at every exit; a return refusal never
  suppressed a leak accusation, contrary to constraint 7's wording and one Lean
  model (§8, `RETURN-REFUSAL-LEAK-REPORT-PREMISE`).
* **Follow-ups 276 → 282**: six refused families registered, one finding
  registered, `RETURN-CALL-INCOMPLETE-LINE-455` retired against this census.

| Measure | BEFORE (W42) | AFTER | Delta |
|---|---:|---:|---:|
| Safe / provisional Unsafe / Unknown functions | 223 / 84 / 4,527 | 223 / 84 / 4,527 | 0 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 49,371 | 49,371 | 0 (941 exchanged) |
| `unknown_alias` / `leak` / `summary_conflict` | 4,665 / 272 / 1 | 4,665 / 272 / 1 | 0 |
| return-expression leaves (Unknown functions) | 1,440 | 499 | −941 |
| functions reached / sole-blocked by the return refusal | 1,109 / 465 | 422 / 145 | −687 / −320 |
| `call to uncontracted function` leaves / functions / sole | 11,350 / 2,444 / 151 | 12,021 / 2,796 / 414 | +671 |
| `indirect memory access …` leaves / functions / sole | 17,430 / 3,149 / 271 | 17,697 / 3,317 / 363 | +267 |

The last two rows matter to other cards: the indirect-access population the
queued general indirect-access card inherits grows by **267 leaves** that
were previously hidden behind the return wording (each is `return f(… p->x …)`
where the classifier's lexical gate fires on the argument first).

## 1. Baseline reproduction

`scripts/run_audit.py` runs the W42 command shape (`--pinned-evidence`, the
pinned Fluent Bit source and Monkey compile database, `QCORE_CLANG=/usr/bin/clang`,
the wave-20 ChunkIO package, `QCORE_SOURCE_CONTRACTS` unset) with this
worktree's release binary. [before-run.json](before-run.json): binary
`de8c12d2…10f46a`, exit **0**, 03:31:30Z–03:36:53Z. The binary was built from
base `ffdddbbb` with the census hooks of commit `64621cc2` already in the tree
but compiled out (feature off); the `source_commit` field names the base.

[before-identity.json](before-identity.json):

| Artifact | vs W42 archive | vs canonical-contract-locations checkpoint |
|---|---|---|
| diagnostics (verdict-bearing) | **byte-identical** | byte-identical |
| ChunkIO contracts | differs | **byte-identical** |
| Monkey source | differs | **byte-identical** |
| selected contracts | differs | identical after worktree-path alias |
| source effects | differs | differs only in identity digests |

The four non-verdict differences from W42 are not this card's: commit
`65157c4c` (canonical contract migration) moved contract sources, which changed
`contract/profiles/fluent-bit/profile.json` and therefore every
`profile_digest` (3 in the ChunkIO sidecar, 19 in the Monkey sidecar), the
selected-contract module paths, and the source-effect identity digests. That
migration's own report measured the same raw digests
(`docs/reports/canonical-contract-locations/corpus-bytes.json`), which the
table's right column reproduces. The selected-contract keys additionally embed
this worktree's path, and the source-effect `resource_contracts:<digest>` keys,
`build_digest`, `configuration_digest`, `task_digest` and `obligation_id`
values hash it ([`scripts/json_leaf_diff.py`](scripts/json_leaf_diff.py): 41
differing leaves, all identity digests or the same values under a re-keyed
digest). No verdict payload differs.

## 2. Measured shape distribution

### Method

The census reads the **analyzer's own** view, not source lines. The
off-by-default `indirect-census` feature gains a `QCORE_RETURN_CENSUS` stream
(`indirect_census::record_return`), written only after every return route has
already refused: the complete parser-delimited return expression, each call's
contract-in-force and in-unit definition, and the census's existing access-chain
facts (base binding, base-type cause, member class). With the feature on, the
census run reproduces all five BEFORE artifacts **byte for byte**
([census-run-identity.json](census/census-run-identity.json)); run identity
[census-run.json](census/census-run.json) (binary `6f07c408…`, exit 0; its
`source_commit` field names the base, the Rust tree is `64621cc2`).

[`scripts/return_census.py`](scripts/return_census.py) joins each record to the
published leaf by unit ordinal, function and internal line, de-duplicates
repeated passes over one body, and assigns every leaf exactly one family by an
ordered, total rule. **1,444 of 1,444** published leaves are covered (1,440 on
Unknown functions, 4 on provisional-Unsafe ones), 0 uncovered. Eight records
from derivation passes join no published leaf (three coordinates in
`rdkafka_transport.c` and `mk_string.c` whose final pass admitted the return
once a callee summary validated), and 7 leaves' passes disagree on whether a
recursive callee's provisional summary was in force; the last pass is used.
Both are disclosed, neither changes a family. Replay from the archives:
`python3 docs/reports/w43-return-expression/scripts/return_census.py docs/reports/w43-return-expression/census/return-census.jsonl.gz docs/reports/w42-priority-integration/artifacts/diagnostics.json.gz`
(exit 0, byte-identical to [census.txt](census/census.txt)).

### The 1,440 leaves on Unknown functions (1,109 functions)

Function columns count functions with at least one leaf in the row; a function
can appear in several rows.

| Family / detail | Leaves | Functions |
|---|---:|---:|
| **A. whole call, uncontracted callee** | **952** | **732** |
| &nbsp;&nbsp;callee defined in this unit | 635 | 465 |
| &nbsp;&nbsp;callee declared elsewhere | 290 | 253 |
| &nbsp;&nbsp;compiler builtin (`__atomic_*`, `__builtin_*`) | 4 | 4 |
| &nbsp;&nbsp;any of the above under a cast | 23 | 18 |
| **B. whole call, contracted callee the classifier refuses** | **13** | **12** |
| &nbsp;&nbsp;`None` result, refused for an argument | 11 | 10 |
| &nbsp;&nbsp;`Owned{nullable}` result under a cast | 2 | 2 |
| **C. compound expression containing a call** | **82** | **76** |
| &nbsp;&nbsp;an uncontracted call in arithmetic/comparison | 53 | 49 |
| &nbsp;&nbsp;conditional `?:` / short-circuit over a call | 27 | 25 |
| &nbsp;&nbsp;every call contracted | 2 | 2 |
| **D. type-operator constant** (`sizeof(LZ4_stream_t)`) | **6** | **6** |
| **E. call-free conditional or short-circuit** | **26** | **26** |
| **F. call-free access expression** | **290** | **264** |
| &nbsp;&nbsp;through a base this body may bind | 186 | 165 |
| &nbsp;&nbsp;through a base whose type the reader will not commit to | 44 | 43 |
| &nbsp;&nbsp;ending at an `Unresolved` member (typedef pointers) | 31 | 28 |
| &nbsp;&nbsp;ending at an aggregate-valued (union) member | 12 | 12 |
| &nbsp;&nbsp;other operand shapes | 17 | 17 |
| **G. other** | **71** | **36** |
| &nbsp;&nbsp;comma operator (nanopb `PB_RETURN_ERROR`) | 67 | 32 |
| &nbsp;&nbsp;statement expression or compound literal | 4 | 4 |

### The 491 leaves of the 465 sole-blocked functions

| Family | Leaves | Functions |
|---|---:|---:|
| A. whole uncontracted call (225 in-unit callee, 92 external, 4 builtin, 9 under a cast) | 348 | 327 |
| F. call-free access (27 base type, 22 unresolved member, 12 union member, 6 bound base, 8 other) | 75 | 75 |
| C. compound with a call | 41 | 39 |
| E. call-free conditional / short-circuit | 17 | 17 |
| D. type-operator constant | 4 | 4 |
| G. other (2 comma, 2 braces) | 4 | 4 |
| B. contracted callee under a cast | 2 | 2 |

Of the 635 uncast leaves whose callee is defined in the unit, 575 name an
`Unknown` body, 15 a provisional-`Unsafe` one and 45 a body the projection does
not publish; among the 246 such sole-blocking leaves the split is 227 / 4 / 15.
**None names a `Safe` body**, which is why constraint 4's validated-summary route
serves none of them (compare `RETURN-CALL-W4-REACH-BLOCKED`).

## 3. The admitted family and its semantics

**Shape.** `return NAME ( ARGS );` where `NAME` is the first token, its `(`
is the second and the matching `)` is the last
(`return_call::whole_expression_call`), `NAME` is not a type operator or
constant (`NOT_A_CALLEE`: `sizeof`, `_Alignof`, `alignof`, `__alignof__`,
`_Generic`, `typeof` forms, `__builtin_offsetof`,
`__builtin_types_compatible_p`, `__builtin_va_arg`, `__extension__`), the body
does not spell the reserved place `__qcore_returned_value`, and every earlier
route — the strict and pointer-leaf rules, the W4 whole-call desugaring and
both B01 routes — has already declined.

**Rule.** The classifier's answer for the synthetic `PLACE = NAME ( ARGS );` is
inspected. If it is `LegacyInstruction::Unknown`, the `return` lowers to that
node (interior) followed by a `NoOp` exit, and the site counter is restored.
Any other answer keeps the return-position refusal verbatim
(`return_call::lowers_refused_whole_call`, called last in
`Parser::return_expression_instruction`).

**Semantics and why it is sound.** `return f(a);` and `{ T = f(a); return T; }`
are the same C, and the gateway has decided the second form since Cycle 9; W4
already performs this desugaring for an admitted call. Here the classifier
refuses, so:

* the evaluation node is an `Unsupported` refusal at the `return`'s own line,
  so the path stays poisoned and the function cannot become `Safe`, be
  certified, or yield a validated summary — a function that loses the return
  refusal **moves on to its remaining obligations and is not thereby Safe**;
* the refusal carries the same bounded binding frame (`RefusedWrites`) over the
  same expression plus the reserved place, and never touches resource states,
  so the state at the `NoOp` exit is the one the old exit-node refusal left and
  `report_leaks` accuses exactly the same sites (§8);
* the `NoOp` exit escapes nothing, so no obligation is discharged on this route
  (invariant 1's forbidden direction is unreachable here);
* restoring `next_site` keeps site numbering and certificate identity unchanged.

**What it is not.** It admits no call, models no argument, applies no contract
effect and widens no indirect-access handling: when the classifier's lexical
indirect-access gate is what refuses, the lowered node is that gate's existing
refusal. It deliberately amends one prior decision: waves 33–34 kept the return
wording for an uncontracted callee so that an exchange could not be *published
as progress*. This card publishes it as what it is — attribution — with zero
verdict movement stated first. The W4 route's own admission set is unchanged.

## 4. Refused families, and why

Each remaining family keeps the return-position refusal verbatim; each has a
registered residual. Counts are Unknown-function leaves / functions (sole-blocked
leaves / functions).

| Family | Remaining | Why refused here | Residual |
|---|---|---|---|
| Whole call under a cast (23 uncontracted/builtin + 2 contracted `Owned`) or in parentheses (1) | 26 / 21 (11 / 11) | Needs positive cast recognition; the contracted half would be an *admission* escaping a fresh site through a conversion | `W43-RETURN-CALL-UNDER-CAST` |
| C. compound expression with a call | 82 / 76 (41 / 39) | No frequency-preserving decomposition for a call whose result feeds further computation or sits under `?:`/`&&` (constraint 5d) | `W43-RETURN-COMPOUND-CALL` |
| F. call-free access expression | 290 / 264 (75 / 75) | Operand binding (constraint 5), base-type reader and member typing — shared with every position; widening it is the general indirect-access and typing cards' work | `W43-RETURN-ACCESS-OPERANDS` |
| G. comma operator / braces | 69 / 34 (2 / 2) | `E1; return E2;` is exact, but `E1` is a pointer-member store through a bound base — indirect access | `W43-RETURN-COMMA-MACRO` |
| E. call-free conditional / short-circuit | 26 / 26 (17 / 17) | Refused for an operand, not the operator | `W43-RETURN-CONDITIONAL-OPERANDS` |
| D. type-operator constant | 6 / 6 (4 / 4) | The fixed type-query rule deliberately admits only scalar types; a record/union `sizeof` is a type-query widening | `W43-RETURN-TYPE-OPERATOR` |

Total remaining: **499 leaves over 422 Unknown functions** (plus 4 F-family
leaves on 3 provisional-Unsafe functions), matching the AFTER artifact exactly.

## 5. Arbiter AFTER and attribution

Implementation commit `29f4a907`; [frozen forecast](PREDICTION.md) committed
before the run (`0f0a0445`). AFTER run ([artifacts/run.json](artifacts/run.json)):
binary `6fa3adb7…0a27`, exit **0**, 03:54:45Z–04:00:13Z. Later commits change
formatting, comments, tests, scripts and documents only.

[`scripts/attribute.py`](scripts/attribute.py) keys attribution on the Rust
rule's own shape predicate applied to the BEFORE token view, and accepts a
delta only as a removed rule-shape return leaf, a same-coordinate classifier
refusal added in its place, or an absorption into an identical existing leaf.
Anything else, any status move and any unit-field change is unattributed and
fails the script. Result ([attribution-summary.json](attribution-summary.json),
per-coordinate list in `attribution.json.gz`): exit **0**,
`attribution_complete: true`.

* 941 BEFORE leaves have the rule's shape; **all 941** are removed and none is
  kept (so no corpus whole call reached the classifier-admits-but-W4-declines
  arm, e.g. a `Reallocated` result).
* 941 leaves are added, **each at the coordinate of a removed leaf**, 0
  absorbed: `call to uncontracted function` 671, `indirect memory access is
  outside the resource projection` 267, `braced initializer, expression, or
  declaration is not modeled` 2, `nested call … inside a modeled call` 1.
* 726 Unknown functions lose at least one return leaf; **0 function status and 0
  unit status or unit-field changes**; top-level fields unchanged; the four other
  artifacts are byte-identical to the same-checkout BEFORE
  ([after-identity.json](after-identity.json)).
* Leak pin **272**, unchanged: no leak leaf is added or removed in either
  direction, as the §8 finding predicts.

**Forecast comparison.** The forecast named 940 leaves; the rule lowered 941.
Two whole calls whose argument is a compound literal (`cmt_histogram.c:91`,
`rdkafka_request.c:3171`) were forecast in family G because the census
classifier tests for braces before calls; the Rust rule, correctly, sees whole
calls the classifier refuses. One forecast leaf, `lz4frame.c`
`return (LZ4F_decodeHeader(…));`, is a parenthesised whole call the Rust rule
reads literally and keeps refused. The forecast of 315 sole-blocked functions
losing every return leaf measured 317 for the same reason. All other forecast
statements held. The miss is recorded, not corrected into the forecast.

**Where the 465 sole-blocked functions stand** ([sole-moves.txt](sole-moves.txt)):
all 465 remain `Unknown`; 237 are blocked only by uncontracted calls, 77 only by
the lexical indirect-access gate, 2 by a braced initializer, 1 by both an
uncontracted call and indirect access, 3 by an uncontracted call plus a
remaining return refusal, and 145 still only by the return refusal.

The fresh [blocker census](blocker-census.txt)
(`python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-return-expression/artifacts`,
exit 0) and the single-callee worklist [sole-callees.txt](sole-callees.txt)
(`scripts/sole_callees.py`, exit 0; 315 functions over 235 callees, led by
`LZ4_decompress_generic` 9, `memcpy` 8, `strcmp` 6) size the next cards. Neither
is a promise: a contract carries its own effects and obligations.

## 6. Tests and negative controls

`crates/qcore-c-gateway/tests/return_refused_call_lowering.rs` drives the real
Clang and gateway (14 rows). The central harness requires the lowered
`return f(args);` to agree with the hand-written `int t; t = f(args); return t;`
on status, certification and every diagnostic kind, to publish the classifier's
refusal, to be neither `Safe` nor certified, and to name no reserved place — it
cannot pass by refusing.

* Re-attribution: uncontracted scalar and pointer calls; a contracted callee
  refused for its nested-call argument.
* Fail-closed: an owned resource handed to an uncontracted call is not escaped
  and is accused exactly as the kept return refusal accuses it (the cast
  spelling measures the old answer on the same body); a live obligation on the
  lowered path is accused identically; a leak on another path stands.
* Negative controls, each keeping the return refusal and non-`Safe`: a cast of
  the call, the call inside arithmetic, a conditional, a parenthesised comma,
  `sizeof (struct …)` (not misattributed to `sizeof`), a `Reallocated` result the
  classifier admits, and a body spelling the reserved place. An admitted W4
  whole call is unchanged and still accuses its leak.

Three existing rows pinned the old wording and are **deliberately re-pinned**,
each documented in place: `return_position_modeled_call.rs`
`an_uncontracted_callee_keeps_the_return_refusal` →
`an_uncontracted_callee_is_refused_by_the_call_classifier`, the nested-call body
of `a_call_that_is_not_the_whole_return_expression_is_refused`, and the
uncontracted-call row of `return_position_pointer_leaf.rs`
`the_adversarial_family_keeps_its_return_refusal`. Each still asserts refusal,
non-`Safe` and no certificate; only the wording moved. Like their neighbours,
the new rows early-return without Clang
(`RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` applies to them too).

## 7. Lean

`proof/QCore/ReturnRefusedCallLowering.lean`, registered in the new
`proof/AuditReturnRefusedCallLowering.lean` (audit files 165 → 166), 14
theorems, only `propext` or no axioms, no placeholders: the lowering replaces
only the return refusal (`the_lowering_only_replaces_the_return_refusal`),
never touches an admitted classification, another shape, a type operator or a
spelled reserved place; a lowered return is still poisoned, escapes nothing and
reports exactly the leaks the refusal reported; a function carrying one is never
`Safe` and the lowering never promotes; the site counter is restored; a witness
moves and its cast twin does not. These are abstract rule theorems; C
extraction and correspondence to `legacy_c.rs` remain constraint-28 premises.

## 8. Finding: a return refusal never suppressed a leak accusation

`report_leaks` is called for every exit node with no unmodeled check, and a
refusal never touches resource states. Measured in the real gateway: a body
`void *p = q_alloc(); return (int) z_take(p);`, whose return keeps the
return-position refusal, publishes `Leak` beside the `Unsupported` leaf and is
provisional `Unsafe`. Gateway constraint 7 ("`report_leaks` **skips** an
unmodeled exit"), the W4 module note on the `None` result, and
`QCore.ReturnPositionEscape.reported .refuse = false` state the opposite; what an
unmodeled path does change is the exit *residual* observation
(`observe_residual`'s `indeterminate`). W43's own model and tests state the
measured behaviour; the W4 module note now carries an amendment pointing here.
The other texts are standing rules or another card's model, so this card
registers `RETURN-REFUSAL-LEAK-REPORT-PREMISE` for the integrator rather than
editing them. It is also why the arbiter moved no leak leaf.

## 9. Validation

GATE-PLACEHOLDER

## 10. Ledger and status

* Registered: `W43-RETURN-CALL-UNDER-CAST`, `W43-RETURN-COMPOUND-CALL`,
  `W43-RETURN-ACCESS-OPERANDS`, `W43-RETURN-COMMA-MACRO`,
  `W43-RETURN-CONDITIONAL-OPERANDS`, `W43-RETURN-TYPE-OPERATOR`,
  `RETURN-REFUSAL-LEAK-REPORT-PREMISE` ([followups.json](followups.json)).
* Retired: `RETURN-CALL-INCOMPLETE-LINE-455` (registered; its discharge asks for
  a multi-line-aware classifier, and this census reads the parser-delimited
  expression and covers 1,444/1,444 leaves with none shape-unknown;
  [retired-followups.json](retired-followups.json) keeps the entry verbatim).
* Follow-ups **276 → 282**; `dimensions.named_followups` regenerated to match.
* `corpus.cycle19_raw_units` now names this report's artifacts; its exact
  previous contents are [previous-corpus-row.json](previous-corpus-row.json).
  `docs/STATUS.md`'s corpus rows point at this baseline; all counts are
  unchanged. The W42 artifacts are frozen as the predecessor and not edited.
* `RETURN-POSITION-SUBSHAPE-B`, `RETURN-POSITION-CONDITIONAL` and
  `RETURN-POSITION-TYPED-MEMBER-REACH` stay open: this card neither lands an
  effectful return instruction nor answers their counterfactual questions; the
  census refines the latter two populations.

## 11. For the integrator

Proposed gateway rule (constraint **5h**), in the style of 5g:

> **W43-RETURN-EXPRESSION lowers a refused whole-call return to its own
> refusal.** When every admitting return route has declined, a return
> expression that is exactly `NAME ( … )` (first and last token; `NAME` not a
> type operator or constant) in a body that does not spell the reserved place
> is classified as the synthetic `PLACE = NAME ( … );`; only if the statement
> classifier answers `Unknown` does the `return` lower to that node followed by
> a `NoOp` exit, with the site counter restored. This re-attributes and never
> admits: the node stays `Unsupported`, the exit escapes nothing, and a
> function that loses the return wording is not thereby `Safe`. Casts,
> parenthesised calls, compound expressions and every non-refused
> classification keep the return-position refusal.

The contracts artifact in `artifacts/` embeds this worktree's path in its 33
source-map keys, as the W42 isolated card's did; a same-checkout re-run at the
root is the proper byte-identity test for that one artifact. Constraint 7's
leak-pin chain can record W43 as an admission-shaped card that, by the §8
finding, could not move the pin.

## 12. Remaining work

The six residuals above, in order of sole-blocked reach: the typed-access
operand families (75 sole, `W43-RETURN-ACCESS-OPERANDS`), compound calls (39),
conditionals (17), casts (11), type operators (4), comma (2). The larger lever
is not in the return position at all: 237 formerly sole-return-blocked
functions now wait only on contracts or validated summaries for named callees,
and 77 on the lexical indirect-access gate.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <run> docs/reports/w42-priority-integration/artifacts --worktree-alias
python3 docs/reports/w43-return-expression/scripts/return_census.py docs/reports/w43-return-expression/census/return-census.jsonl.gz docs/reports/w42-priority-integration/artifacts/diagnostics.json.gz
python3 docs/reports/w43-return-expression/scripts/attribute.py docs/reports/w42-priority-integration/artifacts/diagnostics.json.gz docs/reports/w43-return-expression/artifacts/diagnostics.json.gz docs/reports/w43-return-expression/census/census.json.gz
python3 docs/reports/w43-return-expression/scripts/sole_moves.py docs/reports/w42-priority-integration/artifacts/diagnostics.json.gz docs/reports/w43-return-expression/artifacts/diagnostics.json.gz
```

Each exits 0 on the archived inputs. A census rerun needs a release build with
`--features qcore-c-gateway/indirect-census` and `run_audit.py … --census`.
Private raw outputs: `measure/before/`, `measure/census/`, `measure/after/` in
this worktree.
