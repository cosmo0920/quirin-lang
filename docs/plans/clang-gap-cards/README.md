# Clang gap analysis: open card queue

Review date: 2026-09-29. Status: a candidate card queue, not dispatched work and
not a coverage claim. Every card is registered in the acceptance matrix's
`named_followups` as a candidate implementation card under its full ID
(`CG00-CLANG-GAP-BASELINE` … `CG30-DIALECTS-FLAGS-TYPES`, recorded by
`CLANG-GAP-CARD-QUEUE`); the IDs below are those IDs. Priority is relative within this queue. It does not
override soundness repairs, G1/G4 ordering, the running `.qr` language cards or
the integrator's wave selection.

## 1. Inputs and their standing

The supplied inputs are archived byte-identically ([record](../../reviews/clang-gap-2026-09-29/README.md)) under
[`reviews/clang-gap-2026-09-29/supplied/`](../../reviews/clang-gap-2026-09-29/supplied/):

| File | SHA-256 |
|---|---|
| `Quirin_Clang_Gap_Analysis_EN.md` (the 112-entry inventory, §5; the implementation order, §7) | `92a1177d…a7e2e3` |
| `Quirin_Clang_Response_EN.md` (conversational summary of the same analysis) | `1d22b316…e8710` |
| `Quirin_Source_Evidence_EN.md` (excerpts Q01–Q47) | `80616b1b…a711a440` |

`gap_inventory_EN.json`, `metadata.json`, `raw_boundary_mentions.json`, the
probes and `translation_manifest.json` are named by the analysis but were not
supplied.

**Declared input, not implementation authority.** The analysis is static: it
ran no Rust test (its §0). Every card below therefore opens by reproducing the
cited refusal as an executed BEFORE test. A claim that does not reproduce is a
correction to the input, recorded in the card's report, not work to do.

**Freshness, checked on base `535c327f`.** 44 of the 47 evidence excerpts are
byte-identical to the current files, by the per-file SHA-256 the evidence
records. The three that differ do not change a gap entry:

- Q31 `contract_loader.rs` gained lifecycle clauses (W47-FLB-LIB-LIFECYCLE,
  W47-FLB-LIB-STRICT-PAIRING). The change is additive.
- Q32 `scalar_calls.rs` gained the fixed-width table (W48-QR-SCALAR-TYPES). Its
  header still says it has no wraparound or C undefined-behaviour
  interpretation, so N04 stands for C.
- Q41 `loop_owner_property_keys.rs` gained the service-key reading. It is still
  a specialised `va_arg` shape, so K09 stands.

**Direction.** This queue follows the analysis's §7 order. The central gap is
applying and composing *existing* contracts across C's expression positions,
places and control forms. Missing contract semantics are not the central gap.

1. Common expression elaboration and one contract-application point.
2. Common typed places and tracked stores.
3. Connecting source facts to the existing core checkers (footprint, extent,
   weak memory).
4. Numeric, vocabulary and weak-memory extensions, chosen by the guarantee
   actually wanted.

Class codes (F/R/V/P/O) are the analysis's §4 codes. G is the custody gateway
route and T is `typed_source.rs`. They accept different subsets, so a card
names which route it changes.

## 2. Priority queue

