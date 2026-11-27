# Unsupported coverage: typed access paths, callee closure, and alias residuals

> [!NOTE]
> **Current status (active C-gateway coverage plan; `P0`–`P3` and initial `P4`–`P6` waves landed).** Since the 2026-09-27 baseline below (398 Safe corpus functions), typed access paths, expression elaboration, `AG1`–`AG3`, and `CG00`–`CG30` slices have raised the corpus to **1,079 Safe** on the committed baseline (`556460e31`) and **1,119 Safe** on the latest measured SP1 landing (`2c07390b6`; see [`docs/STATUS.md`](../../STATUS.md)). Legacy shape-route retirement parity (`W46-ACCESS-PATH-PARITY`), bottom-up callee body residuals (`W48-BOTTOM-UP-RESIDUAL-PLAN`), general owned-field pointer stores (`P5`), and remaining alias cards (`P6`) remain open in `acceptance_matrix.json#named_followups`.

## 1. Execution standing across `P0`–`P6`

| Track | Subject | Landed implementations & measurements | Remaining open work |
|---|---|---|---|
| **P0** — `W45-SR5-SOUNDNESS` | Close `STABILITY-REEVAL-5` admitting findings before access/member widening | **Completed** (`85a35431`, [report](../../reports/archive/waves-40-47/w45-sr5-soundness/README.md)) | Closed |
| **P1** — `ACCESS-PATH-CENSUS` | Classify `IND`, `COND`, and `RET` leaves from Clang's typed AST and freeze predictions | **Completed** — [W46-ACCESS-PATH-CENSUS](../../reports/archive/waves-40-47/w46-access-path-census/README.md) | Closed |
| **P2** — `ACCESS-PATH-STATEMENTS` | Typed AST access-path fallback for statements alongside shape routes | **Bounded fallback landed** — [W46-CONTRACT-ACCESS-EXPANSION](../../reports/w46-contract-access-expansion/README.md), expanded by `CG10` rounds 1–4 and `AG1` | `W46-ACCESS-PATH-PARITY` (archived parity required before retiring legacy shape routes); mixed paths over tracked roots (`CG10-*`) |
| **P3** — `ACCESS-PATH-POSITIONS` | Typed access paths in controlling expressions, returns, and call actuals | **Landed** — [W47-ACCESS-PATH-POSITIONS](../../reports/archive/waves-40-47/w47-access-path-positions/README.md), [root reader](../../reports/archive/waves-40-47/w47-typed-root-declaration-reader/README.md), [nested assignment closure](../../reports/archive/waves-40-47/w47-lexical-closure-nested-assignment/README.md), [const pointer value](../../reports/archive/waves-40-47/w47-typed-const-pointer-value/README.md), [position residuals](../../reports/archive/waves-40-47/w47-access-path-position-residuals/README.md), `AG3` | `W46-ACCESS-PATH-PARITY`, `W47-SUMMARY-VALIDATION-PROTECTED-FORMALS`, `?:` actuals/pointer returns, nested-call actuals |
| **P4** — `CALLEE-CLOSURE` | Bottom-up in-corpus summary cascade + high-leverage external provider contracts | **Rounds 2–4 executed** — [R2](../../reports/archive/waves-40-47/w47-callee-closure-r2/README.md), [R3](../../reports/archive/wave-48/w48-callee-closure-r3/README.md), [R4 bottom-up bodies](../../reports/archive/wave-48/w48-bottom-up-callee-bodies/README.md) | `W48-BOTTOM-UP-RESIDUAL-PLAN`, `W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT`, `W48-CALLEE-DECISION-TABLE` |
| **P5** — `FIELD-CUSTODY` | Pointer-valued stores moving custody through struct fields (`p->next = q`, `p->buf = alloc()`) | **Initial slices landed** — [CG11-POINTER-VALUED-STORES](../../reports/archive/gateway-and-clang/cg11-pointer-valued-stores/README.md) (rule 10v), [CG10-MEMBER-STORE-RETAINED-BY](../../reports/archive/gateway-and-clang/cg10-member-store-retained-by/README.md), [CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT](../../reports/archive/gateway-and-clang/cg10-owned-out-parameter-member-slot/README.md), `K6` (rule 81) | `CG11-POINTER-STORE-SUMMARY-MODE`, `CG11-UNRESOLVED-OWNER-POINTER-STORES`, `CG11-UNREAD-POINTER-PLACES`, `CG10-OWNED-OUT-MEMBER-TRACKED-OWNER` |
| **P6** — `ALIAS-RESIDUALS` | Classify and discharge `unknown_alias` leaves in coordination with `IND` and `UNC` | **Census + bounded slices landed** — [W47-ALIAS-RESIDUALS-CENSUS](../../reports/archive/waves-40-47/w47-alias-residuals-census/README.md) (7,440 corpus leaves classified), [AG2-UNRESOLVED-ALIAS](../../reports/archive/gateway-and-clang/ag2-unresolved-alias/README.md), [K4](../../reports/archive/wave-48/k4-formal-custody-aliases/README.md), [K5](../../reports/archive/wave-48/k5-local-origin-aliases/README.md) | `W47-ALIAS-TYPED-SCALAR-NAMES`, `W47-ALIAS-FORMAL-COPY`, `W47-ALIAS-CALLEE-BORROWED-CLOSURE`, `W47-ALIAS-NONNULL-ALLOCATOR-RESULTS`, `W47-ALIAS-FLOW-RESIDUALS` |

