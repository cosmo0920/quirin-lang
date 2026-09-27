# W46-ACCESS-PATH-CENSUS: every access leaf by its Clang-typed access path, and a per-function prediction

Card W46-ACCESS-PATH-CENSUS, 2026-09-27, base `9bce3986`. It is P1 of the
[unsupported-coverage plan](../../plans/unsupported-coverage/README.md).

This is a **measurement card**:
- no rule, contract or Lean change;
- the corpus arbiter stays byte-identical.

The frozen forecast that P2 and P3 are measured against is
[PREDICTION.md](PREDICTION.md).

## 1. Result

The plan's upper bound was *at most 879 functions* if `IND`, `COND` and `RET`
all went. On this baseline it is 877, or 893 with the do-while and switch
conditions counted. Clang's typed AST replaces that bound with a prediction:

| Scope | Freed | Predicted Safe | Predicted Unknown | Predicted provisional Unsafe |
|---|---:|---:|---:|---:|
| P2 (statements) | 45 | **38** | 7 | 0 |
| P3 (plus conditions, returns, call arguments) | 220 | **83** | 137 | 0 |

**Where the 893 go (P3).** Each function is counted once, by precedence:

| Fate | Functions |
|---|---:|
| Freed | 219 (83 Safe, 136 Unknown) |
| A refused path | 286 |
| An unserved residual call | 326 |
| A condition or return refused for a non-access reason | 44 |
| An actual at a custody-taking position | 18 |

The refused paths split as:
- pointer-valued store 107;
- union 64;
- several causes 41;
- aggregate value 26;
- call in the path 20;
- indirect call 20;
- cast 6;
- other 2.

The unserved residual calls split as:
- 111 to another unit only;
- 110 to an Unknown callee in the same unit only;
- 34 external only;
- the rest mixed.

**Unknown predictions.** Every predicted Unknown is a **tracked root with no
dominating null test**. The analyzer may bind such a root to a resource
identity, so the obligation needs one known non-null live identity. The
precedent is W44-INDIRECT-CHAIN-ACCESS, which exchanged all 50 such leaves for
`unresolved resource identity`.

**Callee closure.** It adds almost nothing on its own:
- P3's same-unit closure adds one Unknown function in round 1.
- A cross-unit summary route would add 3 Safe and 5 Unknown.
- The ceiling is 227 + 15 in two rounds. It counts every freed function as
  Safe and serves cross-unit callees.

**The larger pool is call-blocked.** 1,178 Unknown functions have every access
leaf expressible and are blocked only by calls:
- 870 by in-corpus callees alone;
- 78 by external callees alone.

Their in-corpus callees are blocked by their own non-access families, such as
`unknown_alias` and pointer stores.

## 2. Mechanism

### Census stream

One census stream was added, and no scanner.

**Hooks.** `crates/qcore-c-gateway/src/access_path_census.rs` is a new module
behind the off-by-default `indirect-census` feature. It has two thin hooks in
`legacy_c.rs`, one in each unit entry point, right after `next_unit()`.

**Output.** With `QCORE_ACCESS_PATH_CENSUS` naming a directory, it writes for
each translation-unit analysis:
- the exact preprocessed text the analyzer reads, as `unit-<seq>.c`. Its lines
  are the leaves' internal `line`, constraint 12a.
- one line in `units.jsonl`: the unit ordinal, the dialect and the frontend
  flags.

**Guard.** The stream is listed in `census_sink_variables()`, so a census run
analyses one unit at a time. It numbers files by the global unit ordinal.

### Typed AST

[`scripts/access_census.py`](scripts/access_census.py) gives each text to
Clang with exactly the argument vector
`qcore_c_gateway::typed_c_ast::translation_unit_ast` builds:

```
--driver-mode=gcc FLAGS -x c DIALECT -fsyntax-only -Xclang -ast-dump=json -
```

So the typed AST is the gateway's own transport of the analysed text. Nothing
is read from the original sources, and `build/cycle19` is only read.

### Join

**Units.** A unit ordinal matches the document unit whose function set it
contains, uniquely. Five units define no function.

**Leaves.** A leaf `(unit, function, line, family)` binds to that function's
constructs at that line, of its kind:
- a statement or `for` clause for `IND`;
- the controlling expression of the matching statement for `COND`;
- a `return` for `RET`.

An `IND` leaf at `return f(a->b)`, or at a desugared condition, binds to that
construct, because its call is lowered at that line.

**Refusal to publish.** The join exits 3 and writes `unmatched.json` when any
of these holds:
- a leaf has fewer constructs than leaves at its coordinate;
- a construct is bound twice;
- a unit or function has no unique match;
- Clang rejects a text.

