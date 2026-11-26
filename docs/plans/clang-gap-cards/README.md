# Clang gap analysis: card queue and post-card residuals (`CG00`–`CG30`, `SUP01`–`SUP08`)

> [!NOTE]
> **Current status (all 31 initial cards `CG00`–`CG30` plus `SUP03` and `SUP07` have landed bounded implementations; post-card residuals remain open).** Across wave 48 (rounds 1–21+ and subsequent `CG10`/`CG22`/`CG25` follow-ons), every card in Phases 0–4 shipped its bounded gateway/core/contract slice under `docs/reports/cg*/`. Remaining open work consists of the ~95 post-card `CG*` residual entries and open supplemental items (`SUP01`, `SUP02`, `SUP04`–`SUP06`, `MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE`) registered in [`acceptance_matrix.json#named_followups`](../../reviews/r0_3/acceptance_matrix.json). See [`docs/STATUS.md`](../../STATUS.md) for current live-profile baselines.

## 1. Inputs and their standing

The supplied static gap-analysis inputs (2026-09-29) are archived byte-identically ([record](../../reviews/clang-gap-2026-09-29/README.md)) under [`reviews/clang-gap-2026-09-29/supplied/`](../../reviews/clang-gap-2026-09-29/supplied/):

| File | SHA-256 |
|---|---|
| `Quirin_Clang_Gap_Analysis_EN.md` (112-entry inventory §5; implementation order §7) | `92a1177d…a7e2e3` |
| `Quirin_Clang_Response_EN.md` (conversational summary) | `1d22b316…e8710` |
| `Quirin_Source_Evidence_EN.md` (excerpts Q01–Q47) | `80616b1b…a711a440` |

`gap_inventory_EN.json`, `metadata.json`, `raw_boundary_mentions.json`, the probes, and `translation_manifest.json` were named by the supplied text but not provided.

- **Declared input, not implementation authority**: Because the static review ran no Rust tests, every `CG` card opened by reproducing its cited refusal in an executed BEFORE test (`crates/qcore-c-gateway/tests/support/clang_gap.rs`, `clang_gap_phase1.rs`); claims that did not reproduce were recorded as input corrections in the card's report.
- **Architectural direction**: Rather than inventing new contract semantics, Phases 1–4 generalized *where* existing contracts and core checkers apply across C's expression positions (`Phase 1`), typed places and stores (`Phase 2`), core footprint/extent/atomic checkers (`Phase 3`), and guarantee-gated numeric/callback/variadic/weak-memory extensions (`Phase 4`). Route `G` denotes the custody gateway and Route `T` denotes `typed_source.rs`.

---

## 2. Phase tables and card records (`CG00`–`CG30`)

### Phase 0: baseline

| Card | Entries | Effort | Standing & report |
|---|---|---|---|
| `CG00-CLANG-GAP-BASELINE` | all 98 carded entries | Medium | **Landed** — [cg00-clang-gap-baseline](../../reports/cg00-clang-gap-baseline/README.md); residual `CG00-G-CORRECTION-RESIDUALS` |

- **CG00**: Converted all 98 carded entries into executed BEFORE tests across the §8 axes, attributed Unknown corpus functions to blocking diagnostic families, and froze `predictions/` rankings for Phases 1–3.

---

### Phase 1: expression elaboration and contract application (`F`)