---

## 2. Core architectural findings guiding remaining C-gateway work

1. **Token-shape rules hit a typing ceiling; Clang's AST resolves it.** Earlier lexical/token rules (`5h`–`5l`) could not resolve typedef roots, nested aggregates, or indirect single-pointer reads without full declaration types. Lowering typed access paths from `typed_c_ast.rs` while enforcing per-hop non-null and liveness obligations generalizes statements (`P2`) and expression positions (`P3`) uniformly—while refusing unions, bit-fields, pointer arithmetic, volatile/atomic hops, or incomplete records unless a dedicated sound rule (`CG13`–`CG19`) applies.
2. **Uncontracted calls (`UNC`) are primarily a bottom-up in-corpus body problem, not an external contract backlog.** Roughly 75% of uncontracted call sites target corpus-defined functions that are themselves `Unknown`; `W48-BOTTOM-UP-CALLEE-BODIES` showed that making in-corpus callee bodies `Safe` unblocks ~390–416 callers in a single summary-cascade round without adding external assumptions, whereas no remaining bodiless external callee in the existing vocabulary unblocks even two functions alone.
3. **Pointer-valued field stores (`P5`) are custody transitions, not neutral reads.** Storing a pointer into a member (`p->f = q`) transfers, retains, or overwrites custody and must be governed by owned-field, retained-by, and container rules with audited Lean models (`STABILITY-REEVAL-5`), never folded into custody-neutral scalar access rules.
4. **Alias leaves (`P6`) rarely block functions alone.** In `W47-ALIAS-RESIDUALS-CENSUS`, 1,922 `Unknown` functions carried `unknown_alias` leaves, but only **8** were blocked by alias alone (vs. 232 co-blocked with `IND` and 76 with `UNC`); alias cards yield verdict moves when paired with `P3`/`CG10` and `P4`.

---

## 3. Standing card acceptance rules

Every card cut from this queue requires:
- **Committed prediction (`PREDICTION.md`)** before measurement, verified via `quirin-audit measure --declared`;
- **Fail-closed parity (Hard Invariants 1 & 3)**: no refactor turns `Unknown` into `Safe` without discharged obligations, and no legacy shape route is deleted before archived byte-for-byte parity;
- **Audited Lean parity (Hard Invariant 6)**: registered in `proof/Audit*.lean` via `gen_lean_roots.py` with zero placeholders or project axioms;
- **Provisional `Unsafe` discipline**: newly surfaced leak or use-after-release candidates are classified by source reading and never presented as upstream defects.

---

## 4. Historical 2026-09-27 baseline (`w45-container-root-reach`, `b1744a2a`)

Recorded for provenance of the original `P1`–`P5` sizing: 398 Safe / 9 provisional Unsafe / 4,427 Unknown functions; 41,314 unsupported leaves across 31 families (`IND` 17,042 [41.2%, 418 sole], `UNC` 9,179 [22.2%, 436 sole], `COND` 8,578 [20.8%, 83 sole], `RET` 304 [0.7%], `OTHER` 6,211 [15.0%]; `IND+COND+RET+UNC` combined upper bound 2,045 functions). Current figures live in [`docs/STATUS.md`](../../STATUS.md).