**Result.** 180 of 180 function-bearing units matched, over 181 analyses.
All 26,028 leaf groups (26,028 leaves) bound, with 0 unmatched rows.

### Paths

Each construct's expression is walked for maximal lvalues built from `.f`,
`->f`, `*` and `[i]`, skipping unevaluated operands. Each path records:
- **the root:** formal, local, global (file scope or `static` local),
  call result, or other;
- **the steps:** each with Clang's type, typedefs resolved by the unit's own
  typedef table;
- **the value:** scalar, pointer, function pointer, aggregate or address;
- **the access:** read, store, null store, increment or address;
- **the position:** statement, condition, return or call argument.

Two forms are normalised:
- `(&E)->f` is `E.f`, a same-object step with no hop.
- `*&E` is `E`.

**Named refusal causes:**
- `union`
- `bitfield`
- `incomplete-record`
- `cast`
- `pointer-arithmetic`
- `pointer-step`
- `volatile-atomic`
- `pointer-valued-store` (P5)
- `pointer-field-step`
- `call` (a call-result root, or a call in an index)
- `indirect-call`
- `aggregate-value`
- `statement-expression`
- `static-local`
- `root:<kind>`
- `address-of-base`

### Prediction

[`scripts/predict.py`](scripts/predict.py) applies PREDICTION.md §1–2. The
analyzer's own `may_bind` premise comes from the lexical census streams of the
same run: statement, indirect, return and chain.

The condition-desugaring census codes decide which `COND` leaves are access- or
call-refused.

### Inertness

The census run
[artifacts/census-run/identity.json](artifacts/census-run/identity.json), of
the feature build at `8322cd0e`, reproduces the baseline:
- **byte-identical:** diagnostics, chunk-contracts and source;
- **contracts:** identical after the checkout-root alias;
- **source-effects:** differs only in the digests that bind the checkout, as
  W45-SR5-SOUNDNESS and W46-EXTERNAL-CALLEE-CONTRACTS recorded.

The result is `verdict_bearing_identical: true`.

### Determinism

The whole pipeline ([`scripts/pipeline.sh`](scripts/pipeline.sh),
`PYTHONHASHSEED=0`) was run twice. `report.txt`, `census.json` and
`classes.json` were byte-identical.

## 3. The census

Leaves are grouped per `(function, line, family)`. There are 26,028 leaves in
26,028 groups: several constructs at one line (a macro expansion) explain one
leaf together.

| Family | Leaves | P2-expressible | P3-expressible | Refused | No typed access |
|---|---:|---:|---:|---:|---:|
| `IND` | 17,042 | 4,423 | 8,479 more | 4,140 | 462 |
| `COND` | 8,682 | — | 7,983 | 699 | 1,340 |
| `RET` | 304 | — | 170 | 134 | 43 |

The no-typed-access leaves are counted inside the expressible and refused
columns.

**Positions.** Half of `IND` is a call argument:
- 8,635 `IND` leaves have paths only in call arguments;
- 1,622 have paths in call arguments and in the statement;
- 6,323 are statement-only.

That is why P2 (statements) frees few functions.

**Paths.** There are 69,611 paths and 107,510 steps. **Every step has a Clang
type.** 63,698 paths (91.5%) are expressible.

Roots:

| Root | Paths |
|---|---:|
| formal | 47,291 |
| local | 21,655 |
| global | 385 |
| call result | 243 |
| other | 37 |

**Steps:** `->` 76,089, `.` 24,929, `[i]` 4,534, `*` 1,958.

**No typed access.** 1,845 leaves carry no typed access. The lexical gate read
a non-access as one:
- a declarator `*` in `const T *a = _a, *b = _b;`;
- a `sizeof(*p)` operand;
- a multiplication;
- a cast `(char **)&end`;
- an array declarator `uint8_t buf[78];`.

The COND and RET ones are refused for a call, an assignment or a type
operator, not for an access.

**Refusal causes.** Each row counts the leaves carrying the cause, and the
functions whose only P3 blocker it is
([report.txt](artifacts/report.txt)):

| Cause | Leaves | Sole-blocked functions |
|---|---:|---:|
| `pointer-valued-store` (P5) | 2,553 | 44 |
| `union` | 1,641 | 53 |
| `aggregate-value` | 335 | 18 |
| `call` in the path | 240 | 17 |
| `indirect-call` through a member | 185 | 17 |
| `statement-expression` | 128 | 0 |
| `cast` | 110 | 3 |
| `static-local` root | 87 | 1 |
| `pointer-arithmetic` | 33 | 1 |
| `pointer-step` | 22 | 0 |
| literal or computed roots | 30 | 0 |
| `address-of-base`, `pointer-field-step`, `volatile-atomic` | 4 | 0 |
| `bitfield`, `incomplete-record` | **0** | 0 |

