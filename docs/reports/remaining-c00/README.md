# C00 — Classify the six CMetrics Unknown functions — 2026-09-07

Blocker inventory for the six **Unknown** functions of the pinned
`cmetrics/lib/cmetrics/src/cmt_counter.c` translation unit, with a ranked
blocker-family table and a single recommendation for card C02.

This card **classifies why analysis declines**. It promotes no verdict, asserts
no upstream defect, and changes no analysis, checker, contract, profile or test
code. The machine-readable rows are in [`inventory.json`](inventory.json).

## Verdicts carried forward unchanged

| Item | Value | Changed by C00 |
|---|---|---|
| Checked function contract | **1** — `cmt_counter_allow_reset` | no |
| Source-contract Unknown | **6** | no |
| Raw unit resource verdict | **provisional Unsafe**, unpromoted | no |
| Resource diagnostics | leak=2, unknown_alias=2, unsupported=36 | no |
| CT coverage | 11/50 mapped | no |

The canonical audit was re-run at this checkout and reproduced the established
baseline exactly (`logs/cmetrics_audit.log`):

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=36
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

The audit read the shared prepared build at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-{source,production}`
and wrote its report into the session scratchpad, so nothing in that tree was
modified. `source_sha256` was verified against its `build-provenance.json`
(`e50748…ce88f0`).

## Two verdict layers, kept separate

The phrase "1 checked / 6 Unknown" belongs to the **source-contract** layer
(`crates/qcore-c-gateway/src/boolean_source.rs`). The **resource projection**
(`crates/qcore-c-gateway/src/legacy_c.rs`) assigns its own per-function status.
They disagree, deliberately, and neither promotes the other.

| Function | Source contract | Resource projection |
|---|---|---|
| `cmt_counter_create` | Unknown | **Unsafe** |
| `cmt_counter_allow_reset` | **checked_function_contract** | Unknown |
| `cmt_counter_destroy` | Unknown | Unknown |
| `cmt_counter_inc` | Unknown | Unknown |
| `cmt_counter_add` | Unknown | Unknown |
| `cmt_counter_set` | Unknown | Unknown |
| `cmt_counter_get_val` | Unknown | Unknown |

`cmt_counter_allow_reset` holds a checked source contract while its resource
projection is Unknown. That is the "no whole-unit promotion" boundary working
as specified.

Line numbers below are given as `preprocessed / original`, mapped by
reproducing the pinned Clang `-E -P` output for the exact compile-database
command.

## The six Unknown functions and their blockers

| Function | Recorded Unknown reason | Root blocker | Also needs |
|---|---|---|---|
| `cmt_counter_create` | outside single borrowed chunk / int-or-void-result source profile | **indirect member/deref** — `&counter->_head`, `&counter->opts`, `counter->map = …`, `counter->aggregation_type = 2`, `counter->cmt = cmt` (5471/59, 5472/61, 5478/69, 5485/77, 5486/79) | contracts for `cmt_log_print`, `cmt_map_create`; a W4 summary for `cmt_counter_destroy`; call-in-branch-condition (`strlen`) |
| `cmt_counter_destroy` | helper signature or parameter binding mismatch | **source-contract statement grammar** — the only admitted call form is a `void (struct R *)` frame helper on the modeled parameter; `cfl_list_del(&counter->_head)` matches neither the callee type nor the parameter binding | indirect member/deref on all four body statements; contracts for `cfl_list_del`, `cmt_opts_exit`, `cmt_map_destroy` |
| `cmt_counter_inc` | outside single borrowed chunk / int-or-void-result source profile | **source-contract ABI gate** — qualType `int (struct cmt_counter *, uint64_t, int, char **)` is not a registered model ABI | indirect member/deref (5508/106, 5512/110); contracts for `cmt_map_metric_get`, `cmt_metric_inc` |
| `cmt_counter_add` | outside single borrowed chunk / int-or-void-result source profile | **source-contract ABI gate** — five-parameter qualType | indirect member/deref (5522/124, 5526/128); contracts for `cmt_map_metric_get`, `cmt_metric_add` |
| `cmt_counter_set` | outside single borrowed chunk / int-or-void-result source profile | **source-contract ABI gate** — five-parameter qualType; also the only non-checked function that *reads* the modeled field | indirect member/deref (5536/143, 5540/147, 5544/154); branch condition mixing a call and `counter->allow_reset` (5543/153); contracts for three callees |
| `cmt_counter_get_val` | outside single borrowed chunk / int-or-void-result source profile | **source-contract ABI gate** — four-parameter qualType with an out pointer | **unsanctioned out store** `*out_val` (5562/178); indirect member/deref (5555/169, 5559/173); contract for `cmt_map_metric_get_val` |

Every exact blocker cites the emitting site
(`legacy_c.rs:1636` / `contains_unmodeled_access` at `legacy_c.rs:1798`,
`legacy_c.rs:1744`, `legacy_c.rs:2184`, `legacy_c.rs:3006`,
`boolean_source.rs:716`, `boolean_source.rs:1001`). Per-function minimal
reproducers, missing premises and candidate features are in `inventory.json`.

### The causal chain in this unit

The unit's **provisional Unsafe** verdict is not an independent finding — it is
downstream of one gate:

1. `contains_unmodeled_access` (`legacy_c.rs:1798`) rejects any statement whose
   *tokens* contain `->`, `[`, `]`, a non-numeric `.`, or a unary `*`. It is
   token-level and carries no declared type, so it cannot separate a scalar
   field store from a pointer store or a dereference.
2. All four statements of `cmt_counter_destroy` are therefore Unknown, so
   `legacy_summary.rs` publishes **no W4 custody summary** for it — its module
   header requires the callee body to re-analyze `Safe` with no diagnostics.
3. `cmt_counter_create`'s two `cmt_counter_destroy(counter)` calls (5475/64,
   5482/73) are consequently **uncontracted**, so the `calloc` obligation
   acquired at 5466/54 is never discharged.
4. Two `leak` diagnostics fire at the early returns 5476/65 and 5483/74, and
   the raw unit is **provisional Unsafe**.

Those leaks are **withdrawn-support results, not proven upstream leaks**. The
pinned C does release through `cmt_counter_destroy`; the analysis simply cannot
say so. Reproducer `R8` exhibits exactly this mechanism in five lines, and
control `R15` shows the same shape without any member access still produces the
leak once the releaser is uncontracted.

## Reproducers were executed, not asserted

Every minimal reproducer was run through the real gateway entry point
`qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit`, with the pinned
`contract/qcore/libc/memory.qcore` contracts, from a scratchpad-only probe
crate. No repository code or test was touched. Full output:
[`logs/reproducer_probe.log`](logs/reproducer_probe.log).

| Case | Reproducer | Observed |
|---|---|---|
| R1 | `struct s { int f; }; void w(struct s *p) { p->f = 1; }` | Unknown — indirect memory access |
| R3 | `void w(struct s *p) { ext(&p->h); }` | Unknown — indirect memory access |
| R5 | `if (!n \|\| len(n) == 0)` | Unknown — branch condition |
| R6 | `void w(double *out) { double v = 0; *out = v; }` | Unknown — store through `*out` without an out-parameter contract |
| R8 | `calloc` then uncontracted `rel(p)` | **Unsafe** — leak + uncontracted call |
| R9 | `cmt_counter_destroy` body shape | Unknown — unknown_alias + indirect + branch condition |
| R10 | `calloc` then `free` | **Safe** (control) |
| R12/R13 | empty borrowed-pointer function, `void` and `int` | **Safe** (controls) |
| R15 | create shape, no member access, uncontracted releaser | **Unsafe** — leak + uncontracted call (control) |

R1 combined with the R12/R13 controls is the load-bearing pair: a single scalar
field store is the *entire* difference between `cmt_counter_allow_reset`'s
Unknown resource status and a fully modeled body.

## Ranked blocker families

Recurrence measured over the archived 185-unit corpus
`docs/reports/w2-clang-cmake-build/diagnostics.json.gz` (hash-verified under
card B01, not regenerated): **185 units, 4,688 projected functions**, of which
89 carry no diagnostic at all. Ranking is by recurrence and cheaply estimable
precision gain — **not** by whether it would let any unit be marked Safe.

| # | Family | Blocks of the six | Diagnostics | Functions | Units | Earliest in fn | **Sole blocker** | Admissible fail-closed |
|---|---|---|---|---|---|---|---|---|
| 1 | `indirect_member_or_deref` | **6** | 18,671 | 3,233 | 171 | 1,590 | **298** | yes |
| 2 | `uncontracted_call` | 4 | 12,118 | 2,493 | 171 | 846 | 171 | **no** |
| 3 | `branch_condition` | 3 | 8,908 | 2,429 | 166 | 622 | 42 | yes |
| 4 | `unknown_alias` | 2 | 4,199 | 1,361 | 162 | 224 | **0** | yes |
| 5 | source-contract ABI gate | 5 | n/a | n/a | n/a | n/a | n/a | yes |
| 6 | `unsanctioned_out_store` | 1 | 676 | 262 | 87 | 14 | 1 | yes |
| 7 | source-contract statement grammar | 1 | n/a | n/a | n/a | n/a | n/a | yes |

*Sole blocker* counts functions whose **entire** blocker set is that one family
— the honest ceiling on what removing it alone can reach.

Family 2 is marked **not admissible**: treating an uncontracted call as
custody-neutral is precisely the optimistic fallback hard invariant 1 forbids.
Its legitimate routes are declared `.qcore` contracts or multi-unit scope,
neither of which is a bounded analysis capability. Family 4 has **zero**
sole-blocker functions — it is a consequence of state poisoning, and for a
formal it is also the intended trigger of W4's binding search.

Rows 5 and 7 are not cheaply estimable corpus-wide: the source-contract layer
runs only on the configured CMetrics and ChunkIO routes, not on all 185 units.

**Out-of-scope corpus observation.** The largest sole-blocker family in the
whole corpus is `return_expression` — 1,477 diagnostics, 1,148 functions,
124 units, **494 sole-blocker functions**. It blocks **none** of these six
(every return in this unit is already modeled), so it is not a C00 row and not
the C02 candidate. It is recorded here because it deserves its own card.

## C02 recommendation

**Selected family: `indirect_member_or_deref` (rank 1).**
**Capability: typed direct scalar-field access classification.**

Use the Clang typed AST the gateway already imports (`typed_c_ast`, consumed by
`boolean_source` for this same unit) to admit a bounded whitelisted class of
field accesses as resource-projection NoOps. Admit **only** when all hold:

- the statement contains no call expression anywhere;
- every member/subscript access is a direct field access on a `DeclRefExpr`
  base naming a formal or local — no chained deref, no computed base, no
  non-constant subscript;
- each accessed field's **declared** type is a non-pointer arithmetic or
  enumeration type;
- no accessed field is a union member and no record on the access path is a
  union;
- no accessed field is `volatile` or `_Atomic`;
- the statement takes no address of any sub-object;
- an assigned value is itself free of calls and unmodeled access.

Anything else stays **Unknown**, exactly as today. The rule is a conjunctive
whitelist over a typed premise; it never relaxes an existing rejection without
that premise.

**Soundness.** A write to a non-pointer arithmetic or enum field outside any
union cannot create, destroy, alias or transfer a tracked resource identity, so
it is custody-neutral. Unions are excluded because a scalar write can overwrite
an overlapping pointer member; `volatile`/`_Atomic` because they carry side
effects the projection does not model. Classifying the statement as a NoOp
asserts nothing about the base object's validity: if the access traps, that path
does not reach an exit, which can only *remove* leak witnesses, never add one.

**Why this family.** It is rank 1 on every axis measured, it is the only family
blocking all six functions, it is the root of this unit's causal chain, the
typed premise already exists in the gateway, and the rule is stated purely over
declared C types and expression shapes — no library, record or symbol name
appears in it. Rank 2 is inadmissible; rank 3 is better delivered as an
extension of this same typed classification (only 42 sole-blocker functions,
but 253 more pair it with ranks 1 and 2); rank 4 is a consequence; ranks 5–7 are
narrow or single-unit.

### What C02 explicitly would not achieve

- It makes **none** of the six a checked function contract. Five additionally
  need a source-contract ABI/vocabulary extension; four additionally need
  cross-translation-unit callee contracts.
- It does **not** promote the raw unit. `cmt_counter_create` retains its
  uncontracted `cmt_log_print`, `cmt_map_create` and `cmt_counter_destroy`
  calls and both leaks; the unit stays **provisional Unsafe**.
- Within this unit it changes exactly one function's resource projection:
  `cmt_counter_allow_reset`'s lone statement becomes modeled, leaving it
  diagnostic-free, so its **resource** status would move Unknown → Safe
  (controls R12/R13). That is a separate verdict from its already-checked
  source contract, and neither promotes the unit.
- Removing state poisoning can **surface** currently masked `leak` and
  `unknown_alias` diagnostics, so the corpus effect may include *new*
  provisional Unsafe results. That is a precision gain in both directions, not
  a monotone increase in Safe results — and a provisional Unsafe is never an
  upstream defect claim.
- All four archived 185-unit artifacts would change. Hard invariants 3 (parity
  before deletion) and 5 (pinned expectations are intentional) require them to
  be regenerated, diffed and re-pinned deliberately, with the diff archived
  under `docs/reports/`.

### Bounded fallback if the typed bridge is too large for one card

Take rank 6 alone: **non-custodial scalar out-slot classification**
(`legacy_c.rs:3006`). Admit a store through a formal whose declared type is
pointer-to-arithmetic as custody-neutral instead of poisoning all pointer
state. It needs the same typed premise on a *single formal* rather than on
arbitrary field paths, covers 676 diagnostics across 262 functions and 87
units, and unblocks exactly one of `cmt_counter_get_val`'s three blockers. It
promotes nothing.

## Scope and non-claims

- **No raw unit is promoted.** The CMetrics raw unit remains **provisional
  Unsafe** and unpromoted.
- A provisional Unsafe is **not** an upstream defect claim. The two leaks in
  `cmt_counter_create` are consequences of a missing callee summary, itself
  downstream of the indirect-access gate.
- The one checked function contract is reported separately and is unchanged.
- No gate was weakened, no pinned expectation edited, no diagnostic downgraded.
- Corpus recurrence figures come from the **hash-verified, not regenerated**
  185-unit artifact. Projection coverage is not safety coverage; unknowns stay
  in the denominator.
- No CT mapping changes; coverage remains 11/50 mapped.
- The candidate features named here are *candidates*. Nothing in this card
  establishes that any of them is implementable at its stated boundary.

## Files written

Only these paths. Nothing was committed, pushed, reset or cleaned, and no
tracked file was modified.

- `docs/reports/remaining-c00/README.md` (this file)
- `docs/reports/remaining-c00/inventory.json`
- `docs/reports/remaining-c00/logs/cmetrics_audit.log`
- `docs/reports/remaining-c00/logs/reproducer_probe.log`
- `docs/reports/remaining-c00/logs/corpus_stats.txt`
- `docs/reports/remaining-c00/logs/corpus_combo.txt`
- Session scratchpad (outside the repository): audit config, preprocessed unit,
  probe crate.