| Card | Entries | Route | Standing & primary report(s) | Open residuals in `named_followups` |
|---|---|---|---|---|
| `CG01-T-OPERATOR-CLOSURE` | E01 E02 E03(T) E14 | T | **Landed** — [cg01](../../reports/cg01-t-operator-closure/README.md) | `CG01-T-CONSUMER-ADMISSIONS` |
| `CG02-ELABORATION-CORE` | E03(val) E04 E05 E08 E12(order) | G, T | **Landed (rule 10s)** — [cg02](../../reports/cg02-elaboration-core/README.md) | `CG02-COMMA-OPERAND-RESIDUALS`, `CG02-FOR-CLAUSE-SEQUENCES` |
| `CG03-CALL-SUMMARY-INSTANTIATION` | K12 K03 | G, T | **Landed (rule 10u)** — [cg03](../../reports/cg03-call-summary-instantiation/README.md) | `CG03-RECURSIVE-COMPONENT-ADMISSIONS`, `CG03-RETURN-OPERAND-RESIDUALS`, `CG03-TYPED-SUBSTITUTION-RESIDUALS` |
| `CG04-CONDITION-ACQUISITION` | E09 E13 | G | **Landed (rule 10x)** — [cg04](../../reports/cg04-condition-acquisition/README.md), [loop sites](../../reports/cg04-loop-site-generations/README.md) | `CG04-UNCONSULTED-CONDITION-LEAVES` |
| `CG05-NESTED-CALL-EFFECTS` | E10 E11 E12 | G | **Landed (rules 10y, 11n)** — [cg05](../../reports/cg05-nested-call-effects/README.md), [footprints](../../reports/cg05-elaborator-call-footprints/README.md) | `CG05-UNCONTRACTED-INNER-CALLEES`, `CG05-CALL-ROOTED-PLACES`, `CG05-ELABORATOR-FOOTPRINTLESS-CALLS` |
| `CG06-VALUE-CONNECTIVES-AND-CONDITIONAL` | E06 E07 | G, T | **Landed (rules 10z, 11m)** — [cg06](../../reports/cg06-value-connectives-and-conditional/README.md), [footprints](../../reports/cg06-connective-footprints/README.md) | `CG06-VALUE-CONNECTIVE-RESIDUALS` |
| `CG07-GENERIC-AND-STATEMENT-EXPRESSIONS` | E17 E19 | G, T | **Landed (rule 11g)** — [cg07](../../reports/cg07-generic-and-statement-expressions/README.md) | `CG07-STATEMENT-EXPRESSION-POSITIONS`, `CG07-GENERIC-TOKEN-AST-MATCH` |
| `CG08-CASTS-AND-TYPE-QUERIES` | E15 E16 N11 M18 N08(repr) | G, T | **Landed (rule 11b)** — [cg08](../../reports/cg08-casts-and-type-queries/README.md) | `CG08-CAST-AND-QUERY-RESIDUALS` |
| `CG09-TYPED-SOURCE-CONTROL` | C01 C02 C03 | T | **Landed (rule 11l)** — [cg09](../../reports/cg09-typed-source-control/README.md) | `CG09-CONTROL-SHAPE-RESIDUALS` |

- **CG01**: Added binary `-`, `<=`, `>=`, `^`, `<<`, `>>`, unary `+`, `~`, nine compound assignments, and float/char literals to `Importer::expression`; Monkey wrapper admits them while other `T` consumers refuse by name.
- **CG02**: Added `elaborate(expr, ctx)` with explicit sequenced-before order, value assignments, pre/post `++`/`--`, comma sequencing, and single-LHS compound RMW on route G beside retained shape routes.
- **CG03**: Unified contract instantiation at `Parser::instantiate_call` across statement, condition, return, and nested call positions.
- **CG04**: Lowered condition acquisitions (`if ((p = malloc(n)) != NULL)`) via `ResultUse::TestedSplit` without hoisting, plus loop-site generation guards.
- **CG05**: Applied inner non-identity call effects inside-out at arbitrary depth with footprint commutativity checks for unordered sibling calls.
- **CG06**: Lowered value-producing `&&`, `||`, and `?:` through branch regions evaluating only the taken arm on each edge.
- **CG07**: Connected Clang's selected `GenericSelectionExpr` association and lowered GNU statement expressions under hygiene/escape premises.
- **CG08**: Modeled representation casts with provenance/loss flags, implicit `T` enumerators, and Clang-evaluated `sizeof`/`_Alignof` constants.
- **CG09**: Added acyclic `T` control lowering into guarded paths with explicit back-edge refusals.

---

### Phase 2: typed places and stores (`F`/`R`)