Clang never needs `incomplete-record` or `bitfield` on this corpus:
- A member selection requires a complete type, so an incomplete record cannot
  appear at a member step.
- No refused access reads a bitfield.

## 4. The registered classes, re-measured and split

Each class is sized on this baseline by **its own registering script, run
unchanged** over this card's census run
([`scripts/registered_classes.sh`](scripts/registered_classes.sh)):
- W43's `residuals.py`;
- W44-INDIRECT-CHAIN-ACCESS's `chain_census.py`, grouped as its `residuals.py`
  groups readers;
- W44-BRANCH-CONDITION-CALLS's `residuals.py`;
- W44-INDIRECT-CALL-ARGUMENT-ACCESS's `family_residuals.py`.

[`scripts/classes.py`](scripts/classes.py) then joins each class's
`IND`/`COND`/`RET` leaves to the typed rows
([classes.json](artifacts/classes.json)).

The last column counts the class's leaves in families outside this census:
store-through, unrecognized destination, assignment and increment forms.

| Class | Leaves / Unknown functions / sole | Typed: P2 / P3 / refused (top causes) | Outside |
|---|---|---|---:|
| `W43-INDIRECT-UNTYPED-SINGLE-ACCESS` | 2,771 / 1,071 / 69 | 1,711 / 0 / 622 (pointer store 434, aggregate 107, union 100) | 435 |
| `W44-CHAIN-NESTED-AGGREGATE` | 1,442 / 548 / 33 | 455 / 450 / 489 (union 424, pointer store 111) | 48 |
| `W44-CHAIN-TYPEDEF-ROOT` | 588 / 302 / 17 | 287 / 87 / 159 (pointer store 97, union 77) | 55 |
| `W44-CHAIN-UNDECLARED-ROOT` | 222 / 63 / 4 | 85 / 79 / 51 (union 36, pointer store 18) | 7 |
| `W44-CHAIN-INCOMPLETE-RECORD` | 224 / 159 / 10 | 89 / 91 / 39 (union 23, pointer store 9) | 5 |
| `W44-CHAIN-CALL` | 389 / 258 / 15 | 3 / 252 / 131 (union 108, indirect call 36) | 3 |
| `W43-INDIRECT-POINTER-VALUE-STORE` | 350 / 239 / 25 | 10 / 0 / 263 (pointer store 263) | 77 |
| `W43-INDIRECT-OTHER-STATEMENTS` | 1,414 / 706 / 37 | 649 / 2 / 148 (pointer store 101, union 23) | 610 |
| `W44-CONDITION-CALL-INDIRECT-ARGUMENT` | 1,729 / 648 / 19 | 0 / 1,454 / 275 (pointer store 208, union 44) | 0 |
| `W44-CALL-RESULT-INDIRECT-DESTINATION` | 943 / 513 / 28 (`destination`) | typed reading, a store through a path whose value is a call: 1,592 leaves, 101 / 481 / 1,010 (pointer store 965, union 127) | — |

**Where Clang types what the lexical reader could not:**

- **The four reader-typing chain classes.** These are root-untyped,
  root-undeclared, incomplete record and dot link.
  - Clang types every step. 1,623 of their 2,361 typed leaves are expressible.
  - Every remaining refusal is semantic, never a type. The causes are a union
    member, a pointer-valued store or an aggregate value.
  - `W44-CHAIN-INCOMPLETE-RECORD`'s records are all complete to Clang. 180 of
    its 219 typed leaves are expressible, and 39 are refused, 23 of them for a
    union.
- **Single accesses.** The lexical reader left 2,333 of them untyped
  (`W43-INDIRECT-UNTYPED-SINGLE-ACCESS`). Clang types all of them: 1,711 are
  P2-expressible statement accesses, and 622 are refused.
- **Pointer-value stores.** 10 of the 273 typed leaves are not pointer stores
  to Clang: they are scalar or null stores.

## 5. The prediction

[PREDICTION.md](PREDICTION.md) freezes the rules, the counts per scope and per
library, the full P2 list and the P3 Safe list.
[artifacts/prediction.json](artifacts/prediction.json) holds every freed
function with its reason and obligations.