"Absorbs" lists registered `named_followups` whose refusal the card is
expected to lift or decide. The card closes or re-registers each one, with
sizes re-measured. Effort follows the tiers in the
[unsupported coverage queue](../unsupported-coverage/README.md#3-priority-queue):
High for anything that could publish a wrong Safe, Medium for measurement.

### Phase 0: baseline

| Card | Entries | Effort | Prerequisite |
|---|---|---|---|
| CG00-CLANG-GAP-BASELINE | all 98 carded entries | Medium | none (landed, wave 48 round 1) |

- **BEFORE tests.** Turn each cited refusal into an executed BEFORE test at
  the gateway-helper level. The §8 axes (position, effect, evaluation order,
  place, type, boundary, evidence, verdict) name the variants. Each test
  records the entry ID.
- **Attribution.** Attribute the corpus arbiter's Unknown functions to entry
  IDs through their blocking diagnostic families, reusing the census routes
  (`indirect-census`, W46-ACCESS-PATH-CENSUS). Rank the phase 1–2 cards by
  functions freed, not leaves removed.
- **Freeze.** Freeze a per-card PREDICTION.md.
- **Records only.** The card changes no analyzer, and the corpus stays
  byte-identical.

### Phase 1: expression elaboration and contract application (F)

| Card | Entries | Route | Effort | Prerequisite |
|---|---|---|---|---|
| CG01-T-OPERATOR-CLOSURE | E01 E02 E03(T) E14 | T | High | none |
| CG02-ELABORATION-CORE | E03(values) E04 E05 E08 E12(order model) | G, T | High | CG00 |
| CG03-CALL-SUMMARY-INSTANTIATION | K12 K03 | G, T | High | CG02 |
| CG04-CONDITION-ACQUISITION | E09 E13 | G | High | CG02, CG03 (landed, wave 48 round 4) |
| CG05-NESTED-CALL-EFFECTS | E10 E11 E12 | G | High | CG02, CG03 |
| CG06-VALUE-CONNECTIVES-AND-CONDITIONAL | E06 E07 | G, T | High | CG02 |
| CG07-GENERIC-AND-STATEMENT-EXPRESSIONS | E17 E19 | G, T | High | CG02 |
| CG08-CASTS-AND-TYPE-QUERIES | E15 E16 N11 M18 N08(representation) | G, T | High | CG01 |
| CG09-TYPED-SOURCE-CONTROL | C01 C02 C03 | T | High | CG01 |

- **CG01: T operator closure.**
  - *Change.* Add binary `-` `<=` `>=` `^` `<<` `>>`, unary `+` `~`, the nine
    compound assignments T lacks and float/char literals to
    `Importer::expression` (Q11, Q12, Q16).
  - *Consumers.* Every consumer must classify each new node as custody-neutral,
    numerically evaluated or obligation-bearing. A new node no consumer
    classifies stays refused.
  - *Scope.* No numeric guarantee comes with it (N04–N06 are CG20).
  - *Status (wave 48, round 1).* Landed: the importer reads the closure, and only
    the Monkey wrapper projection admits it; the other consumers refuse by name
    (`CG01-T-CONSUMER-ADMISSIONS`). [Report](../../reports/cg01-t-operator-closure/README.md).
- **CG02: elaboration core.**
  - *The layer.* `elaborate(expr, ctx)` yields a value or place, an evaluation
    graph, effect and custody events, and obligations/assumptions (analysis §7).
  - *Value forms.* Assignment as a value, pre/post `++`/`--` with the old and
    new values kept apart, comma sequencing, and the compound-assignment
    read-modify-write with a single left-hand evaluation.
  - *Evaluation order.* Sequenced-before is explicit. An unspecified order is
    never serialised left to right. The layer runs beside the shape routes,
    and parity is required before any shape route retires (hard invariant 3).
  - *Absorbs:* W43-RETURN-COMMA-MACRO.
  - *Status (wave 48, round 2).* Landed on route G as a fallback in statements,
    returns and call-free conditions (gateway rule 10s): 16 Unknown → Safe in the
    corpus arbiter, no correction; route T names the value forms. Residuals
    `CG02-COMMA-OPERAND-RESIDUALS` and `CG02-FOR-CLAUSE-SEQUENCES`.
    [Report](../../reports/cg02-elaboration-core/README.md).
- **CG03: call-summary instantiation.**
  - *One application point.* A contract applies exactly once, where the
    callee, types and actual identities are established. The same contract
    then has the same acceptance range in statement, condition, nested,
    field and profile consumers.
  - *Substitution.* Formal substitution runs value → identity → formal, and
    address-of-slot → Out destination, over CG10 places.
  - *Total-or-reject.* Declared supported clauses keep total-or-reject.
  - *Absorbs:* W46-ACCESS-PATH-PARITY (the plan-vocabulary part). The
    W44-CALL-ARGUMENT-CUSTODY-POSITION refusals stay by design.
  - *Status (wave 48, round 3).* Landed on route G (gateway rule 10u):
    `Parser::instantiate_call` is the one application point (CG04 and CG05
    call it), acceptance is by the value's use, and a scalar `return`
    operand, effectful scalar actuals and a typed substitution are new
    fallbacks. [Report](../../reports/cg03-call-summary-instantiation/README.md).
- **CG04: condition acquisition.**
  - *Example A* of the analysis: `if ((p = malloc(n)) != NULL) free(p);`.
  - *Lowering.* Acquisition identity, binding, NULL/sentinel split,
    success/failure states and merge are lowered together.
  - *No hoisting.* The call is never hoisted (Q47 stays green as a regression).
  - *Composition.* Nested connectives, type operators and increments inside
    condition leaves compose through CG02.
  - *Absorbs:* W44-CONDITION-CALL-ASSIGNMENT, -FREE-OPERAND, -TYPE-OPERATOR,
    -REFUSED-SHAPES, W44-CONDITION-DISCARDED-DERIVED-RESULT,
    W43-INDEX-TRAVERSAL-CONDITION-WRITE and CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED.
  - *Status (wave 48, round 4).* Landed on route G (gateway rule 10x): the
    NULL/sentinel split through `ResultUse::TestedSplit`, refused on any node
    a CFG cycle may pass through (`CG04-LOOP-SITE-GENERATIONS`, a soundness
    defect of the statement form, registered); type operators and increments
    decided per leaf. +1 Unknown → Safe in the corpus arbiter, no correction.
    [Report](../../reports/cg04-condition-acquisition/README.md).
- **CG05: nested-call effects.**
  - *Example B:* `sink(release_and_status(p))`. Inner non-identity effects
    (Owned, Released, Consumed, Out) apply inside-out before the outer call.
    Depth goes beyond one, and calls inside a larger argument
    (`f(g(p) + 1)`) are in scope.
  - *Sibling calls.* `sink2(f(p), g(q))` needs both orders checked, or
    commutativity proved from contract footprints.
  - *Absorbs:* NESTED-CALL-IN-ARGUMENT-POSITION, W44-PRINTF-NESTED-INNER-CALLEES,
    W43-RETURN-COMPOUND-CALL and CJSON-DELETE-NESTED-DETACH.
  - *Status (wave 48, round 4).* Landed on route G (gateway rule 10y): nested
    calls applied inside-out through `instantiate_call` at any depth, data
    actuals, and a footprint order check for unordered calls; +2 corpus Safe,
    no correction. Residuals `CG05-UNCONTRACTED-INNER-CALLEES`,
    `CG05-FOOTPRINT-ADDRESS-CONSERVATISM`, `CG05-CALL-ROOTED-PLACES` and
    `CG05-ELABORATOR-CALL-FOOTPRINTS`.
    [Report](../../reports/cg05-nested-call-effects/README.md).
- **CG06: value-producing connectives and `?:`.**
  - *Positions.* `&&`, `||` and `?:` in assignments, actuals, returns and
    operands. The unevaluated side produces no effects.
  - *Merges.* `?:` merges the selected branch's alias and custody state. The
    GNU omitted middle operand is handled separately.
  - *Absorbs:* RETURN-POSITION-CONDITIONAL, W43-RETURN-CONDITIONAL-OPERANDS,
    and the `?:`-actual and pointer-`?:`-return rows of
    W47-ACCESS-PATH-POSITION-RESIDUALS.
  - *Status (wave 48, round 4).* Landed on route G (gateway rule 10z): branch
    regions in the elaborator, and a refused statement, declaration or
    `return` lowered by branching on each region's deciding operand, only
    the evaluated arm on each edge; **+18** corpus Safe at its base, **+19**
    on main `41719835` (with CG05), no correction.
    Route T unchanged. Residuals by owner in
    `CG06-VALUE-CONNECTIVE-RESIDUALS`.
    [Report](../../reports/cg06-value-connectives-and-conditional/README.md).
- **CG07: `_Generic` and statement expressions.**
  - *`_Generic`.* Use Clang's selected `GenericSelectionExpr` association where
    the associations differ, never evaluating the controlling or unselected
    operands. G uses invariance today (Q21).
  - *Statement expressions.* GNU statement expressions get their inner scope,
    final value, internal calls and escape composed into the outer scope.
  - *Status (wave 48, round 7).* Landed on route G (gateway rule 11g):
    Clang's selected association after route 1 (no corpus selection needs
    it: every one is glibc's invariant shape), and statement expressions
    lowered from the stream when discarded, a whole initializer, assigned to
    a named object or returned, under hygiene and escape premises: 1,414 of
    them in the corpus (1,327 glibc `assert`), braced leaves on their lines
    1,338 → 69, **0** corpus moves (their inner leaves remain, above all
    `__assert_fail`, CG21's). Residuals `CG07-STATEMENT-EXPRESSION-POSITIONS`
    and `CG07-GENERIC-TOKEN-AST-MATCH`.
    [Report](../../reports/cg07-generic-and-statement-expressions/README.md).
- **CG08: casts and type queries.**
  - *Cast kinds.* Pointer↔integer, ToVoid, the ToBoolean kinds, integer↔FP and
    FP-width, atomic and address-space conversions, each a *representation*
    with an explicit loss/provenance flag. Numeric transfer is CG20.
  - *sizeof and _Alignof.* Evaluated VLA operands are separated from
    unevaluated ones. Clang-evaluated constants replace the neutrality marker
    `1` wherever a value is consumed, and the marker is never reused in bounds
    or branches (example D).
  - *Absorbs:* W44-SIZEOF-OPERAND-RESIDUALS, W44-SIZEOF-CAST-SCALAR-QUERY,
    SIZEOF-TYPE-FOLD-NO-LEAN, CAST-RETURN-UNCOMMITTED-OPERAND and
    W45-CALLEE-CAST-LEXICAL-PREMISE.
  - *Status (wave 48, round 5).* Landed on route G (gateway rule 11b) with
    T's implicit enumerators (N11): **+16** corpus Safe, system +3, no
    correction; a wrong Safe repaired (a pointer returned or stored as a
    truth value escaped or bound it; no corpus instance). Residuals by owner
    in `CG08-CAST-AND-QUERY-RESIDUALS`.
    [Report](../../reports/cg08-casts-and-type-queries/README.md).
- **CG09: typed-source control.**
  - *Statements.* General `while`/`for`/`do`, `switch`/`case`/`default`, and
    `goto`/labels/`break`/`continue` in T. Either lower through the
    gateway's CFG or add T edges.
  - *Lifetimes.* Local lifetime ends on every jump.
  - *Precision.* G already has all of these (Q06, Q07). This is not a
    precision card.

### Phase 2: typed places and stores (F/R)

| Card | Entries | Route | Effort | Prerequisite |
|---|---|---|---|---|
| CG10-COMMON-TYPED-PLACE | M01 I04 (K03 place side) | G | High | CG00 |
| CG11-POINTER-VALUED-STORES | M02 M03 M04 | G | High | CG10; the owned-field Lean model; W45-SR5 findings closed |
| CG12-VOID-VIEW-READBACK | M05 | G | High | CG11 |
| CG13-UNION-AND-BITFIELD-PLACES | M06 M07 | G, T, core | High | CG10 |
| CG14-AGGREGATE-VALUES-AND-INITIALIZERS | M08 M09 M10 M11 | G, T | High | CG10, CG11 (landed on G, wave 48 round 7) |
| CG15-STORAGE-DURATION-AND-VOLATILE | M24 M23 | G, T | High | CG10 |

- **CG10: common typed place.**
  - *The place.* DeclID / FieldID / Index / offset / view, with AST-node and
    SourceRange identity in place of unique-line token matching (I04). Mixed
    `->`, `.`, `[]` and `*` paths get a *tracked* root, not only a
    never-bound one.
  - *Absorbs:* W46-ACCESS-PATH-TRACKED-MIXED-PLACES, W44-CHAIN-NESTED-AGGREGATE,
    W43-INDIRECT-UNTYPED-SINGLE-ACCESS, REFUSED-HANDON-LINE-GRANULARITY and
    W44-AGGREGATE-PREMISE3-MEMBER-NAME.
- **CG11: pointer-valued stores.**
  - *Scope.* This is the unsupported coverage queue's FIELD-CUSTODY (P5),
    promoted from candidate. It covers `s->p = q`, `*slot = p` and
    `slots[i] = p`, and stores through caller-owned objects, formals and
    aliases. Nested fields, arrays, multilevel pointers and function pointers
    are in scope.
  - *Semantics.* Value store, alias addition and overwrite, the old
    obligation, and escape are separate events.
  - *Example C.* The Q36 fixture moves only through source store semantics,
    not by a trusted contract on `set`.
  - *Absorbs:* W43-INDIRECT-POINTER-VALUE-STORE, the seven W45-SUB-OBJECT-STORE-*
    rows, W44-CALL-RESULT-INDIRECT-DESTINATION and W43-COLLECTION-NAME-STORE.
  - *Status (wave 48, round 3).* Landed on route G as gateway rule 10v: a
    pointer-typed store is read from the Clang AST and admitted only for a value
    with no identity (null, string literal, function, a function-pointer name
    bound to nothing, the destination's own object) into an untracked owner,
    and the body publishes no summary. 5 Unknown → Safe in the corpus arbiter
    and 1 in the system route, no correction. Example C stays Unknown: its
    missing premise is an owner identity for `*s` that bounds `*q`'s lifetime,
    or a derived Stored summary mode. The entry is amended, not retired.
    Residuals: `CG11-UNRESOLVED-OWNER-POINTER-STORES`,
    `CG11-POINTER-STORE-SUMMARY-MODE`, `CG11-FUNCTION-POINTER-PARAMETER-CLOSURE`
    and `CG11-UNREAD-POINTER-PLACES`.
    [Report](../../reports/cg11-pointer-valued-stores/README.md).
- **CG12: typed readback from `void *` fields.**
  - *Change.* A post-cast view and pointee type are connected to the stored
    identity, by contract or derivation.
  - *Absorbs:* W47-FIELDS-BY-MEMBER-VOID-ESCAPES.
- **CG13: unions and bit-fields.**
  - *Bit-fields* are an R card. Source fields map to `disjoint_footprint`
    storage units, and accesses become read-modify-write events. Different
    bit ranges are not automatically disjoint (example E).
  - *Unions* need an overlapping-storage and effective-type view boundary
    (V), or a contract boundary.
  - *Absorbs:* W46-UNION-MEMBER-PATH, W44-CHAIN-INCOMPLETE-RECORD (the union
    part) and W43-LIBC-GETPEERNAME-SOCKADDR-UNION.
  - *Status (wave 48, round 3).* Landed on route G (rule 10w): union steps
    by storage (punned pointer reads and pointer stores refused, a framed
    store through a pointer-bearing union), bit-fields as C11 storage units
    connected to `disjoint_footprint` for naming an unsequenced pair. Corpus
    +6 Safe; `getpeername`'s transparent union reported and left to the
    declared-ABI reader ([report](../../reports/cg13-union-and-bitfield-places/README.md)).
- **CG14: aggregate values and initializers.**
  - *Copies.* Whole-struct copies and assignments become field-wise alias and
    custody transfer (the `h->value = h->value` fixture).
  - *Initializers.* Designated and general initializers get implicit zero-fill
    and initial aliases. Compound literals get identity and block/file
    lifetime.
  - *Strings and arrays.* String and array initializer extents feed CG16 only
    when bounds are claimed.
  - *Status (wave 48, round 7).* Landed on route G (gateway rule 11f):
    custody-free record copies, identity-free braced initializers (a local
    that can hold a pointer through rule 10v's flow guard), compound literals
    as borrowed actuals only, string initializers with extents recorded and no
    bound. **+8** corpus Safe (2 direct, 6 through derived summaries), system
    and slice unmoved, no correction. Pointer-bearing copies, aggregate values
    in actuals, returns and conditions, and several declaration forms remain
    (`CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES`,
    `CG14-AGGREGATE-VALUE-POSITIONS`, `CG14-DECLARATION-FORMS`); route T is
    unchanged. [Report](../../reports/cg14-aggregate-values-and-initializers/README.md).
- **CG15: storage duration and volatile.**
  - *Storage.* Global, static and thread-local identity and lifetime; T
    currently rejects them.
  - *Volatile.* Volatile accesses become separate observable effects with a
    count and an order, kept apart from custody.
  - *Absorbs:* GATEWAY-VOLATILE-PREMISE and CT41-VOLATILE-COMPLEMENT-UNMEASURED.

### Phase 3: connections to existing core checkers (R)

| Card | Entries | Effort | Prerequisite |
|---|---|---|---|
| CG16-INDEX-AND-OFFSET-EXTENTS | M14 M15 M16 M17 | High | CG10; CG20 for ranges |
| CG17-LAYOUT-DERIVED-EXTENTS | M12 M13 M19 E18 | High | CG08, CG10 |
| CG18-FOOTPRINT-CONNECTION | M20 M21 M22 M26 W08 | High | CG10, CG13 |
| CG19-ATOMIC-SOURCE-ROUTE | W01 W09 | High | CG10 |

- **CG16: index and offset extents.**
  - *Bounds.* `0 <= i < count` against extent contracts. The intervals in
    `indexed_collection.rs` track release obligations, not bounds (Q25).
  - *Offsets.* Byte offset = element size × index.
  - *Pointer comparisons.* Pointer difference and ordering get same-object
    obligations. One-past formation is separated from dereference range.
  - *Absorbs:* W43-INDEX-TRAVERSAL-STEP-COMPUTED,
    W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS and W46-ROUTING-QUERY-EXTENTS.
- **CG17: layout-derived extents.**
  - *Layout facts.* VLA bound evaluation and scope-exit lifetime, and
    flexible-array remaining extent (allocation-size contract + `offsetof` +
    element width).
  - *Obligations.* Access alignment against derived addresses.
  - *container_of.* `offsetof` values and `container_of` base/subobject
    provenance.
  - *Absorbs:* W45-FB-OR6-ELEMENT-IDENTITY.
- **CG18: footprint connection.**
  - *Liveness.* Subobject and view places connect to core generations without
    weakening existing liveness checks.
  - *Stated premises.* Cross-allocation disjointness needs an allocator model
    or a stated assumption. Effective type and `restrict` are stated as
    boundary premises.
  - *Bytes.* memcpy/memmove/memset byte semantics apply only when byte-level
    safety is claimed.
  - *Locks.* Access footprints compose with mutex invariants beyond
    `source.socket.v1`.
  - *Absorbs:* W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE.
- **CG19: atomic source route.**
  - *Change.* C `AtomicExpr`, atomic casts and `_Atomic` accesses route to the
    core atomic model.
  - *Publication.* Publication generalises to initialise → publish → observe →
    use → reclaim, derived from source.
  - *Absorbs:* CT41-ROUTE-HAS-NO-PRODUCTION-CALLER and
    CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING.

### Phase 4: vocabulary, numeric and weak-memory extensions (V), by guarantee

Open a Phase 4 card only when its property is chosen as a published
guarantee. For Fluent Bit, the analysis suggests size and bounds arithmetic,
conditional ownership transfer and callback lifetimes first: CG20, CG23 and
CG22.

| Card | Entries | Effort | Prerequisite |
|---|---|---|---|
| CG20-NUMERIC-DOMAIN | N01–N07, N08(transfer), C04, C05 | High | CG01, CG08 |
| CG21-CALL-EXITS-AND-BOUNDARIES | C09–C13, E20, I06 | High | CG02 |
| CG22-INDIRECT-CALLS-AND-CALLBACKS | K01 K02 | High | CG03 |
| CG23-RELATIONAL-POSTCONDITIONS | K05 K11 | High | CG03 |
| CG24-ALIAS-EXTENT-FRAME-VOCABULARY | K04 K06 | High | CG03, CG16 |
| CG25-RESOURCE-KINDS | K07 | High | none |
| CG26-VARIADICS | K08 K09 K10 | High | CG03 |
| CG27-LOOPS-RECURSION-BUDGETS | C06 C07 C08 | High | CG09 |
| CG28-WEAK-MEMORY-EVENTS | W02–W06 | High | CG19 |
| CG29-EXECUTION-COVERAGE-AND-RECLAMATION | W07 W10 | High (candidate) | CG28 |
| CG30-DIALECTS-FLAGS-TYPES | I01 I02 I03 I05 N12 N13 N14 | High | none |

- **CG20: numeric domain.**
  - *Types.* Clang's canonical widths and signedness per target identity
    (LP64/LLP64), with promotions and the usual conversions imported, not
    re-derived.
  - *Operations.* Sign/zero extension and truncation, and signed overflow
    versus unsigned wrap.
  - *Obligations.* `/` and `%` exception obligations, shift-count and
    signedness obligations, and mixed-signedness comparisons.
  - *Relation to the core.* The core fixed-width table from W48-QR-SCALAR-TYPES
    is checked arithmetic, not C semantics. It is reused only through an
    explicit C interpretation.
  - *Precision.* Switch-value case pruning and branch correlation are
    precision work.
  - *Absorbs:* W45-FB-SCHED-RANDOM-RANGE.
- **CG21: call exits and boundaries.**
  - *Exits.* `noreturn` in every position, `setjmp`/`longjmp` non-local edges
    with invalidation, and `cleanup` attributes on every scope exit.
  - *Other control.* Computed goto target sets.
  - *Boundaries.* asm, asm goto and SEH get explicit boundaries (clobbers,
    memory effects, edges) or refusal. Builtin and extension AST kinds get a
    common opaque boundary. An ordinary function contract is never
    auto-applied to a non-call node.
  - *Absorbs:* W43-LIBC-NORETURN.
- **CG22: indirect calls and callbacks.**
  - *Callees.* Candidate callee sets with common signature and contract
    resolution.
  - *Callbacks.* Capture, escape, repeated invocation and teardown relations
    for callbacks.
  - *Absorbs:* LOADER-ABI-INLINE-FNPTR-DECLARATOR. The EXTERN-CALL-F
    function-pointer part and the specialised W47-LOOP-OWNER-FIELD-KEYED-V-AND-F
    are generalised here.
- **CG23: relational postconditions.**
  - *Change.* Return-code × multi-resource postconditions, and exit-specific
    summary dispositions.
  - *Existing rules.* Existing realloc and nullable rules stay.
  - *Absorbs:* CONDITIONAL-RELEASE-BY-LITERAL, W45-FB-BACKLOG-MAP-ARMS,
    W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL,
    W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED, W43-DESCRIPTOR-REFUSED-SHAPES
    and CJSON-DELETE-SIBLING-CHAIN.
- **CG24: alias, extent and frame vocabulary.**
  - *Frame effects.* Read/write frame effects distinct from custody modes.
    Borrowed is not purity.
  - *Derived relations.* Multi-base, offset/length and callback-spanning
    `Derived` relations.
  - *Absorbs:* W46-SDS-CAT-SAFE-IN-PLACE-REALLOC and
    W45-FB-BACKLOG-REFERENCE-VOCABULARY.
- **CG25: resource kinds.**
  - *Change.* New kinds beyond Heap/Descriptor/Stream/Directory, each with a
    sentinel and a release protocol. No unjustified equivalence to an
    existing kind.
  - *Absorbs:* W44-DLOPEN-DECLARATION-REMEASURE (library handle) and
    W43-LIBC-IN-PLACE-LIFECYCLE.
- **CG26: variadics.**
  - *Custody.* General vararg custody and promotions.
  - *va_list.* A `va_list` state machine
    (`va_start`/`va_arg`/`va_copy`/`va_end`), or whole-helper contract
    boundaries.
  - *Format safety.* A separate checker, only if format safety is claimed.
  - *Absorbs:* W44-VA-START-END-BUILTINS and W46-FCNTL-SETFD-ADDRESS-TAIL.
- **CG27: loops, recursion and budgets.**
  - *Invariants.* Loop-invariant contracts, widening and loop summaries.
  - *Recursion.* Inductive summaries and ranking for general and multi-rank
    recursion.
  - *Budgets.* Budget exhaustion is reported as coverage. It is never Safe.
  - *Absorbs:* W45-COUNTED-USERS-LOOPS, W43-COLLECTION-LOOP-SHAPES and
    JOINED-PROVENANCE-CYCLE-SITES. It re-reads C03-NFR-REACH.
- **CG28: weak-memory events.**
  - *Events.* RMW, exchange and fetch_*; compare-exchange success and failure
    (including weak spurious failure); thread and signal fences.
  - *Release sequences.* Release sequences follow one named standard (C11,
    C++20 or RC11), not a mix.
  - *consume.* Decide: model dependencies, or keep consume explicitly out of
    scope.
  - *Lean.* Lean parity in the WeakMemory module.
- **CG29: execution coverage and reclamation (candidate).**
  - *Execution coverage.* Candidate-execution generation or abstraction with
    justified bounds. One accepted graph is not race freedom (Q34).
  - *Reclamation.* Hazard-pointer and epoch reclamation protocols as
    contracts plus a checker.
- **CG30: dialects, flags and types.**
  - *Dialects.* Explicit C17, C23 and C2y dialects (frontend identity and AST
    differences together). Structured `--target`/`-mabi` flags join evidence
    identity; unrestricted passthrough is not added.
  - *Types.* Canonical type IDs across readers.
  - *Extensions.* `_Complex`, vectors, `_BitInt` and fixed point, only for
    extensions adopted.

## 3. Entries that are not cards

These are class O, or V/O without a chosen guarantee. They are neither
defects nor TODOs. Each is reopened only by choosing its property as a
published guarantee:

- **Out of scope** (LLVM backend, SSA, optimisation, debug output,
  re-implementing Sema, "emit-llvm proves no UB", analysing every external
  body): X01–X07.
- **Delegated or already checked:** K13 (physical ABI lowering), K14
  (cross-TU summary matching; existing rechecking is not a deficiency) and I07
  (per-build guarantee versus all configurations).
- **Guarantee-gated:** N09 and N10 (IEEE and fenv), M25 (uninitialized and
  padding reads) and W11 (liveness and eventual observation).

Coverage: 98 carded entries + 14 non-card entries = 112. Every §5 row
appears exactly once as a primary entry. E03, E12, N08 and K03 are split
between two cards by aspect, as marked.

## 4. Acceptance for every card

The unsupported coverage queue's §4 applies unchanged. That means a frozen
PREDICTION.md, a BEFORE that reproduces the baseline, and an AFTER with every
moving function attributed. It also means named corrections, fail-closed
behaviour, Lean parity registered in an `Audit*.lean`, and no provisional
Unsafe presented as an upstream defect. In addition:

- **Axes.** Test the analysis's §8 axes, not only accepted shapes. Each card's
  report lists which axis values it covers and which stay refused.
- **Route.** Name the route (G, T or core consumer) and source profile that a
  result reaches. A core model's existence is not source reach.
- **Evaluation order.** Never introduce an evaluation order C does not
  guarantee and then claim a source-level result. If a card deliberately
  audits one compiler's order instead, that is a separately stated scope.
- **Contracts as boundaries.** A helper boundary contract is recorded as a
  `Declared` assumption. Proving the helper's body is a separate obligation.

## 5. Dispatch

- **Measured ranking** (CG00-CLANG-GAP-BASELINE, corpus `ae17152b…`; Unknown
  functions freed alone, alias leaves allowed / none, upper bounds;
  [report](../../reports/cg00-clang-gap-baseline/README.md#33-ranking-phase-13-by-functions-freed),
  function lists in its `predictions/`):
  CG10 491/293, CG13 55/48, CG06 40/39, CG11 37/34, CG08 34/22, CG04 31/25,
  CG05 23/19, CG07 18/15, CG03 17/13, CG14 17/11, CG02 12/10, CG15 7/7,
  CG19 2/2, CG16 2/1, CG12 1/1, CG17 0; CG01 and CG09 (route T) and CG18 free
  none in the corpus arbiter. The largest pair is CG10 + CG11 (80 functions).
  2,765 Unknown functions stay blocked by uncontracted calls or other
  unattributed leaves whatever phase 1–3 card lands.
- **Landed:** CG00 (Medium) and CG01 (High, T only).
- **Critical path:** CG02 → CG03, then CG04, CG05 and CG06 in parallel, each
  consuming CG00's function lists. CG10 can start now, in parallel with
  phase 1, and has the largest measured yield.
- **Delegated briefs.** A brief must carry the card's entries, the absorbed
  follow-ups with their current sizes, and the evidence IDs.