| Card | Entries | Route | Standing & primary report(s) | Open residuals in `named_followups` |
|---|---|---|---|---|
| `CG10-COMMON-TYPED-PLACE` | M01 I04 K03(place) | G | **Bounded slices landed (rules 10t, 12i–12k, 14a–14d)** — [cg10](../../reports/cg10-common-typed-place/README.md), [sole blockers](../../reports/cg10-indirect-access-sole-blockers/README.md), [residuals r1–r4](../../reports/cg10-indirect-access-sole-residuals-r4/README.md), [member-store](../../reports/cg10-member-store-retained-by/README.md), [owned-out](../../reports/cg10-owned-out-parameter-member-slot/README.md) | `CG10-COMMON-TYPED-PLACE` (amended), `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS`, `CG10-TYPED-PLACE-RESIDUALS`, `CG10-BOUND-ROOT-*`, `CG10-OWNED-OUT-*`, `CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED`, `CG10-OOM-DECLARATIONS-UNVALIDATED` |
| `CG11-POINTER-VALUED-STORES` | M02 M03 M04 | G | **Bounded slice landed (rule 10v)** — [cg11](../../reports/cg11-pointer-valued-stores/README.md) | `CG11-POINTER-VALUED-STORES` (amended), `CG11-UNRESOLVED-OWNER-POINTER-STORES`, `CG11-POINTER-STORE-SUMMARY-MODE`, `CG11-FUNCTION-POINTER-PARAMETER-CLOSURE`, `CG11-UNREAD-POINTER-PLACES` |
| `CG12-VOID-VIEW-READBACK` | M05 | G | **Bounded slice landed (rule 11j)** — [cg12](../../reports/cg12-void-view-readback/README.md) | `CG12-VOID-VIEW-READBACK` (amended), `CG12-READBACK-NO-STORE-BASE`, `CG12-VIEW-RESIDUAL-FORMS` |
| `CG13-UNION-AND-BITFIELD-PLACES` | M06 M07 | G, T, core | **Landed (rule 10w)** — [cg13](../../reports/cg13-union-and-bitfield-places/README.md) | `CG13-UNION-LEAVES-OTHER-CAUSES`, `CG13-UNION-PUNNING-FLOW-INSENSITIVE` |
| `CG14-AGGREGATE-VALUES-AND-INITIALIZERS` | M08–M11 | G, T | **Bounded slice landed on G (rule 11f)** — [cg14](../../reports/cg14-aggregate-values-and-initializers/README.md) | `CG14-AGGREGATE-VALUES-AND-INITIALIZERS` (amended), `CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES`, `CG14-AGGREGATE-VALUE-POSITIONS`, `CG14-DECLARATION-FORMS` |
| `CG15-STORAGE-DURATION-AND-VOLATILE` | M24 M23 | G, T | **Landed (rule 11h)** — [cg15](../../reports/cg15-storage-duration-and-volatile/README.md) | `CG15-STATIC-TABLE-RESIDUALS`, `CG15-STATIC-REGISTRY-CONTRACT` |

- **CG10**: Introduced AST-backed `DeclID`/`FieldID`/subscript/dereference places (`typed_place.rs`) over tracked roots, followed by sole-blocker rounds 1–4, retained-by member stores, owned-out member slots, bound untracked roots (`cg10-bound-untracked-roots`), embedded-list borrowed containers (`cg10-embedded-list-borrowed-container`), and reader-gap repairs (`cg10-reader-gaps`).
- **CG11**: Admitted identity-free pointer stores into untracked owners (`rule 10v`); general tracked-owner pointer stores and `Stored` summary derivation remain open.
- **CG12**: Admitted object-pointer bit-cast views over untracked `void *` and typed readback of linked `void *` store bases while refusing type punning.
- **CG13**: Connected C11 bit-field storage units to `disjoint_footprint` and enforced union storage/punning boundaries.
- **CG14**: Admitted custody-free struct copies, identity-free braced initializers, borrowed compound literals, and string-initializer extent recording on route G.
- **CG15**: Modeled static-duration objects as identity-free and `volatile` accesses as ordered, value-erasing trace events refusing custody decisions.

---

### Phase 3: connections to existing core checkers (`R`)