**The Safe arms:**
- **P2:** 22 have no pointer obligation, 18 of them no typed access at all, and
  16 have only scope-discharged hops.
- **P3:** 28 have no pointer obligation, 54 have scope-discharged hops only, and
  1 rests on a null-tested tracked root.

No provisional Unsafe is predicted. The census finds no tracked root released
earlier in its block before an access.

## 6. Callee closure (criterion 4)

Callees are split by where their body is:
- the same unit, which the existing summary rule serves when the callee is
  Safe;
- a header body outside the audited scope;
- another unit, which no rule serves today (`W44-CALLEE-CROSS-UNIT-SUMMARY`);
- external;
- an indirect call.

| Scope / variant | Rounds | Freed (Safe / Unknown) |
|---|---|---|
| P2 same-unit | 1 (round 1: 0) | 45 (38 / 7) |
| P2 cross-unit | 1 | 48 (41 / 7) |
| P3 same-unit | 2 (round 1: +1 Unknown) | 220 (83 / 137) |
| P3 cross-unit | 2 | 228 (86 / 142) |
| P3 ceiling (every freed function Safe, cross-unit served) | 3 (227, +15, +0) | 242 |

**Why the closure is shallow.** It is stopped by the callees' own blockers,
not by accesses. The 870 functions blocked only by in-corpus callees after P3
wait on these (functions unblocked alone):

| Callee | Functions | What blocks the callee |
|---|---:|---|
| `cJSON_New_Item` | 10 | a call through a member |
| `LZ4_decompress_generic` | 9 | 10 `unknown_alias` |
| `cmt_atomic_load` (cross-unit) | 8 | its own call |
| `cmt_opts_compare` | 6 | freed Unknown, on an unguarded root |
| `cmt_atomic_store` (cross-unit) | 6 | its own call |
| `cmt_mpack_unpack_array` and `ctr_mpack_unpack_array` (cross-unit) | 5 each | `unknown_alias` |
| `rd_kafka_timer_start0` (cross-unit) | 5 | pointer stores |

**External callees (for P4).** 78 functions are blocked only by external
callees after P3. By functions unblocked alone:

| Callee | Functions |
|---|---:|
| `strcasecmp` | 5 |
| `cfl_sds_cat_safe` | 4 (declined in W46-EXTERNAL-CALLEE-CONTRACTS: in-place realloc) |
| `flb_routes_mask_get_bit` | 3 |
| `strncmp` | 3 |
| `call_once`, `cfl_arena_calloc`, `cfl_kvlist_count`, `pthread_getspecific`, `pthread_key_create` | 2 each |

The full ranking is in [census.json](artifacts/census.json) under `closure`.

These are counted **after** P3. W46-EXTERNAL-CALLEE-CONTRACTS ranked the
external callees on the current baseline, where access leaves still block.
Its nine contracts are in this baseline.

## 7. Recommendation

### 1. P2 scope, in order of Safe-predicted functions freed

[report.txt](artifacts/report.txt) has the greedy order. The P2 steps, with
cumulative totals:

| Step | Adds | Cumulative |
|---|---:|---:|
| The typed answer replaces the lexical gate's access detection (no path means no access) | 18 | 18 |
| `formal->f` store | 5 | 23 |
| `formal->f` read | 4 | 27 |
| `formal->f.g` store | 3 | 30 |
| `formal->f.g.h` store | 2 | 32 |
| `local.f` store | 2 | 34 |
| `formal->f->g.h` store, `global.f` store, `global.f[i]` store | 1 each | 37 |
| `formal->f[i]` store together with `formal[i]` read | 1 | 38 |

**P2 alone is small: 38 Safe.** The statement position holds only 6,323 of
17,042 `IND` leaves, and most functions also have a condition, return or
call-argument leaf.

### 2. Conditions with P2, not after it

The P3 greedy order interleaves the positions from its first step:
- `formal->f` read at a return: 6;
- `formal->f` read in a condition: 5;
- `formal->f` read or store in a statement: 5 and 6.

P3 predicts 83 Safe against P2's 38. The per-hop obligation is the same in
every position; only the evaluation order differs (plan P3).

**Recommendation:** implement the typed layer once for statements, returns and
call-free conditions. Keep call arguments, which need the custody-position
check, as the second step.

### 3. Refusal causes to keep

| Cause | Leaves | Sole-blocked | Decision |
|---|---:|---:|---|
| `pointer-valued-store` | 2,553 | 44 | Keep. It is P5, custody through fields. |
| `union` | 1,641 | 53 | Keep. The member read is not the object's type. |
| `aggregate-value` | 335 | 18 | Keep. A copy carries pointer members. |
| `indirect-call` | 185 | 17 | Keep. The target is unmodelled. |
| `call` in the path | 240 | 17 | Keep. The call's own route decides. |
| `cast` | 110 | 3 | Keep. |
| `pointer-arithmetic` and `pointer-step` | 55 | 1 | Keep. They belong to W43-INDEX-TRAVERSAL's route. |
| `static-local` | 87 | 1 | Keep. The object outlives the call (constraint 10). |
| `statement-expression` | 128 | 0 | Keep. |

`bitfield` and `incomplete-record` stay as refusals. They cost nothing here.

### 4. Parity plan (hard invariant 3)

The typed layer must reproduce, before any route is retired, the pinned
verdicts and coverage of these routes. Each is listed with its gateway test and
Lean module:

| Route | Report | Gateway test | Lean module |
|---|---|---|---|
| W42 `ScalarMemberStore` (5g) | `w42-indirect-access` | `scalar_member_store.rs` | `QCore.ScalarMemberStore` |
| W43 typed scalar access | `w43-indirect-access-general` | `indirect_scalar_access.rs` | `QCore.IndirectScalarAccess` |
| W44 member chains | `w44-indirect-chain-access` | `indirect_chain_access.rs` | `QCore.IndirectChainAccess` |
| W45 sub-object stores (5i second paragraph) | — | `sub_object_stores.rs` | `QCore.SubObjectStore` |
| W43 index traversal, for its typed conditions and stepped reads | — | `index_traversal.rs` | `QCore.IndexTraversal` |

**Refusals the layer must keep.** Its disagreement log must show none of these
admitted:
- constraint 5i's sub-object refusal (`assignment_destination_suffix.rs`,
  `QCore.AssignmentDestinationSuffix`);
- 5k's member-release premises;
- W45-SR5-SOUNDNESS's pins, including `SCALAR-READ-PARENTHESISED-DOUBLE-DEREF`.

**For P3,** add these routes:

| Route | Gateway test | Lean module |
|---|---|---|
| W44 call-argument access | `call_argument_access.rs` | `QCore.CallArgumentAccess` |
| 5h condition desugaring | `condition_call_desugar.rs`, `branch_condition_calls.rs` | `QCore.ConditionCallDesugar` |
| 5j return families | — | `QCore.ReturnExpressionFamilies` |

**Retirement.** Each shape route is retired only after the layer's admitted
leaf set contains that route's admitted set on the corpus arbiter, with the
diff archived. The admitted sets are the routes' attribution files.

## 8. Limits

- **The guard is lexical.** A tracked root counts as guarded when a controlling
  expression at an earlier line null-tests it after its last assignment. No
  dominance is computed. Only 1 Safe prediction depends on it.
- **`may_bind` comes from the lexical streams.** A root no stream names is
  treated as tracked (fail-closed). This covers 30 of the 137 P3 Unknown
  predictions.
- **Contracts are read by name.** A callee counts as contracted when any
  selected module declares it. A header body outside the audited scope counts
  as served when no uncontracted-call leaf of its unit names it.
- **Flow effects are not predicted.** A refused statement's blanket frame
  (constraint 10a) or a newly exposed leak can move verdicts either way. The
  prediction does not forecast them, and P2 and P3 must attribute them.

## 9. Records and gate

**Records:**
- this report, [PREDICTION.md](PREDICTION.md), and the artifacts and scripts
  beside them;
- the wave-46 increment paragraph;
- the plan's P1 status row;
- the ledger: ten amended entries and two registered residuals. None is
  retired, since this card discharges nothing (§10);
- a STATUS row;
- a regenerated `docs/reports/INDEX.md`.

**Gate.** [gate.json](gate.json) records the standing gate by
[`scripts/gate.sh`](scripts/gate.sh), with every exit in its own file. It also
records the census feature's own clippy and library tests, and the final
corpus arbiter's identity.

## 10. Follow-ups

**Amended** with this card's figures, none retired:
- the ten classes in §4;
- `W44-CALLEE-CROSS-UNIT-SUMMARY`, with the closure figure.

**Registered:**
- `W46-LEXICAL-GATE-NON-ACCESS`: 1,845 leaves carry no typed access (462 `IND`,
  1,340 `COND`, 43 `RET`). The `IND` ones alone free 18 P2-Safe functions once
  the gate takes the typed answer.
- `W46-UNION-MEMBER-PATH`: 1,641 leaves read or write through a union member,
  and 53 functions are blocked by that alone. The typed layer keeps them
  refused. A card would state which member is live.