| Card | Entries | Standing & primary report(s) | Open residuals in `named_followups` |
|---|---|---|---|
| `CG16-INDEX-AND-OFFSET-EXTENTS` | M14–M17 | **Bounded slice landed (rule 11s, `c.bounds.v1`)** — [cg16](../../reports/cg16-index-and-offset-extents/README.md), [transport](../../reports/cg16-bounds-transport-and-chunkio-neighbours/README.md), [design brief](cg16-reviewed-index-contracts.md) | `CG16-INDEX-AND-OFFSET-EXTENTS` (amended), `CG16-BOUNDS-CONTRACT-TRANSPORT`, `CG16-BOUNDS-POINTER-EXTENTS`, `CG16-BOUNDS-SYMBOLIC-RANGES`, `CG16-BOUNDS-UNSUPPORTED-FORMS` |
| `CG17-LAYOUT-DERIVED-EXTENTS` | M12 M13 M19 E18 | **Landed (rule 11u)** — [cg17](../../reports/cg17-layout-derived-extents/README.md) | `CG17-FLEXIBLE-EXTENT-UNESTABLISHED`, `CG17-ALIGNMENT-UNDECIDED`, `CG17-CONTAINER-OF-LIST-LOOKUPS` |
| `CG18-FOOTPRINT-CONNECTION` | M20–M22 M26 W08 | **Landed (rule 11k)** — [cg18](../../reports/cg18-footprint-connection/README.md) | `CG18-CROSS-OBJECT-PATH-PAIRS`, `CG18-FOOTPRINT-CONSUMERS` |
| `CG19-ATOMIC-SOURCE-ROUTE` | W01 W09 | **Landed (rule 11i)** — [cg19](../../reports/cg19-atomic-source-route/README.md) | `CG19-ATOMIC-FORMAL-VALUE-STORE-OUT-PARAMETER`, `CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T`, `CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD` |

- **CG16**: Implemented `QCore.IndexExtent`, `bounds_contract_v1` (`C09.1`, `G05b`), and `QCORE_BOUNDS_PROPERTY` (162 Safe / 4,759 Unknown bounds rows; combined custody+bounds 113 Safe at `SUP03`).
- **CG17**: Modeled non-escaping scalar VLAs, Clang-derived flexible-array extents, `layout.alignment.v1` checks, and established `container_of(&x->m)` provenance.
- **CG18**: Connected Clang struct member layouts to `disjoint_footprint` for unsequenced same-object member accesses and stated `P-DISTINCT-ALLOCATIONS-DISJOINT` / `P-EFFECTIVE-TYPE` / `P-BYTE-REPRESENTATION` premises.
- **CG19**: Routed 51 `__c11_atomic_*` / `__atomic_*` / `__sync_*` builtins and `_Atomic` qualified loads/stores/RMWs (`atomic_source_route.rs`, `QCore.AtomicSourceRoute`) to the core atomic model and source publication route.

---

### Phase 4: vocabulary, numeric, and weak-memory extensions (`V`)

| Card | Entries | Standing & primary report(s) | Open residuals in `named_followups` |
|---|---|---|---|
| `CG20-NUMERIC-DOMAIN` | N01–N08 C04 C05 | **Landed (rule 11q, C integer arithmetic property)** — [cg20](../../reports/cg20-numeric-domain/README.md) | `CG20-NUMERIC-CORPUS-CENSUS`, `CG20-RESULT-RANGE-TRANSFER-LEAN`, `CG20-SWITCH-AND-CORRELATION-PRECISION` |
| `CG21-CALL-EXITS-AND-BOUNDARIES` | C09–C13 E20 I06 | **Landed (rule 11o, `resource_no_return()`)** — [cg21](../../reports/cg21-call-exits-and-boundaries/README.md) | `CG21-NORETURN-TYPED-ACTUAL-STATEMENTS`, `CG21-NORETURN-OPERAND-POSITIONS`, `CG21-CLEANUP-SCOPE-EXITS`, `CG21-OPAQUE-BOUNDARIES` |
| `CG22-INDIRECT-CALLS-AND-CALLBACKS` | K01 K02 | **Landed (rules 11p, 13u–13z, `C04.19`)** — [cg22](../../reports/cg22-indirect-calls-and-callbacks/README.md), [registry](../../reports/cg22-callback-registry-contract/README.md), [candidates](../../reports/cg22-callback-candidate-contracts/README.md), [link premise](../../reports/cg22-callback-candidate-link-premise/README.md), [served queues](../../reports/cg22-op-callbacks-served-queues/README.md) | `CG22-OPEN-INDIRECT-CALLS`, `CG22-CALLBACK-REGISTRY-CONTRACT`, `CG22-CALLBACK-CANDIDATE-CONTRACTS`, `CG22-CALLBACK-OPEN-ACTUALS`, `CG22-COMPARATOR-CALLER-RESIDUALS`, `CG22-DESIGNATOR-ACTUAL-ELABORATION`, `CG22-INLINE-FNPTR-LOCAL-DECLARATION`, `CG22-MSGPACK-PACK-INVOCATIONS`, `CG22-REGISTRY-OUTSIDE-THE-FRAME` |
| `CG23-RELATIONAL-POSTCONDITIONS` | K05 K11 | **Landed (rule 11r, `C04.13`)** — [cg23](../../reports/cg23-relational-postconditions/README.md) | `CG23-EXIT-SPECIFIC-BLOCKED-HELPERS`, `CG23-DISPOSITION-TEST-POSITIONS` |
| `CG24-ALIAS-EXTENT-FRAME-VOCABULARY` | K04 K06 | **Landed (rule 11e, `C09.1`)** — [cg24](../../reports/cg24-alias-extent-frame-vocabulary/README.md) | `CG24-CALLBACK-SPANNING-RELATIONS`, `CG24-FRAME-AND-SLOT-PRECISION` |
| `CG25-RESOURCE-KINDS` | K07 | **Landed (rule 11v)** — [cg25](../../reports/cg25-resource-kinds/README.md), [in-place regex](../../reports/cg25-in-place-regex/README.md), [thread-key](../../reports/cg25-thread-key-automatic-revisit/README.md), [void-cast](../../reports/cg25-void-cast-guard-escape/README.md) | `CG25-IN-PLACE-RESIDUAL-PAIRS`, `CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE`, `CG25-KINDED-WRAPPER-RESULTS`, `CG25-DLSYM-SYMBOL-LIFETIME`, `CG25-THREAD-KEY-SETTER-PLACE` |
| `CG26-VARIADICS` | K08–K10 | **Landed (rules 11w, 12g)** — [cg26](../../reports/cg26-variadics/README.md), [tail actuals](../../reports/cg26-tail-residual-actuals/README.md) | `CG26-VA-PROTOCOL-PRECISION`, `CG26-POINTER-VA-ARG`, `CG26-TAIL-IDENTITY-CONDITIONALS`, `CG26-TAIL-UNMATCHED-STATEMENTS`, `CG26-ROUTE-T-VARIADICS` |
| `CG27-LOOPS-RECURSION-BUDGETS` | C06–C08 | **Bounded slice landed (rule 11x)** — [cg27](../../reports/cg27-loops-recursion-budgets/README.md) | `CG27-LOOPS-RECURSION-BUDGETS` (amended), `CG27-LOOP-RECURSION-RESIDUALS` |
| `CG28-WEAK-MEMORY-EVENTS` | W02–W06 | **Landed (rule 11t, `QCore.WeakMemory`)** — [cg28](../../reports/cg28-weak-memory-events/README.md) | `CG28-LEGACY-SYNC-BUILTINS`, `CG28-UNREAD-ATOMIC-OPERAND-SHAPES` |
| `CG29-EXECUTION-COVERAGE-AND-RECLAMATION` | W07 W10 | **Landed (rule 11z)** — [cg29](../../reports/cg29-execution-coverage-and-reclamation/README.md) | `CG29-ARITHMETIC-RMW-VALUE-DOMAIN`, `CG29-SOURCE-FRAGMENT-EXTRACTION`, `CG29-RECLAMATION-SOURCE-ROUTE` |
| `CG30-DIALECTS-FLAGS-TYPES` | I01–I03 I05 N12–N14 | **Bounded slice landed (rule 11y)** — [cg30](../../reports/cg30-dialects-flags-types/README.md), [type fixture](../../reports/cg30-type-identity-fixture/README.md) | `CG30-DIALECTS-FLAGS-TYPES` (amended) |

---

## 3. Entries that are intentionally non-cards (`14` of `112`)

Reopened only if explicitly selected as a new published guarantee:
- **Out of scope**: `X01`–`X07` (LLVM backend/SSA/optimization/debug info, re-implementing Clang Sema, `"emit-llvm proves no UB"`, analyzing every external library body).
- **Delegated or already checked**: `K13` (physical ABI lowering), `K14` (cross-TU summary rechecking), `I07` (per-build vs. all-configuration claim boundary).
- **Guarantee-gated**: `N09`–`N10` (C IEEE/`fenv` guarantees), `M25` (uninitialized/padding byte reads), `W11` (eventual liveness/fairness).

---

## 4. Standing acceptance rules for residual cards

Every follow-on residual card cut from this queue must provide:
1. Frozen `PREDICTION.md` verified via `quirin-audit measure --declared`;
2. Executed BEFORE/AFTER reproduction across the §8 axes with every moving row and correction attributed;
3. Explicit route (`G`, `T`, or core consumer) and profile scope—never introducing evaluation orders C does not guarantee or auto-applying call contracts to non-call AST nodes;
4. Fail-closed `Unknown` preservation, audited `QCore.*` Lean parity, and provisional-only reporting of `Unsafe` candidates.

---

## 5. Dispatch record summary

- **Pre-dispatch ranking (`CG00`, corpus `ae17152b`)**: Upper-bound Unknown functions freed alone (with / without alias leaves): `CG10` 491/293, `CG13` 55/48, `CG06` 40/39, `CG11` 37/34, `CG08` 34/22, `CG04` 31/25, `CG05` 23/19, `CG07` 18/15, `CG03` 17/13, `CG14` 17/11, `CG02` 12/10, `CG15` 7/7, `CG19` 2/2, `CG16` 2/1, `CG12` 1/1, `CG17` 0 (`CG10 + CG11` pair: 80).
- **Executed wave-48 rounds**: All 31 base cards (`CG00`–`CG30`) landed across wave 48 rounds 1–11 (initial sweep reaching 647 Safe at `cf7cce20a`), followed by `CG10` sole-blocker/residual rounds 1–4, `CG22` callback registry & twelve-home comparator slices, `CG25` regex/thread-key/void-cast slices, and `AG1`–`AG3` (raising the corpus above 1,119 Safe; see [`docs/STATUS.md`](../../STATUS.md)).

---

## 6. Supplemental cards (`SUP01`–`SUP08`, 2026-10-02)

Four obsolete unmerged branches from earlier waves were archived under local `archive/*` tags on 2026-10-02, and the remaining open questions were registered as `SUP01`–`SUP08`:

| Card | Question / Scope | Current standing |
|---|---|---|
| `SUP01-C03-CENSUS-INERTNESS-PROOF` | Prove `legacy_census.rs` reach-census inertness (`QCore.NeutralFrameCensus`) | **Open** |
| `SUP02-MSVC-LLP64-WINDOWS-RUN` | Execute `CG20` plain-`long` and `CG30` LLP64 controls against native Windows `x86_64-pc-windows-msvc` Clang | **Open** (related LLP64 fixture portability landed in `windows-ci-llp64-fixtures-2`) |
| `SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN` | Re-measure `QCORE_BOUNDS_PROPERTY` on integrated `main` | **Completed** — [sup03-bounds-tally](../../reports/sup03-bounds-tally/README.md) (162 Safe / 4,759 Unknown; combined custody+bounds 111 → 113 Safe) |
| `SUP04-VARIADIC-UPSTREAM-CANDIDATES` | Validate `CG26` `mk_vhost_set` (`!value` missing `va_end`) and `rd_kafka_cgrp_rejoin` (`ap` before `va_start`) candidates | **Open** (provisional only; never reported upstream without validation) |
| `SUP05-PREDICTION-BEFORE-CODE` | Mechanically verify that a card's `PREDICTION.md` commit precedes its first implementation commit | **Open** |
| `SUP06-CONFIRMED-VERDICT-STATUS` | Proposed fifth status `Confirmed` (fixed on upstream release-candidate tip, defective at audited `HEAD`) | **Open — gated on explicit user decision** (touches Hard Invariant 2 & `§L21.2`) |
| `SUP07-REFUSED-TEST-ACCUSATION` | Prevent leak accusations resting on decisive facts hidden behind refused null tests (`mk_http_error_page`) | **Completed (rule 12e)** — [sup07-refused-test-accusation](../../reports/sup07-refused-test-accusation/README.md) |
| `SUP08` (`MONKEY-DOCUMENTROOT-OVERWRITE-CANDIDATE`) | User validation of Monkey `DocumentRoot` and `mk_config_listen_parse` overwrite candidates | **Open** (awaits user validation; nothing reported upstream without user decision) |
