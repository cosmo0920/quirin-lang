# Gateway rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.
Per-card history compacted out of rules 5c, 7 and 8 at wave 45, and 5l's corrected
sentence, are kept verbatim
in the [wave-45 compaction checkpoint](../development/increments/wave-45-compaction-checkpoints.md#gateway),
which is not a second authority.

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-4"></a>

<!-- preserved:constraint-4:start -->
4. A call into an internal C function is modeled only through a declared
   `.qcore` contract or a summary that `observations_match_contract` validated
   against a fresh analyzer run; everything else stays `Unknown`. When touching
   `crates/qcore-c-gateway/src/legacy_summary.rs`: **do not** let a summary be
   published without validation, **do not** seed a formal parameter with entry
   custody the body did not demand (that asserts non-nullness and deletes
   failure paths), and **do not** accept an assume-guarantee fixed point for a
   recursive component **above the neutral element**. The one admissible
   optimistic start is `N(m)` — every formal `Borrowed`, result `None`, seeded
   for every member of an undeclared component and descended to the greatest
   post-fixed point (`legacy_neutral.rs`, card C03-NFR) — and it is admissible
   on exactly two premises, both load-bearing: (1) `N(m)` states **no
   precondition**, installs no entry resource and asserts no non-nullness, so
   it cannot delete a failure path — the hazard this sentence names; (2)
   `N(m)` is a conjunction of **negative, prefix-closed** facts, so its
   soundness is an induction on execution prefixes
   (`QCore.NeutralPrefixPreservation.neutral_prefix_preservation`) rather than
   a termination rank, and divergence cannot manufacture it — while validation
   by re-analysis alone provably cannot refuse a circular *effectful*
   candidate (`an_assumed_effect_appears_in_no_divergent_prefix`). Any route
   that seeds any element above `N(m)` for a member of its own component must
   supply a well-founded rank, as the contextual and countdown routes do.
   Declared contracts always win; a disagreement is a `SummaryConflict`
   diagnostic, never a silent override.
   **"Defined in this translation unit" is not "W4-derivable", and a reach
   argument that counts the former overstates this route by two orders of
   magnitude.** Validation by re-analysis requires the callee's *own* body to be
   fully modeled and defect-free, so a callee that is itself `Unknown` yields no
   summary and its callers gain nothing from it. Measured over the 643
   `return CALL(...)` refusal leaves of the pinned corpus, on one population so
   the shares are comparable: **379 (58.9%) have a callee defined in the same
   `.c`**, but the callee's own body is `Safe` — the precondition for a
   validated summary — in **4 (0.6%)**; the other 375 are 366 `Unknown` and 9
   `Unsafe` bodies, and 264 (41.1%) of the 643 are external. A card scoped on the larger
   figure will not find the smaller one waiting for it: wave 34's arbiter
   withdrew 14 such leaves, of which **13 were decided by a declared `.qcore`
   contract and exactly 1 by this route**. `w4-custody-inference`,
   `w4-custody-extensions`, `w4-gateway-crate`, `remaining-c03`,
   `w4-recursive-summaries`, `c-gateway-return-call-w4` §3.3.
<!-- preserved:constraint-4:end -->

<a id="constraint-4a"></a>

4a. **The W4 derivation scope is not the reported scope, and linkage decides
    what it may add.** Card W44-CORPUS-CALLEE-SUMMARIES widens, under
    main-source selection, only the set W4 *derives* summaries for: the
    selection plus every unselected definition it reaches through direct calls
    that is `static` and not `extern` on its own definition (internal linkage,
    C11 6.2.2p3) and whose name is not reserved in every context (`__x`, `_X`;
    C11 7.1.3). The reported rows, the effective entry and the whole-unit
    `Safe` scope stay the selection alone, and a newly admitted helper publishes
    a summary only through the unchanged `observations_match_contract` gate.
    **Do not admit an external-linkage definition into this same-unit
    derivation scope**: a C99 `inline` definition
    may be bypassed for the external one at any call (C11 6.7.4p7) and an
    external definition may be interposed, so its body need not be the body the
    call runs (`mpack_*`, 86 leaves, stay refused). Rule 4c states the separate
    conditional cross-unit route. **Do not drop the reserved
    filter here**: it keeps glibc header bodies (`__bswap_*`) with the libc
    provider's decision; the card measured that they would validate
    `Borrowed -> None` and decide 599 leaves if admitted. Widening the scope
    also widens the declared-mode gate's reach (constraint 8): a reached
    `static` definition that contradicts a declaration now withdraws it in that
    unit, the refusing direction. `QCore.CalleeSummaryScope`;
    [the card report](../reports/w44-corpus-callee-summaries/README.md).

<a id="constraint-4b"></a>

4b. **A W4 summary is validated on the premises its function's reported row
    reads.** Amends rule 4 (card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS,
    `legacy_summary_premises.rs`, hooks in `legacy_summary.rs`,
    `legacy_derived_summary.rs`, `legacy_recursion.rs` and `legacy_c.rs`).
    Every W4 route infers on a CFG built in the environment of the moment,
    which does not yet hold the candidate, while the reported run builds the
    body with the candidate in force. The body's construction reads the
    function's own contract through its non-`Borrowed` formal set (both the
    protected roots and the immobile formal slots below) and descriptor names. The protected set
    seeds every typed-root premise, which are rule 10o's never-bound closure,
    `may_bind`, the declared locals and rule 10l's AST closure. A copy
    `const T *a = _a;` of a formal the candidate seeds was therefore a
    never-bound root in validation and a bound one in the reported row, and a
    caller certified on a summary whose own row refused
    (`rd_kafka_topic_partition_cmp_opaque`).

    A candidate the environment does not already govern is now re-checked by
    the unchanged `observations_match_contract` gate. The check runs on the CFG
    built with the candidate read as the function's own contract, and a
    candidate that refuses there publishes nothing. This applies to the
    live-entry, removed-wrapper and derived-result routes. The contextual route
    rebuilds both context CFGs and stores the rebuilt combination, which is
    what its reported run reads. Only those two readings change: a call to the
    function inside its own body still resolves through the environment, so no
    recursive assumption enters (rule 4). An all-`Borrowed` candidate reads
    nothing new. A declared function was already validated under its
    declaration's protection. Inferred contracts are heap-kinded, so the
    descriptor reading adds no name. The counted-user check runs after the
    reported analysis and reads declared clauses only. The re-check writes no
    parity rows; its decisions are recorded as `position: "summary-premises"`
    beside the call graph (`"summary-premises-graph"`).

    **Do not** validate a candidate on premises its reported row does not read.
    **Do not** answer the gap by weakening the reported run's protection.
    **Do not** install the candidate in the environment for the re-check,
    because that would resolve a recursive call through it.

    Measured: 8 corpus summaries withdrawn, all Unknown rows, and one Safe
    correction, `rd_kafka_topic_partition_cmp_opaque`. The system slice does
    not move. Premises stated, not proved: that the Rust construction reads
    the own contract nowhere else (trust boundary 28).
    `QCore.SummaryValidationPremises`;
    [report](../reports/w47-summary-validation-protected-formals/README.md).

    **Amendment (W48-BOTTOM-UP-BATCH2): a scalar-pointer formal's own slot
    may step only under a checked whole-body read frame.** After the old
    classifier refuses, a complete typed self-step of one scalar-pointer
    formal may use `PointerStep` when its own contract grants neither entry
    custody nor an out slot. In the function's Clang AST every reference to
    that exact formal declaration must occur in a scalar dereference's
    lvalue-to-rvalue read or a self-increment/decrement whose result is
    discarded in a complete statement or for clause. A nested step, pointer actual,
    copy, address, pointee store, reassignment, shadowed reference or unknown
    use keeps the refusal. This changes a by-value local parameter slot,
    never the caller's slot; the existing step transfer preserves site status,
    field links and containment, derives the new pointer and refuses losing
    the sole name of a live object. The summary re-check reads the same
    immobile set from its own candidate; every non-`Borrowed` mode refuses
    the exception. Other formal-mutation shapes keep their old refusal;
    this is not a general precedence change. Bounds and pointer arithmetic
    validity remain outside custody. Faithful AST extraction and Rust–Lean
    correspondence remain premises (28). `QCore.IndexTraversal`
    (`a_checked_formal_step_preserves_custody_and_caller_slots`,
    `a_read_only_unbound_formal_step_is_admitted`, the shared
    `formalStepVectors`); [report](../reports/w48-bottom-up-batch2/README.md).

<a id="constraint-4c"></a>

4c. **An external W4 home summary crosses a unit only under an explicit
    linked-program premise.** Card W44-CALLEE-CROSS-UNIT-SUMMARY adds an opt-in
    `QCORE_CROSS_UNIT_LINK_MAP` for configured current and pinned audits. The
    map's SHA-256 and its named function-to-home choices are retained in the
    verdict assumptions. They assert that each external call resolves to that
    one definition with compatible function type and calling convention in
    the actual link and interposition environment; the compile database alone
    cannot establish that assertion. A home must be a unique selected
    main-source definition across every complete configured unit, externally
    linkable with no static or inline declaration, have a Safe function row and
    a validated W4 summary, and lack a declaration in force. The home raw,
    marked and preprocessed source, request and environment identities are
    retained both as a home witness and as independently expected identities
    in the caller's audit trust context; certificate issue and replay reconstruct the home
    scope, re-run W4 and compare the exact summary before re-deriving the
    caller. Imports use a separate inferred channel and yield to a caller's
    own definition or a declared contract. ABI, checked-specification and
    recursive-summary environment rebuilds must retain that import channel,
    the caller-expected home identities and the link-map digest; dropping any
    of them invalidates a candidate Safe replay. Missing or ambiguous home,
    incomplete census, failed replay or changed refutation table keeps the
    call uncontracted. Checkpoint composition and contracts-only routes
    refuse the opt-in mode. `QCore.CrossUnitSummary`;
    [report](../reports/w44-cross-unit-summary/README.md).
    The bounded S3 extension retains outgoing lifetime effects separately from
    ordinary imports, compares them with the independently replayed Safe home
    and its W4 record, and preserves exact home/link identities at the caller.
    Unaware consumers still see no ordinary borrowed contract. Guarded mixtures,
    imported-home chains and missing effects remain refused; see
    [ordered lifetime replay](../reports/event-ordered-lifetime/README.md).
    Card CURRENT-RUN-WORKER-SPIN (amended in place): a caller holds, and so
    replays at certificate issue and check, only the homes it may reach: no
    definition of the name in the caller, and the name used in the caller's
    preprocessed unit (inside any brace or after a file-scope `=`; a
    file-scope prototype is not a use) or a candidate some member's run
    closure can deliver (a function designator stored into a member, or
    passed to a setter whose formal a member stores, transitively). Every
    other import is absent from the caller's environment, so a lookup of it
    finds no summary and the call stays uncontracted. A caller that may reach more than
    `REPLAY_BUDGET_PER_UNIT` homes holds none, and its unit row names the
    exhausted budget (`cross_unit_imports_withheld`); the certificate check
    refuses a home set above the budget before any replay. Every check
    still verifies every held home, but a home unit's W4 re-run is shared
    within one process by every check whose recomputed input and environment
    identities, dialect and selected names equal its own. The replay is
    recorded once in a fresh recording, and that record is written into the
    caller's rule 13f recording at every check, as a replay inside it would
    have written it. The budget is
    stated in the run's `cross_unit_link_resolution` assumption, and the
    `current` route prints per-phase and per-unit progress on standard error
    when the link map is selected. `QCore.CrossUnitReplayBudget`;
    [report](../reports/current-run-worker-spin/README.md).

<a id="constraint-5"></a>

<!-- preserved:constraint-5:start -->
5. The typed scalar-field whitelist's last premise — the base is a name this
   function never binds to a resource identity — exists because dropping it
   turned a read through a released pointer into `Safe`. The fail-closed pin
   caught it and **was not weakened**; keep it that way. `remaining-c02`.

   **Amendment (LEXICAL-SCALAR-NOOP-ALIAS-READ):** source admission retains
   that premise, but it does not discharge a same-type peer or pointer-base
   read. Every admitted record-pointer scalar member access carries its
   dereferenced base and alias peers into the existing checked scalar-read
   transfer. A released, undischarged peer or unresolved base makes the result
   non-Safe. The earlier lexical neutral route cannot erase this read in a
   condition or return expression. Direct record values carry no pointer-base
   read; scalar destinations still forget prior scalar facts. No new site,
   containment, or pointer binding follows from the admission.
<!-- preserved:constraint-5:end -->

<a id="constraint-5a"></a>

<!-- preserved:constraint-5a:start -->
5a. A fixed C `sizeof`/`_Alignof` query's operand-class marker is **never a
    computed value**. Do not use normalized query tokens to construct a CFG or
    refine nullness. Scalar assignments forget old scalar facts; expression
    mutations need modeled writes rather than a `NoOp`. Standalone declared
    scalar increments/decrements and custody-neutral scalar assignments use
    `ForgetScalar` when no precise value is modeled. This includes enum,
    compound and scalar-field assignments: `flag = !0` must not retain an old
    zero. Multiple scalar writes and expression-position assignments or
    increments remain refused until their evaluation is modeled.
    `wave23-c-operations`, `c-gateway-wave`.
<!-- preserved:constraint-5a:end -->

Amended by [rule 11b](#constraint-11b): a type query's operand is read
by its evaluation class from Clang's node; the marker still never
reaches a bound or a branch.

<a id="constraint-5b"></a>

<!-- preserved:constraint-5b:start -->
5b. **A `return` admits what a condition admits, and the difference escapes
    nothing.** Card `C-GATEWAY-RETURN-POSITION` (wave 33) amends card
    REACH-GATE's return-position sentence **deliberately**: a pointer-valued
    bounded typed access chain leaf is now admitted in a `return`, so
    `ExpressionRole::Returned` and `ExpressionRole::Decides` admit the **same**
    expressions. What the distinction still decides is the parser's answer, and
    it must stay decided there — an expression the strict `Returned` rule admits
    keeps its `canonical_place`, so `return p;` still escapes `p` and the
    discharging direction is untouched; an expression **only** the `Decides`
    rule admits is answered `returned_place: None`, making the node a `NoOp`.
    **Do not answer the second case with `canonical_place`**: it spells
    `view->head->next` perfectly well, and escaping it would be the discharging
    direction on a place the admission never established (invariant 1). It is
    sound to answer `None` because `admitted_chain` refuses a bound base while
    every tracked `->` place is rooted at a body-acquired name, which `may_bind`
    reports bound by construction. **That second half is a premise, not a
    theorem, and this constraint said so only from wave 34**
    (`RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`).
    `QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place` is
    **conditional on `TrackedBasesAreBound`**, which it takes as an explicit
    hypothesis; the Lean module states in its own words that this is "a premise,
    stated and not proved here" — that the projection tracks no place whose base
    the body does not bind — resting it on the aggregate store's construction
    fact in `QCore.AggregateStore`. The theorem therefore discharges the
    `admitted_chain` half and **assumes** the `may_bind` half, which is the
    load-bearing one; the Lean file is more explicit about this than the ledger
    was. The widening's refusal-monotonicity is unconditional
    (`the_widening_never_creates_an_escape`). This is not academic: a card that
    widens member typing far enough to put a projection-bound place on an
    admitted chain falsifies exactly this premise, so re-read it before taking
    `FB-BACKLOG-CUSTODY`.
    Two boundaries are **measured, not assumed, and are reported as findings**.
    The `None` answer is a **backstop no test distinguishes**
    (`RETURN-POSITION-ESCAPE-UNREACHABLE`): replacing it with `canonical_place`
    leaves the suite green, because the base premise makes that escape a no-op —
    `the_two_answers_agree_on_every_admitted_chain`. Do not simplify it away
    (`without_the_base_premise_the_answers_part` shows the answers separate the
    moment the premise weakens), and do not read the green suite as evidence for
    it; read the theorem. And the **accusing** arm is proved and unit-tested but
    **reaches no function of the pinned corpus** — zero leaves added at the
    arbiter — so the unchanged leak pin records that absence rather than that
    arm's verification (`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`).
    `c-gateway-return-position`.
<!-- preserved:constraint-5b:end -->

<a id="constraint-5c"></a>

5c. **A constant `do … while (0)` is sequential composition, and the key is
    the token.** `do S while (0)` executes `S` exactly once (C11 §6.8.5.2),
    so its CFG header has `continuation` as its **only** successor and the
    back edge to `body_entry` is a path the C does not have. Removing that
    edge is the **admitting** direction (invariant 1), which is why three
    things must survive refactors. The rule is keyed on the **single literal
    token `0`** and never on an evaluated expression — constraint 5a's rule,
    applied here — so `while ((0))`, `while (1 - 1)`, `while (0 == 0)` and
    `while (0 && 0)` keep the back edge, and answering "not literal zero" is
    the conservative answer, so any widening of `condition_is_literal_zero`
    (the unclaimed spellings `0u`, `0L`, `0x0`, `'\0'` included) is an
    admitting change needing its own measured re-pin. The **exit** edge is
    never the one removed and the header node is retained, so the rule removes
    a path and never a destination, and no leaf's internal coordinate moves
    for an unrelated reason. And the rule does **not** generalise to
    `while (1)`: modelling that exactly would remove an *exit*
    edge, which is the accusing direction and needs its own Lean statement and
    re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`). A pre-existing route already
    removes the **exit** edge for `for (;;)`: in `legacy_c.rs`,
    `condition_always_true: condition.is_empty()` sets the flag where the
    `for` header is read, and `self.nodes[header].successors = if
    *condition_always_true` is where it drops `continuation` from the
    header's successors (cited by identifier, because line numbers drift: `DOCS-COMPACT-7` §7.3
    item 5). It
    is exact (C11 §6.8.5.3) and `break` still reaches `continuation`, but it
    carries neither a Lean statement nor a pin, and it is not owned by card
    `C-GATEWAY-CONSTANT-DO-WHILE` (`FOREVER-FOR-EXIT-EDGE-UNOWNED`).
    The `unmodeled call, aggregate or indirect access in do-while condition`
    leaves (28 on the current baseline, 32 at the card's re-pin) are a
    different question and stay refused. Proved as `QCore.ConstantDoWhile`:
    `constant_do_while_is_its_body` the
    `≡ S` equality, `removing_the_back_edge_only_removes_paths` and
    `the_successor_list_never_grows` the conservativity for **every**
    condition, `the_repeated_release_is_a_path_the_c_does_not_have` the
    non-vacuity.
    A separate route decides a loop header from a **modelled scalar value**
    (pinned by `a_modeled_scalar_condition_is_a_separate_pre_existing_route`,
    owned by card `SCALAR-ROUTE-EXIT-EDGE`, wave 37). Deleting a loop header's
    **exit** edge orphans the function's exit
    node, so `report_leaks` never runs and `classify_diagnostics` returns
    `Safe` — measured for `while (!flag)`, `while (flag == 0)`, all three loop
    kinds and the known-non-null arm, swallowing a double release after the
    loop as readily as a leak, and **unsound** rather than merely undefended
    for a `volatile` object, which C11 §5.1.2.3 lets change outside the
    program. A loop header's exit edge is therefore **never removed**, whatever
    the value says: `LegacyCfgNode::loop_exit_edge` records the leaving
    successor where each header's successors are assigned, the `Infeasible` arm
    retains that edge carrying the unrefined state, and the refusal is reported
    from the **settled** states by `report_retained_loop_exit_edges` rather
    than inside the work list, because the arm is reached on intermediate
    iterates (constraint 6). The sibling **back** edge is deliberately
    unchanged (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`), an `if`'s path sensitivity
    is unchanged, and `for (;;)` carries no exit
    successor for the rule to reach. Proved as
    `QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed`, with
    `the_rule_never_promotes_to_safe` for the other direction. Finally, removing a back edge
    **makes a function acyclic**, and that changes which route
    `legacy_joined_provenance::derive` takes for it (constraint 10b) — so this
    rule silently changes *that* pre-pass's reach (654 functions at the card's
    re-pin, with a measured diagnostic effect of zero; reach that moves to the
    topological pass, not reach lost, as `JOINED-PROVENANCE-CYCLES` §8
    measured). Any widening of the key therefore moves two subsystems and must
    re-measure both (`docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py`,
    `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`). The measurement chain, the
    superseded `CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED` ownership and the
    wave-35 close's two integrator clauses are in the wave-45 compaction
    checkpoint. `c-gateway-constant-do-while`, `joined-provenance-cycles`,
    `scalar-route-exit-edge`.

<a id="constraint-5d"></a>

5d. **The B01–B03 effectful-call lowering preserves evaluation frequency.**
    This additional route admits exactly one ordinary, unconditionally evaluated
    contracted call whose callee has no definition in the translation unit, and a
    custody-neutral residual. A local provisional summary cannot supply this new
    route's external-declaration premise. It refuses short-circuit,
    conditional, assignment, increment/decrement, type-operator, nested-call and
    multiple-call shapes. This boundary applies to the additional route; it does
    not revoke the older neutral-expression admissions. The existing statement
    classifier supplies the call instruction and all argument, alias and effect
    obligations. In an `if`, both unknown-result successors receive the post-call
    state. In loops the call occupies the condition node: back edges and
    `continue` revisit it (`for` after its increment), while `do`/`while` `break`
    bypasses it. `switch` evaluates it once. No loop exit or back edge disappears.
    Declined probes restore site numbering. B01's scalar residual route additionally
    requires an externally declared callee (no definition in this translation unit),
    a resource-free call result, and uses a `NoOp` exit, never a synthetic
    custody escape. Existing exact whole-call return behavior is preserved.
    [B-series implementation and evidence](../reports/b-series-integration/README.md).

<a id="constraint-5e"></a>

5e. **Opaque return evaluation does not establish custody identity.** B01's
    additional direct-member route retains the ordinary `OpaqueValue` classifier's
    complete typed-read, untracked-base and alias-read obligations, requires one
    member selection, and checks the reserved result-place collision before
    admission. It evaluates the read and uses a `NoOp` exit. It must not escape a
    canonical spelling whose resource identity the classifier did not establish.
    These are maintained extraction/correspondence premises, not a whole-C proof.

<a id="constraint-5f"></a>

5f. **Every admitted nested sibling retains its obligations.** B04 widens the
    existing one-level neutral nested-call rule from one call to a complete list
    of sibling arguments. Each inner call independently needs an in-force
    contract with result `None`, every parameter `Borrowed`, exact arity, a whole
    outer argument in a data/Borrowed slot, and denotable or data-only actuals.
    The existing scalar-result dereference wrapper keeps its typed-result check.
    Deeper nesting, partial arguments, missing contracts and custody effects stay
    refused by this route. The complete sibling set is admitted or refused
    together. Every sibling's liveness and may-alias obligations survive, and
    every inner identity transfer precedes the outer call. Identity transfers
    make C's unspecified sibling order unobservable in this resource model;
    they do not establish arbitrary runtime purity or whole-C correctness.
    [B04 evidence](../reports/b04-nested-call-argument/README.md).

<a id="constraint-5g"></a>

5g. **W42-INDIRECT-ACCESS adds one route disjoint from the existing unbound
    scalar-field no-op:** an exact `BASE->FIELD = RHS` statement may lower to
    `ScalarMemberStore` only when `FunctionTypes::may_bind(BASE)` is true,
    `BASE` is a declared record pointer, `FIELD` is a declared scalar field,
    and `RHS` is nonempty, scalar-only, contains no
    member/chain/subscript/dereference/call/assignment, and contains no
    increment or decrement. Transfer retains the existing same-type
    alias-read obligation and admits the store only when `BASE` resolves to
    exactly one known, non-null, live identity with neither nullable nor
    unresolved-effect markers. The scalar destination and scalar-only RHS
    cannot rebind tracked identity, so both admitted stores and flow-refused
    stores preserve pointer, custody, field-link, and scalar-fact maps; a flow
    refusal still records `UnknownAlias` and keeps the function non-Safe.
    Pointer fields remain refused by this route; shapes outside this
    conjunction retain their prior handling, including the separate unbound
    scalar-field no-op. See [the card report](../reports/w42-indirect-access/README.md).
    **Amendment (card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS; ledger
    `RECORD-BODY-TOTAL-OR-REJECT`): the lexical declaration reader reads past
    function-pointer declarators, and this route keeps its domain.**
    `typed_field_access::read_record_body` reads a record whose only unparsed
    member declarations are exactly `SPECIFIERS ( * NAME ) ( PARAMETERS )`
    (one declarator, no array, no qualifier, a balanced parameter list closing
    the declaration): such a member joins no class, so every access of it
    keeps its refusal, and every other member is classified as in the same
    body without it. `read_parameters` reads a formal of that shape as a name
    of `Opaque` type, which no typed rule admits, instead of refusing every
    declaration of the function. Any other parenthesised declaration, and any
    `{`, `}`, `:` or `=` in a body, still refuses the whole record or function
    (`record_body_refusal`, shared with the record-body census). **Do not**
    let this route's base domain grow with the reader: it lowers to a store
    whose flow check refuses an unsettled base before the statement's refusal
    reaches the later fallbacks (the Clang-typed statement reading, rule 10k),
    so `scalar_member_store_base` reads the member through
    `scalar_field_of_a_fully_read_record` (records read with every member)
    and declines a function read past a function-pointer formal
    (`FunctionTypes::function_pointer_formals`).
    Without that, the card's first corpus run moved `cio_options_init`
    `Safe -> Unknown` (`memset(options, …)` then scalar stores the Clang
    reading had decided). Other lexical routes with flow obligations (W43's typed
    scalar statement, the W44 chain reading of a condition) are not so held; the precedence itself is
    `LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS`. Premise stated, not
    proved: a function pointer holds a code address, never an object the
    projection tracks. `QCore.FunctionPointerDeclarators`
    (`skipping_preserves_the_other_members`,
    `a_function_pointer_member_is_never_classified`,
    `an_unreadable_member_refuses_the_record`,
    `the_reading_extends_the_old_reading`,
    `a_record_read_past_a_function_pointer_is_outside_the_store_route`,
    `a_function_read_past_a_function_pointer_formal_is_outside_the_store_route`,
    `a_function_pointer_formal_is_opaque`,
    `an_unreadable_formal_refuses_the_function`, the vectors);
    `tests/function_pointer_declarators.rs` and the reader's unit tests;
    [report](../reports/fb-loop-chain-blockers/README.md).

**Amendment (LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS): a flow-refused scalar
access may use an independently discharged Clang plan.** This narrowly amends
the older-route precedence of this rule and rule 10k for `ScalarMemberStore`
and `IndirectScalarAccess` whose destination is scalar, with no borrowed
pointer destination or readback. The existing route remains stored and is
selected unchanged whenever all its obligations discharge. Otherwise the same
exact statement or call-free condition operand may carry a separate Clang
plan, read under that reader's own declaration, occurrence, closure and
per-hop premises. A candidate must have exactly the old scalar destination
and refused-write frame, no pointer store/destination/readback, and every old
same-type alias read in addition to its own. Syntax admission is not flow
discharge: only zero diagnostics from the complete typed transfer on the same
incoming state selects it. If it refuses, the exact old output and diagnostics
are selected. Probes isolate pointer/custody state, diagnostics, null-premise
frames and the consultation log; only the chosen outcome and its named premises
are committed, with one scalar effect and no site allocation or call. All outer
descriptor, in-place, iteration, formal-mutation, source/ABI and held-contract
guards remain. No route is deleted; no null question, alias, custody effect or
unresolved call is declared discharged by precedence. The existing Clang
extraction and Rust–Lean correspondence premises remain stated rather than
proved. `QCore.ScalarPrecedence` proves the bounded selection and same-effect
model; [report](../reports/lexical-routes-typed-fallbacks/README.md).

**Amendment (W48-BOTTOM-UP-BATCH5): opaque field reads preserve scalar type.**
An older `OpaqueValue` assignment may use the complete existing Clang statement
plan when its exact destination is scalar, with no borrowed destination,
readback, pointer store or refused-write frame. Retain every original alias
read together with the plan's liveness and per-hop read obligations. The checked
read transfer forgets only that scalar destination's old binding; it introduces
no opaque pointer identity or known scalar value. Every other pointer binding,
resource, diagnostic and feasible comparison outcome remains. A missing,
unsupported or pointer-valued plan keeps the old instruction. Outer descriptor,
in-place, formal-mutation, iteration and source-identity guards remain; null-test
and status-witness semantics are unchanged. Faithful extraction and Rust–Lean
correspondence remain premises28. `QCore.ScalarReadIdentity` models the exact
admission and scalar forgetting; [report](../reports/w48-bottom-up-batch5/README.md).

<a id="constraint-5h"></a>

5h. **A desugared condition applies a call exactly where C evaluates it.**
    W44-BRANCH-CONDITION-CALLS lowers a refused call-bearing controlling
    expression to its calls, each the ordinary classifier's `ContractCall`, and
    one branch per operand, with `&&`, `||` and `?:` as edges
    (`legacy_condition_desugar.rs`; `QCore.ConditionCallDesugar.the_desugaring_is_c_evaluation`).
    It is consulted only where the position's lowering would be its refusal,
    and a declined plan restores `next_site` and the element counter. Keep four
    premises: a custody result refuses (so `if ((p = malloc(n)) == NULL)` keeps
    its refusal); a local callee in the enclosing function's own recursive
    component refuses, because its contract may be constraint 4's provisional
    `N(m)`; two calls of one operand are admitted only when each is the
    identity on the abstract state; and in a loop every edge to the exit is a
    `loop_exit_edge` (constraint 5c's retained exit). An unresolved name the
    desugaring now null-tests reports `unknown_alias` where the refusal stood;
    that exchange is expected. Rule 10x amends the first premise (a custody
    result split on its own failure value off every cycle) and replaces the
    whole-condition type-operator and increment refusals by its per-leaf
    decision. [Report](../reports/w44-branch-condition-calls/README.md).


    **Amendment (W48-BOTTOM-UP-BATCH3): scalar accessor comparisons.** A
    refused controlling-expression leaf may read exactly `*F()` or `(*F())`
    compared by `==`/`!=` with one unsuffixed decimal literal. F must have no
    body in this unit, an in-force zero-parameter contract with result None
    and no container effect, and an unambiguous unit declaration returning
    exactly one pointer level over a scalar type the existing pointee reader
    admits (volatile, atomic and unresolved pointees refuse). The ordinary
    instantiator still supplies the complete checked call, at its original
    short-circuit/loop position, before a NoOp decision exploring both edges.
    No pointer identity, scalar value, nullness or successor pruning follows.
    Parameters, derived/owned results, volatile or unread pointees, richer
    residuals and local bodies gain no admission; return consumers are
    unchanged. The existing accessor custody contract remains a declared
    assumption, with its reading-premise text naming this position explicitly.
    Memory validity outside tracked custody remains outside this property.
    `QCore.ScalarAccessorCondition` proves observation coverage and preservation
    of the complete post-call state and diagnostics, with all32 admission
    vectors bound in Rust; extraction and correspondence remain premises28.
    [Report](../reports/w48-bottom-up-batch3/README.md).

    **Amendment (W48-BOTTOM-UP-BATCH4): existing checked read leaves compose.**
    After every whole-condition route refuses a call-free controlling
    expression, its complete desugared plan may also stand when at least one
    node is an existing `IndirectScalarAccess`. This amends rule 10k's
    call-free-plan retention condition only: a leaf may have used an older
    typed read instead of the Clang access-path reader. Every leaf must still
    classify completely; its original instruction, alias/liveness/per-hop
    obligations, branch decision and short-circuit/loop position remain.
    No new leaf syntax, scalar value, pointer fact or call effect follows,
    and a refused leaf still refuses the whole expression with probe state
    restored. Switch shape and tracked-descriptor guards stand unchanged.
    `QCore.CallFreeConditionReads` binds all 8 retention vectors and reuses the
    Boolean-tree evaluation theorem for checked-read events, proving identical
    full checked state/error traces for an arbitrary read checker. Extraction
    and Rust correspondence remain premises 28.
    [Report](../reports/w48-bottom-up-batch4/README.md).

<a id="constraint-5i"></a>

5i. **An assignment's destination is its whole left-hand side.**
    `direct_assignment_destination` names a place only when it is the whole
    left-hand side or follows a declarator prefix (identifiers, `*`, `,`). A
    left-hand side ending in a place-spelling suffix it does not name is a
    store into a sub-object, and the lexical gate refuses it
    (`names_a_sub_object`; `QCore.AssignmentDestinationSuffix`). Do not
    reintroduce suffix naming: it bound `recs[i].name`'s value to a local
    `name` and certified a body that leaks one object and frees another, and
    it certified `LZ4_write16`/`LZ4_write32` Safe. A reader that only
    over-approximates a refusing inventory (the protected-formal closure) may
    keep the suffix reader. A static-helper summary whose body validated only
    through a misnamed store is withdrawn with it (`mk_iov_init`).
    [Report](../reports/w44-assignment-destination-suffix-place/README.md).

    A refused sub-object store is re-admitted only as the sub-object it is
    (`LegacyInstruction::SubObjectStore`, W45-SUB-OBJECT-STORES): one member
    of a complete `struct` reached from a root the declarator reader resolved,
    writing that member's place and no name. Its flow premises fall back to
    the gate's refusal byte for byte, so a store that stays refused still
    hands on. It admits only stores that move no custody: a scalar, null,
    literal or never-bound value. Three premises are load-bearing and must not
    be relaxed without a new measurement: a value bound to any tracked
    identity keeps the refusal, including an owned one stored into a record
    the caller reaches (no summary mode states that transfer, so a caller's
    `Safe` would rest on it unseen; the pins of O04-B08 and W44-LEAK-PIN-AUDIT
    hold it, and an escape model produced `release_after_escape` artifacts in
    `cio_file_open`); a formal's pointer is never a stored value (a derived
    summary would call it borrowed while the callee linked it into a record:
    `cfl_list_add`, `mk_list_init`); a statement reading a tracked descriptor
    never reaches the rule. [Report](../reports/w45-sub-object-stores/README.md).
    Rule 10v (CG11) later reads two stores this rule refuses, on the typed
    statement route and under its own premises: a store of the destination's
    own object (`r->next = r`), whose body then publishes no summary, so the
    `mk_list_init` hazard above cannot recur; and a null or other
    identity-free store into a chain or a record this reader does not resolve.
    Every premise above still holds for this rule.

<a id="constraint-5j"></a>

5j. **A scalar-result return decides a read; a cast of a whole call is the
    call.** In a function whose definition declares a scalar result, a
    `return` expression that only reads (no call, write, comma, brace, `?:`
    or unary `&`; no tracked descriptor; a name the body may bind only as the
    root of one always-evaluated dereference) lowers to a typed read over its
    dereferenced bases and an exit that hands nothing back. `return (T) CALL(…);`
    is classified as W4 classifies the call: a refusal stays itself, a `None`
    result is admitted, an `Owned` result only under a pointer cast of a
    declared object pointer (escaping its fresh site); everything else keeps
    the return refusal. A pointer-valued read is never this rule's: an
    interior address is a derived result (`legacy_return_family.rs`;
    `QCore.ReturnExpressionFamilies`). A `*` counts through parentheses, and
    the one dereference ends only after the closing ones: `(*q)->len`,
    `(*q)[0]`, `**q`, `(q->next)->len` and `*(q->f)` are second dereferences
    and keep the refusal, while a `*` applied to a cast group never counts for
    the type name inside it (W45-SR5-SOUNDNESS).
    [Report](../reports/w45-return-expression-families/README.md).

<a id="constraint-5k"></a>

5k. **A member release is custody: admit it only as the last use of its
    place, and guard its summaries at callers.** W45-CALLEE-SUMMARY-CASCADE
    (`legacy_member_release.rs`) admits `B->F` at a `Released { nullable:
    true }` position, lowered `Borrowed` after the unchanged
    W44-INDIRECT-CALL-ARGUMENT-ACCESS reads, **only** under four lexical
    premises: P1 `B` is a formal never copied (every use is `B->G`, a null
    test of `B`, or `R(B)` for a release `R`); P2 the place has one release and
    before it only null tests; P3 no loop encloses the release (`do … while
    (0)` excepted) and the body has no `goto`/`setjmp`; P4 a top-level `R(B);`
    follows with no `return` between, so the formal is released on every path
    and a summary says `Released` for it. A body that releases through a formal
    without releasing the formal gets no rule 1 and no summary. **Do not relax
    these to "untracked liveness is outside the property"**: the first version
    did, and certified `free(n->buf); free(n->buf); free(n);` and a caller
    releasing a member a callee had already released `Safe` (integration
    review). Do not admit a non-null release, a member address, `B[I]` or `*B`.
    A null test of one pointer member lowers to the typed read of its base.
    Every `BranchOnNull` whose place dereferences a pointer, the O04-B08
    aggregate test included, is preceded by the typed read of every pointer it
    reads through; before this card the aggregate test carried no obligation
    and a test through a released base was `Safe`
    (`AGGREGATE-NULL-TEST-AFTER-RELEASE`). Before every call to a function the
    unit defines, the environment does not declare, and whose body (or a body
    it calls, to a fixed point) hands a pointer read to a call,
    `ArgumentReads` refuses a plain-name actual whose target owns sub-custody:
    a caller's linked child at `B->F` would otherwise outlive the release. Do
    not widen the guard to declared callees (`flb_input_chunk_create` left
    `Safe` over the declared `static inline` `flb_free`) or to every undeclared
    one (callers of validated `Removed` wrappers own sub-custody by
    construction). Premises stated, not proved: distinct members own distinct
    objects; W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 4 (a stored pointer is
    tracked only through a bound place or sub-custody), so a card that stores a
    tracked identity into a member without recording either must refuse this
    read; a declaration's `Borrowed` covers what is reachable through the
    argument (P-B08-SHALLOW). `QCore.MemberRelease`;
    [report](../reports/w45-callee-summary-cascade/README.md).
    **A callee that releases the member again** (W45-SR5-SOUNDNESS): P1's
    `R(B)` was accepted on its contract's release mode alone, and
    `q_nullable_free(n->buf); node_destroy(n);` was `Safe` at function and
    unit level with `node_destroy` releasing `n->buf` itself. Every `R(B)` of
    the body is now read for what `R` does to `B->F` (`MemberEffect`): a
    callee this unit defines, declared or not, whose formal is used only as
    `p->G`, in null tests and as the single actual of a callee read the same
    way, releases and touches known members; any other body is unknown. A
    callee that releases `B->F` and follows the release unconditionally makes
    it a `DoubleRelease` candidate (the release statement refuses under
    `member-released-again-by-callee`); one that only touches it, or is
    unknown, withholds the place. A callee with no body here is governed by
    its declaration: a declared release does not cascade to members
    (P-B08-SHALLOW, stated). `QCore.MemberReleaseCallee`;
    [report](../reports/w45-sr5-soundness/README.md).
    Amended by [rule 11c](#constraint-11c): the B08 join records a
    one-sided link as partial, never as a silent unresolved effect, and the
    escape cascade refuses a possibly-live child it does not carry on every
    path.
    Amended by [rule 38j](#constraint-38j): a body that keeps the base may
    release its members under P1-P3 when it states the release as a member
    effect, whose contract only the direct-call route reads; P4 still governs
    every body that releases the base.

<a id="constraint-5l"></a>

5l. **An intrusive-list iteration is modelled on its original spelling, and a
    specification of an audited body is checked, never trusted.**
    W45-FB-BACKLOG-CUSTODY (`legacy_list_iteration.rs`, `legacy_checked_spec.rs`).
    (1) The preprocessed Monkey/CFL headers `for (C = (H)->next; C != (H);
    C = C->next)` and `for (C = (H)->next, N = C->next; C != (H); C = N,
    N = C->next)` (and `prev`) are recognised on the header's **original**
    spelling (rule 10c(3)'s distrust of the stripped clauses stands for every
    other header; `W45-FOR-CLAUSE-ORIGINAL-SPELLING` is discharged for this
    family only). The init is the typed read of `H`'s root (and member chain),
    the condition keeps both successors and the exit edge, and each cursor
    assignment leaves its name unbound: list memory this function does not
    track. (2) The `container_of` statement expression over an active cursor,
    a `(H)->next`/`->prev` read or an unwritten, never-bound formal binds its
    destination the same way; the declaration reader skips the one
    declaration inside such an expression instead of refusing the function.
    (3) A **plain** iteration refuses a statement that hands its current node
    (`C`, `&E->F`) to any call, or `C` or an element bound from it to a
    position its contract does not declare `Borrowed`; a **safe** iteration
    protects its saved successor `N` the same way and admits unlinking and
    releasing the current element. That unlink is constraint 10f's foreign
    unlink: where 10f's premises hold it gives the body live custody of the
    element at a fresh site (an exit that still holds it leaks it, a second
    release is a double release), and otherwise it refuses on its own.
    Premises, stated not proved: the body does not write or address `C`/`N`
    (checked lexically; declined otherwise); an element is not an identity the
    body tracks until a 10f unlink binds it (the function is declined when one
    call receives both `&X->…` of a name `X` that may bind a tracked identity
    and the iterated head; that lexical guard does not see an element 10f has
    bound, and a re-publication under another head spelling refuses on 10d's
    own rules instead, pinned by W45-SR5-SOUNDNESS); a callee handed
    the container does not unlink the protected node; a plain body is braced.
    **Do not** admit a header from its stripped clauses, bind an element from
    a cursor of a declined iteration, or widen case (2) to a written name.
    (4) A declaration carrying `requires resource_body_checked;` (contract
    spec C06.3) is in force in a unit only while the unit defines the body,
    outside every recursive component, and the body establishes it under the
    set still standing; otherwise it is withdrawn and the defining row
    carries `declared specification of … is not established`. **Do not**
    apply one on a body the unit does not define, and do not accept one
    beside a `SummaryConflict`. It composes with the revision (C06.2) and
    review (C06.1) markers: an unpinned or withheld declaration is never a
    specification. (5) A contracted call refused **before its effects** on a
    member-address or bound-read actual whose aggregate does not resolve
    (O04-B09B10, O04-READ-BOUND) reports `UnknownAlias`, not `Unsupported`, and
    is a refused statement all the same: the live identities it may move by
    its contract — the actuals at custody positions, the entries of its
    container effects and the bound reads it takes — carry an unresolved
    effect (`refused_operands::withdraw_refused_call`), as W44-LEAK-PIN-AUDIT's
    hand-on rule marks a refused statement's. A `Borrowed` actual with no
    container effect is not handed on (the refusal is about the read's
    liveness, not the call's effect; `borrow_residuals_boundary.rs` pins the
    leak that keeps). It only withdraws accusations; the node keeps its
    refusal. Without it a publication refused on a nullable base
    (`mk_list_add(&e->_head, …)` after an unchecked allocation) was accused of
    a leak. **Do not** widen it to every `UnknownAlias`: most report a read
    whose effect still applies. `QCore.ListIteration`,
    `QCore.CheckedSpecification`, `QCore.RefusedCallHandOn`;
    [report](../reports/w45-fb-backlog-custody/README.md).

<a id="constraint-6"></a>

<!-- preserved:constraint-6:start -->
6. **C02-P-ALIAS-LEAK-ORDER:** `report_leaks` on intermediate iterates is
   scheduling-sensitive; a pre-pass keeps it scheduling-neutral with
   `AbstractState` byte-identical. Recorded, not repaired — do not disturb it
   incidentally. `remaining-c02-p-alias`, `remaining-borrow-liveness`,
   `remaining-hygiene-14`.
<!-- preserved:constraint-6:end -->

<a id="constraint-7"></a>

7. Leak diagnostics are **34** across the pinned 185-unit corpus
   (`diagnostic_counts.leak` of the combined baseline
   `docs/reports/w45-container-root-reach/integration/artifacts/`). A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. Every move so far has been deliberate and of one of two kinds, and a
   re-pin names which:
   * **Accusing**: a newly modelled exit, path or call exposes a leak. Admitting
     a previously unmodelled exit is an accusing move by construction, because
     `report_leaks` **skips** an unmodelled exit; an admission card reaching
     bodies that retain should expect the pin to rise, and that rise is a
     finding to attribute, not a regression. (`C-GATEWAY-RETURN-CALL-W4`, the
     first admission card to move no leaf, modelled wrappers that retain nothing.)
   * **Deciding** a leaf, where a **withdrawal** — the site marked through the
     unresolved-effect mechanism, so `report_leaks` skips it (`W44-LEAK-PIN-AUDIT`'s
     refused hand-on rule, extended by constraint 5l (5)) — is not a **discharge**; only a
     discharging contract or rule answers the leak itself.
   No move has promoted anything: a withdrawn leak moves a provisional `Unsafe`
   to `Unknown`, never to `Safe`, and no leak row is a validated upstream
   defect claim. A declarative card that decides uncontracted-call leaves is
   not automatically a leak-pin event (`W36-CORPUS-CONTRACTS-3` and
   `W37-CORPUS-CONTRACTS-4` moved it not at all); the pin is then re-verified,
   not re-derived. A leak leaf's identity is its preprocessed line, and a
   presentation re-pin rewrites a description, not a leaf.
   The pin read 266 from wave 23, 272 from the `C-GATEWAY-WAVE` re-pin into wave
   43, and 271 from later wave-43 cards to `W44-LEAK-PIN-AUDIT`'s base, which classified
   every leaf; its hand-on rule then withdrew 249 on the combined tree
   (270 → 21), and the wave-44/45 cards moved it 21 → 19 → 34 → 35 → 37 → 34,
   each move attributed in its card's report and archived arbiter. The
   per-leaf chain through wave 37 is in the wave-45 compaction checkpoint;
   its residuals stay registered: `AOM-DESTROY-JOINED-PROVENANCE`,
   `AOM-SIBLINGS-BINDING-POISON` (whose repair moves sites toward the
   *discharging* arm, a direction invariant 1 constrains) and
   `CONSTRUCTOR-RESIDUALS-LEAK-271` (withdrawn by refusal at wave 23 for five
   of its six leaves; L19544, on the success path, stands).
   `remaining-o04-b08`, `corpus-repin`,
   `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`, `wave22-aom-destroy`,
   `wave22-corpus`, `aom-destroy-siblings` §6 and §6.1,
   `aom-binding-poison` Headline and §5.2, `gateway-c-location` §5.3,
   `c-gateway-wave`, `c-gateway-return-call-w4` §5, `w44-leak-pin-audit`.

<a id="constraint-8"></a>

8. **A declared contract must agree with the C the unit declares** — in types
   and in modes. A contract whose parameter or result ABI projections the
   analysed translation unit's own C declaration contradicts, or whose custody
   *duty* the validated W4 summary of that callee's body in this unit
   contradicts, is **withdrawn for that unit**: every call site is rewritten to
   a marked uncontracted callee (`mode-parameter-contradiction` /
   `mode-result-contradiction` for the mode arm) and the name is removed from
   the environment the derivation seeds. Both ABI sides are read from what a
   certificate already holds (the preprocessed token stream; the environment's
   retained texts), never from the typed Clang AST, so W4 derivation, W4
   validation and certificate rechecking reproduce the decision — do not move
   either reader. A withdrawal is monotone in the refusing direction but **can
   turn a `Safe` unit into a provisional `Unsafe`** when the withdrawn
   declaration was discharging an acquisition; that is `Unknown`-class evidence
   about the declaration, never an upstream defect claim (invariant 2). The
   derived summary is **never installed in the declaration's place** — an
   inferred summary can be weaker than the declaration it convicts. The mode
   gate takes the *duty* inequality
   (`Retained`/`Transferred`/`Delivered`/`Restored`; `Absent`/`Produced`) and
   deliberately does **not** withdraw on a refinement inside one duty:
   nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
   parameter keep `SummaryConflict` as their only report, because every
   derivable comparison in the pinned corpus is a nullability refinement and
   withdrawing on inequality would withdraw a release contract in every unit
   including `rd_free`/`mk_mem_free`/`flb_free`'s headers. Absence of evidence
   is not evidence: nothing is withdrawn on a body this unit does not define or
   W4 derives no summary for. The scalar projection assumes the LP64 model of
   the audited targets and the reader has no target (`LOADER-ABI-LP64`).
   The [MSVC reader](../reports/msvc-abi-reader/README.md) also reads the closed
   `__int64` / `signed __int64` / `unsigned __int64` family as the existing
   64-bit projections, with the same 64-bit-pointer premise for `isize` /
   `usize`; ordinary `long` is not reinterpreted as LLP64. A newly readable
   cdecl result or parameter involving ordinary `long`, including a typedef
   chain, instead stays unreadable because its target width is not established.
   Exactly one
   function-prefix `__cdecl` preserves the default C parameter/result
   projection, including result pointer levels and qualifiers. Other
   conventions, repeated annotations and mixed/repeated sized specifiers
   remain unreadable. `QCore.MsvcAbiReader` proves the abstract decisions;
   faithful extraction and implementation correspondence remain boundary-28
   premises. Native caller certificates use their declared main-source scope,
   never silently certify unrelated header definitions.
   `remaining-loader-abi`, `remaining-loader-abi-modes`.
   Two precedents fix how the arms read. The **nullability-refinement** arm
   keeps a faithful declaration: `rd_kafka_mem_free` declares
   `Released{nullable}` where W4 derives `Released{nonnull}`, and `rd_free(NULL)`
   reaches `free(NULL)`, so the declaration governs every call site and the
   defining unit's own function stays `Unknown` — the gate working; declaring
   `nonnull` to silence it would assert a precondition the body does not have
   (constraint 4; card `W36-CORPUS-CONTRACTS-3`, `w36-corpus-contracts-3` §5.2). The **withdrawal** arm also
   fails closed on a *reader* boundary, not only on a disagreement: the ABI
   reader resolves a function-pointer **typedef** (the form
   `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it) but not an **inline
   function-pointer parameter declarator**, so such a declaration
   (`rd_kafka_topic_partition_list_sort`, card `W37-CORPUS-CONTRACTS-4`,
   leaf-neutral) is withdrawn at every call site with
   `abi-unreadable-declaration` (`LOADER-ABI-INLINE-FNPTR-DECLARATOR`,
   `LOADER-ABI-UNDECLARED`'s neighbour).

<a id="constraint-9"></a>

<!-- preserved:constraint-9:start -->
9. **A fixture prelude is not a production declaration.** A fixture result is
   a result *for that fixture's declarations*: card O04-B08's flagship
   aggregate store was measured never to reach production, because the real
   member is `void *` and the reduced prelude's is `struct cio_chunk *`. When
   a card publishes a per-function count, say which declarations it rests on
   (`NEIGHBOURS-FIXTURE-PROXY`). `remaining-alias-aggregate`,
   `remaining-neighbours`.
<!-- preserved:constraint-9:end -->

<a id="constraint-10"></a>

<!-- preserved:constraint-10:start -->
10. **A store into an object that outlives the call is refused, not escaped.**
    Publishing a tracked identity into a file-scope object or a `static` local
    withdraws both directions and retires the residual leak through the
    unresolved-effect mechanism. The escape model was measured to **promote**
    the publish-without-release body from `Unsafe` to `Safe`, which invariant 1
    forbids; do not reintroduce it. A null store and a scalar store stay
    admitted on stated grounds, and the decision reads no per-site status and no
    provenance (one exclusion, a declared *scalar* source, was added by
    measurement). **The borrowed and `Out` admissions are withdrawn** (wave 17):
    a body publishing a *borrowed* formal no longer certifies, because
    `Borrowed` asserts the caller's obligations are unchanged and a publication
    falsifies exactly that — the launder it enabled crossed the **summary**
    boundary, so W4 published `Borrowed` for the helper and the helper's caller
    certified too. A declared `Out` slot naming such a destination is refused
    before the call's effects apply, being the `Acquire` arm seen from the
    callee's side. The indirect (pointer-base) shape carried no false `Safe`, so
    its residual *violation* claim is withdrawn rather than strengthened, each
    statement keeping the refusal it already had. `remaining-ctwin-publish`,
    `remaining-ctwin-residuals`.
<!-- preserved:constraint-10:end -->

<a id="constraint-10a"></a>

<!-- preserved:constraint-10a:start -->
10a. **A refused statement invalidates only the bindings it could have
    written.** The blanket poison — every refused statement setting `unknown`
    and `may_be_null` on every binding in the state, including bindings its own
    text cannot have written — is bounded, and only where the reader positively
    recognised all three premises: a plain name, an automatic object of this
    body whose address is never taken, and no write position for that name in
    the refused text. The allow-list is built **positively**, so an
    unrecognised place is always poisoned. A parser-delimited complete
    condition or return expression now carries its own write frame; a
    structural control-flow failure without complete expression text **keeps
    the blanket poison**. Prefix mutation through parentheses is a write
    (`++(p)` must not spare `p`). The refusal
    itself is never weakened: the `Unsupported` diagnostic and `unmodeled`
    path flag are retained, which prevents this rule from certifying a refused
    function. Exposing an additional violation can make its status provisional
    `Unsafe`. Wave 25 measured
    zero unit/function status moves and an unchanged leak pin; the condition
    extension's separate measurements are in `c-gateway-wave`. The frame
    premise is the ordinary C object model under the **no-undefined-behaviour**
    reading (`AOM-POISON-NO-UB`), and the declaration reader
    (`publication::body_declarations`) over-approximates in the direction that
    *reduces* refusal, so the file-scope exclusion is applied unconditionally
    here rather than with `publication::destinations`' shadowing exemption
    (`AOM-POISON-DECLARATION-READER`). Do not widen `may_write_names` without
    measuring: sparing a binding can **unmask** a stale scalar fact a `NoOp`
    mutation left behind (`AOM-POISON-NOOP-INCREMENT`, constraint 5a's already
    recorded hazard). `aom-binding-poison`, `c-gateway-wave`.
<!-- preserved:constraint-10a:end -->

<a id="constraint-10b"></a>

<!-- preserved:constraint-10b:start -->
10b. **Non-nullness does not establish lookup identity.** A null guard on an
    unknown binding may narrow only that binding; it must retain its identity
    refusal and may not remove resources or refine another alias. Conditional
    provenance is derived separately from the custody worklist, for unshadowed
    automatic locals whose addresses are never taken:
    a name denotes a local site on every path where that site
    exists. An absent predecessor is neutral; present predecessors intersect.
    No caller custody is seeded. A **call** forgets every relation it could have written and keeps only those it provably could not (card `JOINED-PROVENANCE-CALL-FRAMES`, wave 36): it forgets its destination and every `Out` slot — both already removed by `rebound_names` — and a `Reallocated` parameter, and it forgets every name outside `cfg.conditional_names`, the unshadowed automatic locals of this body whose addresses it never takes. A name on that list designates a slot no callee can reach under the same no-undefined-behaviour C object model constraint 10a rests on, so the call cannot change what it denotes; passing the name as an argument hands over its value, never its slot. An unrecognised name is always forgotten, and the rule only ever weakens (`QCore.JoinedProvenanceCallFrames.a_call_never_adds_a_name`), the pre-card blanket forget being exactly this rule at the empty frame (`the_empty_frame_is_the_old_blanket_forget`). An **acyclic**
    CFG is solved in one topological pass; a **cyclic** CFG is solved by the
    greatest fixed point of the same transfer (card
    `JOINED-PROVENANCE-CYCLES`, wave 35), which derives facts **only for sites
    whose creating node lies on no cycle** — a site created inside a loop stands
    for one abstract resource per *site* rather than per dynamic object, so a
    per-site identity fact would not be a statement about one of them — and
    which declines the function outright if it has not settled within its
    iteration bound. That bound is **this graph's own proved bound**,
    `nodes * (1 + sites + sites * names)`
    (`QCore.JoinedProvenanceRoundBound.rounds_are_bounded_by_the_graph_budget`,
    card `JOINED-PROVENANCE-ROUND-BOUND`, wave 37) and **not a constant**: that
    card measured the flat `FIXED_POINT_ROUNDS = 64` losing facts at 9 of the
    corpus's 159 sweeping visits, across 4 functions, and replaced it. Reaching
    the cap would now falsify that theorem rather than exhaust a guess, so the
    declining arm is unreachable from `derive`
    (`JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE`) and is retained **only** as
    the fail-closed answer. **Do not restore a constant cap**, and do not weaken
    the declining arm: it is what makes the bound safe to raise. The theorem's
    descent hypothesis is stated of, not proved of, the Rust transfer
    (`JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`), and the naive form of it is
    false — a site's *arrival* increases a count that reads only stored names,
    so absence must read as the top of that site's own lattice. The two routes
    agree wherever both apply: the fixed-point
    equation has exactly one solution on an acyclic graph
    (`QCore.JoinedProvenanceCycles.acyclic_solution_is_unique`), so no acyclic
    body's verdict can move, and soundness on a cycle is
    `fixed_point_is_sound`, proved by induction on the arrival with no
    acyclicity hypothesis. Refusing is always available and always safe
    (`weakening_stays_sound`). A conditional shallow release may settle a known local
    site's disposition only under the checked premises in
    `legacy_joined_provenance::release_site`, retaining the lookup's
    `UnknownAlias` and every other uncertainty. Never apply it to overwritten
    live acquisitions, unresolved prior effects, child/reallocation relations,
    or additional failed call premises. This does not close the production
    `input_chunk_get` residual: its address-taken binding crosses other refused
    operations, and **cycles were never that residual's blocker** — card
    `JOINED-PROVENANCE-CYCLES` measured that function's CFG acyclic, so the
    wave-31 pass already served it. `c-gateway-wave`,
    `joined-provenance-cycles`, `joined-provenance-call-frames`,
    `joined-provenance-round-bound`.

<!-- preserved:constraint-10b:end -->

Rule 10b is amended by [rule 12e](#constraint-12e): on the null edge of a
refused guard, a site joined provenance (over the tie names) ties to the
tested name is settled as a decided guard's null edge settles it; the identity
refusal stays.

<a id="constraint-10c"></a>

10c. **An expression statement is a classified effect or a refusal, never an
    unrecorded no-op, and a callee is read as C reads it.** Every lexical call
    reader keys on the pair `NAME (`; a callee spelled any other way was
    invisible to all of them, and the classifier's last arm lowered the
    statement to `NoOp` (`q_free(p); (q_free)(p);` certified `Safe`). Card
    W45-PARENTHESISED-CALLEE (`legacy_parenthesised_callee.rs`) keeps three
    rules. (1) In every function body, `(`ᵏ`NAME`)ᵏ followed by `(` is rewritten
    to `NAME` right after tokenizing, **before** the review hold, both contract
    withdrawals and W4 read a call: a later rewrite would let `(f)(x)` escape a
    withdrawal that keys on `f (`. Only a callee is rewritten: `NAME` is not a
    keyword or a typedef name the function trusts (`(T)(x)` is a cast; a
    typedef name the function may also use as an object is not trusted), and
    the group is not a call's argument list (`g (f) (x)` calls `g(f)`'s result).
    (2) After it, a `(` that applies a call to anything but a name — after `]`,
    or after a `)` closing an argument list or a parenthesised expression rather
    than a cast or a control header — refuses its position (statement,
    condition, desugared condition, return) wherever that position would
    otherwise be admitted, with `call through a callee expression that is not
    a name`; a position already refused keeps its refusal byte for byte. A
    declaration's own declarators are exempt outside initialisers and array
    bounds. (3) A `for` init or increment clause reaches the classifier with
    its grouping tokens stripped (`split_for_clauses`,
    `W45-FOR-CLAUSE-ORIGINAL-SPELLING`), so `q_free(p)` there was the names
    `q_free p`; a clause whose stripping is not faithful (a bracket, or a group
    that is an argument list, a callee, a cast or wraps more than one name) is
    refused unless its stripped classification already was. Only (1) admits,
    and exactly what the plain spelling admits;
    (2) and (3) only replace an admission by a refusal. **Do not** add a call
    reader that keys on `NAME (` without the rewrite ahead of it, and do not
    let a new admission route bypass the rule-2 hook in `classify_simple`.
    Premises stated, not proved: the lexical reader tells a cast, a header, an
    argument list and a declarator from a callee (trust boundary 28), and the
    typedef reader over-approximates object uses. `QCore.ParenthesisedCallee`;
    [report](../reports/w45-parenthesised-callee/README.md).

<a id="constraint-10d"></a>

10d. **A publication moves custody into a caller's container, and only
    there.** Card W45-FB-RETRY-LIFETIME (`legacy_container_publication.rs`;
    contract specification §C04.8) applies a declared `resource_published` /
    `resource_unpublished` clause after the call's borrowed reading. A body
    object linked into a container reached from an unrebound formal, or from a
    local whose one write is its declaration's initialiser from one, is
    discharged (`ESCAPED`, recorded as published; the set joins by
    intersection). Keep four premises. An unbound formal entry reports its
    unresolved identity, so W4 binds it and derives `Consumed`: **do not** let
    it fall back to the borrowed reading, which is constraint 10's launder. Any
    other container, entry or binding refuses with an unresolved effect: a
    local list does not outlive the call (a static one is 10f's), and a body-acquired
    owner would need an owner/child relation `mk_list` does not have. An
    object that owns linked members is published with them withdrawn, never
    cascaded as escaped, because an unlink could not hand them back. A release
    of a published object keeps `release_after_escape`; a `return` of it and
    an unlink of anything not published on every reaching path refuse (10f
    admits an element of a formal's container). Every
    route that admits a call as the identity because its modes are `Borrowed`
    (nested calls, desugared conditions, neutral positions, sub-object store
    values) must refuse a container effect (`has_container_effect`). The
    lexical reader of entries, containers and rebinding is trust boundary 28.
    `QCore.ContainerPublication`;
    [report](../reports/w45-fb-retry-lifetime/README.md). W45-SR5-SOUNDNESS
    adds two refusals (`QCore.Sr5Soundness`). A publication into a container
    whose root the body also hands whole to a position its contract does not
    declare `Borrowed` (`q_free(q)`, a summary's `Released`) refuses: the
    container need not outlive the call, and whether its release hands on its
    elements is no stated premise (a formal and a local initialised from it
    alone are one object; 10f(2)'s foreign unlink out of such a container, a
    teardown, stays admitted). An entry rule 5l(2) bound over a formal
    (`ctx = container_of(arg, …)`) takes the unbound formal entry's rule: its
    borrowed reading let a derived summary call `arg` `Borrowed` while the
    body linked it.
    Amended by [rule 12d](#constraint-12d): an owner whose field-linked
    children are each exactly live and unencumbered carries them into the
    container, and an unlink of it hands them back; every other owner keeps
    the withdrawal.

<a id="constraint-10e"></a>

10e. **A counted user is an obligation checked per path, and a declaration
    of a defined body is verified, never assumed.** Card W45-FB-RETRY-LIFETIME
    (`legacy_counted_users.rs`; contract specification §C04.7) enumerates each
    reported body's paths after its custody analysis and decides every
    `resource_user_exit` clause: refuted on a path that definitely returns the
    value with a known contradicting count (a provisional `Unsafe` candidate,
    never an upstream defect), undecided where a count or the returned value is
    not known, held otherwise; an uncovered exit must restore every tracked
    count, and a drop below the entry count that reaches an exit with no
    re-addition or release check is the C3 shape. A declared operation or exit
    clause whose function this unit defines is applied at a call only when its
    own body validates here; otherwise the call is opaque. **Do not** apply such
    a declaration unchecked, attribute an operation to anything but an unrebound
    formal, prune a branch the pass has not decided, or treat an uncontracted or
    expression callee as neutral. Loops, `switch`, `goto` and labels leave the
    function's clauses undecided. The frame premise UC-FRAME — a declaration with
    no counted-user clause and no body here changes no count — is stated, not
    proved. A derived custody summary is compared with a declaration's custody
    projection only. `QCore.CountedUsers`;
    [report](../reports/w45-fb-retry-lifetime/README.md). Since
    W45-SR5-SOUNDNESS a call or counter write C may skip (behind `?:`, `&&` or
    `||`) and a call through a call's result (`f(a)(b)`, 10c's rule 2)
    undecide the counts instead of applying on every path, and a declared
    `resource_user_neutral()` of a body the unit defines is applied only when
    that body verifies it, as every other declared clause is.

<a id="constraint-10f"></a>

10f. **A container outlives the call only by a stated root, and an unlinked
    element is custody taken once.** Card W45-CONTAINER-ROOT-REACH
    (`legacy_container_publication.rs`, rules 5–6) widens 10d under stated
    premises. (1) A publication into `&G` or `&G.F…`, `G` an unshadowed
    object of static storage duration (`PublicationFacts::destinations`),
    discharges as a caller's container does (P-CRR-STATIC); a list link is
    additive, so constraint 10's lost-slot argument does not apply, and
    `G->F` stays refused. (2) `mk_list_del(&E->F)` of an element this body did
    not publish gives the body live custody of `E` at a fresh site when every
    write of `E` is a `container_of` binding rule 5l admitted as the
    statement's final instruction, over an iteration cursor (not a saved
    successor) or a `(H)->next`/`->prev` lookup, each container rooted at an
    unrebound formal, a local initialised from one, or a static object; and
    `E` holds no binding at the unlink. An exit that still holds it leaks it,
    a second release is a double release, a second unlink refuses, and a
    publication moves it on. Premises, stated not proved: an object linked in
    a container is a heap object the container owns (P-CRR-MEMBER), and no
    call between its binding and the unlink removed or released it
    (P-CRR-LINKED). (3) A body with such an unlink publishes no summary and
    establishes no specification (`has_foreign_unlink` in the W4 gate): a
    caller would read `Borrowed` for the container's formal and keep
    restoring its own published objects the callee released. **Do not**
    admit a formal's own object with no named container, a container a
    callee returned, a local container, an element name written any other
    way, or an embedded element, and do not drop the summary gate without a
    caller-side rule that forgets the published sites the call may release.
    `QCore.ContainerRootReach`;
    [report](../reports/w45-container-root-reach/README.md).

<a id="constraint-10g"></a>

10g. **A declared contract its own analysed body refutes serves no caller,
    in any unit.** Card W45-REFUTED-CONTRACT-WITHDRAWAL
    (`legacy_refuted_contract.rs`, `qcore_audit::refuted_contracts`; contract
    specification §C06.4). A body **refutes** its declaration when a
    counted-user clause of its own row (declared, or the implicit uncovered-exit
    obligation) is refuted, or its validated W4 summary contradicts a declared
    custody duty (constraint 8's mode arm). A refinement `SummaryConflict`
    (`rd_kafka_mem_free`) is not a refutation, and an undecided clause, a body
    that does not validate or no body at all leaves the declaration
    *unchecked*: still the trusted premise of constraints 4 and 8. The
    declaration stays in force for its own body, whose row keeps the
    refutation; every call site outside that body is rewritten to a marked
    undeclared name before any reader sees it and refuses with
    `contract-refuted-by-body`, which is `Unknown` (category `stale-evidence`),
    never `Pending`: a refutation is the tool's decision, and no named human
    act resolves it. In the defining unit the hold is the unit's own decision
    (after `checked_spec::settle`, then settled again over the held stream); a
    name the mode gate withdrew keeps constraint 8's key there. Across units,
    every audit route runs **two phases**: the first analyses every unit and
    folds the rows' `contract_refutations` into a table (a set, independent of
    unit order, `--jobs` and grouping, with a digest over the empty table too);
    when it is not empty, the second analyses every unit with the table in
    `ResourceContracts::refuted_by_body`. **Do not** withdraw on an unchecked
    declaration (making `resource_body_checked` the default would withdraw 22
    of the 24 declarations with an analysed corpus body, on no evidence, and
    every cross-unit application), install a derived summary in a refuted
    declaration's place, hold a call inside the refuted function's own body, or
    compose unit results analysed under different tables: a checkpointed run
    joins the table's digest to every group's identity (`.qo` schema 2) and
    runs its second phase over every group. Premises stated, not
    proved: the second pass reads the table only by membership, and a held call
    can only make a refutation undecided, never create one.
    `QCore.RefutedContractWithdrawal`;
    [report](../reports/w45-refuted-contract-withdrawal/README.md).

<a id="constraint-10h"></a>

10h. **An external audit subcommand is found only where it was declared, and
    what it says reaches a verdict only through the plugin firewall.** Card
    W45-AUDIT-SUBCOMMAND-PLUGINS (`qcore_audit::subcommand_plugins`, the SDK
    `crates/quirin-audit-plugin`; contract specification §C11.2). `quirin-audit
    NAME` runs `quirin-audit-NAME` only from `--plugin-path` and
    `QUIRIN_AUDIT_PLUGIN_PATH`: **do not** search `PATH`, and never run an
    external executable of a built-in name (`BUILTINS` is pinned against
    `main.rs` by a test; a new built-in joins it). The environment is cleared
    to a named allow-list, the plugin runs in its own process group with its
    output as working directory, and a quota breach kills the group; this is
    containment, not a sandbox (constraint 26). Every claim is classified by
    `PluginClaim` exactly as a checker plugin's answer: **do not** construct a
    `Safe` from a subcommand plugin's claim or certificate reference until a
    trusted-core checker exists for it (`W45-SUBCOMMAND-CERTIFICATE-ROUTE`);
    an uncertified `Safe` or `Unsafe` is `Pending`, and a spawn failure,
    timeout, quota breach, nonzero exit, malformed response, version mismatch,
    reported failure or unverified artifact makes the run `Unknown`, whose
    claims join nothing. A row is the join of the host's verdict, the run's
    and the claims, so it never lowers a host verdict, and a run with no row
    publishes no verdict rather than a vacuous `Safe`. The protocol version is
    checked before the shape; a new field is a new version. Premises stated,
    not proved: the host reads the plugin's answer only through
    `parse_response`, and a descendant that leaves the group is outside the
    kill. `QCore.SubcommandPlugins`;
    [report](../reports/w45-audit-subcommand-plugins/README.md).

<a id="constraint-10i"></a>

10i. **A held or withdrawn call is only ever minted by the tool, and a
    `Pending` unit passes every `Safe` gate.** Card W45-SR5-SOUNDNESS
    (`legacy_reserved_spelling.rs`; language specification §L21.1). Six
    rewrites mark a call site by prefixing its callee, and every reader
    classifies the call by the prefix: `qcore_review_pending__` and
    `qcore_pin_mismatch__` (both `Pending`), `qcore_refuted_by_body__`,
    `qcore_abi_withdrawn__`, `qcore_mode_withdrawn__` and
    `qcore_premise_unmet__`. A source spelling `qcore_pin_mismatch__zz(0);`
    published `Pending` with a human-resolvable reason no tool state decided
    (STABILITY-REEVAL-5). Before any rewrite, on every route that applies
    them, each source identifier spelled with a reserved prefix (the escape
    prefix `qcore_source_spelled__` included) is renamed with the escape
    prefix, consistently across the unit; an escaped callee refuses as an
    uncontracted call (`reserved-prefix-in-source`, `Unknown`). **Do not** add
    a rewrite prefix without adding it to `reserved_prefixes`, or classify a
    held reason from text a source can spell. A `Pending` unit claims that a
    named human act alone stands between it and `Safe`, so the
    unread-definition gate, the certificate target scope and the audit's
    inventory withholding apply to it as to `Safe`: a unit any of them refuses
    is `Unknown`, its function rows keeping their own held reasons. Premise,
    stated: the lexer yields the identifiers the escape reads (trust boundary
    28). `QCore.ReservedSpelling`, `QCore.Sr5Soundness`;
    [report](../reports/w45-sr5-soundness/README.md).
    **Amendment (W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL): a held call
    never masks another reason.** A held call's refusal replaces its
    statement's other refusal (the classifier checks the hold first) and has
    no effect, so a row was `Pending` although accepting the declaration would
    leave the statement's argument refusal or a violation standing
    (`k_close(0)` beside a pin mismatch, a double release held for review, a
    caller of a body a re-pin would leave refuted). A unit with a `Pending` row is analysed a second time under
    `ResourceContracts::with_human_holds_in_force` (every declaration awaiting
    review or withheld for a pin mismatch in force as written, nothing held),
    and every `Pending` row is joined with each reason above `Pending` its
    function carries there, recorded as an `Unsupported` standing reason
    (`reason standing beside a call held for a human act: …`), so the row is
    `Unknown`; a violation there is recorded the same way, never as a
    violation (`legacy_review_standing.rs`). Only a row whose second analysis
    is `Safe` stays `Pending`; a failed second analysis or a missing row keeps
    nothing `Pending`. Rows that are not `Pending` are never read, and a unit
    with none is analysed once. **Do not** repair this per statement (which
    actuals a position admits depends on the declaration in force), publish a
    verdict from the second analysis, or lower a row with it. The four
    `Unknown` holds (refuted, ABI, mode, literal premise with its
    `loop_owner__`/`thread_sync__` marks) absorb what they preempt and move
    attribution only. Premises stated, not proved: the second analysis is the
    held one with the holds lifted; a declaration another unit's body would
    refute once accepted is not seen (`W45-REVIEW-HOLD-CROSS-UNIT-REFUTATION`).
    `QCore.ReviewHoldStanding`;
    [report](../reports/w45-review-hold-preempts-statement-refusal/README.md).
    **Amendment (W47-REVIEW-HOLD-CONCRETE-REASON; user decision 2026-09-29):
    a held row names the act it awaits.** Every published function row that
    records a review-held call carries `review_holds`: each held
    declaration (name order), each of its markers (clause order), with the
    reason verbatim and a category read from the marker's structure — a
    trailing `rewrite_before_acceptance` (contract specification §C06.1)
    is `must-be-rewritten-before-acceptance`, a plain marker
    `acceptable-as-written-after-review` (`review_hold_reasons.rs`, hooked
    after `AuditReport::record` in `projection.rs`, so every route and a
    checkpoint composition carry it). A marker whose own reason says the text
    as written is false carries the word; its callers **stay `Pending`** under
    the standing rule above. **Do not** read the category from a reason's
    prose, let it enter a verdict or the second analysis, or publish a held
    row whose callee the environment cannot name (a missing declaration or
    marker fails the run). Additive: a row without a held call is
    byte-identical; the `.qo` schema is 4. Contract-pin-mismatch holds carry
    no marker and are not listed (`W47-PIN-MISMATCH-ROW-REASON`).
    `QCore.ReviewHoldStanding.ConcreteReason`;
    [report](../reports/w47-review-hold-concrete-reason/README.md).

<a id="constraint-10j"></a>

10j. **An external callee is declared only for the effect its own body has
    at every audited revision, in its owner's crate, and a command
    multiplexer only at the command read.** Card W46-EXTERNAL-CALLEE-CONTRACTS
    (`providers/*/contracts/`, `contracts.lock`; contract specification C04.5,
    C06.2). An external callee (no definition in any analysed unit) is ranked
    by the functions it alone would unblock, then read at both `66910c10` and
    `046accf6`: every vendored copy and configuration arm must agree on the
    custody effect (bodies may differ), and the declaration goes to the
    owner's crate, pinned there; libc only to `quirin-provider-libc`,
    flexible. A callee whose effect depends on a scalar argument is declared
    for one value only, with `resource_literal_argument` (`fcntl` at
    `F_SETFD`, as `prctl` at `PR_SET_NAME`); every other value stays
    undeclared. A `static inline` header body the units carry is not an
    analysed function, so its declaration is an unchecked premise pinned to
    the header text (rule 10g checks only an analysed body). **Do not** declare
    an effect v1 cannot state exactly to obtain a `Safe`: an in-place
    reallocation through a `T **` (`cfl_sds_cat_safe`), a refcounted keep or
    release, a retained pointer, a callback, an in-place lifecycle, a
    no-return call (**amended by CG21-CALL-EXITS-AND-BOUNDARIES, rule 11o**:
    a terminating exit is declared with `resource_no_return()`; a non-local
    exit stays undeclarable) or a handle kind stays undeclared with its reason, and a
    system library outside the audited source has no pinnable identity.
    [Report](../reports/w46-external-callee-contracts/README.md).
    W47-CALLEE-CLOSURE-R2 adds that a pointer result is declared with the mode
    that keeps what the body makes it depend on: an allocation inside storage
    a borrowed argument owns and releases (`cfl_arena_calloc`) is
    `resource_return_interior` of that argument, never
    `resource_return_none()`, which forgets the dependence and was measured
    admitting a use after the base's release as `Safe`; the interior extent is
    then read as the storage the base owns, a stated premise
    (`W47-ARENA-INTERIOR-READING`). Rank on the post-access-path corpus before
    contracting: P3 exposed callees no earlier ranking saw
    ([report](../reports/w47-callee-closure-r2/README.md)).

<a id="constraint-10k"></a>

10k. **A typed access path is read in a condition operand, a return or a
    call actual only after every older route of that position refused, and
    only where C evaluates it.** Card W47-ACCESS-PATH-POSITIONS
    (`legacy_typed_positions.rs`, `typed_access_positions.rs`) applies the
    W46-ACCESS-PATH-STATEMENTS reader, with every refusal it has (unions and
    typedef-hidden unions, bitfields — both read by rule 10w since CG13 —
    casts within a path, volatile/atomic,
    pointer arithmetic, a tracked mixed embedded/indexed path, pointer
    stores, calls, missing or ambiguous AST facts), to a sub-expression
    matched by its line and exact tokens to one Clang node. (1) A call-free
    condition operand lowers to the typed read with no destination and no
    frame, checked at the operand's node before either edge; operands are
    wired by rule 5h's desugaring, so an operand of `&&`, `||` or `?:` that C
    skips on a path contributes no obligation there, and `p && p->x` reaches
    `p->x` only on the edge the existing null branch refined. A call-free
    condition is desugared only when an operand needed this reader or an
    existing checked read retains its plan under rule 5h; one
    `NAME = PATH` operand (a declared non-formal local) is P2's statement,
    then the existing null or neutral decision on `NAME`, whose identity
    refusal constraint 10b keeps. (2) A `return` of one scalar value, or of
    one pointer read whose every root the custody closure proves never bound
    **and** `may_bind` does not name (rule 5b's premise), evaluates the read
    with a `NoOp` exit: nothing is escaped or handed back. An array member
    returned by decay is an interior address and refuses (rule 5j); `&&`,
    `||`, `?:`, assignments and calls in a return refuse. (3) A call actual
    the lexical recogniser refused joins W44-INDIRECT-CALL-ARGUMENT-ACCESS's
    probe only at a `Borrowed` or declared data-tail position of a callee
    whose contract or validated summary is in force; a pointer-valued actual
    needs never-bound roots and keeps `ArgumentReads`; its chains keep their
    per-link obligations. A custody-taking position, an uncontracted callee
    and a position beyond the declaration keep the refusal. (4) The
    owner-aware enum arm counts `enum TAG` for a declared tag, or a typedef
    whose declaration is directly an `EnumType`, as a scalar; every other
    typedef spelling keeps the positive-builtin refusal. **Do not** consult
    this reader before an older route of the position (hard invariant 3; the
    `QCORE_TYPED_PATH_PARITY` stream records both, with a `position` key, and
    claims no parity), relax the pointer-return premise to `never_bound`
    alone (it would admit a return through a written or call-argument root,
    which rule 5b's pins refuse), or admit a connective inside one read.
    Premises stated, not proved: faithful Clang extraction and node matching
    (trust boundary 28), and that a never-bound root reaches no tracked
    place (P2's premise). `QCore.TypedAccessPathPositions`;
    [report](../reports/w47-access-path-positions/README.md).
    **Amendment (W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT):** the pointer
    return premise may omit one call-actual occurrence from `may_bind` only
    when the same unit contains the exact
    `int rd_list_cnt(const rd_list_t *rl) { return rl->rl_cnt; }` body and
    the call resolves directly to that `FunctionDecl` with its sole actual
    exactly `&ROOT->member[.member]*`. The body establishes a read-only
    frame for that actual; a `Borrowed` custody mode alone does not. The
    lexical and AST readers each replay `may_bind` with only those exact
    occurrences omitted. The root must remain a unique formal, never bound
    by the custody closure, not protected at entry, and unnamed by every
    other binding event. An absent or altered body, indirect call, changed
    actual, other write/address/call, or unanswered AST refuses. The call's
    own contract or validated summary remains an independent obligation.
    `QCore.ReadOnlyActualFrame` models the event exclusion and return
    decision; source extraction and Rust–Lean correspondence remain the
    stated premises. [Report](../reports/w48-pointer-return-readonly/README.md).
    Amended by [rule 11d](#constraint-11d): (3)'s pointer read on roots that
    are eligible chain roots is left to the flow, which reads each only as
    untracked (a tracked root refuses); an address actual and (2)'s return
    premise are unchanged.

<a id="constraint-10l"></a>

10l. **A typed position's root premises are read from the Clang AST only
    where the lexical declaration reader answers nothing, and never less
    conservatively than the lexical closure.** Card
    W47-TYPED-ROOT-DECLARATION-READER (`typed_root_declarations.rs`, hooks in
    `legacy_typed_roots.rs`). Rule 10k's never-bound, `may_bind` and
    declared-local premises are the lexical answers wherever
    `FunctionTypes` read the function, unchanged. Where it did not, a name is
    answered only when the function's `FunctionDecl` declares it exactly once,
    as a formal or an automatic local, and no reference names another object
    of that spelling; it is never bound when neither the AST closure (the
    object root of a `&` operand, an increment, a compound assignment or a
    sub-object store; every variable of an actual at a position its contract
    does not declare `Borrowed`, except an `->` base; an assignment or
    initialiser whose value holds a call, an address, a statement expression,
    a bound name or an undeclared one, to its fixed point) nor the lexical
    closure, computed there too, binds it. A returned pointer's root is also
    named by neither `may_bind` (rule 5b). A declared local is one automatic,
    non-formal declaration of a type the typed reader reads; every lexical
    `yes` stands. An unprototyped (K&R) definition, inline assembly, a block,
    a block-scope `typedef`, a missing body or AST, and an unsettled closure
    answer nothing. **Do not** serve the AST answer where the lexical reader
    answered, drop the lexical closure from the join, answer a shadowed,
    `static` or file-scope name, or relax the pointer-return premise to
    never-bound alone. The twelve forecast misses that motivated this card
    were measured to refuse **with every premise granted** (a `T *const`
    value, a rule 5j array decay, a `void *` bit-cast), so it moves none of
    them (`W47-TYPED-CONST-POINTER-VALUE`). Parity runs record both readers
    per function (`position: "root-declarations"`). Premises stated, not
    proved: faithful Clang extraction (trust boundary 28), and that the Rust
    loop returns a seeded, settled set. `QCore.TypedRootDeclarations`;
    [report](../reports/w47-typed-root-declaration-reader/README.md).
    Rule 10k's checked read-only call actual is the sole narrower
    pointer-return answer: the ordinary `may_bind` answers remain unchanged
    for every other position and the AST closure is recomputed with only
    that framed event omitted, never simply overridden by a lexical answer.

<a id="constraint-10m"></a>

10m. **A pointer value is read by its unqualified type, and a `void *`
    bit-cast into a declared object pointer is the same pointer.** Card
    W47-TYPED-CONST-POINTER-VALUE (`typed_const_pointer_value.rs`, hooks in
    `typed_access_path.rs` and `typed_access_positions.rs`). (1) A pointer
    member read through a `const` pointee, or an element of a `T *const[]`
    member, is a `T *const` lvalue; the value read from it is the unqualified
    `T *` (C11 6.3.2.1p2, and Clang's own `LValueToRValue` type). The typed
    reader's value check admits a value whose canonical type (Clang's
    `desugaredQualType`, else `qualType`) has exactly one top-level
    qualifier, `const`, after the last declarator `*`, over a type the
    unchanged positive pointer test admits, in every rule 10k position and in
    P2's statements. (2) At the top of a pointer-destination statement
    (`NAME = VALUE`, `T *NAME = VALUE`) an `ImplicitCastExpr <BitCast>` from
    exactly `void *` to an object pointer, the destination's declared type an
    object pointer too, reads the typed path under it as the uncast statement
    does: P2's borrowed, target-free alias with the inner read's obligations.
    Measured as separate steps. Every per-hop obligation, rule 5b's
    never-bound / not-`may_bind` return premise and rule 10b's identity
    refusal are unchanged. **Do not** erase `volatile`, `restrict` or
    `_Atomic`, or any list but a lone `const`; do not read a qualifier by
    trimming spellings, or erase one anywhere but the value itself (a middle
    hop is already Clang's unqualified conversion); do not admit an explicit
    cast, any other conversion, a cast to a function or `void` pointer, a cast
    from another object pointer, or a cast inside a path. A `T *const`
    destination or plain name is not this rule's (it reads values, not
    declarations). Premises stated, not proved: faithful Clang extraction of
    the printed type and cast kinds (trust boundary 28).
    `QCore.TypedConstPointerValue`;
    [report](../reports/w47-typed-const-pointer-value/README.md).
    Amended by rule 10o: where both readers answer, a spelling the AST closure
    binds is bound (the join only refuses more).

<a id="constraint-10n"></a>

10n. **A P3 residual shape is read only where C's evaluation order and the
    callee's custody premise admit it, after every older route and rule 10k's
    reader refused.** Card W47-ACCESS-PATH-POSITION-RESIDUALS
    (`typed_position_shapes.rs`, `legacy_typed_residuals.rs`, thin hooks in
    `typed_access_path.rs`, `typed_access_positions.rs`,
    `legacy_typed_positions.rs` and `legacy_c.rs`). (1) **An address**
    `&PATH` is read with every per-hop obligation of `PATH`'s read and no read
    of the place itself, so only the place's value type is waived: as an
    operand of a pointer comparison (never as a truthiness operand, whose
    answer C fixes), and handed to a `Borrowed` or declared data-tail actual
    only when the place is exactly a member (`& BASE -> FIELD` or `.`, no
    element, no parenthesized place) whose Clang type is not a pointer,
    array or function pointer and not `volatile`/`_Atomic` (O04-B09B10's
    disjointness premise: a callee writing through the address reaches no
    member the projection links), with every root never bound (rule 10k's
    pointer-actual premise). `(&X)->f` is read as `X.f` (its mixed-path
    premise applies). An address is never returned (rule 5j). (2) **An
    ambiguous operand** (several Clang nodes with its line and tokens) is
    matched to the one candidate inside a controlling expression (`if`,
    `while`, `do`, `switch`, a `for` condition) or a `return` value, else to
    the one of those in a Boolean leaf position (its nearest non-wrapper
    parent is the slot, `&&`, `||`, `!`, `?:` or `__builtin_expect`); a unique
    match is unchanged and several survivors refuse. (3) **An inline
    anonymous enumeration member** is a scalar on structural evidence: its
    `FieldDecl` follows the unnamed `EnumDecl` in the same complete struct and
    its type spells that anonymous enumeration; a union member, a pointer to
    one, a bitfield or a spelling alone keeps the refusal. (4) **A scalar
    connective return** (`&&`, `||` or a scalar `?:` at the top of the Clang
    node, no call, no assignment or increment, no tracked descriptor) is
    desugared exactly as rule 5h desugars a condition, both exits joining one
    `return` with a `NoOp` exit. **Do not** admit a pointer-valued `?:`
    return, a call or an assignment in a return, an array-decay return, a
    `?:` actual (its arms need a branching call lowering) or a nested call
    actual under this rule; each keeps its refusal here (rule 10z lowers the
    pointer `?:` return and the `?:` actual by branching). Premises stated, not
    proved: faithful Clang extraction and node matching (trust boundary 28).
    Parity runs record every refused position's shape
    (`position: "residual-shape"`, its unread constructs and whether every
    root premise granted would admit it) and every connective return
    (`position: "return-connective"`). `QCore.TypedAccessPathPositionResiduals`;
    [report](../reports/w47-access-path-position-residuals/README.md).
    Amended by [rule 14c](#constraint-14c): (1)'s callee shape also admits a
    scalar element `&PATH[I]`; and by [rule 14d](#constraint-14d): an
    interior offset `P ± n` at an actual is read under (1)'s premise.

<a id="constraint-10o"></a>

10o. **The never-bound premise reads every assignment of the body, and a
    typed position serves the join of the lexical and AST closures.** Card
    W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (`lexical_nested_assignment.rs`,
    hooks in `borrowed_actual.rs`, `legacy_typed_roots.rs`). The lexical
    custody closure (`untracked_bases`, every consumer of `never_bound`: C02-P,
    O04-B08 values, rules 5j and 5l, W44 actuals, P2/P3) read one assignment
    per `;{}` run, its first top-level `=`. It now also reads, beside that
    unchanged reading, every simple `=` wherever it stands — nested in a
    condition or loop header, a `for` clause, a comma or `?:` operand, a later
    declarator, a parenthesised (macro-expanded) destination `(p) = …`, a
    scalar or array declarator's brace initialiser — with the rest of its
    group as the value, and seeds `&(p)`, `(p)++`, `++(p)`, `(p) op= …`; the
    closure iterates to its fixed point (at least `assignments + 1` rounds)
    and refuses the function if it does not settle. A statement-expression
    value is still read as its opening `(` (rule 5l's `container_of`). The
    same reader feeds `may_bind` (parenthesised writes), O04-B08's premises 3
    and 5 (a nested, later-declarator or parenthesised alias or second
    binding of a store base) and the protected-formal alias closure (the
    aliases its run-first reading skipped). At every P2 statement and P3
    position of a function the lexical reader reads, a root is never bound
    only when the extended lexical closure says so **and** the rule-10l AST
    closure does not bind its spelling (file-scope objects included) wherever
    the AST reader answers the function; a pointer root also needs neither
    `may_bind` (lexical, or the AST's spelling). Where the AST reader refuses
    the function the lexical answer stands; where the lexical reader answers
    nothing, rule 10l is unchanged. **Do not** drop the old statement reading
    (the superset reading is monotone by construction), serve a lexical
    answer the AST closure contradicts at a typed position, or read a
    statement expression's value past its `(` in the lexical closure without
    a measured re-pin of rule 5l. The join withdraws Safe functions whose
    typed rows rest on classes the AST closure reads more conservatively than
    rules 5l and 5i (a `container_of` element, the admitted store's own
    sub-object root, a static array initialiser in `may_bind`);
    `W47-JOIN-AST-CONSERVATIVE-CLASSES` tracks restoring them. Premises
    stated, not proved: the token reader's extraction of assignments and
    faithful Clang extraction (trust boundary 28), and that the Rust loops
    return seeded, settled sets. `QCore.LexicalClosureNestedAssignment`;
    [report](../reports/w47-lexical-closure-nested-assignment/README.md).
    Rule 10k's checked read-only actual is an explicit exception to the
    pointer-root `may_bind` over-approximation; the joined never-bound
    closure and every other lexical/AST binding remain required.

<a id="constraint-10p"></a>

10p. **A loop-owned collector operation is admitted only on a derived path
    whose thread is its loop's owner, and every route that derives no thread
    refuses it.** Card W47-LOOP-OWNER-CONTRACTS (`qcore_core::loop_owner`,
    `thread_entry_census.rs` read by `quirin_source_analysis::loop_owner_census`,
    the hook in `legacy_literal_premise.rs`;
    contract specification C04.9). `requires resource_loop_owner(p,
    read|mutate);` on `flb_input_collector_{running,pause,resume,delete}`
    (`fluent_bit/loop_owner.qv`, pinned at `66910c10`, `046accf6` and the
    `90978ac6` export) is discharged only where every derived context of the
    caller runs on `owner(L)` for the mode it fixes (the single derived
    thread of the engine loop's dispatcher `flb_input_collector_fd` for a
    non-threaded instance, of the worker loop's `input_collector_fd` for a
    threaded one) and no path to the caller is unresolved. (1) The per-unit
    resource route derives no thread, so it rewrites every call of such a
    callee under the literal premise's reserved prefix with a loop-owner mark
    and refuses it (`outside its declared loop-owner obligation`): the clause
    only refuses there. (2) The thread-entry census reads one program's files
    lexically and derives contexts through direct calls and hops A (the test
    table), S (spawn wrappers: a new thread named by its entry; an
    unresolved spawner roots it in every mode), V/V\* (field-keyed callback
    registration and dispatch, the revised test's copied vtable included), R
    (collector registration on `cb_collect`), L (the two loop dispatchers fix
    the mode) and F (a call in an `if (flb_input_is_threaded(x))` arm
    requires that mode). An address taken anywhere else, a name in a macro
    body, a hop field read as a value and an unstructured conditional group
    make the reached functions unresolved; a parameter of the same name
    shadows a function. One resolved non-owner context refuses the site; an
    unresolved or absent context withholds the discharge; nothing here is
    `Unsafe`. C2 (an enqueue discharges only "request available") and C3 (a
    paired-collector state only at the callback boundary with both results
    succeeded, a resume's running state only under active ingestion) are the
    kernel's; the public pause/resume, registration and `mk_event_*` stay
    undeclared (rule 10j; since card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION
    the registrations and `mk_event_*` are declared as retentions, removals
    and id-forwarded registrations, rules 12t and 12n, with no loop-owner
    obligation). **Do not** apply a loop-owner declaration on a
    route that derives no thread, treat an unresolved hop or a missing
    caller as discharged, name a spawned thread after its spawner, or relax a
    refusal because another context discharges. Premises stated, not proved
    (trust boundary 28): the lexical reading of the files and the hop
    tables; that nothing outside the read files names a read function; that
    a function stored in a field is called only through that field's name;
    P-INSTANCE (along a derived path the instance a collector operation names
    is the one whose mode the path fixed, `ctx->ins`); and that the threaded
    flag is immutable after init. `QCore.LoopOwner` derives, for this route,
    the single-dispatcher premise `QCore.EventDriven` takes as supplied;
    [report](../reports/w47-loop-owner-contracts/README.md).
    **Amendment (W47-LOOP-OWNER-COROUTINE-HOP): hop C and the configured
    reading.** A coroutine runs on the thread that resumes it. The census
    derives the input coroutine's resumers: `flb_input_coro_collect` names
    `input_pre_cb_collect` at `co_create`, and each function that constructs
    it as `v = flb_input_coro_collect(…)` and resumes `v` through
    `flb_input_coro_resume` runs the entry (the two loop dispatchers, each in
    its loop's mode, as hop L); the entry's calls up to its single switch back
    to the resumer run in the constructor. A constructor call not so resumed, a
    wrapper call on any other handle, or a hand-off that is not one switch
    outside a loop leaves the entry unresolved. A function the entry
    dispatches after the hand-off is a coroutine body: one that may reach a
    non-terminal yield (a call of a function that switches back to its
    resumer, or of a macro that does, outside `flb_input_return_do`) is
    unresolved, its resumer after the yield not derived; a collector registrant
    whose plugin directory's descriptors derive `.flags` without
    `FLB_INPUT_CORO` is not dispatched from the entry at all, and one
    registered outside `plugins/` keeps the dispatch. An include guard's
    closing `#endif` after a file's last token closes its group. The census may
    read under a build configuration (each read `.c` file's compile-command
    `-D` set, the intersection for other files, `FLB_`-prefixed macros owned):
    an arm is dropped only when its condition (`defined`, `!`, `&&`, `||`) is
    decided false or an earlier arm is decided true, and an owned macro is
    undefined only when no command defines it and no read file or generated
    header `#define`s it. **Do not** take a generic `flb_coro_resume` of a
    handle not tied to its constructor as a derived resumer, treat a body that
    may yield as running on its first resumer, or guess across an undecided
    `#if`. Premises added (trust boundary 28): P-CORO-HANDLE (an input
    coroutine's handle reaches a generic resume site only through its own
    `flb_coro_get()`, after it yields, and `ins->input_coro_list` is never
    resumed from); P-PLUGIN-DIR (a plugin directory registers collectors only
    on instances of the plugins it declares, and descriptor flags do not
    change at run time); and that the test file is configured as the library's
    common set. `QCore.LoopOwner` proves the hop (a coroutine resumed only on
    `owner L` runs on `owner L`; an undetermined resumer never discharges), the
    gating and the configured reading;
    [report](../reports/w47-loop-owner-coroutine-hop/README.md).
    **Amendment (W47-LOOP-OWNER-TYPED-DISPATCH): type-keyed V and a plugin
    copy's instance flag.** The census may read Clang's typed AST of each
    recorded compile command (`typed_dispatch_census.rs`): a field dispatch
    then reaches only registrants stored in the record type Clang resolved for
    its field (typedefs resolved; an anonymous record by the typedef naming it),
    a collector registration is keyed by the record its registration function
    stores the callback parameter in, and a site with no typed fact keeps the
    field-keyed reading. A dispatch through a union field, an anonymous record
    no typedef names, or an explicit cast on its object path or callee reaches
    nothing and makes every registrant of the field unresolved; a store of a
    converted function makes it unresolved. A collector registrant registered
    outside `plugins/` is derived to run no coroutine only when it is
    registered on the first parameter of a callback stored only in a local
    plugin copy whose every use is modelled (a whole copy, field stores,
    numeric `.flags` writes, one `X->p = &copy`, no escape of its holder) and
    whose flags derive none, **and** the instance `X` is created once in that
    function by `flb_input_new` with a literal plugin name whose one
    descriptor derives none, `runs_in_coroutine` being written by the
    constructor alone. **Do not** key a site the typed reading does not name,
    attribute an unmatched dispatch to any record, or take a copy's flags for
    the instance's (the route switch reads the flag set at creation).
    Premises added (trust boundary 28): the recorded build's commands are the
    program's; C's effective-type rule; the test is typed with the library's
    arguments and a configured test header; and P-CALLBACK-INSTANCE (a
    callback stored in an input plugin object's field runs with an instance
    whose `p` is that object). `QCore.LoopOwner` proves type-keyed dispatch a
    sub-relation of field-keyed dispatch that removes only cross-type edges or
    refuses, and the route dropped only when both flags derive none;
    [report](../reports/w47-loop-owner-typed-dispatch/README.md).
    **Amendment (W47-LOOP-OWNER-INSTANCE-FLOW): the instance a plugin copy
    is installed on.** Step B may tie an instance read as the head of a
    context's input list (`X = mk_list_entry(H, struct flb_input_instance,
    _head)`, `H` a context's `config->inputs.next`) to its creation
    (`loop_owner_instance.rs`, child of `thread_entry_census.rs`): the context
    is a local returned by a function of the test's file that assigns it once
    from `flb_create()`; on the test's path to the read (the test up to it,
    that function, and `flb_create` read in place; a block that ends in
    `return NULL;` pruned only where the test returns on a null context or
    the context constructor itself returns null) exactly one call, outside
    every loop, reaches a hazard over the census graph (all dispatch edges,
    spawns and coroutine entries, the coroutine route not pruned), and it is
    a wrapper that passes one unassigned parameter as the plugin name to one
    `flb_input_new` call, called with a literal the typed reading records;
    each function read in place must itself be free of hazards. A
    hazard is the constructor, a use of `struct flb_config::inputs` or
    `::in_plugins` other than a read or the listed initialisations, a write
    of a context's `config`, an untyped function naming those members, a
    function of an unstructured file whose text names one of them or a
    function reaching one, or a call to a name only such a file may define;
    and when the path's closure makes an unmodelled indirect call or has
    a function the build's typed reading could not read, no escaped function
    (an address taken, a macro name, a registrant of a function-pointer field
    read as a value through a matching record) may reach a hazard. Every
    descriptor whose name matches the literal ignoring ASCII case must derive
    no coroutine (`check_protocol`); the literal-constructor path now matches
    the same way. The copy's holder may escape only as `X->data = &holder`
    into the installed instance, when the holder's `struct` is named in the
    test's file alone, the copy member is written or its address taken
    nowhere else there, and `struct flb_input_instance::p` is written only by
    the constructor and that installation. P-CALLBACK-INSTANCE is now derived,
    or the flag is underived: every dispatch of the copy's callback field
    that may reach it passes `X` with `X->p->F(X, …)` or `Q->F(X, …)` after
    `Q = X->p`, or is the callback passing its own first parameter; and
    `struct flb_input_collector::instance` is written only by
    `collector_create`, from a parameter each registration passes its first
    parameter to. The typed reading records the listed members' accesses,
    function-pointer members read as values, the functions it read and each
    unmodelled indirect call. **Do not** take a list head for an instance
    without the single-creation derivation, count a creation inside a loop
    once, read a key-dependent call as not taking a branch, or treat an
    unstructured or unread function as hazard-free. Premises added (trust
    boundary 28): P-HEAD-INSTANCE (the list the test reads is not empty: its
    context's and its one input's creations succeeded), P-FRESH-CONTEXT,
    P-REGISTERED-DESCRIPTORS (the build's descriptors, one registration each,
    before any registration the path reaches), P-HOLDER-OPAQUE (code that does
    not name the holder's type reads or writes no holder byte through
    `ins->data`), P-INSTANCE-FIELDS-BY-MEMBER (an instance's `p` and flag and
    a collector's instance are written only through member accesses) and
    acutest's `TEST_CHECK` as its argument's truth; the stated ones are
    recorded as provider contracts held for review
    (`fluent_bit/input_instance.qv`: `flb_create`, `flb_input`,
    `flb_input_new`, `flb_service_set`, pinned at `66910c10`, `046accf6`
    and the `90978ac6` export). `QCore.LoopOwner` proves a derived head the
    one creation on every path, a second creation or an unmodelled mutation
    keeping the route, and an escape that writes neither the flag nor the
    instance keeping the derivation;
    [report](../reports/w47-loop-owner-instance-flow/README.md).
    **Amendment (W47-SERVICE-PROPERTY-KEYS): the keys a property setter
    carries.** In Step B's list-head region a call of a listed variadic
    setter (`flb_service_set`, `flb_input_set`, `flb_output_set`,
    `flb_filter_set`; `loop_owner_property_keys.rs`) reaches only its keys'
    effects when every part is derived: the typed reading records one
    argument list at the call's line, NULL-terminated (a `NullToPointer`
    zero), an even number of arguments before it, no other null, every key a
    string literal of printable ASCII without a backslash or a quote; the
    setter's body reads its list with exactly `while ((KEY = va_arg(VA, char *)))` and
    `VALUE = va_arg(VA, char *);` after `va_start` on its last named
    parameter, `KEY` and `VALUE` assigned nowhere else, `KEY` one whole
    argument of exactly one call (the property function), no jump; the
    property function walks one table (`KV = TABLE[0].KF`, `while (KV !=
    NULL) { if (CMP(KV, K, LEN) == 0) {…} KV = TABLE[++I].KF; }`, `LEN =
    strnlen(K, N)`), whose elements the typed reading (keys after macro
    expansion, type enumerators) and the configured lexical reading
    (`offsetof` fields) give alike, element for element; `CMP`'s body is
    exactly the read `strnlen`/`strncasecmp` comparison, so a key matches an
    entry when they agree ignoring ASCII case over the whole key; the matched
    block holds one `if (!strncasecmp(KV, C, M)) {…} else if … else {…}`
    chain, each constant typed; an entry takes the first arm its key
    satisfies, else the final `else`, whose `switch` stores
    `*(T *)((char *)config + TABLE[I].OFF)` are admitted only into the
    entry's own `struct flb_config` field, not a listed list member, declared
    once with exactly the store's type (`unsigned`/`signed` ignored), no such
    arithmetic standing elsewhere. The call then reaches what the setter's
    body and the property function's body outside the arms no key selects
    name (a call, a value, or an invoked macro's replacement list, through
    nested macros), the two functions read in place; any other list, shape,
    unknown key, unclassified entry, undecided conditional group in a read
    body or field dispatch keeps the whole call. The derived table and each
    key's reach are printed per call (`typed.property_keys`). **Do not** read
    a key the typed reading did not record as one literal, read a value as a
    key, drop an arm a matched entry may take, treat an unmatched key as
    matching nothing, or admit a scalar store whose field or type is not
    derived. Premises added (trust boundary 28): the typed reading's argument
    and initializer extraction; P-KEY-FOLD (the C library's `strncasecmp`
    folds such printable-ASCII keys no more permissively than ASCII case
    folding); `strnlen` and `va_arg` as C specifies them. The observation is
    recorded as the held `flb_config_set_property` declaration and
    `flb_service_set`'s reworded held tail (`fluent_bit/input_instance.qv`,
    pinned at `66910c10`, `046accf6` and the export). `QCore.LoopOwner` proves
    a read key list NULL-terminated and paired, an unreadable list or an
    unmatched key keeping every branch, the reading covering every run of a
    prefix of the keys, a matched hazard staying read, case variants
    matching the same entries, and a read reach holding a hazard keeping the
    route; [report](../reports/w47-service-property-keys/README.md).
    **Amendment (W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS): reserved macros
    decided from the recorded compiler.** The configured reading may decide a
    macro reserved to the implementation (C11 7.1.3: `__X`, `_` and an
    uppercase letter) that a read file's conditional directive names
    (`predefined_macros`, `loop_owner_predefined_macros.rs`, child of
    `thread_entry_census.rs`; the facts written by the
    `predefined_macro_facts` example). Each recorded compile command is run
    with its own compiler and flags: a probe `#ifdef` per candidate at the
    start of a unit (built-ins, then the command line's `-D`/`-U` in order),
    the `-dM` listing (every listed candidate must probe defined), and `-E
    -dD` of the command's file (every `#define`/`#undef` outside the
    built-in and command-line segments, system headers included, and every
    file it reads). The compiler's identity (path, resolved path, SHA-256,
    `--version`) is recorded, and the facts decide nothing unless its
    `--version` equals the build record's, the database's SHA-256 and each
    command's file and compiler equal the census's database, and no
    compiler is unavailable. A name is then **defined** in a file when every
    command of that file starts with it and no directive of those units
    `#undef`s it, **undefined** when none starts with it and none `#define`s
    it; a file without a command is decided over every command, only when
    some unit read it and no derivation failed; a failed derivation leaves
    its own file undecided. A read file's `#define`/`#undef` and a generated
    header's `#define` still win, the `FLB_` rules are unchanged, and a
    value comparison (`__GNUC__ >= 4`) is never evaluated. The census reports
    each decision per file (`predefined_macros.decisions`). **Do not** decide
    a predefined macro from an assumed target, a compiler other than the
    recorded one, or a derivation that failed, read a comparison of a
    predefined value, or decide a header from its includers alone. Premises
    added (trust boundary 28): P-RECORDED-COMPILER (the compiler at the
    recorded path with the recorded version behaves as it did in the build)
    and P-DD-COMPLETE (Clang's `-dD` prints every directive it processes, and
    its line markers name every file a unit reads).
    At the export the fixed test's 3 sites are **discharged at this path
    under these derived premises** on `(flb_lib_worker, NonThreaded)` and
    `(input_thread, Threaded)` (P-CORO-HANDLE, P-PLUGIN-DIR, P-HEAD-INSTANCE,
    P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS, P-HOLDER-OPAQUE,
    P-INSTANCE-FIELDS-BY-MEMBER, P-KEY-FOLD, P-CALLBACK-INSTANCE derived,
    typed-reading faithfulness, the region's unmodelled macro calls of
    `W47-LOOP-OWNER-REGION-MACRO-CALLS`, and the two above), never a
    verified fix; the old test's stay not discharged. `QCore.LoopOwner`
    proves a decided macro agreeing with its definedness at every point of
    every unit, the three-valued condition reading agreeing with the
    preprocessor, a comparison keeping its arm, an absent derivation keeping
    every arm, `-U` undefining, a file's directive winning and the old test's
    path refused whatever the arms;
    [report](../reports/w47-loop-owner-predefined-macro-arms/README.md).
    **Amendment (W47-PREMISES-CORO-PLUGIN): P-CORO-HANDLE, P-PLUGIN-DIR and
    the immutable threaded flag derived.** With a typed reading the census
    derives the three premises from the program (`loop_owner_coro_plugin.rs`,
    child of `thread_entry_census.rs`; typed member accesses of
    `struct flb_input_coro::coro`, `struct flb_libco_in_params::{coro,coll}`,
    both `input_coro_list`s, `struct flb_input_plugin::flags` and
    `struct flb_input_instance::is_threaded`, and the setter's key literals)
    and withholds what rests on one it does not derive. P-CORO-HANDLE:
    every access of the handle members stands in the coroutine's closed set
    (constructor, hand-off, entry, wrapper, `flb_coro_resume`, destruction);
    inside it a handle is only dereferenced, tested, stored into a handle
    member or local, or passed to a listed parameter; the creation and the
    hand-off are called only inside it; `input_coro_list` is only initialised
    and appended to; an unmatched-record access, or a function a unit Clang
    could not read may compile naming the member, fails it; not derived ⇒
    every coroutine entry unresolved and no route dropped. P-PLUGIN-DIR: a
    descriptor's `flags` is written only into a fresh allocation before it
    is named anywhere else, `runs_in_coroutine` only by the constructor, and
    every coroutine construction stands in `if (C->instance->runs_in_coroutine)
    {…}` on its own collector argument; not derived ⇒ no directory or Step B
    flag is used and no route dropped; and per registrant registered only in
    `plugins/D`, every registration's instance is an unassigned parameter
    chained through direct calls to a callback stored only in `D`'s
    descriptor initializers (P-CALLBACK-INSTANCE derived for those fields, an
    instance's `p` written only by the constructor or by installing a copy of
    its own plugin into which no plugin function is stored) or to a collector
    callback of `D` so registered whose dispatches pass `C->instance`; not
    derived ⇒ that registrant keeps the coroutine route. The immutable
    threaded flag: every `is_threaded` write is the property setter's in its
    `threaded`-keyed arm (its key parameter unassigned), or into an instance
    its writer created itself from an allocator or the constructor; not
    derived ⇒ every mode-fixing edge's callee is unresolved. A reading
    without typed facts derives none and keeps the earlier statements (no
    measured reading is untyped). The census prints each derivation
    (`typed.premises`). **Do not** drop a coroutine route, use a directory's
    flag or fix a mode on an underived premise, count a setter call with an
    unread key as unable to write the flag, or treat a header a failed unit
    may compile as outside the build. Premises narrowed (trust boundary 28):
    P-CORO-HANDLE to **P-CURRENT-HANDLE** (no thread resumes a coroutine
    while it runs through the running coroutine's handle of itself,
    `flb_coro_get()`/`co_active()`; 25 sites listed); the threaded flag to
    **P-THREADED-SET-BEFORE-START** (the listed creation writes and setter
    calls whose key is not one other literal run before the instance they
    write starts; 6 sites); P-PLUGIN-DIR to P-REGISTERED-DESCRIPTORS and
    **P-DESCRIPTOR-FLAGS-BY-MEMBER** (a descriptor's flags are written only
    through member accesses). `QCore.LoopOwner.CoroPlugin` proves an escaping
    handle never discharged and a census discharge covering every possible
    resumer, a run-time flags write or a registration on another plugin's
    instance keeping the route, no write after start fixing the flag, and
    the old test's path refused whatever the premises;
    [report](../reports/w47-premises-coro-plugin/README.md).
    **Amendment (W47-PREMISES-INSTANCE): the list-head instance's premises
    derived.** The premised census (`--derive-premises`,
    `loop_owner_premises_instance.rs`, child of `thread_entry_census.rs`)
    derives what the list-head derivation stated, or its flag is underived:
    **P-FRESH-CONTEXT** (the context constructor stores the one config
    initializer result in the context it returns; the initializer returns its
    one allocation, from an allocator returning only a null pointer or
    `calloc(…)`, after `mk_list_init` of the input and registered-plugin lists
    at its top level, `mk_list_init` read as leaving an empty list);
    **P-REGISTERED-DESCRIPTORS** (the build's generated registration, read from
    the generated root and resolved to by every compile command of the
    initializer's file through its `-I` order, is exactly allocate /
    `memcpy(V, &OBJ, …)` / `mk_list_add(&V->_head, &CFG->in_plugins)` blocks,
    called once at the initializer's top level after the list's
    initialisation; every registered descriptor's name is read and the
    creation's literal matches one); and the checkable part of
    **P-INSTANCE-FIELDS-BY-MEMBER** (the typed reading records every whole
    assignment, copy-initialised or by-value object, initializer list and
    pointer conversion of `struct flb_input_instance` and `struct
    flb_input_collector`; one assignment, copy, initializer list, byte
    function or character view refutes it). A generated function naming an
    input list, another caller of the registration, an unlink or re-link of
    an input instance (`_head` used other than by the constructor's append)
    and a listed-member mutation in a function the census does not read are
    hazards. **P-HEAD-INSTANCE** is narrowed: the constructor returns a
    non-null instance only after appending its one allocation (every other
    return null, no jump between); stated: **P-CREATIONS-SUCCEED** (the
    context's and the one creation's constructors return non-null; the test
    does not check them before its read). **P-INSTANCE** is checked per
    obligated collector operation: derived where the instance argument is its
    function's unassigned first parameter, every edge into the function a
    loop dispatch `C->cb_collect(C->instance, …)` and a collector's `instance`
    written only by its constructor through member writes; a refuted part
    (another instance dispatched, the parameter reassigned, a collector
    re-pointed or written bytewise) makes a discharged site unresolved; any
    other form stays stated and is named per site (`ctx->ins`:
    P-INSTANCE-CONTEXT; a coroutine entry's dispatch: P-CORO-HANDLE's).
    **Do not** read a generated file the includer's search path does not
    resolve to, take an unread registration or an unmatched literal as the
    build's descriptors, count a function the census does not read as
    hazard-free, discharge a site whose P-INSTANCE is refuted, or read a
    conversion to `void *` as a member access. Premises added (trust boundary
    28): P-CREATIONS-SUCCEED; the residual of P-INSTANCE-FIELDS-BY-MEMBER (a
    pointer converted to `void *` or an integer is not written through
    bytewise, one converted to another object type is not used to access an
    instance, under C's effective-type rule, and no offset from another
    member's address reaches these members); P-INSTANCE-CONTEXT; that the
    generated root's file is the one the build compiled. At the export the
    fixed test's 3 sites stay **discharged at this path under these derived
    premises**, P-INSTANCE derived at each; the old test's stay not
    discharged. `QCore.LoopOwner` proves a derived initializer leaving the
    empty list the head derivation starts from, a registered creation's
    instances running no coroutine and an unread registration keeping the
    route, a straight-line constructor block appending or returning null, a
    failed creation leaving the fresh list empty, a refuted P-INSTANCE never
    discharged with the premised verdict adding none, and a non-member write
    keeping the route;
    [report](../reports/w47-premises-instance/README.md).
    **Amendment (W47-PREMISES-HOLDER-REGION): the region's macro invocations
    and the holder's opacity derived.** (1) With region-macro facts
    (`region_macros`, `loop_owner_region_macros.rs`, child of
    `thread_entry_census.rs`; written by the `region_macro_facts` example),
    each unit of the list-head region's files is run with its recorded
    compiler and flags under Clang's token dump, and every outermost
    invocation in a census file the unit read is recorded with its whole
    expansion (each token a replacement list's or an argument's) and the
    unit's own function definitions (one outside the census's files with
    its body's names). Step B reads every invocation on the region's lines
    (the test up to the read, the maker, the constructors read in place, a
    keyed setter's bodies but the arms no key selects) from those facts: a
    replacement list's `return`, `goto` or unmatched `break`/`continue`, a
    field dispatch, or a listed member (`inputs`, `in_plugins`, a written
    `config`) refuses; a replacement-list name is resolved in the unit (its
    definition in a read file; one outside the census read through its
    body's names; else an external definition, a name only an unstructured
    file may define refusing), an argument name as the lexical region
    resolves it; a resolved function that reaches a hazard refuses (a call
    an expansion makes, drops, repeats or loops). The region's own
    functions' typed indirect calls, and a unit function under the source
    root without a typed reading or with an indirect call, are unmodelled
    calls. The census graph the region reaches over gains every function's
    macro-made calls, from the units' expansions for the files they read,
    else every read file's definition of the invoked macro (every arm; a
    target no compiled unit defines dropped) and every registrant of a field
    its replacement list calls; a root that reaches a hazard only through one
    refuses, and the widened closure and reach feed the unmodelled-call
    blocker. A region file no derived unit read, a failed unit, or a text
    other than the facts' refuses; the facts decide nothing unless the
    database's SHA-256, every command of a derived file and the recorded
    compiler's version match. Without the facts the reading is unchanged
    and the premise stays stated. (2) P-HOLDER-OPAQUE is derived: every read
    of `struct flb_input_instance::data` the typed reading records (a
    function without a typed reading read lexically, a member of an object
    it declares with another record type set aside) is followed as the
    holder's pointer: compared, copied into a local, or passed whole to a
    read function's parameter or through a field dispatch to its
    registrants' (the dispatch of the instance's own callback, `X->p->F(…,
    X->data)` or through `Q = X->p`, reaches only the copy's `F`: the
    installing function's store or the creating descriptors' designated
    initializer, since the plugin and the user pointer are installed
    together and have no other writer but the constructor); a carrier may be
    compared, read through or passed on, and in the holder's file a member
    other than the copy may be written. A write, an address or a dereference
    through a carrier outside the holder's file, the copy written, a carrier
    returned, stored or passed to a function the reading does not read, or a
    use it does not classify leaves the flag underived. **Do not** expand a
    region macro by name across the read files, read an invocation from
    another unit's expansion or text, treat a macro-made call as absent from
    the census graph, admit a carrier write outside the holder's file, or
    take another plugin's callback for the holder instance's. Premises added
    (trust boundary 28): **P-DUMP-TOKENS-COMPLETE** (Clang's token dump
    prints every token the unit's preprocessor produces with its expansion
    and spelling locations); P-INSTANCE-FIELDS-BY-MEMBER extended to reads
    of `data` (no whole-instance copy or byte access carries it); C's
    effective-type rule for a member of an object declared with another
    record type; and, outside the units' files, rule 10p's premise that a
    macro defined only outside the read files names no read function. At
    the export the fixed test's 3 sites stay **discharged at this path under
    these derived premises** with P-HOLDER-OPAQUE and the region-macro
    premise now derived (206 invocations of 43 macros on the region, 16,741
    macro-made calls added to the graph, the holder's pointer reaching 5
    carriers); the old test's stay not discharged. `QCore.LoopOwner` proves
    a read region deriving what the expansion runs, an unexpandable or loud
    invocation refusing, the unit's own definition read, a root reaching a
    hazard only through a macro-made call refused, an opaque holder keeping
    the derivation, a writing carrier keeping the route and the holder's
    instance running its copy's callback;
    [report](../reports/w47-premises-holder-region/README.md).

<a id="constraint-10q"></a>

10q. **A condition-variable call is admitted only where the census derives
    the lifted shape, and storage another thread may retain is released only
    after a checked join of every live thread; every route that derives
    neither refuses.** Card W47-CONDVAR-JOIN-PUBLICATION
    (`qcore_core::condvar_join`, `condvar_census.rs` read by
    `quirin_source_analysis::condvar_join_census`, the hook in
    `legacy_literal_premise.rs`; contract specification C04.10; constraint 27
    as amended). `requires resource_thread_sync(...)` on
    `pthread_cond_{wait,timedwait,signal,init,destroy}` and `pthread_join`
    (`libc/pthread_sync.qv`, flexible, custody neutral; `pthread_join` only
    at a null result slot) is never discharged on the per-unit resource
    route, which rewrites every such call under the literal premise's
    reserved prefix with a `thread_sync__` mark and refuses it (`outside its
    declared thread-synchronization obligation`). The census reads one
    program's bodies into statement trees and decides: (C4) a wait only as
    the body of a `while`/`for` rechecking a predicate under the mutex it
    releases (a timed wait may `break` on a nonzero return); a signal only
    while the condvar's paired mutex is held; a publication only when the
    signalling section writes every result before the predicate; a guarded
    read only where the predicate was observed under the mutex since it was
    acquired, or where its single publisher signals at most once per thread
    (derived: one signal site, called only by one spawn entry, never twice on
    a path); `init`/`destroy` only on a fresh object, after that single last
    signal, or after a covering join. (C5) A path interpreter runs each test
    root on the test thread, inlining every function whose direct calls reach
    a spawn, a join or a release, and a release (`unlink`, `flb_destroy`, the
    destroy or scope end of a local whose address the root stored into a
    field) discharges only when every live retaining thread was joined on
    every path with its zero result checked; publication, a timeout, an
    unchecked or a failed join, a join identity the reader cannot derive and
    a joined thread's own children leave it refused or unresolved. Kept
    refusals (broadcast, a wait outside a predicate loop, a signal without
    the mutex, one condvar with two mutexes, a static initialiser, `cnd_*`,
    rwlocks, try/timed locks, recursive mutex shapes) refuse. **Do not** apply
    a thread-sync declaration on a route that derives no predicate loop or
    join, treat publication or a timeout as a join, admit an unchecked join,
    or relax a refusal because another path discharges. `pthread_create`
    stays undeclared (rule 10j: a retained `arg` and a callback). Premises
    stated, not proved (trust boundary 28): the lexical reading and its
    binding tables (spawn wrappers with their identifier out-arguments,
    releases, allocators, acutest's `TEST_CHECK` as its argument's truth);
    P-SYNC-INSTANCE (a field-keyed condvar, mutex or guarded field names one
    object per instance, and a waiter and its signaller name the same one);
    P-JOIN-IDENTITY (a thread identifier stored in a field is the one the
    same context spawned); P-RETAIN (the test's published storage and the
    fixture are retained only by threads that run the tail input's or the
    test's code, derived from the loop-owner census's contexts; the engine
    context by every live thread); and acutest's per-test fork. A refusal is
    `Unknown`, never `Unsafe` or an upstream defect claim. `QCore.CondvarJoin`;
    [report](../reports/w47-condvar-join-publication/README.md).
    **Amendment (W47-CONDVAR-JOIN-STOP-PATHS): stop paths, POSIX joins and
    descendants.** (1) The path reader drops the null arm of a field test
    (`!X->F`, `X->F == NULL`) only where the kernel's
    `null_branch_excluded` admits it: an earlier top-level statement on every
    path to the test in the same function dereferences `X->F` (not in a
    call's arguments, under `sizeof` or beside a conditional operator),
    `X` and `X->F` are never assigned and `&X` never taken there, `X`'s
    declared type is read, and every store of `F` into an object of that
    type in the file set is a pre-publication store (into a local holding a
    table allocator's result that no call, store or return has received on a
    path to it), with no address of the field and no whole-object write of
    the type; an unread base type anywhere fails it. (2) A join of a thread
    the path already joined is **repeated** and never ends it. An unchecked
    **first** join (of a thread the path started, recorded with its spawn
    wrapper) counts as successful only where `join_succeeds` admits all four
    POSIX exclusions: *joinable* (the wrapper's body calls `pthread_create`
    once through its out-parameter with a null or `PTHREAD_CREATE_JOINABLE`
    attribute and names no detach, and no reached detach may name the
    thread), *not busy* (no reached join that may name it runs off the
    joining thread or on a path some hop leaves unresolved), *not self* and
    *no cycle* (no reached `pthread_self()` that may keep its value runs on
    the joining thread or on an unresolved path; one passed straight to
    `pthread_equal`/`pthread_kill` is not kept). It stays "not checked"
    otherwise. (3) Each spawned entry is run on its own thread to its exit
    (bounded by a step budget whose exhaustion is unresolved); a live
    retaining thread's descendants are decided per parent: *joined* (every
    exit path joined it by a checked or admitted first join, nothing
    unresolved, and so its own descendants), *never joined* (no reached join
    may name it and every join identity is read: not discharged), or
    unresolved with the reason (a spawn wrapper's own internal entry is
    unresolved, never never-joined). **Do not** exclude a null arm on a
    dereference the build may drop (a macro argument), read an unchecked
    join as a checked one, count a join whose POSIX case is open, or treat an
    unread join identity as naming another thread. Premises added (trust
    boundary 28): **P-DEREF** (an execution that dereferences a null pointer
    is not analysed) and **P-POSIX-JOIN** (`pthread_join` returns 0 unless
    its target is not joinable, already joined, awaited by another joiner,
    the caller, or in a join cycle through it; the default and the
    `PTHREAD_CREATE_JOINABLE` detach states are joinable); the declared-type
    reader's scope (a local declaration shadows a file-scope one; a name
    declared by a declaration the reader cannot split is unread) and C's
    effective-type rule. Every decision names the premises it used
    (`release_premises`). `QCore.CondvarJoin` proves the dropped arm's
    derivation, the four-case join rule, the repeated join and the
    descendant block; the observed engine-thread facts are recorded as
    Fluent Bit contracts awaiting review (`fluent_bit/lib_engine_thread.qv`,
    §C06.1: withheld, never `Safe`);
    [report](../reports/w47-condvar-join-stop-paths/README.md).
    **Amendment (W47-FLB-LIB-LIFECYCLE): a declared lifecycle is checked
    against its bodies, and only a precondition gates a release.**
    `requires resource_lifecycle(ctx, start|stop|destroy)` (contract
    specification C04.11, `qcore_core::paired_lifecycle`, the census's
    `condvar_census/lifecycle.rs` read by
    `quirin_source_analysis::lifecycle_census`) states a paired
    start/stop contract; the census checks it, never trusts it. Each declared
    stop is run by the path reader from each effect clause's state (never
    started, after a start that returned 0, after a stop, after a start that
    returned nonzero) recording every join it executes: a clause is in force
    only where every followed path satisfies it with nothing unresolved,
    refuted only where every followed path contradicts it, held otherwise.
    A refuted clause is withdrawn from every caller (rule 10g's sense) and
    the stop is read from its body; the preconditions (start and destroy only
    from created or stopped) are in force as restrictions. On a test root's
    paths each declared call records its context's history (keyed by a field
    key or a frame-local name; created where a creator's result is assigned,
    `flb_create` in `FLUENT_BIT_LIFECYCLE_CREATORS`), and a release that a
    path violating a precondition reaches is refused first
    (`lifecycle_gate`): the reader keys threads by entry, so a second start
    would collapse two engine threads into one and one join would look like
    it ended both. The per-unit route refuses every declared call
    (`lifecycle__` mark under the reserved prefix, behind the review hold
    that applies first). **Do not** apply a held or refuted clause, derive a
    state from a history without its creation or after its instance's
    destroy (P-LIFECYCLE-INSTANCE, stated: one key names one instance at a
    time), refute on a contradiction some followed paths only show, or let
    the lifecycle discharge a release: it can only refuse. At the three
    trees `flb_stop`'s body satisfies L2 and refutes L3-L5 (every feasible
    path joins `ctx->config->worker`), candidates only; the fixed test
    conforms, and its C5 stays not discharged by the body's repeated join.
    `QCore.CondvarJoin` (section `Lifecycle`);
    [report](../reports/w47-flb-lib-lifecycle/README.md).
    **Amendment (W47-FLB-LIB-STRICT-PAIRING): the lifecycle is a strict
    pair; the stop's "no effect" clauses become caller preconditions.** On the
    user's decision of 2026-09-29 (a double join hits memory issues, so
    `flb_stop` is not idempotent), the state machine of contract
    specification C04.11 is: a start only from created (L1; a restart is not
    derived from any body, so it is refused); a stop only from started-ok, a
    start that returned 0, and once (L3 no stop after a stop, L4 none after a
    failed start, L5 none of a never-started context: caller preconditions,
    not effects); a destroy from created, start-failed or stopped (L6). A stop
    or destroy after a start whose value the reader does not derive is **not
    established** and refused like a violation. L2 (the first stop after a
    successful start joins that start's thread once) stays a body clause, and
    L7 (a start that returns nonzero leaves no thread it began unjoined, the
    body fact behind L6's start-failed admission) is added as one; both are in
    force only where every followed path satisfies them. What the stop's body
    does from a forbidden state is reported as an observation outside the
    contract, never a clause. Each release additionally reports its paths
    split into conforming (every history derived, none refused), undetermined
    and refused, and the verdict over the conforming paths alone
    (`release_split`); the release's own verdict stays the join over every
    path. **Do not** read a stop from a state the strict pair forbids as
    anything but a caller violation, admit a restart without a body
    derivation, count a path whose history is not derived as conforming, or
    let a conforming-path verdict replace the release's own. At the three
    trees L2 is in force and L7 held (the reader cannot tell a failed
    `mk_utils_worker_spawn`); in the in_tail test roots 50 of 280 declared-call
    readings violate (L4, 5 also L5) over 46 roots, each a candidate against
    the test, never a library defect; no release verdict moves. The fixed
    test's failed-start path is refused and its conforming paths stay not
    discharged (the status branch's unchecked first join and unresolved
    descendants); the old test stays not discharged. `QCore.CondvarJoin`
    (section `Lifecycle`, rewritten);
    [report](../reports/w47-flb-lib-strict-pairing/README.md).

<a id="constraint-10r"></a>

10r. **Route T reads a node only where the consumer that imports through it
    classifies that node; every other consumer refuses it by name.** Card
    CG01-T-OPERATOR-CLOSURE (`crates/qcore-c-gateway/src/typed_source_nodes.rs`,
    the hook in `typed_source.rs`). The bounded typed-source importer reads
    binary `-` `<=` `>=` `^` `<<` `>>`, unary `+` `~`, the ten compound
    assignments `+= -= *= /= %= <<= >>= &= ^= |=` **as statements**, and
    floating and character literals (`SourceExpression::Floating` /
    `Character`, Clang's spelling or code with its Clang type). Operators need
    promoted integer operands and results (pointer arithmetic and comparison
    are gap entries M15/M16). A compound assignment's place is one of T's local
    scalar types. Its computation types are the ones Clang records. It projects
    one place evaluation, one read, the operation with explicit `IntegralCast`s,
    and one writeback. An RHS call or an effectful address is refused, and so is
    a compound assignment used as a value (CG02).
    **Reading is not admission.** Every importer call passes a
    `NodeAdmission`, the consumer's own classification, validated against
    `NODE_TABLE` before any function is read:
    - *custody-neutral* only for a node with no C fault case and no store;
    - *numerically evaluated* only through `checked_binary`, which refuses
      signed overflow, `/` `%` by zero, `MIN / -1`, an out-of-range or negative
      shift count, a left shift of a negative value, and the
      implementation-defined right shift of one;
    - *obligation-bearing* otherwise, never discharged.

    The legacy entry points
    `import_source_functions` / `_from_profile` pass `NodeAdmission::LEGACY`,
    which admits nothing and refuses each node by name. Only the Monkey wrapper
    projection (`import_monkey_wrappers`) admits the closure; it certifies
    nothing. **Do not** give a consumer an admission without reading every
    reader of its summaries: the census found readers that would accept an
    unknown operator without deciding it (call-use guards, Cycle 18 timeout
    steps, event-payload identities). Do not call a fault-bearing or storing
    node custody-neutral, and do not evaluate one outside `checked_binary`.
    `QCore.TypedSourceOperators`, bound to the Rust tables by a shared-vector
    test; [report](../reports/cg01-t-operator-closure/README.md).

<a id="constraint-10s"></a>

10s. **The common expression elaborator is asked after every older route of a
    statement, a `return` or a controlling expression, orders only what C
    sequences, and admits an unordered pair only when it commutes.** Card
    CG02-ELABORATION-CORE (`crates/qcore-c-gateway/src/elaboration.rs`, the
    route-G adapter `legacy_elaboration.rs`, thin hooks in `legacy_c.rs`,
    `legacy_condition_desugar.rs` and `typed_source.rs`). `elaborate(expr,
    ctx)` reads Clang's typed AST into a result (a value whose increment
    origin, old or new, is kept apart), an evaluation graph (events in one
    permitted serialisation, explicit sequenced-before edges, explicit
    unsequenced operand groups), read/write/call events and access, contract
    and numeric obligations or assumptions. The operands of an operator, an
    assignment's place and value, and a call's actuals are unsequenced; a side
    effect follows only its operands' value computations; a comma, and the
    sequence point before a call, order every earlier event. The graph is
    refused unless **every pair the closure leaves unordered commutes**: two
    reads, or a pair on disjoint places (distinct objects; a path inside
    another object; a path through a pointer against an automatic object whose
    address is never taken). A write/write or write/read pair on one named
    object is C undefined behaviour and refuses; a pair on places not proved
    disjoint (two paths) refuses without that claim; a call unordered with any
    event refuses (CG05). Every form outside `FORM_TABLE`'s admitted rows refuses by name
    with its owning card (value `&&` `||` `?:`: branch regions since rule
    10z; nested call CG05, pointer
    store CG11, pointer arithmetic CG16, type query operand CG08, `_Generic`
    and statement expression CG07, compound literal CG14, volatile CG15, atomic
    CG19, indirect call CG22). **Route G** (1) lowers a refused statement that
    holds a compound assignment, an increment, a comma or an assignment used
    as a value, matched to one Clang node by line and exact tokens, in
    sequenced pieces: each comma operand and a chain's inner assignment is a
    statement the older lowering decides first (a call keeps its one contract
    application), then the piece's own events — a scalar local write is
    `ForgetScalar`, a path access is the typed access-path reader's
    `IndirectScalarAccess` with its per-hop obligations, and a call outside a
    piece, a pointer-valued store, a protected formal, a tracked descriptor, a
    non-local or non-scalar object write refuse the whole statement; (2) lowers
    a refused `return` as its leading comma operands, then the older return
    lowering of the last operand (`return NAME = E` as `NAME = E; return
    NAME;`), or a scalar value's events with a `NoOp` exit in a scalar-result
    function; (3) lowers a refused call-free controlling expression holding
    such a form, with no connective and a scalar value, as its events then a
    branch that refines nothing. **Route T** names each value form in its
    refusal: no typed-source consumer decides one (CG01's census). **Do not**
    ask the elaborator before an older route (hard invariant 3; the
    `QCORE_TYPED_PATH_PARITY` stream records each consultation with
    `position: "elaboration-…"`), serialise an unordered pair that does not
    commute, treat two paths as disjoint (CG10/CG13 own footprints), lower a
    call inside a piece's events, or read a statement's own top-level `=`
    as this card's form (its refusal is the older routes' or another card's).
    Premises stated, not proved: faithful Clang extraction and node matching
    (trust boundary 28), and that `IndirectScalarAccess` and `ForgetScalar`
    are the custody meaning of a scalar access and write (the W43/W46 rules).
    `QCore.ElaborationCore` (`interleave_run`, `read_stable`,
    `commutes_sound`, `unsequenced_refusals`), bound to `FORM_TABLE`,
    `commutes` and ten C order vectors by a shared-vector test;
    [report](../reports/cg02-elaboration-core/README.md).
    Amended by [rule 11b](#constraint-11b): the cast and type-query rows
    are read by `CAST_TABLE` and `query_operand`, and a statement, `return`
    or condition holding a cast or a type query is consulted too.

<a id="constraint-10t"></a>

10t. **A mixed place on a root the closure does not prove never bound is read
    only with the chain's own per-hop obligations, and a duplicate statement
    only by its own occurrence.** Card CG10-COMMON-TYPED-PLACE
    (`typed_place.rs`, thin hooks in `typed_access_path.rs`, `legacy_c.rs`,
    `legacy_typed_positions.rs`, `typed_access_positions.rs`). (1) **The
    place.** A common typed place is read from the Clang AST only: a root
    `DeclRefExpr` naming a `ParmVarDecl` or `VarDecl` (its DeclID), then
    `->F` / `.F` (FieldDecl id), `[I]` and `*`, each with Clang's type and
    view; a subscript or indirection is a *hop* only through an
    `LValueToRValue` pointer read, never through `ArrayToPointerDecay`. A
    missing fact, a union or bit-field member (read by rule 10w since
    CG13), a `volatile`/`_Atomic` base,
    any cast but those, a call, pointer arithmetic or any other form refuses.
    (2) **The tracked root.** Where W46's mixed-path premise (a never-bound
    root) fails, the place is admitted only on a pointer root that the
    function declares exactly once as a parameter or automatic local, with no
    reference to another object of its spelling, and only when its hops are
    the root alone (`R->in.x`, `R->arr[i]`, `R[i].x`) or the root and the
    direct member place `R->F` (`R->items[i].v`, `*R->p`, `R->F->in.x`). It
    lowers to the existing chain obligation with `root_never_bound` false:
    the root must be exactly one known, non-null, live identity, and `R->F`
    must carry the admitted store's evidence (`chain_resolves`); a later hop
    needs an untracked owner. A store through a subscript or indirection
    keeps W43's refused-write frame; a non-pointer read or store changes no
    custody, status, marker or field link; a pointer value read is P2's
    target-free borrowed alias. (3) **Identity (I04).** Several Clang
    statement candidates with one statement's line and tokens are told apart
    only when the statement being classified is the run the parser consumed
    (its stream index and length), the line's tokens equal the analysed
    source's own, every candidate lies on that line at a distinct offset, and
    the candidates number exactly the run's occurrences on the line; the k-th
    occurrence is the k-th candidate in source order. A clause, operand,
    desugared or rewritten run keeps the unique-match refusal. **Do not**
    serve the never-bound premise from this route (a root with no binding, a
    target-free or possibly-null root refuses at the flow), admit a pointer
    loaded from a place deeper than `R->F` or from an element or pointee of
    the root, admit a record-object, array, `static` or file-scope root, a
    pointer-valued store (CG11), a custody-taking actual
    (`W44-CALL-ARGUMENT-CUSTODY-POSITION`), or relax rule 10k's pointer-return
    and pointer-actual premises; do not consult this route before an older
    route of the position (hard invariant 3). Parity runs record every place
    decision (`position: "typed-place"`, its root, steps, types and refusal).
    Premises stated, not proved: faithful Clang extraction of types, field
    and declaration ids and cast kinds, and that the gateway's tokens of a
    line are the analysed source's (trust boundary 28).
    `QCore.TypedPlace`, bound to the Rust decision table by a shared-vector
    test; [report](../reports/cg10-common-typed-place/README.md).
    Amended by [rule 11d](#constraint-11d): an eligible root with no tracked
    identity is read as untracked on a path where nothing bound it and
    nothing was refused before the access; the never-bound premise stays
    off this route.
    Amended by [rule 12j](#constraint-12j): a `.`-only scalar place on a
    record object of the function's automatic storage is read with no root
    premise (no pointer is read).

<a id="constraint-10u"></a>

10u. **A call contract is instantiated once, and a consumer takes the
    instantiated call by the use C makes of its value, never by where it
    stands.** Card CG03-CALL-SUMMARY-INSTANTIATION (`call_instantiation.rs`,
    the route-G adapter `legacy_call_instantiation.rs`, thin hooks in
    `legacy_c.rs`, `legacy_condition_desugar.rs` and `legacy_elaboration.rs`).

    (1) **The application point.** `Parser::instantiate_call(callee,
    statement, destination, use)` is the one place a consumer asks for a
    contract. It runs the classifier's one lookup and per-formal substitution
    into the `ContractCall` the flow applies once. A call whose actuals the
    lexical gate refused arrives with the statement position's typed
    argument reads (W44's `IndirectScalarAccess`/`ArgumentReads`) before it.
    The standing is read before the classifier runs.

    (2) **Acceptance** (`call_instantiation::accept`) is a function of the
    callee's standing, the result class, identity, and the value's use:
    - a callee in the caller's recursive component is refused in every use
      (constraint 4's provisional summary);
    - a discarded or tested value (a condition leaf, a scalar `return`
      operand) must own nothing, unless it is rule 10x's NULL split
      (`TestedSplit`);
    - a stored value is the classifier's own;
    - a whole returned value carries only fresh custody;
    - a passed value is the identity with no result (card B04), or, since
      rule 10y, any result but a reallocation, which the nested-call
      consumer binds to a reserved place or reads as data under its
      footprint order check.

    The condition desugaring asks it for every leaf call with identical
    refusals.

    (3) **Three fallbacks, each asked after every older route of its
    position:**
    - **A scalar `return` operand.** In a scalar-result function, a refused
      `return` that is one condition leaf (negations allowed, no connective)
      is lowered exactly as the same leaf in an `if` would be: its calls
      (two only if each is the identity), then an exit that escapes nothing.
      It keeps rule 5j's premise (no unary `&` outside the calls' actuals)
      and the reserved-place guard.
    - **Effectful actuals.** A statement piece that is exactly one direct
      call whose actuals carry effects is its elaborated events (sequenced
      before the call by C) and then the call, with each effectful actual
      data at a `Borrowed` formal. This is the elaborator's
      `EffectfulActual`.
    - **Typed substitution.** A call statement whose lexical substitution
      was refused (`call has an unmodeled formal address or borrowed
      expression`) or left a custody formal empty (`arguments do not match
      resource contract`) is read from Clang's AST (value → identity →
      formal):
      - a named pointer object, through parentheses and value-preserving
        pointer conversions, is its identity;
      - `&x` at an `Out` formal is the slot;
      - an expression whose evaluated reads are scalar (a `sizeof` operand
        unevaluated) is data.

      The call is reclassified with those actuals and admitted only if every
      custody formal is bound to exactly the identity the AST names.

    **Do not:**
    - admit a place, an address or a scalar at a custody-taking formal
      (`W44-CALL-ARGUMENT-CUSTODY-POSITION` stays by design; a scalar
      custody actual is the descriptor route's);
    - read pointer arithmetic (CG16), a nested or unordered call (rule 10y's), a
      value connective or conditional (CG06) or any other cast (CG08) as a
      substitution;
    - widen a use's acceptance for one consumer only;
    - consult a fallback before an older route (hard invariant 3).

    The statement and whole-call `return` consumers keep their older
    admission of a recursive-component callee (the fixed point's own use).
    Every such call, and every older admission the acceptance declines, is a
    parity row (`position: "call-instantiation"`), as is every consultation.

    Premises stated, not proved: faithful Clang extraction, that a bit cast
    between object pointers keeps the address (C11 6.3.2.3p7), and the
    token/AST node match (trust boundary 28).

    `QCore.CallInstantiation`, bound to `ACCEPTANCE_VECTORS` and
    `SUBSTITUTION_VECTORS` by shared-vector tests;
    [report](../reports/cg03-call-summary-instantiation/README.md).
    Amended by [rule 11b](#constraint-11b): the data reading takes
    `FloatingToBoolean` and the null test of a named pointer object.

<a id="constraint-10v"></a>

10v. **A pointer-valued store is read only where no custody moves: a value
    with no identity into an untracked owner, and a body with one publishes no
    summary.** Card CG11-POINTER-VALUED-STORES (`pointer_store.rs`, the flow
    in `legacy_pointer_store.rs`, thin hooks in `typed_access_path.rs`,
    `legacy_c.rs`, `legacy_summary.rs`, `indirect_chain_access.rs`
    (`chain_owner`), `legacy_joined_provenance.rs` and
    `legacy_descriptor_custody.rs`). On the typed statement route only, after
    every older route declined, a refused `P = V` whose place has pointer
    type is read from the Clang AST, with four events kept apart:
    (1) **the value store** writes `P` with the place's own per-hop
    obligations, read exactly as W46/CG10 read a scalar store's; (2) **alias
    and overwrite**: every binding of a place rooted at `P`'s root is
    forgotten, and the value adds none; (3) **the old value's obligation**:
    the store is admitted only when the root *and* the object the member lives
    in are untracked at the node (`chain_owner`; a never-bound record object
    of this frame selected with `.` only), and no binding of a place rooted
    at `P`'s root carries a target or is unknown; a tracked owner may own
    its member, and only O04-B08's field-link store rewrites one;
    (4) **escape**: `V` carries no identity, being the null constant, a string
    literal, a function designator (or its address), a name of
    function-pointer type bound to no identity on any reaching path, or the
    address of the destination's own object (the root itself, or `&` of a
    place read through no pointer but the root's). Every other value refuses
    by name: a formal's object pointer (Q36's example C; its missing premise
    is an owner identity for `*s` that bounds `*q`'s lifetime, or a derived
    Stored summary mode), a local's pointer, a pointer read, a call,
    arithmetic, another object's address, and a cast other than a
    qualification conversion. Every failing premise makes the node exactly
    the gate's refusal, byte for byte. A body with such a store publishes no
    derived summary and establishes no specification (`has_pointer_store` in
    the W4 gate): a caller would read `Borrowed` for the formal whose object
    the body overwrote, the `cfl_list_add`/`mk_list_init` hazard of rule 5i.
    **Do not** admit a value that carries or may carry an identity (the
    STABILITY-REEVAL-5 `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE` shape needs
    one in a member), a store into a tracked owner or through a tracked
    root's untracked child, a store the elaborator reads as a piece, an
    `*slot` store (the out-slot refusal is not consulted) or an element store
    an older route decides; **do not** drop the summary gate without a
    summary mode that states the overwrite. Premise P-CG11-CALLER-SLOT,
    stated, not proved: the old value of a slot in an untracked object is not
    this body's obligation. Premises stated, not proved: faithful Clang
    extraction of types, cast kinds and declaration ids (trust boundary 28).
    `QCore.PointerStore`, bound to the Rust decision table by shared-vector
    tests; [report](../reports/cg11-pointer-valued-stores/README.md).

<a id="constraint-10w"></a>

10w. **A union-member or bit-field place step is read only as its storage
    permits: a union member as overlapping storage, a bit-field as its storage
    unit.** Card CG13-UNION-AND-BITFIELD-PLACES (`union_bitfield.rs`, thin
    hooks in `typed_access_path.rs` (`Reader::path`), `typed_place.rs`
    (`MemberStorage` on each step, `Refusal::UnionHop`), `elaboration.rs`
    (`Context::shared_bitfield_unit`, `Refusal::SharedBitfieldUnit`),
    `legacy_elaboration.rs` and `typed_c_ast.rs`). It amends the union and
    bit-field refusals of rules 10k and 10t; every other refusal of theirs
    stands. (1) **The step.** A member whose `FieldDecl` a complete `union`
    owns is a union step; a `struct` bit-field whose width Clang gives is a
    bit-field step, in the storage unit C11 makes it (a maximal run of
    adjacent non-zero-width bit-fields in declaration order, never a guessed
    layout). A union bit-field, a bit-field without a width, a member whose
    owner is not read, a `volatile`/`_Atomic` base and every other form
    refuse. (2) **The decision** (`decide`, per step, from the access through
    it): a scalar read is custody-neutral whatever member is live; a pointer
    value read (the path's value, or a pointer later read through) is P2's
    target-free borrowed alias, refused when the function stores through,
    increments or takes the address of another member of a union of that
    record, or writes a whole object of the union's type (type punning, M22);
    a scalar store through a union that can hold a pointer (a member not
    transitively a builtin scalar, enumeration or array of them) carries
    W43's refused-write frame, which poisons every binding but a spared
    local, so no sibling pointer member keeps a tracked identity, and through
    a custody-free union it is frame-neutral; a pointer-valued store through a
    union member refuses (CG11's store, and no contract fixes the view); a
    bit-field read is a read of its unit and a store a read-modify-write of
    the whole unit, which holds only bit-fields and is frame-neutral. On a
    tracked root a union step joins rule 10t's decision unchanged, but no
    pointer loaded after a union step is a link (`union-hop`). (3) **Units.**
    Two bit-fields of one unit are one memory location: disjoint bit ranges
    never make their footprints disjoint. When the elaborator refuses an
    unordered pair of two bit-field places of one object as not proved
    disjoint, it asks the core `disjoint_footprint` model with a layout
    contract read from Clang's own `-fdump-record-layouts-complete` output and
    names the refusal `SharedBitfieldUnit` (Q661) when the model does; the
    pair stays refused either way, and no field pair is admitted as disjoint
    here (CG18). **Do not** bind a pointer place or a field link through a
    union member, admit a punned pointer read, drop the frame of a store
    through a pointer-bearing union, treat two bit-fields of one unit as
    disjoint, guess a layout, or consult this rule before an older route of
    the position (hard invariant 3). Premises stated, not proved: faithful
    Clang extraction of field owners, widths and record layouts (trust
    boundary 28), and `union-live-member-view` — a callee's store through
    another member is not visible, so a pointer so read is still only a
    target-free borrow. `QCore.UnionBitfieldPlace`, bound to the Rust
    decision table by a shared-vector test and reusing
    `QCore.DisjointFootprint`; [report](../reports/cg13-union-and-bitfield-places/README.md).

<a id="constraint-10x"></a>

10x. **An acquisition in a condition is lowered only as its statement pair,
    split on its own failure value off every cycle, and a complex leaf is
    decided leaf by leaf.** Card CG04-CONDITION-ACQUISITION
    (`condition_acquisition.rs`, the route-G adapter
    `legacy_condition_acquisition.rs`, thin hooks in
    `legacy_condition_desugar.rs`, `legacy_elaboration.rs`
    (`elaborated_leaf`), `legacy_c.rs` and `call_instantiation.rs`). It
    amends rule 5h's custody-result premise and whole-condition type-operator
    and increment refusals; every other premise of 5h stands.

    (1) **The NULL split** (gap entry E09, example A). A `(NAME = CALL(…))`
    group whose leaf's residual is the existing null test of `NAME` (a
    pointer) or its existing validity test (a descriptor) is the use
    `ResultUse::TestedSplit`, which `call_instantiation::accept` takes for
    every result class (the tested row widened, rule 10u; a recursive-component
    callee still refuses). The leaf lowers to exactly the instructions of the
    statement pair `NAME = CALL(…); if (NAME …)` — the classifier's
    `ContractCall` with destination `NAME` (fresh identity, binding), then
    `BranchOnNull`/`BranchOnValidity` (success and failure edges), wired by
    the plan's edges (the merge) — so the flow decides the condition as it
    decides the pair: the obligation on the success edge only, a
    reallocation's failure edge keeping the old identity live. The call stays
    a node of the condition's plan (never hoisted), and an acquisition in the
    right operand of `&&`/`||` is applied only on the edges C evaluates it on
    (rule 5h's edges). Premises: `NAME` is one automatic, non-formal local the
    lexical or Clang reader types (rule 10l's declared local); a descriptor
    name only as the destination of its own integer-carried acquisition; a
    custody result is split only on its own carrier's failure value
    (`carrier`: a pointer's NULL, a descriptor's sentinel); and **no custody
    split on a node a CFG cycle may pass through unless the flow keeps the
    site's generations apart** (`placement`: a loop position, a loop body, or
    a range a backward `goto` closes; an unread extent, an unknown label or
    `setjmp` makes the whole body a cycle). **Amended by
    `CG04-LOOP-SITE-GENERATIONS` (rule 11a): the flow now indexes sites by
    generation, so the guard is lifted** (`placement` takes
    `SITES_ARE_GENERATION_INDEXED`, and refuses only for a flow that does
    not); a split on a cycle decides as its statement pair, each earlier
    generation apart, the refuted failure edge removing only the fresh one.
    An owned result discarded, bound but not split, or
    split on another carrier keeps the refusal. An out-parameter acquisition
    (`if (f(&p) == 0)`) gets no split: no contract states the result/out
    relation (CG23); rule 5h admits the call as before, with the contract's
    unconditional `Out` on both edges.

    (2) **Complex leaves** (E13). Leaf by leaf (`leaf_route`): a type
    operator whose parenthesised operand holds no call is residual text
    (decided by the existing residual routes); a call inside a type operand,
    a type operator without a parenthesised operand, an increment beside a
    call in one leaf (CG05) and a connective below a leaf holding a call
    (CG06) refuse; a call-free leaf with an increment skips the older
    call-free routes and is the elaborator's (rule 10s: its events, then a
    branch that refines nothing); a call-free leaf the older routes refuse is
    asked of the elaborator last. A call-free condition keeps a plan only if
    a typed or elaborated operand was needed (rule 10k's premise, unchanged).

    **Do not** split a custody result on a cycle in a flow without
    generation-indexed sites (rule 11a), apply a call a
    type operand holds, hoist a condition's call, lower an increment beside a
    call, or widen `TestedSplit` for one consumer (a scalar `return` operand
    reads the same leaf; its split is no custody-neutral exit, so it keeps
    rule 10u's refusal). Every consultation is a parity row
    (`position: "condition-desugar"`, its answer and widenings). Premises
    stated, not proved: the flow's `BranchOnNull`/`BranchOnValidity` and
    reallocation arms are the custody meaning of the split (the statement
    pair's), and faithful Clang extraction (trust boundary 28).
    `QCore.ConditionAcquisition`, bound to `USE_VECTORS`, `CARRIER_VECTORS`,
    `PLACEMENT_VECTORS` and `LEAF_VECTORS` by shared-vector tests;
    [report](../reports/cg04-condition-acquisition/README.md).

<a id="constraint-10y"></a>

10y. **A nested or sibling call's contract applies inside-out, through the one
    application point, and unordered calls are admitted only when their
    contract footprints commute.** Card CG05-NESTED-CALL-EFFECTS
    (`nested_call_effects.rs`, the route-G adapter `legacy_nested_call.rs`,
    additive items in `elaboration.rs` (`Cell`, `Footprint`, `cells_overlap`,
    `footprints_commute`, `EvalGraph::check_footprints`,
    `Refusal::CallFootprintConflict`) and `call_instantiation.rs` (the widened
    passed row, `Decline::PassedReallocation`), thin hooks in `legacy_c.rs`).

    (1) **Where.** Asked last, only for a statement the classifier refused
    with O04-B09B10's `nested call … inside a modeled call` or with `resource
    call with a formal embedded in an unmodeled expression`, and for a
    `return` the older routes refused or lowered to the nested refusal. The
    statement is `CALL(…)`, `NAME = CALL(…)` or a one-declarator declaration of
    one, or a scalar store of a scalar value holding calls; the `return` is a
    whole call (C-GATEWAY-RETURN-CALL-W4's reserved place, use `Returned`) or,
    in a scalar-result function, a scalar value holding calls with an exit
    that escapes nothing. The node is matched by line and exact tokens (or,
    for W44's invariant `_Generic` rewriting only, by line, destination and
    callee) and every call, arity and plain actual is checked against the
    statement's own tokens.

    (2) **The reading.** Each actual is *plain* (no evaluated call: the
    classifier substitutes it), *bound* (the whole actual is one pointer call,
    through parentheses, pointer `NoOp`/`BitCast` and a selection's selected
    association, written to a reserved `__qcore_nested_value_N`, use
    `Passed`) or *data* (a scalar value holding calls, read by CG03's data
    reading with each call a scalar, only at a `Borrowed` formal or a declared
    tail; its calls `Passed` when whole, `Tested` otherwise), at any depth.
    Every call is instantiated by `instantiate_call` inside-out in the
    graph's order and must apply exactly the contract its footprint was read
    from; the reserved places are cleared after the statement, so an owned
    value the outer call only borrowed is reported as a leak.

    (3) **The order.** Each call is sequenced after every event of its
    actuals (C11 6.5.2.2p10); every other pair is unordered and must commute
    by its footprint: an identity at a `Borrowed` formal is read, at any other
    formal written, a pointer object's `&` is a write of its storage, an owned
    result is a written fresh identity; a pointer read reads its object and
    its identity. Two identity cells always overlap (names may alias), an
    identity reaches a named object only if its address is taken, a fresh
    identity overlaps only itself. No order C leaves unspecified is chosen.

    **Do not:** read a callee with no contract in force, a container effect, a
    reallocated result (`passed-reallocation`), an owned scalar result (the
    descriptor route's), data at a custody-taking formal, a place, pointer
    arithmetic, an effectful form, a conditional or a cast in a data value,
    an arity mismatch without a declared tail, or a body that spells a reserved
    place; treat two names as distinct identities; consult this rule before an
    older route (hard invariant 3). Premises stated, not proved: faithful Clang
    extraction and node matching (trust boundary 28), and that a call's effect
    on the abstract state is its contract's (constraint 4), as for a call
    standing alone. `QCore.NestedCallEffects` (`footprint_commute_sound`,
    `every_interleaving_runs_alike`, `inner_effects_precede_the_outer_entry`,
    `a_non_commuting_pair_is_refused`), bound to `OVERLAP_VECTORS` and
    `ORDER_VECTORS` by shared-vector tests; `QCore.CallInstantiation`'s passed
    row; [report](../reports/cg05-nested-call-effects/README.md).

<a id="constraint-10z"></a>

10z. **A value `&&`, `||`, `?:` or GNU `X ?: B` outside a controlling
    expression is lowered by branching on what decides it, with only the
    evaluated arm on each edge.** Card CG06-VALUE-CONNECTIVES-AND-CONDITIONAL
    (`value_connective.rs`, the route-G adapter `legacy_value_connective.rs`,
    thin hooks in `legacy_c.rs` (the statement's and `return`'s last step),
    `legacy_elaboration.rs` (`leaf_events`), `typed_access_positions.rs`
    (`argument_node`, `expression_node`) and `elaboration.rs`). It amends
    rule 10s (the three value rows of `FORM_TABLE` are `Branching`, plus the
    omitted-middle row) and rule 10n's pointer-`?:`-return and `?:`-actual
    refusals; every other refusal of theirs stands. (1) **The elaborator.** A
    value region is its deciding operand's events, `Event::Branch`, each
    arm's events after the branch (`A && B`: `[B]`/`[]`; `A || B`: `[]`/`[B]`;
    `C ? A : B`: `[A]`/`[B]`; `X ?: B`: `[]`/`[B]`, `X` once), then
    `Event::Merge`. The two arms are exclusive: the order check never pairs
    them, and checks each against every event outside the region it is
    unordered with. `Branch`/`Merge` touch no storage. (2) **Regions.** In a
    statement, a one-declarator declaration or a `return` every older route
    refused, a region is a connective or conditional C evaluates as part of
    the full expression, found through parentheses, casts, unary and binary
    operators, a direct call's actuals, a member or subscript base and the
    root's `=`/`op=` operands (`TRANSPARENT`); a nested one inside an arm is
    a region of that arm; one in a deciding operand is the condition CFG's;
    one under `sizeof` is no region; one under a comma, a nested assignment,
    a statement expression, an initializer list, `_Generic` or an indirect
    callee refuses by name. (3) **Order.** The lowering evaluates the
    deciding operand first; C leaves it unsequenced only with the region's
    siblings (subexpressions off the root-to-region path), so every sibling
    must be free of calls, stores, increments and volatile accesses, and a
    deciding operand with a call needs every sibling inert (a literal, a
    designation, or a read of an automatic object whose address is never
    taken); [rule 11m](#constraint-11m) asks these two refusals of a
    composition with CG05's footprints. `X ?: B` is admitted only when `X`
    is reads only: the lowering
    reads `X` again on the non-zero edge, which C's single evaluation
    permits only then. At most 8 leaves. (4) **The lowering.** The first
    region's deciding operand (`C`, `X`, or the whole connective, whose
    short circuit is rule 5h's condition CFG) is an `if` condition lowered
    by the `if` statement's own routes; each edge carries the expression
    with the region replaced by the arm C evaluates there (`1`/`0` for a
    connective, `X` for `X ?: B`), lowered as its own statement or `return`
    by the position's older routes, a refused statement edge then from its
    Clang node with the arm substituted, lowered from its elaborated events
    as a rule-10s piece; on an edge a call actual's typed reader reads the
    arm's node, and among several nodes with an operand's tokens the one
    inside the deciding operand. Any refused condition or edge declines the
    whole expression, byte for byte. (5) **Merge.** The edges join at the
    `if`'s join: two owned identities become one binding with two targets,
    and a release through it is refused (never resolved to one); every
    obligation of either edge is carried. **Do not** duplicate a connective
    in a controlling expression, serialise a deciding operand before a
    sibling with an effect, re-read an `X` with an effect, pick one identity
    of a merged binding, or consult this rule before an older route of the
    position (hard invariant 3). Parity runs record each consultation
    (`position: "value-connective-statement"`/`"-return"`, its regions and
    refusal). Route T is unchanged: no typed-source consumer decides a value
    connective. Premises stated, not proved: faithful Clang extraction and
    node matching (trust boundary 28), and that an edge's older route decides
    its statement as it would the same statement written in source.
    `QCore.ValueConnectives` (short circuit, selection, single evaluation,
    the re-read, order, merge, forms and positions), bound to
    `ADMISSION_VECTORS`, `TRANSPARENT`, the elaborator and route G by
    shared-vector tests; [report](../reports/cg06-value-connectives-and-conditional/README.md).

<a id="constraint-11-series"></a>

Gateway rules after 10z continue as the 11 series (`11a`, `11b`, …) beside
[source-identity rule 11](source-identity.md#constraint-11); each keeps its
own anchor here.

<a id="constraint-11a"></a>

11a. **A re-acquired site keeps its earlier generations apart: the previous
    one precisely, every older one in a summary that never discharges.** Card
    CG04-LOOP-SITE-GENERATIONS (`loop_generations.rs`, the route-G adapter
    `legacy_loop_generations.rs`, thin hooks in `legacy_c.rs` (the work
    list, the settled exit pass, `kind_of`, `acquisition_line`) and
    `legacy_leak_witness.rs`). It amends the flow's acquisition
    (`create_resource`, `Acquire`, a foreign unlink's element) and lifts rule
    10x's placement guard.

    (1) **The defect.** The flow names a resource by its acquisition node.
    A node on a CFG cycle re-inserted the site `LIVE` over a possibly live
    earlier generation, and a refuted nullable acquisition's failure edge
    then removed the site, earlier generation included:
    `for (;;) { p = malloc(n); if (!p) break; use(p); }` and
    `do { p = malloc(n); if (p) use(p); } while (p);` were `Safe` while every
    earlier generation leaks. (2) **Generations.** Each site has three
    identities: *current* (the site), *previous* (`site | 2^29`) and *older*
    (`site | 2^30`). Current and previous are each one object per execution,
    so every precise update the flow makes applies to them; older is a
    summary. (3) **The shift.** Before a node runs (and before the driver's
    per-node reads), every site it produces that the state already mentions
    shifts: previous merges into older (statuses, nullability, unresolved
    effects and aggregate edges joined; every binding that named it becomes
    unknown, so any custody effect through it refuses; two generations'
    collection ranges are `Unknown`), current is renamed previous, and the
    node's own transfer creates a fresh current. A refuted failure edge
    removes only the fresh current. (4) **Refusal.** The merge is refused
    (`Unsupported`, the summary unresolved, the path unmodeled) when the
    merged summary may be live and an edge that can update it without a
    binding holds it (a field link, a containment, a pending reallocation, a
    publication, an out slot the node does not overwrite), or when one side
    may be live and the other carries an unresolved effect that would hide
    it (`decide`, positive in every input, so monotone: the settled state
    decides as every iterate did); a site the tags cannot index refuses. (5) **A refused re-acquiring node** (its transfer reported a
    path-poisoning diagnostic) leaves a shifted site it did not create
    present, live and unresolved: the call C ran may have produced it.

    **Do not** let any precise update reach the summary, remove it by a null
    refinement, exempt it through a binding, or widen it towards
    "discharged"; do not merge a live generation into a held or unresolved
    summary without the refusal; do not read a site's kind or line other
    than through its base. Measured (corpus `66910c10`, system, tail-pause
    slice): no function re-acquired a site over a live earlier generation
    without an unresolved effect on the base (the census, parity
    `position: "loop-site-generation"`), so no Safe moved, and none moved at
    all (586 / 10 / 4,238; 34 / 2 / 198; 26 / 1 / 242 / 1 Pending). Step 1
    changes leaves in 28 corpus functions (26 Unknown: rule-11a refusals
    where an unresolved generation meets a live one; 2 Unsafe:
    `rd_kafka_snappy_java_uncompress` gains 4 leak leaves that are the
    flow's imprecision on a `pass == 1` guard,
    `LOOP-GENERATION-COUNTER-CORRELATION`), 4 system and 2 slice Unknown
    functions; lifting rule 10x's guard changes 2 more Unknown functions
    (`rd_gz_decompress`, `rd_kafka_lz4_decompress`). Premises stated,
    not proved: that the shift renames every mention of an identity in the
    state (trust boundary 28) and that every other rule reports a refusal
    wherever it leaves an identity unresolved (the census
    `position: "withheld-obligation"` measures where one does not; since
    [rule 11c](#constraint-11c) a settled exit that withholds one on a path
    no refusal reached is itself refused). `QCore.LoopSiteGenerations`
    (`reacquisition_keeps_the_earlier_obligation`,
    `the_failure_edge_removes_only_the_new_generation`,
    `the_summary_never_discharges`, `a_live_summary_is_never_safe`,
    `for_break_reports_exactly_the_concrete_leak`,
    `the_one_site_flow_hides_the_leak`), bound to `STEP_VECTORS`,
    `IDENTITY_VECTORS` and the gateway's verdicts on the pinned shapes
    (`SHAPE_VECTORS`) by shared-vector tests;
    [report](../reports/cg04-loop-site-generations/README.md).

<a id="constraint-11b"></a>

11b. **A cast is a representation with no custody effect, and a type query's
    operand is read by its evaluation class; a pointer converted to a truth
    value or an integer never carries an identity.** Card
    CG08-CASTS-AND-TYPE-QUERIES (`cast_representation.rs`, the route-G
    adapter `legacy_cast_representation.rs`, additive items in
    `elaboration.rs` (`representation`, `type_query`, `pointer_operand`,
    `Context::untracked_pointer`, `Refusal::PointerEscape`, nine
    `FORM_TABLE` rows) and `call_instantiation.rs` (two data-reading cast
    rows), thin hooks in `legacy_c.rs` and `legacy_elaboration.rs`). It
    amends rules 5a, 10s and 10u and the W44 cast return; every other
    premise of theirs stands. (1) **Casts** (E15, M18, N08). Every Clang
    cast kind the route reads has a row of `CAST_TABLE` with a preservation
    flag (preserving / possibly lossy), a provenance flag (kept / lost / not
    applicable) and an admission; no row has a custody effect. The
    elaborator asks the table after CG02's own arms: a scalar conversion
    (`IntegralToBoolean`, `FloatingToBoolean`, `IntegralToFloating`,
    `FloatingToIntegral`, `FloatingCast`) is its operand's events and a
    computed scalar under `NoNumericClaim` (numeric transfer is CG20's);
    `PointerToBoolean` is the null test, the pointer's read and a truth
    value; `PointerToIntegral` is data only of a pointer read of an
    automatic local or formal the custody closure proves never bound
    (`untracked_pointer`), otherwise `PointerEscape` refuses by name;
    `IntegralToPointer` yields a pointer value with **no** identity; an
    object-pointer `BitCast`/`NoOp` is the same pointer value;
    `NullToPointer` the null pointer; `ToVoid` no value; the atomic
    conversions refuse as CG19's, `AddressSpaceConversion` and
    `ArrayToPointerDecay` as before, and an unlisted kind by name. Route G
    consults the elaborator for a refused statement, `return` or call-free
    condition holding an explicit cast, one of those implicit conversions or
    a type query, beside CG02's value forms. (2) **The null test is no
    escape** (a soundness repair asked *before* the older routes, whose
    premise that the value is the pointer is false): a `return`,
    declaration or assignment whose value Clang converts with
    `PointerToBoolean` is the elaborator's (its events, then a `NoOp` exit or
    a scalar write) and never escapes or binds the pointer; where the
    elaborator declines, the older answer stands only when the pointer's
    root is an automatic object the closure proves never bound, and
    otherwise the position refuses. (3) **Pointer destinations.** A refused
    `T *NAME = (U *) V;` or `NAME = (U *) V;` whose explicit casts are
    object-pointer `BitCast`/`NoOp` conversions is lowered as the uncast
    statement by the older routes, and only if they admit it. (4) **The
    cast return** (`CAST-RETURN-UNCOMMITTED-OPERAND`). `return (INTEGER)
    NAME;` the W44 rule leaves to the pre-card parse is refused with its
    refusal when Clang's kind is `PointerToIntegral` and `NAME` may be
    bound. (5) **Type queries** (E16). `query_operand` reads the class from
    Clang's node: an operand whose type is not variably modified is
    unevaluated and has **no event** (`sizeof(*p)`, `sizeof p->f`,
    `sizeof f()`); a `sizeof` of a variably modified type evaluates the size
    expressions Clang lists, each once, and must only read (a call, store or
    increment refuses); `_Alignof` of one, an evaluated expression operand
    other than a plain name and the other traits refuse. In an ordinary call
    statement a parenthesised `sizeof ( … )` whose every Clang node on its
    line is unevaluated joins W44's operand spans at the lexical gate; a
    parenthesised type-query operand is no call to the elaborator's
    call-free premise. (6) **Values.** No route decides a bound or a branch
    from a query's number: the marker `1` stays a class marker of the
    custody predicates (rule 5a), and a consumer that needs the number takes
    Clang's evaluated constant (`clang_constant`: the value Clang's
    evaluator gives a probe enumerator) or refuses; route T's enumerator
    values are Clang's (an explicit initializer's `ConstantExpr`, and since
    this rule an implicit enumerator's `clang_constant`, N11). **Do not** read a tracked
    pointer's integer as data, give an integer-to-pointer value an identity,
    escape or bind a pointer through its truth value, evaluate an operand C
    does not evaluate or evaluate one twice, use the marker in a bound or a
    branch, or consult rules (1), (3) and (5) before an older route of the
    position (hard invariant 3). Parity runs record the null test, the
    uncast statement and the integer return (`position:
    "cast-representation-…"`) and the cast/call census of
    `W45-CALLEE-CAST-LEXICAL-PREMISE`. Premises stated, not proved: faithful
    Clang extraction of cast kinds, types and constants and the token/AST
    node match (trust boundary 28), and C11 6.3.2.3p7 for an object-pointer
    conversion. `QCore.CastRepresentation` (no custody effect, the escape,
    no identity, the null test, the kept identity, unlisted refusal, the
    operand classes, single evaluation, consumed constants and the
    type-size fold), bound to `CAST_TABLE`, `query_operand`, Clang's
    constants and `fold_type_sizes` by shared-vector tests;
    [report](../reports/cg08-casts-and-type-queries/README.md).
    Amended by [rule 12h](#constraint-12h): the kind table is refined by
    type classes into one conversion table every route-G position consults.

<a id="constraint-11c"></a>

11c. **A field link that holds on some paths only never leaves its child's
    obligation unresolved without a diagnostic, and no settled exit that
    withholds an obligation is `Safe`.** Card
    B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED, a soundness repair
    (`field_link_join.rs`, the route-G adapter `legacy_field_link_join.rs`,
    thin hooks in `legacy_c.rs` (the field-link join, the store, the bound
    read, the chain view, the escape cascade, the work list and the settled
    exit pass) and `legacy_loop_generations.rs` (`rename`)). It amends rule
    5k's B08 join and escape cascade.

    (1) **The defect.** The B08 join recorded every child linked on one side
    only as an unresolved effect and nothing else: no diagnostic reported it,
    `report_leaks` skipped it, and the escape cascade escaped a live joined
    child anyway. `if (n) b->item = it; else k_release(it); k_drop(b);` was
    `Safe` although the linking arm leaks `it` (P-B08-SHALLOW), and
    `if (n) b->item = it; return b;` was `Safe` although the other arm leaks
    it; a must-linked child of mixed status was left unresolved silently at
    an escape. (2) **The join** keeps the union of the links and records each
    `(owner, field, child)` that does not hold on both sides as *partial*
    (`join_link`, `partial_links`, always a subset of `field_links`); it adds
    **no** unresolved effect, so an owner's release leaves a possibly-live
    child to be reported exactly as in the straight-line form. A store makes
    its link hold again; a generation shift renames partial triples with the
    links. (3) **The escape** decides each child it reaches (`escape_step`):
    an exactly-live child linked on every path escapes; one already escaped
    or untracked is kept; one live on no path (released, or contained and
    so its container's) is withdrawn as before (an unresolved effect, no
    report: this escape owes nothing for it); every other
    possibly-live child (not carried on every path, or of mixed status)
    refuses (`Unsupported`, "field-link escape: …"), keeping its status with
    an unresolved effect. (4) **Reads.** A bound read and a chain step
    through a partial link decline (the link no longer denotes one identity
    on every path). (5) **The floor.** A settled exit whose path no refusal
    reached (`unmodeled` false) and that withholds a possibly-live identity
    with an unresolved effect, not the caller's, reports a refusal
    (`exit_refuses`, "exit withholds a possibly-live obligation …"): any
    other rule that leaves an identity unresolved without reporting is a
    refusal here, never a `Safe` (and a W4 summary over it is not
    validated, which requires a body with no diagnostic).

    **Do not** record a one-sided link as an unresolved effect, escape a
    possibly-live child a partial link reaches, read through a partial link,
    or leave an identity unresolved on a path without a path-poisoning
    diagnostic (the floor then refuses the function). Premises stated, not
    proved: P-B08-SHALLOW (unchanged), that every rule that marks an
    identity unresolved reports a path-poisoning diagnostic on that path (the
    audit in the report lists each insertion and its report; the floor turns
    a violation on a modeled path into a refusal), and faithful extraction
    (trust boundary 28). Measured (corpus `66910c10`, system,
    tail-pause slice): no Safe result withheld an obligation at its exit
    (CG04's census) and no escape reached a possibly-live child that was
    one-sided or mixed (the census, parity `position: "field-link-escape"`),
    so nothing moved: every report is byte-identical (602 / 10 / 4,222,
    `2fc3af8c…`; 37 / 2 / 195; 26 / 1 / 242 / 1 Pending). The repair closes
    the class for the code widening cards admit; the fixtures were `Safe`
    on the base. The residual imprecision (a correct one-sided escape is
    refused) is `FIELD-LINK-ESCAPE-PATH-CORRELATION`. `QCore.FieldLinkJoin`
    (`a_decided_escape_leaves_no_live_child`,
    `an_escape_over_a_one_sided_live_child_refuses`,
    `the_join_adds_no_unresolved`, `a_live_child_at_exit_is_reported`,
    `an_unresolved_possibly_live_obligation_at_exit_is_never_safe`,
    `the_model_is_sound`, `the_base_flow_hides_the_leak`), bound to
    `LINK_VECTORS`, `ESCAPE_VECTORS`, `EXIT_VECTORS` and the gateway's
    verdicts on the pinned shapes (`SHAPE_VECTORS`) by shared-vector tests;
    [report](../reports/b08-field-link-join-silent-unresolved/README.md).

<a id="constraint-11d"></a>

11d. **A typed place's root the body may bind is read as an untracked,
    borrowed object only on a path where nothing bound it and nothing was
    refused before the access.** Card CG10-BOUND-UNTRACKED-ROOTS
    (`bound_untracked.rs`, the flow adapter `legacy_bound_untracked.rs`,
    additive items in `indirect_chain_access.rs` (`AccessChain::root_bound_untracked`,
    `AccessChain::root_untracked_only`, `ChainView::bound_untracked`), thin
    hooks in `typed_place.rs`, `typed_access_path.rs`,
    `typed_access_positions.rs` and `legacy_c.rs`). It amends rules 10t
    and 10k(3); the never-bound premise (rules 10k, 10l, 10o) is unchanged
    for every root. (1) **Eligibility** (static, from the Clang AST): the
    root of a typed-reader chain is a pointer declared exactly once in the
    function as a parameter or automatic local, every reference names that
    declaration, and no `&` of it appears anywhere (class (d), an address,
    stays refused). (2) **The premise** (`decide`, at the state entering the
    access): the root's binding is absent, or known, non-null and
    target-free in a state that widened nothing (a borrowed alias, a
    null-tested derived result of an untracked base); no place rooted at it
    is bound; and the state's `unmodeled` flag is clear: no path reaching
    the access passed a refused statement or an unresolved effect. Every
    custody-taking position that receives an object with no tracked
    identity reports there (`Consumed`/`Released`: the unknown identity;
    `Owned`/`Stored`/`Removed`: `require_live_resource`), and so does every
    call to an uncontracted function, so on every path reaching the access
    no custody position has received the root or any alias of it; a
    `Borrowed` actual reports nothing and preserves the premise. Where it
    holds, `chain_owner` reads the root exactly as a never-bound root
    (untracked; every link untracked); a root with a tracked identity keeps
    the tracked reading, and every other answer is unchanged. The premise is
    path-wide, not per root: an unrelated refusal ends it too (it frees no
    function the per-root form would, a refused path being `Unknown`, but
    lifts fewer leaves). (3) **Rule 10k(3)'s pointer actual.** A pointer read
    handed to a `Borrowed` or declared data-tail position whose roots are
    not never bound is left to the flow when each is the dereferenced root of
    the plan's eligible chains; the flow reads such a root only as untracked
    (`root_untracked_only`): a tracked root refuses there, so the
    pointer-actual premise is not relaxed to a tracked root. An address
    actual keeps rule 10n's never-bound premise. **Do not** serve the
    never-bound answer from this premise, read a root whose address is
    taken, read a possibly-null, widened, mixed or tracked binding as
    untracked, keep the premise past a refusal or unresolved effect on any
    reaching path, or admit a tracked root at a deferred actual. Premise
    P-CG10B-ABSENT-UNTRACKED, stated, not proved: an object reached through
    a name with no binding on any reaching path is disjoint from every
    identity the frame tracks (a contract's `None` result carries no
    custody; rule 10k's premise that a never-bound root reaches no tracked
    place, here per path). Premises stated, not proved: faithful Clang
    extraction of declarations and `&` operands (trust boundary 28).
    Measured (corpus `66910c10`, system, tail-pause slice, base `a235ead6`):
    **+6 / +4 / +2 Unknown → Safe**, no Safe → non-Safe, no leak or
    violation added; of 7,455 corpus root-refusal leaves on 1,635 functions,
    7,053 remain (path refused 5,126, a nullable or tracked binding 1,409,
    ineligible 505, rooted 13). Parity runs record every settled chain root
    the never-bound premise does not cover (`position:
    "bound-untracked-root"`) and the lexical closure's reasons
    (`"bound-untracked-closure"`). `QCore.BoundUntracked` (the decision, the
    interlock `the_never_bound_answer_is_unchanged` and
    `every_old_admission_stands`, per-hop sufficiency, the per-path event
    model: a `Borrowed` actual preserves, a custody actual and an
    uncontracted call end it on every later path, a `None` or null-tested
    derived result satisfies it, the join keeps a refusal), bound to
    `DECISION_VECTORS` and `EVENT_VECTORS`, whose paths the gateway runs
    (`tests/bound_untracked_roots.rs`);
    [report](../reports/cg10-bound-untracked-roots/README.md).

<a id="constraint-11e"></a>

11e. **A call's effect on bounds facts is its declared frame, never its
    custody mode: a read keeps, a write withdraws the local it receives, a
    move withdraws every fact on an object the source did not establish; a
    derived result is inside its base only on decided containment.** Card
    CG24-ALIAS-EXTENT-FRAME-VOCABULARY extends rule 11s's vocabulary
    (contract specification C09.1, grammar G05b; `qcore_core::index_extent_frames`,
    `qcore_contract::bounds_contract`'s `Frame`/`ReturnDerived` clauses and
    slot parameters, hooks in `crates/qcore-c-gateway/src/index_extent.rs`;
    no custody code, no hook in `legacy_c.rs` or `typed_source.rs`, and
    resource v1 still rejects both clause families). (1) **Frames.** A
    declaration that states `bounds_frame_reads`, `bounds_frame_writes` or
    `bounds_frame_moves` states one for every pointer and slot
    (`ptr<mut, ptr<mut, u8>>`) parameter (a closed frame; the callee touching
    nothing else and retaining nothing derived from a parameter beyond the
    call or its stated relation is its `Declared` premise). At the call, a
    read position keeps every fact; a write position withdraws the facts of
    the local whose address `&x` it receives and keeps extent facts on
    objects it only writes; a move position (the in-place reallocation, e.g.
    `cfl_sds_cat_safe`'s slot) also withdraws every fact on an object the
    source did not establish. No frame, a refuted declaration or an ABI
    mismatch is CG16's unstated call; `bounds_preserves_extents()` keeps its
    meaning. A held frame keeps its other facts only as held premises
    (Pending at most). (2) **Tracked locals.** A local whose every address
    is a direct actual (through parentheses and value-preserving pointer
    conversions) of a resolved closed frame position that is no derived base
    is tracked; its frame writes join the written sets of full expressions,
    loops, switches and the counted `for`. Any other address keeps it
    untracked (rule 11s(5)). (3) **Derived relations.**
    `bounds_return_derived(P, OFF, LEN, UNIT)` (several bases: multi-base) is
    used only after the call's preconditions are established: a single base
    whose extent decides `0 <= start` and `start + LEN <= N` makes the result
    that base at `start` (unless the call may move a non-source base); a base
    too small is `bounds-declaration-contradicted` and no fact; otherwise the
    declared view of `LEN` units when every relation states the same one.
    (4) **Refutation.** An analysed body refutes its frame by a live write
    through a parameter declared read (or a hand-off to a stronger frame
    position) while the parameter is unwritten, and its relation by
    returning a source object, a parameter that is no declared base, or a
    base at a constant element offset no relation states; a refuted
    declaration is withdrawn from the unit's callers (rule 11s(7)'s phase
    one). **Do not** read a custody mode as a frame, track a local whose
    address escapes elsewhere or reaches a held frame, keep a non-source fact
    across a move, alias a derived result into a base the call may move, or
    let a relation enlarge a base. Premises stated, not proved: faithful
    Clang extraction (trust boundary 28) and C06's `Declared` reading of a
    resolved frame or relation. Measured (corpus `66910c10`, system,
    tail-pause slice, base `6cab1ad9`, the bounds property selected on all
    three with no bounds declaration): custody and bounds artifacts
    **byte-identical** (corpus `ceefb6c4…` 645 / 10 / 4,179, bounds rows
    `5671d81d…` 162 Safe / 4,759 Unknown); the vocabulary reaches no function
    until bounds declarations are transported (CG16-BOUNDS-CONTRACT-TRANSPORT).
    `QCore.IndexExtentFrames` (`write_frame_withdraws_its_local`,
    `read_frame_preserves`, `write_frame_keeps_unrelated_extents`,
    `move_frame_withdraws_declared_facts`, `unstated_is_the_weakest`,
    `held_frame_holds_declared_facts`, `derived_view_within_base`,
    `derived_contradiction_sound`, `a_contradicted_base_refuses_the_view`,
    `a_refuted_frame_is_unstated`, `a_body_within_its_frame_never_refutes`),
    bound to `qcore_core::index_extent_frames` by `frameVectors`,
    `derivedVectors` and `refuteVectors`
    (`crates/qcore-core/tests/index_extent_frames.rs`) and to route G by
    `tests/cg24_frames_and_relations.rs`;
    [report](../reports/cg24-alias-extent-frame-vocabulary/README.md).

<a id="constraint-11f"></a>

11f. **A whole-record copy is read only for a record that holds no pointer; a
    braced initializer, a compound literal and a string initializer only where
    every value they store carries no identity; and a block compound literal's
    address is admitted only as a borrowed actual.** Card
    CG14-AGGREGATE-VALUES-AND-INITIALIZERS (`aggregate_value.rs`, the route-G
    adapter `legacy_aggregate_value.rs`, additive items in
    `typed_access_path.rs` (`Reader::aggregate_target`,
    `StatementIndex::aggregates`, the unit's `AggregateFacts`), thin hooks in
    `legacy_c.rs` (the last fallback of `expression_statement`) and `lib.rs`).
    One fallback, asked after every older route of a statement refused it
    with the braced-initializer, indirect-access or unrecognised-destination
    refusal, matched to one Clang node by line and exact tokens.
    (1) **Copies (M08).** `P = Q` and `T l = Q;` of record type are
    field-wise: a copied pointer member would be an alias of the source's
    value, never a second owner. The rule admits the copy only when the
    record is *custody-free* (`AggregateFacts::class_of` over the unit's
    `RecordDecl`/`TypedefDecl` facts: every member, transitively, a builtin
    scalar, an enumeration, or an array or record of them; unread,
    incomplete, ambiguous and `volatile`/`_Atomic` types refuse) and each side
    is a typed place or an automatic object of the frame: it is the source's
    typed read and the destination's typed store with the place's own per-hop
    obligations (W46/CG10's `Reader::path`, rule 10w for a union step, W43's
    frame for a store through `*`/`[]`). A record with a pointer member refuses
    by name (`pointer-members`): which copy owns is unstated, and the owned-field
    model makes a record with an owned field uncopyable. (2) **Initializers
    (M09).** A declaration of one automatic record or array local with a
    braced initializer is read from Clang's semantic `InitListExpr`
    (designators resolved; an unnamed member or element is its
    `ImplicitValueInitExpr` or array filler, the zero of its type, a zero
    pointer being null). Every leaf must carry no identity: a zero, a scalar
    the typed reader reads, the null constant, a string literal, a function
    designator, or a custody-free record read as in (1); an initial alias (a
    formal's or local's pointer, a pointer read, an address, a call, a
    nested compound literal) refuses by name. A local that can hold a pointer
    is lowered through rule 10v's flow (`PointerStore`, owner `None`, value
    `null`): it keeps its older refusal where a place rooted at it is bound
    with a target or unknown, every such place is forgotten after it, and the
    body publishes no derived summary. (3) **Compound literals (M10).** A
    literal is admitted only as a whole actual of a contracted call at a
    `Borrowed` formal, with an initializer as in (2) that reads no memory;
    the call is instantiated once (rule 10u) with the literal respelled `0`,
    since it carries no identity, and the borrow ends at the call's return,
    inside the literal's block. A custody-taking formal refuses
    (`literal-custody-position`); a literal returned, stored, bound to a
    pointer or nested in an actual keeps the gate's refusal. (4) **Strings
    (M11).** `char buf[N] = "…";` of an automatic character array is a
    declaration with no custody effect; its byte extent and terminator fact
    are recorded for CG16 (parity), and **no bound is claimed**. **Do not**
    admit a copy of a record with a pointer member without a contract or an
    owner premise stating which copy owns, read a value that may carry an
    identity into an initializer, let a block literal's address reach any
    position but a borrowed actual, claim a bound from a string extent,
    widen route T's initializer (no T consumer decides one), or consult this
    rule before an older route (hard invariant 3). Parity runs record every
    consultation (`position: "aggregate-value"`, with the form, the class,
    the leaves or sides and the refusal code). Premises stated, not proved:
    faithful Clang extraction of semantic initializer lists, record members,
    typedefs and types (trust boundary 28), and that the typed reader's
    scalar and place reads are the custody meaning of a leaf's evaluation
    (the W43/W46 rules). `QCore.AggregateValue` (`copy_adds_no_owner`,
    `a_copy_only_aliases`, `a_custody_free_copy_moves_no_identity`,
    `releasing_both_copies_is_a_double_release`,
    `the_owned_field_model_agrees`, `zero_fill_has_no_identity`,
    `an_admitted_initializer_holds_no_identity`,
    `a_literal_borrow_ends_in_its_block`, `a_literal_never_escapes`,
    `an_unlisted_form_refuses`), bound to `COPY_VECTORS`, `LEAF_VECTORS` and
    `LITERAL_VECTORS` by a shared-vector test;
    [report](../reports/cg14-aggregate-values-and-initializers/README.md).

<a id="constraint-11g"></a>

11g. **A generic selection is its selected association, and a statement
    expression is lowered only where nothing is unsequenced beside it, as its
    block's statements then its value.** Card
    CG07-GENERIC-AND-STATEMENT-EXPRESSIONS (`generic_association.rs`,
    `statement_expression.rs`, the route-G adapter
    `legacy_statement_expression.rs`, additive items in
    `legacy_generic_selection.rs` (route 2), `elaboration.rs` (two arms, a
    lifetime commutation arm, `Refusal::InnerEscape`, the `FORM_TABLE` rows
    `generic selection` and `statement expression` admitted and a new
    `statement expression control` row) and `typed_access_path.rs`
    (`original_line_texts`), thin hooks in `legacy_c.rs`). It amends rules
    10s (the two CG07 rows) and the W44 generic-selection route; every other
    premise of theirs stands. (1) **`_Generic`** (E17). Route 1 (selection
    invariance) is asked first and keeps every lowering it made. A selection
    it refuses as not invariant, or whose removed cast it cannot justify, is
    matched to one Clang `GenericSelectionExpr` by its line and its ordinal
    among that line's `_Generic` keywords (only where the line's tokens are
    the analysed source's and the counts agree), and its associations must
    be Clang's, kind for kind; it is then lowered to the association Clang
    marks `selected` (nested selections first), parenthesised unless a
    direct call or a name. The controlling expression and every other
    association are dropped: C evaluates neither, so neither yields an event
    or an obligation. The elaborator reads a `GenericSelectionExpr` as its
    selected association. (2) **Statement expressions** (E19), route G, a
    fallback after every older route of a statement or `return`. The full
    expression is matched to one Clang node by line and exact tokens and its
    statement expressions are read by `statement_expression::decide`
    (`admit`): the **position** is discarded (an expression statement's
    operand through parentheses, `__extension__`, `(void)` and commas), the
    whole initializer of a one-declarator declaration, the whole right
    operand of an assignment to a named object (`NAME`, or `*&NAME`), or the
    whole `return` operand; the **names** are hygienic (no inner name is a
    formal's, a declaration's outside every other statement expression, an
    enclosing or a second declaration's, or a referenced object's, function's
    or constant's declared outside them: the route has no block scopes);
    no inner declaration is `static`, `extern` or thread-local; and, where
    the value is used, the block takes no inner object's address (`&t`, an
    inner array's decay, a member or element of either) and ends in an
    expression. The block is then lowered **from the parser's own stream**:
    a discarded one as a block (each comma operand in order, a plain one a
    statement of its own), a used one as its statements before the last,
    then `T NAME = e`, `NAME = e` or `return e` through the position's own
    routes. An inner statement is read exactly as in an ordinary block and
    keeps its own refusal. The elaborator reads a straight-line block
    (declarations, expression and null statements) as its full expressions
    in order, each inner object's `LifetimeStart` at its declaration and
    `LifetimeEnd` after the value, the whole complete before the value is
    used; a lifetime event touches its own object only; any other statement
    refuses as `statement expression control`, a pointer value where the
    block takes an inner address as `InnerEscape`. Every other position
    (an operand, a call actual, a controlling expression, a cast's operand,
    a path destination) keeps its refusal. **Do not** evaluate a controlling
    expression or an unselected association, ask route 2 before route 1,
    lower a statement expression beside an unsequenced sibling, admit an
    inner name the flat lowering could confuse, let a used value carry an
    inner address, or consult the hook before an older route of the position
    (hard invariant 3). Parity runs record each consultation (`position:
    "statement-expression-statement"`/`"-return"`, each block's position,
    family, admission and refusal). Route T is unchanged (it names neither
    node). Premises stated, not proved: faithful Clang extraction of the
    selection, the block and its declarations and the token/AST node match
    (trust boundary 28). Measured (corpus `66910c10`, system, tail-pause slice,
    base `c374de96`): **no move on any route**; 1,414 corpus blocks lowered
    (1,327 glibc `assert`, 41 `RD_KAFKAP_STR_DUPA`, 39 typeof temporaries),
    braced leaves on their lines 1,338 → 69, the inner leaves exposed
    (1,232 `__assert_fail`, a no-return callee: `W43-LIBC-NORETURN`); no
    corpus `_Generic` needs route 2 (every one is glibc's invariant shape).
    `QCore.StatementExpressions` (selection: `select_events`,
    `control_no_event`, `unselected_no_event`; `value_is_final`,
    `lifetimes_end_at_block`, `lowering_exact`, `discarded_exact`,
    `returned_exact`, `flat_names_exact` with its counterexample
    `shadow_breaks_flat`, `admitted_no_escape`, the refusals and the
    position table), bound to `ADMISSION_VECTORS`, `POSITIONS` and
    `commutes` by shared-vector tests and to route G by
    `tests/cg07_statement_expressions.rs`;
    [report](../reports/cg07-generic-and-statement-expressions/README.md).

<a id="constraint-11h"></a>

11h. **Storage duration decides lifetime and custody: a static-duration
    declaration runs nothing, a static object's value carries no identity,
    and no value-derived fact is kept about a `volatile` object.** Card
    CG15-STORAGE-DURATION-AND-VOLATILE (`storage_duration.rs`, the route-G
    adapter `legacy_storage_duration.rs`, thin hooks in `legacy_c.rs` (the
    statement walker, the `return` routes, the built CFG),
    `legacy_call_argument_access.rs` (the typed actual),
    `typed_root_declarations.rs` and `typed_field_access.rs` (the two
    never-bound closures) and `typed_source.rs` (route T's refusal)). It
    names rule 10's publication as this card's escape and amends the
    never-bound closures (rules 10l, 10o) for static initializers.
    (1) **Storage.** An object's storage duration is Clang's (`storage_of`):
    automatic (no storage class or `register`, one frame), static (`static`,
    `extern`, file scope, the program) or thread (`_Thread_local`/`__thread`,
    its thread); a static- or thread-duration object outlives every frame of
    the function that names it. (2) **Static initialisation is no event** (a
    soundness repair, asked first): a statement Clang confirms is one
    `DeclStmt` whose every declarator has static or thread duration (a tag
    it defines runs nothing either) is lowered to nothing, whatever its
    constant initializer (C11 6.7.9p4), and neither never-bound closure
    counts that initializer as a write. Before this rule a braced one was
    refused and `static void *cache = 0;` was lowered as a store on **every**
    call, so `if (cache) free(cache);` was `Safe` by pruning an arm the
    second call takes. (3) **A static object's value carries no identity.**
    A read of one is a borrowed alias with no identity (a custody-taking use
    refuses, as rule 10k(3) and the unresolved alias already did); its
    address (an array of one decaying, or `&` of a place in one, reached by
    array subscripts and `.` members whose indices only read) is a pointer
    with no identity (`static_address`), admitted as a `return` value and as
    a borrowed or data-tail actual after every older route declined. (4)
    **A tracked identity stored into static storage escapes the frame to no
    named party** and is refused (rule 10, unchanged): never `Safe` and never
    a per-call leak; a function-local `static` holding an allocation across
    calls is this form. A registry contract with a release protocol would
    decide it; none exists (`CG15-STATIC-REGISTRY-CONTRACT`). (5) **The
    premise, stated** (`GATEWAY-VOLATILE-PREMISE`, beside 5a and 10a): the
    abstract state assumes that no object it holds a value-derived fact
    about is modified asynchronously. For a non-`volatile` object that is
    the no-undefined-behaviour reading (a data race and a signal handler's
    store to a non-`volatile sig_atomic_t` object are undefined), so it is
    `AOM-POISON-NO-UB`'s. For a `volatile` object it is false (C11 5.1.2.3p2),
    and the importer now reads the qualifier (`settle_volatile`, over the
    built CFG, in a unit that spells `volatile`): a node that binds a
    `volatile` object (a store into a static-storage one stays rule 10's), or
    hands one to a position that is not `Borrowed`, refuses; a known-value
    store to one is
    forgotten; its null, validity, distinctness and counted-loop tests
    refine nothing. Every value-derived fact the flow keeps (the census in
    the report: `refine_edge`'s null and validity pruning, the distinct-pair
    fact, collection and counted-loop facts, a non-nullable release's
    nullness, scalar facts) reads a binding keyed by a name or an element
    place, so none reads a `volatile` object named directly; a `volatile`
    member or element is refused by the typed-place, store and collection
    rules. (6) **A custody decision on a `volatile` read refuses**: a branch
    node whose condition Clang shows reading a `volatile` object, and whose
    successors' reaches differ in a custody-bearing node, refuses by name.
    This is policy, not what soundness rests on (a branch on an unbound value
    explores both arms). A branch whose arms carry no custody (a spin-wait on
    a flag) stays admitted. (7) **The observable trace.** A `volatile` access
    is an event of its own (`volatile_trace`, parity rows only): each access
    counted once, ordered only where C sequences it, an unsequenced pair on
    one object named and never ordered (C11 6.5p2). No verdict reads it.
    `volatile` is not `_Atomic` (CG19's). (8) **Route T** refuses a static-
    or thread-duration local by name: no typed-source consumer decides a
    lifetime longer than the frame. **Do not** lower a static-duration
    declaration as a store, count its initializer as a binding, give a
    static object's value or address an identity, model a store of an
    identity into static storage as a discharge, keep a binding, known value
    or refinement of a `volatile` object, or order two unsequenced accesses.
    Premises stated, not proved: faithful Clang extraction of storage
    classes, qualifiers, node lines and call and return nodes (trust boundary
    28), and that a branch's decided region (the nodes one successor reaches
    and another does not) holds every node the branch decides. Measured
    (corpus `66910c10`, system, tail-pause slice, base `c374de96`): **+7 /
    +1 / 0 Unknown → Safe**, no Safe → non-Safe, no leak or violation added
    (the five librdkafka name tables, `rd_kafka_broker_name`'s thread-local
    buffer, `LZ4F_getErrorName`'s static pointer; system
    `flb_engine_adaptive_flush_interval`); M24's 132 corpus leaves → 52; one
    corpus function holds a `volatile` object (`rd_kafka_desensitize_str`,
    two refusals of this rule in place of four older ones) and none decides
    custody on a `volatile` read. The two repairs have no corpus instance.
    `QCore.StorageDuration` (lifetimes, the declaration's no event and the
    per-call misreading, the escape refused and never a leak, no identity
    from a static read, the environment's volatile value, the refused and
    the sound neutral volatile branch, the trace's count and order, the
    table and unlisted refusal), bound to `FORM_TABLE` and the gateway's
    verdicts on the pinned shapes (`SHAPE_VECTORS`) by shared-vector tests
    (`tests/storage_duration_volatile.rs`);
    [report](../reports/cg15-storage-duration-and-volatile/README.md).

<a id="constraint-11i"></a>

11i. **A C atomic reaches the weak-memory model only as a plain load or
    store with an order the model represents; every other atomic is refused
    by name, and nothing a route admits claims ordering beyond one checked
    execution.** Card CG19-ATOMIC-SOURCE-ROUTE (`atomic_source_route.rs`, the
    route-G adapter `legacy_atomic_route.rs`, additive
    `qcore_core::event_order_represented`, thin hooks in `legacy_c.rs` (the
    end of `expression_statement` and of the `return` chain) and
    `typed_source.rs` (`qual_type`, `extended_expression`), the
    atomic-publication extractor, `qcore_verification::atomic_publication_verdict`
    and `quirin-audit atomic-publication`). (1) **The route.**
    `ATOMIC_FORM_TABLE` (51 builtins) classifies `AtomicExpr` and the builtin
    fence/`__sync` calls; `<stdatomic.h>` reaches it through Clang's
    `__c11_atomic_*` lowering. The `_Atomic` operator forms are an lvalue
    conversion (`AtomicToNonAtomic`: a seq_cst load), a simple assignment
    (`NonAtomicToAtomic` value: a seq_cst store), and a compound assignment or
    increment (a read-modify-write). A plain load, store or `__c11_atomic_init`
    maps to `AtomicLoad`/`AtomicStore`/`AtomicInit` with Clang's numeric order
    decoded (0–5) and admitted per C11 7.17.7 (a load relaxed, acquire or
    seq_cst; a store relaxed, release or seq_cst); read-modify-writes,
    compare-exchange, fences, a consume load (by value or by the enumerator
    `memory_order_consume`), an invalid or unevaluated order, a generic
    by-pointer form, a pointer or aggregate value and every unlisted builtin
    of the families refuse by name, CG28 owning the unrepresented events.
    **Amended by [rule 11t](#constraint-11t)** (CG28-WEAK-MEMORY-EVENTS):
    read-modify-writes, compare-exchanges and fences are now routed events;
    the consume, order, by-pointer, value and unlisted refusals stand.
    (2) **Route G.** Asked last, only where the classifier refused: an
    uncontracted atomic builtin's statement or `return`, and an `_Atomic`
    store or read-modify-write statement. A routed plain access is its
    custody effect, that of the plain access: the typed reader's dereference
    of a pointer object (`*p`, the pointee possibly `_Atomic(T)`), read with
    the value type, as `IndirectScalarAccess` (a store with the refused-write
    frame); `&x` reads nothing and stores forget a scalar local; a loaded
    value's scalar destination forgets its fact; a returned scalar escapes
    nothing. A stored value must be a literal, an enumerator or a non-pointer
    named object. Every other answer replaces the refusal's text with the
    route's named refusal and keeps its writes. G's verdict is custody
    only: the order goes to the parity row, never into a claim. (3) **Route
    T** names an `_Atomic` type in its own arm, asked before the unchanged
    `volatile`/`restrict` arm, and an atomic builtin by the route's answer; no
    T consumer reads a routed access. (4) **Publication.** The extractor reads
    the flag's store and load through the route (`__c11_atomic_*`, the
    generics, the `_Atomic` forms, and GNU `__atomic_*` builtins on a
    non-`_Atomic` `int` flag both of whose accesses are one, with the stated
    assumption that those builtins are atomic operations), accepts a literal
    or canonical-enumerator order, a `seen != READY` early return or a
    `seen == READY` guarded use, and records the initialise → publish →
    observe → use stages; reclamation is not derived (CG29).
    `atomic_publication_verdict` is `Safe` exactly when extraction and the
    core model accept; any refusal is `Unknown`; both carry
    `SINGLE_GRAPH_ASSUMPTION` (one candidate execution, not race freedom over
    every execution, W07/CG29; no liveness, W11). **Amended by [rule
    11z](#constraint-11z)** (CG29-EXECUTION-COVERAGE-AND-RECLAMATION): `Safe`
    also needs the extracted shape race-free over every candidate execution
    within stated bounds, and then carries `ALL_EXECUTIONS_ASSUMPTION`
    instead; `Unknown` keeps the single-graph assumption. `quirin-audit
    atomic-publication CONFIG.json` is its production caller, writing the
    verdict contract's fields and exiting non-zero on `Unknown`. **Do not**
    read a routed access's order as an ordering or race-freedom claim of
    route G, admit a read-modify-write, compare-exchange, fence or consume
    load as a neutral call, strengthen consume to acquire, evaluate an
    enumerator order from the function's AST alone, or publish a publication
    verdict without the single-graph assumption. Premises stated, not
    proved: faithful Clang extraction of the builtin name, operands and order
    (trust boundary 28); the Rust-to-Lean correspondence by review. Measured
    (corpus `66910c10`): see the report. `QCore.AtomicSourceRoute` (the form
    table, `routed_plain_agrees_with_model`, the per-class refusals,
    `consume_load_refused`, `order_one_never_routed`, `unlisted_refused`,
    agreement with `QCore.AtomicPublication` on Clang's numbering, the
    publication verdict's single-graph assumption and gate), bound to
    `ATOMIC_FORM_TABLE`, `decode_order`, `route_order` and
    `event_order_represented` by `tests/unit/atomic_source_route.rs`;
    [report](../reports/cg19-atomic-source-route/README.md).

<a id="constraint-11j"></a>

11j. **A `void *` value is viewed only through an object-pointer bit-cast
    where a pointer is read through, and a `void *` member is read back as
    the identity an admitted store linked only where the body alone writes
    it and every view names the stored type.** Card CG12-VOID-VIEW-READBACK
    (`void_view.rs`, the route-G adapter `legacy_void_view.rs`, additive
    items in `typed_access_path.rs` (`StatementPlan::readback`, the view step
    of `Reader::path`, the explicit view at a pointer destination) and
    `legacy_c.rs` (`IndirectScalarAccess::readback`, a hook in
    `typed_path_statement` and in the access's transfer)). It amends rule
    10m (an explicit cast and a cast inside a path) and card ALIAS-AGGREGATE's
    store-only `void *` member; `bound_read_actuals` still refuses the member
    at every call actual. (1) **The view.** An explicit cast whose Clang kind
    is `BitCast`, from `void *` (`const`/`restrict` allowed) to an object
    pointer, is the same pointer with the target's pointee type (C11
    6.3.2.3p1/p7; CG08's `CAST_TABLE` row, no custody effect). In a typed path
    it may stand only as the pointer a `*`, `->` or `[]` reads through, and
    only over an untracked object: over the root, whose chain is then read
    only as untracked (`root_untracked_only`: never bound, or rule 11d's
    premise at the node; a tracked identity's object has its contract's type,
    which the view does not check), or over a pointer loaded from a place of
    a root the closure proves never bound (rule 10k's premise: every such
    load is outside the tracked graph). A view over a load from any other
    root (`view-over-link`), a view of a root as `struct TAG *` where the body
    declares another name of that C02-P-ALIAS class, a cast from another
    object pointer (a pun, M22 and CG18), to a function or `void` pointer, a
    `NoOp` or any other kind, and a view over a call result or arithmetic keep
    their refusal. At a pointer
    destination `T *c = (T *) E;` / `c = (T *) E;` the explicit view to the
    destination's own pointee type is read as rule 10m's implicit one.
    (2) **The readback.** A pointer destination (an automatic local) whose
    value is a read of a `void *` member `R->F` (directly, through rule 10m's
    implicit bit-cast or through an explicit view) takes the member's
    identity, instead of P2's target-free borrowed alias, only when:
    `R` is an admitted store base and `F` its `void *` member (premise 1);
    every stored value of known type and every view of `R->F` in the body
    name one pointee type (premise 2; otherwise the statement is **refused**,
    "void * member read back as a type other than the one stored (type
    punning) …"); every reference to `R` is a member access (never `&R->G`),
    its own assignment, a null test or a whole `Released`/`Consumed` actual
    of a contracted call (premise 3); every occurrence of `R->F` is a simple
    store destination or a read (premise 4); and, at the node,
    `bound_read_resolves` holds (premise 5: the owner one live identity this
    body acquired, the binding exactly the field link, on every path, its
    child live with no unresolved effect). A refused statement or an
    uncontracted call poisons the binding and a later admitted store replaces
    the link, so the value read is the last store's identity or has none.
    Where premise 1, 3, 4 or 5 fails the destination is the borrowed alias it
    was, never a guessed identity. **Do not** admit a view outside a
    read-through position, over a load of a root the closure does not prove
    never bound, or from a type other than `void *`; bind a readback whose
    owner reaches any callee but its release, whose member is addressed or
    whose views disagree; or consult either rule before an older route of the
    position (hard invariant 3). Parity runs record each static decision
    (`position: "void-view-readback"`) and each flow decision
    (`"void-view-readback-flow"`). Premises stated, not proved: faithful Clang
    extraction of cast kinds, types and declaration and field ids (trust
    boundary 28), C11 6.3.2.3p7, and **P-CG12-VIEW-EFFECTIVE-TYPE** (an access
    through a view of a root or of a never-bound load reads the object at its
    effective type, C11 6.5p7; the view is checked against a stored type only
    by premise 2). Measured (corpus `66910c10`, system, tail-pause slice,
    base `c374de96`): **+2 / +2 / 0 Unknown → Safe** (`LZ4_write16`,
    `LZ4_write32`; `sb_get_context` and its caller), and on main `e93dbe4e`
    +3 / +2 / 0 (with `LZ4_writeLE16`, jointly with CG14), no Safe → non-Safe, no
    leak or violation added; 18 of CG00's 76 M05 leaves lifted; 116 corpus
    readback sites, none bound to an identity (115 with no store base,
    `CG12-READBACK-NO-STORE-BASE`). `QCore.VoidViewReadback` (the view decision and position, the readback
    decision with `a_type_mismatched_view_is_refused`, the event model with
    `a_readback_returns_the_stored_identity`,
    `an_intervening_unknown_store_yields_no_identity`,
    `a_partial_link_yields_no_identity`,
    `a_released_child_is_never_read_back` and
    `a_readback_returns_only_a_stored_identity`), bound to `VIEW_VECTORS`
    (read by Clang), `POSITION_VECTORS`, `DECISION_VECTORS` and
    `EVENT_VECTORS`, whose bodies the gateway runs
    (`tests/cg12_void_view_readback.rs`);
    [report](../reports/cg12-void-view-readback/README.md).

<a id="constraint-11k"></a>

11k. **Two places are disjoint only by the core footprint model's decision:
    two members of one struct object, from Clang's layout; nothing across two
    objects; every footprint premise named where it is taken.** Card
    CG18-FOOTPRINT-CONNECTION (`footprint_connection.rs`, additive items in
    `elaboration.rs` (`Context::disjoint_places`, default declining, and
    `EvalGraph::check_with`, of which `check` is the declining case), a thin
    hook in `legacy_elaboration.rs`; the core's `disjoint_footprint.rs` and
    new `footprint_mutex.rs`, and `qcore_frontend::source_mutex_footprint`).
    It amends rule 10s's path-pair refusal and rule 10w's (3) (CG13 left
    every field pair refused for this card); every other refusal of theirs
    stands. (1) **Member pairs** (M20/M21). An unordered write/write or
    write/read pair of two *path* places is admitted when both lie, by their
    own structure (only `.` member steps and parentheses above the step), in
    two different members of one `struct` object whose base expression is the
    same on both sides **structurally** (Clang kinds, declarations, member
    steps, literal values; never the place spelling, which erases indices),
    and the core model admits `AssertDisjoint` on the two members' field
    handles of one allocation generation (the struct read from the members'
    own declarations, never from a type spelling a typedef hides) under a
    two-member layout contract
    read from Clang's `-fdump-record-layouts-complete` (an ordinary member
    spans to the next member's first byte, a superset; a bit-field spans the
    bytes of its bits). A union member, an anonymous member, an unnamed or
    unread record, a subscript, `*` or `->` above the step, one member on
    both sides, two bit-fields of one C11 unit (CG13's `Q661`) and two
    different base objects refuse. (2) **Across objects** (M21). No route
    assumes two allocation identities disjoint: the core refuses them
    (`Q662`) unless both live generations were declared fresh
    (`DeclareFreshAllocation`), and then its certificate names
    `P-DISTINCT-ALLOCATIONS-DISJOINT`; route G takes no allocator premise,
    so `s->a = t->b++` stays refused. (3) **Types** (M22). A core view is a
    character view (no premise), an effective-type view
    (`P-EFFECTIVE-TYPE`) or a punned view, refused as `TypePunnedAlias`
    (`Q667`), as CG13 refuses a punned pointer read. Route G reads a punned
    scalar load as custody-neutral under constraint 10a's no-undefined-
    behaviour reading and claims no effective-type result; route T keeps
    refusing `restrict`, and no route derives disjointness from it
    (`P-RESTRICT` is relied on nowhere). (4) **Bytes** (M26). The core's
    `ByteCopy`/`ByteFill` check `memcpy`/`memmove`/`memset` ranges (inside
    both handles, `Q653`; a `memcpy` whose ranges share a byte stays the
    excluded `OverlappingCopy`, `Q667`), for a property that claims byte
    safety only; every custody contract of these calls is unchanged, and
    each admitted transfer names `P-BYTE-REPRESENTATION`: its bytes are read
    back only through an lvalue they validly represent. That is the premise
    `W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE` names for a
    character-typed store, the same one the Borrowed `memset`/`memcpy`
    declarations rest on (W43-LIBC-RESULT-ALIAS), under "the ordinary C
    object model under the no-undefined-behaviour reading" of rule 10a
    (`AOM-POISON-NO-UB`). (5) **Generations** (M20). Rule 11a's current and
    previous identities of a site are two core allocations, its older
    summary has none, and a reallocation is the core's `Reallocate` (every
    earlier handle stale, `Q652`, as the flow's `UnknownAlias`); the
    correspondence is checked by a replay against the core checker, and no
    liveness check of the flow changes. (6) **Mutex invariants** (W08).
    `compose_footprints_with_mutex_invariant` admits a checked mutex trace
    only when every interval it names is exactly a live derived handle's
    export (else `Q665`) and every pair of two resources has both declared
    fresh (else `Q662`); it then drops `DeclaredFootprintShape` for the
    footprint premises. `source.mutex.guard.v1` reaches it without changing
    its own certificate; no C route has mutex-invariant evidence, so a C
    field update claims no lock invariant. **Do not** admit a pair by its
    spelling, through a subscript, a dereference or `->` above the diverging
    member, across two base objects, or through a union member; do not treat
    two allocation identities as disjoint without both freshness
    declarations, admit a punned view, drop `OverlappingCopy`, or drop
    `DeclaredFootprintShape` while any interval stays declared; do not
    consult the hook before an older route of the position (hard invariant
    3). Parity runs record each consultation (`position:
    "footprint-member-pair"`, the pair and its decision). Premises stated,
    not proved: faithful Clang extraction of member steps, record types and
    layouts (trust boundary 28), and the three named core premises where a
    certificate carries them. Measured (corpus `66910c10`, system, tail-pause
    slice, base `b59aaae2`): **no function moved on any route** (608 / 10 /
    4,216, `8b9e4047…` → `41c7cefd…`; system and slice byte-identical); 5
    member pairs admitted in 2 corpus functions (`mk_http_parser`,
    `rd_kafka_mock_partition_pidstate_get`, whose statements now reach the
    flow as `unknown_alias` leaves), 5 cross-object pairs refused
    (`CG18-CROSS-OBJECT-PATH-PAIRS`); no new premise in a published verdict.
    Re-measured on main `436e0ee7` (625 / 10 / 4,199, `ac2d4e19…` →
    `8391ba75…`; system 42 / 2 / 190 and slice 29 / 1 / 239 / 1 Pending
    byte-identical): the same pairs, functions and leaves.
    `QCore.DisjointFootprint`'s CG18
    section (`without_the_premise_nothing_changes`,
    `cross_allocation_disjointness_needs_the_premise`,
    `an_admitted_judgement_is_grounded_or_premised`,
    `a_punned_view_is_refused`, `an_admitted_view_names_its_premise`,
    `an_admitted_memcpy_shares_no_byte`,
    `a_cross_allocation_memcpy_needs_the_premise`,
    `composition_backs_every_mutex_disjointness`,
    `two_generations_of_a_site_are_not_assumed_disjoint`,
    `the_previous_generation_is_the_former_current`,
    `the_summary_has_no_footprint`), bound to `PAIR_VECTORS`,
    `COPY_VECTORS` and the replay's generation vectors by shared-vector
    tests and to route G by `tests/footprint_connection.rs`;
    [report](../reports/cg18-footprint-connection/README.md).

<a id="constraint-11l"></a>

11l. **Route T reads the control statements, lowers an acyclic form only
    where the importing consumer lists it, ends every left scope's locals on
    every path, and refuses every back-edge by name.** Card
    CG09-TYPED-SOURCE-CONTROL (`crates/qcore-c-gateway/src/typed_source_control.rs`,
    a child module of `typed_source.rs`, with thin hooks there: three `Stmt`
    variants, the `Path::jump` field, the block-entry, `do … while (0)` and
    derive hooks, and the `import_…_admitting_control` entry points). It amends
    rule 10r only by adding a second, independent admission.
    (1) **Reading.** The importer reads `switch` / `case` / `default` (with
    fallthrough), `break`, `continue`, forward `goto` and labels, `while`, and
    a `for` or `do` outside its legacy shape (a canonical scan; a literal zero
    condition). Every other statement keeps the generic refusal.
    (2) **Admission.** Every importer call passes a `ControlAdmission`,
    validated against `CONTROL_TABLE` before any function is read. A form
    the consumer does not list is refused by name, and the legacy entry points
    list nothing.
    (3) **Back-edges.** A back-edge form (a general loop, or a `goto` to a
    label already read) cannot be listed. Route T checks no invariant or
    summary (card CG27), so every consumer refuses one by name. A `for` that
    is not a canonical scan names the scan shape's own reason. `continue`
    is acyclic because only `do … while (0)` can take it: every loop is refused
    at its own node first.
    (4) **Lowering.** A `switch` evaluates its selector once. Its promoted
    integer type comes from Clang, and each case label is Clang's constant
    under Clang's own conversion, never evaluated on the host. Each label is
    an entry path:
    - a `case` carries `selector == constant` taken;
    - `default` carries every case guard not taken;
    - with no `default`, one more path carries every case guard not taken.

    From its entry, a path runs every later statement (fallthrough) until a
    `break`, which the `switch` takes. `do … while (0)` takes `break` before
    its condition, and takes `continue` into it. A forward `goto` skips
    statements until a block resumes it at its label.
    (5) **Refused shapes.** Each is refused by name:
    - a `goto` whose label no enclosing block reaches later (a jump into an
      `if`, a loop or a `switch`);
    - a `goto` that skips a declaration into its own scope;
    - a declaration or an ordinary label at a switch body's top level;
    - a case label anywhere else;
    - a GNU case range;
    - a selector that is not a promoted integer.
    (6) **Lifetimes.** Every path that leaves a block, by its end, by `return`
    or by a jump, ends that block's locals, innermost first. Under a
    consumer that lists any control form, a use of a local's storage after
    its `LocalEnd` is refused on every path, jump or not. The legacy
    admission keeps T's pre-CG09 projection of ended storage, which the
    Monkey callback tests pin; the legacy consumer decides it.
    (7) **Budgets.** The budgets are unchanged: 64 paths, 512 operations,
    depth 64 and 2,048 nodes. Exceeding one is Unknown.
    **Consumers.** The Monkey wrapper projection lists the four acyclic forms
    (`MONKEY_WRAPPER_CONTROL`); it certifies nothing. The other 19 production
    import sites keep `ControlAdmission::LEGACY`. **Do not** list a form for a
    consumer without reading every reader of its summaries
    (`CG01-T-CONSUMER-ADMISSIONS`). Do not admit a back-edge without a checked
    invariant or summary. Do not evaluate a case label on the host.
    `QCore.TypedSourceControl` (the table and admission, `back_edge_refused`,
    `unlisted_statement_refused`, `jump_out_ends_locals`,
    `jump_then_use_refused`, `fallthrough_composes`, `switch_vectors_hold`,
    `exhausted_never_safe`) is bound to `CONTROL_TABLE`, `LEGACY_STATEMENTS`,
    `CONTROL_STATEMENTS`, the four budgets and the importer's own `switch`
    lowering by a shared-vector test;
    [report](../reports/cg09-typed-source-control/README.md).

<a id="constraint-11m"></a>

11m. **A value region beside a sibling with a call is admitted only when
    CG05's contract footprints prove every unordered pair commutes, on each
    arm; the order premise is never discharged by choosing an order.** Card
    CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part
    (`connective_footprints.rs`; additive items in `nested_call_effects.rs`
    (`Operands`: CG05's reading of a whole operand into one footprinted
    graph, its `check` being `EvalGraph::check_footprints` unchanged) and
    `value_connective.rs` (`admit_composed`, `Reading::compose`,
    `Refusal::Composition`, `edge_root`/`with_edge_root`); thin hooks in
    `legacy_value_connective.rs` (the composer over `nested_facts`, the edge's
    root) and `legacy_nested_call.rs` (`nested_facts`/`GFacts` `pub(super)`,
    and the edge-root fallback)). It amends rule 10z's (3) only; every other
    refusal of 10z and 10y stands. (1) **Where.** Only where rule 10z's
    `admit` refuses a sibling with an effect or a deciding operand with an
    effect beside a sibling that is not inert. An omitted-middle operand with
    an effect stays refused. (2) **The composition.** For each arm
    separately, one graph: the deciding operand's events, `Branch`, that
    arm's events, `Merge` (the edges C guarantees inside the region), and
    every sibling's events, unordered with the region's and with each other's
    (no order is chosen between them). Every operand is read by CG05's own
    reading (each call at any depth with its contract footprint, each named
    read with its cells). Admitted only when `check_footprints` admits both
    arms' graphs; the two arms are never in one graph. (3) **Refused by
    name.** A sibling, deciding operand or arm with an effect that is not a
    call (a store, an increment, a statement expression, a volatile access:
    invisible to the footprints); a sibling call with no contract in force
    (no footprint); an operand CG05's reading refuses; a deciding operand
    reading an object whose address a sibling's call takes, or whose call
    takes the address of an object a sibling reads (CG05 gives a scalar's
    storage no cell, but the deciding value steers the branch: an extra,
    refusing-only premise); every pair whose footprints do not commute. Two
    names are never distinct identities (CG05's `cells_overlap`), so a
    sibling releasing any named identity refuses beside a region that reads
    one; only a fresh owned result is provably distinct. (4) **Edges.** On an
    edge of rule 10z's lowering, where no source node carries the edge's
    tokens, rule 10y's two consumers read the edge's own node (the source
    node with each region replaced by its arm, the node rule 10z's
    `leaf_events` reads); each edge is still lowered only by the position's
    older routes, and any refused edge declines the whole expression byte for
    byte. **Do not** serialise a sibling before or after the region, pair
    the two arms, treat two names as distinct identities, compose an
    uncontracted or non-call effect, or consult this rule before an older
    route (hard invariant 3). Parity rows name the composition's outcome
    (`…; composed with CG05's footprints: …`). Premises stated, not proved:
    faithful Clang extraction (trust boundary 28), CG05's (a call's effect on
    the abstract state is its contract's, constraint 4), and rule 10z's (an
    edge's older route decides its statement as written in source).
    Measured on main `0a22a998`: of rule 10z's 1,137 order refusals, 987
    compose (985 sibling, 2 deciding), 150 stay refused (143 an uncontracted
    sibling call, 3 overlapping footprints, 2 an uncontracted deciding call,
    1 a sibling increment, 1 an indirect sibling call). Of the 987, 2 are
    lowered (two cmetrics `snprintf` statements, one indirect-access leaf
    gone each), 976 then refuse on an edge an older route refuses (808
    statements, 799 `rd_kafka_log0(&rk->rk_conf, …)`; the first refusal on
    the line is the lexical indirect-access gate for 757 and `sizeof` for
    48), 5 exceed 8 leaves, 4 have a deciding operand no condition route
    decides. The edge-node fallback moves none of the 411 older edge
    refusals. **No function moves** on any route (corpus 628 / 10 / 4,196;
    system and tail-pause slice byte-identical).
    `QCore.ValueConnectives`' composition section
    (`composed_region_runs_alike_on_each_arm`, `decision_value_stable`,
    `an_overlapping_sibling_is_refused`, `an_uncontracted_sibling_is_refused`,
    `a_non_call_sibling_is_refused`, `composition_extends_admit`,
    `uncomposed_is_admit`, `composition_never_rereads`, reusing
    `QCore.NestedCallEffects.every_interleaving_runs_alike` and
    `footprint_commute_sound`), bound to `COMPOSED_VECTORS` and
    `FOOTPRINT_VECTORS` by shared-vector tests and to route G by
    `tests/cg06_connective_footprints.rs`;
    [report](../reports/cg06-connective-footprints/README.md).

<a id="constraint-11n"></a>

11n. **An elaborated call event carries its contract footprint, an unordered
    pair holding a call commutes only by footprints, and the call is applied
    once through the one application point; an address-taken automatic stays
    reachable by every identity.** Card CG05-ELABORATOR-CALL-FOOTPRINTS
    (`crates/qcore-c-gateway/src/legacy_elaborator_calls.rs`, additive items
    in `elaboration.rs` (`Context::call_footprint`, default declining;
    `event_footprint`; `EvalGraph::check_with_footprints`;
    `Refusal::CallLowering`; `Elaboration::{call_footprints, call_nodes}`),
    `nested_call_effects::call_footprint`,
    `parenthesised_callee::strip_callee_parentheses`, thin hooks in
    `legacy_elaboration.rs` and `legacy_c.rs`). It amends rule 10s's "a call
    unordered with any event refuses" and "do not lower a call inside a
    piece's events"; every other refusal of 10s, 10y and 11k stands.
    (1) **Footprints.** In route G's statement route only (the whole-statement
    check, a sequenced piece, CG03's effectful piece), each call event's
    footprint is read exactly as rule 10y reads a call of its tree: the
    contract in force, an identity at a `Borrowed` formal read and at any other
    formal written, `&x` and an array decay a write of the object, an owned
    result a written fresh identity keyed to the call's event. A callee with
    no contract in force, an indirect callee, a container effect, a
    reallocated result or an arity mismatch has none. Every other event's
    footprint is `event_footprint`'s: a named object touches its storage, a
    pointer value read or stored also an identity, a path its root object and,
    through a pointer or for a pointer value, an identity; a lifetime event
    writes its object; a control event touches nothing. (2) **Order.**
    `elaborate` asks `check_with_footprints`: a pair `commutes` leaves as a
    call pair commutes only when both events carry a footprint and the
    footprints commute (`footprints_commute`); every other pair is decided
    exactly as CG18's `check_with`, in the same pair order, and with no call
    footprint the map is empty and the check is `check_with`. A scalar local
    whose address is never taken is disjoint from every identity, so `x +=
    f(p)` is admitted with `f` releasing `p`; `*q += f(q)` and a call with no
    footprint stay refused. (3) **Lowering.** A refused piece whose graph
    holds calls is lowered in the graph's event order: each read or write as
    rule 10s lowers it, each call by `Parser::instantiate_call` with use
    `Tested` (it must own nothing), its scalar actuals at `Borrowed` formals
    data (their reads lowered as events) and its pointer actuals the
    classifier's substitution; the contract applied must be the one the
    footprint was read from, and a call whose actuals carry effects stays
    CG03's. (4) **Aliasing.** `aliasing` counts as taking an object's address
    `&x`, the address of its member or element (`&s.m`, `&a[i]`) and an
    array's decay other than a subscript base, rooted through `.` members and
    array subscripts. (5) **The address boundary** (CG05-FOOTPRINT-ADDRESS-
    CONSERVATISM, option (c)). An identity read by one operand
    (`fprintf(stderr, …)`) is never proved distinct from an automatic whose
    address another operand passes (`rd_kafka_msgq_len(&q)`): a `Borrowed`
    formal is not a lifetime-checked borrow (contract spec §C04.2), so the
    callee may retain the address, and no route has an escape analysis for
    it. No premise is taken; the 25 consultations over 14 functions stay
    refused as a measured boundary. (6) **Matching.** A statement whose
    callee the token stream normalised (`(f)(x)` to `f(x)`, rule 10c) is
    matched to its Clang node with the callee parentheses stripped the same
    way when no exact match exists. **Do not** read a footprint for a callee
    with no contract in force, treat two names as distinct identities, take an
    automatic's address as unreachable because it is passed only at
    `Borrowed` formals, lower a call outside the application point, or
    consult the hook before an older route of the position (hard invariant
    3). Premises stated, not proved: faithful Clang extraction and node
    matching (trust boundary 28), and that a call's effect on the abstract
    state is its contract's (constraint 4), as for a call standing alone; no
    new premise. Measured (corpus `66910c10`, system, tail-pause slice, base
    `d4df096f` rebased on main `cfb5d8e1`): corpus 628 / 10 / 4,196 →
    **629 / 10 / 4,195** (`f9b3420e…` → `3d53a5d0…`; the same attribution on
    the base and on `0a22a998`), one Unknown → Safe move
    (`rd_kafkap_leader_discovery_tmpabuf_add_alloc_topic`, `(tbuf)->size +=
    (strlen(topic_name) + 1 …)`: a read/read pair), 18 elaborator statements
    admitted over 9 functions, 9 more refused by the application point (an
    owned result in a consumed value); no leak, violation or Safe → non-Safe
    move. The CG05-ELABORATOR-CALL-FOOTPRINTS population is 621 rows over 411
    functions at this base (the ledger's 98 / 33 is its compound family, now
    118 / 39), 603 at the tip, every one a call with no footprint (515 with
    the callee's own `call to uncontracted function` leaf). System and slice
    move no status: 2 functions on each lose an `indirect memory access` leaf
    at `x->m = time(NULL)` (system both +1 `unknown_alias`; slice
    `flb_tail_file_rotated` +1, `flb_tail_fs_stat_add` −2). `QCore.ElaboratorCallFootprints`
    (`a_footprintless_call_is_refused`, `empty_map_is_check_with`,
    `a_scalar_destination_read_commutes_with_a_call`,
    `a_disjoint_read_observes_alike`, `the_compound_assignment_runs_alike`,
    `a_read_through_the_written_identity_is_refused`,
    `an_address_taken_destination_is_refused`,
    `two_identity_writes_are_refused`, `the_address_boundary_is_necessary`
    with `aliased_den_meets_the_premises`), bound to
    `check_with_footprints` and `event_footprint` by `footprintVectors`;
    `QCore.NestedCallEffects.footprint_commute_sound` carries the pair to
    the state; [report](../reports/cg05-elaborator-call-footprints/README.md).

<a id="constraint-11o"></a>

11o. **A call statement ends its path only where a terminating exit is
    declared and its Clang declaration is `noreturn`; non-local exits,
    returns-twice calls and extension nodes take no declaration, and
    assembly, SEH, `cleanup`, computed `goto` and label addresses are named
    boundaries asked first.** Card CG21-CALL-EXITS-AND-BOUNDARIES
    (`crates/qcore-c-gateway/src/call_exits.rs`, the route-G adapter
    `legacy_call_exits.rs`, thin hooks in `legacy_c.rs`
    (`Statement::PathEnd`, compiled to a node with no successor that is not
    an exit; the boundary hook at the head of `expression_statement`; the
    declared hook after its classification; `goto *` in the `goto` arm; the
    named uncontracted refusal), additive items in `typed_access_path.rs`
    (`StatementIndex::no_return_declarations`), and in
    `qcore-contract::contract_loader` (`resource_no_return()`,
    `no_return_callees`, `declared_no_return_callees`,
    `UNDECLARABLE_C_NAMES`)). It amends rule 10j's no-return exclusion.
    (1) **Path end.** A statement whose classification is a `ContractCall`
    with no destination, to a callee whose contract carries `ensures normal
    resource_no_return();` (a terminating exit: no normal exit and no frame
    of the calling thread resumes; contract specification C04.3.3), is
    `Statement::PathEnd` when `call_exits::resolve` reads this body's Clang
    AST as `NoReturn`: every `DeclRefExpr` naming the callee refers to a
    `FunctionDecl` whose type ends in `__attribute__((noreturn))` (no pointer
    declarator in the spelling) or which carries `NoReturnAttr`,
    `C11NoReturnAttr` or `CXX11NoReturnAttr` (inherited included), the
    references agree and no object of the name is declared in the body. The
    node keeps the call's instruction, so the contract's modes (every one
    `resource_borrowed`) are checked as at any call. Otherwise the call is
    refused (`no-return contract of `f` not established at this call: …`).
    In every other position the declared call is applied as an ordinary
    borrowing call with its continuation (an over-approximation of C's
    paths, never a path end); **amended by rule 15y**: wherever a route
    lowers it, its CFG node ends its path where this rule's premises hold
    and is refused otherwise. A summary is read from the normal exits only;
    a body with none publishes none (`observations_match_contract`'s existing
    refusal of a function with no reachable exit). (2) **Exit
    kind.** The C attribute does not establish termination: `longjmp`,
    `siglongjmp`, `__longjmp_chk`, `setcontext`, `swapcontext`,
    `pthread_exit` and `thrd_exit` are `noreturn` and abandon the frame's
    live resources while the program goes on, and the returns-twice calls
    (`setjmp`, `_setjmp`, `__setjmp`, `sigsetjmp`, `__sigsetjmp`, `savectx`,
    `vfork`, `getcontext`) have a second return no CFG here carries. The
    loader rejects a declaration of any of them, and of the Clang extension
    nodes spelled like calls (`__builtin_choose_expr`, `__builtin_bit_cast`,
    `__builtin_offsetof`, `__builtin_types_compatible_p`,
    `__builtin_convertvector`,
    `__builtin_shufflevector`, `__builtin_FILE`/`FUNCTION`/`LINE`/`COLUMN`),
    so no ordinary contract is applied to a non-call node; an uncontracted
    call to one is refused with its reason and the writes the uncontracted
    refusal had. A builtin Clang parses as a call stays an ordinary call. (3)
    **Boundaries.** A statement holding `asm`/`__asm__`/`__asm` (an `asm`
    label or register binding included), `asm goto`, `__try`/`__except`/
    `__finally`/`__leave`, a `cleanup`/`__cleanup__` attribute or a label
    address (`& &label` where an operand starts), and a computed `goto *E;`,
    is an `Unknown` naming the boundary with unbounded writes, asked before
    every older route (a soundness repair: an older route could admit the
    statement without the scope-exit call or the assembly's effects); a
    computed `goto` no longer fails its function's parse. **Do not** end a
    path on the attribute alone, on a declaration the call's Clang
    declaration does not establish, or on a name; declare a non-local exit,
    a returns-twice call or an extension node; or lower a boundary's effect
    by another route. Every consultation is a parity row (`position:
    "call-exit-statement"` with callee, resolution and decision;
    `"call-exit-boundary"`; in parity runs only, `"call-exit-census"`, an
    uncontracted whole-call statement's resolution, which changes nothing).
    Premises stated, not proved: each `resource_no_return()` declaration
    (`abort`, `exit`, `__assert_fail` in `quirin-provider-libc`'s
    `stdlib.qv`; since CONTRACT-NORETURN-AND-BORROWED-TAILS also
    librdkafka's `rd_kafka_crash` in `qcore-provider-librdkafka`'s
    `memory.qv`, pinned; constraint 28) and faithful Clang extraction of the
    attribute (trust boundary 28). Measured (corpus `66910c10`, system,
    tail-pause slice, base `b6a3dc42`): corpus 629 / 10 / 4,195 →
    **641 / 10 / 4,183** (`3d53a5d0…` → `c9cdfb1c…`), **12 Unknown → Safe**
    by their own path ends (`__assert_fail` in ten, `abort` and `exit` in
    Monkey's `mk_header_set_http_status` and `mk_socket_reset`), all 1,265
    `__assert_fail`/`abort`/`exit` leaves gone (1,519 statement path ends,
    every one resolved `noreturn`, none outside a statement), no correction,
    no diagnostic added but the renames; system and slice no move.
    `QCore.CallExits` (`decision_vectors_hold`, `path_end_iff`,
    `attribute_alone_not_path_end`, `declaration_alone_not_path_end`,
    `path_end_no_exit`, `path_end_rest_unchecked`, `path_end_checks_pre`,
    `refused_no_claim`, `model_sound`, `nonlocal_path_end_hides_leak`,
    `boundaries_refused`, `boundary_table_refused`), bound to
    `DECISION_VECTORS` and `BOUNDARIES` by shared-vector tests and to route G
    by `tests/cg21_call_exits.rs`;
    [report](../reports/cg21-call-exits-and-boundaries/README.md).

<a id="constraint-11p"></a>

11p. **A call through a function pointer is the calls C may make only when
    the unit closes its candidate set; an open set keeps its older refusal,
    and no callback registration is a capture any contract states.** Card
    CG22-INDIRECT-CALLS-AND-CALLBACKS (`indirect_calls.rs`, the route-G pass
    `legacy_indirect_calls.rs`, thin hooks in `legacy_c.rs` (the two
    analysis entries, right after rule 10c's normalisation),
    `typed_access_path.rs` (`IndirectCallUnit`, `with_indirect_call_unit`,
    `TableFacts` retained with the unit's AST) and
    `legacy_declared_abi.rs` (`inline_function_pointer_parameter`)).
    (1) **Candidate sets** (K01, `resolve`), from Clang's AST only: a
    function designator through parentheses, `*` and `&` is itself; an
    automatic local whose address is never taken and which has an
    initializer is the union of its initializer and every assignment to it
    (a flow-insensitive superset of its reaching definitions); a place whose
    function-pointer type is `const`, read by `.` members and array
    subscripts from a named object with an initializer (a constant table,
    rule 11h), is its initializer's member or element (every element for a
    non-literal index; an array filler is a null candidate); an effect-free
    conditional is the union of its arms. Every other callee is an **open
    set**: a formal, a member or element read through a pointer anywhere on
    its base chain (vtable-like dispatch: any object of the type may be the
    one read), a call's result, an explicit cast, a null definition, an
    address-taken or uninitialised local, a mutable static, a candidate a
    declaration in the function shadows. No set is ever "every function of
    this signature". A builtin's call (`BuiltinFnToFnPtr`) is direct, not
    indirect (CG21's boundary). (2) **The rewrite.** Once per unit, before
    the reserved-spelling escape, every hold and withdrawal, the literal
    premises, W4's derivation scope and every classifier position, a closed
    set is rewritten in the token stream: a singleton's callee tokens become
    its candidate's name (in any position); a larger set's whole statement
    `S` (a call statement, `NAME = CALL;` or `return CALL;`) becomes
    `{ int c₁; …; if (c₁) S[f₁]; else … else S[fₙ]; }` over fresh,
    never-assigned scalars, so the flow explores every candidate's arm and
    joins them. Each candidate is then held, withdrawn, restricted,
    summarised and classified exactly as its direct call written in source:
    this pass decides no contract. A resolved local every other reference of
    which is a rewritten callee has each definition's value replaced by `0`
    (it is never read again). A site is placed only when its callee's tokens
    occur on its line as often as Clang's sites with those tokens, all with
    one resolution, and no conditional directive falls inside an edit;
    otherwise, and for every open set, nothing changes and the older
    refusal (`call to uncontracted function …`, the indirect-access gate,
    rule 10c's opaque callee) stands byte for byte. A lexical pre-filter
    skips reading the AST for a unit whose bodies call only file-scope
    names, keywords and builtins (cost only, never an admission). (3)
    **Callbacks** (K02, `callback_trace`): registration needs a live
    context and no earlier teardown, an invocation the registration in force
    and the context live, the context's release a prior unregistration; a
    trace that ends registered is an escaped capture (`Unknown`). The
    contract language states no capture and no unregistration, so route G
    has no registry: a registration call stays the call it is (uncontracted
    ones refuse; a declared `Borrowed` context, `msgpack_packer_init`'s
    `data`, is the contract's word, `CG22-CALLBACK-REGISTRY-CONTRACT`; since
    that card the contract language states them, rules 13u–13z), an
    invocation through a resolved pointer applies the candidate's contract,
    so a released context is refused as at a direct call, and the relation
    classifies registrations in the parity census only. (4) **The inline
    declarator** (absorbing LOADER-ABI-INLINE-FNPTR-DECLARATOR): the
    declared-ABI reader reads a parameter `RET (*NAME)(PARAMETERS)` (or
    abstract `(*)`) as the typedef reader's `fnptr`; a deeper, qualified or
    returning-a-pointer inline declarator stays unread. **Do not** close a
    set from a formal, a member through a pointer, a signature or a
    configuration; resolve through a cast; drop a callee or condition with
    an effect; rewrite a site that is not placed uniquely; let the pass
    decide a contract (the rewrite precedes every hold); or rewrite after an
    older route of the stream. Premises stated, not proved: faithful Clang
    extraction (callee shapes, declaration ids, `const` qualification,
    initializer order) and token placement (trust boundary 28); the no-UB
    reading (`AOM-POISON-NO-UB`): a call through an indeterminate value
    (a jump past the initializer) or an out-of-bounds table index is
    undefined, so the set need not cover it
    (`P-CG22-DEFINED-POINTER-VALUE`, `P-CG22-TABLE-INDEX-IN-BOUNDS`).
    Measured (corpus `66910c10`, system, tail-pause slice; BEFORE main
    `b6a3dc42`, AFTER rebased on `06ad81cb`; re-taken as BEFORE main
    `a6adb483`, AFTER rebased on `31a72343`): **no site has a closed set**
    (corpus 572 distinct indirect sites, 235 in reported functions; system
    432; slice 585; plus 4,996 / 2,460 / 2,524 builtin calls, which are not
    indirect), so the rewrite changes no token; the declarator moves corpus
    629 / 10 / 4,195 → 631 / 10 / 4,193 and, on the final base,
    645 / 10 / 4,179 → **647 / 10 / 4,177** (`ceefb6c4…` → `ed384eb0…`;
    `rd_kafka_topic_partition_list_sort_by_topic`, `…_by_topic_id`), no
    Safe → non-Safe move, no leak or violation added; system and slice
    byte-identical.
    `QCore.IndirectCalls` (`join_over_covers_sound`,
    `an_open_set_can_miss_a_violation`, `a_local_holds_one_of_its_definitions`,
    `invocation_after_release_is_never_safe`,
    `registration_after_teardown_is_never_safe`, the inline-declarator
    theorems over `QCore.TypedefProjection`), bound to `JOIN_VECTORS` and
    `CALLBACK_VECTORS` by shared-vector tests and to route G by
    `tests/cg22_indirect_calls.rs`;
    [report](../reports/cg22-indirect-calls-and-callbacks/README.md).

<a id="constraint-11q"></a>

11q. **C integer semantics are the target's as Clang reports them, the core's
    checked table is reached only through an explicit C interpretation, and
    integer arithmetic is a separate property whose obligations are never
    discharged by assumption.** Card CG20-NUMERIC-DOMAIN
    (`crates/qcore-c-gateway/src/c_numeric.rs`,
    `crates/qcore-c-gateway/src/c_numeric_obligations.rs`; thin hooks in
    `legacy_declared_abi.rs` (`project_arithmetic`) and
    `qcore-verification`'s `cycle18_callback_obligations::evaluate_integer`).
    (1) **Target identity.** Every integer width is read from the analysing
    Clang for the unit's flags (and a declared `--target`, for this property
    only; the custody gateway transports none, CG30): `__CHAR_BIT__`,
    `__SIZEOF_{SHORT,INT,LONG,LONG_LONG,POINTER}__`, `__CHAR_UNSIGNED__`,
    the `__*_MANT_DIG__` precisions, cross-checked against the `*_MAX__`
    values (`TargetIntegerModel::from_macros`; a missing or inconsistent macro
    refuses). No width comes from a table or from the host. (2)
    **Conversions are Clang's.** A consumer reads the promotions and usual
    conversions Clang recorded (`ImplicitCastExpr`, each operator's type,
    `computeLHSType`/`computeResultType`) and refuses an operator whose
    operand types differ from its computation type; `promote` and
    `usual_arithmetic` state C17 6.3.1.1/6.3.1.8 only to bind Clang's records
    to the Lean vectors under LP64 and LLP64. (3) **The C interpretation.**
    `c_numeric::binary`/`convert` map a C type to its fixed-width core type
    and use `scalar_types::checked`/`convert` where they answer; the rest is
    decided by C: unsigned `+ - *` and conversions to unsigned wrap
    (defined), signed overflow, `/ %` by zero, `MIN / -1` and `MIN % -1`,
    a shift count outside `[0, width)` and a left shift of a negative value
    are undefined, and a right shift of a negative value, a conversion to a
    signed type that cannot represent the value, an inexact
    integer-to-floating conversion and pointer/integer conversions are
    implementation-defined. Implementation-defined cases are obligations:
    no implementation premise is declared. (4) **The property.**
    `analyze_numeric_obligations` walks every function body of the unit and
    gives each integer operation, increment, compound assignment, recorded
    conversion and bit-field store its fault cases as obligations over
    intervals (a literal, a Clang `ConstantExpr` value or a Clang-evaluated
    enumerator is a point; anything read is its type's full range; no flow,
    path or loop fact). `Safe` only when every obligation is `Discharged` and
    nothing is refused; a `Violated` obligation (constant operands) is a
    definite fault *if reached* and makes the function `Unknown`, never
    `Unsafe`, since reachability is not decided. Floating arithmetic is
    refused by name (N09/N10); pointer arithmetic is the bounds property's
    (CG16) and is counted, not discharged. The verdict is published beside
    custody and never enters the custody denominator; a custody verdict
    reads none of it. (5) **Width-independent constants.** The cycle-18
    evaluator accepts a value only inside its type's portable range (C17
    5.2.4.2.1's minimums; plain `char` refused), with exact casts, so its
    value is the same under every data model (`portable_constant`). (6)
    **Plain `long` in the lexical ABI reader.** One `long` word is read as the
    64-bit, pointer-width projection only when the analysing Clang's default
    target reports a 64-bit `long` and pointer (`plain_long_is_lp64`); under
    LLP64, ILP32 or a failed query the declaration is unreadable. This
    removes the reader's LP64 premise for plain `long` that the MSVC ABI
    reader's report recorded; `long long` and `__int64` keep their reading.
    A per-unit `-arch` flag is not seen by this process-wide query
    (`CG20-ABI-READER-ARCH-FLAG`). **Do not** read a width from a table, the
    host or a data-model name; re-derive a conversion Clang recorded; call
    the core's checked result C's; discharge an implementation-defined case
    by the implementation's documented choice; publish a `Violated`
    obligation as `Unsafe`; or let a custody verdict depend on this property.
    Premises stated, not proved: Clang's predefined macros are its target
    widths and its JSON AST is read faithfully (trust boundary 28); two's
    complement representation (C23; every Clang target). Measured: corpus `66910c10`, system and tail-pause slice at base `b6a3dc42`: **byte-identical** BEFORE → AFTER on all three (corpus 629 / 10 / 4,195 `3d53a5d0…`; system 44 / 2 / 188, slice 29 / 1 / 239 / 1 Pending, digests `e363acc8…` and `acc73cd9…` in this checkout), the corpus parity stream identical as a multiset (466,022 rows); no custody function moves, as predicted. The property itself has no corpus census yet (`CG20-NUMERIC-CORPUS-CENSUS`).
    `QCore.CNumeric` (37 audited theorems: `long_plus_uint_lp64`,
    `long_plus_uint_llp64`, `kernel_ok_iff_unreached`,
    `unsigned_arithmetic_defined`, `wrap_in_range`, the `*_discharge_sound`
    family, `portable_in_every_model`, …), bound to `c_numeric` by
    `promotionVectors`, `usualVectors`, `kernelVectors` and
    `conversionVectors` (and to Clang by `clang_records_the_lean_usual_conversions`);
    [report](../reports/cg20-numeric-domain/README.md).

<a id="constraint-11r"></a>

11r. **A return-code disposition is applied only where the caller decides the
    returned value's class; an untested call keeps the join, which refuses;
    a body-derived disposition is one class per returned literal, each
    validated by the ordinary route; a declaration its body contradicts is
    withdrawn.** Card CG23-RELATIONAL-POSTCONDITIONS (vocabulary
    `crates/qcore-schemas/src/relational_post.rs` (`RelationOp`,
    `ReturnRelation::meets`, `ExitClass`, `RelationalContract::{select, join,
    disjoint, total}`; `ResourceContracts::{relational, relational_contract,
    insert_relational, withdraw_relational}`, additive, skipped when empty),
    the loader clause `crates/qcore-contract/src/contract_relational.rs`
    (contract spec §C04.13, grammar G05), the pure decisions
    `crates/qcore-c-gateway/src/relational_post.rs` (`read_test`,
    `class_name`, `integer_constant`), the route-G adapter
    `legacy_relational_post.rs`, the summary route
    `legacy_relational_summary.rs`, the stream guard
    `legacy_stream_line_order.rs`, thin hooks in `legacy_c.rs`
    (`build_function_cfg_with`, the `if` arm, the uncontracted fallback),
    `legacy_summary.rs` (`derive_custody_summaries`) and
    `legacy_reserved_spelling.rs` (the class prefix)). It amends rule 10x's
    "an out-parameter acquisition gets no split" for callees with a
    disposition; every other premise of 10x and 5h stands. It composes with
    rule 11v: an in-place kind's initialiser status stays 11v's witness, and
    the loader refuses a disposition beside an in-place kind.
    (1) **Never in the function map.** A disposition lives in
    `ResourceContracts::relational`, so every route that knows nothing of it
    sees an uncontracted callee. `build_function_cfg_with` hands the parser an
    environment with each class installed under a minted, reserved name
    (`qcore_relational_class__K__f`), only when the stream names a callee
    with a disposition. (2) **The decided test.** Only an `if` condition that
    is exactly one call tested against a plain decimal constant (`f(…)`,
    `!f(…)`, `f(…) OP K`, `K OP f(…)`, the call optionally `(NAME =
    f(…))`) is read (`read_test`); each edge takes the one disposition every
    class it can reach agrees on (`select`, reachability decided exactly on
    the constants' breakpoints), and the plan is a branch that decides
    nothing, then on each edge the call classified once under that edge's
    class. A test whose edge reaches disagreeing classes or none, or whose
    class call the classifier refuses, is the plan of its named refusal.
    `NAME = f(…);` followed immediately by such an `if` testing `NAME` is
    rewritten to the same C with the assignment in the condition (statement
    boundary `;`, `{`, `}` before `NAME`); the rewritten `if` head (the
    `if`, its parenthesis, the condition before `NAME` and the added
    parenthesis) takes `NAME`'s line and the parenthesis closing the moved
    call the call's own, so the stream's lines stay non-decreasing (card
    RELATIONAL-REWRITE-LINE-ORDER,
    [report](../reports/relational-rewrite-line-order/README.md): before it the `if` kept its own line
    ahead of the moved call's, and `typed_place::occurrence` declined every
    statement of the unit). The stream the parser reads, after every
    rewrite, is guarded: one whose lines are not non-decreasing is a hard
    error in debug and test builds and, in release builds, a function whose
    first instruction is the refusal `stream-line-order` (never `Safe`,
    counted as a leaf). An out slot a class leaves
    unwritten (`resource_borrowed` beside `resource_out` in another class)
    is passed as data on that class, `&NAME` only, and `NAME` then holds an
    untracked value (`OpaqueValue`). (3) **Untested.** Every other call of a
    callee with a disposition refuses with `relational-untested` (the join of
    disagreeing classes; the loader rejects a declaration whose classes all
    agree, and the summary route publishes none). (4) **Exit-specific
    summaries.** After every ordinary route published nothing for an
    undeclared `int NAME(…)` whose every `return` returns a plain integer
    literal (two values at least), the body is re-read per value `v` with
    every other `return` replaced by `for (;;);`, and the ordinary inference
    and validation run unchanged; the class `eq v` takes the validated
    contract (an exit falling off the end belongs to every class). (5)
    **Body check.** A declared disposition whose function the unit defines is
    compared with the body's ordinary or exit-specific summary; a
    contradiction withdraws it (`relational-withdrawn` at every call site);
    no summary leaves it a declared assumption (rule 8). **Do not** put a
    disposition in the function map, apply a class on a call whose value is
    not decided by its own test, prune an edge a selection calls
    infeasible, treat `resource_borrowed` on an out slot as unwritten
    outside a disposition, or leave a rewritten stream's lines out of
    order. Premises stated, not proved: faithful Clang
    extraction (trust boundary 28), the declared dispositions (constraint 4),
    and that replacing a `return` by divergence leaves every state on every
    other path as it was (the CFG's own construction).

<a id="constraint-11s"></a>

11s. **Bounds is a separately selected property: every access is
    inventoried and judged against a source extent or a reviewed declaration,
    a declaration never overrides the source, a held one discharges and
    accuses nothing, and custody is never changed by it.** Card
    CG16-INDEX-AND-OFFSET-EXTENTS (`crates/qcore-c-gateway/src/index_extent.rs`;
    the decisions are `qcore_core::index_extent`, the vocabulary
    `qcore_contract::bounds_contract` (grammar G05b), the audit selection
    `QCORE_BOUNDS_PROPERTY` in `crates/qcore-audit/src/projection.rs`; no hook
    in `legacy_c.rs` or `typed_source.rs`, and no custody reader calls any of
    it). Contract specification C09.1. (1) **Selection.** The property
    `c.bounds.index-extent.v1` runs only when asked for (the gateway API, or
    the audit variable, which writes its rows and tally beside an untouched
    custody report). A request naming custody and bounds joins the two rows
    (`compose`); a requested row that is missing is `Unknown`. (2)
    **Inventory.** From Clang's typed AST of the translation unit (read
    streaming, keeping only the selected bodies), every subscript,
    dereference, `->`, pointer formation (`p ± k`, `&a[i]`, `p++`, `p += k`),
    pointer subtraction and ordering, and every pointer actual of a call, in
    every position (conditions, returns, actuals, loop bodies), each recorded
    with its position and line; an unread expression or statement form is a
    named function reason (`bounds-unsupported-form`), never skipped; the
    operands of `sizeof`/`offsetof` are unevaluated, a variable-length type is
    refused (CG17). (3) **Extents.** A fixed array's count is Clang's type
    (locals, globals, members through `.` or `->`, string and compound
    literals, each dimension of a nested array); a declared extent enters
    only from a bounds contract: the function's own `bounds_extent`,
    `bounds_index`, `bounds_global_extent` (a conditional scope, named on the
    row) and a callee's `bounds_return_extent`, used only when every
    precondition of that call is established. A declaration that differs
    from a source extent is a reported contradiction
    (`bounds-declaration-contradicted`, Unknown) and the source extent
    decides. (4) **Values.** Only literals, enumeration constants, `+ - *`
    of two constants in their C type (signed overflow refuses, unsigned
    wraps), value-preserving or unsigned conversions (an out-of-range signed
    conversion refuses), a tracked local's or parameter's fact, guard facts
    from comparisons with a constant or a stable parameter, and the canonical
    `for (i = c0; i < E; i++)` with `i` unwritten in the body; everything
    else is `bounds-unsupported-range` (CG20 owns C's numeric semantics).
    (5) **Invalidation.** Facts live on locals and parameters that are not
    `static`, `volatile` or address-taken; a write withdraws the variable's
    facts and every fact naming it, a call withdraws every fact on an object
    the source did not establish unless the callee states
    `bounds_preserves_extents()`, a loop withdraws what it writes, a label
    keeps nothing, a join keeps what both edges share. (6) **Verdict.**
    Each obligation is judged with only source and resolved premises in force
    and again with the held ones in force as written; `Unsafe` only for an
    out-of-bounds decision on an exact index **definitely reached** (the
    straight-line prefix of the body, no call, loop, branch, label or
    unresolved obligation before it, not under `&&`, `||` or `?:`, and no
    other call in its full expression), `Pending` only when held
    declarations are the sole obstacle and in force leave none, a
    hypothetical violation is the standing reason
    `bounds-held-premise-violation`, an access after `return`/`break`/
    `continue`/`goto` with no label, or in a constant-false arm, is
    excluded by that check and stays in the inventory. (7) **Refutation and
    admission.** A body that returns a source object smaller than its own
    declared `bounds_return_extent` refutes it: the row keeps
    `bounds-contract-refuted-by-body` and the unit's callers see an
    uncontracted callee (the API takes a cross-unit table). Recorded evidence
    is admitted only when a re-derivation from the same source, target
    (Clang's `__SIZEOF_*__`, `__CHAR_UNSIGNED__`, `__PTRDIFF_MAX__`) and
    bounds contracts has the same identity, accesses and verdict. Amended by
    [rule 12l](#constraint-12l): a body whose access through a quant-checked
    formal is decided outside its check refutes it the same way. **Do not**
    let a custody result, a custody profile's no-undefined-behaviour premise
    or a loaded declaration discharge a bounds obligation, let a declaration
    enlarge or replace a source extent, publish `Unsafe` for a violation not
    definitely reached or resting on a held declaration, drop an access from
    the inventory, or read the bounds rows from a custody route. Premises
    stated, not proved: faithful Clang extraction (trust boundary 28) and
    that a resolved declaration describes its external object or body
    (C06's `Declared`). Measured (corpus `66910c10`, system, tail-pause
    slice, base `b6a3dc42`): every custody report **byte-identical**
    (corpus 629 / 10 / 4,195 `3d53a5d0…`); with the property selected on the
    corpus and no bounds declaration, bounds **162 Safe / 0 Pending / 0
    Unsafe / 4,759 Unknown** over 4,921 definitions (87 of them the
    pinned-build exclusions, `bounds-import-failure`), combined custody and
    bounds **111 Safe / 4,800 Unknown / 10 Unsafe**; the largest obstacle is
    `bounds-missing-extent` (4,650 functions, 4,133 with a pointer passed to
    an uncontracted call). `QCore.IndexExtent` (`element_access_sound`,
    `formation_sound`, `out_of_bounds_sound`, `one_past_formed_never_accessed`,
    `empty_extent_admits_no_access`, `byte_access_sound`, `scale_sound`,
    `declared_never_overrides_source`, `write_withdraws_every_mention`,
    `unsafe_only_for_a_reachable_violation`,
    `safe_only_when_every_obligation_discharges`, `held_never_safe`,
    `held_never_accuses`, `pending_only_as_the_sole_obstacle`,
    `another_obstacle_keeps_unknown`, `a_missing_requested_row_is_unknown`,
    `combined_safe_needs_both`, `admitted_evidence_matches`), bound to
    `qcore_core::index_extent` by `elementVectors`, `byteVectors`,
    `resolveVectors` and `verdictVectors`
    (`crates/qcore-core/tests/index_extent.rs`) and to route G by
    `tests/cg16_index_extents.rs`;
    [report](../reports/cg16-index-and-offset-extents/README.md).

<a id="constraint-11t"></a>

11t. **A read-modify-write, compare-exchange or fence reaches the weak-memory
    model as its own event under C11 as published (N1570), never as a neutral
    call; consume stays out of scope and is refused, never read as
    acquire.** Card CG28-WEAK-MEMORY-EVENTS (`crates/qcore-core/src/weak_memory.rs`,
    additive; `atomic_source_route.rs`, `legacy_atomic_route.rs`, the
    atomic-publication extractor's refusal mapping). It amends rule 11i's
    refusal of read-modify-writes, compare-exchange and fences; 11i's
    consume, order, by-pointer, value and unlisted refusals, route T's arm
    and the publication verdict stand. (1) **The model.** `MemoryEventKind`
    gains `AtomicReadModifyWrite` (read and written value, any order),
    `AtomicCompareExchange` (expected, observed, desired, success and failure
    orders, strength, outcome: a success is a read-modify-write with the
    success order, a failure a load with the failure order; a strong exchange
    fails only on a differing value, a weak one may fail spuriously) and
    `AtomicFence` (thread or signal scope). RMW atomicity (N1570 7.17.3): the
    write an RMW reads is its immediate modification-order predecessor
    (`AtomicityViolation`). Release sequences follow **one** standard, N1570
    5.1.2.4p10: the head, then every modification by the head's thread or
    that is an atomic RMW (not C++20's P0982 weakening, not RC11).
    Synchronises-with is 5.1.2.4p11 plus 7.17.4p2–p4 (fence–fence,
    fence–operation, operation–fence, through the hypothetical release
    sequence); a seq_cst thread fence joins S with 7.17.3's three fence
    observation rules (not C++14's added modification-order rule). A signal
    fence orders only against a handler of its own thread, which the model
    has none of: it synchronises nothing and joins no S. A compare-exchange's
    failure order is neither release nor acq_rel and no stronger than its
    success order (7.17.7.4p2, `compare_exchange_failure_admitted`). (2)
    **Consume** (decision). The model has no consume order and no
    dependency-ordered-before. Reading consume as acquire would add
    happens-before edges the program lacks, so race freedom could be claimed
    for a racy program: a consume load, read-modify-write or compare-exchange
    order (success or failure) is refused by name. A consume *fence* is an
    acquire fence by 7.17.4.1p2's own text and is routed as one. (3) **The
    route.** The table's read-modify-write rows (`__atomic_exchange` is
    reclassified a by-pointer form), compare-exchange rows (strength by name
    for `__c11_*`, by the evaluated weak flag for the GNU forms; the generic
    form's desired value by pointer), thread- and signal-fence rows, and the
    `_Atomic` compound assignment and increment are routed
    (`RoutedAtomic::event`); the legacy `__sync` builtins (GCC full
    barriers, no C11 order) are refused by name, their sized spellings as
    unlisted. (4) **Route G.** A fence is no custody effect; a
    read-modify-write is its object's dereference with a write; a
    compare-exchange its object's dereference with a write, its `expected`
    operand's write (`*e` through the typed reader, or `&x` forgetting the
    scalar local) and its desired value's read; a produced value (an RMW's old
    value, a compare-exchange's outcome) may go to a scalar destination or a
    scalar `return`; an `_Atomic` compound assignment or increment through a
    pointer is the RMW's effect. Operand values must be a literal, an
    enumerator or a non-pointer named object. G's verdict is custody only and
    carries no ordering claim. (5) **Publication.** A routed RMW,
    compare-exchange or fence of the flag is refused by name as not a
    publication shape (CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD). **Do not**
    read consume as acquire, mix release-sequence definitions, let a signal
    fence synchronise threads, admit a failure order stronger than the
    success order, route a `__sync` builtin as a C11 order, or read a routed
    event's order as an ordering or race-freedom claim of route G. Premises
    stated, not proved: faithful Clang extraction of the builtin, operands,
    orders and weak flag (trust boundary 28); the Rust-to-Lean correspondence
    of the checker by review, bound on 20 litmus graphs; an RMW's arithmetic
    and a failed compare-exchange's write to `expected` are the supplied
    graph's premises. Every weak-memory result is of **one supplied candidate
    execution** (`SINGLE_GRAPH_ASSUMPTION`; execution coverage is CG29's).
    Measured (corpus `66910c10`, base `b6a3dc42`, re-measured on main
    `b934c560`): corpus 629 / 10 / 4,195 → **630 / 10 / 4,194** on the base
    (`3d53a5d0…` → `d7be9d8c…`) and 630 / 10 / 4,194 → **631 / 10 / 4,193**
    on main (`5ade5aab…` → `c2a96c6f…`), one Unknown → Safe move
    (`cmt_atomic_compare_exchange`, the generic compare-exchange of
    `cmt_atomic_clang.c`, its only leaf), no correction, no leak or
    violation added; atomic leaves 1 → 0; system and slice byte-identical,
    no atomic leaf in either.
    `QCore.WeakMemory` (new; `release_sequence_definition`,
    `rmw_continues_release_sequence`, `foreign_store_ends_release_sequence`,
    the four synchronisation theorems, `accepted_rmw_is_atomic`,
    `cas_failure_no_stronger`, `weak_failure_may_be_spurious`,
    `litmus_vectors_hold`, bound to `check_c11_execution` by
    `crates/qcore-core/tests/unit/weak_memory_cg28.rs`, which parses the
    litmus graphs from the Lean file) and `QCore.AtomicSourceRoute`
    (`read_modify_write_routed`, `fence_routed`, `consume_fence_is_acquire`,
    `consume_order_refused`, `routed_failure_admitted_by_model`,
    `cas_vectors_hold`); [report](../reports/cg28-weak-memory-events/README.md).

<a id="constraint-11u"></a>

11u. **A layout fact is Clang's: a VLA is admitted only with every use inside
    its block, a flexible-array extent only from an established allocation
    size, an alignment obligation only from Clang's numbers, and a
    `container_of` value is its enclosing object only when the pointer is
    established as that member's address.** Card CG17-LAYOUT-DERIVED-EXTENTS
    (`layout_extents.rs`, the route-G adapter `legacy_layout_extents.rs`,
    additive `typed_access_path::analysed_unit_constants`, thin hooks in
    `legacy_c.rs` (the last fallbacks of `expression_statement` and of the
    `return` chain, and the parity census)). (1) **VLAs** (M12). Asked last,
    only for a refused statement Clang confirms is one `DeclStmt` of one
    automatic `VarDecl` of variably modified array type, matched by line and
    exact tokens: `vla_decision` admits it only when the element is a builtin
    scalar (no identity), the bound Clang prints (`ELEM[BOUND]`, one bracket)
    only reads (integer literals, `+ - * ( )` and names declared exactly once
    as non-`volatile` integer parameters or automatics; a call, store or
    increment is `bound-effect`), the declared name is declared once, and
    every use of the object is an element access or a `sizeof` operand. It is
    then lowered to a `ForgetScalar` of its name (a new object each time it
    runs; no custody event) and carries three numeric obligations it
    never discharges (`bound-defined`, `bound-positive` (C11 6.7.6.2p5),
    `size-representable`; CG20's). Any other use (a decay into an actual, a
    return, a pointer store, `&a`, `&a[i]`) may carry the address out of the
    block, whose exit ends the lifetime, and keeps the refusal: a VLA address
    returned is never Safe. (2) **Flexible arrays** (M13). For `R->data` of
    an incomplete-array member, the remaining extent is `(size −
    offsetof(data)) / sizeof(T)` (`flexible_extent`), with Clang's offset and
    width (`__builtin_offsetof`/`sizeof` evaluated by Clang in one probe per
    function) and only when the allocation size is established; no current
    contract establishes one (CG16 owns the extent vocabulary, rule 11s), so
    the extent is `size-not-established`. It is a fact for bounds, never a
    custody fact. (3) **Alignment** (M19). Every `BitCast` of a derived
    address (pointer arithmetic or `&x`, through value-preserving pointer
    conversions) to a pointer whose referent is not a character or `void`
    carries C11 6.3.2.3p7's obligation, decided by `alignment` from Clang's
    `_Alignof`/`sizeof`: aligned when the base's guaranteed alignment and the
    offset are multiples of the referent's, misaligned when the base is and a
    constant offset is not, undecided otherwise (a `char *`/`void *` base
    guarantees one byte; a missing number decides nothing). The published
    property `layout.alignment.v1` (`layout_report`) is `Safe` only when every
    obligation is aligned; a misaligned one is a provisional candidate with
    no reachability validation and joins to `Unknown`, never a published
    `Unsafe`. A pointer value's base alignment is its pointee's under
    `P-CG17-POINTER-ALIGNED` (C11 6.3.2.3p7), named in the report only when
    used. (4) **container_of** (E18). Asked last, only for a refused
    declaration, assignment or `return` whose whole value is `(T *)((char *)P
    - OFF)`, `OFF` an `offsetof(T, m…)` (its own tokens) or `(integer)
    &((T *) 0)->m…`: `container_of` names the enclosing object `x` only when
    `P` is `&x->m…` (one arrow at the root, then `.` steps, no anonymous
    member), or a pointer local whose only write is that initializer while
    neither it nor `x` is otherwise written or addressed and `x` is declared
    once, with `x`'s pointee spelling, the result's and `OFF`'s record equal
    and the member paths equal. The statement is then lowered as the
    respelled `… = x;` / `return x;` by the older routes, and only if they
    admit it; every other pointer (a formal of the member's type, a list
    lookup such as `mk_list_entry(H->next, …)`, whose element is rule 10f's
    container premise, never member provenance) keeps the refusal. **Do not**
    admit a VLA whose address may leave its block, discharge a VLA's numeric
    obligation, count a flexible extent without an established allocation
    size, decide an alignment from a host number or without Clang's, publish
    a misaligned obligation as `Unsafe`, read `container_of` of an
    unestablished pointer as its container, or consult these hooks before an
    older route (hard invariant 3). Parity runs record each consultation
    (`position: "layout-extents-vla"`, `"-container-of"`,
    `"-container-of-statement"`/`"-return"`) and every function's census
    (`"layout-extents-census"`: VLAs, flexible accesses, `container_of`
    shapes, derived-address conversions, `offsetof` count). Premises stated,
    not proved: faithful Clang extraction of types, bounds, offsets, sizes,
    alignments, record and declaration ids and the token/AST node match
    (trust boundary 28), and `P-CG17-POINTER-ALIGNED`. Measured (corpus
    `66910c10`, system, tail-pause slice, base `b6a3dc42`): **no function
    moved and every report is byte-identical** (629 / 10 / 4,195,
    `3d53a5d0…`); the corpus declares no VLA, its 355 `container_of` shapes
    (189 functions) are all list lookups (`not-a-member-address`, none
    reaching the hook), 441 derived-address conversions (232 functions) are
    41 aligned and 400 undecided, 16 flexible accesses (12 functions) read
    `size-not-established`. `QCore.LayoutExtents` (`every_counted_element_lies_inside`,
    `the_count_is_maximal`, `an_unestablished_size_counts_nothing`,
    `the_bound_is_read_once`, `the_address_is_dead_after_the_block`,
    `an_escaping_address_is_never_admitted`,
    `an_admitted_declaration_carries_the_bound_obligation`,
    `aligned_is_sound`, `misaligned_is_sound`,
    `a_misaligned_obligation_is_never_safe`,
    `the_round_trip_returns_the_enclosing_object`,
    `the_round_trip_needs_the_premise`, `an_unrelated_pointer_is_refused`),
    bound to `FLEX_VECTORS`, `VLA_VECTORS`, `ALIGNMENT_VECTORS` and
    `container_vectors` by `tests/unit/layout_extents.rs` and to route G and
    the report by `tests/layout_extents.rs`;
    [report](../reports/cg17-layout-derived-extents/README.md).

<a id="constraint-11v"></a>

11v. **A resource kind beyond the four has its own carrier, sentinel and
    release protocol; an in-place kind binds its obligation to one automatic
    object, and its initialiser's status is its sentinel.** Card
    CG25-RESOURCE-KINDS (`crates/qcore-c-gateway/src/legacy_in_place_custody.rs`,
    `crates/qcore-contract/src/contract_in_place.rs`, additive items
    `ResourceKind::{DynamicLibrary, Regex, carrier, is_in_place}` and
    `ResourceCarrier` in `qcore-schemas`, `AbstractState::status_witnesses`,
    thin hooks in `legacy_c.rs`: the actual places, the formal-address gate,
    the statement guard, the transfer wrapper, the null-test refinement and
    the CFG check; contract specification §C04.6, §C04.6.1). (1) **Kinds.**
    `dynamic_library` is pointer-carried with failure value NULL and is no
    other kind (`free(h)`, `dlclose(p)` discharge nothing; `dlsym(h, …)`
    after `dlclose(h)` is `use_after_release`). `regex` is **in place**: its
    initialiser (`resource_out(p, nullable)` + kind) creates an obligation in
    the object `p` designates, its finaliser (`resource_released(p,
    nonnull)` + kind) discharges it, and a kinded `resource_borrowed` position
    uses it. (2) **Storage.** An in-place actual is `&NAME` for one automatic,
    non-pointer, non-array object Clang declares once in the function; its
    place is `NAME`. Any other initialiser or finaliser actual refuses; a use
    naming no tracked object keeps the plain borrowed reading. (3) **The
    status is the sentinel.** The initialiser's status must be held by a
    plain, not address-taken name, which witnesses the object until it is
    written (`writes_scalar`); a null test of the witness refines the
    object's binding (zero certifies, nonzero removes); witnesses join by
    intersection. A discarded status, or one consumed by the expression that
    tests it, refuses. (4) **Custody, never data.** A statement mentioning a
    tracked object is admitted only as a contracted call of a declared callee
    whose every mention is a whole `&NAME` actual at an in-place position of
    the object's kind, or as the object's declaration; the formal-address gate
    already refuses `&NAME` at any other position. A call node with an
    in-place initialiser or finaliser lacking its place is refused after the
    CFG is built, so no route drops the effect. **Do not** equate a new kind
    with an existing one, bind an in-place obligation to a member, a global,
    a static or storage reached through a pointer, refine on a status that is
    not the witness, or admit tracked storage on a data route. Premises
    stated, not proved: the libc contracts (`regcomp` returns zero exactly
    when it initialised the object; `dlclose` releases its handle) and
    faithful Clang extraction (trust boundary 28). Measured (corpus
    `66910c10`, system, tail-pause slice, base `b6a3dc42`): corpus 629 / 10 /
    4,195 → **630 / 10 / 4,194** (`3d53a5d0…` → `5ade5aab…`), one Unknown →
    Safe move (`rd_regex_match`: `r = regcomp(&re, …); if (r) return 0; …
    regfree(&re);`), 5 functions changed, every leaf attributed, no
    correction, leak or violation; a possibly-NULL kinded handle returned
    without a contract keeps refusing (`mk_plugin_load_dynamic`,
    `rd_dl_open0`: inferred summaries stay heap, C04.6 rule 7); system and
    slice change no function. Residuals `CG25-IN-PLACE-RESIDUAL-PAIRS`,
    `CG25-KINDED-WRAPPER-RESULTS`, `CG25-DLSYM-SYMBOL-LIFETIME`.
    `QCore.ResourceKinds` (28 theorems, `AuditResourceKinds.lean`; the kind
    table and the protocols bound to Rust and to the libc declarations by
    `kindVectors` and `protocolVectors`); `QCore.KindedCustody.Kind` gains the
    two constructors; [report](../reports/cg25-resource-kinds/README.md).

<a id="constraint-11w"></a>

11w. **A `va_list` is read only as C11 7.16's protocol, established on every
    path before any of its builtins is custody-neutral; a trailing actual is
    read per position only as far as its callee's tail clause states.** Card
    CG26-VARIADICS (`crates/qcore-c-gateway/src/va_list_protocol.rs`,
    `variadic_tail.rs`, the route-G adapter `legacy_variadics.rs`, thin hooks
    in `legacy_c.rs` (two statement fallbacks after rule 11i's, the
    `variadic_tail_widening` cell around the tail refusal, the
    `variadic_borrowed_tails` set), `typed_field_access::read_parameters`
    (the trailing `, ...`), and `qcore_contract::contract_loader`
    (`resource_variadic_borrowed_tail()`, `variadic_borrowed_tails`,
    `declared_variadic_borrowed_tails`)). It amends rule 10s/E20's
    uncontracted-builtin refusal for the four `va_*` builtins only (CG21's
    common builtin boundary is never asked for them) and O04-TRACE's tail
    premises T1-T6 only by T7 and T8 below. (1) **The protocol.** A list is a
    `va_list` local of the body's outermost block (no initializer, no storage
    class; initially uninitialised, owing `va_end` on every exit) or a
    `va_list` parameter (initially active, owing nothing; `va_start`, `va_copy`
    into and `va_end` on it refuse). Its events, from Clang's AST only:
    `__builtin_va_start(L, P)` (P the last parameter of a variadic
    definition), `VAArgExpr`, `__builtin_va_copy(D, S)` (a read of S, a start
    of D), `__builtin_va_end(L)`, `L` as a whole argument of any other call (a
    pass: indeterminate until `va_end`, 7.16p3) and `&L` of a local as a whole
    argument (a lend: read like `va_arg`). Every other mention, two events in
    one full expression, an event under `&&`/`||`/`?:`/a statement
    expression/an unevaluated operand, a statement expression holding a jump,
    a computed `goto`, a `longjmp` and an unread statement form refuse the
    function's protocol by name (`va_list protocol refused: …`). A control
    graph is built (sequence, `if`, loops, `switch`/`case`/`default`, `break`,
    `continue`, `return`, `goto`/labels), the per-list sets of states are
    iterated to a fixed point, and the function is admitted only when the
    certificate holds list by list: the entry holds the initial state, every
    reachable node's events step (`step`: start from uninitialised or ended;
    `va_arg`, a pass and a copy read from active only; end from active or
    passed) and its result is contained in each successor's set, and the
    exit's set is accepted. (2) **The lowering.** Only for a statement every
    older route refused as a call to an uncontracted `va_*` builtin, matched
    to its Clang node: under an admitted protocol `va_start`/`va_end`/
    `va_copy` and a discarded `va_arg` are `NoOp` (a list is never bound) and
    `x = va_arg(L, T)` / `T x = va_arg(L, T)` for an arithmetic or enumeration
    `T` into a scalar local is `ForgetScalar(x)`; a pointer `va_arg` keeps a
    named refusal (K09's pointer half). Under a refused protocol the statement
    keeps its refusal with the protocol's reason: a missing `va_end`, a use
    after `va_end` or after a pass, and a `va_copy` without its own `va_end`
    are never `Safe`. (3) **Variadic definitions.** The lexical declaration
    reader reads a definition's named parameters before a trailing `, ...`
    (C11 6.7.6.3p9: the ellipsis declares no object), so a variadic
    definition's typed rules are no longer refused wholesale. (4) **Trailing
    actuals.** Only for a call statement every older route refused with the
    tail refusal, whose callee declares a tail, after Clang's promoted type
    of every trailing argument (C11 6.5.2.2p7) is an arithmetic, enumeration
    or pointer type, the statement is re-classified with every trailing
    position `Borrowed` (`borrowed_tail_contract`) and each trailing actual
    T1-T6 data, an admitted nested call, **T7** `&NAME` of a declared scalar
    that is not a descriptor name or protected formal (its fact forgotten
    after the call), or — only under `resource_variadic_borrowed_tail()` —
    **T8** a plain name read as a `Borrowed` place (the flow's liveness and
    alias checks apply). Under the data tail an identity-carrying actual keeps
    O04-TRACE's refusal. Rule [12g](#constraint-12g) reads a position T1-T8
    refuse from its Clang node. **Do not** read a list event the AST does not show,
    admit a protocol on any path that is not certified, lower a `va_*`
    statement before an older route (hard invariant 3), read a pointer
    `va_arg` value as an identity, widen the data tail to identities, or claim
    format agreement (K10, out of scope). Route T is unchanged: it still
    refuses a variadic signature and `VAArgExpr` (K08/K09's T half).
    Premises stated, not proved: faithful Clang extraction and node matching
    (trust boundary 28), and a declared tail clause (a trusted assumption;
    the borrowed tail is declared for `mk_vhost_set` (CONTRACT-PIN-TIP-MODE)
    and, since CONTRACT-NORETURN-AND-BORROWED-TAILS, for `printf`,
    `fprintf`, `snprintf`, `mk_print` and `flb_log_print`).
    Measured (corpus `66910c10`, main `b6a3dc42`): 629 / 10 / 4,195 →
    **631 / 10 / 4,193** (`3d53a5d0…` → `42c7df13…`), +2 Unknown → Safe
    (`rd_kafka_set_thread_name`: protocol; `rd_kafka_open_cb_generic`: T7),
    no correction, no leak or violation added; 117 of 124 uncontracted `va_*`
    leaves gone, 19 named refusals in 8 functions (`mk_vhost_set`'s
    missing `va_end` and `rd_kafka_cgrp_rejoin`'s pass before `va_start`
    read as correct; the rest precision or pointer `va_arg`); system
    byte-identical, tail-pause slice no status move.
    `QCore.Variadics` (`step_sound`, `step_refuses`, `certified_invariant`,
    `certified_no_violation`, `certified_exit`,
    `the_skipping_protocol_is_never_certified`,
    `borrowed_tail_is_the_identity`, `a_scalar_address_demands_nothing`),
    bound to `STEP_VECTORS` by a shared-vector test;
    [report](../reports/cg26-variadics/README.md).

<a id="constraint-11x"></a>

11x. **A loop is summarized only by an invariant its tokens are checked to
    preserve, a recursion only by a rank its actuals are checked to
    decrease, and a budget's exhaustion is reported, never `Safe`.** Card
    CG27-LOOPS-RECURSION-BUDGETS (`legacy_counted_loops.rs`, a child of
    `legacy_counted_users.rs` with one `Stmt::Loop` hook there;
    `legacy_signed_rank.rs`, a child of `legacy_recursion.rs`, whose
    `prepare` now shares `member_header` and whose contextual CFG respells a
    checked sign guard; one `RankedRoute` arm in `legacy_summary.rs`). It
    amends rule 10e's "loops leave the function's clauses undecided" and
    nothing else. (1) **Counted-user loops.** A `while`, `for` or `do` (other
    than `do … while (0)`) whose extent is read is summarized as the identity
    on every count, pending release check and possibly-destroyed mark, and
    forgets the value of every name it spells, exactly when no token of it
    writes a declared counter, calls a function whose contract declares a
    counted-user operation or exit clause, calls through an expression, or
    is `return` or `goto`, and every other named call is neutral as
    `call_effect` decides it anywhere (a verified declared neutrality, a
    recursively neutral body here, or a declaration without counted-user
    clauses under UC-FRAME). The check is flow-insensitive, so it holds for
    every interleaving of body paths and every number of iterations; any
    other loop keeps the pre-existing refusal and wording. (2) **Signed
    threshold rank.** A recursive component that the ordinary, zero/nonzero
    ranked and neutral-frame routes all left unpublished is read once more:
    one control type for every member, `int`, `long` or `long long` (never
    narrower, never mixed, so no actual is converted); every `if` condition
    naming the control is one sign guard (`n <= c`, `n < c + 1`, `n > c`,
    `n >= c + 1`, or mirrored) at one literal threshold `0 <= c <= 1024`;
    every recursive actual for the control is `n - k` with a decimal literal
    `1 <= k <= 1024` (several ranks at once) or the literal `0`; the control
    appears nowhere else. The base context (`n <= c`) is checked with every
    component summary absent, the step context (`n > c`) with the base
    contracts in force, and both must publish the same contract, rechecked
    on the combined CFG, as on the zero/nonzero route. The rank is
    `max(n - c, 0)`; the guard is respelled as the null test whose truth
    agrees with it in each context, and the certificate keeps the original
    source and re-runs the extraction. (3) **Budgets.** No budget changed.
    The custody alias widening (`MAX_ALIAS_TARGETS`), the counted-user path
    bound (512) and route T's four budgets (rule 11l(7)) are each reported
    and leave their function non-`Safe`. **Do not** summarize a loop whose
    invariant was not checked over every one of its tokens, read a recursive
    actual as decreasing without the base context excluding it, admit a
    zero/nonzero partition for a signed decrement (it diverges from a
    negative value), admit `n - k` for `k > 1` on an unsigned control (it
    wraps), consult the signed route before the routes it follows, or turn a
    budget's exhaustion into a verdict. Premises stated, not proved: faithful
    token extraction of the loop extent, guards and actuals (trust boundary
    28); the counted-user pass's existing premises (UC-FRAME; values change
    only by a syntactic write, now forgotten for every name a loop spells).
    Measured (corpus `66910c10`, system, tail-pause slice, base
    `b6a3dc42`): all three **byte-identical** (629 / 10 / 4,195
    `3d53a5d0…`; 44 / 2 / 188; 29 / 1 / 239 / 1 Pending), as predicted:
    no function on any route carries the counted-user loop refusal (the
    retry functions are blocked earlier by unattributed operations), and
    the recursive components refuse at the containment gate before any
    rank is read; no budget is exhausted on any route. Four neutral-route
    pins of an effectful `if (n > 0)` recursion were re-pinned to the
    unranked `if (n)`. `QCore.LoopInvariants` (`Induction`:
    `invariant_holds_after_every_iteration`, `a_broken_transition_is_refused`,
    `postfixpoint_bounds_every_iterate`; `CountedLoop`:
    `an_admitted_loop_preserves_delta`, `an_admitted_loop_preserves_pending`,
    `a_counting_body_is_refused`, `an_opaque_body_is_refused`; `Widening`:
    `a_strict_growth_lowers_the_potential`, `a_widened_alias_is_never_safe`;
    `SignedThreshold`: `an_admitted_actual_decreases`,
    `threshold_calls_are_well_founded`, `publish_by_threshold_induction`,
    `a_read_guard_is_decided_by_its_context`,
    `an_unsigned_double_step_can_increase`; `Budget`:
    `an_exhausted_budget_is_never_safe`), bound to `sign_guard` and
    `step_actual` by `guardVectors` and `stepVectors`;
    [report](../reports/cg27-loops-recursion-budgets/README.md).

<a id="constraint-11y"></a>

11y. **A dialect, a target and a type each have one identity, carried into
    the evidence; a registered dialect difference or an unadopted extension
    is refused by name, and a target is admitted only under the readers' LP64
    premise.** Card CG30-DIALECTS-FLAGS-TYPES (entries I01 I02 I03 I05 N12
    N13 N14; `crates/qcore-c-gateway/src/c_frontend_identity.rs`, thin hooks
    in `legacy_dialect_hook.rs` (route G, from `legacy_c.rs`) and
    `typed_source.rs` (route T); shared items named for other cards:
    `CTargetIdentity` (card CG20 keys integer widths on it, rule 11q),
    `CTypeId`, `CExtensionFamily`, `DialectDifference`). (1) **Dialects.**
    `CCompilerDialect` gains `C17`, `C23` and `C2y`, named exactly
    (`from_std_name`; `c18`, `c2x`, `gnu23` and every alias stay refused).
    Each dialect's AST differences from the C11/GNU baseline are registered
    together (`dialect_differences`: none for the baseline and C17; 17 for
    C23; 6 more for C2y) and join the request digest
    (`dialect_identity_bytes`; the four baseline dialects keep their one
    historical byte, so no earlier digest moves) and the compile-task identity
    (`dialect_differences_sha256`, absent for a dialect with none). No
    difference is adopted. Route G withdraws the body of a definition whose
    declaration or body spells one to a single call of the reserved marker
    `qcore_dialect_refused__KEY` (escaped in source by `reserved_spelling`)
    before any reader, W4 or replay reads the stream, so the row is `Unknown`
    with no summary and can never publish an `Unsafe` the token reader's
    misreading would cause (a `nullptr` null test is the measured instance);
    the refusal names the construct, key and dialect. A digit separator,
    which the token reader would lex as a character literal, refuses the
    unit. Route T refuses a registered node, flag, trait or type spelling by
    name. `nullptr` is never excluded for its `CXX` node name. (2) **Targets.**
    `--target=`/`-target` and `-mabi=` are parsed into closed vocabularies
    (`CTargetTriple`, `CAbiName`), transported verbatim to every Clang
    invocation as `CFrontendFlag::Target`/`Abi`, and serialised into the
    request digest and the compile-task identity, so a changed target is a
    different evidence identity and a certificate minted for one is refused as
    another's. A selection is admitted only when its data model is LP64 (the
    premise the ABI reader's projection states) or no explicit selection
    fixes one; ILP32, LLP64, an unrecorded component and a doubled selection
    are refused by name (`unsupported compilation target`/`ABI`,
    environment category). The existing `-arch` is checked the same way,
    and route G (before any reader) and route T admit a unit's own frontend
    flags again (`CTargetIdentity::admit_flags`), so a caller that bypasses
    the compile-database reader is held to the same premise; every admitted
    unit is therefore LP64 by its own selection, and card CG20's
    default-target `long` question in the ABI reader (`plain_long_is_lp64`)
    is never answered for another data model (its numeric route already
    queries Clang with the unit's flags, so it keys on the transported
    target). There is no flag passthrough. The pinned Cycle 19 route
    (`compiler_options_compatibility`) is unchanged and still reads no target
    word (constraint 11). (3) **Types.** `CTypeId` is Clang's canonical
    spelling (whitespace collapsed, `bool` read as `_Bool`, an arithmetic
    multiset in Clang's order); routes G (`typed_access_path::type_id`) and T
    (`typed_source::canonical_type_id`) read it from the same AST spelling,
    and the ABI reader (`declared_abi::declared_type_id`) from a specifier
    multiset through its own typedef resolution. Compatibility stays Clang's;
    the identity is a key, and no reader's admission changes. (4)
    **Extensions.** `_Complex`/`_Imaginary` (and `__real__`/`__imag__`,
    imaginary literals), vector/matrix/sizeless types and their operations,
    `_BitInt` and fixed point are refused by name in every dialect: route G
    adds `C extension type `…` (FAMILY) is not adopted (CG30)` to a selected
    definition whose typed AST holds one (read only where the unit spells an
    extension; an unreadable AST there is refused too), keeping every other
    leaf (an existing validated `Unsafe` stays); route T refuses first. None
    is adopted. **Do not** reuse a verdict across dialects or targets, fold a
    `-std=` alias onto an identity, pass a compiler flag through, admit a
    non-LP64 target before card CG20 keys widths on `CTargetIdentity`, let a
    token reader read a registered difference, or drop an extension refusal
    without adopting the family in every reader. Premises stated, not proved:
    faithful Clang extraction (trust boundary 28) and SHA-256 collision
    freedom for the digests. Measured (corpus `66910c10`, system, tail-pause
    slice; BEFORE base `b6a3dc42`, AFTER tip on main `b934c560`): no status
    move on any route and no correction; 4 Unknown `rdhttp.c` definitions gain
    the unreadable-AST refusal (their own AST nests beyond `serde_json`'s
    128), the slice's `flb_skip_leading_zeros_simd` the vector refusal; no
    measured unit uses a new dialect or a target word. The census found
    vectors in 5 Fluent Bit definitions (none Safe), `_Complex` only in glibc
    typedefs, `_BitInt` and fixed point nowhere.
    `QCore.DialectTargetIdentity` (`dialectByte_injective`,
    `distinct_dialect_distinct_bytes`, `baseline_bytes_unchanged`,
    `c2y_extends_c23`, `lookup_minted_under_key`, `no_reuse_across_dialects`,
    `no_reuse_across_targets`, `admit_iff`, `ilp32_abi_refused`,
    `llp64_target_refused`, `withdrawn_is_unknown`, `refusal_never_safe`,
    `refusal_keeps_unsafe`, `refusal_is_monotone`, `perm_same_typeId`,
    `bool_is_Bool`, `long_unsigned_int_vector`, `distinct_counts_distinct`),
    bound to the Rust tables by `tests/unit/c_frontend_identity.rs`;
    [report](../reports/cg30-dialects-flags-types/README.md).

<a id="constraint-11z"></a>

11z. **Race freedom over every execution is claimed only from an exhaustive
    enumeration of a bounded, loop-free fragment, with its bounds in the
    evidence identity; one accepted graph is never race freedom; reclamation
    is safe only by a checked protocol whose helpers are `Declared`.** Card
    CG29-EXECUTION-COVERAGE-AND-RECLAMATION (`crates/qcore-core/src/execution_coverage.rs`
    and `reclamation.rs`, new; additive `weak_memory::derived_synchronizes_with`;
    the atomic-publication verdict in
    `qcore_verification::atomic_publication_source`). It amends rule 11i's
    publication verdict only. (1) **Execution coverage** (gap entry W07). A
    fragment declares atomic locations with initial values and per thread
    guarded atomic loads, stores, exchanges, compare-exchanges, fences and
    non-atomic reads and writes. `check_program_executions` enumerates every
    candidate execution: a path per thread for every read value in the
    location's domain and every outcome of a weak compare-exchange on its
    expected value, every reads-from source of equal location and value,
    every modification order (a permutation of the location's writes), every
    S order; synchronises-with is derived; thread 0 initialises and spawns,
    so initialisation happens before every thread. Each candidate is checked
    by the unchanged `check_c11_execution`; one reporting at most data races
    is consistent. `RaceFree` only when every candidate was checked and no
    consistent one races; `Race` carries a consistent racing witness;
    `Unknown` names a loop, a malformed fragment (a location declared twice,
    an undeclared atomic location, a location both atomic and non-atomic), an
    exceeded bound (threads, static events, values per location, candidates)
    or no consistent candidate. A location's domain is its initial value and
    every literal written to it, justified without control flow, so an
    out-of-thin-air cycle cannot escape it; arithmetic read-modify-writes are
    not in the fragment (`CG29-ARITHMETIC-RMW-VALUE-DOMAIN`). (2) **The
    publication verdict.** `Safe` now also needs the extracted shape (flag
    initialised, publisher, consumer guarded by its load) `RaceFree` within
    `PUBLICATION_COVERAGE_BOUNDS` (threads 2, events 7, values 2, candidates
    16); it then carries `ALL_EXECUTIONS_ASSUMPTION` in place of
    `SINGLE_GRAPH_ASSUMPTION`. `Unknown` keeps the single-graph assumption.
    Every shape the extractor admits is `RaceFree`, so no verdict moved.
    (3) **Reclamation** (gap entry W10). Hazard-pointer and epoch-based
    reclamation are checkers over a trace of protocol steps:
    * hazard pointers: protect, validate only while linked, access only
      through a validated hazard, retire only once unlinked, reclaim only when
      retired and no slot holds the object;
    * epochs: access inside a critical section to a linked object or one
      unlinked during it, advance only when every active thread is in the
      current epoch, reclaim two epochs after the retire.
    Neither checks "not reclaimed" at an access; that no accepted trace
    accesses a reclaimed object is proved from the rules. A
    `ReclamationContract` maps helpers to roles and is checked: a duplicate
    or a role of the other protocol refuses, each used helper is a `Declared`
    boundary on the certificate, and the announce/validate handshake with
    the contract's declared ordering is decided by execution coverage
    (`RaceFree`: discharged within bounds; `Race`: the contract refuses;
    `Unknown`: `Declared`). A refused trace is `Unknown`, never `Unsafe`: the
    trace is supplied, not derived from source
    (`CG29-RECLAMATION-SOURCE-ROUTE`). **Do not** claim race freedom from one
    graph, enumerate a fragment with a loop, leave a bound out of the
    evidence identity, race-check a location accessed both atomically and
    non-atomically, test reclamation by the object's status at the access
    instead of by the protocol's rules, or present a refused trace as a
    defect. Premises stated, not proved: the Rust-to-Lean correspondence of
    the enumerator and the protocol checkers by review, bound on shared
    vectors parsed from the Lean files; the checker's own premises (rule
    11t); no liveness or eventual observation (W11). Measured (corpus
    `66910c10`): no corpus, system or slice route reaches either module; the
    corpus, system and slice are unchanged (see the report). Lean
    `QCore.ExecutionCoverage` (`encode_mem_candidates`,
    `read_values_are_literals`, `verdict_race_free_sound`,
    `verdict_race_witnessed`, `candidate_is_execution`,
    `race_free_needs_bounds`, `coverage_vectors_hold`), `QCore.Reclamation`
    (`hp_accepted_safe`, `ebr_accepted_safe`, their invariants, the trace
    vectors), `QCore.WeakMemory` (a spawn event, additive) and
    `QCore.AtomicSourceRoute` (`safe_publication_covers_every_execution`,
    `unknown_publication_carries_single_graph`);
    [report](../reports/cg29-execution-coverage-and-reclamation/README.md).

<a id="constraint-12c"></a>

12c. **A `va_arg` in a controlling expression is one protocol event per
    evaluation, lowered before the test only as the whole `(NAME = va_arg(L,
    T))`; a pointer `va_arg` value is a never-owned derived binding.** Card
    VA-LOOP-AND-POINTER-ARG (`crates/qcore-c-gateway/src/legacy_va_condition.rs`,
    new; `legacy_variadics.rs`; one hook in `legacy_condition_desugar.rs`'s
    `planned_condition`; shared items `LegacyInstruction::VaArgPointer`,
    `variadics_route::{StoredClass, stored_assignment}`,
    `Parser::stored_va_arg`). It amends rule 11w's lowering only (the
    protocol, its graph and its certificate are unchanged). (1) **The
    condition.** Asked after every older route of an `if`, `while`, `for` or
    `do` controlling expression, CG02's elaborator included, and only for one
    whose Clang node (matched by line and exact tokens) holds a `VAArgExpr`:
    the whole expression `(NAME = va_arg(L, T))` under a protocol rule 11w
    admits is lowered to the event's statement (exactly what `NAME =
    va_arg(L, T);` lowers to) followed by the test of `NAME` (a null branch
    for a pointer, a branch refining nothing for a scalar), the plan's entry
    being the event so that a back edge and `continue` re-evaluate it: one
    event per evaluation, then the test, C's order (C11 6.5.16p3; the
    condition is a full expression). Every other expression holding a
    `VAArgExpr` (a comparison, a connective, a bare `va_arg`, a `switch`, a
    destination or value the statement rule refuses) is refused by name, and
    under a refused protocol the condition carries the protocol's reason.
    (2) **The pointer value.** `T *x = va_arg(L, T *)` and `x = va_arg(L,
    T *)` for an object pointer type (no function pointer, array or
    `volatile`) into an automatic local (a body `VarDecl` with no storage
    class, the only object of its spelling, not a formal, protected formal or
    descriptor name) is `VaArgPointer(x)`: the binding a derived result over
    an unbound base gets (no target, unknown, possibly null, `Derived`), so by
    the derived-pointer rules a null test narrows the binding alone, a
    `Borrowed` position reads it, and a release, consumption, store, escape,
    return or reallocation through it is never applied and refuses
    (`UnknownAlias`; never accused, it has no target). Every other
    destination keeps rule 11w's named refusal. **Do not** lower any other
    expression shape holding a `va_arg`, reorder the event after the test,
    read the value as an identity or as owned, bind it as an identity-unknown
    `OpaqueValue` (its null test would refuse), or lower before an older
    route (hard invariant 3). Premises stated, not proved: faithful Clang
    extraction and node matching (trust boundary 28), and the Rust lowering's
    correspondence to the Lean `lower` (review, pinned by a unit test).
    Measured (corpus `66910c10`, main `958e76593`): **647 / 10 / 4,177**, no
    status move (`ed384eb0…` → `7f7d8328…`), 10 Unknown functions change
    leaves (loop-condition refusal −8, pointer-`va_arg` refusal −3, one named
    shape refusal and one `va_end` refusal in their place, unknown_alias −2),
    no leak, violation or Unsafe added; system byte-identical; tail-pause
    slice `flb_service_set` Unknown → Pending (its one held call); the probe
    `mk_vhost_set` at Monkey's fix tip keeps only `mk_vhost_set_property`.
    `QCore.VariadicConditions` (`lower_sound`, `lower_complete`,
    `lowered_no_violation`, `lowered_exit`,
    `the_leaking_loop_is_never_certified`, `never_owned_is_never_released`,
    `never_owned_changes_no_custody`, `never_owned_is_never_accused`,
    `a_null_test_refines_only_the_binding`), bound to `decide` and
    `derive_pointer` by unit tests;
    [report](../reports/va-loop-and-pointer-arg/README.md).

<a id="constraint-12d"></a>

12d. **A publication carries its owner's field-linked children only when
    each is exactly live and unencumbered, and an unlink of the owner hands
    them back.** Card MONKEY-VHOST-PROPERTY-CUSTODY (`carried_publication.rs`,
    thin hooks in `legacy_container_publication.rs`; the identifier follows
    gateway rule 12c (VA-LOOP-AND-POINTER-ARG)). It amends rule 10d's withdrawal of an
    object that owns linked objects. (1) **Carry.** A publication into a
    caller's or a static container (rule 10d's admitted containers) of an
    owner with no contained child (`resource_stored`) decides every child it
    links through an O00-B08 field link: a child that is exactly live, linked
    on every path (not rule 11c's partial triple), with no unresolved effect,
    not itself published and owning no link or contained child of its own is
    carried (`ESCAPED` with its owner); any other child, an untracked one
    included, keeps rule 10d's withdrawal byte for byte. The links are kept.
    (2) **Hand-back.** An unlink of the published owner hands back every
    must-linked child that is exactly `ESCAPED` (`LIVE` again, as the owner
    is); a child with an `ESCAPED` bit that is partial or of mixed status
    refuses the unlink (`… may have been carried into the container on some
    paths only`); any other child was never carried and is left as it is.
    Releasing the owner afterwards does not cascade (P-B08-SHALLOW), so a
    child still held at an exit is a leak. Hand-back is sound because nothing
    else moves a carried child: a store into a published owner's member
    refuses (its owner is not live), and a release, consumption or return of
    an escaped identity is a violation candidate or refused. **Do not** carry
    an untracked child (it may be a caller's object, whose custody no rule
    moves), a contained child, or a child through a container rule 10d
    refuses; **do not** drop the hand-back, without which `mk_list_del`
    followed by a release of the owner would hide the child's leak. The
    nested-container relation (a publication into a container inside a
    body-acquired owner, `W45-CONTAINER-ROOT-REACH`) is not this rule.
    Premises stated, not proved: the flow's status bits, partial triples,
    unresolved effects and published set are what the Rust reads; faithful
    lexical reading of entries and containers (trust boundary 28).
    `QCore.CarriedPublication` (`carry_iff`, `restore_iff`,
    `the_name_branch_is_safe`, `the_withdrawal_refused_the_name_branch`,
    `releasing_the_owner_after_the_unlink_leaks_the_child`,
    `the_carry_only_replaces_a_refusal`, the shared `publishVectors` and
    `unlinkVectors`); [report](../reports/monkey-vhost-property-custody/README.md).

<a id="constraint-12e"></a>

12e. **A leak claim never rests on a refused test: on the null edge of a
    guard the projection could not decide, a site joined provenance ties to
    the tested name is settled.** Card SUP07-REFUSED-TEST-ACCUSATION
    (`crates/qcore-c-gateway/src/legacy_refused_test.rs`, new; one hook in
    the custody work list's `PointerRefinement::Unknown` arm and its mirror in
    the leak-witness census; shared items `remove_null_site` (factored out of
    `refine_pointer_null`, byte for byte) and `LegacyCfg::tie_names`). It
    amends rule 10b. (1) **Settle.** Where a null (validity) test refuses
    (`UnknownAlias`, `unmodeled`), each site the tested binding may denote
    that joined provenance ties to the tested name at the test (the name
    denotes the site on every path on which the site exists) is removed on
    the null (invalid) edge exactly as a decided test's null edge removes its
    site: on that edge it is NULL or absent. The identity refusal stays, so
    no function becomes `Safe` by this rule; the other edge, an untied or
    untargeted site, a status witness's test (11v) and every decided test are
    unchanged. (2) **Tie names.** The provenance read is joined provenance
    over `conditional_names` plus each unique local whose only `&` uses are
    member addresses through it (`&NAME->F`, `&(NAME)->F`): C reads the
    operand as the postfix `NAME->F` (C11 6.5.2.3, 6.5.3.2), so no pointer to
    `NAME`'s slot exists and the call and indirect-store frames' premise
    holds of it; a refused statement still forgets it (its sparing frame is
    `frame_protected_locals`' as before). Only this rule reads them; rule
    10b's conditional release and every frame keep `conditional_names`.
    **Do not** settle without the tie (a site live under no name on the null
    edge, after an overwrite of the tested name, is an established leak), and
    **do not** replace the settlement with an unresolved-effect marker: the
    marker joins by union and withdraws a leak established on the other edge
    where the two meet. Both were the interim withhold rule's defects
    (`7493e76a2`, which this rule replaces; its controls stay and pass).
    Premises stated, not proved: joined provenance's binding sets mean what
    the Lean `Tied` says (rule 10b's own premise), and the lexical member
    address reader (trust boundary 28). Measured (corpus `66910c10`, tip
    rebased onto main `f50fa2009`): **647 / 9 / 4,178** (`dde4837e…`),
    byte-identical to main with the withhold rule; against main `cffe232d4`
    (`a7d68d49…`) 2 corrections, provisional Unsafe → Unknown
    (`mk_http_error_page`, `mk_sched_add_connection`), leak 42 → 39, nothing
    else; system and tail-pause slice byte-identical; the `mk_vhost_set`
    probe unchanged. `QCore.RefusedTestAccusation`
    (`tied_null_edge_owes_nothing`, `without_the_tie_a_null_edge_may_owe`,
    `settle_iff`, `the_rule_matches_the_executions`,
    `the_old_claim_was_false`, `an_established_claim_survives_the_join`,
    `a_marker_would_withdraw_it`, `a_refused_test_is_never_safe`, the shared
    `edgeVectors`); [report](../reports/sup07-refused-test-accusation/README.md).

<a id="constraint-12f"></a>

12f. **A refused scalar `return` value is read as the value of a statement.**
    Card W48-BOTTOM-UP-CALLEE-BODIES
    (`crates/qcore-c-gateway/src/legacy_scalar_return_value.rs`, new; one
    hook asked last in the `return` lowering of `legacy_c.rs`, after rule
    11u's, only for a return every route refused; no shared item changed).
    C11 6.8.6.4p3 converts a returned value as if by assignment to an object
    of the result type, so in a function whose declared result is a scalar
    value (`return_facts.scalar_result`) `return E;` evaluates what
    `T tmp = E; return tmp;` evaluates. Where W43's typed value walker
    (`typed_field_access::typed_scalar_value`, the statement position's
    reader: no call, no unary `&`, no assignment, increment or comma, no
    pointer-valued name, at least one typed access) admits `E`, the return is
    lowered with the statement value's instruction (one
    `IndirectScalarAccess` with the walker's bases, their dereference
    obligations and the record bases' alias reads, no destination, no frame)
    and an exit that escapes nothing. A member `S.F` of an automatic aggregate
    carries no obligation, as in a statement, even after `&S` was handed to a
    call: the typed access-path reader asks a never-bound root of every `.`
    path only because it also serves stores and pointer reads
    (W48-RETURN-AGGREGATE-MEMBER-READ, `rd_clock`). **Do not** admit a
    pointer result, a value naming a tracked descriptor or an opaque callee
    spelling, or a value the walker refuses, and **do not** ask the hook
    before an older route: every older admission keeps its code (hard
    invariant 3). Premises stated, not proved: the walker's lexical reading
    and declaration facts (W43's) and the correspondence with the Rust (trust
    boundary 28). Measured (corpus `66910c10`, main `c85cb8806`):
    **647 / 9 / 4,178 → 648 / 9 / 4,177** (`dde4837e…` → `bcc98d2d…`); one
    move, `rd_kafka_timer_next` Unknown → Safe through the validated header
    bodies `rd_clock` and `rd_uclock` and the helpers they alone blocked; 104
    rows change, 112 call leaves to those helpers removed (one of them
    nested), and in 14 functions 15 statements re-attributed from an
    indirect access or a condition to an unresolved identity on the same
    line (refusal to refusal); no Safe → non-Safe move, no leak or violation
    added; system and tail-pause slice byte-identical.
    `QCore.ScalarReturnValue` (`an_automatic_member_carries_no_base`,
    `admitted_value_is_defined`, `a_released_base_fails_its_obligation`,
    `return_matches_statement`, `a_scalar_return_escapes_nothing`,
    `an_owned_object_stays_owed`, `decide_iff`, the shared
    `decisionVectors`, `a_declined_return_is_never_safe`);
    [report](../reports/w48-bottom-up-callee-bodies/README.md).

<a id="constraint-12g"></a>

12g. **A trailing actual rule 11w's token premises refuse is read from its
    Clang node: data only when no name in it carries a tracked identity, a
    `Borrowed` place only as one whole answered pointer name under a declared
    borrowed tail, and a cast is never read.** Card CG26-TAIL-RESIDUAL-ACTUALS
    (`crates/qcore-c-gateway/src/variadic_tail_actuals.rs`, new; thin hooks in
    `legacy_variadics.rs` (`tail_readings`, `tail_reading_admits`, a fourth
    alternative of rule 11w's `tail_widened`); shared items named:
    `Parser::variadic_tail_readings` in `legacy_c.rs`, set only around rule
    11w's fallback re-classification, and `Parser::position_answered` in
    `legacy_typed_roots.rs`, rule 10l's "declared exactly once" premise). It
    amends rule 11w(4) only by one more admission per trailing position,
    asked after T1-T8 refuse it, in the same statement fallback and after the
    same promoted-type premise. The call's argument node is read
    (`TailExpr::from_node`; parentheses and implicit conversions transparent,
    and the operand of an arithmetic implicit conversion read as arithmetic
    even when Clang spells its type only by a typedef of an anonymous
    enumeration) and classified by one recursion (`classify`): **data** — a
    literal, a predefined identifier, an enumeration constant, a
    constant-size `sizeof`/`_Alignof`, an arithmetic object that is not one of
    the function's integer-carried identities, a pointer or array object rule
    10l's served never-bound premise answers, and a unary `- + ~ !`, a binary
    arithmetic, bitwise, shift, comparison or logical operator or a `?:`
    every operand of which is data; **identity** — one name that may carry a
    tracked identity; or a **named refusal** (`cast`, `pointer-arithmetic`,
    `identity-arm`, `identity-operand`, recorded in the parity row
    `tail_readings`). `read` admits data, and admits an identity as a
    `Borrowed` place of that name only for a pointer or array object, only
    under `resource_variadic_borrowed_tail()`, only when rule 10l answers the
    name (one formal or automatic local of that spelling), and only when the
    token actual is exactly that name, so the flow's liveness and alias
    checks apply to the object the actual denotes (`integer-identity`,
    `identity-under-data-tail`, `unanswered-root` otherwise). A refused
    reading keeps O04-TRACE's refusal byte for byte. **Do not** read an
    explicit cast (the card CAST-FORMALIZATION's, rule 12h), read pointer
    arithmetic or a conditional with an identity-carrying arm or condition as
    data or as a place, admit an identity under the data tail (nothing widens
    it), or ask the reading before T1-T8 (every older admission keeps its
    code, hard invariant 3). Premises stated, not proved: Clang's node and
    type and the node match (trust boundary 28), rule 10l's never-bound and
    declaration premises, the integer-carried identities of the descriptor
    reader, and the declared tail clauses (rule 11w). Measured (corpus
    `66910c10`, main `cb3085365`): 649 / 9 / 4,176 unchanged
    (`450a9e54…` → `b864e7ba…`); 25 of the 40 tail leaves removed in 15
    Unknown functions, no other leaf moves, no leak or violation added; the
    15 left are 6 casts, 3 identity conditionals and 6 leaves of two
    `rdhttp.c` tests whose statements rule 11w's fallback never matches.
    System: no status move, `flb_sched_timer_cb_create`'s 2 leaves removed;
    tail-pause slice byte-identical. `QCore.VariadicTailActuals`
    (`data_iff_nothing_carried`, `every_data_reading_carries_nothing`,
    `borrowed_is_the_whole_root`, `the_data_tail_is_never_widened`,
    `a_cast_is_never_read`, `pointer_arithmetic_is_never_read`,
    `an_identity_arm_is_refused`, `an_identity_condition_is_refused`,
    `an_unanswered_root_is_refused`, `an_integer_identity_is_refused`,
    `the_reading_is_custody_neutral`, the shared `readVectors`);
    [report](../reports/cg26-tail-residual-actuals/README.md).
    Amended by [rule 12h](#constraint-12h): the abstract expression is built
    through a cast the conversion table reads as its operand at a tail
    position (`cast_semantics::tail_transparent`); `a_cast_is_never_read`
    now holds of every other cast.

<a id="constraint-12h"></a>

12h. **A cast is read by its conversion class, from one table, at every
    position: an identity-preserving pointer conversion is its operand's
    identity, an access through it is admitted only as the table's access
    rule allows, and nothing outside the table is read.** Card
    CAST-FORMALIZATION (`cast_semantics.rs`, the route-G adapter
    `legacy_cast_semantics.rs`; thin hooks in `legacy_c.rs`,
    `legacy_return_family.rs::stars_before`, `typed_access_path.rs`,
    `void_view.rs::explicit_view`, the elaborator's object-pointer
    `BitCast`/`NoOp` arm and rule 11b(3)'s `uncast_operand`; shared items
    added: `cast_semantics::{classify, classify_node, CONVERSION_TABLE,
    access_view, transparent_cast, type_group_open, explicit_casts}`). It
    amends rule 11b (and rule 11j's view, and rule 5j's dereference count);
    every other premise of theirs stands. (1) **The table.** A Clang cast
    kind is refined by the operand's and the result's type classes (Clang's
    desugared spellings: scalar, `void`, a pointer to `void`, to a character
    type, to a function or to another object; `_Atomic` and `volatile` noted)
    into one conversion class (C11 §6.3, §6.5.4): `scalar` (data; numbers
    are CG20's), `null-test`, `pointer-to-integer` (provenance lost; rule
    11b's escape), `integer-to-pointer` and `null-pointer` (no identity),
    `byte-view` (an object pointer to `void *` or a character pointer:
    identity preserved, a character access needs no premise, 6.5p7),
    `typed-view` (from `void *` or a character pointer to another object
    pointer: identity preserved, an access needs `P-EFFECTIVE-TYPE`; the
    round trip of a byte view, 6.3.2.3p1/p7), `punned-view` (between two
    other object pointer types: identity preserved, an access is an
    incompatible effective type and is refused, never certified),
    `qualifier` (the same pointee type: the same object; an added
    `volatile` keeps rule 11h), `function-pointer` (CG22's callee set),
    `function-object` (not C: refused), `discard` (`(void) e`: its events,
    no value), and the refused `atomic`, `address-space`, `array-decay`,
    `unlisted`. Every class's kinds are `CAST_TABLE` rows and its identity
    effect is their provenance flag. (2) **Access positions** (a member
    root, a dereference, a subscript base, in the typed access-path
    reader). A `typed-view` (now also from a character pointer) or a
    `byte-view` is a view under rule 11j's premises unchanged (an untracked
    root or a never-bound load, the C02-P-ALIAS check); a `qualifier`
    conversion is read through with no view; every other class refuses.
    (3) **Value positions.** A call actual or a variadic tail actual that is
    an identity-preserving pointer conversion of a place is that place: a
    call the lexical classifier admitted with such an actual is classified
    as its uncast statement, whose answer replaces the original's — a
    soundness repair asked first (the classifier read the actual as no
    place, so a `Borrowed` `(void *) t` after `t`'s release was `Safe`).
    A statement every older route refused, whose identity-preserving
    pointer conversions of a plain name stand at a call actual, a variadic
    tail, a pointer-step operand, an initializer or an assignment value, is
    lowered as its uncast statement by the older routes and kept only where
    they admit it (asked last); in a variadic tail a `scalar` conversion of
    a declared scalar that is not a descriptor name is removed too, and rule
    11w's widening is applied with the original statement's Clang node and
    rule 12g's readings. Rule 12g's reader builds its abstract expression
    through a `scalar` or identity-preserving pointer conversion
    (`tail_transparent`: the operand's own reading, so an integer-carried
    identity or an unanswered root still refuses) and keeps its `cast`
    refusal for every other class.
    (4) **Dereference count.** Before a member or subscript root a cast
    group is transparent to rule 5j's count: `*(T *)q->f` dereferences
    twice. **Do not** remove a cast at an access position (its type is the
    access's), admit an access through a `punned-view` or of a tracked
    root through any view, remove a `scalar` conversion outside a variadic
    tail, give an integer made a pointer an identity, read a function
    pointer as an object, or ask the asked-last hooks before an older route
    (hard invariant 3). Parity runs record every explicit cast
    (`position: "cast-semantics-census"`: kind, class, position, statement
    line) and every value-position decision (`"cast-semantics-value"`).
    Premises stated, not proved: faithful Clang extraction of cast kinds and
    type spellings and the token/AST match (trust boundary 28), C11
    6.3.2.3p7 (`P-ALIGNED-POINTER-CONVERSION`) for a pointer conversion, and
    `P-EFFECTIVE-TYPE` for a typed view's access. Measured (corpus
    `66910c10`, main `9c5f2079c`): **649 / 9 / 4,176 → 654 / 9 / 4,171**
    (`b864e7ba…` → `d13ed4c1…`): `LZ4F_readLE32`, `LZ4F_readLE64`,
    `LZ4F_writeLE32`, `LZ4F_writeLE64` and librdkafka's `unaligned_copy64`
    Unknown → Safe, 34 more rows change leaves (rule 12g's six cast tail
    leaves among them), no Safe → non-Safe move, no leak or violation added;
    system 44 / 2 / 188 unmoved (two functions change leaves), slice
    byte-identical. `QCore.CastSemantics`
    (`classify_vectors_hold`, `agrees`, `view_preserves_identity`,
    `round_trip_same_pointer`, `pointer_to_integer_loses_provenance`,
    `integer_to_pointer_no_identity`, `admitted_access_without_premise`,
    `view_access_needs_the_premise`, `punned_access_refused`,
    `cast_keeps_events`, `discard_is_effect_only`, `star_through_cast_is_two`
    and nine more), bound to `CONVERSION_TABLE`, `classify` and rule 5j's
    reader by shared vectors;
    [report](../reports/cast-formalization/README.md).

<a id="constraint-12i"></a>

12i. **A `return` whose one call the gate refused for its actuals is read
    with rule W44's call-argument access exactly as the same call as a
    statement, and returned only as the return position already accepts.**
    Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (`legacy_return_call_access.rs`,
    a thin hook in `legacy_c.rs::return_statement`; shared items added:
    `return_call_access::is_gate_refusal`). It amends C-GATEWAY-RETURN-CALL-W4
    and B01-RETURN-EXPRESSION; W44's premises and the return position's
    acceptance stand unchanged. Asked last, where every route of the
    position declined or only re-attributed the gate's refusal
    (W43-RETURN-EXPRESSION), for (1) **a whole-call return**: the synthetic
    `PLACE = f(args)` (the reserved place unspelled) refused by the gate,
    which W44 lowers to its block (the typed reads of every base the used
    actuals dereference, their second obligation, the call) whose call is a
    contracted call into exactly `PLACE` with a result
    `escapes_only_fresh_custody` accepts (a derived result only in the CFG a
    derived summary is validated on); the return is then the existing pair
    `PLACE = f(args); return PLACE;` with the reads first; and (2) **B01's
    one call in a scalar expression** (an externally declared callee, the
    surrounding operands scalar) whose call W44 lowers to a contracted call
    with no destination and a `None` result; the exit escapes nothing. C
    evaluates every actual before the call (C11 6.5.2.2p10). **Do not** use
    an actual at a custody-taking position (`W44-CALL-ARGUMENT-CUSTODY-POSITION`
    stays by design), read an uncontracted callee, a descriptor read or a
    declined probe, accept a reallocated result, or ask the rule before an
    older route (hard invariant 3); a declined probe restores the site and
    element counters. Four pins flipped deliberately with statement-form
    controls (`neutral_expression_positions`, `summary_validation_premises`). Parity runs record every consultation
    (`position: "return-call-access"`, its form and answer). Premises stated,
    not proved: W44's and the return position's, and the token/AST match
    (trust boundary 28). Measured (corpus `66910c10`, main `cdc2fa878` plus the card): 40 whole-call and
    1 scalar-call returns admitted of 317 consultations; with rule 12j
    **654 / 9 / 4,171 → 695 / 9 / 4,130** (`d13ed4c1…` → `d2f5a3f7…`), 14
    of the 41 Unknown → Safe directly by this rule (comparators, atomic
    getters, `rd_strcmp3`, `XXH_readLE32_align`/`XXH_readLE64_align`), 12 more
    through callee summaries it validates; no Safe → non-Safe move, no leak
    or violation added (two Unknown functions now carry the return
    position's own refusal of a nullable owned result instead of the gate's).  `QCore.ReturnCallActuals`
    (`the_lowering_is_the_statement_lowering_then_the_exit`,
    `the_reads_precede_the_call_which_precedes_the_exit`,
    `a_statement_w44_refuses_keeps_the_refusal`,
    `a_custody_position_is_never_used`,
    `a_reallocated_result_keeps_the_refusal`,
    `the_scalar_form_returns_no_custody`,
    `the_exit_discharges_nothing_prior` and six more), composed from
    `QCore.CallArgumentAccess` and `QCore.ReturnCallCustody`, bound to the
    gateway by `tests/indirect_access_sole_blockers.rs`;
    [report](../reports/cg10-indirect-access-sole-blockers/README.md).

<a id="constraint-12j"></a>

12j. **A scalar place selected with `.` only from a record object of the
    function's automatic storage is read with no root premise: no pointer is
    read, so no binding is read or changed.** Card
    CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (`record_object_place.rs`, a thin hook
    in `typed_access_path::Reader::path`; shared item added:
    `typed_place::record_place_row`). It amends rules 10k(1) and 10t's
    record-object root; the never-bound premise is unchanged and asked first.
    Where it refused a `.`-rooted place, the place is admitted when (1) every
    step is a `.` member selection (a union or bit-field step is rule 10w's,
    with its decision and frame), (2) the root's Clang declaration is a
    `ParmVarDecl` of the function (C passes a record by value) or a
    block-scope `VarDecl` of it with no storage-class specifier and no
    thread-local storage (C11 6.2.4p5), and (3) the access is a scalar read or
    a scalar store. The place lowers to no base and no chain: no dereference
    obligation exists, and the store writes only the scalar member's storage,
    which no binding names; a store through a union that can hold a pointer
    keeps W43's refused-write frame. **Do not** read a place with a hop
    (`->`, a subscript or `*`), a `static`, `extern`, `register`,
    thread-local or file-scope root, a pointer value, an address, a
    pointer-valued store or a whole aggregate under this rule, or serve the
    never-bound answer from it. Parity runs record each decision as a
    `typed-place` place (`rule: "12j"`, the root's storage, the access and
    the answer). Premises stated, not proved: faithful Clang extraction of
    declaration ids, storage classes and member steps (trust boundary 28).
    Measured on the same run: 544 place decisions admitted (statement and
    position readers, deduplicated), 15 of the 41 Unknown → Safe directly
    (12 librdkafka telemetry value builders, `XXH32_reset`, `XXH64_reset`,
    `cnd_timedwait_ms`); system **83 / 4 / 361 → 85 / 4 / 359**
    (`flb_engine_started`, twice, by rule 12i), slice no status move.  `QCore.RecordObjectPlace` (`decision_vectors_hold`,
    `admitted_exactly_on_a_dot_only_scalar_place_of_automatic_storage`,
    `a_hop_is_never_admitted`, `a_root_outside_automatic_storage_is_refused`,
    `the_never_bound_answer_is_unchanged`, `a_scalar_store_changes_no_binding`,
    `a_union_store_keeps_the_frame` and five more), bound to
    `DECISION_VECTORS` by a shared-vector test;
    [report](../reports/cg10-indirect-access-sole-blockers/README.md).

<a id="constraint-12k"></a>

12k. **The gate's refusal of a call whose uncontracted callee decides names
    that callee: a re-attribution, never an admission.** Card
    CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (`legacy_uncontracted_access.rs`; thin
    hooks, asked last, at the end of `legacy_c.rs::expression_statement` and
    in `return_statement` after rule 12i; shared item added:
    `Parser::reclassified_with_borrowed_actuals` in
    `legacy_call_argument_access.rs`). The lexical gate refuses a statement
    whose call's actuals read memory before the call's contract is looked up,
    and rule W44 reads such actuals only at a declared `Borrowed` position or
    data tail, so an uncontracted callee hid behind `indirect memory access is
    outside the resource projection` and the callee closure (P4) never counted
    it. For exactly the gate's refusal of a statement with no braces that reads
    no tracked descriptor, whose first call names a callee with no contract in
    force and no review or pin hold, the statement is classified again with
    every memory-reading actual of that call read as W44 reads one at a
    `Borrowed` position (the lexical recogniser, else the typed actual of rule
    10k, recording no parity); when that reading is refused with exactly
    ``call to uncontracted function `F` `` for the same `F`, the refusal keeps
    the gate's writes and takes that description. A whole-call `return` is read
    through its synthetic `PLACE = f(args)` and only its returned evaluation's
    description changes. The reading's counters are restored and its
    instruction discarded. **Do not** let the reading lower anything, re-attribute
    a held call (its refusal is `Pending`, a verdict move), an actual neither
    reader recognises (`(*__errno_location())`, a nested call, a cast), an
    access outside the actuals (a member destination) or a descriptor read, or
    ask the rule before any older route (hard invariant 3). The node stays
    `Unknown` with the same writes, so no verdict moves; the diagnostics are a
    set keyed by line and description, so a line holding two refused statements
    that carried one access leaf now shows both. Parity runs record every
    reading (`position: "uncontracted-access"`, its form, callee and answer).
    Nine gateway pins that asserted the gate's text on such a statement were
    flipped deliberately to the named callee, each still `Unknown` (report
    section 6). Premises stated, not proved: that the reading leaves no state behind
    (bound by the corpus comparison) and the token/AST match (trust boundary
    28). Measured (corpus `66910c10`, main `2ecd39531` plus the rule):
    **695 / 9 / 4,130, no status move**, `d2f5a3f7…` → `00462adf…`; 2,962
    access leaves re-attributed in 1,457 functions (access 13,083 → 10,121,
    uncontracted call 8,433 → 11,442); of the 331 functions the two
    uncontracted classes of CG10-INDIRECT-ACCESS-SOLE-BLOCKERS held alone, 215
    are now blocked by uncontracted calls only; system and slice no status
    move (474 and 530 leaves). `QCore.UncontractedAccessAttribution`
    (`the_status_of_a_function_is_unchanged`,
    `the_verdict_of_every_node_is_unchanged`, `the_writes_are_kept`,
    `the_result_is_the_gate_refusal_or_the_callees`,
    `a_held_callee_keeps_the_gate_refusal`,
    `an_unrecognised_actual_keeps_the_gate_refusal` and nine more), bound by
    `tests/indirect_access_sole_residuals.rs`;
    [report](../reports/cg10-indirect-access-sole-residuals/README.md).

<a id="constraint-12l"></a>

12l. **A quantified index check `quant checks P, Idx #i: PRED;` is a check of
    the body and an obligation of every caller, never a postcondition: an
    access through `P` is in bounds only when its index is decided inside the
    admitted set, an access decided outside refutes the declaration, and the
    check never accuses.** Card QUANT-INDEX-CHECKS extends rule 11s's
    vocabulary (contract specification
    [C09.2](../QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks),
    grammar G05c; `qcore_core::index_quant`, the reader
    `qcore_contract::bounds_quant`, the gateway module
    `index_extent/quant_checks.rs` with thin hooks in `index_extent.rs`;
    shared items added: `SourceContractClauseKind::QuantCheck` in the
    structural parser and `BoundsClause::QuantCheck`; no custody code, no hook
    in `legacy_c.rs` or `typed_source.rs`, and resource v1 rejects the clause).
    (1) **Reading.** `#i` is universal over the indices used through the
    pointer formal `P`, in elements of its C pointee type, over the
    mathematical integers; comparisons `==`, `<`, `>`, `<=`, `>=` against a
    bound term (literal, scalar formal, `+`/`-`, one formal added once), `&&`,
    `||` read literally, parentheses; the interval reading is exact. An
    unbounded set, or an empty constant one, rejects the module; the facts
    carry a bounded constant union or the prefix `[0, f)`, and any other
    symbolic form is `bounds-quant-check-no-fact`. (2) **Body.** `P` enters
    with the extent its callers must cover (object `P@entry`, offset
    `-min(lo, 0)`, count to the greatest admitted index), registered as a
    quant object; an element access, dereference or `->` through it (directly,
    through a copy or `P + k`) is decided against the set first: inside, the
    extent decision follows; outside (while live), the row keeps
    `bounds-contract-refuted-by-body` and the unit's callers see an
    uncontracted callee (rule 11s(7)'s two phases), the access recording
    `bounds-quant-check-refuted`; undecided, `bounds-quant-check-undecided`.
    A decision out of bounds against a quant object's seeded extent (a
    formation beyond it, a callee requiring more) is
    `bounds-range-not-contained`, never a violation: the check bounds the
    extent from below only. (3) **Callers.** The obligation is the element
    accesses at the least and greatest admitted index under the actuals'
    values (`both_in_bounds`), or rule 11s's symbol cover for `[0, f)`; a
    violation is `Unsafe` only under rule 11s(6)'s reachability and in-force
    premises, an empty set under the actuals is
    `bounds-inconsistent-premises`, and a callee's return facts are used only
    when this obligation is established. (4) **Escape.** A quant object passed
    to a contracted callee is decided inside only when the callee's constant
    check on that position, shifted by the constant offset, lies inside the
    set; otherwise `bounds-quant-check-escapes` (never a refutation). **Do
    not** let a quant check enlarge a source extent, publish a violation of
    the check itself, decide an access inside on an undecided or symbolic
    index other than the matching prefix, reinterpret `||`, or read a quant
    check in a custody consumer. Premises stated, not proved: faithful Clang
    extraction (trust boundary 28) and that a resolved check describes its
    external body (C06's `Declared`). Measured (corpus `66910c10`, system, tail-pause slice, base `0ebe77f86`,
    binaries `1e028e2f…` before and `a9c41374…` after): every route
    **byte-identical** (corpus 756 / 9 / 4,069 `ce57d194…`, system 85 / 4 /
    359 `878b06d4…`, slice `0fcc81a6…`, the three typed-path parity streams
    equal multisets) and the bounds-selected corpus identical (bounds rows
    `822a7633…`, 162 Safe / 4,759 Unknown): the audit routes carry no bounds
    module, so no quant check reaches them. Through the API, chunkio's header
    `cio_file_st_get_meta_len` and `cio_file_st_set_meta_len` under test-only
    checks are Pending held and Safe resolved; a 16-byte caller written for
    the probe is Unsafe resolved, Unknown held.
    `QCore.IndexQuant` (`mem_reading`, `reading_sound`, `reading_complete`,
    `or_reading_literal`, `over_wide_disjunction_admits_every_index`,
    `bounded_hull_sound`, `empty_admits_nothing`,
    `unbounded_admits_beyond_every_bound`, `inside_sound`, `outside_sound`,
    `an_unknown_index_is_never_inside`, `quant_refusal_never_safe`,
    `quant_refusal_never_accuses`, `covering_the_ends_covers_the_hull`,
    `shifted_within_sound` and 21 more), bound to `qcore_core::index_quant` by
    `admittedVectors`, `decideVectors` and `shiftVectors`
    (`crates/qcore-core/tests/index_quant.rs`) and to route G by
    `tests/quant_index_checks.rs`;
    [report](../reports/quant-index-checks/README.md).

<a id="constraint-12m"></a>

12m. **An object a callee hands its callback (`supplied_owned`) is admitted
    only when every candidate takes it on every return; one that borrows or
    keeps it, or takes it on some return codes only, is refused.** Card
    CG22-OP-CALLBACKS-AND-SERVED-QUEUES (contract specification C04.19.1,
    grammar G05), on the user's decision of 2026-10-14 (clause 1's word).
    `qcore_schemas::callback::CallbackActual::SuppliedOwned`; the loader
    (`contract_callback.rs`) admits it once per actual list of a synchronous
    or member invocation; `callback_contracts::classify` reads a candidate's
    position there as `TakesSupplied` (`resource_consumed` /
    `resource_released`) or `KeepsSupplied` (`resource_borrowed` /
    `resource_owned`), and `legacy_callback_route::Parser::classified` reads a
    candidate with only a return-code disposition (C04.13) as
    `KeepsSupplied`; `decide` refuses `callback-supplied-kept`, and reads a
    call whose every candidate takes it (and is the identity elsewhere) as its
    custody contract: the handed object was never the caller's. The
    requirement a candidate puts on the call (rule 13u) never comes from a
    `supplied_owned` position. A synchronous invocation keeps rule 13y's body
    check (`supplied_owned`, like `_`, matches any actual not naming the
    callback). **Do not** read a borrowing candidate as taking the object, a
    relational candidate as taking it, or a handed object as the caller's.
    `QCore.OpCallbacks` (`a_kept_supplied_object_is_refused`,
    `a_taking_candidate_takes_the_object`,
    `an_admitted_hand_off_never_leaves_the_callee_holding`, the shared
    `suppliedClassifyVectors`, `suppliedDecisionVectors`);
    `tests/cg22_op_callbacks.rs`.

    **Return-code hand-offs** (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF,
    contract specification C04.19.5; USER DECISIONS 2026-10-15, "a callback
    that takes the object only under conditions the caller does not branch
    on is refused"). A candidate whose declaration states `quant live #P:
    #owner |-> #callback when #result OP @C;` (withheld from the function
    map, `RetainingContract::handoff`) is classified, at an invocation
    clause's `supplied_owned`, `_` or parameter position, by
    `legacy_callback_route::handed_class`: a declaration that also carries
    any other side clause reads uncontracted; otherwise the **caller branch
    check** (`crates/qcore-c-gateway/src/handoff_branch.rs`, `admit`) reads,
    over the run's facts (the pre-pass) or the analysing unit's own, the
    path from the invoker along which the callback formal (`Formal`) or
    the handed object (`Member`) flows through direct calls to its
    invocation sites; per site the result is split into the constants the
    body compares it with and the rest (`function_handling`, a structured
    walk over Clang's AST: `if`, loops, `switch`, `goto`, `?:`, `&&`/`||`,
    statement expressions, noreturn calls), and a class the guard admits
    must not name the object again before a rebinding or a noreturn call;
    what the function returns there (the result, a constant, anything) is
    checked at its caller's site; the path ends at a local (the object was
    created there), and at the invoker a handed parameter is never returned
    on an admitted path; no site on the path stores the result through a
    conversion (`Site::result_converted`); some function on the path compares
    the result with the guard's own constant. Admitted, the custody contract decides
    with the handed formal taken at `supplied_owned` and kept by the
    invoker at `_` or a parameter (its requirement on the call kept);
    refused, `callback-handoff-unbranched`. A direct call of a hand-off
    declaration is read only as below (otherwise `callback-handoff-direct`).
    Measured on the corpus
    (`examples/handoff_census.rs` over the 94 librdkafka units, as the
    pre-pass reads them): `rd_kafka_poll_cb`'s `!= PASS` is admitted at
    `rd_kafka_q_serve0`, `_serve` and `_serve_maybe_consume` (the serve loop
    asserts `res != PASS` and never names the op otherwise) and refused at
    the three pop serves (`rd_kafka_q_pop_serve0` returns the op for every
    value outside HANDLED, KEEP and YIELD) and at `rko_serve` (its call
    goes through a local copy of the member); the KEEP-only
    `rd_kafka_txn_op_send_offsets_to_transaction` is admitted at both serve
    loops' `rko_op_cb` (through `rd_kafka_op_handle`, `_handle_std` and
    `rd_kafka_op_call`) and at `rd_kafka_op_destroy` (`rd_assert(res !=
    KEEP)`, glibc `assert` with no `NDEBUG`). Scope (stated, not proved):
    the check reads the invoker's own path, as the decision names it; that
    the invoker invokes the callback only there is the declaration's
    assumption. **Do not** admit a hand-off on a path not read, a guard no
    function on the path compares, a site that stores the result through a
    conversion, a callback formal that escapes, or at a
    direct call outside the reading below. `QCore.ReturnCodeHandOff`
    (`owner_after_is_the_guard`,
    `an_admitted_branch_never_touches_a_taken_object`,
    `an_unbranched_guard_never_admits`,
    `a_converted_result_never_admits`,
    `an_admitted_branch_never_converts_the_result`,
    `admission_composes_through_a_forwarder`, `owner_vectors_hold`,
    `branch_vectors_hold`); `tests/return_code_handoff.rs`;
    [report](../reports/supplied-owned-return-code-handoff/README.md).
    **An exact copy of the member** (card HANDOFF-CANDIDATE-OWN-CALLBACKS,
    C04.19.5 amended): a call through a variable the body writes exactly
    once, `v = o->m` or `T v = o->m;` with no conversion but an lvalue read,
    whose address it never takes, `o` a formal the body never writes, is
    also a site of the member (`handoff_branch::exact_copy`,
    `Site::copy`, followed by `search`); the variable may be a formal
    (`rd_kafka_op_handle`'s `callback = rko->rko_serve`): it then holds the
    member's value or the one it arrived with, and the site's handling is
    the same either way. A written cast, a second write, another member, a
    taken address or a written object is not followed (the member's
    invocation through it is not read, which refuses as before).
    `QCore.CandidateComposition` (`an_exact_copy_calls_a_member_value`,
    `a_copied_local_calls_a_member_value`,
    `an_exact_copy_stays_in_the_closed_set`,
    `an_inexact_copy_may_call_another_value`, `copy_vectors_hold` bound to
    `COPY_VECTORS`, `an_admitted_copy_is_the_only_write`). Measured
    (`examples/handoff_census.rs`): `rd_kafka_poll_cb` at
    `rd_kafka_q_serve0`'s `rko_serve` is now admitted; at the pop serves it
    is refused as at their formal; at `rd_kafka_q_serve`,
    `_serve_maybe_consume` and `rd_kafka_op_req0` the member's invocation is
    still not read (the op is created below the invoker, and
    `rd_kafka_op_req0` records no site).
    **A direct call** (card HANDOFF-DIRECT-CALL-READING, C04.19.5 amended):
    the caller's state for the handed object forks at the call, **taken**
    (the actual `resource_consumed(P, nonnull)`) where the guard holds and
    **kept** (the declared `resource_owned(P, nonnull)`) where it fails, and
    the two join only at the caller's own test of the call's result, read as
    rule 11r reads a return-code disposition: the call tested where it
    stands, or stored by `NAME = f(…);` and tested by the next statement,
    against a non-negative integer constant or an enumerator of the callee's
    own result type (rule 13l's resolution in the analysing unit) that Clang
    reads the body comparing the result with
    (`handoff_direct::read_test`, the rewrite and the `if` plan in
    `legacy_handoff_direct.rs`); each edge takes the owner every class it
    reaches agrees on (`handoff_direct::edge_owner`, over the classes `eq K`
    and `ne K`). First the calling body's own reading must admit the call
    (`handoff_branch::direct_admission`, the analysing unit's Clang AST):
    every direct site of the callee read, the handed actual a pointer
    variable the walk follows, no class the guard admits naming the object
    again before a rebinding or a noreturn call, the result compared with
    the guard's own constant, and the result not stored through a conversion
    (`Site::result_converted`). A body that names the object on a value the
    guard gives the callback refuses `callback-handoff-unbranched`, never a
    finding; a reading not made (no site, an unread construct, an
    unbranched or ignored result, a test not where the result stands or the
    next statement, an edge reaching both classes, a narrowing store)
    refuses `callback-handoff-direct`. **Do not** split on anything but the
    result value, carry the split past a statement the rewrite does not
    read, or admit a direct call whose body disagrees with the guard.
    `QCore.HandOffDirect` (`a_class_owner_is_the_guard`,
    `the_split_state_is_sound_per_class`,
    `a_caller_that_disagrees_never_decides`,
    `an_unbranched_direct_call_never_decides`,
    `a_decided_call_never_touches_a_taken_object`,
    `a_decided_split_is_the_guard_on_every_value`, `direct_vectors_hold`
    bound to `DIRECT_VECTORS`); `tests/handoff_direct_call.rs`;
    [report](../reports/handoff-direct-call-reading/README.md).

<a id="constraint-12n"></a>

12n. **A member invocation (`resource_callback_member_invoked(OBJ,
    TAG.FIELD, A…)`) is decided only over a closed set of the stores into the
    member, in the unit and then in the run; an open, empty or uncovered set
    is never decided.** Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (contract
    specification C04.19.3), on the user's decision of 2026-10-14 (clause 3,
    dotted `TAG.FIELD`, premise `P-CG22-MEMBER-CLOSED-WORLD`).
    `crates/qcore-c-gateway/src/callback_member_closure.rs` (`MemberFacts`,
    collected from every top-level declaration of the unit's Clang AST beside
    rule 11p's tables; `closure`, the least set over any units);
    `legacy_callback_route::Parser::member_candidates` decides a call over the
    unit's own set (rule 13u's composition; `callback-member-open` for an open
    or empty set, naming why); the facts are recorded per unit
    (`contract_dependencies::member_evidence`, from rule 13y's `settle`
    hook, whenever a member invocation is in the environment), and
    `crates/qcore-audit/src/callback_member_fold.rs` folds them over the run
    after every unit: a `Safe` or `Pending` row whose `contract_dependencies`
    name a member-invoking declaration stays only when the run's set is closed
    and a subset of its unit's, otherwise it is demoted to `Unknown` with the
    reason; a unit row with no facts opens every member; the sidecar's
    `callback_members` section lists each member's run closure. Card
    CALLBACK-MEMBER-DATA-ONLY-UNITS: a data-only unit (no definitions, so never
    analysed) is read whole for its facts alone
    (`callback_member_closure::read_unit` over its preprocessed text, dialect
    and frontend flags, from `projection`'s data-only branches through
    `callback_member_fold::data_only_facts`), so a unit with no store
    contributes the empty set read from its AST; a unit whose preprocessing,
    Clang run or JSON fails, or whose root is not a translation unit with a
    declaration array, records none and opens every member. An initializer
    list of a record the unit defines once at file scope is mapped onto its
    fields (unnamed bit-fields skipped, a union by its `field`): each
    function-pointer field's entry is a store into `TAG.FIELD`, an implicit
    entry a null pointer. A formal
    stands for its callers' actuals only when it is a function pointer and its
    function's address no unit takes (an internal function's in its own unit);
    a variable, a call's result, a cast, a non-function-pointer formal, an
    address-taken member and a record an initializer list that cannot be
    mapped so (an anonymous member, a tag defined again inside a function, an
    entry count that differs, another field's entry that is a record or array
    copy or a function), a whole-object assignment or a `memcpy`-family call
    may write open it; a checkpoint
    composer refuses a run whose contracts invoke a member unless its
    checkpoints carry the run's facts (card MEMBER-PREPASS-OTHER-ROUTES,
    below). A member invocation is a declared assumption (rule 13i). **The
    unit's closed set orders the W4 derivation** (card
    RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN):
    `callback_route::callback_edges` gives a body that calls a
    member-invoking declaration an edge to every in-unit function of the
    member's closed set in its unit (an open set adds none), so the
    candidates are summarised before their caller. Without it a formal
    released through the call read `UnknownAlias`: the caller's summary was
    derived first, read the candidates as uncontracted there and refused, so
    the reported run (which decides the member as for a tracked local) never
    bound the formal. This is ordering, not a soundness guard: whether a
    candidate may alias the formal is answered, in every run, by its own
    contract (a candidate that releases what it receives or a global, keeps
    something or has none is not the identity and refuses). **The
    candidates' own contracts** (card OP-CALLBACK-CANDIDATE-CONTRACTS):
    the op callback protocol reads `PASS` (the op stays with
    `rd_kafka_op_handle`) and `HANDLED` (the handler destroys it) as an
    `rko_op_cb` candidate borrowing the op, and `KEEP` and `YIELD` as it
    taking the op; `rd_kafka_op_destroy`'s call (`RD_KAFKA_RESP_ERR__DESTROY`,
    neither `KEEP` nor `YIELD` asserted) borrows it; an `rko_serve`
    candidate owns the op on every result but `PASS`. So an `rko_op_cb`
    candidate is declared only as the identity (every formal borrowed,
    `resource_return_none()`) when it returns only `HANDLED` or `PASS` and
    releases, keeps or moves out nothing the op holds, and an `rko_serve`
    candidate only as taking the op (`resource_consumed(rko, nonnull)`, rule
    12m) when it never returns `PASS`. A candidate taking the op on `KEEP`
    only (C04.13 states no disposition over the unsigned
    `rd_kafka_op_res_t`, and rule 12m refuses it anyway), one that releases
    or moves out an object the op's members hold, and one calling an
    application callback stay undeclared: 19 of the 30 non-poll candidates
    are declared (`client.qv`, each body read at `66910c10` and `046accf6`,
    pinned, every one a declared assumption carrying the premise
    `rk_log_callback` for the application's `rk_conf.log_cb`), 11 are
    residuals (nine in the amended `OP-CALLBACK-CANDIDATE-CONTRACTS`, two in
    `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`, after
    `OP-APP-RK-CALLBACKS-VIA-POLL-CB`). A unit decides a member only over its
    own closed set and the fold keeps a row only when the run's set is in
    it, so a declared candidate frees no row by itself; measured, the 13
    `callback-uncontracted` leaves of `rd_kafka_op_destroy` in `rdkafka.c`
    and `rdkafka_offset.c` decide and no status moves.
    [Report](../reports/op-callback-candidate-contracts/README.md).
    **Application rk_conf callbacks** (card OP-APP-RK-CALLBACKS-VIA-POLL-CB,
    C04.19.3 and C04.19.4 amended; USER DECISIONS 2026-10-14, "Closed over
    FB's registrations" and "Build the agent's recommended set": D1, Q2, Q3,
    U2, R1, F1, E1, S1). *Keys:* the callback closure keys a member by its
    innermost named record (`MemberFacts::callback_key`; `rk->rk_conf.F`
    and `conf->F` are `rd_kafka_conf_s.F`, so `rk->rk_conf = *conf` copies
    values already counted), a union's members share one key (`U.*` or
    `TAG.PATH.*`), and a declared spelling is normalised through the run's
    record tables (`callback_member_closure::Tables`: an opaque
    whole-object write of a record holding the member by value, a
    `memcpy`-family call or an assignment from another type, opens it, and
    its initializer list does not; a union key aliases the records it
    embeds). *Registrations:* every call of a declaration stating
    `resource_callback_member_registered(P, TAG.FIELD, F)` with `P` a
    parameter (a setter) is a store of its `F` actual, and a constructor's
    stated function default (`quant live #result: TAG.FIELD |-> f;`) a store
    of `f` (`member_run_facts::declarations_unit`). *The run's set:* the
    pinned route's pre-pass (`qcore_audit::member_prepass`) preprocesses
    every unit of the production compile database with its own command,
    reads every C unit naming a token of an invoked member's closure graph
    (record tags that can carry a store, setters and formals' functions,
    iterated to a fixed point) and installs the run's facts on each
    analysing thread (`member_run_facts::scoped`); a unit then decides over
    the run's set, a closed empty set invokes nothing (E1,
    `callback_contracts::decide_run`), a member that is not a
    function-pointer member of a record the run defines refuses, a unit that
    names a token but is not read opens every member, and the profile's
    `member-facts-scope.json` names the units outside the program with their
    evidence (the unlinked `rdkafka++` wrapper; generated units and units
    whose generated headers are absent from the pinned inputs, their raw
    texts checked); the fold compares the run's set with itself and stays
    demote-only. **Every route reads the run's set** (card
    MEMBER-PREPASS-OTHER-ROUTES, amending this rule in place; the ledger
    entry's DISCHARGE): the current route runs the same pre-pass over its
    whole `--compile-database` (a selection chooses the units analysed, not
    the program) plus the Monkey units it analyses, and the no-build route
    over its selected units under their assumed compile records (no profile,
    so nothing is excluded); both install the facts per unit
    (`CurrentAudit`/`NoBuildAudit::set_member_prepass`) and fold over them. A
    checkpoint (schema 5) carries the run's facts: their digest in the run
    identity (a changed run set recomputes every unit and never composes with
    another), the fold's inputs (`callback_member_fold::FoldEnvironment`)
    and the run's closure of each invoked member in `global`, each unit's
    member and state facts in `contracts`; the composer folds them as the run
    does (`fold_in`) and refuses a composition the fold would demote, since
    it never rewrites rows; checkpoints without the run's facts refuse a
    member-invoking run as before. **The scope file is re-checked at run
    time:** `member-facts-scope.json` (schema 2) names its
    `evidence_revision` and each entry's typed evidence, which the pre-pass
    re-verifies against the run's own sources inside the token fixed point,
    against the final tokens: `tokens-absent` (the unit's raw text, every
    file Clang resolves for it under its own command with `-M -MG`, and the
    named generator sources, a file with the files its directives resolve
    or a directory with every file under it, name no token, and every
    unresolved include is named as generated) or `not-linked` (the named
    link lines above the unit's `CMakeCache.txt` name the archive and not
    the wrapper). An exclusion that does not hold or cannot be checked is
    dropped, recorded in the section's `scope.dropped`, and its unit read
    like any other (an unreadable one opens every member). **An unread C++
    unit is scoped to the members it names** (card
    CALLBACK-MEMBER-NON-C-UNITS, amending this rule in place): a C++ unit
    (`.cpp`, `.cc`, `.cxx`, `.cp`, `.c++`, `.C`) that names a final token but
    is not read, whose `-E` text under its own command is the whole unit (no
    `-include`, `-Xclang`, precompiled-header or module flag in its command,
    no module declaration, import or `#pragma clang module` in its text),
    opens only a member whose own closure-graph tokens its text names, or
    one with a graph token it was not checked against
    (`member_run_facts::RunMemberFacts::opens`, `scoped`, `checked`;
    `member_prepass::scoped_cxx`). This is the skipping reading above
    (C++, like C, reaches a member only through its record's definition,
    which spells its tag or a typedef of it, and a function only through its
    name), applied to each member's graph instead of the union of every
    member's. A failed preprocessing, an assembly unit (which writes
    through offsets and symbols no record names), a C unit whose AST is not
    read, and a C++ unit whose text is not the whole unit still open every
    member; the lifetime standing (S3) still reads every unread unit as a
    declared assumption. The section's `non_c_units` lists each scoped
    unit's tokens and the units opening each member; the facts digest
    covers them. At Fluent Bit `046accf6` the current route's two unread
    units are `lib/ripser-1.2.1/ripser.cpp` (names only libstdc++'s `event`,
    an S3 lifetime token) and `src/simdutf/flb_simdutf_connector.cpp`
    (names `cmt`, `event`, `evl`): the rk_conf members, `consume_ctx` and
    `rko_serve` close, `cmt.log_cb` stays open on the connector.
    [Report](../reports/callback-member-non-c-units/README.md).
    Measured (corpus `66910c10` on main `b7facc899`): 936 / 9 / 3,889 → 936
    / 9 / 3,889, 97 Unknown rows trade leaves; the pre-pass reads 845 of
    1,444 units, none unread; the rk_conf members close (`dr_msg_cb`
    {`cb_kafka_msg`}, `log_cb` {`cb_kafka_logger`, `rd_kafka_log_print`},
    `sasl.oauthbearer.token_refresh_cb` {Fluent Bit's and librdkafka's five},
    the other seven empty) and `rd_kafka_op_s.rko_op_cb` reads open (its
    union-keyed `rko_u` dependency).
    [Report](../reports/op-app-rk-callbacks/README.md). **Do
    not** decide a member over the unit's set alone where the run's facts
    are installed, exclude a unit whose evidence the run has not
    re-verified, compose a member-invoking run from checkpoints without the
    run's facts, promote a row in the
    fold, follow an address-taken function's formal, read a value the
    facts do not name as no candidate, treat a unit whose AST was not read
    (or a unit absent from the run) as closed (a scoped C++ unit keeps only
    the members whose graph tokens it does not name; never an assembly unit,
    a failed preprocessing or a text that is not the whole unit), scope a
    unit by tokens it was not checked against, or derive a member-invoking
    caller before its closed candidates. `QCore.OpCallbacks`
    (`an_open_member_never_decides`, `an_open_run_demotes`,
    `a_kept_decision_covers_every_run_candidate`,
    `an_unread_unit_keeps_the_member_open`, `an_unread_unit_demotes`,
    `a_read_unit_without_a_store_adds_nothing`,
    `the_run_set_is_exactly_the_read_sets`, `run_vectors_hold`);
    `QCore.MemberSummaryOrder`
    (`a_closed_member_candidate_precedes_its_caller`,
    `an_uninstalled_candidate_refuses_at_summary_time`,
    `a_formal_reads_as_the_local_once_candidates_precede`,
    `the_formal_is_never_safe_where_the_local_is_not`,
    `a_candidate_that_could_alias_keeps_the_formal_unsafe`);
    `callback_member_closure`'s `CLOSURE_VECTORS`; `callback_member_fold`'s
    `RUN_VECTORS`; `tests/cg22_op_callbacks.rs`,
    `tests/callback_member_data_only.rs`;
    [report](../reports/retained-release-formal/README.md).
    `QCore.AppRkCallbacks`
    (`a_registration_closes_the_member_only_with_every_run_store`,
    `the_registered_set_is_exactly_the_read_and_registered_sets`,
    `an_open_run_member_never_decides`,
    `a_closed_empty_run_set_invokes_nothing`,
    `a_unit_empty_set_stays_refused`,
    `skipping_a_silent_unit_keeps_the_run_set`,
    `an_embedded_store_and_a_direct_store_share_the_key`,
    `normalise_is_idempotent`, `a_stated_default_is_a_candidate`,
    `key_vectors_hold`, `run_decision_vectors_hold`);
    `callback_member_closure`'s `KEY_VECTORS`; `member_run_facts`'s
    `RUN_DECISION_VECTORS`; `tests/app_rk_callbacks.rs`.
    `QCore.MemberPrepassRoutes`
    (`a_named_token_drops_the_exclusion`,
    `a_stale_exclusion_never_closes_a_member`,
    `held_exclusions_keep_the_run_set`,
    `a_checkpoint_without_facts_refuses`,
    `the_composed_decision_is_the_runs`,
    `a_run_decided_over_its_facts_composes`);
    `crates/qcore-audit/tests/member_prepass_routes.rs`;
    [report](../reports/member-prepass-other-routes/README.md).
    `QCore.MemberNonCUnits`
    (`an_undecided_unit_keeps_the_member_open`,
    `a_named_graph_token_keeps_the_member_open`,
    `an_unchecked_graph_token_keeps_the_member_open`,
    `a_scoped_unit_naming_no_graph_token_holds_no_store`,
    `a_scoped_unit_naming_no_graph_token_keeps_the_run_set`,
    `the_closed_run_set_covers_every_store`, `scoped_vectors_hold`);
    `member_run_facts`'s `SCOPED_VECTORS`;
    `crates/qcore-audit/tests/callback_member_non_c_units.rs`.
    **A union's places by layout** (card RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION,
    C04.19.3 amended; the brief's second evidence, Clang's record layout, as
    the discriminant route needs a stated arm map and an op-type analysis
    over 115 sites without constant local evidence): a place under a union
    keeps its arm path beside the union's key (`U.*@arm.path`,
    `callback_member_closure::ARM`); `read_unit` (the pre-pass and data-only
    units) reads Clang's layout of every union its keys name in one
    enumerator probe (`MemberFacts::probe_layouts`, `UnionLayout`: the
    range of each arm path, function-pointer member and record held by
    value, transitively); `Tables::aliases` keeps two places of one union
    apart only on disjoint known bytes (`places_alias`), a record the union
    holds only where none of its member's placements meets the place
    (`meets_held`), and an opaque write of a held record notes the place
    only where a placement meets it (`meets_record`); a missing range, an
    unnamed member, an array of records and two units that disagree
    (`merge_layout`) stay shared. A `memcpy`-family call into a place the
    union holds and an address of one converted to another pointer type
    (a `memset`-family filler excepted) write the whole union opaquely, and
    an anonymous arm assigned whole from another type writes its own bytes.
    An anonymous record (Clang's `struct T::(unnamed at …)`) is no longer
    read as its holder `T` below a named root: `rko_u` was keyed
    `rd_kafka_op_s.*` (the op's whole record, so `rko->rko_replyq` read as
    an arm) and a place below an anonymous structure lost its path. Measured
    over librdkafka's units, `rd_kafka_op_s.rko_op_cb` closes over its 27
    candidates and `rko_u.*@leaders.cb` over two; weaker than a discriminant
    and specific to the corpus's target. **Do not** separate two places on
    type alone, read an ungiven range as disjoint, or merge two units'
    differing layouts. `QCore.UnionArmLayout`
    (`a_disjoint_write_never_changes_the_read`,
    `a_separated_store_never_changes_the_read`,
    `an_ungiven_range_aliases`, `a_disagreeing_union_aliases_everything`,
    `the_layout_takes_only_shared_stores`, `layout_vectors_hold`);
    `LAYOUT_VECTORS`; `tests/rko_union_layout.rs`;
    [report](../reports/rko-op-cb-union-closure/README.md).
    **Opaque writes keyed by target** (card
    UNION-OPAQUE-RECORD-WRITES-BY-TYPE, C04.19.3 amended): every opaque
    write of a record (`MemberFacts::opened`) keeps its target expression
    (`opened_targets`, `OpaqueTarget`, read as `callback_key` reads an
    assignment's place): a place a union holds (`U.*@arm.path`; an
    assignment there is also an open store at that place, so its own bytes
    decide by `places_alias`), a separate object (a local, global or
    by-value formal, or a member path in one), a member path below a pointer
    to a named record, or untargeted (a dereference, an array element, a
    pointer handed to a copier, an initializer list). A union target reads a
    held record's write by the record's placements (`meets_record`) only
    where the write may lie in that union (`Tables::may_lie_in`,
    `opaque_write_may_lie`): untargeted, into another union, below a record
    the union holds by value, or on a path the tables do not read as
    meeting no union (`path_avoids_unions`); facts without targets are read
    by type as before. Non-union readings (a member keyed by its record, its
    containers) stay type-keyed. **A declared spelling keeps its arm**:
    `Tables::normalise` keys a path into a union `U.*@rest`, as a read of
    the same place (it was the whole union, so the layout never applied to
    a declared member: `rd_kafka_op_s.rko_u.offset_commit.cb` aliased every
    place and held record of `rko_u`). Measured (corpus `66910c10` on main
    `1c7333db1`): 938 / 9 / 3,887 unchanged, the four rows naming the union
    trade one leaf each; `rko_u.offset_commit.cb` and `rko_u.terminated.cb`
    close over the four candidates on union bytes 8..16 (three arms share
    them), none contracted. **Do not** key a write into a union by its
    record's type where its target is read, read a separate object or an
    unheld record's path into a union, or read a path the tables do not
    give as avoiding one. `QCore.UnionArmLayout`
    (`the_target_reading_only_narrows_the_type_reading`,
    `an_untargeted_write_is_read_by_type`,
    `a_separate_object_never_keys_into_the_union`,
    `a_write_below_an_unheld_record_never_keys_into_the_union`,
    `a_union_write_is_decided_by_its_place`,
    `a_separate_write_never_changes_the_read`,
    `a_non_overlapping_opaque_write_never_changes_the_read`,
    `an_unknown_range_opaque_write_aliases`, `opaque_vectors_hold`);
    `OPAQUE_VECTORS`; `tests/union_opaque_writes.rs`;
    [report](../reports/union-opaque-record-writes/README.md).
    **A candidate's own member invocations** (card
    HANDOFF-CANDIDATE-OWN-CALLBACKS) are decided by this rule as in the
    candidate's own body, over the run's set only, when the candidate is
    composed at an invocation (rule 13u's amendment).
    **A registration forwarded through an id** (card
    FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, contract specification
    C04.19.3; USER DECISION 2026-10-16, "Result + list"):
    `resource_callback_member_registered(#id, TAG.FIELD, F) where #id =
    #result in P.LIST;` (`contract_callback.rs` `by_id_clause`;
    `CallbackClause::MemberRegistered::by_id`, `IdLink`, omitted when
    absent) is read everywhere as the setter form: at a call it changes
    nothing on the caller's state, and for the run's closure every call is a
    store of its `F` actual (`callback_member_fold::declared_stores`
    unchanged). Its body check (`crates/qcore-c-gateway/src/registration_by_id.rs`,
    hooked in `legacy_callback_route::settle`, evidence `12n`): in a unit
    that defines the setter, the clause stands only where the body, or the
    one in-unit helper it hands `F` and `P` whole to, creates exactly one
    local `struct TAG *` record from a declaration whose result is owned,
    links it once into `P->LIST` with a declaration publishing its first
    actual into its second, stores `F` once into the member (no other
    access to it) and returns the record's one id member or a negative
    constant (a helper: the record or a null constant), the setter naming
    its record only through member accesses off the member, a null test
    and that return; anything else withdraws the declaration there
    (`callback-withdrawn`); elsewhere a declared assumption (rule 13i). A
    view `&r->H` handed to a call reaches that member only (premise
    `P-PATH-MEMBER-VIEW`, rule 30's reading). **Do not** read the id as the
    record at a later call (no consumer resolves it yet), validate a body
    that creates or links twice, or follow a helper beyond one level.
    Declarations: the three collector setters in `quirin-provider-fluent-bit`
    (`fluent_bit/input_collector_registration.qv`, bodies identical at
    66910c10, 046accf6 and 90978ac6, pinned at all three).
    `QCore.RegistrationById`
    (`a_forwarded_registration_is_the_in_body_store`,
    `an_unresolved_or_ambiguous_id_never_decides`,
    `two_records_never_validate`, `an_unlinked_record_never_validates`,
    `vectors_hold`); `VECTORS`; `tests/event_loop_retention.rs`;
    [report](../reports/fb-event-loop-retention/README.md).

    **A function pointer is a pointer to a function type** (card
    LOG-PRINT-BODY-CONTRACTS, a fail-closed repair).
    `MemberFacts::function_pointer` and the typedef reading accept a type
    spelling as a function pointer only with a parameter list after `(*)`
    (`spells_function_pointer`; a function typedef only from a function type,
    `spells_function_type`). Clang keeps a declarator's parentheses as sugar,
    so `void (*f)` (a `void *`) is spelled `void (*)` and an array pointer
    `int (*)[N]`, and both were read as function pointers: a `void *` formal
    stored into a member forwarded only its callers' function-pointer
    actuals, which a `void *` actual never records, and closed the member
    (empty in the fixture; chunkio's `cio_set_log_callback(struct cio_ctx *ctx,
    void (*log_cb))`, `chunkio.c:316-319`, storing into
    `cio_options.log_cb`). Such a formal now opens the member ("a formal that
    is not a function pointer"), as a formal spelled `void *` always did; the
    repair only opens member sets. **Do not** read a function pointer from
    `(*)` alone. `QCore.MemberFormalType`
    (`a_non_function_formal_store_is_open`,
    `an_open_unit_keeps_the_member_open`,
    `a_non_function_formal_store_never_decides`,
    `the_defect_would_close_the_member_empty`);
    `tests/log_print_member_contracts.rs`;
    [report](../reports/log-print-body-contracts/README.md).

<a id="constraint-12o"></a>

12o. **A type-switched release reads, under an undecided discriminant, the
    cautious join over its arms; a value no arm and no `otherwise` declares is
    refused, never neutral; the arms stand in a unit that defines the body
    only where its switch hands exactly the declared members to calls.** Card
    CG22-OP-CALLBACKS-AND-SERVED-QUEUES (contract specification C04.19.2), on
    the user's decision of 2026-10-14 (clause 2's per-arm `when` /
    `otherwise` form). `qcore_schemas::arm_release` (`Discriminant`,
    `ArmClause`, `join_at`), the loader `contract_arm_release.rs`
    (`requires resource_discriminant(P, FIELD, MASK)`,
    `resource_arm_released(P, PATH)` / `resource_arm_empty(P)` with `when
    FIELD == VALUE` or `otherwise`), `RetainingContract::{discriminant, arms,
    arms_withdrawn}`; route G (`legacy_callback_route::ArmOperands`,
    `apply_arms`): with no `otherwise`, every call refuses (`arm-undeclared`);
    after the declared release, an object linked at a member every arm
    releases is released, one some arms release refuses (`arm-undecided`),
    one no arm releases stays (premise P-B08-SHALLOW). **A decided
    discriminant** (card OP-CONSTRUCTOR-DECLARATIONS, C04.19.4, on the
    user's decision of 2026-10-14, A1): a constructor declaring
    `resource_discriminant_set(result, TAG.FIELD, P, FLAGS)` puts on its
    tracked result's site, per path, the value `actual | FLAGS` when Clang's
    call folds the actual to a constant (a formal or computed actual decides
    nothing); the join keeps a value only when every arriving path holds it
    (`legacy_constructed_route.rs`). At the release, an object whose path
    state decides `FIELD` reads, at each linked member, the join of the arms
    its reachable values select (the decided value closed under the run's
    classified flag-ors and case-guarded rewrites, rule 14e;
    `constructed_state::decided_join`), each value read under the release's
    own mask; a reachable value with no arm and no `otherwise` refuses
    (`arm-undeclared`). The state is read only where the unit's stores keep
    it and the calling function stores into `FIELD` nowhere itself; anything
    else reads the cautious join. Body check
    `crates/qcore-c-gateway/src/arm_release_check.rs` (evidence `12o`, rule
    13i's `ClauseKind::Arm`): one `switch` reading `P->FIELD` (`& E`, `E`
    folding to `~MASK`); case groups read with stacked labels and
    fall-through, enumerator labels folded from the unit's enumerations; each
    value's payload paths (member paths of `P` under the declared arms' first
    segments, handed as a call's actual through `&`, casts and a subscript's
    base) equal to the declared ones, an unlabelled value taking `default:`;
    a `break` under an `if` outside a loop refuses; otherwise the declaration
    is withdrawn in that unit (`arm-withdrawn`). **Do not** read an
    undeclared value as releasing nothing, decide a discriminant at a call,
    or read the arms' calls as proved releases; decide a discriminant from
    an undeclared constructor, a type actual that does not fold, or a state
    the run's stores do not keep. `QCore.OpCallbacks`
    (`an_undeclared_arm_is_unknown`, `the_cautious_join_is_sound`,
    `a_call_without_otherwise_is_refused`, `an_undecided_link_is_refused`,
    `armJoinVectors`; OP-CONSTRUCTOR-DECLARATIONS:
    `a_decided_discriminant_selects_exactly_its_arm`,
    `a_decided_value_with_no_arm_is_unknown`,
    `a_rewrite_within_one_arm_keeps_it`,
    `the_decided_join_refines_the_cautious_join`,
    `an_undecided_discriminant_gives_the_join`,
    `an_unstable_discriminant_gives_the_join`, `decidedJoinVectors`);
    `QCore.BodyClauseStanding` gains `arm` and `r12o`;
    `tests/op_constructor_declarations.rs`.

<a id="constraint-12p"></a>

12p. **The queue serving family and `rd_kafka_op_destroy` are declared with
    the three clauses, decide nothing in the corpus, and the served-queue
    premise stays.** Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES. In
    `qcore-provider-librdkafka/contracts/librdkafka/client.qv` (written by the
    report's `scripts/write_contracts.py`, every body read at `66910c10` and
    `046accf6`, pinned at both): `rd_kafka_q_serve0`, `_serve`,
    `_serve_maybe_consume`, `rd_kafka_q_pop_serve0`, `_pop_serve`,
    `_pop_serve_maybe_consume` (every formal borrowed;
    `resource_callback_invoked(callback, _, _, supplied_owned, cb_type,
    opaque)`; the op's `rd_kafka_op_s.rko_serve` taking it the same way and
    its `rko_op_cb` borrowing it; the pop variants' `PASS` op an owned
    nullable result) and `rd_kafka_op_destroy` (`resource_released(rko,
    nonnull)`, the op's `rko_op_cb` over `(_, _, rko)`, the discriminant
    `rko_type` under `RD_KAFKA_OP_FLAGMASK = 1610612736`, 169 arm releases
    over 60 values and `otherwise` empty, as rule 12o's reading of the body
    gives them). The declared-ABI reader (`legacy_declared_abi.rs`,
    `read_function_typedef`) reads one pointer to a function-type typedef
    (`typedef R (T)(…)`, `T *callback`, as `rd_kafka_q_serve_cb_t *`) as the
    same `fnptr` the pointer typedef and the inline declarator read; the type
    itself and a deeper pointer stay unread (`tests/loader_abi.rs`). Every corpus callback candidate (`rd_kafka_poll_cb`,
    `rd_kafka_consume_cb`, `rd_kafka_background_queue_serve`) calls a callback
    the application registered in `rk` (the pending `rd_kafka_destroy_app`
    decision, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`) and has no contract; the op
    members' candidates have none (`OP-CALLBACK-CANDIDATE-CONTRACTS`); the
    four forwarders hand a formal on (`SERVE-FORWARDED-CALLBACK-FORMALS`). The
    served-queue premise (C04.18) on `rd_kafka_q_wait_result` and
    `rd_kafka_q_purge0` is unchanged. Measured (corpus `66910c10` on main `579c2e6e2`, binary `21989e3e…`): **935 / 9 / 3,890 → 935 / 9 / 3,890** (`bdf5f1c8…` → `527a4cad…`), no status move, 102 Unknown rows trade leaves (`call to uncontracted function` 155 → `callback-member-open` 119, `callback-uncontracted` 16, `callback-withdrawn` 5 in `rdkafka_queue.c` by rule 13y, `callback-undecided` 1, access-gate refusals at member actuals 14); rule 12o validates `rd_kafka_op_destroy`'s arms in `rdkafka_op.c`; both op members close over the recorded units (27 and 4 candidates) and the fold reads them open for four data-only units (`CALLBACK-MEMBER-DATA-ONLY-UNITS`), demoting nothing; system 93 / 4 / 351 and slice 52 / 1 / 347 / 4 Pending, no row moves; the null property byte-identical on all three;
    [report](../reports/cg22-op-callbacks-served-queues/README.md).

<a id="constraint-12q"></a>

12q. **State a consultation, scope or lowering sets belongs to it and is gone
    on every exit: nothing one consultation leaves on a worker thread reaches
    another.** Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC
    (`consultation_decisions.rs`; shared items added: a `decisions` parameter
    on `StatementIndex::statement_with`, `condition`, `assignment`,
    `returned` and `argument`, on `typed_place::tracked_chain` and
    `record_object_place::record`; `typed_access_path::Reader::recording` and
    `Reader::decisions`; `Parser::call_graph_components`). A typed
    access-path reader's parity decisions (rule 10t's and 12j's places, rule
    10v's classifications) go into a `Decisions` value that the consumer which
    records them owns and hands to the reader; a reader without one keeps
    nothing, and the value is dropped with the consumer's frame on every exit
    path, early returns and declined probes included. Thread-local state that
    scopes a lowering (rules 10z and 11m's edge, edge root and deciding
    operand, route G's lowering flag, the analysed source) is set by a drop
    guard that restores the previous value, an unwind included. A cache that
    outlives one consultation is keyed by the complete identity of what it
    answers (GATEWAY-ANALYSIS-PATH-OPTIMIZATION's launch caches), never by an
    address. **Do not** add a thread-local list one consultation fills and a
    later one drains, a set-then-reset-to-a-constant pair around a call that
    can nest, or a cache keyed by a pointer. The replaced state never reached
    a verdict: the decision lists were written only on parity runs and read
    only into parity rows, and the address-keyed call graph only into parity
    rows. No Lean change: the rule moves parity bookkeeping and restores, and
    no decision table, transfer or verdict changes. Measured (corpus
    `66910c10`, system and tail-pause slice, main `0ebe77f86` plus the rule,
    and again on `cdad4dfd0`):
    every `resource-diagnostics*.json` and sidecar byte-identical (756 / 9 /
    4,069, `ce57d194…`); the corpus parity stream, which five runs of the
    BEFORE binary wrote in two multisets, is one multiset over five AFTER
    runs; the 122 removed `typed-place` rows (43 kinds, 30 functions) were
    leftovers written by statement consultations whose own node did not
    match; [report](../reports/gapo-parity-decisions-nondeterministic/README.md).

<a id="constraint-12r"></a>

12r. **A bounds declaration reaches an audit route only from its provider
    crate's bounds view, and is in force there only under its pin: a
    declaration whose pin is not in force is held (`contract-pin-mismatch`),
    discharging nothing and accusing nothing, never Safe.** Card
    CHUNKIO-META-LEN-BOUNDS-CONTRACTS (the user's request to write the listed
    `cio_file` functions' contracts; contract specification C09.1 and
    C06.2): the new audit module `bounds_selection.rs` with thin hooks in
    `projection.rs` (`record_bounds` reads the installed environment),
    `report.rs` (the selection record beside the rows), `pinned_route.rs`,
    `current.rs` and `no_build.rs` (one `select_for_route` call after the
    resource pins); shared items added: `LoadPathManifest::bounds_views` and the
    crate record's `bounds_view` (`qcore_contract::load_path_manifest`),
    `BoundsFunctionContract::pin_hold`, `PinHold`,
    `BoundsContracts::hold_for_pin_mismatch` and the hold in `held()`,
    `holds()` and `digest()` (`qcore_contract::bounds_contract`),
    `qcore_core::index_extent::CONTRACT_PIN_MISMATCH` (a held clause carrying
    it is reported under that code, not `awaiting-human-review`), the bounds
    view in `probe_resource_views`, `pin --bounds` (`contract_pin.rs`, its
    targets now carry their declarations), and the operator gate's admission of
    a `quant checks` predicate's `&&`/`||` (`pipe.rs`, G05c); no custody code
    reads any of it. (1) **Placement.** A bounds contract v1 module lives in
    its library's provider crate and is listed in the crate record's generated
    `bounds_view` (the owned modules the bounds loader accepts alone; all views
    load together), disjoint from its `resource_view` (the Rust reader refuses
    a module in both), so no resource selection loads it and resource v1 keeps
    rejecting its vocabulary. (2) **Selection.** With `QCORE_BOUNDS_PROPERTY`
    set, the pinned route, the configured route and the contracts route load
    every bounds view on the load path their resource manifest names
    (`all-bounds-views`, crate order, keyed `crate:module`) before any unit is
    analysed and analyse every unit's bounds rows under it; the record
    (`ROWS.contracts.json`: modules and digests, environment digest, pin
    section) is written beside the rows. Unset, nothing is read or installed;
    a manifest naming no load path installs the empty environment, said so.
    (3) **Pins.** Every bounds declaration is pinned (no flexible mode; **amended by
    CELL-GUARD-RESULT-RANGE-CHECKED, rule [38e](#constraint-38e)**: a C library
    `result_set` of `quirin-provider-libc` is the one flexible bounds
    declaration, and a derived range's lock entry records its derivation). Its
    lock is the crate's `bounds-contracts.lock` (C06.2's format, the
    declaration's normalised text digest, which covers the quant check's
    predicate), written only by `quirin-audit pin --bounds` and verified with
    the resource locks by `pin --verify-contracts`; at audit time the C06.2
    decision (`pin_standing`) runs over the audited source, and `no-lock`,
    `not-in-lock`, `contract-changed` or `source-changed` holds the whole
    declaration exactly as a review marker does: Pending at most, its
    violations standing reasons, the hold part of the environment digest.
    **Do not** load a bounds module through a resource selection, select bounds
    modules without the variable, put a declaration in force without an
    in-force pin, or let a pin hold accuse. Premises stated, not proved: the
    lexical site scanner's identity (C06.2's boundary) and C06's `Declared`
    reading of a resolved declaration; a header-defined function has no row on
    the audit routes (static inline definitions are outside
    `MainSourceDefinitions`), so its declaration is applied at its callers and
    its body is checked only through the API. Measured (corpus `66910c10`,
    system, tail-pause slice; base main `e2031f65a`, binaries `9c91a239…`
    before and `b53ecd02…` after): custody **byte-identical** on every route
    (corpus 756 / 9 / 4,069 `ce57d194…`, system `eab76f04…`, slice
    `bfe1ff94…`, the three typed-path parity streams equal multisets); the
    bounds-selected corpus loads chunkio's two declarations, both pinned in
    force at `66910c10` (and at `046accf6`), with **no verdict move** (162 Safe
    / 4,759 Unknown, combined 111 / 4,801 / 9 Unsafe): every row's environment
    digest changes, and the 9 obligations naming the two accessors in 9
    chunkio functions read the quant check instead of `states no bounds
    contract`, still `bounds-missing-extent` because every caller passes
    `cf->map` (CG16-BOUNDS-POINTER-EXTENTS). `QCore.BoundsPinHold`
    (`in_force_iff`, `changed_clause_never_in_force`,
    `changed_body_never_in_force`, `another_identity_keeps`, `repin_in_force`,
    `standing_resolved_iff`, `a_mismatch_is_held`,
    `a_declaration_is_never_a_source_fact`, `judge_never_accuses`,
    `pin_mismatch_never_safe`, `repinned_pending_is_safe` and 7 more), bound by
    `decisionVectors` (`crates/qcore-audit/tests/bounds_pin_hold.rs`) and
    `standingVectors` (`crates/qcore-contract/tests/bounds_contract.rs`), and
    end to end by `tests/bounds_pin_hold.rs` (unpinned, pinned, changed body,
    changed clause, missing entry) and
    `crates/qcore-c-gateway/tests/chunkio_meta_len_bounds.rs`;
    [report](../reports/chunkio-meta-len-bounds-contracts/README.md).

<a id="constraint-12s"></a>

12s. **A whole-call return of a callee with a return-code disposition
    forwards its classes, each validated on the body restricted to that
    class; a function whose disposition is in force has its row decided per
    class; and a disposition is pinned, ABI-checked and kept through a
    unit's withdrawals like every declaration.** Card
    CALLEE-RETENTION-CLASS1, stage (a), on the user's decision of 2026-10-05
    ("split by shape", contract specification C04.13 *Forwarding*, C04.14):
    `legacy_relational_forwarding.rs` (`infer_forwarded`, `relational_row`,
    `restrict`), thin hooks in `legacy_summary.rs`
    (`derive_custody_summaries`: the forwarding route after rule 11r's
    exit-specific one, and as a body summary of a declared disposition),
    `legacy_relational_summary.rs` (`BodySummary::Forwarded`, compared by
    relation overlap) and `analyze_defined_function`; shared items added:
    `ResourceContracts::carry_side_declarations` (used by
    `legacy_declared_abi.rs`'s `withdraw_environment` and
    `legacy_checked_spec.rs`'s reduction), `PinMismatch::{relational,
    retaining}` and the side tables in `withhold_for_pin_mismatch`,
    `with_human_holds_in_force`, `in_force_before_pins` and
    `check_source_assumptions`, `AbiAgreement::read` over the side tables,
    `qcore-audit`'s `apply_pins` over them. It amends rule 11r. (1)
    **Forwarding.** After every ordinary route and the exit-specific route
    published nothing, a body whose every `return` is a plain integer
    literal or `return f(ARGS);` (the whole expression one call, optionally
    parenthesised) of callees with dispositions in force sharing one set of
    class relations, whose result type is spelled exactly as each forwarded
    callee's (C converts nothing between the two values), whose literals each
    lie in a class, and which calls each forwarded callee only in those
    returns, takes the callee's classes: for each class `r` the literals
    outside `r` become `for (;;);` and the forwarded callee is read under its
    class `r` installed under its own name — the contract rule 11r's decided
    test `f(ARGS) OP K` selects on its true edge — so the return keeps its
    spelling and position for every typed reader; the ordinary inference
    and validation run unchanged, and `r` takes the contract validated
    there. Classes that all agree, a non-`unit` result or classes whose entry
    requirements differ publish nothing. (2) **The row.** A function whose
    own disposition is in force (declared and not withdrawn, or inferred) is
    decided per class: every restricted body is analysed under its class's
    contract and must validate (`observations_match_contract`); the
    published row is the first class's restricted analysis for a forwarding
    body, and the whole body under the common entry for a literal-only one
    (no diagnostic); any failure keeps the contract-free row. (3) **The
    integration gaps closed.** Before this card a disposition was never in
    the function map, and so (i) a unit that withdrew any declaration (an
    ABI or checked-specification reduction) rebuilt its environment from the
    function map and dropped every disposition, (ii) no disposition was
    compared with the unit's C declaration, and (iii) the pin evaluation
    skipped dispositions, so a pinned one was in force at any revision; no
    corpus callee carried one, so none of this had been exercised. Now each
    reduction carries the side tables, the ABI agreement reads them, and a
    pin mismatch withholds a disposition (or a retaining declaration, rule
    12t) from its own table and restores it there. **Do not** forward
    through a converting return, a callee also called outside its forwarded
    returns, or callees with different class sets; read a forwarded callee
    under a class anywhere but in its restricted body; publish a row from
    one class while another failed; or let a reduction rebuild an
    environment from its function map alone. Premises stated, not proved:
    faithful extraction (trust boundary 28), the declared dispositions
    (constraint 4), and that replacing a `return` by divergence leaves every
    state on every other path as it was (rule 11r's own). Measured
    (corpus `66910c10`, system and tail-pause slice; base main `6fc62c015`,
    binary `331fb897…` with the stage's six pinned dispositions): corpus
    803 / 9 / 4,022 → **812 / 9 / 4,013** (`2734991a…` → `cc47d362…`),
    **+9 Unknown → Safe** (`cJSON_AddItemToArray`, `cJSON_ReplaceItemInObject`
    and `_CaseSensitive`, `ctr_attributes_set_array`/`_kvlist` by their own
    per-class rows, the four `ctr_span*` setters), six functions changing
    leaves only; system 85 / 4 / 359 → 87 / 4 / 357
    (`flb_output_task_queue_enqueue` in both its units: an exit-specific
    summary rule 11r already inferred, its row now decided per class);
    slice no move; no Safe → non-Safe move, no leak or violation added.
    `QCore.RelationalForwarding` (`every_path_is_kept_by_one_class`,
    `no_path_is_kept_by_two_classes`, `a_forwarded_call_applies_the_callee_class`,
    `the_published_class_holds_at_every_exit`,
    `a_caller_edge_selects_the_exit_disposition`, `the_row_covers_every_path`,
    `a_conversion_can_move_a_value_between_classes` and three more), bound by
    the module's unit tests and `tests/callee_retention_class1.rs`;
    [report](../reports/callee-retention-class1/README.md).

<a id="constraint-12t"></a>

12t. **A pointer a callee retains stays the caller's, is never borrowed and
    forgotten, and is `Safe` only where the caller establishes that its
    object outlives the holder; a release while held is refused, never
    accused.** Card CALLEE-RETENTION-CLASS1, stage (b), on the user's
    decision of 2026-10-05 ("split by shape", contract specification C04.14,
    grammar G05): the vocabulary `qcore_schemas::retained_by`
    (`RetainingContract`, `Retention`, `RetentionHolder`;
    `ResourceContracts::{retaining, retaining_contract, insert_retaining}`,
    additive, skipped when empty), the loader clause
    `crates/qcore-contract/src/contract_retained_by.rs`, the pure decisions
    `crates/qcore-c-gateway/src/retained_by.rs` (`decide`, `replay`,
    `DECISION_VECTORS`, `TRACE_VECTORS`), the route-G adapter
    `legacy_retained_by.rs`; shared items added: `ContractCall::retention`,
    `AbstractState::retained` (joined by union), hooks in
    `build_function_cfg_with` (the minted environment), the classifier's
    uncontracted fallback (before rule 11r's), the `ContractCall` transfer
    (after the container effect), `apply_resource_action` and the exit
    report; a tenth reserved prefix (`qcore_retained__`, `legacy_reserved_spelling.rs`);
    `in_place_custody::one_automatic_object` made crate-visible. (1)
    **Withheld.** A declaration with `resource_retained_by(P, H)` or
    `resource_holder_reset(H)` is never in the function map; every consumer
    but route G sees an uncontracted callee, and no inference replaces it.
    (2) **Decided at the call.** `P` is a string literal (static storage), an
    unrebound formal this body holds no custody of (a caller's object), one
    known live identity this body tracks, or anything else; `H` is a frame
    holder (`&L`, `L` one automatic, non-pointer, non-array object of this
    function named only as whole `&L` actuals of retaining or resetting
    callees), one tracked identity, the `global` holder, or anything else.
    A static `P` outlives every holder; a caller's object outlives a frame
    holder; a tracked `P` in a frame or tracked holder, and a caller's object
    in a tracked holder, are recorded; everything else — a formal or
    member holder, `global` for a non-static `P`, an unreadable actual — is
    refused (`retention-undecided`, Unknown). (3) **Records.** A reset of a
    frame or tracked holder, or a release of a tracked holder, drops what it
    held; a release or transfer of a recorded `P` is refused
    (`retention-released-while-held`), never a violation; at an exit a frame
    holder dies, and a tracked holder still holding refuses. **Do not** read
    a retained pointer as borrowed, decide a holder the caller supplied,
    accept a static object other than a literal without a storage reading,
    accuse a release while held, or forward a retention into an ordinary
    summary (the S3 exception below carries its obligation explicitly).
    Premises stated, not proved: faithful extraction (trust boundary 28),
    the declared retentions (constraint 4; `RETAINED-BY-BODY-CHECK`), and
    P-RETAINED-LITERAL (the lexer's `__qcore_literal__` at a pointer position
    is a string literal: a character literal there is an integer-to-pointer
    conversion Clang rejects by default). Measured
    (binary `19a16144…`, the five pinned retaining declarations added to
    stage (a)'s tree): **no status moves on any route** (corpus
    812 / 9 / 4,013, `cc47d362…` → `a43d02cb…`; system 87 / 4 / 357; slice
    unchanged), as predicted for callers that each pass a formal as both the
    pointer and its holder: 20 corpus functions trade
    ``call to uncontracted function`` for `retention-undecided` (16), the
    nested `LZ4_compressBound` actual the uncontracted call hid (3) or the
    access gate (1); system 3 functions in two units; no leak, violation or
    `Unsafe` added.
    `QCore.RetainedBy` (`a_safe_path_never_releases_a_held_identity`,
    `a_safe_path_leaves_no_tracked_holder_holding`, `undecided_is_refused`,
    `reset_before_release_is_safe`, `release_before_reset_is_refused`,
    `a_holder_outliving_its_pointer_is_refused`, `global_holder_controls`,
    `an_unknown_holder_is_refused`, `decision_vectors_hold`,
    `trace_vectors_hold` and ten more), bound to `retained_by.rs` by the
    shared vectors and to route G by `tests/callee_retention_class1.rs`;
    [report](../reports/callee-retention-class1/README.md).
    **Amendment (card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION, contract
    specification C04.14.3; USER DECISIONS 2026-10-16, "Tracked holder path"
    and "R3: quant state"): holder paths and the removal of one
    registration.** Loader `contract_retained_by.rs` (`path_clause`,
    `removal_clause`, `paths_and_removals`), schema
    `qcore_schemas::retained_by::{PathRetention, Removal}`
    (`RetainingContract::{path_retentions, removals}`, omitted when empty);
    decisions `retained_by.rs` (`Holder::Path`, `Retained::TrackedInterior`,
    `HeldObject::SiteInterior`, `HeldBy::Path`, `removal_key`,
    `Event::Remove`, `PATH_DECISION_VECTORS`, `PATH_TRACE_VECTORS`); route G
    `legacy_retained_by.rs` (`path_stable`, `Parser::{path_holder,
    retained_spelling}`, `Operands::removals`, printed only when present).
    Shared items: `HeldObject::site` (the two held-identity checks of
    `legacy_slot_move_borrow.rs` and `legacy_reference_count.rs` read an
    interior too) and the access gate's contract lookup in
    `legacy_call_argument_access.rs`, which now reads a withheld
    declaration's borrowed positions as the function map's (discharging
    `RETAINING-CALL-MEMBER-ACTUAL`; a call the hook refuses keeps the gate's
    refusal). (1) **A path holder** is the object at a member path below an
    unrebound formal the body names only through member accesses off the
    path (premise `P-HOLDER-PATH-STABLE`); every other member spelling is a
    holder the body cannot name. A caller's object, a tracked identity or an
    interior `&X->f` retained there is recorded, and the record outlives the
    frame: it refuses at an exit unless a removal of exactly it ended it. (2)
    **A removal** (`quant live #H: #holds(P) |-> #absent;`) drops only the
    record its actuals key, and nothing otherwise: an unmatched removal never
    decides, and a later release of the held object refuses. **Corrected
    (card RETENTION-UNMATCHED-REMOVAL-POISONS, USER DECISION 2026-10-17
    "Yes, small fix card now"):** Monkey's `_mk_event_del` clears the event's
    registered status even through a loop that does not hold it, so a later
    removal through the right loop does nothing; the composition above
    accepted "unmatched removal, matched removal, release". A removal now
    drops the exactly matching record only when it is not **poisoned**, and
    poisons every other record whose held object it may name
    (`retained_by::{remove, may_name}`: an undecided object or holder names
    more, never less; only two distinct tracked identities are distinct
    objects; shared item `AbstractState::poisoned`, joined by union). A
    poisoned record ends only when its holder ends (`end_holder`: the loop's
    destroy, a released tracked holder, a declared reset, a registry's
    drop); at an exit it refuses as `retention-removal-poisoned`, and a
    release of its object while it is held refuses as before. **A destroy of
    the loop ends every record it holds**, poisoned or not:
    `mk_event_loop_create` (`resource_return_owned(nullable)`) and
    `mk_event_loop_destroy` (`resource_released(loop, nonnull)`) are declared
    in `monkey/event_loop.qv`, so a tracked loop's destroy is a released
    holder. A destroy at a path (`mk_event_loop_destroy(config->evl)`) ends
    the path's records in the model (`Event::Destroy`), but on route G it
    releases a member actual, a custody-taking position the access gate keeps
    refused (W44-INDIRECT-CALL-ARGUMENT-ACCESS premise 1), so such a call
    still refuses before any record is read. Two distinct paths are never one
    loop: a derived equality (D3, "derived only") is the follow-up
    `RETENTION-DERIVED-LOOP-PATH-EQUALITIES`. (3) A release
    or transfer of an identity while an interior of it is held refuses. A
    retention without the bounded S3 evidence below is never forwarded into
    a summary. The original measurement therefore refused every Fluent Bit
    registration at its registering function's exit (each removal is in
    another function, the census's §1.4;
    `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`). **Do not** resolve a path
    below a local or a call's result, match a removal to a record by its
    holder alone, end a poisoned record by any removal, read two distinct
    paths as one loop without a derivation, or forward a retention into an
    ordinary summary. Declarations: Monkey's seven event-loop
    operations in `qcore-provider-libevent` (`monkey/event_loop.qv`, pinned
    at 66910c10, 046accf6 and the 90978ac6 export; since card
    MK-EVENT-DESCRIPTOR-QUANT their descriptor effects are descriptor-cell
    states, rule 38, and `mk_event_channel_destroy`'s removal holds under its
    member-equality guard), and the loop's create and destroy. `QCore.RetentionPath`
    (`a_path_record_ends_only_by_its_own_removal_or_its_destroy`,
    `an_unresolved_path_never_decides`,
    `an_unmatched_removal_poisons_what_it_may_name`,
    `a_poisoned_record_ends_only_by_its_loop_destroy`,
    `the_old_sequence_no_longer_decides`,
    `a_safe_path_never_releases_a_held_identity`,
    `a_safe_path_leaves_no_outliving_holder_holding`, the vectors; the
    earlier `an_unmatched_removal_keeps_every_record` and
    `a_path_record_ends_only_by_its_own_removal` are corrected, not kept);
    `tests/event_loop_retention.rs`;
    [report](../reports/fb-event-loop-retention/README.md),
    [poisoning report](../reports/retention-unmatched-removal-poisons/README.md).
    **S3 amendment (FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME, user direction
    2026-10-18; C04.21):** an unconditional declared record footprint may
    forward an exact caller-owned record at a typed interior/holder path.
    The first source route supports a self-record below an unrebound formal
    and bounded same-unit calls. The record and its poison remain present;
    a separate proof names its exact origin, source-qualified paths and
    protocol identity. At a join, every arm where that record is present
    must prove its forwarding; an ended/re-registered record needs new
    evidence. Body admission exports the outstanding obligation and its
    named protocol assumption, not a claim that every caller meets it.
    Callers must discharge or forward it, and unresolved roots remain
    Unknown. Source/configuration/function-qualified summary and certificate
    replay retain/rederive these effects; ordinary W4 records and declared
    borrow-only custody cannot erase them. Cross-unit lifetime import uses a separate effect-bearing channel: the
    home raw/marked/preprocessed source, request, environment and linked-program
    identities are retained and independently rederived before the exact
    outgoing effects are consumed. An ordinary borrowed import cannot replace
    that record; missing, altered or duplicate homes/effects refuse.

    Every available registration/store site participates in protocol standing:
    a contradiction withdraws the clause; an uncheckable closure is an
    attributed DeclaredAssumption (rule 13i), never a validated closure.
    An unused footprint supplies no premise to ordinary custody/callback use.
    Holder equality is still derived-only (D3). Closed allocation candidates
    supply neither runtime equality nor universal destroy evidence. Call
    composition retains may-release ordering: a concrete must-destroy proved before release can discharge an incoming
    record. A bounded complete ordered trace may additionally replay exact
    removals against the incoming poison state; it cannot clear poison or
    repair an earlier release. Same-unit ordered transport requires internal
    linkage, exact substitution and stable snapshots. Differing exit traces,
    unresolved/lifted actuals and traces beyond 4096 primitive steps withhold
    ordered discharge. A later destroy or an unordered removal supplies none.
    Removal-only C helpers also require a separate effect-bearing record with
    the complete ordered formal-index trace. Its source CFG is one acyclic
    path of removal calls and exits, with stable typed parameter slots and no
    extra effect or unresolved alias. Ordinary W4 publication cannot discard
    removals. Mixed effect vectors, writes, additional calls and conditional
    paths refuse this source-summary class. Imported traces are compared with
    the independently replayed home; local inference is rederived, never
    accepted as pre-seeded authority. Unsupported control flow, member stores
    and unread call frames withhold must-ending/prefix evidence; a custody
    contract is no heap frame proof. Exact source/type/object/snapshot
    extraction remains trust boundary 28, not a theorem about Clang.

    The accompanying local-condition route checks conditional prerequisites
    at caller uses; unknown conditions preserve both feasible cases. A body
    may use its own active conditional requires with exact substituted
    object/member/polarity and the named snapshot premise. An annotation alone
    creates no external caller fact. Guarded declarations remain outside
    unaware effect tables and ordinary summary publication, including after
    withdrawal/pin withholding; unsupported structured mixtures and conditional
    type-wide record correlation refuse. New member/protocol facts remain
    runtime dependencies under rule 31, without new envelope coverage.
    `QCore.LocalCondition`, `QCore.LoopLifetimeSummary` and their registered
    audits cover the abstract decisions and ordering boundaries, with shared
    Rust vectors; [the S3 report](../reports/fb-event-loop-lifetime-s3/README.md)
    owns the source controls, evidence boundaries and measurements.

    **Holder-frame and identity amendment (EVENT-HOLDER-IDENTITY):** a Safe
    source-derived lifetime summary may additionally carry checked preservation
    of one direct, nonvolatile/non-atomic struct holder member below an exact
    formal. Its complete straight-line body has no member stores or unread
    effects; the original member-only path census, or already derived internal
    callee frames, covers every root use. A whole-record call supplies that
    frame only through its typed direct actual, exact formal/record/member and
    internal linkage or independently replayed external home. Borrowed custody
    alone supplies none. The entire caller body must also preserve every
    outgoing holder; a later opaque whole-record call or holder write withholds
    forwarding. The lexical and typed never-bound closures may read
    that record's custody component; actual calls still apply full lifetime
    effects, and no ordinary function-map contract is installed. An exact
    uniquely named, initialized, unaddressed, single-write, type-preserving
    local pointer-value copy may name its original pointer formal's collector
    or loop path on a fully read
    straight-line body. Every collector-pointer-copy use participates in the path
    census; declarations, casts, writes, shadowing, address escapes, unknown
    control, address initializers, value formals and another formal never
    establish identity. Existing alias/liveness
    reads remain required: lifetime identity does not discharge them. Numeric
    ids, record types and allocation candidates supply no runtime equality.
    Poisoned outstanding effects remain explicitly forwarded rather than
    treated as ended. Imported frames are compared with freshly rederived home
    records; schema 24 prevents older readers from silently ignoring them.
    P-HOLDER-PATH-STABLE's existing member-use premise, faithful extraction,
    declared event semantics, link resolution and Rust/Lean correspondence
    remain assumptions. This is no production collector-search, conditional
    host/loop, thread-join or backend-body proof. `QCore.HolderFrameIdentity`;
    [report](../reports/event-holder-identity/README.md).

    **Collector source-audit boundary (EVENT-COLLECTOR-SELECTION):** the
    separately replayed `collector_identity` imports read an exact internally
    linked first-match owner-list/id search, including its GNU container-of
    expansion and null result, and a constructor-local instance store plus
    conditional worker/config loop stores. A checked internal subobject-only
    initialization frame may intervene. These are source-shape/snapshot reports,
    not lifetime summaries or ordinary borrowed contracts; no Safe route consumes
    them. The selected list's owner does not establish the selected record's
    instance member without a membership invariant. Duplicate ids retain first
    match semantics; matching ids do not establish record identity across list
    changes. The false arm keeps the exact config formal and supplies no
    config/instance equality. A constructor snapshot gives no enduring loop or
    flag stability. Source/configuration/compiler binding and independent replay
    do not discharge list/key/record frames, thread teardown, descriptor custody
    or backend-body proof. Faithful extraction and Rust/Lean correspondence
    remain premises. `QCore.CollectorIdentity`;
    [report](../reports/event-collector-selection/README.md).

<a id="constraint-12u"></a>

12u. **An enumeration in a declaration's C prototype is the integer type
    Clang states for it in the analysed unit, exactly, and nothing when Clang
    does not state it.** Card W43-ABI-ENUM-AND-CONST-PP, on the user's
    decision of 2026-10-05 (contract specification C04.1): the declared-ABI
    reader (`legacy_declared_abi.rs`; shared items added:
    `TypedefTarget::UntaggedEnum`, the two-phase read in
    `AbiAgreement::read`, the census entering the source it reads) projects an
    enumeration parameter or result — `enum TAG`, an untagged
    `typedef enum { … } NAME`, any typedef chain to either — through
    `legacy_enum_projection.rs`: a first reading records every enumeration a
    contracted declaration names, one appended Clang probe per unit
    (`typed_access_path::appended_declarations`, a shared item; `sizeof(T) *
    __CHAR_BIT__`, the sign of `(T)-1` and `__builtin_types_compatible_p`
    against the ten standard integer types) answers them, and the reading is
    repeated with the answers. The projection is the one spelling of the probed
    width and signedness (`i8`…`u64`, never `isize`/`usize`), admitted only
    when exactly one standard type is compatible and its signedness is the
    probed one. A failed launch, an incomplete enumeration, a block-scope
    typedef, a source spelling the probe prefix and every reading outside
    `AbiAgreement::read` refuse (`abi-unreadable-declaration`), so `int` is
    never assumed. **Do not** guess an enumeration's type from its
    enumerators, read it outside the analysed unit, or admit the pointer-width
    readings. Premises stated, not proved: faithful extraction (trust boundary
    28), Clang's `__builtin_types_compatible_p` answering exactly the
    enumeration's integer type, and the unit read under the gateway's frontend
    request (`ABI-ENUM-UNIT-FRONTEND-FLAGS`). Measured with rules 12v and 12w
    and fifteen pinned declarations (binary `552d86ab…`, corpus manifest
    selecting `librdkafka/client.qv`): corpus 812 / 9 / 4,013 → **846 / 9 /
    3,979** (`a43d02cb…` → `686fca16…`, **+34 Unknown → Safe**, no Safe →
    non-Safe move, no leak or violation added), system and slice no row
    moves. `QCore.AbiEnumConstPointer`; tests
    `tests/abi_enum_const_pp.rs`, `tests/loader_abi.rs`;
    [report](../reports/w43-abi-enum-and-const-pp/README.md).

<a id="constraint-12v"></a>

12v. **A pointer to a pointer to const is `ptr<mut, ptr<const, u8>>`, and a
    qualifier on the pointer in between has no spelling.** Card
    W43-ABI-ENUM-AND-CONST-PP (contract specification C04.1; loader
    `POINTER_ABI_TYPES`, a shared item): `ptr<A, ptr<B, u8>>` reads its
    qualifiers outside in, `A` on the slot (the inner pointer object), `B` on
    the innermost pointee. The reader's `pointer_projection(2, true)` answers
    the new spelling for `const char **`, `const void **`, an adjusted
    `const char *p[]` and a typedef chain whose innermost specifiers carry the
    `const`; a qualifier between the two `*`, a qualifier on a typedef name
    that stands for a pointer (`const str *`) and a chain link qualifying a
    pointer refuse, as every depth-two `const` reading did before. `char **`
    and `const char **` never admit each other; `resource_out` still requires
    `ptr<mut, ptr<mut, u8>>`; derived results and retentions stay depth one.
    **Do not** drop a slot qualifier or fold the two spellings together.
    `QCore.AbiEnumConstPointer`; [report](../reports/w43-abi-enum-and-const-pp/README.md).

<a id="constraint-12w"></a>

12w. **A guarded retention holds where the caller's actual is decided to
    satisfy its guard, everywhere else the cautious union stands, and in its
    own unit only a body that cannot keep the pointer when the guard fails
    keeps the guard.** Card W43-ABI-ENUM-AND-CONST-PP, on the user's decision
    of 2026-10-05 (contract specification C04.14.1, grammar G05):
    `resource_retained_by(P, H) when X OP K` (`RetentionGuard`,
    `GuardRelation`, `GuardDecision` in `qcore_schemas::retained_by`, additive,
    skipped in the serialised form when absent; the loader's
    `split_guard`/`guard_partitions`), route G's
    `legacy_retention_guard.rs` with two hooks: (1) rule 12t's classifier
    decides the actual at `X` — a decimal constant (matched parentheses, an
    integer suffix, a leading `-`) or a file-scope enumeration constant whose
    value an appended Clang probe states and which the calling function's own
    Clang definition does not shadow — in `X`'s range: a decided failure drops
    the clause at that call; a decided success, a variable, a value C converts
    and anything unread keep it; (2) beside the ABI gate, in a unit defining
    the callee, `settle` removes every guard the body does not validate: each
    mention of `P` outside a guard region (an exact `if (X OP K) { … }`, or a
    `case` arm of `switch (X)` no other label falls into) is a null or
    equality test or a dereference, `X` is never written, no `goto`.
    **Do not** read the favourable arm for an undecided actual, guard
    anything but a retention, or take an enumeration constant's value from
    anywhere but Clang. No corpus declaration carries a guard: the decision's
    example, `rd_kafka_confval_set_type`, retains on the holder's own type
    (`GUARDED-CLAUSE-WIDENING`). `QCore.RetentionGuard`
    (`an_undecided_guard_is_safe_only_where_the_cautious_reading_is`,
    `the_cautious_reading_bounds_every_reading`,
    `a_validated_body_keeps_nothing_when_the_guard_fails`,
    `an_admitted_guard_partitions_its_range`); tests
    `tests/retention_guard.rs`, `crates/qcore-contract/tests/retention_guard_clause.rs`;
    [report](../reports/w43-abi-enum-and-const-pp/README.md).

<a id="constraint-12x"></a>

12x. **A slot move keeps the slot's one object and invalidates every other
    name of it; anything the call cannot decide is refused.** Card
    SLOT-MOVE-AND-INTERIOR-BORROW, class 2, on the user's decision of
    2026-10-05 (contract specification C04.15, grammar G05):
    `resource_slot_moved(s, n)` (`qcore_schemas::slot_move::SlotMove`, a
    field of `RetainingContract`, skipped when empty; the loader's
    `contract_slot_move.rs`) is withheld from the function map with the
    retaining declarations; the pure decisions are
    `crates/qcore-c-gateway/src/slot_move_borrow.rs` (`decide_slot`,
    `slot_step`, `SLOT_VECTORS`, `SLOT_TRACE_VECTORS`), the route-G adapter
    `legacy_slot_move_borrow.rs`; shared items added:
    `retained_by_route::Operands::slots`, `AbstractState::{invalidated,
    borrows}` (joined by union), `Parser::slot_address` (the classifier's
    `&NAME` exemption at a slot position), the `transfer` wrapper's
    `before`/`after` hooks, `RetainingContract::retains` (rule 12t's holder
    test now asks it), `ResourceContracts::withdraw_declaration`. At a call
    the slot actual is `&L`, a formal slot the body names only as whole
    slot-moving actuals (passed through: nothing moves here), or anything
    else (refused, `slot-move-undecided`). For `&L`: exactly one live tracked
    identity held by no structure but names (no field link, container,
    holder, collection, pending replacement or nullable marker) is moved:
    `L` keeps it and its one obligation, and every other name bound to it
    loses its binding and is invalidated, so a later read is refused
    (`slot-move-alias`), never accused; a definitely-NULL `L` under
    `nullable` moves nothing; anything else refuses. The failure arm needs
    no rule: the slot still owes its object. **Do not** read a moved slot as
    borrowed, accuse a use of an old alias (the object may not have moved),
    move a field reached through a borrowed parameter (`SLOT-MOVE-FIELD-SLOT`),
    or admit a slot move inside a return-code disposition before a member
    needs it (`SLOT-MOVE-RETURN-CODE-SPLIT`). Premises stated, not proved:
    faithful extraction (trust boundary 28) and the declared slot moves
    (constraint 4; rule 13a checks what it can). Measured with rules 12y,
    12z, 13a and fourteen declarations (binary `8f811155…`): corpus 846 / 9 /
    3,979 → **872 / 9 / 3,953** (`686fca16…` → `52d14db1…`, **+26 Unknown →
    Safe**: 7 over the two slot moves, 15 over the borrowed results, 2 over
    `rd_slice_reader0`, 2 through new summaries), no Safe → non-Safe move, no
    leak or violation added; system no row moves; slice leaves only.
    `QCore.SlotMove`
    (`the_slot_keeps_its_identity_and_its_obligation`,
    `every_old_alias_is_invalid_after_a_move`,
    `a_use_of_an_old_alias_is_refused`, `the_failure_arm_keeps_the_original`,
    `a_safe_path_never_uses_an_invalid_name`, `slot_vectors_hold`,
    `slot_trace_vectors_hold`); tests `tests/slot_move_interior_borrow.rs`,
    `crates/qcore-contract/tests/slot_move_clause.rs`; [report](../reports/slot-move-and-interior-borrow/README.md).

<a id="constraint-12y"></a>

12y. **A borrowed result owns nothing, ends at its parent's release or at
    any mutation it does not make itself, and is refused, never accused,
    after it ends.** Card SLOT-MOVE-AND-INTERIOR-BORROW, class 8 (contract
    specification C04.3.4): `resource_return_borrowed_from(p, n)` loads to
    the derived result at a new extent, `DerivedExtent::Borrowed` (a shared
    item; every consumer of the extent was audited: an offset, a step and a
    derivation keep it, `interior_extent` changes only an exact alias, the
    inference publishes `Borrowed` when any exit is borrowed and the
    summary check lets only a `Borrowed` declaration cover a borrowed exit,
    `derive_pointer` takes a borrowed result's nullness from the
    declaration). A borrowed call records the destination's borrow (its take
    and its parent's root name; an alias copies it). Before every
    instruction, a read of an ended name refuses (`interior-borrow-ended`);
    a call or store ends every borrow unless every name it may write is in
    the borrow's family (its aliases and the borrows taken from them) or a
    declared scalar, and an actual it does not read ends every borrow
    (premise P-BORROW-FRAME: a call reaches only what its actuals reach); a
    rebound parent ends its borrows; an ended borrow's binding is dropped.
    A release of a borrow is refused (never `invalid_release`); a store
    through a non-null borrow is admitted as a write to its parent (it keeps
    the borrow and ends its siblings); a use after the parent's release that
    the settled-state reports would have accused is refused instead. The
    base may be spelled through a pointer cast of a place (`cast_place`,
    for the `Borrowed` extent only; `DERIVED-BASE-CAST-READING`). **Do not**
    publish a plain interior summary over a borrowed exit, let a borrowed
    result outlive a mutation it did not make, or read an arena result as
    borrowed (the interim `cfl_arena_calloc` reading of C04.3.1 stands).
    `QCore.InteriorBorrow` (`a_borrow_release_is_refused`,
    `a_parent_release_ends_its_borrows`,
    `an_invalidating_mutation_ends_a_borrow`, `a_use_after_the_end_is_refused`,
    `a_write_through_a_borrow_keeps_it`, `a_write_through_a_borrow_ends_its_sibling`,
    `a_lazily_created_child_is_owned_by_the_parent`,
    `a_safe_path_never_uses_an_ended_borrow`, `borrow_trace_vectors_hold`);
    [report](../reports/slot-move-and-interior-borrow/README.md).

<a id="constraint-12z"></a>

12z. **A derived out slot receives a pointer that owns nothing and never
    overwrites a tracked one.** Card SLOT-MOVE-AND-INTERIOR-BORROW
    (contract specification C04.16, absorbing `INTERIOR-OUT-SLOT-MODE`):
    `resource_out_derived(s, p, n)` (`DerivedOut`, withheld with the
    retaining declarations). At a call: `&L` with `L` holding no tracked
    identity binds `L` to a pointer derived from `p`'s binding, possibly
    NULL, read at the `Borrowed` extent without a borrow record (the slot
    may keep its old untracked value, so its release and a use after `p`'s
    release are refused, never accused, and no mutation ends it); `&L`
    holding a tracked identity is refused (`out-derived-undecided`); a NULL
    slot under `nullable` and a caller's slot passed through change nothing
    here; anything else refuses. **Do not** read the slot as `resource_out`
    (fresh custody) or as borrowed and forgotten. `QCore.DerivedOutSlot`
    (`a_tracked_slot_is_never_overwritten`, `the_slot_value_is_never_released`,
    `the_base_release_ends_the_slot_value`,
    `a_safe_path_never_uses_a_value_its_base_outlived`, `out_vectors_hold`);
    [report](../reports/slot-move-and-interior-borrow/README.md).

<a id="constraint-13a"></a>

13a. **In a unit that defines a slot-moving, derived-out or borrowed-result
    declaration, the declaration stands only where its body validates it.**
    Card SLOT-MOVE-AND-INTERIOR-BORROW (`slot_move_route::settle`, beside
    rule 12w's at the three unit entry points): a slot move when every
    mention of the slot formal is a whole actual at a slot-moving position;
    a derived out slot when every mention is `*s = EXPR;` with `EXPR`
    reached from the base, or a whole actual at a derived-out position; a
    borrowed result when the base is never assigned nor its whole address
    taken and every `return` is NULL or reached from the base by member
    reads (through casts, parentheses and offsets, `-` `>` read as one
    arrow), through locals every assignment of which is so reached (the
    greatest fixpoint, address-taken locals excluded), or a local assigned
    from a call whose result the body stores into a member of a reached
    name (a lazily created child). Otherwise the declaration is withdrawn in
    that unit (`ResourceContracts::withdraw_declaration`): its call sites
    there see an uncontracted callee and an inferred summary of the body
    governs. Structural: it establishes the provenance of what the body
    hands back or forwards, and the declaration stays a declared assumption
    in every other unit. Unit tests in `legacy_slot_move_borrow.rs`, the
    withdrawal control in `tests/slot_move_interior_borrow.rs`; [report](../reports/slot-move-and-interior-borrow/README.md).

**Amendment (CJSON-DETACH-SAME-ITEM, user decision 2026-10-18):** the bounded
`resource_return_removed(item, n)` result returns the same checked-contained
identity, never fresh custody. `legacy_detach` checks sole-owner membership,
settled identity and a unique live parent before any call effects; NULL item
moves nothing. In a defining unit it checks the complete physical unlink
frame and same-record `child`/`next`/`prev` layout, under the exact named
`detach_list_representation` premise (contract spec C04.3.5). The premise states
finite acyclic child/next representation, first.prev = tail including singleton,
predecessor links, last.next = NULL and no concurrent writer. It never supplies
membership, alias or return identity evidence, remains DeclaredAssumption and
is retained in dependent results/replay. No successful template check is full
body safety or validation of that premise. Wrong parent, borrowed-only
provenance and ambiguous/unknown relations refuse; whole-return and passed
consumers restricted to fresh results still refuse. Ordinary W4 never forgets
the effect into borrowed-only custody. `QCore.DetachSameItem`; the report
`docs/reports/cjson-detach-same-item/README.md` records the measured boundary.

**Amendment (DETACH-LOOKUP-MEMBERSHIP):** a complete typed read-only
indexed `child`/`next` frame can establish a nullable Borrowed result through
ordinary fresh-run derived-summary validation (rule4b). Both original
source-bound typed reads must exist; an opaque lexical refusal without retained
Clang context does not fall back. Original typed reads and their alias/base/
chain obligations remain intact. Only status-free name-only inference sites
may use a checked untracked view at the exact read point; real, unresolved,
nullable or conflicting parent bindings never gain that exception. The cursor
is narrowed only under its source-proven non-NULL while-body guard. A released
peer prevents the view. Assigned direct decimal0–255 calls retain
ordinary ContractCall visibility and carry independently rederived CFG metadata.
An identity refinement additionally needs explicit must-held physical links,
a caller actual of the callee's complete canonical defining record type, and
the same resolved record type at each stored-link owner. Prefix facts are keyed
by site, record tag and field; a differently typed overlapping store invalidates
prior facts for that owner. Unknown types, including opaque alias views, supply
no identity descriptor. These are source-evidence checks for the existing
physical-prefix premise. The audited eligibility predicate refuses unknown or mismatched
canonical tags; the Rust extractor supplies completeness and exact equality.
It also needs a live parent and an existing settled child uniquely contained by that parent;
absence is Unknown, never NULL, and no new resource site is created. Facts meet
by intersection and are carried only where a checked lookup call exists.
Unknown/wrapper writes and unproved calls destroy them. Lookup aliases retain
the ordinary borrow lifetime; only the already checked same-item Removed
transition releases that old parent lifetime for the restored exact identity.
The current source bridge proves stored head/empty-head cases; generic
successor closure through contained children remains refused. Whole-return or
nested Removed transport stays separate. Custody only; no runtime or C memory
safety claim and no validation of the detach representation premise.
`QCore.ListLookup`; [report](../reports/detach-lookup-membership/README.md).

<a id="constraint-13b"></a>

13b. **A reference count is a resource kind: each function balances the
    references it takes or hands them off, the last drop invalidates every
    name of the object, and keep and drop are never read as custody-neutral.**
    Card REFCOUNT-RESOURCE-KIND, class 3, on the user's decision of
    2026-10-05 (contract specification C04.17, grammar G05):
    `resource_reference_kept(p)` and `resource_reference_dropped(p)`
    (`qcore_schemas::reference_count`, `RetainingContract::{references,
    references_withdrawn}`, skipped when empty; the loader's
    `contract_reference_count.rs`) are withheld from the function map with the
    retaining declarations, so every consumer but route G sees an uncontracted
    callee; `resource_return_kind(reference)` beside `resource_return_owned`
    is a returned reference (`ResourceKind::Reference`, a shared item: the
    kind table, `observed_kind`, the use-after-close candidate and the Lean
    kind list exclude or list it). The pure decisions are
    `crates/qcore-c-gateway/src/reference_count.rs` (`decide_keep`,
    `decide_drop`, `owed`, `decide_exit`, `ref_step`, `REFERENCE_VECTORS`,
    `REFERENCE_TRACE_VECTORS`), the route-G adapter
    `legacy_reference_count.rs`; shared items added:
    `AbstractState::references` (joined by site, a disagreeing count
    undecided), `retained_by_route::Operands::references`, the reserved site a
    keep may mint (`Parser::reserve_reference_site`, read by the kind table),
    `slot_move_route::Invalid::{ReferenceDropped, ReferenceFreed}`
    (`defined_body` made `pub(super)`), hooks in `apply_contract_call` (a
    returned reference starts an acquired family), `remove_null_site`,
    `report_leaks` (families are the route's), the exit report, the loop
    generations' `rename` (a family's count follows its site) and the slot
    move's held-elsewhere test (a family is never moved). A keep of a name
    bound to no tracked identity (an unrebound formal, an untracked local,
    through a pointer cast) mints a family held once and binds the name and
    the result; a keep of a live family adds one; anything else, and a keep
    into a place that is not a plain name, refuses (`reference-undecided`). A
    drop subtracts one; the one that reaches zero is possibly final: the
    family is discharged and every name bound to it is invalidated, so a later
    read refuses (`reference-dropped`), never accused. A drop the function does
    not hold (an unbound formal, a member read, a family handed off or held by
    a field link, a container or a holder) refuses
    (`reference-drop-unheld`); a drop past an **acquired** family's last drop
    is an over-drop, a `DoubleRelease` candidate on a path with no refusal.
    At an exit a family owes its count, less one when it escaped (alone or
    with the owner it is linked into), was stored or was published: a leak
    candidate on a path with no refusal, `reference-unbalanced` on one with
    a refusal; an identity a refused statement may have handed on is
    withdrawn as for any resource. A heap release of a family is a kind
    mismatch. **Do not** declare an `rd_atomic32_*` primitive as a reference
    or as neutral (a reference keep and a statistics counter are the same
    call after preprocessing: `rd.h:298,351`), read keep or drop as borrowed,
    infer a reference result, or accuse a use after a last drop. Premises
    stated, not proved: faithful extraction (trust boundary 28) and the
    declared keeps and drops (constraint 4; rule 13c checks what it can).
    Measured (binary `bfd36e2a…`, sixteen pinned
    declarations in `librdkafka/client.qv`): corpus 872 / 9 / 3,953 →
    **884 / 9 / 3,941** (`52d14db1…` → `7a4595d6…`, **+12 Unknown → Safe**:
    the five destructor `_free` wrappers, `rd_kafka_topic_partition_destroy`
    and `_destroy_free`, the three `rd_kafka_queue_get_*` and one cascade over
    the queue handle constructor, and `rd_kafka_toppar_list_copy`, a keep
    returned), no Safe → non-Safe move, no leak or violation added; system and
    slice no row moves (parity multisets identical); 138 corpus leaves read
    `reference-drop-unheld`, 131 of them through a plain name holding a
    reference from an undeclared reference-returning call
    (`rd_kafka_toppar_get2`, `rd_kafka_q_new0`) or a caller's reference
    (`REFCOUNT-REFERENCE-RETURNING-DECLARATIONS`).
    `QCore.ReferenceCount` (`a_safe_path_is_balanced`,
    `a_missing_drop_is_never_safe`,
    `a_statistics_counter_is_never_read_as_a_reference`,
    `a_counter_changes_no_family`, `the_neutral_reading_is_not_admitted`,
    `an_alias_use_after_a_possibly_final_drop_is_refused`,
    `an_over_drop_of_an_acquired_family_is_a_violation`,
    `reference_vectors_hold`, `reference_trace_vectors_hold` and sixteen
    more); tests `tests/refcount_resource_kind.rs` (the brief's seven
    controls and the vectors bound to the Lean text),
    `crates/qcore-contract/tests/reference_clause.rs`;
    [report](../reports/refcount-resource-kind/README.md).

<a id="constraint-13c"></a>

13c. **In a unit that defines a keep or drop declaration's body, the
    declaration stands only where its body validates it, and a withdrawn one
    keeps refusing.** Card REFCOUNT-RESOURCE-KIND (`reference_route::settle`,
    beside rule 13a's at the three unit entry points; a header `static
    inline` body is defined in every unit that includes it): the referenced
    formal is never assigned and never has its address taken; it is mentioned
    only through member accesses, as a whole actual of a call, or in a null
    test; nothing stores or returns it except an alias keep's `return p;`
    (every other body returns nothing); the body has no `goto`; and it
    touches the object (a member access, or a whole actual at the same
    reference position of a declared callee: `rd_kafka_q_destroy_owner`
    forwards to `rd_kafka_q_destroy0`). Otherwise the declaration is marked
    withdrawn in that unit (`references_withdrawn`): every call there refuses
    (`reference-withdrawn`) and, unlike rule 13a's withdrawal, it stays in the
    retaining table so no inference reads the body in its place (the neutral
    reading is never admitted). Structural: it establishes that the body hands
    back exactly its argument and stores nothing of it; the count arithmetic
    stays a declared assumption pinned to the body. Unit tests in
    `legacy_reference_count.rs`, the withdrawal control in
    `tests/refcount_resource_kind.rs`; [report](../reports/refcount-resource-kind/README.md).

<a id="constraint-13d"></a>

13d. **A returned reference stands in a unit that defines its body only
    where every `return` hands back a reference the body obtained and did
    not give away; otherwise every call there refuses, and no inference
    reads the body in its place.** Card
    REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (contract specification
    C04.17, "Returned references checked against bodies"):
    `crates/qcore-c-gateway/src/legacy_reference_result.rs`
    (`reference_result_route::{validated, settle}`), called first from rule
    13c's `reference_route::settle` at the three unit entry points (shared
    item: `reference_route::settle` now chains it; `settle_clauses` is 13c's
    old body). The body has no `goto` and at least one `return`, and each
    `return` is a null constant (Clang's `( ( void * ) 0 )`, under pointer
    casts and parentheses; only for a `nullable` declaration), a call of a
    declared keep with an alias result or of a declared returned reference
    (pointer casts erased), or a local, never a formal, every assignment of
    which is a null constant, such a call, or a call of a declared heap
    allocation (a constructor's fresh object), whose address is never taken
    and which is never an actual at a declared release, consumption, store or
    drop. A declaration that fails moves, in that unit only, to the retaining
    table marked `references_withdrawn` (`legacy_retained_by.rs` now refuses
    a withdrawn declaration with no reference clause too), so every call
    refuses (`reference-withdrawn`). **Do not** admit a returned formal, a
    member read, an undeclared call or a bare allocation, or read the
    declared non-null result or the fresh count as checked: both stay
    declared assumptions pinned to the body. Nine pinned returned-reference
    declarations (`rd_kafka_toppar_get0`, `_desired_get`, `_new0`,
    `_desired_add`, `_get2`, `_get_avail`, `rd_kafka_q_new0`,
    `ut_create_mock_toppar`, `ut_create_mock_queue`) and one drop
    (`ut_destroy_mock_queue`, rule 13c) in `librdkafka/client.qv`; every
    defining body validates (no `reference-withdrawn` on any route). Unit
    tests in `legacy_reference_result.rs`; controls in
    `tests/refcount_reference_returning.rs`; `QCore.ReferenceResult`
    (`a_validated_body_never_returns_a_formal`,
    `a_validated_body_returns_only_obtained_references`,
    `a_validated_non_null_body_never_returns_null`,
    `a_withdrawn_declaration_is_never_safe`, `body_vectors_hold`);
    [report](../reports/refcount-reference-returning-declarations/README.md).

<a id="constraint-13e"></a>

13e. **A pointer retained by the call's result is `Safe` only where the
    result is released, or the function drops its last reference on it,
    before the function returns; a returned or stored holder is
    `retention-undecided`, never `Safe`.** Card
    REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, absorbing
    `RETAINED-BY-RESULT-HOLDER` (contract specification C04.14.2, grammar
    G05): `resource_retained_by(P, result)` (`RetentionHolder::Result`, a
    shared item; the loader admits it only beside
    `resource_return_owned(nonnull)` and rejects a parameter named `result`).
    Rule 12t's table decides it: the holder is the one live identity the
    call bound its plain destination to (`legacy_retained_by.rs`:
    `HolderSpelling::Result`, `holder_class` now takes the call's
    destination; `retained_by_route::apply` gains a `destination` argument),
    any other destination a holder the body cannot name. Shared items: the
    final drop of an **acquired** family discharges what its site holds
    (`legacy_reference_count.rs`, `holder_released`; premise
    P-RESULT-HOLDER-LAST-DROP: every reference the function could see on an
    object it obtained itself is dropped, and a reference a callee took
    through a position declared `resource_borrowed` is that declaration's
    assumption, constraint 4); a borrowed family's last drop discharges
    nothing. `__func__`, `__FUNCTION__` and `__PRETTY_FUNCTION__` are read as
    static storage at a retained position (`retained_by::PREDEFINED_IDENTIFIERS`;
    P-RETAINED-PREDEFINED, C11 6.4.2.2p1, as `variadic_nested`'s T6 reads
    them). **Do not** decide a result bound to a member or another place,
    discharge a holder at a borrowed family's drop, or forward a retention
    into a summary. `rd_kafka_q_new0` retains `rk` and `func` by its result;
    `rd_kafka_queue_new0` is re-declared retaining `rk` by its result (it
    leaves the function map). Measured (binary `a3398282…`, base
    `e27f98d5b`): corpus 884 / 9 / 3,941 → **880 / 9 / 3,945**
    (`7a4595d6…` → `79dc429d…`): no Unknown → Safe; **4 Safe → Unknown,
    each a named correction of REFCOUNT-RESOURCE-KIND**
    (`rd_kafka_queue_get_main`, `_get_consumer`, `rd_kafka_queue_get_sasl`,
    each returning a handle that retains its caller's `rk`: the leaf
    reported is the access gate's on the member actual `rk->rk_rep`, which
    the gate no longer reads now that the declaration is withheld
    (`RETAINING-CALL-MEMBER-ACTUAL`), and behind it the returned holder
    refuses at the exit; and `rd_kafka_share_queue_get_sasl`, whose summary
    over `_get_sasl` is gone); no leak or violation added; system and slice no row moves
    (parity multisets identical). `QCore.ReferenceResult`
    (`a_result_holder_released_first_is_safe`,
    `a_result_holder_dropped_first_is_safe`,
    `a_result_holder_handed_on_is_never_safe`,
    `a_borrowed_familys_last_drop_keeps_its_records`,
    `an_object_released_while_its_handle_holds_it_is_refused`,
    `a_predefined_identifier_outlives_every_holder`, and the transfer:
    `a_drop_after_a_transfer_is_an_over_drop`); controls in
    `tests/refcount_reference_returning.rs`;
    [report](../reports/refcount-reference-returning-declarations/README.md).

<a id="constraint-13f"></a>

13f. **Every published legacy-C row carries the contract declarations its
    derivation consulted, taken from the route's own consultations; the set
    may over-approximate and never omits one.** Card
    VERDICT-CONTRACT-DEPENDENCIES (contract specification C06.5, language
    specification L20): `crates/qcore-c-gateway/src/contract_dependencies.rs`.
    Shared items: `qcore_schemas::contract_consultation` (a per-thread log;
    `ResourceContracts::{contract, relational_contract, retaining_contract}`
    note the name they were asked for and whether they found it, only while a
    log is open); `contract_dependencies::within` around
    `build_function_cfg_with` (split from `build_function_cfg_unscoped`),
    `analyze_defined_function`, `infer_validated_summary` and
    `recursion::contextual_cfg_under`; `contract_dependencies::body` in the CFG
    builder (the identifiers of the body in the stream the build reads);
    `CustodyDerivation`'s end-of-derivation `inferred` notes; the loader's
    duplicate check reads the maps directly. A row's set is every lookup of
    its own work **whose name its body spells** (a minted or held spelling
    read back to its declaration: passes such as the literal-premise rewrite
    scan the whole unit inside one function's build), every callee its solved
    CFG applies a contract of (`ContractCall`, nested calls, element stores and
    releases: every contract the solver applied), and its own name (its entry
    contract), closed over every in-unit summary it consulted. Standing at the
    consultation: `held` (pending review or pin mismatch, or a held spelling),
    `withdrawn` (a withdrawal spelling, a declaration withdrawn in the unit,
    or a refuted one outside its own body), otherwise `declarations`. The
    audit opens a `Recording` around one unit's analysis
    (`qcore_audit::projection`; the certificate recheck runs outside it) and
    writes `contract_dependencies` into every row after the verdict is
    settled (`qcore_audit::contract_dependencies::annotate_unit`). **Do not**
    read the log from any analysis decision, attribute a lookup a body does
    not spell to its row, or source the set from a lexical scan alone. The
    one exception is the obligation carriage's attribution (card
    ARM-OBLIGATIONS-UNIT-GRAIN, rule 37): while a unit frame is open,
    `within` and `body` also note every function's work and body in the
    frame's own `contract_dependencies::attribution`, in every run whether
    or not a recording is open, so it never reads the audit's recording and
    leaves the recorded evidence unchanged; it may only narrow an
    obligation from the unit's rows to the rows reaching the admitted
    declaration.
    Measured (binary `377d3462…`): every row of corpus (4,834), system (448)
    and slice (404) carries it; corpus Safe rows rest on 0.77 declarations on
    average (at most 12). Controls in `tests/contract_dependencies.rs`;
    `QCore.ContractDependencies` (`an_applied_call_is_a_dependency`,
    `a_consulted_declaration_is_a_dependency`,
    `the_entry_contract_is_a_dependency`, `a_summary_dependency_propagates`,
    `closure_vectors_hold`);
    [report](../reports/verdict-contract-dependencies/README.md).

<a id="constraint-13g"></a>

13g. **A declaration is `validated` for a run only where every unit that
    examined one of its bodies validated it and at least one did; one held
    for a human act is `held`; every other is a `declared-assumption`.**
    Card VERDICT-CONTRACT-DEPENDENCIES (contract specification C06.5). A
    unit's evidence (`contract_dependencies::{examined, validated, withdrawn,
    failed, unit_evidence}`): `W4` (the final derivation's summary equals a
    declaration that has no container or counted-user clause), `entry` (the
    body under the declaration's entry custody matches every exit, `Safe`,
    no diagnostic), `11r` (a body summary confirms a declared disposition;
    shared item: `CustodyDerivation::relational_evidence`), `12w`, `13a`,
    `13c`, `13d` (the structural checks admitted it). Failures: a summary
    conflict, a refutation (C06.4), a withdrawal by the ABI or mode gate, a
    checked specification not established, a contradicted disposition, a
    failed structural check, a guard dropped. A failure wins in its unit; a
    unit that examined the body and found neither is `unvalidated`. Evidence
    is not kept under hypothetical holds (`contract_dependencies::hypothetical`
    around the second analysis of rule 10i's W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL amendment), nor from intermediate derivations.
    Each unit row carries `contract_body_evidence`; the run's standing is
    `qcore_audit::contract_dependencies::body_standing_of`. **Do not** label a
    declaration with no examined body, or one some unit did not validate,
    `validated`. Measured: corpus 330 declarations consulted, 73 validated
    (W4 45, entry 38, 13c 10, 13d 9, 13a 7, 11r 2; 28 on a structural check
    alone), 256 declared assumptions, 1 held. `QCore.ContractDependencies`
    (`a_declared_assumption_is_never_validated`,
    `no_examined_body_is_a_declared_assumption`,
    `an_unvalidated_body_is_never_validated`,
    `a_failed_body_is_never_validated`,
    `validated_needs_evidence_in_every_unit`, `a_failure_wins_in_its_unit`,
    `a_held_declaration_is_never_validated`, `outcome_vectors_hold`,
    `standing_vectors_hold`).

<a id="constraint-13h"></a>

13h. **The dependency record is evidence: no verdict reads it, and removing
    it restores every report byte for byte.** Card
    VERDICT-CONTRACT-DEPENDENCIES: the row field, the unit field and the
    sidecar's `contract_dependencies` section (each route's sidecar; the
    composer folds it from the rows like the refutation section, under the
    `contract_premises` the current route records before its units). The
    premise clause `requires resource_premise(NAME, "STATEMENT");`
    (contract specification C04.18) changes no mode and is covered by the
    pin digest (control: without the re-pin both premised declarations are
    `contract-changed`, not in force). Measured on the base's inputs plus the
    premise: corpus report projection byte-identical to BEFORE (`36fd1649…`);
    system and slice verdict rows identical (`c000abd8…` / `0cd51920…` raw
    BEFORE; their only other differences are the contract-source digests the
    current route's `compile_configuration` names), and the system run
    before the premise (`a3`) byte-identical in full; parity multisets
    identical on all three. Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the
    clause's shape is the contract format's, refused at parse in every
    module. Bounds v1 admits it only beside a scoped member form (rule 14x).
    A Quirin model contract may state it (`contract_loader::module_premises`).
    **Amended by card OUTER-CONTRACT** (rule 31; USER DECISION 2026-10-17,
    D3 "Both from one run"): the runtime projection is the **one reader** of
    the dependency record that publishes a verdict. It reads the record after
    every static verdict is written, publishes its verdicts in its own
    document (`QCORE_OUTER_PROJECTION`) beside the unchanged static ones, and
    only lowers (the join with the static verdict); no static verdict reads
    the record, and the record gains the run-set and stated member keys
    (`run_set_members`, `stated_members`) of the declarations a row rests on.
    `QCore.ContractDependencies`
    (`publishing_the_set_changes_no_verdict`,
    `two_sets_publish_the_same_verdict_row`); `QCore.OuterContract`
    (`static_projection_is_identity`, `runtime_never_promotes`).

<a id="constraint-13i"></a>

13i. **Body evidence validates the clauses it compares or reads, never a
    whole declaration by itself.** Card VERDICT-BODY-STANDING-CLAUSE-SCOPE
    (contract specification C06.5), at the user's direction "handle as safe
    as possible" (2026-10-09), taking the per-clause discharge of
    `VERDICT-BODY-STANDING-CLAUSE-SCOPE`, not the decision route.
    `crates/qcore-c-gateway/src/contract_clause_standing.rs`:
    `declaration_clauses` reads a declaration's tables into clauses (each
    parameter mode with its kind, the result mode with its kind, each
    container and counted-user clause, each retention and the guard of a
    guarded one, each holder reset, slot move, derived out slot and
    reference clause, each exit class); `text_clauses` adds, from the
    retained texts, each named premise, a variadic tail and
    `resource_no_return()` (a literal-argument premise and the loop-owner,
    thread-synchronization and lifecycle obligations bound callers and are
    not clauses); `covers` is the whole table: `W4` and `entry` cover the
    custody projection (parameters and result with kinds), `11r` every exit
    class, `12w` guards, `13a` slot moves, derived out slots and a borrowed
    result **of a declaration in the function map** (its hook reads no other
    table: a retaining declaration's borrowed result is an ordinary result
    clause), `13c` reference keeps and drops, `13d` a returned reference,
    `13y` (card CG22-CALLBACK-REGISTRY-CONTRACT) a synchronous invocation or a
    registration in a parameter holder (`CallbackChecked`); nothing covers
    the rest, every other callback clause (`Callback`) included. Shared item: `contract_dependencies::unit_evidence`
    now records `W4` whenever the final derivation's validated summary equals
    the declaration's custody projection, a declaration with container or
    counted-user clauses included (covering its custody clauses only).
    **Do not** let a structural check cover a clause of another kind (the
    `resource_borrowed` formals beside the clause it reads stay uncovered),
    count a clause covered on evidence from another unit, or read a text
    the loader refuses as declaring no text clause (`Unreadable`, never
    covered).

<a id="constraint-13j"></a>

13j. **A declaration is `validated` only when every clause is covered in every
    unit that validated it; otherwise `partly-validated` (some clause
    covered everywhere, the rest listed) or a `declared-assumption`.** Card
    VERDICT-BODY-STANDING-CLAUSE-SCOPE. Each unit row's
    `contract_body_evidence` gains `clauses`: for every declaration the unit
    validated, each clause label with the evidence that covered it there
    (`contract_clause_standing::unit_coverage`, called by
    `qcore_audit::contract_dependencies::unit_field`). The run folds it
    (`Facts`; the composer folds the same unit rows): rule 13g's standing is
    the precondition; under it a clause is `validated` when every validated
    unit covered it, and the declaration is `validated` only when it has a
    clause and every clause is (`clause_standing_of`). The sidecar publishes
    each declaration's `clauses` (per-clause standing) and, for a
    partly-validated one, `uncovered_clauses`; `counts.partly_validated`.
    A unit row without `clauses` (a report written before this rule) never
    makes a declaration validated. **Do not** publish `validated` for a
    declaration with an uncovered clause, a premise, or no clause.
    `QCore.BodyClauseStanding` (`validated_needs_every_clause`,
    `an_uncovered_clause_is_never_validated`,
    `a_partly_validated_declaration_has_an_uncovered_clause`,
    `a_declaration_with_a_premise_is_never_validated`,
    `structural_evidence_alone_never_validates_a_formal`,
    `a_structural_check_covers_only_what_it_reads`,
    `a_structural_check_never_covers_a_formal`,
    `nothing_covers_these_clauses`, `validated_implies_previously_validated`,
    `coverage_vectors_hold`, `clause_standing_vectors_hold`).

<a id="constraint-13k"></a>

13k. **The Safe roll-up's "only validated" counts rows whose every
    declaration is fully validated; a row on a partly-validated declaration
    and no declared assumption is counted apart; no verdict reads any
    label.** Card VERDICT-BODY-STANDING-CLAUSE-SCOPE:
    `safe_rollup.resting_on_a_partly_validated_declaration` and
    `partly_validated_declarations` (each with its Safe rows and uncovered
    clauses); the console tally names both. Every row counted "only
    validated" was so counted under rule 13g
    (`the_validated_only_rollup_is_a_subset`). Only the unit field's
    evidence and the sidecar section change: removing the unit key restores
    every report byte for byte (row fields included) and removing the
    section restores the sidecar. Measured (binary `65d7c0d8…`, base
    `9564eeb6d`): verdict rows identical on all three routes (`97cbdcf3…`,
    `09b7a59e…`, `ce1be80e…`), report and sidecar projections identical,
    parity multisets identical; corpus validated 73 → **48** (25 of the 28
    structural-alone declarations `partly-validated`, their `resource_borrowed`
    formals and plain results uncovered; 3 stay validated: two 11r exit-class
    declarations and the parameterless `ut_create_mock_queue`), partly
    validated 29 (with 4 counted-user declarations `W4` now covers in part);
    Safe roll-up 418 / **64** only validated / 22 partly / 377 declared
    assumption (64 of the previous 85 kept); system 47 / 3 / 2 / 35, slice
    29 / 0 / 2 / 14. `QCore.BodyClauseStanding`
    (`the_validated_only_rollup_is_a_subset`,
    `validated_implies_previously_validated`);
    [report](../reports/verdict-body-standing-clause-scope/README.md).

<a id="constraint-13l"></a>

13l. **A `#` form's keywords are reserved and an `@` name is a C
    enumerator: Clang resolves it in the guarded result's own enumeration,
    the pin covers that enumeration, and an alias is one module's second
    spelling of one `@` name.** Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF
    (contract specification C04.19.5, grammar G05 and G05d), on the user's
    decisions of 2026-10-15. `#` marks binders (`#result`, `#rko`) and value
    keywords (`#absent`, `#owner`, `#callback`); the clause-kind word after
    `quant` stays bare. The lexer reads `@` as `Symbol::At` (a lexer error
    before; the census found no `@` and no `alias` in code position). `alias
    #NAME = @C_NAME;` (`SourceProgram::aliases`) stands only in a contract
    module, once per name, its name no reserved keyword and no parameter of
    any declaration of the module (`contract_handoff::module_aliases`,
    `alias-collision`); it is part of the contract digest of every
    declaration spelling `#NAME` (`declaration_digests`), so an alias never
    changes meaning under an unchanged pin. A declaration carrying a `#`
    form rejects a parameter named `absent`, `owner`, `callback` or
    `result` (its binder would read as the keyword); a declaration with no
    `#` form keeps such a name (13 parameters in the tree, none beside a `#`
    form). **Resolution.** Per unit, `@C` resolves to an enumerator of the
    candidate's own result type through Clang's AST
    (`MemberFacts::{result_types, typed_constants}`,
    `handoff_branch::resolve`; units in scope must agree), otherwise the
    hand-off is refused there; on the pinned route, after the pre-pass, a
    hand-off whose candidate a read unit declares and whose `@C` does not
    resolve rejects the run's contracts
    (`member_prepass::check_handoff_constants`): the loader runs no Clang.
    **Pinning.** `quirin-audit pin` adds the top-level enumeration declaring
    each `@` name a declaration spells, directly or through an alias
    (`declaration_c_names`), to its reviewed identity (`c_identity`, site
    kind `enumeration`, the whole statement's digest; `identity_sites` at
    pin, check and audit), and refuses a name no enumeration declares; the
    enumerator's value is so pinned at both revisions. **Do not** resolve an
    `@` name by its spelling alone, accept a bare `absent`, read an alias
    across modules, or keep a declaration in force whose enumeration
    changed. `QCore.ReturnCodeHandOff` (`an_alias_means_its_name`,
    `an_undeclared_alias_rejects`, `the_migration_preserves_absent`,
    `a_bare_absent_rejects`, `a_function_designator_is_unchanged`);
    `crates/qcore-contract/tests/return_code_handoff_clause.rs`,
    `c_identity`'s `an_enumerator_is_pinned_by_its_whole_enumeration`.

<a id="constraint-13m"></a>

13m. **A W4 conflict that only refines a declaration is a body failure
    unless the declaration's own entry check passes on that body; passing,
    it is `13m` evidence covering the custody projection.** Card
    DECLARED-ASSUMPTION-BODY-CHECKS (contract specification C06.5), at the
    user's direction "make the Safe count more trustworthy rather than larger"
    (2026-10-09). `crates/qcore-c-gateway/src/legacy_declared_entry_check.rs`:
    for a function-map declaration that is its own custody projection, in
    force in the derivation exactly as declared, not refuted (C06.4), not
    withdrawn by the mode gate and not a withdrawn checked specification
    (C06.3), whose final W4 summary conflicts with it (`checked`), the body is
    analysed under the declaration's entry custody on the CFG the reported run
    builds (a derived result on the derived-return CFG) and passes only when
    `observations_match_contract` holds at every exit, the result is `Safe`
    with no diagnostic, no callee double release is found and rule 11y's
    refusals name nothing. `unit_evidence` records through `recorded`: `13m`
    and no `conflict` failure when it passed, the failure otherwise; `covers`
    gives `13m` exactly the clauses of `W4` and `entry`. Shared items:
    `contract_dependencies::unit_evidence` (takes the check's result),
    `contract_clause_standing::{EVIDENCE, covers}`; the hook reads
    `dialect_hook::Extensions` before `unit_evidence`. **Do not** check a
    declaration with a container or counted-user clause, a contradiction the
    mode gate withdrew, or a body without a conflict (a scope-only route was
    measured establishing nothing and withdrawn); never install the
    declaration in place of the summary or clear the row's
    `SummaryConflict`. Measured (binary `7b8460cc…`, base `d8a1a2a4e`):
    `rd_free` (66 units), `mk_mem_free` (26), `rd_strndup` (13),
    `rd_kafka_mem_free` and `flb_free` validated; `rd_malloc`, `rd_calloc`
    and `rd_strdup` covered everywhere but `rdhttp.c` (no W4 summary there).
    `QCore.DeclaredEntryCheck` (`new_evidence_needs_the_entry_check`,
    `new_evidence_needs_a_conflict`,
    `a_container_or_counted_user_declaration_is_never_checked`,
    `a_withdrawn_or_refuted_declaration_is_never_checked`,
    `a_failed_check_keeps_the_conflict_a_failure`,
    `a_contradiction_still_fails`, `without_a_passing_check_nothing_changes`,
    `only_a_conflict_changes`, `the_new_evidence_covers_only_custody`,
    `recording_vectors_hold`); controls in `tests/declared_entry_check.rs`;
    [report](../reports/declared-assumption-body-checks/README.md).

<a id="constraint-13n"></a>

13n. **The declared-entry check is evidence only: it runs only while body
    evidence is collected, with parity rows muted and the dependency recorder
    suspended, and no verdict reads it.** Card
    DECLARED-ASSUMPTION-BODY-CHECKS. Shared items:
    `contract_dependencies::collecting` (a recording is open and no
    hypothetical analysis runs) and `contract_dependencies::unrecorded` (the
    recorder taken out and a fresh consultation log opened and discarded
    around the work), with `typed_access_path::without_parity_rows`. **Do
    not** run it without a recording, let it note a consultation, a body
    spelling or evidence of its own, or feed its result to any analysis.
    Measured (base `d8a1a2a4e`, binaries `f4b6b753…` → `7b8460cc…`): verdict
    rows identical on all three routes (`97cbdcf3…`, `09b7a59e…`,
    `ce1be80e…`), report and sidecar projections identical, parity multisets
    identical (662,667 / 139,962 / 50,407 rows); corpus Safe roll-up 418 /
    63 / 23 / 377 → 418 / **91** / 23 / **349** (validated 46 → 51,
    declared assumptions 252 → 247), system 47 / 1 / 4 / 35 → 47 / 2 / 4 /
    34, slice 29 / 0 / 2 / 14 → 29 / 2 / 2 / 12.
    `QCore.DeclaredEntryCheck` (`without_a_passing_check_nothing_changes`);
    `contract_dependencies` unit test `unrecorded_work_leaves_no_trace`.

<a id="constraint-13o"></a>

13o. **A unit whose Clang JSON AST nests beyond `serde_json`'s default
    recursion limit keeps its typed index when it nests at most 256 levels.**
    Card RDHTTP-HEADER-SUMMARY-GAP, at the user's direction "handle as safe
    as possible" (raise the checked count without weakening any check).
    `crates/qcore-c-gateway/src/deep_unit_ast.rs` (hook:
    `typed_access_path::AstIndex::from_json`): the whole-unit document is
    parsed as before; only when that parse fails is its nesting measured
    (`depth`, counted exactly as `serde_json` counts it), and a document of at
    most `ADMITTED_DEPTH` (256, twice the default 128) is parsed again with
    the limit disabled on a dedicated 256 MiB stack, as the typed dispatch and
    index-extent readers already parse. The text and the parser are the same,
    so the index is the document the default parse would have read without
    its stack guard; every reader then reads the unit as any other, and no
    reader's admission changes. Shared items: the hook in `from_json` (one
    call) and the comment of `typed_access_path::definition_ast`. **Do not**
    raise the bound without measuring the readers' recursion on the deepest
    admitted unit, read a different document, or let a unit parse
    differently because it is deep. Cause isolated: librdkafka's `rdhttp.c`
    nests 224 levels (four definitions calling `curl_easy_setopt`, whose GCC
    type-check macro from `curl/typecheck-gcc.h` expands into nested
    conditionals); without its index no definition of the unit, header
    bodies included, had a typed reading, so `rd_malloc`'s, `rd_calloc`'s and
    `rd_strdup`'s `rd_assert(p)` (glibc's `assert`, a statement expression
    under `gnu17`, rule 11g) was never read, W4 published no summary there and
    rule 13m had no conflict to check. Measured (binaries `18cf8371…` →
    `b51d4b20…`, base `b21f0c9a2`): corpus 881 / 9 / 3,944 unchanged, no
    status move; every report and parity difference is in `rdhttp.c` (13 rows'
    leaves, all refusals: the four `typed AST is unavailable (CG30)` leaves
    gone, 939 uncontracted calls now refused, 908 of them the type-check
    macro's warning functions; no leak or violation added); system and slice verdict rows and parity
    multisets identical; `rd_malloc`, `rd_calloc`, `rd_strdup` validated
    (`13m` in `rdhttp.c`), corpus Safe roll-up 418 / 91 / 23 / 349 →
    418 / **94** / 23 / **346**, declarations 51 → **54** validated,
    247 → **244** declared assumptions. `QCore.DeepUnitAst`
    (`an_index_read_before_is_read_after`, `the_index_is_the_document`,
    `only_the_deep_band_changes`, `depth_vectors_hold`, `band_edges`);
    controls in `tests/deep_unit_ast.rs`;
    [report](../reports/rdhttp-header-summary-gap/README.md).

<a id="constraint-13p"></a>

13p. **Beyond the bound, or on a failed reparse, the unit keeps no index and
    the default parse's error stands; a body it examined without evidence
    stays `unvalidated` in rule 13g's fold.** Card RDHTTP-HEADER-SUMMARY-GAP.
    A malformed document, a thread that cannot start and a document nesting
    more than 256 levels return the default error, so the unit is as before
    (rule 11y's unreadable-AST refusal included). **Do not** change the
    fold to skip a unit that examined a body without validating it, or read
    a partial index. Control: a header wrapper beside an `else if` chain of
    130 arms (JSON depth 273) stays `Unvalidated`. `QCore.DeepUnitAst`
    (`beyond_the_bound_there_is_no_index`,
    `a_unit_without_an_index_keeps_the_declaration_unvalidated`,
    `the_fold_never_skips_a_unit`).

<a id="constraint-13q"></a>

13q. **The analysed-unit filtered launch and the closed-world whole-unit AST
    read a document within rule 13o's bound the same way.** Card
    DEEP-AST-TRANSPORTS-AND-WALL-TIME, at the user's direction "handle as safe
    as possible". `typed_c_ast::unit_declarations` (Clang's name-filtered dump
    for a consultation of the analysed unit: `typed_access_path::definition_ast`,
    read by rule 11y's extension fallback and the retention guard's shadow
    check, and `appended_declarations`, the rule-12u probes) parses the stream
    as before and, only when that fails, measures the text's nesting (its
    deepest value's) and reads a text of at most 256 levels again without the
    limit on a dedicated 256 MiB stack (`deep_unit_ast::values_from_slice`);
    `typed_c_ast::translation_unit_ast` (rule 11i's closed world) reads its
    document through `deep_unit_ast::from_slice`. Beyond the bound, or on a
    failed reparse, the default error stands (rule 13p's shape). The two
    launches keep separate caches, so a stream only the band reads is never
    answered to an importer. Every reader of what they return is measured at
    the bound with the stack high-water mark of a fresh thread (debug build,
    `deep_ast_transports::stack::high_water`): 236 KiB for an `else if` chain
    whose filtered stream nests 255 levels, 232 KiB for a nested conditional
    expression (253), 232 KiB for the closed world at 256, and 232 KiB on
    librdkafka's `rd_http_ssl_configure` itself (its filtered value nests 221;
    56 KiB on a shallow definition), each within the controls' 1 MiB budget,
    half an analysis worker's 2 MiB stack. **Do not** admit the band on a
    transport whose readers are not measured, raise the bound without
    measuring them again, or let the importers read the band through the
    shared cache. Corpus, system and slice outputs identical (no consultation
    on a deep definition there). `QCore.DeepAstTransports`
    (`reads_iff_within_the_bound`, `the_band_keeps_every_default_reading`,
    `nothing_beyond_the_bound`, `a_stream_is_read_iff_every_value_is`,
    `fits_at_the_bound_fits_within_it`); controls in
    `deep_ast_transports::tests`;
    [report](../reports/deep-ast-transports-and-wall-time/README.md).

<a id="constraint-13r"></a>

13r. **Every other Clang JSON transport keeps its limit, each for a recorded
    reason.** Card DEEP-AST-TRANSPORTS-AND-WALL-TIME. The inventory is
    `deep_ast_transports::TRANSPORTS` (bound to `QCore.DeepAstTransports.transports`):
    the source importers' `typed_c_ast::ast_declarations` and `function_ast`
    (`typed_source`, `boolean_source`, `typed_source_match`, `layout_extents`,
    `cast_representation`: interpreters whose recursion is not measured deep,
    over named functions of fixed shapes, so a refusal is the fail-closed
    answer), `c_numeric_obligations::typed_ast` (no route consumes it) and the
    FFI layout probe (`ffi_layout`) keep `serde_json`'s default limit. The
    typed dispatch census and the bounds side pass already parse without a
    limit on their own 256 MiB stacks, where their readers run too; they are
    recorded, unchanged (residual `UNBOUNDED-DEPTH-AST-READERS`). **Do not**
    move a transport between bounds without measuring its readers and
    updating the table and the Lean list together.
    `QCore.DeepAstTransports` (`a_default_transport_is_unchanged`,
    `the_band_transports`); `deep_ast_transports::tests::lean_transports_are_the_rust_transports`.

<a id="constraint-13s"></a>

13s. **A callee's call-exit resolution is computed once per callee per
    function of one unit index.** Card DEEP-AST-TRANSPORTS-AND-WALL-TIME.
    `function_reading_reuse::FunctionReuse::resolution` (hook:
    `legacy_call_exits::call_exit_resolution` through
    `StatementIndex::call_resolution`) answers `call_exits::resolve(function,
    callee, no_return_declarations)`: computed by `resolve` on the first ask
    for that name and stored in the function's entry of the unit index (one
    Clang document of one launch identity), answered from it after. The
    measured cost it removes: the parity-only call-exit census walked each
    function once per uncontracted call statement, 33 % of `rdhttp.c`'s
    samples (908 calls of 44 warning functions in four macro-expanded
    definitions). **Do not** key the memo on anything but the callee within
    one function entry, share it across documents, or store an answer other
    than `resolve`'s. Measured identity and wall time in the report.
    `QCore.DeepAstTransports` (`the_memo_answers_as_the_function`,
    `asking_keeps_the_memo_sound`, `every_ask_is_answered_by_the_function`);
    `function_reading_reuse::tests::the_memo_is_resolve_for_every_name`.

<a id="constraint-13t"></a>

13t. **`elaboration_node` reads each node's lines from the function's line
    tree and each span's tokens from a per-function memo.** Card
    DEEP-AST-TRANSPORTS-AND-WALL-TIME. `Parser::elaboration_node`'s walk takes
    a node's begin and end line from `function_reading_reuse::LineTree`
    (built on the walk's first descent into a node, in `inner` order, from
    `StatementIndex::node_line` and `node_end_line`) and a candidate's tokens
    from `StatementIndex::node_tokens_once` (one tokenization per source span
    per function). The tree pairs each child with its own entry and the memo
    stores `node_tokens`' own answer, so the walk visits, prunes and picks
    exactly as before. The measured cost it removes: 44 % of `rdhttp.c`'s
    samples, where the type-check macro puts whole subtrees on one line.
    **Do not** read a line tree beside a node it was not built from, or
    reorder the walk. `QCore.DeepAstTransports`
    (`the_annotated_walk_is_the_walk`, the memo theorems);
    `function_reading_reuse::tests::the_line_tree_mirrors_the_function`,
    `a_span_is_tokenized_once`.

<a id="constraint-13u"></a>

13u. **A synchronous invocation reads as its callee's custody contract only
    when every candidate callback's own contract is the identity on the
    caller's state; it is never read as a benign borrowed pointer.** Card
    CG22-CALLBACK-REGISTRY-CONTRACT (contract specification C04.19, grammar
    G05), on the user's decision of 2026-10-05 (class 1 shape (c)).
    `crates/qcore-c-gateway/src/callback_contracts.rs` (`classify`,
    `requirements`, `decide`), `legacy_callback_route.rs`
    (`Parser::callback_operands`, `callback_edges`), loader
    `crates/qcore-contract/src/contract_callback.rs`, schema
    `qcore_schemas::callback`; hooks in `legacy_retained_by.rs`
    (`retaining_call`: a callback declaration is a retaining one, withheld
    from the function map) and `legacy_summary.rs` (the call-graph edges).
    `resource_callback_invoked(F, A…)`: during the call only, `F` is called
    zero or more times with `(A…)` (a parameter word, `_` a value the callee
    supplies). At a call: the candidates are rule 11p's resolution of the
    actual over Clang's AST, read at every Clang call of the callee on the
    statement's line and kept only when all agree (glibc's
    `__glibc_const_generic` puts one `bsearch` per `_Generic` association
    there); an open set refuses (`callback-undecided`, naming 11p's reason),
    a candidate with no contract in force (no declaration in the function
    map, no validated in-unit summary) refuses (`callback-uncontracted`), a
    candidate whose contract is not the identity (arity, a mode other than
    `resource_borrowed`/`resource_owned`, a kind, a result, a side clause)
    refuses (`callback-effectful`). `resource_owned` is a requirement: met by
    the callee at `_`, and at a parameter position put on the call itself
    (the classifier reads that position `resource_owned(n)`, the requirement
    a direct call of the candidate with that actual carries). Then any number
    of invocations is the identity and the call is its custody contract so
    strengthened. A designator at a callback position is a W4 call-graph edge
    (summarised first). A named premise (C04.18) on a candidate's
    declaration is not a side clause: it changes no mode, and it is published
    with the candidate's declaration (card OP-CALLBACK-CANDIDATE-CONTRACTS:
    `rk_log_callback` and the other premises of the op-callback candidates).
    A row deciding an invocation through candidates its body does not spell
    lists them among its own dependencies since card
    HANDOFF-CANDIDATE-OWN-CALLBACKS (below), except through a registry or a
    decided registration (`MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED`). **Do not** read a callback as borrowed, admit a
    candidate on a summary of another unit, admit a releasing or keeping
    candidate outside a registry's `element` position, or place a call whose
    Clang calls disagree. **Amended by rule 15w**: an actual that is the
    calling function's own callback formal, handed on exactly as that
    function's declaration in force states its invocation, is that declared
    invocation (decided at its own callers), not an open set. **Amended by
    rule 30**: a candidate whose declaration states member-path states (and
    nothing else beside its custody contract and a hand-off) reads as that
    contract, its states composed with the invoker's arms at the paths the
    caller linked below a parameter actual. **A candidate with its own
    member invocations** (card HANDOFF-CANDIDATE-OWN-CALLBACKS, contract
    specification C04.19.7; amends 12m and 12n with it): a candidate whose
    declaration in force carries `resource_callback_member_invoked` clauses
    and nothing else beside its custody contract and a hand-off
    (`candidate_composition::composable`) is the composition of its own
    clauses at the invocation (`Parser::classified_in`, `Parser::composed`):
    each own invocation decided as rule 12n decides it in the candidate's own
    body, over the **run's** set only (no pre-pass: refused, since the fold
    checks only the members of the declarations a row lists), each inner
    candidate classified with the candidate as rule 12m's invoker, then
    `decide_run`; its formals strengthened by every inner requirement
    (`strengthen`: a borrowed formal an inner candidate requires live reads
    `resource_owned(n)`); its hand-off checked at the outer invoker after
    the composition (`handed_with`); an inner member-path state at a formal
    the outer invocation hands a parameter of its caller refuses (two levels
    of rule 30 are not composed). Contracted only when every own invocation
    decides; otherwise `callback-own-invocation-refused`, naming the
    invocation and its inner refusal (`Classified::detail`). **Least fixed
    point** (as rule 15w): a candidate reached again while its own
    invocations are decided refuses (`with_stack`, naming the cycle), so a
    cycle never supports itself; a coinductive reading is not adopted. A
    declaration with any other own callback clause stays uncontracted as a
    candidate. **Do not** decide an own invocation over a unit's own set,
    admit a candidate on its own support, or compose a member-path state
    through two invocations. `QCore.CandidateComposition`
    (`a_composed_candidate_is_contracted_iff_each_own_invocation_decides`,
    `an_undecided_own_invocation_refuses`, `a_cycle_never_supports_itself`,
    `a_self_cycle_never_stands`, `a_two_candidate_cycle_never_stands`,
    `compose_vectors_hold` bound to `COMPOSE_VECTORS`);
    `tests/handoff_candidate_own_callbacks.rs`. A decided synchronous, table
    or member invocation records its candidates (and a composed candidate's
    inner candidates) as the row's dependencies
    (`contract_consultation::note_relied`; registry invocations and decided
    registrations do not yet, `MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED`).
    Measured (corpus `66910c10`, base `d708641bc`, binary `049bf574…`): 936 /
    9 / 3,889 → **938 / 9 / 3,887**: `rd_kafka_oauthbearer_refresh_op`
    composes, so `rd_kafka_op_destroy`'s 131 leaves in 83 rows decide and
    `rd_kafka_event_destroy` and `ut_op_destroy_free` become Safe (34
    declarations listed each); `rd_kafka_consume_cb` composes;
    `rd_kafka_poll_cb` refuses at its own run-open `rko_u.offset_commit.cb`;
    system and slice byte-identical, null tallies identical;
    [report](../reports/handoff-candidate-own-callbacks/README.md).
    `QCore.CallbackContracts`
    (`invocations_of_identities_are_identity`,
    `an_identity_decision_is_the_custody_reading`, `decide_refuses_*`,
    `identity_only_borrows_or_requires`, `requirements_are_owned_parameters`);
    `tests/cg22_callback_registry.rs`.

<a id="constraint-13v"></a>

13v. **A table dispatch closes its candidates only over an automatic array's
    braced initializer that the body names nowhere but as whole table
    actuals.** Card CG22-CALLBACK-REGISTRY-CONTRACT.
    `resource_callback_table(T, A…)`: during the call only, function
    pointers read from the array `T` designates are called zero or more times
    with `(A…)` (the cmetrics and ctraces map unpackers). The candidates are
    every function designator of the actual's initializer
    (`Parser::table_candidates`): the actual is a named automatic array of
    the calling function with a braced initializer, every reference to it is
    a whole table actual of a declared table dispatcher, and every other
    initializer leaf is a string, a scalar or a null pointer constant (never
    called: premise `P-CG22-DEFINED-POINTER-VALUE`); anything else is an open
    set (`callback-undecided`, naming why). The candidates are then decided
    as rule 13u's. **Do not** close a table that is written, address-taken,
    passed elsewhere, static or reached through a pointer.

<a id="constraint-13w"></a>

13w. **A registry is used only under a registration in force on every path
    in the calling body.** Card CG22-CALLBACK-REGISTRY-CONTRACT
    (`QCore.IndirectCalls.callbackTrace`'s premise: an invocation needs the
    registration in force). `resource_callback_registered(F, H)` records on
    the path (`AbstractState::registries`, joined by agreement: a
    registration not made alike on every arriving path is undecided) the
    candidates of `F` (a null constant: none) with their contracts, for a
    holder the body names: `&L` for `L` one automatic, non-pointer,
    non-array object named only as whole `&L` actuals of holder-naming
    callees, or one tracked live identity (a constructor's `result`
    included); any other holder refuses at the call
    (`callback-registry-undecided`). `resource_callback_registry_invoked(H,
    A…)`, `resource_callback_unregistered(H)`, `resource_elements_dropped(H)`
    and `resource_released_if_constructed(H)` read the registration in force
    before the call; none in force refuses (`callback-registry-undecided`).
    An invocation over identity candidates changes nothing; over candidates
    that all release the `element` they receive it releases the elements the
    registry took (rule 13x); a mixed, uncontracted, effectful or
    requirement-bearing registration refuses. **Do not** read a registration
    made in another frame (a holder reached through a formal or a member),
    or keep one across a join that does not agree. **Through a member**
    (card OP-CONSTRUCTOR-DECLARATIONS, C04.19.4, on the user's decision of
    2026-10-14, B1): `resource_callback_member_registered(result, TAG.FIELD,
    F)` records on the constructor's tracked result, per path, the candidates
    of `F` (rule 11p; a null constant: none) in its member `TAG.FIELD`, and
    rule 14e's `absent` records none; a join keeps it only when every
    arriving path holds it alike. A later `resource_callback_member_invoked`
    on that object (rule 12n), when the unit's closure refuses, reads the
    registration instead: an identity set decides, `absent` invokes nothing,
    and anything else refuses naming the registered candidate; with no state
    in force there the closure's refusal stands. The state is read only where
    the unit's stores into the member keep it (rule 14e) and the calling
    function stores into it nowhere itself, and the run's fold demotes a row
    resting on the constructor when the run's stores do not keep it
    (`constructed-state-open`). `QCore.CallbackContracts`
    (`an_invocation_needs_a_registration`,
    `an_undecided_registration_refuses`, `the_callback_trace_agrees`);
    `QCore.OpCallbacks`
    (`a_registration_then_destroy_invokes_only_the_registered_candidate`,
    `an_absent_member_invokes_no_candidate`,
    `an_unclassified_store_voids_the_absent_state`).

<a id="constraint-13x"></a>

13x. **An element's custody passes to a registry only when the registry
    releases its elements and is a tracked identity; a list frees itself
    only when a constructor made its registration.** Card
    CG22-CALLBACK-REGISTRY-CONTRACT. `resource_element_added(E, H)`: under a
    releasing registration held by a tracked identity the element is
    contained in the holder (`attach_contained_resource`: the holder's escape
    carries it, the registry's invocation releases it); under a releasing
    frame registration it refuses (`callback-element-custody-undecided`: a
    frame object has no identity to own it); under no callback or identity
    callbacks the holder only retains it (rule 12t's decision table, a frame
    or tracked holder). A plain release of a holder whose registry still owns
    elements refuses (`refuse_release_of_registry`), never reads as releasing
    them; elements dropped without their release refuse.
    `resource_released_if_constructed(H)` (`rd_list_destroy`'s
    `RD_LIST_F_ALLOCATED`) releases the holder after the registry's effects
    exactly when the registration in force was a constructor's (`result`
    holder), and nothing for an in-place one. **Do not** move an element into
    a frame registry, read a holder's release as releasing its elements, or
    release a holder whose registration is undecided. `QCore.CallbackContracts`
    (`a_moved_element_is_released_by_its_registry`,
    `a_moved_element_left_at_the_exit_is_lost`,
    `a_moved_element_dropped_without_invocation_is_lost`,
    `a_retained_element_at_the_exit_is_refused`).

<a id="constraint-13y"></a>

13y. **A synchronous invocation or a registration in a parameter holder
    stands in a unit that defines its body only when the body validates it.**
    Card CG22-CALLBACK-REGISTRY-CONTRACT (`callback_route::settle`, beside
    rules 13a and 13c at the three entries of `legacy_c.rs`). An invocation:
    the body names `F` only as the callee of direct calls (`F(…)`,
    `(*F)(…)`) whose actuals match `A…` position by position (a parameter
    word: that formal, never rebound or address-taken; `_`: anything not
    naming `F`). A registration: the body names `F` only as the value of
    stores `H->M = F;`, at least once, `F` and `H` never rebound. Parameter
    names are read as rule 12w's `definition` reads them, plus an inline
    `RET (*NAME)(…)` declarator; an unread name never validates. A body that
    does not validate withdraws the declaration in that unit (every call
    refuses, `callback-withdrawn`; no inference reads the body in its place).
    Evidence `13y` covers exactly those clauses (rule 13i,
    `ClauseKind::CallbackChecked`); every other callback clause is a declared
    assumption. **Do not** validate a clause on a body that hands `F` on,
    rebinds it, or stores it anywhere else. **Amended by rule 15w**: a body
    whose every mention of `F` hands it on unchanged to the same clause
    position of a callee whose declaration is in force (an exact forward)
    lets the invocation stand, without `13y` evidence; every other hand-on
    still withdraws.

<a id="constraint-13z"></a>

13z. **No declaration reads a callback as benign: the two that did are
    re-declared, and their callers' `Safe` is corrected.** Card
    CG22-CALLBACK-REGISTRY-CONTRACT. `rd_kafka_topic_partition_list_sort`
    gains `resource_callback_invoked(cmp, _, _, opaque)` (its body rebinds
    `cmp` to a default and hands it to `rd_qsort_r`, so rule 13y withdraws it
    in `rdkafka_partition.c`); `msgpack_packer_init` gains
    `resource_retained_by(data, pk)` and `resource_callback_registered(callback,
    pk)` (rule 13y validates the registration). Members declared:
    `bsearch`, `qsort`, `pthread_once` (`quirin-provider-libc`), the cmetrics
    and ctraces array and map unpackers, `rd_list_init`, `_new`, `_add`,
    `_destroy`, `_clear`, `pb_write`, `pb_encode_varint`; each body read at
    `66910c10` and `046accf6`, pinned at both. Not declared:
    `rd_kafka_q_serve*` (its callbacks take over ops the callee supplies) and
    `rd_kafka_op_destroy` (a member callback registered elsewhere and a
    type-switched union release); the served-queue premise (C04.18) on
    `rd_kafka_q_wait_result` and `rd_kafka_q_purge0` stays: the op callbacks
    they run are read from op members registered by the op's creator, an open
    set no registration in force decides. Measured (corpus `66910c10`, rebased
    onto `d03c59c73`): **905 / 9 / 3,920 → 903 / 9 / 3,922** (on the card's
    base `84ec54d1d`: 881 / 9 / 3,944 → 879 / 9 / 3,946), exactly two
    `Safe → Unknown` corrections (`rd_kafka_topic_partition_list_sort_by_topic`,
    `_by_topic_id`, whose comparators have no contract in force), no
    `Unknown → Safe`, no leak, violation or `Unsafe` added; system 89 / 4 /
    355 and slice 45 / 1 / 354 / 4 Pending unchanged in status;
    [report](../reports/cg22-callback-registry-contract/README.md).

<a id="constraint-14a"></a>

14a. **A scalar statement whose value holds exactly one call is the call, then
    the statement's typed reads, only when the call takes no custody.** Card
    CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (`legacy_residual_access.rs`
    with the Clang reading in `residual_access_reading.rs`; one thin hook in
    `legacy_c.rs::expression_statement`, after every statement fallback and
    before rule 12k; shared items added: `Reader::opaque_call` in
    `typed_access_path.rs`, read only by this rule, and
    `StatementIndex::statement_node`, read out of `statement_with` unchanged).
    For exactly the gate's refusal of a statement with no braces that reads no
    tracked descriptor, matched to one Clang statement (`T x = E`, `x = E`, or
    `P = E` with `P` a typed scalar place, never a pointer destination) whose
    value holds exactly one `CallExpr` with a plain callee: the statement is
    read by the typed path reader with that call an opaque scalar operand
    (every per-hop obligation of each place read or stored, no connective, no
    `?:`, no cast but a scalar one), and the call is lowered alone, as the
    statement `f(args)` (W44's reads when the gate refuses its actuals). The
    pair is admitted only when the callee's contract in force (no review, pin
    or refutation hold) has a `None` result and every declared position
    `Borrowed`, and the call lowers to exactly that contracted call with no
    destination; the block is the call (after its reads), then the
    statement's typed reads and store. C leaves the order of the call and the
    other operands unspecified (C11 6.5p2, 6.5.2.2p10): a call that takes and
    releases no custody changes no binding a read is decided on, so either
    order decides alike. Every other answer keeps the gate's refusal byte for
    byte with its counters restored. **Do not** admit two calls, a
    custody-taking or releasing position, an owned or derived result, an
    uncontracted or held callee, a pointer-valued destination or a
    conditionally evaluated call under this rule. Parity runs record every
    reading (`position: "residual-access"`, `rule: "14a"`, the callee and the
    answer). Premises stated, not proved: faithful Clang extraction and node
    matching (trust boundary 28). Measured (corpus `66910c10`, main
    `84ec54d1d` plus the card): 131 readings admitted, 1,135 declined; **+8
    Unknown → Safe** (`rd_kafka_broker_outbufs_space`, the four
    `calculate_share_*_rate`, `rd_kafka_cgrp_set_last_err`,
    `rd_kafka_cgrp_update_session_timeout`, `rd_kafka_msgbatch_set_first_msg`).
    `QCore.ResidualAccessReadings` (`a_borrowed_call_changes_no_custody`,
    `a_read_is_decided_alike_on_either_side_of_the_call`,
    `a_releasing_call_does_not_commute`, `an_admitted_call_takes_no_custody`,
    `scalar_call_vectors_hold`), bound to `SCALAR_CALL_VECTORS`;
    [report](../reports/cg10-indirect-access-sole-residuals-r2/README.md).

<a id="constraint-14b"></a>

14b. **A pointer local given an interior pointer of a never-bound root binds
    the pointer read's target-free alias.** Card
    CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (same module and hook as rule
    14a). For the gate's refusal of a call-free statement declaring or
    assigning a pointer variable by name, whose value is `&PATH` (rule 10n(1)'s
    address reader: `PATH` a member or element place whose type is not a
    pointer, an array or a function) or `PATH ± n` (`PATH` a pointer-valued
    member, element or `*` path, `n` a scalar value), with **every root of the
    plan never bound** (the statement route's closure, rule 10o): the
    statement is `IndirectScalarAccess` with `PATH`'s per-hop obligations and
    a `borrowed_destination`, exactly what a pointer-valued path read
    `p = s->f;` binds — target-free, unknown, possibly null (C11 6.5.3.2p3:
    `&*E` may be null). The value points into the object `PATH` reaches, which
    the never-bound premise puts outside the tracked graph. A release, a
    publication or a dereference through the alias is decided by its own rule
    and never by the gate. **Do not** admit a bound root, a cast or call value,
    the address of a pointer member (a slot) or a member destination under
    this rule, or bind the alias non-null or to a target. Parity rows as rule
    14a's (`rule: "14b"`, its form). Measured: 89 readings admitted; **+1
    Unknown → Safe** (`update_offset`); in most functions the refusal moves to
    the alias's uses (`append`, `peek`, `sink_peek`,
    `rd_kafka_share_ack_type_from_msg_op`, `mk_sched_broadcast_signal`: an
    unresolved identity where the alias is dereferenced, possibly null;
    `header_lookup`: its publication refused, the alias not one live object).
    `QCore.ResidualAccessReadings` (`interior_vectors_hold`,
    `a_bound_root_is_never_read`, `the_interior_alias_is_the_pointer_reads`,
    `a_release_through_the_alias_discharges_nothing`,
    `the_alias_is_never_known_non_null`), bound to `INTERIOR_VECTORS`.

<a id="constraint-14c"></a>

14c. **An address handed to a `Borrowed` or data-tail actual may be a scalar
    element `&PATH[I]`.** Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2
    (`scalar_element_address.rs`, a thin hook in
    `typed_position_shapes.rs::address`). It amends rule 10n(1), whose
    callee shape was exactly a member: the place may also be an element whose
    Clang type is a builtin scalar or an enumeration, so the array it belongs
    to holds no pointer a callee writing through the address could reach.
    Rule 10n(1)'s type test, its per-hop obligations and its never-bound
    premise on every root stand. **Do not** admit an element of pointer,
    record or array type, or relax the root premise. Premise stated, not
    proved: the callee writes within the array the element belongs to (bounds
    are outside the property, as for every W44 actual). Measured: 19 actuals
    admitted that the base never read; **+7 Unknown → Safe**
    (`metric_hist_exchange` in two units, `cmt_metric_hist_get_value`,
    `summary_quantile_exchange` directly; `cmt_metric_hist_inc`, `_set`,
    `cmt_metric_hist_bucket_inc` through the summaries they validate).
    `QCore.ResidualAccessReadings` (`address_vectors_hold`,
    `a_member_keeps_rule_10n`, `an_element_is_read_only_when_scalar`,
    `nothing_else_is_read`), bound to `DECISION_VECTORS`.

<a id="constraint-14d"></a>

14d. **An interior offset `P ± n` handed to a `Borrowed` or data-tail actual
    is read under the address's premise.** Card
    CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (`residual_access_reading.rs`,
    a thin hook in `typed_access_positions.rs::StatementIndex::argument`).
    `memcpy(cf->map + 2, …)`: the actual is read as rule 14b reads the
    offset, with `P` a pointer-valued path or a plain pointer variable the
    closure proves never bound, `n` a scalar value, and every root never bound
    (rule 10n(1)'s address premise). The callee is handed a pointer into the
    object `P` reaches, which it could be handed as `P`; W44's position
    premise and its second obligation (`ArgumentReads`, the value pointer
    valued) apply unchanged. **Do not** read a cast or call operand, a bound
    root or name, or a custody-taking position under this rule. Measured: 9
    actuals admitted that the base never read; **+2 Unknown → Safe**
    (`finalize_checksum`, `rd_segment_write_update`).
    `QCore.ResidualAccessReadings` (`offset_vectors_hold`,
    `a_bound_name_is_never_read`, `a_statement_reads_only_a_path_operand`),
    bound to `OFFSET_VECTORS`.

<a id="constraint-14e"></a>

14e. **A constructor's stated result state (`quant live #result: TAG.FIELD
    |-> #absent;`, `resource_discriminant_set`,
    `resource_callback_member_registered`) is read at a later call only
    while every store the run makes into the member is classified; the
    `#absent` state stands in a unit that defines the constructor only where
    its body zero-allocates the result and never writes the member.** Card
    OP-CONSTRUCTOR-DECLARATIONS (contract specification C04.19.4, grammar
    G05 and G05d), on the user's decisions of 2026-10-14 (A1, B1, C; the
    maps-to token `|->`, `qcore_frontend::lexer::MAPS_TO`).
    `qcore_schemas::constructed_state` (`ConstructedState`, the canonical
    parts of both levels), the loader `contract_constructed_state.rs` (the
    shorthand elaborates into `quant live #result;` and the member's
    `#absent` (card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the value keyword,
    its one spelling; a bare `absent` rejects the module naming `#absent`,
    `quant-live-absent`), and the two levels give one state; a bare `->` there rejects
    the module naming `|->`), `crates/qcore-c-gateway/src/constructed_state.rs`
    (`StateStore`, `collect`, `stability`, `result_state_validates`), route
    G's `legacy_constructed_route.rs`, the run fold in
    `qcore_audit::callback_member_fold` (`constructed_states`). Every store
    into a stated `TAG.FIELD` in every unit (a data-only unit read whole) is
    classified: into an object a local of the storing function holds that
    only direct calls of owned-result declarations assign (a constructor's
    own result; never another activation's object), or an automatic record;
    a null pointer constant into a function-pointer member (a candidate
    removed); a constant flag-or `P->F |= E`; a store `P->F = C` inside a
    case group of a `switch` reading the same `P->F` (`& E` allowed), `P`
    rebound only to a null constant in the function (a rewrite: the decided
    values gain `C`); a sub-object write touches only the members under it.
    Anything else, a whole-object write of the record (an assignment, a
    `memcpy`- or `memset`-family call) and a unit with no facts leave the
    state not kept (premise `P-OPCTOR-STORES-SEEN`: every unit that can store
    into the member is analysed and no store is type-punned past the
    record). A unit reads a state only where its own stores keep it and the
    calling function stores into the member nowhere; the fold demotes a
    `Safe` or `Pending` row resting on a constructor whose stated member the
    run does not keep, and the demoted function's null and bounds rows read
    `Unknown` custody (their combined verdict follows). Body check (evidence
    `14e`, rule 13i's `ClauseKind::MemberState`): one local returned on
    every `return`, assigned once from `calloc` or `rd_calloc`, used only as
    a member store's base, the return and inside `sizeof`, no store reaching
    a `#absent` member, a stated discriminant stored once from its
    parameter with its flags; otherwise the state is withdrawn in that unit
    (the result is tracked, the state undecided) and stays a declared
    assumption elsewhere (rules 13i-13k). **Do not** read a state across an
    unclassified store, a whole-object write, a non-null rebinding, or a
    store the calling function makes itself; validate `#absent` on a body
    that does not zero the object; or promote a row in the fold.
    `QCore.OpCallbacks` (`the_shorthand_and_its_parts_mean_the_same`,
    `the_shorthand_is_live_and_absent`, `the_shorthand_entails_quant_live`,
    `both_levels_are_one_canonical_set`,
    `an_unclassified_store_voids_the_absent_state`); `QCore.BodyClauseStanding`
    gains `memberState` and `r14e`; `tests/op_constructor_declarations.rs`,
    `crates/qcore-contract/tests/op_constructor_clause.rs`.

<a id="constraint-14f"></a>

14f. **A declared body a unit defines is checked against its declaration in
    every unit and published as evidence for the declaration, never as a
    verdict row: a header definition stays outside the denominator.** Card
    CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (contract specification C09.1
    in C06.5's shape; `qcore_core::bounds_body_standing`, the gateway module
    `index_extent/body_evidence.rs` with a thin hook in phase one of
    `analyze_bounds_preprocessed`, the audit module `bounds_evidence.rs` with
    thin hooks in `projection.rs` and `report.rs`; shared items added:
    `BoundsUnitReport::declaration_bodies`,
    `BoundsFunctionRow::contract_dependencies` (absent when empty),
    `AuditReport::bounds_evidence`, `index_extent::bounds_body_standing`
    (a re-export); no custody code reads any of it). (1) **Checked.** Phase
    one already analysed, in every unit, the body of each declaration with a
    frame, quant check, return extent or relation that the unit defines
    (`static inline` header definitions included) to withdraw a refuted one
    from the unit's callers; it now also analyses each selected declared
    main-source body, under the declarations as loaded (no refutation table,
    so a route's second phase finds the same evidence), and keeps the result.
    (2) **Coverage.** A body whose own row is `Safe` and does not refute
    covers its quant checks and preconditions (`bounds_extent`,
    `bounds_index`); a frame clause only by rule 14g's positive frame check of
    its parameter; a derived relation (its length is a fact about the
    caller's object), a return extent, `bounds_preserves_extents()` and a
    global extent by nothing. (3) **Standing.** Per unit: held (no evidence is
    kept under a review marker or a pin hold), failed (refuted), validated
    with its covered clauses, or unvalidated; per run, rules 13g–13j's fold:
    `validated` only when some unit examined a body, every examining unit
    validated it and every clause is covered in every one,
    `partly-validated`, `held`, otherwise a `declared-assumption`. (4)
    **Publication.** The bounds selection record (`ROWS.contracts.json`)
    gains `body_evidence` (every declaration of the environment, one no unit
    examined a declared assumption; each unit's outcome; per-clause standing)
    and a Safe roll-up of the bounds rows by the declarations each row's
    analysis could have consulted (its own name and every called declared
    name, an over-approximation), with a console tally. **Do not** publish a
    row for a header definition, keep evidence from a held declaration or a
    hypothetical analysis, cover a clause by evidence of another kind, or let
    any verdict read the section. Premises stated, not proved: faithful Clang
    extraction (trust boundary 28) and that a `Safe` row means every access
    through a quant-checked formal was decided inside (rule 12l(2)).
    `QCore.BoundsBodyStanding` (`a_refuting_body_covers_nothing`,
    `nothing_covers_these`, `body_evidence_needs_a_safe_body`,
    `frame_evidence_needs_the_frame_check`,
    `a_safe_body_alone_never_covers_a_frame`, `a_held_unit_keeps_no_evidence`,
    `validated_needs_every_unit_and_clause`, `a_failure_wins`,
    `partly_names_its_uncovered_clauses`,
    `only_validated_needs_every_dependency_validated`,
    `an_unknown_dependency_is_an_assumption` and 7 more), bound by
    `coverageVectors` and `standingVectors`
    (`crates/qcore-core/tests/bounds_body_standing.rs`); controls in
    `crates/qcore-c-gateway/tests/bounds_body_evidence.rs` and
    `crates/qcore-audit/tests/bounds_transport.rs`;
    [report](../reports/cg16-bounds-transport-and-chunkio-neighbours/README.md).

<a id="constraint-14g"></a>

14g. **Two narrow readings serve the body check: a positive frame check that
    reads the body's syntax only, and a byte view of a quant object.** Card
    CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (`index_extent/body_evidence.rs`,
    `index_extent/quant_checks.rs`; shared items added: `TuIndex::typedefs`
    and the `TypedefDecl` names and types the streaming reader now keeps).
    (1) **Frame.** A pointer or slot parameter's declared frame is confirmed
    when the parameter is never assigned, copied or has its address taken,
    and every occurrence is a read through it (`P[k]`, `*P`, `P->f`), a write
    through it when its mode is not `reads`, a comparison operand, an actual
    at a resolved, in-force closed frame position no stronger than its own and
    not a derived base of the callee, or (with a relation on it) `return P` or
    `return P ± k`; inline assembly or any other occurrence leaves it
    unconfirmed. Evidence only: no analysis reads it. (2) **Byte view.** A
    pointer conversion between C's byte types (`char`, `signed char`,
    `unsigned char`, typedefs through the unit's `TypedefDecl`s) of a pointer
    into a **quant object** keeps its target, and a pointer local assigned
    such a view keeps the fact: every byte type has size 1, so an index
    through the view is the same index into the object and the quant check
    decides it as `P + k` (chunkio's `(uint8_t *) &map[10]`). **Do not**
    follow a view of any other object, a view to a wider type, or confirm a
    frame through a copy the walk cannot follow. Controls:
    `a_byte_view_reads_only_byte_types_of_quant_objects` (an index through the
    view outside the check refutes; a four-byte view and a byte view of a
    local array keep their refusal), `controls_refute_or_leave_clauses_uncovered`.

<a id="constraint-14h"></a>

14h. **A bounds declaration whose body refutes it in one unit is withdrawn
    from every unit's callers by the route's second phase.** Card
    CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (C06.4's two phases for the
    bounds property; `refuted_contracts::run_two_phases_with_bounds`,
    `bounds_evidence::refutation_table`,
    `bounds_selection::apply_refutation_table`; hooks in `pinned_route.rs`,
    `current_route.rs` and `no_build.rs`; shared items added:
    `UnitProducts::bounds_evidence`, `EntryProducts::bounds_evidence`). The
    table is the union of every unit's refuting bodies (rule 14f's evidence,
    so independent of unit order and of the table applied); phase two runs
    when the custody table or the bounds table is not empty, with the bounds
    table recorded in the installed environment's `BoundsContracts::refuted`
    (the record names it as `refutation_table`), so every caller outside the
    defining body sees an uncontracted callee: the obligation is unresolved,
    never `Safe`, never an accusation. With both tables empty nothing runs
    twice, so a run without a refutation is byte-identical. **Do not**
    withdraw a declaration from its own body, read the table from a phase-two
    analysis, or run phase two on the custody table alone when the bounds
    table is not empty. `QCore.BoundsTransport`
    (`the_empty_table_changes_nothing`, `its_own_body_keeps_it`,
    `a_refuted_declaration_is_withdrawn`, `a_withdrawal_never_accuses`,
    `a_caller_of_a_refuted_declaration_is_never_safe`,
    `mem_bounds_table_iff`, `the_table_is_stable_across_phases`); control
    `a_body_refuting_its_declaration_withdraws_it_in_every_unit` (a caller in
    another unit moves Safe → Unknown).

<a id="constraint-14i"></a>

14i. **A bounds declaration may track the tip of a branch exactly as a
    resource declaration does (C06.2): in force only at the tip's sites, then
    a named assumption with its notice; otherwise held.** Card
    CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (`contract_pin.rs`:
    `pin --bounds --track-tip REF` and `--untrack-tip`, no longer refused;
    `bounds_selection::apply_bounds_pins` and `add_tip_tracking`). The
    decision is `pin_standing`'s, unchanged; the pin section's outcome reads
    `tip-in-force`, carries the tip record and its notice (`info` in force,
    `warning` not), and, only when some declaration tracks a tip, the section
    gains `tip_meaning`, `tip_counts`, `tip_notices` and
    `assumptions.tip_tracked` (`tip-tracked:FUNCTION`); the console prints
    one `bounds contract notice` line per tip. A run with no tip entry is
    byte-identical. **Do not** keep a tip entry in force on another site or
    an unresolved tip, or let a notice decide anything. `QCore.BoundsTransport`
    (`a_tip_is_in_force_iff`, `an_out_of_force_tip_is_held`,
    `a_tip_assumption_is_listed_iff_in_force`, `tip_vectors_hold`), bound by
    `tip_vectors_match_lean`; control
    `a_bounds_tip_entry_is_a_named_assumption_in_force_and_held_otherwise`.

<a id="constraint-14j"></a>

14j. **What does not transport bounds refuses rather than drops: a
    checkpointed run refuses the bounds selection, and a bounds module that
    includes or inherits never reaches a bounds view.** Card
    CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (`checkpoint_run::run`). A
    `.qo` checkpoint carries no bounds row, body evidence or bounds
    environment, and its identity names neither the environment nor its
    refutation table, so a reused or composed run would omit every requested
    row; `current --checkpoint-dir` with `QCORE_BOUNDS_PROPERTY` set refuses
    and writes no rows. G05a stays outside bounds contract v1: the bounds
    loader refuses `#include` and `contract inherit`, so the generated views
    leave such a module out and a load path that names one in a bounds view
    refuses the whole selection. Transport (sections in the checkpoint, the
    environment and table digests in its identity) and inclusion (the
    elaboration with a bounds agreement gate over renamed quant predicates and
    counts, and evidence replay) remain registered
    (`CG16-BOUNDS-CONTRACT-TRANSPORT`). **Do not** compose bounds rows from
    checkpoints or elaborate a bounds module without that gate.
    `QCore.BoundsTransport` (`a_composition_would_omit_the_requested_rows`,
    `a_bounds_run_is_never_composed_from_checkpoints`); controls
    `a_checkpointed_run_refuses_the_bounds_selection`,
    `an_inheriting_bounds_module_never_reaches_a_bounds_view`.

<a id="constraint-14k"></a>

14k. **Null dereference is a separately selected property, `c.null.deref.v1`:
    every evaluated dereference is a check, and a function is `Safe` only when
    every check is proved or moved to a check that is.** Card
    NULL-SAFETY-PROPERTY (USER DECISION 2026-10-10; contract specification
    C09.3; `null_deref.rs`, `null_deref/{ast,walk}.rs`,
    `qcore_core::null_deref`). The consumer reads Clang's typed AST (every
    definition compacted as it is read) and walks each body forward, joining
    paths. A check is every evaluated `*p`, `p->f`, `p[i]` read or write, an
    atomic access, and an actual at a `quant null-deref` position (rule 14o);
    its outcome is `proved`, `moved` (rule 14m), `held`, `candidate` (with its
    sources), `violation` or `excluded`. `Unsafe` only for a pointer null on
    every path dereferenced on the straight-line prefix from entry (no branch,
    loop, call or label before it); a violation elsewhere is
    `null-violation-reachability-unresolved` (`Unknown`); a held clause the
    sole obstacle is `Pending`; a form the walk does not read
    (`null-unsupported-form: KIND`), a loop without a fixed point
    (`null-loop-unconverged`) or a missing definition (`null-import-failure`)
    is `Unknown`. The audit selects it with `QCORE_NULL_PROPERTY` (rows as JSON
    lines, `.summary.json` beside them; a checkpointed run refuses it) and
    publishes the custody status and the join of a request naming both
    (`compose`); the custody report is byte-identical with or without the
    selection. **Do not** publish a candidate as a defect, or read a row
    `Safe` while any check is a candidate. `QCore.NullDeref`
    (`check_sound`, `no_safe_on_an_undecided_pointer`,
    `unsafe_only_for_a_reachable_violation`, `combined_safe_needs_null_safe`;
    `checkVectors`, `verdictVectors`).

<a id="constraint-14l"></a>

14l. **Sources and guards of the null property.** Card NULL-SAFETY-PROPERTY.
    Possibly null: an owned or reallocated *nullable* contract result
    (`allocation`), a derived nullable result (`lookup`), a pointer result no
    contract states (`uncontracted-result`), a formal with no clause
    (`formal`), a pointer read from memory or a global (`memory-field`,
    `global`), a null constant on some path (`null-literal`), an uninitialized
    local. Non-null: an address, an array decay, a string literal, a function,
    a non-null owned contract result (a listed `declared:` assumption), a
    non-null in-unit summary result (rule 14p), `errno`'s address (C11 7.5p2).
    A NULL test (`if (p)`, `!p`, `p != NULL`, `p == NULL`, `&&`, `||`, `?:`,
    `__builtin_expect`) proves its non-null arm and makes the other null; an
    arm that ends in a `return`, `break`, `goto` or a no-return call does not
    reach the join. `assert(p)` is a guard only where the analysed text still
    holds it (`NDEBUG` absent there), and a proof resting on it lists the
    premise `asserted:` (the deployed build keeps the assertion). **Do not**
    read a guard the analysed text does not hold, or a non-null result no
    contract or in-unit body states. `QCore.NullDeref` (`guard_sound`,
    `walk_sound`, `join_sound_left`).

<a id="constraint-14m"></a>

14m. **Decision 1's dominating dereference moves the NULL question; a
    reassignment ends it.** Card NULL-SAFETY-PROPERTY. After a real read or
    write through `B` (check `c`), `B` is non-null with its question moved to
    `c` (`moved`, proofs `moved:c`), and so is `&B->F`; until `B` is assigned
    again. `B ± n` is non-null exactly when `B` is (C11 6.5.6p8), so an offset
    keeps the moved question. Forming `&B->G`, `&B[i]`, `&*E` or an array
    member's decay, and an unevaluated operand (`sizeof`, `_Alignof`,
    `typeof`, a trait, an unselected `_Generic` association) are not
    dereferences. A local or formal whose address is never taken keeps its
    fact across calls; a memory path (`s->next`, a global, an address-taken
    local) loses its fact at every call and every store that could write a
    pointer, and keeps it across a non-character scalar store only under the
    listed premise `effective-type:` (C11 6.5p7). Loops reach a fixed point
    (16 rounds, else `Unknown`); a label a later `goto` reaches forgets every
    fact. **Do not** move a question across a reassignment, a call (for a
    memory path) or a label. `QCore.NullDeref`
    (`moved_question_is_the_origins`, `nullness_kept_without_assignment`,
    `afterDereference_sound`, `offset_sound`).

<a id="constraint-14n"></a>

14n. **Custody reads a dereference through a target-free, possibly-null
    borrowed alias non-null only under a named premise.** Card
    NULL-SAFETY-PROPERTY (decision 1; resolves
    `CG10-INTERIOR-ALIAS-NULLABILITY`; `legacy_null_premise.rs`,
    `null_premise.rs`). In the typed scalar access's base and chain
    obligations (W43-INDIRECT-ACCESS-GENERAL, W44-INDIRECT-CHAIN-ACCESS), a
    binding with no target, `unknown`, possibly null, of the identity
    derivation, in an unwidened state (what a pointer-valued path read and
    rule 14b bind) is read non-null at this dereference, and every other part
    of the obligation is decided as for a non-null binding (a bound chain root
    still needs rule 11d's premise). The reading records the premise
    `null-deref:FUNCTION:LINE:VARIABLE` on the summary (`null_premises`, kept
    by the analysis that relied on it), on the report row, in the row's
    `contract_dependencies.null_premises` (its own and, through the
    consultation log, every in-unit summary's) and in the Safe roll-up
    (`resting_only_on_a_null_premise`, `carrying_a_null_premise`, never
    counted as resting on nothing). With the null property selected, each
    custody row's premises carry what the null row says of the variable's
    checks at that line (`proved` only on that line; `nearby:` within three
    lines, never counted proved). The binding stays possibly null for every
    other rule (a release, an escape or a store through it decides as
    before); a tracked, derived, mixed, widened or known binding is never
    premised. **Do not** read a possibly-null binding non-null without
    recording its premise. `QCore.NullDeref` (`a_relied_on_reading_is_listed`,
    `a_tracked_binding_is_never_premised`,
    `a_known_or_widened_binding_is_never_premised`; `premisedVectors`).

<a id="constraint-14o"></a>

14o. **`quant null-deref #B;` is a bounds contract v1 check: a caller
    obligation and a body check, never read by the bounds consumer.** Card
    NULL-SAFETY-PROPERTY (decision 2; grammar G05d). The callee's `B` is
    non-null at entry and its row is conditional on the clause; each caller's
    actual is a `call-precondition` check (a possibly-null actual is a
    candidate at the call), after which the actual's question is that check's.
    A held declaration (review marker or pin not in force) is `held` at the
    call and moves nothing; a declaration whose parameters do not match is
    withdrawn (`Unknown`). The body is checked against its own clause:
    `necessary` when every terminating path dereferences `B` before any test or
    assignment of it, else `stronger-than-necessary`, never wrong. The bounds
    consumer reads `BoundsContracts::for_bounds()` (no null-deref clause; a
    declaration or module left empty leaves the bounds identity), so no bounds
    row moves. **Do not** admit the clause in resource contract v1 (it would
    change custody's environment digest for a clause custody never reads).
    A function declared both in a resource view and in a `null_deref.qv`
    module is a registered checked relation `null_deref_beside` (contract
    views: signature agreement, and the bounds declaration states only the
    selector and null-deref checks).
    `QCore.NullDeref` (`caller_obligation_establishes_the_entry`).

<a id="constraint-14p"></a>

14p. **In-unit inference of the clause and of non-null results; neither
    leaves the unit.** Card NULL-SAFETY-PROPERTY. Bodies are analysed callees
    first (a recursive edge is cut: no clause, no result). A formal that every
    terminating path dereferences (directly, through a copy or an offset, or
    by handing it to a clause position) before any NULL test or assignment of
    it infers `quant null-deref #B;`: the body's row is `Safe` conditional on it
    (`inferred_preconditions`, the scope names it) and every in-unit caller
    carries the obligation; a body whose every return is non-null gives its
    in-unit callers a non-null result. A caller in another unit sees neither
    unless the clause is declared (rule 14o). **Do not** carry an inferred
    clause or result across a unit boundary. `QCore.NullDeref`
    (`an_inferred_clause_is_necessary`; `firstVectors`, `inferredVectors`).

<a id="constraint-14q"></a>

14q. **A dereference contract has two levels and one canonical form: the
    shorthand `quant deref #B;` is elaborated into its parts before any
    consumer reads it.** Card NULL-DEREF-QTT-GROUNDING (USER DECISION
    2026-10-11, "Two levels"; USER DIRECTION 2026-10-11, "This can be derived
    from the proven lean. And it can be natural."; contract specification
    C09.3, grammar G05d; `deref_parts.rs`). The bounds contract v1 loader
    elaborates `quant deref #B;` into `quant null-deref #B;`, `quant checks
    B, Idx #i: #i == 0;` and `quant live #B;`, merges a written part that
    agrees with a shorthand's and refuses one that does not
    (`quant-deref-conflict`); the parser refuses an unknown `quant WORD #B;`
    (`quant-shorthand-unknown`). Consumers, standing and dependency sets see
    only canonical parts: the bounds consumer the extent part (`for_bounds`
    removes the existence and liveness parts), the null property the
    existence part, and custody none (resource v1 refuses both new forms).
    An audit route admits a liveness part only where the function's resource
    v1 declaration in force puts `B` at a `resource_owned` position, and
    refuses the route otherwise (`quant-live-unbacked`,
    `quant-live-contradicts-custody`). The pin stays over the text as written;
    `canonical_sha256` says when two texts mean the same, published in the
    bounds selection record only when some declaration states a liveness
    part. `quant null-deref #B;` keeps its meaning (existence alone). **Do
    not** let a consumer read a shorthand, read a liveness part custody's own
    declaration does not state, or use the derived `checks(B, #i == 0) ⊢
    null-deref(B)` to discharge a NULL obligation (that is the registered
    `NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`). `QCore.DerefDecomposition`
    (`elaboration_preserves_meaning`,
    `merging_an_agreeing_overlap_is_idempotent`,
    `checks_zero_entails_null_deref`,
    `existence_and_extent_do_not_discharge_liveness`); grounding
    `QCore.NullDerefGrade`.

<a id="constraint-14r"></a>

14r. **A strong-source NULL-dereference candidate is a finding only when a
    path search validates a path to it with the pointer NULL under a named
    condition.** Card NULL-CANDIDATE-TRIAGE (contract specification C09.3;
    `null_deref/witness.rs`, `qcore_core::null_triage`). A candidate whose
    sources include an allocation, a nullable or lookup result, a null
    constant, an uninitialized local or a global is searched from the
    function's entry, one path at a time (at most 1,024 paths, 40,000 nodes
    each, 64 iterations a loop). A branch is taken only when the search
    decides its condition (a constant, an enumeration constant, a NULL test
    of a pointer the path fixed, an interval of a value narrowed by earlier
    comparisons with constants, a repeated pointer comparison) and agrees,
    or when every value it reads is unconstrained on the path (a formal, a
    call's result, a memory value read since the last call or store); a
    condition over a value an earlier decision related (a local assigned
    from an expression, another value it was compared with, a memory
    spelling decided before a call and read again after it) is correlated
    and the path is unresolved (`admit`). The NULL condition is named: an
    allocation that fails, a lookup that misses (only when rule 39 admits the
    miss on the path), a null constant, the null arm of the function's own
    test. Outcome per candidate (`triage` on the
    row): `finding` (with its decisions, its NULL event and the
    dereference), `infeasible` (every path explored, none reaches it NULL
    under a named condition), `unresolved` (with the reason) or
    `indeterminate` (an uninitialized value reaches it: not a NULL finding).
    **Do not** take a correlated branch, name a formal's or a memory
    value's possible null-ness as a NULL condition without the function's
    own test, or present a finding as an upstream defect.
    `QCore.NullTriage` (`a_validated_path_is_satisfiable`,
    `a_validated_finding_faults`, `a_correlated_condition_is_never_taken`,
    `interval_path_is_satisfiable`; `admitVectors`).

<a id="constraint-14s"></a>

14s. **A finding makes its row provisional `Unsafe`, labelled by class; it
    is never a `Safe` and never an upstream defect.** Card
    NULL-CANDIDATE-TRIAGE. The check's outcome is `finding`, the row's
    verdict the join of its checks' verdict with `Unsafe`
    (`triaged_verdict`), its reason `null-deref-finding` (provisional). The
    class is `null-on-allocation-failure` when the path needs an allocation
    to fail (the dereferenced value's, or any other the path tested NULL),
    otherwise `null-on-normal-path`. Each finding is published with its
    file, line, function, path, condition and source excerpts from the
    pinned revision (`docs/reports/null-candidate-triage/artifacts/findings.json`).
    **Do not** count a finding as anything but provisional until a person
    reviews it. `QCore.NullTriage` (`triage_never_makes_safe`,
    `unsafe_only_for_a_violation_or_a_finding`).

<a id="constraint-14t"></a>

14t. **A declared non-null member is read non-null and checked at every
    store the analysed units make; a store not proved non-null withdraws it
    for the run.** Card NULL-CANDIDATE-TRIAGE (grammar G05e; spelled
    `null_member_nonnull` until NULL-MEMBER-INVARIANT-SCOPE's rename, C09.3
    "Naming", no alias).
    `requires nonnull_member(TAG, FIELD);` in bounds contract v1 (the plain
    kind of the one member clause, grammar G05f, since card
    NONNULL-MEMBER-UNIFIED-QUALIFIER; a
    type-level declaration its host function, the record's constructor,
    carries; pinned in the crate's `bounds-contracts.lock`): a read of
    `E->FIELD` or `E.FIELD` of a `struct TAG` (through typedefs) with no fact
    of its own on the path is non-null (proof `result:member:TAG.FIELD`, the
    assumption `declared-member:`, the row's dependency `member:TAG.FIELD`,
    carried through in-unit summaries). Every assignment, compound update and
    initializer list (an omitted member is a null store) to the member is a
    store check in `member_stores`; the run's fold (rule 14w) withdraws the
    declaration when one is not discharged. Its standing is never
    `validated`: the clause `construction` (a zero-initialised or
    indeterminate object's member is never read before its first store) is a
    premise no store covers; `partly-validated` when every examined store is
    proved, `declared-assumption` when no analysed unit stores it.
    The check covers the analysed units' stores only, so the declaration may
    name a premise (card NONNULL-MEMBER-UNIFIED-QUALIFIER, USER DECISION
    2026-10-16 "Yes, all kinds"; rule 14x's premised facts): every null row
    whose dependency set names `member:TAG.FIELD` lists it, and so does the
    fold's `member_declarations` record. librdkafka's four plain
    declarations (`rd_kafka_toppar_s.rktp_rkt`, `rd_kafka_topic_s.rkt_rk`
    and `.rkt_topic`, `rd_kafka_cgrp_s.rkcg_rk`,
    `rd_kafka_transport_s.rktrans_rkb`) name `rk_plugin_callback_closure` by the
    census rule (`docs/reports/nonnull-member-unified-qualifier/`).
    **Do not** read a held or withdrawn member declaration, or label one
    `validated`. `QCore.NullTriage` (`member_invariant_holds`,
    `a_member_declaration_is_never_validated`, `a_failed_store_withdraws`;
    `memberVectors`).

<a id="constraint-14u"></a>

14u. **A declared non-null result is checked against every analysed body
    that defines the function.** Card NULL-CANDIDATE-TRIAGE (grammar G05e;
    spelled `null_result_nonnull` until the same rename).
    `ensures normal nonnull_result();` (bounds contract v1, a pointer
    result): every caller reads the result non-null (proof
    `result:declared-result:NAME`, dependency `result:NAME`). Each unit that
    analyses a body is evidence (`validated` when every return is non-null,
    `refuted` when a return is null on a path, else `unvalidated`); rule
    13g's fold gives `validated`, `withdrawn` (a refutation) or
    `declared-assumption`. **Do not** read a withdrawn result non-null.
    `QCore.NullTriage` (`a_refuting_unit_withdraws`,
    `validated_needs_a_validating_body`; `resultVectors`).

<a id="constraint-14v"></a>

14v. **An inferred clause and a non-null result travel to callers in other
    units only from an external definition analysed in exactly one unit.**
    Card NULL-CANDIDATE-TRIAGE (completes rule 14p across units). Each unit
    exports every analysed definition (`definitions`: linkage, parameter
    count, inferred clause positions, whether every return is non-null and
    the declarations its summary rests on) and records each pointer actual
    of a call to a function it neither defines nor declares
    (`external_calls`, judged as a check would be). After every unit, a
    caller in another unit gains a `call-precondition` check at each actual
    in an exported clause position (same parameter count), and a candidate
    whose every source is the untested result of exported callees whose
    every return is non-null (no summary resting on a withdrawn declaration)
    is proved (`result:cross-unit:NAME`). A name two units define, a static
    or `inline` definition, and a mismatched arity export nothing. Travel
    never re-analyses a caller: it adds checks (never improving a row) and
    proves only sole-source result candidates. **Do not** carry a summary
    from an ambiguous or internal definition. `QCore.NullTriage`
    (`travel_never_improves_a_row`).

<a id="constraint-14w"></a>

14w. **The null property's run fold: standings, withdrawals and travel,
    after every unit and before the rows are written; custody never reads
    it.** Card NULL-CANDIDATE-TRIAGE (`qcore_audit::null_fold`). The audit
    keeps each unit's null evidence (`record_null_evidence`), folds it in
    `write_null_selection`: member and result standings (rules 14t, 14u),
    the withdrawn set, travel (rule 14v), then every changed row's verdict
    recomputed from its checks (`triaged_verdict`, `demote`: a row whose
    dependency set names a withdrawn declaration is never `Safe` or
    `Pending`, and a check a withdrawn declaration proved is a candidate
    again, source `withdrawn:`), its custody premises' standings and its
    combined verdict. The summary's `fold` section publishes all of it.
    **Do not** let a withdrawn declaration leave a row `Safe`, or let the
    fold change a custody byte. `QCore.NullTriage`
    (`a_withdrawn_dependency_is_never_safe`).

<a id="constraint-14x"></a>

14x. **A stable member is checked at every store the analysed units make
    to an existing object; an initializer list writes a fresh object and
    is exempt; one store not proved withdraws it, and so does a proof
    resting on a withdrawn declaration.** Card NULL-MEMBER-INVARIANT-SCOPE
    (contract specification C09.3, grammar G05f; `null_deref/scope.rs`,
    `walk.rs` (`stable_store`), `qcore_audit::null_member_scope_fold`).
    `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];` in
    bounds contract v1 (spelled `nonnull_member_stable(TAG, FIELD)` until
    card NONNULL-MEMBER-UNIFIED-QUALIFIER, which merged the three member
    forms into one clause, rejects the old word naming the new form and
    keeps the fact unchanged, `QCore.NullMemberQualifier`; a
    type-level declaration its host, the record's constructor, carries;
    pinned in the crate's `bounds-contracts.lock`) says `FIELD` is never
    made NULL on an existing `struct TAG` object. Every assignment and
    compound update of `E->FIELD` or `E.FIELD` (through typedefs) is an
    examined store (`stable_stores`, kind `assignment`); a struct
    assignment of a `struct TAG` lvalue, and a `memset`, `memcpy`,
    `memmove`, `bzero` or `explicit_bzero` (or builtin) whose destination,
    casts stripped, is typed `struct TAG *`, is examined and never proved
    (`whole-object`); an initializer list of a declared object or a
    compound literal is counted (`initializer`) and exempt, since it writes
    an object no use held before it. Each store records the declarations
    its value's proof rests on (`relies`: stable members, rule 14t's
    members, rule 14u's results). The run fold withdraws the declaration
    when an examined store is not proved or rests on a withdrawn
    declaration, iterating from the refuted results to the least closed
    set (`withdrawals`; rule 14t's members cascade the same way), and
    publishes `stable_declarations` (and `scope_declarations`) in the
    summary's `fold` section only when some are declared. The standing is
    `partly-validated` at best: a whole-object write through an untyped
    pointer is the premise `whole-object`. A member with an exit store
    (NULL stored into an existing object: `rd_kafka_buf_s.rkbuf_rkb`'s
    reset in `rd_kafka_recv`, `mk_list_del`, `cfl_list_del`) has no stable
    reading and is not declared. An allocator's zeroed object (`calloc`,
    `flb_calloc`) is no store at all: a fresh object starts outside the
    stable state, so its NULL member is never read non-null, and only a
    non-null fact the walk sees persists (card
    FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: Fluent Bit's plugin descriptors
    `flb_{input,output,filter,processor}_instance.p`, hosted on the four
    constructors, each `flb_calloc`ed and stored once with a plugin the
    constructor dereferenced; every examined store proved on the routes
    that analyse it). **Do not** exempt a store to an existing
    object, count a store proved only by a withdrawn declaration as
    proved, or label a stable declaration `validated`.
    **Premised facts** (card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER, USER
    DECISION 2026-10-15 "Only nonnull_member* need it"; contract
    specification C04.18, C09.3). The check covers the analysed units' stores
    only. A declaration stating a member clause of any kind (this rule's,
    rule 14z's, and since card NONNULL-MEMBER-UNIFIED-QUALIFIER, USER
    DECISION 2026-10-16 "Yes, all kinds", rule 14t's plain kind) may
    therefore name `requires resource_premise(NAME, "STATEMENT");`; in
    bounds v1 nowhere else (`premise-beside-member`). The facts are then in
    force only under it. Every null row whose dependency set names
    `stable:TAG.FIELD`, `member:TAG.FIELD` (or a premised scope function),
    including a set the fold's travel grows
    (`null_fold::add_dependencies`), lists it in `premises` and as
    `premise:NAME`. The fold's records list it. No decision reads it. A
    `quant null-deref` fact takes none. librdkafka's 67
    `rd_kafka_broker_s.rkb_rk` declarations name `rk_plugin_callback_closure`:
    a store to the member is reachable from a function a plugin's callback
    may call (`docs/reports/librdkafka-plugin-premise-remainder/`).
    **Do not** list a premise on a row that rests on no premised fact, or
    let one change a verdict. `QCore.NullMemberPremise`
    (`a_resting_row_lists_the_premise`,
    `a_listed_premise_comes_from_a_named_fact`,
    `listed_premises_put_every_fact_in_force`, `listing_changes_no_verdict`,
    `more_dependencies_never_drop_a_premise`).
    `QCore.NullMemberScope` (`stable_fact_persists`,
    `a_fresh_store_does_not_reach`, `an_exit_store_breaks_persistence`,
    `a_stable_declaration_is_never_validated`, `a_failed_store_withdraws_it`,
    `failure_is_monotone`; `stableVectors`).

<a id="constraint-14y"></a>

14y. **A stable member's non-null fact on `V->FIELD`, `V` a tracked local
    or formal, survives calls and stores until `V` is assigned; nothing
    else does.** Card NULL-MEMBER-INVARIANT-SCOPE (`walk.rs`:
    `memory_written`, `stable_member_of_key`; `qcore_core::null_member_scope::persist`).
    At a call, an atomic operation or a store that could write a pointer,
    the walk keeps exactly the path facts of the shape `V->FIELD` (one
    dereference of a tracked variable, whose value only an assignment the
    walk sees can change) whose `V` points to a `struct TAG` with `FIELD`
    stable in force and whose value is non-null, adding the proof
    `stable:TAG.FIELD` (`result:stable:TAG.FIELD` on a check). A check,
    a returned value or a stored value relying on it lists the assumption
    and the row dependency `stable:TAG.FIELD`; a withdrawn declaration
    makes each such check a candidate again (`withdrawn:stable:…`) and the
    row never `Safe` (rule 14w). The reading holds on every thread and
    through every callee because every store to an existing object is
    checked (rule 14x). A null or possibly-null fact, a fact on a memory
    base (`(*pp)->f`, `a->b->f`) or an untracked base, and every fact on
    `V` once it is assigned are forgotten as before. **Do not** keep a fact
    that is not non-null, or one whose base pointer the walk does not
    track. `QCore.NullMemberScope` (`stable_fact_persists`,
    `a_kept_fact_is_non_null`, `nothing_is_kept_without_stability`;
    `persistVectors`).

<a id="constraint-14z"></a>

14z. **A function-scope entry is the callee's entry fact and a checked
    obligation at every call.** Card NULL-MEMBER-INVARIANT-SCOPE (`walk.rs`:
    the entry facts, `scope_obligation`; `null_deref.rs`, the row's scope).
    `requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];`
    (spelled `nonnull_member_scope(TAG, FIELD, P)` until card
    NONNULL-MEMBER-UNIFIED-QUALIFIER; `P` is required with `[[scope]]` and
    rejected otherwise; the evidence labels below keep the old kind name,
    byte for byte) on a function's bounds
    declaration (`P` a pointer parameter) says that on entry `P->FIELD` is
    non-null. The body reads it so (proof `precondition:declared:
    nonnull_member_scope(…)`, held while the declaration is), its row is `Safe`
    only conditional on it (the scope names it) and depends on the
    function's declaration; with `FIELD` stable the fact lasts the whole
    body (rule 14y), otherwise until the first call or pointer store. At
    each call, the actual's `FIELD` is a `call-precondition` check read
    from the caller's facts (a tracked actual's path, a memory path's own;
    any other actual is a memory-field candidate), subject `ACTUAL->FIELD
    at `F` nonnull_member_scope(…)`; a held declaration's check is `held`; a
    declaration whose parameters do not match the call or the definition
    is withdrawn (`Unknown`). A call through a function pointer is not
    checked (the registered indirect-call residual), so an address-taken
    scope function's row stays conditional, as for `quant null-deref`.
    A check is discharged only by a fact on the actual's own path: a copy
    tested before the call (`p = ins->p; if (!p) continue;`) leaves
    `ins->p` a candidate at the call. Entries are chosen from a stable
    reading's run, never to hide a caller's question: a function whose
    remaining candidates are all `V->FIELD` of one formal, with external
    linkage and its address taken nowhere (card
    FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: ten `nonnull_member_scope(
    flb_input_instance, p, ins)` entries; their unproved caller checks are
    named corrections).
    **Do not** read an entry fact the declaration does not state, or drop
    a caller's check because the callee's member is stable. A scope
    declaration may name a premise (rule 14x's premised facts): the
    function's own row and every caller's row that depends on it list it.
    `QCore.NullMemberScope` (`a_scoped_use_sees_non_null`);
    `QCore.NullMemberPremise`.

<a id="constraint-15q"></a>

15q. **A call's integer result is bounded, never free.** Card
    NULL-TRIAGE-UNRESOLVED-PATHS (contract specification C09.3, grammar G05e;
    `null_deref/witness.rs`, `null_deref/range.rs`). In the path search
    (rule 14r) a non-pointer call result is a value the callee chooses: its
    range is a **bound**, the C type's range (an unsigned or `size_t` result
    is never negative), narrowed by a declared `ensures normal
    nonnull_result_range(LO, HI);` in force (not held, not refuted by this
    unit's own body: `range:NAME` joins the row's dependencies) and by this
    unit's validated summary of the callee's body (every `return` a constant
    or a local signed counter that starts at constants and only grows by
    non-negative constants). A comparison the bound decides takes its one arm;
    any other comparison over a call result is inexact (rule 15r). A declared
    range is checked against every analysed body under rule 13g's fold
    (`validated` when the body's summary lies inside it, `withdrawn` when a
    `return` gives a constant outside it, otherwise a declared assumption;
    the fold's `result_range_declarations`); a withdrawn range makes every
    check it proved a candidate again (`result:declared-range:NAME`).
    **Do not** read a call's result as any value of its type, or a declared
    range as values the callee returns. `QCore.NullTriagePaths`
    (`a_call_result_never_contributes`,
    `a_witness_holds_whatever_the_callees_return`,
    `a_bound_excludes_its_outside_arm`, `counter_stays_above`).

<a id="constraint-15r"></a>

15r. **A condition the search cannot read exactly is explored on both arms
    and never makes a witness.** Card NULL-TRIAGE-UNRESOLVED-PATHS
    (`qcore_core::null_paths::explore`). A call result its bound does not
    decide, a correlated value or pointer test (rule 14r), a value the search
    does not follow (an unfollowed expression, a floating value, a mask of
    several bits, a value of unknown range), an ordered pointer comparison
    and a switch over such a value end a path of the search's first, exact
    phase (rule 14r's reading, so its witnesses are found in the same order
    and budget as before). When that phase neither validates a path nor
    ends every path without reaching the dereference NULL, a second phase
    explores both arms (each case) of every such condition, marks the path
    inexact with the first reason, and draws only an infeasibility proof
    from it (rule 15t): a path so marked that reaches the dereference NULL
    (or reaches it uninitialised) leaves the candidate `unresolved` ("an
    inexact path reaches the dereference NULL: …"). Only an exact path
    (decided arms and arms over unconstrained entry values) is a finding or
    `indeterminate` (`conclude`). **Do not** take an inexact arm as
    evidence that an execution takes it. `QCore.NullTriagePaths`
    (`an_inexact_arm_is_never_exact`, `a_finding_needs_an_exact_path`).

<a id="constraint-15s"></a>

15s. **Relational facts are followed only when exact.** Card
    NULL-TRIAGE-UNRESOLVED-PATHS (`qcore_core::null_paths::relation_arms`).
    `x + k` and `x - k` of a followed value are followed as an offset only
    when no value of its range wraps or overflows in the expression's type
    (also through a conversion that holds every value); `a + b` of two atoms
    only when the sum interval lies in the type. A comparison of two
    independent, unconstrained entry values over hole-free intervals (or of
    an offset with another, or of a sum with a constant) takes an arm only
    when the difference (sum) interval meets the arm's set; any other
    relation is inexact. Before this card a comparison of two values took
    both arms whatever their ranges (a widening, corrected here). Both values
    are constrained after the decision (a later condition over either is
    correlated). **Do not** follow a call's result, a value with excluded
    points or a floating value in a relation. `QCore.NullTriagePaths`
    (`difference_attains`, `sum_attains`, `lt_arm_is_attained`,
    `an_unmet_arm_has_no_value`; `armsVectors`).

<a id="constraint-15t"></a>

15t. **A proved-infeasible candidate is removed, resting on everything the
    search read.** Card NULL-TRIAGE-UNRESOLVED-PATHS
    (`qcore_core::null_paths::{conclude, removes}`, `null_deref.rs`). A
    candidate is `infeasible` only when every path was explored to its end
    (no loop bound, budget, unread form, uninitialised test or label the
    search cannot enter cut one) and none, exact or inexact, reached the
    dereference NULL. Its check becomes `proved` with the proof
    `path-search:infeasible`, plus `result:member:`, `result:declared-result:`,
    `result:declared-range:` and `result:declared:` proofs for the
    declarations the explored paths read (all of them also join the row's
    dependency set), and the row lists the assumption `path-search:` (the
    search's facts are a review premise, trust boundary 28). A withdrawal
    makes the check a candidate again (rule 14w). The loop budget (64
    iterations), the path budget (1,024) and the step budget (40,000) are
    unchanged (rule 11x): their exhaustion leaves the candidate unresolved.
    **Do not** remove a candidate on a search that was cut. `QCore.NullTriagePaths`
    (`exhaustive_search_is_sound`, `infeasible_needs_a_complete_empty_search`).

<a id="constraint-15u"></a>

15u. **A cached memory value dies with the variable it spells.** Card
    NULL-TRIAGE-UNRESOLVED-PATHS (`witness.rs`, `forget_mentions`). The path
    search caches a memory value by its spelling until a call or a store;
    an assignment or a declaration of a followed variable now also forgets
    every cached spelling that names it (before this card `p->n` kept its
    value across `p = q`, which could decide a test wrongly in either
    direction). **Do not** reuse a cached memory value after a variable it
    names was assigned. Premise (trust boundary 28): the search's memory
    facts, so forgotten, hold for the C program.

<a id="constraint-15w"></a>

15w. **A callback formal handed on unchanged to the same clause position of
    a callee whose declaration is in force satisfies the caller's own
    declaration for that position, as rule 12s forwards dispositions; any
    deviation keeps rule 13y's withdrawal.** Card
    SERVE-FORWARDED-CALLBACK-FORMALS, on the user's decision of 2026-10-14
    (no forwarded-formal rule in the CG22 card; this card carries it).
    `crates/qcore-c-gateway/src/legacy_callback_forward.rs` (`forwards`,
    `stated_invocation`, `corresponds`, `forward_validates`,
    `Parser::forwarded_invocation`), hooks in `legacy_callback_route.rs`
    (`settle`, `Parser::callback_operands`); shared items added:
    `callback_route::{parameter_names, rebound}` become `pub(super)`. It
    amends rules 13u and 13y. (1) **The forward.** A mention of a callback
    formal `F` is a forward when it is the whole actual, at position `p`, of
    a direct call `G(…)` (`G` an identifier, not reached through `.` or
    `->`, not a keyword). The callee's declaration **in force** must name
    `p` in exactly one clause, `resource_callback_invoked(p, A'…)`, keep it
    `resource_borrowed`, never retain it and register it in no constructed
    result (`stated_invocation`); and `A'…`, read through the call's
    actuals, must be the caller's `A…` position by position: a caller
    parameter word `k` against a callee parameter word `k'` whose actual is
    exactly the caller's formal `k`, never rebound; `_` against `_`;
    `supplied_owned` against `supplied_owned` (`corresponds`). (2) **The
    body check (13y's forward case).** `resource_callback_invoked(F, A…)`
    stands in a unit that defines its body when either rule 13y's direct
    check holds or `F` is never rebound and **every** mention of `F` is
    such a forward (`forward_validates`). A callee whose body the unit
    defines is in force only once its own clauses stand, so the standing
    set is the least fixed point of the check (`settle`): a declaration
    whose only support would be itself (a recursive forward) never stands,
    and nothing stands on a callee the unit withdrew. A declaration standing
    by a forward records no body evidence (its clause rests on the callee's
    declaration and stays a declared assumption, rule 13i); `13y` is
    recorded only where the direct check holds alone. (3) **The forwarding
    call (13u's forward case).** At a call whose actual at the callee's
    stated position is the calling function's own formal `F`, where the
    calling function's declaration is in force in the unit (so its body
    validated) and every clause of it naming `F` is an invocation
    corresponding to the callee's, the clause is that declared invocation,
    decided at the calling function's own callers (rule 13u there), so this
    call neither resolves `F` nor puts `F`'s candidates' requirements on it.
    **Do not** read a forward through a reassignment, an address-of, a
    store, a cast, a parenthesised use, an operand, a local invocation
    beside a forward, another position, an actual that is not the caller's
    own unchanged formal, a callee with no declaration in force there
    (uncontracted, withdrawn, or the declaration itself), or a callee that
    names the position otherwise; nor record body evidence for a forwarded
    clause. Premises stated, not proved: faithful extraction (trust boundary
    28) and the callee's declaration (constraint 4). `QCore.CallbackForward`
    (`forwarded_invocations_meet_the_caller_clause`,
    `an_exact_forward_stands_iff_the_callee_clause_is_in_force`,
    `a_forward_to_a_callee_not_in_force_refuses`,
    `a_forward_that_does_not_correspond_refuses`, `a_changed_formal_refuses`,
    `any_other_mention_refuses`, `no_mention_refuses`,
    `a_recursive_forward_alone_never_stands`, `forward_vectors_hold` bound to
    the module's `FORWARD_VECTORS`); `tests/serve_forwarded_callback_formals.rs`.
    Measured (corpus `66910c10`, system and tail-pause slice, base main
    `2afce9973`, binary `087147af…`): corpus **936 / 9 / 3,889**, the report
    byte-identical (`ec4ccfeb…`), system 93 / 4 / 351 and slice 52 / 1 /
    347 / 4 Pending byte-identical, the null property's rows identical on
    all three; no forward stands in the corpus: the four served-queue
    forwarders (`rd_kafka_q_serve`, `_serve_maybe_consume`,
    `rd_kafka_q_pop_serve`, `_pop_serve_maybe_consume`) are exact forwards,
    but their callees `rd_kafka_q_serve0` and `rd_kafka_q_pop_serve0` are
    withdrawn in `rdkafka_queue.c` (each hands `callback` to the undeclared
    `rd_kafka_op_handle` and to itself), so no forwarder's callee is in force
    (`SERVE0-CALLBACK-HAND-ON-WITHDRAWN`). Rebased onto `89907068d`
    (OP-CALLBACK-CANDIDATE-CONTRACTS' candidates in force): corpus `0a38c3b3…`,
    system and slice byte-identical against main's own binary.
    [report](../reports/serve-forwarded-callback-formals/README.md).

<a id="constraint-15x"></a>

15x. **A declined classification probe hands back every field it moved.**
    Card GAPO-RETURN-CORRECTNESS (`legacy_probe_state.rs`: `ParserProbeState`,
    `Parser::{probe_state, restore_probe_state, probe}`, `PARSER_FIELDS`;
    shared items added: `ForeignOrigins::{lowering, restore_lowering}` and
    `OriginsLowering` in `legacy_container_publication.rs`;
    `contract_consultation::{snapshot, restore}` in `qcore_schemas`). Every
    field of the parser is classified by construction (`probe_state`
    destructures the parser without `..`, and a unit test pins the class list
    against the declaration): **probe state** (the cursor, `next_site`, the
    element lowerings, the element origins' admitted and pending bindings, the
    argument probe, the typed-position and condition-widening counters, the
    variadic-tail widening and readings, the typed place origin, and rule
    13f's active consultation log), **monotone
    facts** (a list iteration's protected element names and links, rule 4;
    the names rule 5l(2) bound over a formal), function facts, lazy caches
    and census labels. Every probe that declines restores the whole probe
    state: the W4, B01 and W45-K probes of a refused return, every guard of
    `classify_simple` that replaces an admission (the W43 descriptor guard
    included, which restored nothing), rule 11w's tail widening when it is
    not admitted (which kept its counters), and every fallback that restored
    `(next_site, element_lowerings)` only; rule 12h's H-U0 classifies the
    uncast statement from the state the first classification started from
    (before, the first one's sites stayed minted: a gap). Monotone facts are
    kept by a declined probe: restoring them could only remove a refusal.
    Parity rows outside the parser are not parser state and stay recorded.
    **Do not** restore part of the probe state, or restore a monotone fact.
    Measured (corpus `66910c10`, system, tail-pause slice, base `1a399e6a1`):
    every report, the null property streams and the parity streams (Clang node
    addresses aside) byte-identical; the leak was not a verdict defect on
    these inputs. The site gap is shown by `tests/gapo_return_correctness.rs`.
    `QCore.ReturnCorrectness` (`declined_restores_probe_state`,
    `declined_restores_consultations`, `declined_keeps_facts`,
    `declined_without_new_facts_is_identity`, `declined_probe_moves_no_site`,
    `replacement_from_before_has_no_gap`, `replacement_after_first_gaps`);
    [report](../reports/gapo-return-correctness/README.md).

<a id="constraint-15y"></a>

15y. **A declared no-return call ends its path at every lowered position,
    or is refused.** Card GAPO-RETURN-CORRECTNESS (`legacy_no_return_paths.rs`,
    `Parser::settle_no_return_nodes`, called once on the built CFG after
    rule 11v's in-place check; `call_exit_resolution` widened to the
    parent). It amends rule 11o, whose statement hook reads only the
    instruction `classify_simple` and rule 12h produced: a call lowered later
    (W44's call actuals, 10u, 10y, 11w's widening, rule 12h's H-U, an
    elaborated or connective piece, a condition's call or plan, a `for`
    clause, a return evaluation) kept its continuation. Every CFG node that
    still has a successor and whose instruction calls a callee the
    environment declares `resource_no_return()` (its own call, a nested call,
    or a call carried by an element, sub-object or pointer store) is
    settled: exactly one discarded whole call with no nested call, no loop
    exit edge and not an exit takes rule 11o's decision over this body's
    Clang resolution (a path end: its successors removed, its own
    instruction still transferred; the refusal `no-return contract of `f`
    not established at this call: …` otherwise), and every other such node is
    refused (`no-return call `f` is not the whole discarded call of its
    node, so its path cannot end there (rule 15y)`), with unbounded writes.
    A node with no successor (rule 11o's path end, an exit) and a node with
    no declared callee are unchanged. A stored call cannot occur: the loader
    rejects a no-return declaration that does not return `unit`. Every
    settled node is a parity row (`position: "no-return-node"`). **Do not**
    let any lowering keep a declared no-return call's continuation, or end
    a path on a premise rule 11o does not establish. Measured (base
    `1a399e6a1`): **119 nodes** settled in the corpus, all `rd_kafka_crash`
    (the `rd_kafka_assert` expansion) lowered in a W44 condition plan at 38
    source lines of 29 functions, every one resolved `noreturn` and made a
    path end; all 29 functions stay `Unknown` on their other refusals, so no
    verdict moved (system and slice: no node); the only other effect is
    rule 11d's census rows (`bound-untracked-root`) turning `path_refused`
    false at six lines of three of those functions, whose refused path now
    ends. `QCore.ReturnCorrectness`
    (`no_return_never_continues`, `settle_identity`, `settle_keeps_refusal`,
    `settled_ends_or_refuses`, `settled_path_end_only_when_established`,
    `settle_call_cases`, `settle_cases`), over `QCore.CallExits.decide`.

<a id="constraint-15z"></a>

15z. **A returned call asks rule 12h's H-U0 as a statement does; the other
    statement-only rules are outside the return route's scope.** Card
    GAPO-RETURN-CORRECTNESS (`legacy_cast_semantics.rs`:
    `uncast_returned_call`, `returned_call_hides_a_place`; hooks in
    `legacy_return_routes.rs`, `legacy_return_family.rs`,
    `legacy_return_call_access.rs`). (1) **12h at the return position.**
    The call W4 classifies as `PLACE = f(…)`, the call B01 classifies alone
    inside a scalar expression and the operand W45-K classifies under a
    conversion are each re-read, when admitted with an identity-preserving
    cast of a place at an actual (rule 12h's table, Clang's call node), as
    its uncast spelling from the probe state the first classification
    started from (rule 15x), and that answer replaces the original before
    the position's own admission test (before, `return r((void *) t)` after
    `q_free(t)` was `Safe`: the cast actual was read as no place). Rule 12i,
    whose W44 re-classification H-U0 is not asked inside, declines a
    returned call that holds such a cast (or `* )` with no unique Clang call
    node). Each re-read is a parity row (`cast-semantics-value`,
    `uncast-return-actuals-admitted`/`-refused`). (2) **Out of the route's
    scope, with their reason.** Rule 11o's hook is replaced at every
    position by rule 15y. Rule 11w's tail widening and `va_*` lowering and
    rule 10u's substituted call are not asked at the return position: a
    return they would read keeps its refusal (11w's tail refusal, a
    `va_*` refusal, the K03 refusal), or, for a W4 admission with an unbound
    custody-taking position, the flow's own `arguments do not match resource
    contract` refusal, so none can be `Safe` there; each such return is a
    parity row (`position: "return-route-gap"`, `gap`: `11w-tail`, `11w-va`,
    `10u-k03`, `10u-unbound`; a lower bound, since a refused return with no
    evaluation names no reason). **Do not** admit a returned call whose
    cast actual hides a place without reading the place. Measured (base
    `1a399e6a1`): **91 returned calls** re-read uncast in the corpus (in 29
    functions; 50 punned, 29 typed, 8 byte, 4 qualifier views), every one
    still admitted, so no verdict moved (27 of those functions stay `Safe`,
    2 `Unknown`); rule 12i's 105 whole-call consultations of such returns
    (none had admitted) are no longer made, nor the 18 W44 rows they wrote
    in one function; the gap census found 3 returns
    (`10u-k03`, one function, `Unknown`) and no `11w` or `10u-unbound`
    return; system and slice: none. The H-U0 gap after the statement
    position's later lowerings is registered
    (`GAPO-UNCAST-ACTUALS-AFTER-LATER-LOWERINGS`). `QCore.ReturnCorrectness`
    (`uncast_at_least_as_strict`, `cast_actual_hid_a_released_place`).

<a id="constraint-30"></a>

30. **A callback's member-path state composes with the release's arms
    before them: a linked object is decided only when every candidate's
    every reading takes it out of the caller's custody exactly once, or
    none does; a stale path, a may-set at a path no arm releases, an
    unstated path and candidates that disagree refuse.** Card
    OP-CALLBACK-MEMBER-PATH-RELEASE (contract specification C04.19.6,
    grammar G05d), on the user's decision of 2026-10-15 ("B: state + set",
    "Build it now"). Loader `crates/qcore-contract/src/contract_member_path.rs`
    (`quant live #P: TAG.PATH |-> V [when GUARD];`, `V` one of `#released`,
    `#moved`, `#stale`, `#kept` or exactly `{V', #kept}`; a guard only beside
    a hand-off of the same formal, its complement), schema
    `qcore_schemas::member_path` (`compose_fate`, `join`, `compose`,
    `state_at`, `COMPOSE_VECTORS`), `RetainingContract::{member_paths,
    member_paths_withdrawn}`; route G `legacy_member_path_route.rs`
    (`operands`, `apply`), hooked in `legacy_callback_route.rs`
    (`classified`: such a candidate reads as its custody contract, after rule
    12m's branch check when it also hands its formal; `callback_operands`:
    the states at each parameter position of a decided invocation, and a
    direct call beside a hand-off refuses `member-path-direct` (every other
    direct call: the amendment below); `apply`: the composition before
    `apply_arms`), in `legacy_constructed_route.rs` (`decide_member`: a
    decided registration's candidates, the same composition) and in
    `legacy_retained_by.rs`. Only a tracked object the caller linked at a
    member path of the object handed at a parameter position is affected
    (`supplied_owned` and `_` hand an object that was never the caller's); a
    path every candidate leaves `#kept` is left to the arms as before.
    Otherwise each candidate's state there is composed with the arms' join at
    the path (`every` / `some` / `none`; an arm on another object counts
    `none`) by C04.19.6's table and joined over the candidates: `gone`
    releases the linked object once and drops the link, `stays` keeps it,
    and the refusals are `member-path-double-release` (a stale path an arm
    releases), `member-path-stale` (a stale path no arm releases: an
    invocation is zero or more times), `member-path-undecided` (a may-set at
    a path no arm releases, candidates that disagree, a link that is not one
    live object) and `member-path-unstated` (a path below a stated one; a
    candidate with no declaration in force refuses
    `callback-uncontracted`). Body check `crates/qcore-c-gateway/src/member_path_check.rs`
    (evidence `30`, rule 13i's `ClauseKind::MemberState`): in a unit that
    defines the declaration, a pointer store into a member path of the stated
    formal other than a null constant into a `#released` / `#moved` path
    (or set), a store of a record holding a stated path, a store into or the
    address of a `#stale` or `#kept` path, and a pointee other than `struct
    TAG` withdraw the states there (the candidate has no contract in force).
    **Amended by card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS**: validation
    is the bounded path reading of `crates/qcore-c-gateway/src/member_path_reading.rs`
    (`Reader`, `validated_exits`; `member_path_check::validates` now takes
    the unit's `callee` lookup and returns `Standing`): the body as a
    control-flow graph (`goto`/labels, `switch`, loops) read to a fixpoint
    over each stated path's value set (`ORIG`, `ORIG_NULL`, `NULL`,
    `OTHER`), a null test refining each branch; a direct call to a `static`
    function of the unit (a header `static inline` included) receiving the
    formal or the address of a member path related to a stated one is
    followed with its parameters bound (depth 8, never recursively, a budget
    of 50,000 nodes), and `return f(…)` keeps the callee's outcome per
    returned value; any other call receiving such a view leaves what it
    reaches `OTHER` (the frame of a call is its arguments: a member's value
    reaches nothing of the formal, a pointer to the formal stored in memory
    is not followed); a view copied into a variable, effects touching the
    formal in two unsequenced operands, an unmodelled construct and a
    pointer store into an unstated path inside a followed helper end the
    reading. Validated when every exit where a state's guard may hold (an
    exit returning exactly a `!=` guard's enumerator is exempt) reads null
    for a must path, no `OTHER` for a set (with a null store read), untouched
    for a stale or kept path; otherwise a declared assumption, never a
    withdrawal. Measured (corpus): `…query_leaders_async_worker_op_cb` and
    `rd_kafka_DeleteRecords_leaders_queried_cb` become partly validated;
    `rd_kafka_admin_fanout_worker` becomes a declared assumption (a
    correction: the formal is passed to the indirect
    `cbs->partial_response` after its null store, which the base check did
    not read); `rd_kafka_admin_worker` stays one (`rd_list_move`, external,
    and the indirect `parse` / `request` reach its stated paths); no verdict
    moves. **Do not** follow a non-`static` or indirect callee, read a
    member's value as the formal, or exempt an exit whose returned value is
    not the guard's own enumerator. `QCore.MemberPathUnitHelpers`
    (`aexec_sound`, `a_validated_must_path_reads_null`,
    `a_validated_set_path_is_kept_or_null`,
    `a_validated_stale_path_is_untouched`, `a_call_summary_is_inlining`,
    `a_call_runs_its_body`, `a_path_not_followed_never_validates`,
    `the_depth_bound_never_validates`, `an_exempt_exit_falsifies_its_guard`,
    `an_unnamed_return_is_never_exempt`, `an_eq_guard_is_never_exempt`);
    [report](../reports/member-path-unit-helpers/README.md). Measured by
    OP-CALLBACK-MEMBER-PATH-RELEASE on the corpus: the
    eight librdkafka candidates of shape (b) are declared
    (`OP-CALLBACK-CANDIDATE-CONTRACTS`); `rd_kafka_op_destroy`'s 131 leaves
    now name `rd_kafka_oauthbearer_refresh_op`, no status moves. **Do not**
    read an unnamed member as released, a stale path as decided, a may-set
    as taken where no arm releases the path, or a member-path declaration
    beside a hand-off at a direct call. `QCore.MemberPathRelease`
    (`gone_leaves_exactly_once`, `stays_never_leaves`,
    `a_stale_path_never_decides`, `an_unstated_member_never_decides`,
    `a_refusal_refuses_the_join`, `a_gone_join_is_gone_for_every_candidate`,
    `the_set_is_the_join_of_its_members`,
    `the_complement_partitions_the_results`, `compose_vectors_hold`);
    `tests/member_path_release.rs`;
    [report](../reports/op-callback-member-path-release/README.md).
    **Amended by card FB-TEARDOWN-DESTROY-DECLARATIONS** (C04.19.6, on the
    user's decision of 2026-10-16, "Yes, allow it"): **a direct call of a
    member-path declaration with no hand-off reads its states once.**
    `legacy_callback_route::callback_operands` builds
    `member_path_route::direct_operands` (each parameter guard decided from
    the call's actual by `retention_guard::constant_value`, an `@` constant
    resolved by `handoff_branch::resolve_parameter` in the guarded
    parameter's own enumeration, `MemberFacts::parameter_types`); hook 2
    runs `apply_direct` before the arms: for each tracked object the caller
    linked at a member path of the actual's object, `compose_direct`
    (`qcore_schemas::member_path`) decides `gone` (`#released`, `#moved`:
    released once, the link dropped), `stays` (`#kept`, an unnamed path,
    P-B08-SHALLOW as before), `taken` (`#stale`: released once, the link
    kept, the site in `AbstractState::member_path_taken`, so a later release
    or hand-off of it refuses `member-path-double-release` in
    `apply_resource_action`), or refuses (`member-path-undecided` for a
    set, an undecided guard's join or a link that is not one live object;
    `member-path-unstated` for a path below or above a stated one, and for a
    declaration its unit's body check withdrew). The loader admits a binder
    stated `resource_released` (a destroyer's own object) and `when #X OP K`
    on a plain integer parameter other than the binder (six comparisons; a
    canonical decimal in range that partitions it, an `@` enumerator or a
    module alias; `ParameterGuard`, `MemberPathState::parameter_guard`); a
    failing guard reads `#kept`, an undecided one the cautious join
    `{V, #kept}` (`state_under`), and at a callback invocation a parameter
    guard always reads the join (`invocation_states`). The body check reads a
    parameter-guarded state at every exit. Declared in
    `quirin-provider-fluent-bit` (`fluent_bit/teardown_destroy.qv`): 14
    declarations, 10 with member-path states, none guarded (no destroyer's
    custody depends on a scalar). **Do not** decide a guard from a value
    that is not a constant in range, read an undecided guard's favourable
    arm, read a stale path at a direct call as gone, or release an object
    a direct call took through a stale path again.
    `QCore.MemberPathDirectCall` (`the_direct_call_state_is_sound`,
    `a_release_after_taken_is_a_double_release`,
    `a_set_never_decides_at_a_direct_call`,
    `an_unstated_path_never_decides`, `a_constant_selects_its_arm`,
    `an_unknown_value_gives_the_join`, `an_unknown_value_never_releases`,
    `the_join_is_cautious`, `direct_vectors_hold`, `guard_vectors_hold`);
    `tests/member_path_direct_call.rs`;
    [report](../reports/fb-teardown-destroy-declarations/README.md).

<a id="constraint-31"></a>

31. **An outer contract states what code loaded at runtime may do; the
    runtime projection joins each closed-world fact a row rests on with
    `Pending` where an in-force outer contract reaching it covers it and
    `Unknown` where an applicable envelope leaves it uncovered, beside the
    unchanged static verdicts.** Card
    OUTER-CONTRACT (contract specification C04.20, grammar G05g; USER
    DECISIONS 2026-10-17, D1 "B: loader envelope", D2 "Yes, as proposed",
    D3 "Both from one run", D4 "Later"). The loader is
    `qcore_contract::outer_contract` (total-or-reject, its own
    `outer_contract_v1` vocabulary, a crate's `outer_view` disjoint from its
    resource and bounds views, `qcore_audit::providers::probe_resource_views`
    loads a crate's outer modules together); the pin is `quirin-audit pin
    --outer` into each crate's `outer-contracts.lock`; the selection, the
    reach and the projection are `qcore_audit::outer_selection`
    (`select_for_route` beside the bounds selection, `compute_reach` beside
    the member pre-pass, `AuditReport::write_outer_projection` after the null
    rows) on the pinned, configured and no-build routes, only with
    `QCORE_OUTER_PROJECTION=PATH`. A row's closed-world facts come from its
    dependency record: a null row's `stable:TAG.FIELD` (rule 14x) and
    `member:TAG.FIELD` (rule 14t) and listed premises; a custody row's
    `run_set_members` (rule 12n) and `stated_members` (rule 14e), which
    `contract_dependencies::annotate_unit` adds to the row field from the
    declarations it names, and the premises of those declarations; every
    named premise is closed-world except `outer_selection::NOT_CLOSED_WORLD`
    (a new one is closed-world until listed). The reach of a `#loaded`
    binder is read from Clang's AST of the loader's own unit
    (`qcore_c_gateway::outer_record_graph`): typed pointers, embedded records,
    arrays, the reverse `container_of` edge of an embedded record; a `void *`,
    an undefined or location-named record, a held loader or an unread unit
    reaches every record. Under each scenario (each contract alone, and
    `all`) a reached fact contributes `Pending`
    (`PendingReason::OuterContract`, reason `rests on outer contract
    NAME@DIGEST`, category `outer-contract`) when the contract is in force and an
    envelope line covers it (`#kept`: a stable, declared or stated member or
    a run set; `#registered`: a run set; `#premise NAME`), and `Unknown`
    (`outer-uncovered`) otherwise; a premise is reached by every contract;
    the runtime verdict is the join with the static one. The static report,
    its sidecar and the null rows are written exactly as without the
    projection (only the dependency field's two member lists are new), and
    the projection is its own document. Fluent Bit (stage 1):
    `flb_external_plugin` on `flb_plugin_load` and `flb_plugin_proxy_create`,
    envelope the four stable `flb_*_instance.p`. User direction 2026-10-18
    removes those four promises from `rk_plugin_library`: reachability does
    not require adding unrelated conditions, and a different envelope cannot
    cover a missing promise on its behalf. The [scope correction](../reports/rk-outer-loader-scope/README.md) retains the
    original checkpoint measurements. Librdkafka's
    `rk_plugin_callback_closure` replaces the retired no-library premise;
    `rk_plugin_library` and the extended `flb_external_plugin` cover it in a
    loaded scenario only where each pinned envelope stands and the named
    outside-code conditions are reviewed. Both cover the relevant librdkafka
    member and premise facts, including a condition forbidding loaded code
    from changing cJSON allocator hooks; interceptor lists require separately
    pinned runtime candidates before a `#registered` envelope may cover them. **Do not** let the projection change a
    static byte or verdict, read a fact a contract does not reach as covered,
    count an uncovered or unpinned fact as `Pending`, promote any row, or
    publish an outer `Pending` without its `NAME@DIGEST`.
    `QCore.OuterContract` (`static_projection_is_identity`,
    `runtime_never_promotes`, `a_row_without_closed_world_facts_is_unchanged`,
    `a_covered_fact_is_pending_with_its_contract`,
    `a_pending_contribution_names_its_contract`,
    `every_outer_pending_names_a_contract`,
    `an_uncovered_fact_is_never_safe_or_pending`, `runtime_is_the_join`,
    `reachability_over_approximates`, `projection_vectors_hold`);
    `crates/qcore-contract/tests/outer_contract.rs`,
    `crates/qcore-c-gateway/tests/outer_record_graph.rs`,
    `crates/qcore-audit/tests/outer_projection.rs`;
    [report](../reports/outer-contract/README.md).

    Runtime list candidates (`RK-PLUGIN-INTERCEPTOR-CANDIDATES`, C04.20.1)
    are a separate versioned `outer-candidates.json` inventory, bound by
    its exact digest in the defining outer statement and exact reviewed
    source bytes. The generic `qcore_audit::outer_candidates` loader keeps
    list/element/union-arm/signature identities distinct, rejects missing,
    malformed, ambiguous or unbound inventories, and holds citing loaders
    on a source mismatch. Its neutral-custody and success/lifecycle
    restrictions are premises for human review of loaded callbacks and
    their transitive calls, never effects inferred from signatures. The
    runtime record carries the inventory and standing. It does not enter
    rule 12n's static registry: list invocation remains unsupported and
    Unknown. No list is coerced to a function pointer and no union-arm
    closure is inferred. An in-force covered runtime fact remains Pending.
    `QCore.OuterListCandidates`; `outer_candidates`' Rust tests;
    [report](../reports/rk-plugin-interceptor-candidates/README.md).

<a id="constraint-32"></a>

32. **A caller-frame declaration states exactly the premises its body's
    effects leave visible from the caller's frame.** Card
    W48-RK-LOG-CALLER-FRAME, on the user's decision of 2026-10-22 (a
    caller-facing contract with a named premise, challenged for Safe
    outright). A caller-frame declaration borrows every formal and its
    variadic tail (`resource_variadic_borrowed_tail()`), returns nothing and
    carries no other custody clause, so its callers read the call as the
    identity on their own custody. `qcore_contract::caller_frame` checks
    a projection (`caller-frame-v1`) of a reviewed effect model
    (`logger-effects-v1`, held by its digest): every model obligation is
    classified at each event that raises it as `internal` (fresh object,
    borrowed read, library state, queue state reached only through a
    borrowed formal, after the frame, non-returning traces only, trust
    boundary 28) or as covered by a named `premise`; each path's standing is
    derived (proved, premised with exactly the premises its in-frame events
    name, infeasible for a fresh object, or outside the frame with its
    in-frame prefix equal to an in-frame path) and a stated standing that
    differs refuses; `check_declarations` then requires every declaration
    the projection names to be caller-frame shaped (textually and as loaded)
    and to state exactly the derived premise set, no more and no fewer. No
    obligation is discharged: the model's are all still Open; an `internal`
    one is only not the caller's. Declared in `qcore-provider-librdkafka`
    (`librdkafka/log_caller_frame.qv`): `rd_kafka_log0` and the static
    `rd_kafka_log_buf`, each naming `rk_plugin_callback_closure` (the log
    callback), `rk_log_enqueue_no_event_callback` (a queue event callback
    `rd_kafka_q_io_event` runs synchronously) and
    `rk_log_enqueue_forward_not_last` (a concurrent unforward-and-release
    that makes the forward drop the last). Proved without a premise: the
    filtered, `rk == NULL`, `!log_queue`, `!rk->rk_logq` and no-callback
    paths, the disabled-queue reply of the fresh op (its empty reply queue:
    destroyed, freeing only its own string), the copied serve callback
    (never invoked by the enqueue) and the balanced forward keep/drop in
    sequential execution; so callers are **conditionally Safe**, never Safe
    outright. The declarations' bodies are checked by the existing route
    (W4/`entry`, rule 13i: the variadic tail and the premises are never
    covered; rule 10g withdraws a refuted one), and the declarations serve only
    callers outside the frame (rule 32a). Measured on the eight-home corpus
    (BASE `4dd8c8013`, TIP `f53a127e1`): seven callers Unknown→Safe on the
    three premises, the planner's three included; `rd_kafka_log0` and
    `rd_kafka_log_buf` stay Unknown. **Do
    not** declare a caller-frame contract whose premises differ from its
    projection's derived set, classify a callback, a reference release that
    may be the last or a write to an object the caller may hold as
    `internal`, or read a premise as discharged by the projection.
    `QCore.RkLogCallerFrame` (`premised_covers_every_caller_event`,
    `proved_path_needs_no_premise`, `a_reply_path_is_infeasible`,
    `model_paths_derive`,
    `the_consumer_path_leaves_the_frame_after_the_local_enqueue`,
    `each_premise_is_needed`, `enq_restores_and_destroys_nothing`,
    `enq_links_only_the_fresh_op`, `a_tracked_object_is_untouched`,
    `the_disabled_reply_frees_only_the_fresh_objects`,
    `caller_frame_admission`,
    `an_interleaved_release_makes_the_forward_drop_last`,
    `an_event_callback_can_release_a_tracked_object`);
    `crates/qcore-contract/tests/caller_frame.rs`;
    [report](../reports/w48-rk-log-caller-frame/README.md).

<a id="constraint-32a"></a>

32a. **A caller-frame declaration serves only callers outside its frame.**
    Card W48-RK-LOG-CALLER-FRAME (the halted review of 2026-10-23: the
    logger's own row became `Safe` by reading its sibling's declaration). The
    members of a reviewed caller-frame projection are its `declarations`,
    registered in `qcore_contract::caller_frame::FRAMES` (held equal to the
    checked projection by test). Inside a member's own body, every call to a
    member of the same frame whose declaration is in force, the recursive
    call included, is **held**: rewritten to `qcore_caller_frame_self__`
    (`legacy_caller_frame_hold`, beside the pin hold in all three token
    pipelines, so W4 derivation, validation and a certificate recheck
    reproduce it; the prefix is reserved and a source spelling is escaped).
    Which calls are held is decided by the rule package
    `crates/gateway-rules/rule-32a-caller-frame-hold` (card RULE32A-PILOT;
    the rule interface's token hold `caller-frame-hold`, registry row `32a`,
    which reserves the reason key), from the frames and whether the callee's
    declaration is in force; the host finds the member bodies and call
    positions and spells the hold. The hold is **not switchable**
    (`off = "not-switchable"`): a `QCORE_GATEWAY_RULES_OFF` list naming
    `32a` is refused, and the host asks the package under every list, a
    refused one included, because removing the hold would let a member read
    its frame's declaration again (pluggable-cards plan §6 item 10).
    The statement refuses `Unknown`, never `Pending`, with reason key
    `caller-frame-self-reliance` (category `incomplete-obligation-coverage`:
    the frame's own obligations have no derivation). Each member's row is
    therefore decided by its body, with the model's obligations still Open;
    a caller outside the frame reads the declarations unchanged, with their
    premises. Monotone: the hold never moves a row towards `Safe`. **Do
    not** let a member's row read its own or a sibling's caller-frame
    declaration, register a frame member set that differs from its checked
    projection's declarations, treat the hold as a refutation (the
    declaration still serves outside callers), or give the hold a switch
    that removes it. `QCore.RkLogCallerFrame`
    (`a_frame_hold_is_unknown`, `a_member_never_reads_a_member_declaration`,
    `a_held_member_row_is_never_safe`,
    `an_outside_caller_reads_the_declaration`, `the_hold_never_certifies`,
    `the_logger_rows_are_decided_by_their_bodies`);
    `QCore.CallerFrameHoldSwitch` (the package's decision table, the fixed
    switch, `a_refusing_off_would_widen`,
    `holding_more_changes_no_member_row`);
    `crates/qcore-c-gateway/tests/caller_frame_hold.rs`,
    `crates/qcore-c-gateway/tests/rule32a_hold.rs`;
    [report](../reports/w48-rk-log-caller-frame/README.md),
    [package](../reports/rule32a-pilot/README.md).

<a id="constraint-33"></a>

33. **A checked removed-selection frame transports its selected child as a
    caller prerequisite: it is never fresh custody, and its membership is
    never read off borrowed provenance or the representation premise.** Card
    DETACH-RESULT-WRAPPER-COMPOSITION (`legacy_selection.rs`, pure decisions
    `removed_selection.rs`). A *selection detach* `F` is a function whose
    complete body, after renaming and the detach/lookup readers' NULL
    normalization, is exactly `if (W < 0) { return NULL; } return D(P, L(P,
    (size_t)W));` for a `R *P` naming the lookup's canonical record (a direct
    defining-typedef alias or `struct` tag), an `int W`, an unsigned
    `size_t`, an `L` that passes rule 13a's complete indexed lookup frame with
    its borrowed-from summary in force, and a `D` in force with exactly the
    same-item `Removed` shape, heap kinds, no container, lifetime, retaining
    or guarded clause and the exact named `detach_list_representation`
    premise. A *selection release* `G` is exactly `void G(R *P, int W) {
    X(F(P, W)); }` for such an `F` and an `X` in force with exactly one
    nullable heap `Released` formal and no result. An external-linkage
    `inline` definition is no frame (C11 6.7.4p7); every other frame
    definition is the body this unit's calls run, as for a selected summary.
    The selected child is `sel(P, W) = (W < 0) ? NULL : L(P, (size_t)W)`.

    Each frame's own row is analysed by the ordinary analyzer over its
    expanded body (the build-stream rewrite beside the other body rewrites;
    the reserved `__qcore_selection_*` places must be unspelled) under a
    *selection entry*: `P` one live, settled, non-NULL owned site and `W`
    borrowed. At the frame's one lookup call, reached from entry through
    branches and reserved initializations only, the selected child appears as
    one nullable child uniquely contained by `P` (`materialize`: frame node,
    live owner identity, pristine path, fresh child, surviving borrowed
    result). It is recorded as a contained identity, not a borrow. The
    existing same-item adapter then decides the detach unchanged. The row
    validates only when the run is diagnostic-free and `Safe`, selects on
    some path and accepts every exit (`accept_exit`): `P` live, no caller
    containment, out slot, pending replacement or derived-base effect, and
    for `F` either never selected and returning nothing (the `W < 0` arm the
    exact frame isolates) or returning exactly the escaped detached child;
    for `G` returning nothing with the child absent or released. A row that
    does not validate falls back to the contract-free analysis. Nothing is
    published as a `ResourceFunctionContract` or summary record, so no other
    route, unit or ordinary W4/fresh-result reading sees the selected child.

    A caller transports the summary only by expansion: a statement `d =
    F(q, K);`, `R *d = F(q, K);`, `struct T *d = F(q, K);`, `F(q, K);` or
    `G(q, K);`, starting after `;`, `{` or `}`, with a plain parent name and
    a decimal constant `K` in 0–255, becomes the frame's own constituent
    calls with the guard decided (`K >= 0`) and `(size_t)K == K`. Rule 13a's
    lookup identity, explicit physical prefix and record binding, the
    same-item membership, NULL and alias checks and `D`'s declaration with
    its named premise then apply unchanged at the caller. Every other use of
    `F` or `G` (a formal or nonliteral index, a nested, returned, cast or
    conditional call, an unknown or wrong parent) keeps the uncontracted-call
    refusal. **Do not** derive the selected child's membership from the
    representation premise, borrowed-from provenance or a lookup summary;
    publish a frame as an ordinary contract; admit a frame whose detach,
    lookup or release is not exactly the checked shape; or expand any other
    call shape. String-keyed object lookup has no checked membership frame
    and stays refused. Every promoted row remains conditional on `D`'s
    `DeclaredAssumption` premise and `L`'s null premise.

    The rule's body is the package `crates/gateway-rules/rule-33-removed-selection`
    (card RULE33-PILOT: the frame bodies' word shapes, the caller expansion,
    the selection decisions), asked through the rule interface's
    `StatementExpansion` and `EntryRow` capabilities at two registered arms:
    the body rewrite of each CFG build (`selection_route::expand`) and the
    frame's own row (`selection_route::row`, which installs the frame).
    `legacy_selection.rs` stays the host: it reads each frame against the
    contract environment, the type model and the linkage, builds the
    selection entry and observes the run. `QCORE_GATEWAY_RULES_OFF=33` asks
    neither arm: no body is expanded and no frame is installed, so every
    frame and caller keeps its pre-rule-33 refusal and every frame name takes
    the older routes.

    `QCore.RemovedSelection` (`selected_is_a_member`,
    `nonnegative_constant_is_the_lookup`, `the_result_is_the_selection`,
    `detach_inventory_is_preserved`, `delete_releases_exactly_the_selection`,
    `materialization_needs_every_fact`,
    `accepted_detach_exit_returns_the_selection_or_nothing`,
    `accepted_release_exit_keeps_no_selection` and the three shared vector
    tables) and `QCore.RemovedSelectionSwitch` (on is the rule, off is the
    gateway before it, off never widens a frame row or a caller;
    `AuditRemovedSelectionSwitch.lean`);
    `crates/qcore-c-gateway/tests/detach_wrapper_composition.rs`,
    `crates/qcore-c-gateway/tests/rule33_switch.rs`;
    [report](../reports/detach-result-wrapper-composition/README.md),
    [migration](../reports/rule33-pilot/README.md).

<a id="constraint-34"></a>

34. **An element destroyer unlinks, then releases; nothing else about the
    entry changes.** Card W48-SCHED-CLEANUP-PRODUCTION (contract
    specification C04.8, last paragraph). A declaration may state
    `resource_unpublished(p)` beside `resource_released(p, …)`
    (`qcore_contract::contract_counted_users::validate`; a publication's
    entry stays `resource_borrowed`, and an unlink beside any other custody
    mode rejects the module). At a call, the transfer arm `destroyer-order` of
    the rule package `crates/gateway-rules/rule-34-element-destroyer` (card
    RULE34-PILOT; the rule interface's transfer position, route map §10,
    asked by `transfer_rules::container_order`) takes those unlinks out of
    the call's container operands and the `ContractCall` arm of
    `legacy_transfer.rs` applies them **before** the call's own reading, so its release ends exactly the custody the unlink
    took: rule 10d's restored published site, or rule 10f(2)'s admitted
    foreign element (a `container_of` element of a safe iteration, or a
    `->next` lookup, over a container rooted at an unrebound formal, a local
    initialised from one, or a static object) at its fresh site. Every other
    entry keeps the unlink's refusal (`unlink by ... which this function has
    not published`): a formal's own object, a body acquisition never
    published, a local, rebound or callee-returned container, an element
    name written another way, an element already unlinked, a second
    destroyer. A plain iteration and a saved successor keep rule 5l's
    refusals; a use after the destroyer reads a released binding; a release
    after it is a double release. Rule 10f(3) is unchanged: a body that
    lowers such an unlink publishes no summary, so its callers keep their
    uncontracted call. The premises are P-CRR-MEMBER and P-CRR-LINKED at the
    caller, and whatever the declaration names. First production use:
    `mk_http_thread_destroy` (`qcore-provider-libevent`,
    `monkey/intrusive_list.qv`, pinned at `046accf6` and `66910c10`), whose
    eight `resource_premise` clauses carry the scheduler decomposition's
    Open hypotheses (parent embedding, callback footprint with the
    creation-NULL source reading, params key ownership and API success,
    coroutine, borrowed chain, membership and writers, later aliases,
    Valgrind request); every row resting on it lists them. **Do not** apply
    the release before the unlink, admit an entry 10d/10f do not, read a
    destroyer as a borrow anywhere a container effect must refuse, or drop
    10f(3)'s summary gate without a caller-side rule that forgets the
    published sites the call may release. **Switch.** With `34` in
    `QCORE_GATEWAY_RULES_OFF` the arm is not asked, and the host's guard
    refuses a call that still declares an unlink of a parameter it releases,
    before any of its effects apply (`element destroyer `NAME`: no transfer
    rule orders its unlink before its release (gateway rule 34 is off)`; its
    arguments lose their residual claim and every binding is poisoned): the
    call reads `Unknown`, as an uncontracted call did before rule 34, and no
    other call changes. The guard is the host's end of the transfer table,
    not a package, so it is not switchable: the host's own order would
    release before unlinking, which is what this rule forbids, so off never
    falls back to it. `QCore.ElementDestroyer`
    (`destroying_an_admitted_element_is_safe`,
    `the_release_never_precedes_the_unlink`,
    `the_modelled_destroy_is_an_unlink_then_a_release`,
    `a_modelled_cleanup_unlinks_and_retires`, …); `QCore.ElementDestroyerSwitch`
    (`the_phase_vectors_are_the_arm`, `the_switch_on_is_rule_34`,
    `a_call_without_a_destroyer_ignores_the_switch`,
    `the_switch_off_refuses_the_destroyer`, `the_switch_off_never_widens`, …);
    `crates/qcore-c-gateway/tests/element_destroyer.rs` (with its switched
    child), the package's `tests/unit/destroyer_order.rs`;
    [report](../reports/w48-sched-cleanup-production/README.md),
    [migration](../reports/rule34-pilot/README.md).

<a id="constraint-35"></a>

35. **A `*` applied to a string literal that is only read is constant data,
    and an array member decayed at a `Borrowed` or data-tail call actual is
    rule 14c's element address `&PATH[0]`.** Card W48-BU-SINGLE-LEAF-BODIES;
    since card RULE35-PILOT the rule package
    `crates/gateway-rules/rule-35-literal-decay` (`literal_read.rs`, the
    classification arm `literal-read`, hosted by `rule_hosts::literal_read` in
    `legacy_token_shapes::contains_unmodeled_access`; `array_member_decay.rs`,
    the typed-reader arm `array-member-decay`, hosted by
    `rule_hosts::decay_reads` in `typed_access_positions::StatementIndex::argument`;
    switch `35` turns both off). It
    amends the lexical access gate (arm 1) and rules 10n(1) and 14c (arm 2);
    every other refusal of theirs stands. (1) **Literal read.** A `*` token
    whose next token is a literal (the lexer spells every string and
    character literal `__qcore_literal__`) is not an access when the literal
    run is not followed by `=` (other than `==`), a compound assignment, `++`
    or `--`. A string literal is an array of static storage duration (C11
    6.4.5p6), so `*"..."` reads its first character: no pointer the
    projection names is read and no binding, field link or obligation
    changes; `* 'c'` can only be a multiplication. No value is folded
    (constraint 5a), so both edges of a branch on it stay. Every other token
    of the predicate answers as before (`->`, `.`, `[`, `]` and every other
    `*`), so the exempted stars are a subset of the old refusals. This is
    librdkafka's `rd_assert(!*"...")`, `RD_NOTREACHED()` and `RD_BUG(...)`.
    (2) **Decayed array member.** At a call actual the W44 typed reader
    reads (positions the caller already found admissible), an
    `ImplicitCastExpr` of kind `ArrayToPointerDecay` whose operand, through
    parentheses, is a member place of a one-dimensional array of builtin
    scalars, neither `volatile` nor `_Atomic`, is read as the address of its
    initial element (C11 6.3.2.1p3): rule 10n(1)'s address reading with every
    hop's obligation, the array not read, and the never-bound premise on
    every root unchanged. **Do not** exempt a stored or incremented literal,
    a subscripted literal (`"..."[I]`) or any other `*`; read a decayed
    element of pointer, record, union, array, enumeration or opaque type, a
    qualified member, a decayed local or file-scope array (the older routes
    keep their answer), or relax the root premise; or use the decay at a
    custody-taking position. Parity runs record each decay decision as a
    `typed-place` place (`rule: "35"`). Premises stated, not proved: faithful
    lexing of literals and Clang's cast kinds and member types (trust
    boundary 28), and, as for rule 14c, that a callee writes within the
    array the address points into. `QCore.LiteralDecayReadings`
    (`literal_vectors_hold`, `a_star_is_exempt_exactly_on_a_read_literal`,
    `the_gate_only_admits`, `every_other_token_is_unchanged`,
    `a_run_refused_only_at_literal_stars_is_admitted`,
    `a_literal_read_changes_no_binding_and_owes_nothing`,
    `decay_vectors_hold`, `a_decay_is_admitted_exactly_for_a_scalar_member`,
    `the_decay_is_the_first_element_address`,
    `an_admitted_decay_is_a_14c_element_address` and four more), bound to
    both Rust `DECISION_VECTORS` tables;
    `crates/qcore-c-gateway/tests/literal_decay_readings.rs`;
    [report](../reports/w48-bu-single-leaf-bodies/README.md).

<a id="constraint-36"></a>

36. **An exact borrowed-formal return preserves identity and custody.**
    W48-BU-CMT-MAP-METRIC-GET's narrowed first feature. The derived-result CFG
    alone may lower a bare `return P;` to `ExactFormalReturn`. A positive,
    unambiguous declaration reading must give the result and the original
    formal matching depth-one object-pointer types. The typed source reader
    must confirm the formal's parameter index, declaration identity, original
    return reference, complete spans and matching lexical/AST occurrences;
    writes or address-taking of that formal, conflicting value bindings,
    volatile/atomic pointer objects, or missing facts decline. Existing source
    queries and earlier non-neutral results retain precedence. A neutral `None`
    result may be refined only by a freshly validated derived summary with the
    identical parameter custody vector; a failed refinement retains the old
    result. The exit admits only
    a known identity: definitely NULL, or one status-free original formal site
    with the matching index and no unresolved effect. Known non-null untracked,
    unknown, mixed, derived, owned, released or unrelated bindings refuse. An
    admitted transfer preserves every state field; a refusal adds UnknownAlias
    and the existing unresolved-effect exposure without changing resource
    status, binding or containment. The new instruction is appended to preserve
    existing serialized tags, participates in volatile settlement, and admits
    no descriptor read. The ordinary full-body, all-reachable-exit,
    untouched-base and fresh reported-premise validation gates still decide the
    summary. Callers apply the existing `Derived(Exact)` semantics: the alias
    retains the base's nullness and invalidation. The independent NULL path
    search preserves known NULL for an exact result and cuts an unresolved
    derived base as unresolved, never infeasible by loss of that base. The alias
    owns nothing and discharges
    no allocation obligation. `QCore.ExactFormalReturn` models the source
    conjunction, classifier and transfer, proves preservation and relative
    exactness including NULL/interior bases, and shares fourteen flow vectors
    with Rust. Source extraction and Rust correspondence remain premise 28.
    This establishes none of the CMetrics getter chain's foreign-member,
    collection, secondary-index or cache ownership obligations; its remainder
    stays Open. [Design](../reports/w48-bu-cmt-map-metric-get/DESIGN.md).
    The body is the rule package `crates/gateway-rules/rule-36-exact-formal-return`
    (card RULE36-PILOT), registry id `36`, the first `return-route` package:
    with `36` in `QCORE_GATEWAY_RULES_OFF` the gateway asks none of its parts,
    so the lexical candidate is false, the return keeps its pre-rule-36
    lowering and the classifier refuses; off only removes admissions
    (`QCore.ExactFormalReturnSwitch`). [Pilot](../reports/rule36-pilot/README.md).

<a id="constraint-36a"></a>

36a. **A checked initializer call retains its caller-frame condition.**
    CG11 already admits literal self-address stores and withholds their
    ordinary summaries. A new `SelfInitCall` may replace only a refused direct
    call in a complete source-checked void caller with one original depth-one
    struct-pointer formal. Its body contains only blocks, unvalued returns,
    base NULL guards, literal self-stores and direct calls passing that same
    formal to initializer leaves. A leaf contains at least one self-store and
    no calls. Complete matching source spans, exact declaration identities and
    pointer types, and positive complete nonunion/nonbitfield member facts are
    required. Missing facts, aliases, other effects, escaping function
    designators, recursive calls and wrappers of wrappers decline. Every leaf's
    actual CFG must validate through the existing CG11 stores, with its stores
    matching the source and every reachable exit custody-empty.
    The call is conditional on an internal checked-row entry capability,
    default false, an empty modeled custody frame and no member-place aliases.
    Admission preserves the complete state; refusal executes the exact old
    Unknown fallback. Structurally unreachable markers retain their old
    lowering. The marker is appended to preserve older serialized tags and
    cannot establish an ordinary custody summary. A private caller row must
    independently validate the complete CFG and every exit; nothing is
    installed in the contract environment or summary records. Declared calls,
    older modeled statements and interfaces retain precedence. Ordinary
    inference, callbacks and cross-unit imports gain no unguarded effect.
    Tracked/interior caller transport and full owned-list semantics remain
    separate obligations. `QCore.CheckedSelfInitCall` models source and frame
    refusal, state/ledger preservation, finite call composition, lifetime and
    transport boundaries. Extraction and Rust correspondence remain premise 28;
    new readers must fit rule 13q's existing measured stack budget.
    [Design](../reports/w48-cmt-list-self-init/DESIGN.md).

<a id="constraint-36b"></a>

36b. **A tracked initializer transports writes without erasing ownership.**
    A `TrackedSelfInit` may replace only the old refused direct-call or
    indirect-access instruction after earlier modeled statements decline.
    Reuse rule 36a's complete source/CG11 leaf validation, retaining the exact
    declaration and self-write field set. The actual is a uniquely declared
    depth-one struct pointer or the address of its embedded record through at
    most three ordinary fields (one arrow followed by dots). Loaded pointer
    members, arrays, casts, unions, bitfields, qualified or incomplete types,
    descriptor/in-place effects and escaping callee designators decline.
    The caller must hold one known, nonnull identity with exactly LIVE status
    and no nullable-site fact. The original route requires every custody field
    other than pointer bindings, resource statuses and nullable sites to equal
    its default; it runs first, unchanged. Its existing rooted member facts may
    be empty or name only the same owner.
    An additive nondefault route permits only nonempty child field links of
    that actual owner, with partial triples a subset of those links; all root
    and child identities must be precise Heap, exactly LIVE and nonnullable,
    with no self child, caller custody or sanctioned out slots. Every other
    custody field must still equal its default, including future fields. No
    foreign-owner, containment, membership, publication, collection, retention,
    borrow, callback or unresolved relation is discarded for admission.
    The optional source proof binds each protected ordinary direct pointer
    slot to its actual field declaration and unique complete root record.
    Rule 11k's core footprint checker must decide it disjoint from every
    initializer write using Clang layout, or from the whole first enclosing
    member for an embedded target. Different allocation IDs give no separation
    or allocator-freshness premise. Missing, ambiguous or overlapping evidence
    declines. Across the whole caller, pointer initializers/assignments permit
    only same-type local copies, direct in-force Owned results or known NULL;
    pointer slot addresses, pointer increments/compound assignments, retyping
    and unsupported places decline. All other calls need ordinary contracts
    in force, without Out/Stored/Removed, membership/count, relational or
    retention/lifetime effects; indirect, guarded and missing interfaces decline.
    Current owner identity aliases must be known, nonnull, singleton and source
    type compatible. Older aggregate-store alias refusals remain unchanged.
    A declined optional probe restores its reading and consultation state.
    The existing indirect-access check must add no diagnostic or state change.
    The default route forgets all owner-rooted member pointer facts through
    every alias. The nondefault route retains only existing protected direct
    slot facts through compatible identity aliases, conservatively forgetting
    every other recognized member fact across all roots; unprotected owner-rooted
    facts with foreign/unknown targets still decline. Plain/derived bindings,
    every resource status and nullable fact, and all field/partial links remain
    intact. Unrecognized pointer/out-slot keys decline the nondefault route.
    It creates no identity or ownership link and exports no empty-list or
    membership postcondition. Failure executes the exact old fallback;
    structurally unreachable markers regain that fallback. The appended tag
    never supplies an ordinary summary, callback permission or cross-unit
    contract; the whole caller and its exits still decide its own result.
    Full owned-list semantics and unsupported custody frames stay separate
    tracked obligations. The canonical CFL `cfl/intrusive_list.qv`
    declares these preconditions/effects as a model, outside both interpreted
    resource and bounds views; it grants no resource-v1 initializer contract.
    `QCore.TrackedSelfInit`, registered in `AuditTrackedSelfInit`, models source
    admission, invalidation, ledger preservation, finite composition and
    embedding lifetime, with shared source/flow vectors. The additional
    `QCore.TrackedSelfInitDisjointFrame`, registered in its own audit, connects
    same-owner byte separation and embedded subwrites with ledger/field/partial
    preservation, conservative fact retention, refusals and finite composition;
    shared flow/core-layout vectors and source controls bind the reviewed model.
    Faithful extraction,
    parent embedding and Rust/model correspondence remain premise 28.
    [Design](../reports/w48-cmt-tracked-interior-self-init/DESIGN.md).
    [Child-slot frame design](../reports/cmt-self-init-disjoint-frames/DESIGN.md).

<a id="constraint-36c"></a>

36c. **CMetrics API contracts preserve partial and transitive ownership.**
    Canonical `cmetrics/map_lifetime.qv` and `cmetrics/opts_lifetime.qv` are
    source-audited declared models, outside interpreted views. A model file or
    successful parse supplies no executable contract permission. Map creation
    copies label keys and retains nonowning opts/parent record views. A getter
    returns a map-borrowed embedded or primary-owned metric, never fresh caller
    custody; NULL can follow committed creation, and read lookup can mutate the
    secondary index. Primary ownership, secondary/cache references and required
    backpointers are distinct obligations. Retirement needs external user
    coordination and invalidates affected borrows; lookup locking alone is not
    a returned loan. Opts initialization retains committed SDS fields on
    failure. Cleanup frees their contents without clearing pointer fields or
    freeing the enclosing record, and is not generally idempotent. Opts string
    retirement can precede map teardown when no later operation reads them.
    Do not erase these effects into Borrowed/None or assume rollback, child
    ownership or membership from a function name. Trusted adoption requires
    checked source/effect, lifetime, alias, representation and synchronization
    consumers under `CMT-MAP-OPTS-CONTRACT-ADOPTION`. Labeled hash-zero versus
    embedded static-storage accounting remains separately Open under
    `CMT-LABELED-HASH-ZERO-STATIC-STORAGE`, without an upstream defect claim.
    `QCore.CmetricsApiContracts` is registered in `AuditCmetricsApiContracts`; shared
    Rust/Lean lifecycle witnesses model these boundaries. Source extraction
    and model correspondence remain premise 28. The actual model source gate
    and any earlier native measurements retain their separate source/input
    identities. [API specification](../reports/w48-cmt-tracked-interior-self-init/CMETRICS_APIS.md).

<a id="constraint-37"></a>

37. **Checked result constructors.** Aggregate field stores may ignore a
    pointer-name occurrence only inside a `sizeof` operand admitted by the
    existing typed, non-variably-modified operand reader. A syntactic `sizeof`
    alone supplies no exemption. The base must still be a uniquely bound local
    aggregate with complete declarations, an admitted member shape, no alias or
    evaluated escape, and a currently live, known, non-null owner. The exemption
    establishes neither non-nullness nor a resource identity. Linked children
    escape with a returned aggregate; a shallow release leaves a live child
    outstanding, and overwriting a field cannot discharge its old identity.

    A wrapper whose normal return is non-null only because allocation failure
    terminates needs a checked-body specification and the named
    `asserted_allocation` premise. Adopt it only after the complete preprocessed
    body establishes the specification and the reviewed source pins match.
    Missing bodies, disabled or unread assertions, and unproved terminating
    effects withdraw the specification. Keep the premise visible in verdict
    assumptions; a body observation does not prove the deployed-build premise.
    Do not change unrelated libc or wrapper nullability by family or spelling.

    The null-property readers must not treat a checked custody specification as
    an unchecked non-null result fact. They may use an independent null source
    summary or the null property's own established declaration. The resource
    identity must bind relevant checked-result metadata. Ordinary unchecked
    declarations retain their existing declared-assumption reading.

    Rule 37's model proofs depend on faithful C extraction, faithful checked
    operand spans, and Rust/model correspondence. Keep these premises separate
    from fixture observations and production-body validation. Publish only the
    card's fresh per-body and per-leaf BASE/TIP movements, with each movement's
    bounds and assumptions; planner ceilings are not gains.

    The rule's body is the package `crates/gateway-rules/rule-37-result-constructors`
    (card RULE37-PILOT), asked through the rule interface at two registered
    arms: the store scan's checked operand spans (`aggregate_store::store_bases`)
    and the checked-specification settlement (`legacy_checked_spec::settle`),
    where a specification naming `asserted_allocation` stands only while the
    package admits it with that premise as an obligation the declaration
    states. While such a specification stands, every row whose derivation
    reaches it carries the admission's premise among its assumptions
    (`rule_obligations`, folded into the verdict contract's `assumptions`;
    card ARM-OBLIGATIONS, which carries any package admission's obligations
    into the rows resting on it and keeps a row with an open one out of
    `Safe`). Card ARM-OBLIGATIONS-UNIT-GRAIN narrows the carriage from the
    unit to those rows: the specification's own row and every row whose
    work, or the work of an in-unit function it consulted, transitively,
    consulted the specification's name (its body's spellings, minted ones
    read back, its applied callees, its relied-on callback candidates), noted
    per unit frame in every run (`contract_dependencies::attribution`), never
    read from the audit's recording, so the certificate recheck reproduces
    it. An admission without an established attribution (a host's, a nested
    analysis's hand-up) keeps the unit grain. A withdrawn specification's
    admission carries nothing, since no row rests on it.
    `QCORE_GATEWAY_RULES_OFF=37` removes both answers: the scan counts every
    mention, as before this rule, and every such specification is
    withdrawn; its callers then see only what W4 validates from an in-unit
    body. The null readers' checked-result guard is a restriction, so it stays
    in the host and is never switched off.

    Lean: `QCore.ResultConstructors` (all companion theorems registered in
    `AuditResultConstructors.lean`) and `QCore.ResultConstructorSwitch` (on is
    the rule, off is the old reading, off never widens;
    `AuditResultConstructorSwitch.lean`);
    [report](../reports/w48-bu-rk-result-constructors/README.md),
    [migration](../reports/rule37-pilot/README.md).

<a id="constraint-38"></a>

38. **Descriptor cells are read only where the path decides them.** Card
    MK-EVENT-DESCRIPTOR-QUANT (contract specification
    [§C04.6.2](../QCORE_CONTRACT_SPEC_r0_3.md#c0462-descriptor-cells); USER
    DECISION 2026-11-01, verbatim: "I approve the changes of D1-D5."). The
    package `crates/gateway-rules/rule-38-descriptor-cells` holds the cell
    table (`cells::Cells`); the host is `legacy_descriptor_cells.rs`, asked from
    rule 12t's retaining call and its transfer. A cell is a **name** of a
    descriptor identity; the table records, per path, each name's identity
    and whether it is live, undecided (`Maybe`) or stale.

    (1) **Keys.** A member cell's actual must have one Clang type in every
    call of the callee in the calling function (its type before the
    conversion to `void *`), and that type must be `struct TAG *`, or (card
    MK-EVENT-FIRST-MEMBER-CELL, C11 6.7.2.1p15: a pointer to a structure,
    suitably converted, points to its initial member) `struct R *` where the
    unit's one complete definition of `R` declares its first member `m` with
    type `struct TAG`. A place `X` of the second kind (a name followed by
    member steps) is read as `&X->m`, the direct reading's own key, so a
    creation through `X` and a release through `&X->m` meet. A first member
    of any other type (a pointer or array of the event, a qualified event,
    another record, so a nested chain refuses), a later member, a record with
    no member, a union, an incomplete or doubly defined record, a mix of
    types, an unknown type and any other spelling of the actual refuse
    (`descriptor-cell-undecided`; package `initial_member::member_view`).
    The object is then read as rule 12t reads a retained pointer (a caller's
    formal or interior, a tracked identity or its interior). A slot's actual
    is `&L` (a local) or `&X->f[K]`. Anything else is no key.
    (2) **Creation** (`#fresh(K)`) records a new identity under its key,
    `Maybe` until the witness decides it; a key that may name a live
    descriptor refuses (`descriptor-cell-overwritten`), and the same call
    reached again while its identity is recorded refuses. `#same(S)` records
    a second name of the slot's identity. (3) **The witness**: the call's
    result held by a plain name, until the name is written; a validity test
    of it (`!= -1` / `== -1` guards) or a null test (`== 0` / `!= 0`) makes the
    creation's names live on the edge its class holds and stale on the other
    (the failure class may leave a closed number in the cell); a test of
    another kind decides nothing. (4) **Release** (`#stale(K)`, or a guarded
    release through the cell an integer actual reads) of a live name stales
    every name of its identity; of a stale name refuses
    (`descriptor-cell-double-release`); of an undecided name, or through an
    unresolved or unmatched key that may name a recorded cell, refuses
    (`descriptor-cell-undecided`); with no cell this function records it may
    name, it is the identity on custody, under the rule premise
    `P-CELL-STATE-NOT-FORWARDED`, which every declaration carrying a cell
    state publishes with its rows (a tracking limit, not a closed-world fact).
    (5) **The member-equality guard** is equal only when both sides name the
    same live identity and distinct only when they name two live ones (two
    open descriptors never share a number); otherwise the call refuses
    (`descriptor-cell-guard-undecided`) and a guarded removal keeps its record
    and poisons what it may name (rule 12t). (6) **Exits and owners**: a name
    that may be live at an exit refuses (`descriptor-cell-escapes`), and so
    does the release of an object holding one
    (`descriptor-cell-owner-released`). The join keeps only what both paths
    agree on (a name live on one path only is `Maybe`); witnesses join by
    intersection. With `38` in `QCORE_GATEWAY_RULES_OFF` every call of a
    cell-bearing declaration refuses.

    **Do not** read a cell from an actual of another type (beyond the one
    initial-member level of (1)), decide a class
    without its witness, release through a key that may name a recorded cell
    it does not match, or present any refusal here as a finding. Premises
    stated, not proved (trust boundary 28): the declarations' bodies
    (declared assumptions, pinned at both revisions), that two open
    descriptors never share a number, and `P-CELL-STATE-NOT-FORWARDED`. The
    Lean model is `QCore.DescriptorCells`
    ([report](../reports/mk-event-descriptor-quant/README.md); the initial
    member: [report](../reports/mk-event-first-member-cell/README.md)).

<a id="constraint-38a"></a>

38a. **Rule 38 is reached through a constant-index integer element.** Card
    MK-EVENT-CELL-ARRAY-SLOTS (rule 38's keys, §C04.6.2; no clause spelling
    added). Rule 38 keys a slot `&X->f[K]` and its by-value read `X->f[K]`.
    Before this card, the access gate refused such an actual at the call
    before rule 38 was asked. Fluent Bit's channel operations
    (`&sched->ch_events[0]`, `config->ch_self_events[0]`) therefore kept a gate
    refusal.

    The rule has two parts, both in `legacy_call_argument_access.rs`. Each
    applies only when the contract in force is a withheld declaration (rule
    12t) that carries a descriptor-cell state.

    (1) **The element actual** (`element_read`). It is asked only at a
    `Borrowed` position, never at a data tail, and only after every older
    route refused. Its form is `B -> F [ K ]`, `S . F [ K ]`,
    `& B -> F [ K ]` or `& S . F [ K ]`, where:

    - `B` and `S` are typed exactly as W44-INDIRECT-CALL-ARGUMENT-ACCESS types
      the base of `B -> F` and `S . F`;
    - `F` is declared `INTEGER-TYPE F [ N ]` in the record's single complete
      non-union definition: one dimension, a canonical decimal length, no
      `*`, no `volatile` or `_Atomic`, one declarator in its declaration
      (`integer_array_length`, which reads the unit's tokens after the
      declaration reader's completeness check);
    - `K` is a canonical decimal literal with `K < N` (`canonical_decimal`).

    The actual is handed to W44 as the member read of its base, with the same
    bases and names. So `B`'s typed read, the second obligation, and the
    accusation of a released base apply unchanged. Every other shape keeps
    the gate's refusal:

    - a variable index, a negative or out-of-bounds index, or an octal,
      hexadecimal or suffixed index;
    - a pointer member;
    - a floating, `_Bool`, enumeration or plain-`char` element;
    - a multi-dimensional or computed-length array;
    - a custody-taking position;
    - a function-map callee, or a withheld declaration with no cell state,
      such as `mk_event_add`.

    (2) **The minted name** (`AdmittedArgument::matches`). Rule 12t classifies
    the call again under `qcore_retained__<callee>`. The probe's admitted
    actuals are also found under that name, and only as their own callee.
    Before this card they left the gate only in the outer classification, and
    the inner one refused every actual that no older route exempts. Examples
    are `s->evl` over a base this body binds, and `&c->event` in a function
    that also passes `c` whole to a cell call
    (`MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`).

    Rule 38 then decides the cell. With `38` in `QCORE_GATEWAY_RULES_OFF`,
    an admitted element's call refuses as every cell call does.

    **Amended by rule 38b** (card RETAINING-CALL-MINTED-NAME-ACTUALS): part
    (2), the minted-name match, now applies at every withheld declaration's
    `Borrowed` position, not only at a cell-bearing one. Part (1), the
    element, stays cell-bearing.

    **Do not** admit an element at a declaration that is not cell-bearing, or
    accept a non-canonical index. Rule 38 keys the index as spelled, so `010`
    and `8` would be two keys of one slot. Do not read an element with a
    variable or unproved index as a cell.

    Premises stated, not proved (trust boundary 28):

    - an in-bounds element of an integer array is an integer object inside the
      member (C11 6.5.2.1, 6.5.6p8), so its address holds no pointer;
    - the minted name stands for the same callee and positions.

    The Lean model is `QCore.CellArraySlots`
    ([report](../reports/mk-event-cell-array-slots/README.md)).

<a id="constraint-38b"></a>

38b. **A withheld call's admitted actuals leave the gate in both of rule 12t's
    classifications.** Card RETAINING-CALL-MINTED-NAME-ACTUALS (discharging
    the follow-up of that name, registered by MK-EVENT-CELL-ARRAY-SLOTS). The
    code is in `legacy_call_argument_access.rs` (`minted_admission`,
    `AdmittedArgument::matched`, `minted_positions_borrowed`).

    Rule 12t classifies a withheld call a second time, under
    `qcore_retained__<callee>`. W44-INDIRECT-CALL-ARGUMENT-ACCESS's probe
    keyed its admitted actuals by the plain callee. Rule 38a matched the
    minted name only at a cell-bearing declaration. So at every other
    withheld declaration, an actual the probe admitted left the gate in the
    outer classification and was refused again in the inner one, unless an
    older route exempted it there. Examples are `mk_event_add(loop, c->ch[0],
    …)` over a record this body allocates, and `in_tail_init`'s
    `flb_input_set_collector_event(in, cb, ctx->ch_manager[0], config)`.

    (1) **The match.** An entry is also found under the minted name when
    `minted_admission` holds, and then only as its own callee, at the same
    index and with the same tokens:

    - at a cell-bearing declaration, every admitted actual (rule 38a,
      unchanged);
    - at any other withheld declaration, an actual admitted at a declared
      `Borrowed` position that is not a rule-1 release
      (`member_release::admits_release`).

    A withheld declaration never has a variadic data tail (the loader refuses
    one beside a retention clause). A function-map callee is never classified
    under a minted name, so its entries are never minted. The minted name is
    matched rather than the call classified under the plain name: rule 12t
    reaches its route as the classifier's uncontracted fallback, so the
    plain name has no contract in force, and the custody contract installed
    under the minted name differs from the declaration only in its result
    (`custody_contract`).

    (2) **The position in force.** Hook (a) records that an actual left the
    gate under the minted name (`minted_used`). The probe then admits the
    call only if each such actual sits at a `Borrowed` position of the
    classified call (`minted_positions_borrowed`). Rule 12t's route reads a
    callback's requirement as `resource_owned` on the call's own position
    after the inner classification (rule 13u). That is not a read, so the
    call keeps the gate's refusal.

    Everything else keeps the refusal byte for byte:

    - a custody-taking position or a rule-1 release at a withheld
      declaration;
    - an uncontracted callee;
    - an actual the probe refused;
    - a statement whose re-classification is not a contracted call, so
      wherever rule 12t's route itself refuses (an undecided retention
      spelling, a registry holder the body does not name, an open callback
      set).

    Rule 38a's element admission (`element_read`) stays limited to
    cell-bearing declarations. A by-value element `X->f[K]` that W44 already
    reads as a typed scalar (at `mk_event_add`'s `fd`) is admitted as that
    scalar, never as a cell.

    **Do not** match the minted name of another callee, admit under the
    minted name what the probe did not admit under the plain one, mint a
    custody position or a release, or let an admitted actual stand at a
    position the call in force reads as custody.

    Premises stated, not proved (trust boundary 28):

    - the reserved prefix is escaped in source, so a plain name is never a
      minted one;
    - the minted name stands for the same call, actuals and declared
      positions.

    The Lean model is `QCore.RetainingCallMintedName`
    ([report](../../reports/retaining-call-minted-name-actuals/README.md)).

<a id="constraint-38c"></a>

38c. **A declared cell protocol carries a record's cells across a function
    boundary, and only an exact reading of it decides.** Card
    MK-EVENT-CELL-CROSS-FUNCTION (contract specification
    [§C04.6.3](../QCORE_CONTRACT_SPEC_r0_3.md#c0463-declared-cell-protocols),
    grammar G05i; USER DECISION 2026-11-02, verbatim: "B as recommended",
    accepting the proposal's Q1 B, Q2 the spelling, Q3 yes and Q4 no). The
    package model is `crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs`
    (`Fact`, `PROTOCOL_VECTORS`). The host is `legacy_descriptor_cells.rs`
    (`ProtocolStep`, `refine_protocol`, `apply_step`, `touch`,
    `join_protocols`). The parser hooks are the `if` arm (`cell_test`, read
    on a desugared plan's one deciding node) and `expression_statement`
    (a `CellProtocolStep` node after the statement's own lowering).

    (1) **The claim.** `quant cell #B: TAG.CELL |-> #fresh(descriptor) when
    #B.GUARD OP V;` (and `#same(CELL2)`) states that whenever the guard
    holds, each `#fresh` cell names its own live descriptor and each `#same`
    cell its target's, and that otherwise no cell names a live one. It is a
    declared assumption (rule 13i, premise `P-CELL-PROTOCOL-DECLARED`), never
    a checked closure: every row that reads it depends on the declarations
    stating it (`contract_dependencies::relied`). Two declarations that
    disagree leave the record without a protocol.
    (2) **The test.** A branch condition that is exactly `r->GUARD OP V`, or
    its complement (`== -1`, `<= 0`, `!= 1`), where `r` is declared `struct
    TAG *`, is read only where the path holds no fact of `r`'s object. The
    live edge introduces the caller's identities: one per `#fresh` cell, a
    `#same` cell sharing its target's. They stay pending until an operation
    reads the cell, and are then adopted into rule 38's table. The other edge
    records that no cell is live, except a `> 0` guard's: it holds descriptor
    0, so its cells stay undecided (rule [38f](#constraint-38f), card
    CELL-PROTOCOL-PER-END-GUARDS, which also adds per-end sentinels beside
    the one record guard, one fact per guard, and the holder's release).
    (3) **Stores and closes.**
    - A store `r->PATH = V` to the guard or a cell never overwrites a cell
      that may be live (`descriptor-cell-overwritten`).
    - A sentinel guard (`!= -1`, `> 0`) is admitted only at its dead value.
    - A flag (`== 1`) is admitted at `1` only over live cells, and at another
      value only over none.
    - Any other value refuses (`descriptor-cell-protocol-broken`).
    - A creation into a protocol cell is admitted only where the guard was
      read dead or the cell is recorded. A creation never establishes `> 0`
      or a flag.
    - `close(r->CELL)` of a one-segment cell, on a caller's record with a
      fact, is that cell's `#stale` (Q3).
    (4) **Exits and calls.** At an exit a record keeps its protocol when it
    is untouched, or every cell is live under its identity and the guard
    says live (a `!= -1` guard whenever its cell is live), or no cell is live
    and the guard says dead. Its cells are then forwarded under the protocol
    and do not escape. At a call that may reach the record (other than a
    cell-bearing one), a record that keeps it is forgotten, to be read again
    from the protocol. A record that does not keep it refuses
    (`descriptor-cell-protocol-broken`) and its live cells still escape. A
    join keeps the protocol where both paths kept it; otherwise the guard
    joins to a conflict.

    **Do not** read a test of another shape, a test over a record the path
    already holds a fact of, a copied number (Q4), a multi-segment close or a
    close on a tracked record, or decide a guard from a protocol a call may
    have changed. With `38` in `QCORE_GATEWAY_RULES_OFF` nothing here is
    read. Nothing here is `Unsafe`. The access gate refuses an integer
    member at `close`'s custody position (W44 premise 1) except the close's
    own cell actual, which rule [38d](#constraint-38d) admits for this
    close. Premises stated, not proved (trust boundary 28):
    the declared protocol at every store this rule does not read (other
    units, other threads, writes through other names or by uncontracted
    callees), and those of rule 38. The Lean model is
    `QCore.DescriptorCellProtocol`
    ([report](../reports/mk-event-cell-cross-function/README.md)).

<a id="constraint-38d"></a>

38d. **A declared cell's close actual leaves the access gate only for rule
    38c's close.** Card CELL-PROTOCOL-CLOSE-ACTUAL (registered by
    MK-EVENT-CELL-CROSS-FUNCTION; USER DECISION 2026-11-02, verbatim: "B as
    recommended", whose Q3 reads a close of a declared cell member as that
    cell's `#stale`). The decisions are
    `crates/gateway-rules/rule-38-descriptor-cells/src/close_actual.rs`
    (`admits`, `reading`, `ADMISSION_VECTORS`, `READING_VECTORS`); the host is
    `legacy_call_argument_access.rs` (`cell_close_actual`, `cell_read`,
    `integer_member`), asked from `legacy_c.rs`'s `expression_statement` after
    the statement's own lowering, and rule 38c's close step in
    `legacy_descriptor_cells.rs` (`ProtocolStep::Close { admitted }`).

    (1) **The admission.** Only for a statement whose own lowering is exactly
    the gate's refusal (`indirect memory access is outside the resource
    projection`) and that reads no tracked descriptor (W44 premise 2), and
    only when all of these hold: rule 38 is in force; the statement is rule
    38c's close step, `close(r->CELL)` with `r` declared `struct TAG *`, `TAG`
    a record whose declared protocol names the one-segment `CELL`; the
    callee's contract in force is a function-map contract whose one actual
    sits at a `resource_released` position of kind `descriptor`; and the
    actual is exactly `r->F` with `F` an integer member, or `r->F[K]` with
    `F` an integer array member and `K` a canonical constant below its length
    (rule 38a's element read), of `TAG`'s single complete definition without
    nested definitions. The statement is classified again with that actual
    admitted and must be the plain contracted call to the callee, with no
    destination and no nested call.
    (2) **The lowering.** The actual's typed read of `r` and its second
    obligation (W44 premises 3 and 4, as every admitted actual), the call
    with its released position read as `Borrowed` (the number read is no
    identity the projection tracks, as rule 1 reads a member release), and
    rule 38c's close of the cell marked admitted. The close is the
    statement's whole custody effect.
    (3) **The reading.** On a path holding a fact of `r`'s caller object, a
    cell naming a live single identity is released once through rule 38's
    table and the record's pending names of it are staled (Q3; `#same` names
    with it). A stale cell refuses (`descriptor-cell-double-release`), and a
    cell that is maybe live, live under a join's identity or not recorded
    (the guard's dead edge) refuses (`descriptor-cell-undecided`). Where the
    path holds no fact of a caller's record (no guard test, a call that
    forwarded the record, a tracked record, a released formal), rule 38c
    reads nothing and the gate's refusal stands. With `38` in
    `QCORE_GATEWAY_RULES_OFF` nothing is admitted, and an admitted close
    would refuse at the gate.

    **Do not** admit an undeclared record, a member that is not a declared
    cell, a multi-segment cell (`event.fd`), a copied number (Q4), a used
    result (`r = close(…)`, `return close(…)`), a parenthesised or computed
    actual, a non-release custody position, a non-descriptor kind, a
    withheld or uncontracted callee (Windows `closesocket` has no contract),
    or any statement whose lowering is not exactly the gate's refusal. The
    read end's registration is left to the loop's destroy (rule 12t), as Q3
    states. Nothing here is `Unsafe`. Premises stated, not proved (trust
    boundary 28): rule 38c's declared protocol (`P-CELL-PROTOCOL-DECLARED`)
    and the callee's declared release of a descriptor. The Lean model is
    `QCore.CellProtocolCloseActual`
    ([report](../reports/cell-protocol-close-actual/README.md)).

<a id="constraint-38e"></a>

38e. **A result set decides a cell's class only where it is derived, in
    force and read exactly.** Card CELL-GUARD-RESULT-RANGE-CHECKED (contract
    specification [C09.4](../QCORE_CONTRACT_SPEC_r0_3.md#c094-declared-c-library-result-sets),
    grammar G05j; USER DECISIONS 2026-11-03, verbatim: "Yes, as recommended",
    "This should be needed to use here.", "C: a new result-set form that
    allows unions, such as result_set(-1 | [0, MAX]). … I choose this because
    we must not loosen the custody of the contracts if we cannot select other
    options.", "Also, we need to use mathematical symbols on them and we can
    use XOR or AND or OR with the normal symbols of programming languages.").
    The set algebra and the class decision are `qcore_core::result_set`
    (`SetExpr::eval`, `test_class`, `SET_VECTORS`, `CLASS_VECTORS`); the
    derivation is `qcore_c_gateway::null_deref::derive_result_set_c11`
    (`null_deref/range.rs`, `Deriver`); the pin is `contract_pin.rs`
    (`--derive-results`, `derive_results`) with the lock's `derivation`
    record (`contract_revision.rs`: `ResultDerivation`, `with_helper_sites`);
    the route's standing is `bounds_selection.rs` (`result_ranges_of`,
    `stale_rest`, `result_range_table`), carried by the resource environment
    (`ResourceContracts::result_ranges`); the reading is
    `legacy_descriptor_cells.rs` (`compared_class`, `EdgeFact::Compared`) and
    `LegacyInstruction::BranchOnResult` (`descriptor_custody::result_test`).

    (1) **The C library's sets.** `ensures normal result_set(SET);` states a
    value set only and is admitted only beside `requires
    bounds_revision(flexible);` on a declaration with nothing else, from
    `quirin-provider-libc` only (held elsewhere): rule 12r's one flexible
    bounds declaration, listed in its lock, under `libc-c-standard-abi`. It
    binds a call only where the unit defines no function of that name and the
    call's C type is the declared result type. No custody statement is added
    or changed to host one (`pipe` stays undeclared for custody).
    (2) **The derivation.** At pin time the declaration's one defining file is
    read from the configured compile database through Clang; every `return`'s
    set is a constant, an in-unit helper's derived set, a C library set, or a
    plain local's union of assignments (address never taken, C11 6.3.2.1p2),
    else the type's range. The derived set must lie in the declared range or
    nothing is pinned; the lock records it, the helpers (whose sites join
    every identity) and each C library declaration with its digest. A re-pin
    of a derived entry without `--derive-results` refuses; a changed
    derivation voids earlier identities. At audit time a derived range is in
    force only with its pin and with every C library set it rested on loaded,
    listed and unchanged (otherwise held, `contract-changed`); a pinned range
    the lock never derived is `unchecked`.
    (3) **The reading.** For a creation whose `#fresh` class is a status guard
    (`#result == 0` / `!= 0`) and whose callee's derived set `S` is in force,
    a branch that is exactly `ret OP K` (or `K OP ret`, `K` a literal; a null
    test keeps its own reading) decides the class on each edge exactly when,
    on `S`, the test holds where the class holds (or where it fails); a test
    that misclassifies a value of `S`, or compares with a constant `S`
    excludes, decides nothing. A set out of force is never read: the reading
    is then the universe's, where only an exact-on-every-`int` test decides
    (Lean `out_of_force_reads_the_universe`).

    **Do not** read a result set as custody, host one on a custody clause,
    read a pinned range rule 38 did not see derived, decide a class from a set
    that is held, unchecked or stale, read a caller's test as evidence of the
    result set (the set comes from bodies and declared C library contracts),
    or present any undecided reading as a finding. Premises stated, not proved
    (trust boundary 28): the C library's documented result sets
    (`libc-c-standard-abi`), Clang's AST of the configured unit, and the
    lexical site scanner's identity of the helpers. The Lean model is
    `QCore.ResultSet` ([report](../reports/cell-guard-result-range-checked/README.md)).

    **Amended (card CELL-RESULT-SET-ALIGNMENT, USER DECISIONS 2026-11-04,
    Q1 (a) with its refinements; "Write [1, MAX]").** A C library declaration
    may state, beside its `result_set`, `ensures normal written_set(SLOT[K],
    SET) when #result == C;` (G05j as amended): on a return of `C` the `int`
    stored at `SLOT[K]` is in `SET`. `pipe` states `written_set(pipedes[0|1],
    [1, MAX]) when #result == 0`, a declared policy, not POSIX's set. On a
    pinned declaration the clause is read only where `pin --bounds
    --derive-results` derived it, and the route carries it as
    `FUNCTION#SLOT` in the result table; rule 38f's live-set creation check
    reads it there.

    **Amended (card CELL-RESULT-SET-WRITTEN-SET-DERIVATION; the user
    launched it on 2026-11-04, verbatim: "CELL-RESULT-SET-WRITTEN-SET-DERIVATION.").**
    The slot derivation is `qcore_c_gateway::null_deref::derive_written_sets_c11`
    (`null_deref/written.rs`, `WrittenDeriver`), with its C-free rule in
    `qcore_core::written_set` (`combine`, `narrow`, `resolve`, `after_store`).
    It runs path by path over the configured unit's AST. A slot set comes
    from forwarding `return G(.., P, ..)` (`G`'s written set of that position
    for the class), or from a store `*P = A[k]` after `ret = G(.., A, ..)`
    once a test narrows `ret`, within `G`'s result set, to exactly one class
    (`G`'s written set of that class). A `return` whose narrowed set excludes
    the class constrains nothing. A path that may return the class with the
    slot unwritten or unknown derives nothing.

    The pin reads every configuration of the defining file, and each derived
    set must lie in the declared one. The lock's derivation records them
    (`written_sets`). The route keys an in-force one `FUNCTION#P[K]==C`
    (`qcore_schemas::result_range::written_set_key`), and rule 38f reads only
    the class its `#fresh` state holds in.

    Pinned: `mk_event_channel_create`'s `r_fd[0]`/`w_fd[0]` and
    `flb_pipe_create`'s `pipefd[0..1]`, `[1, MAX]` when `#result == 0`, at
    66910c10, 34155955 and 046accf6. Each rests on `pipe`'s DECLARED
    `[1, MAX]`, not POSIX's `[0, MAX]`, under which the chain derives
    `[0, MAX]` and nothing is pinned (follow-up
    CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY).

    **Do not** read a pinned written set the lock did not derive, read one
    of another class, borrow another configuration's or another tree's body,
    or narrow a local whose address is taken. Premises stated, not proved
    (trust boundary 28): Clang's AST of each configuration, and the C library
    declarations' documented written sets: a call writes its out array and
    keeps no pointer to it. The Lean model is `QCore.WrittenSetDerivation`
    ([report](../reports/cell-result-set-written-set-derivation/README.md)).

<a id="constraint-38f"></a>

38f. **Each end is its own guard, and descriptor 0 is never read as an
    absent end.** Card CELL-PROTOCOL-PER-END-GUARDS (registered by
    CELL-PROTOCOL-CLOSE-ACTUAL; contract specification
    [§C04.6.3](../QCORE_CONTRACT_SPEC_r0_3.md#c0463-declared-cell-protocols),
    grammar G05i as amended). USER DECISION 2026-11-03 (verbatim, on "a
    sentinel on each channel end, or several protocols per record"):
    "Per-end sentinels". USER DIRECTION 2026-11-03 (standing, verbatim): "The
    Fluent Bit's caller tests sometimes check the incorrect assertions so we
    need to pay attention for the assertions and actual returning values of
    Fluent Bit functions." The package model is
    `crates/gateway-rules/rule-38-descriptor-cells/src/protocol.rs`
    (`Guard::dead_edge_decides`, `Protocol::per_end`, `Fact::introduced`,
    `Fact::join`, `Fact::normalize`, `Fact::release_holder`, `PER_END_VECTORS`)
    and `cells.rs` (`Cells::join_keeping_stale`); the host is
    `legacy_descriptor_cells.rs` (`FactKey`, `apply`, `touch`, `join_cells`,
    `refuse_release_of_holder`); the loader is `local_condition.rs`
    (`cell_protocols`) and the table `ResourceContracts::cell_protocols`.

    (1) **Per-end sentinels.** A record's `quant cell` clauses are grouped by
    guard. A per-end sentinel is a `!= -1` or `> 0` guard over exactly one
    `#fresh` cell, that cell itself (`when #B.END > 0` on the end's own
    clause), with its `#same` names under the same guard. A record states at
    most one other guard (rule 38c's record guard: a flag, or a sentinel over
    several cells); every other guard is a per-end sentinel. This replaces
    "one guard per record", not "one protocol per record": the record's
    groups load and are compared as one value, and two declarations stating
    different groups leave the record without a protocol.
    (2) **One fact per guard.** A path holds a fact per (record object,
    guard). A test of exactly `r->G OP V` (or its complement) is read for the
    group whose guard is `G` where the path holds no fact of that group;
    stores, closes, exits and calls read the group whose cell or guard they
    name. At a cell-bearing call a per-end group gains a fact only through a
    cell of its own the call spells (a cell-bearing call's effect is its
    declared states); a record guard keeps rule 38c's reading (every actual
    rooted at the record; follow-up CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD).
    (3) **The `> 0` dead edge.** A sentinel's dead edge decides that no cell of
    it is live only when no possible descriptor is on it (rule 38's premise:
    a live descriptor is never `-1`). `!= -1`'s dead edge holds only `-1` and
    decides; `> 0`'s holds descriptor 0, a success value of every creator and
    calloc's absent value at once, so it never decides: its cells are
    introduced undecided (`Maybe`, the fact marked `zero`). Its live edge
    decides, since the dead set holds no value above 0. A close there refuses
    (`descriptor-cell-undecided`), a store over such a cell refuses
    (`descriptor-cell-overwritten`), and a touched record holding one does not
    keep its protocol at an exit, its cells escaping. Rule 38c's `> 0`
    record guard reads the same.
    (4) **Joins.** A join never drops a name that may be live: each path's
    pending names whose cell either path recorded go into its own table first
    (`Fact::normalize`); a pending name live or undecided on one path is
    undecided in the join; and a protocol cell both paths left stale stays
    stale whatever identities they name (`join_keeping_stale`). Two holding
    paths join untouched, keeping their undecided names.
    (5) **The holder's release ends the protocol.** A call that releases the
    record (an actual naming it at a `resource_released` position) ends every
    fact of it and forgets its cells. Where some cell may still be live the
    release refuses: `descriptor-cell-zero-undecided` when a `> 0` dead edge
    left it undecided (the end may be descriptor 0, a candidate defect only,
    never `Unsafe`, nothing filed upstream), else
    `descriptor-cell-owner-released`; a tracked holder's release refuses as
    before (rule 38 (6)). Between a close that stores no sentinel and that
    release, a call that may reach the record refuses
    (`descriptor-cell-protocol-broken`): the closed end still looks live,
    unless its callee's frame over the record's type stands (rule
    [38g](#constraint-38g), card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES).

    **Do not** read a destroyer's `> 0` (or `<= 0`) test as the end's absence,
    take a sentinel's meaning from the test a caller writes rather than from
    the creator's result set and the absent value the stores write, give a
    per-end group a fact through an actual that names none of its cells, drop
    a possibly live name at a join, end a protocol at a release while a cell
    may be live without refusing, or present any refusal here as a finding.
    With `38` in `QCORE_GATEWAY_RULES_OFF` nothing here is read. Premises
    stated, not proved (trust boundary 28): rule 38c's declared protocol
    (`P-CELL-PROTOCOL-DECLARED`) and rule 38's premise that a live descriptor
    is never `-1`. The Lean model is `QCore.CellProtocolPerEnd`
    ([report](../reports/cell-protocol-per-end-guards/README.md)).

    (6) **Declared value sets** (card CELL-RESULT-SET-ALIGNMENT; USER
    DECISIONS 2026-11-04, verbatim: "Basically, Q1(a) and Q2(i) But we must
    not handle descriptor 0 as a valid handle. So, we need to exclude the 0
    descriptor like: written_set(pipedes, [1, MAX]); a live descriptor is in
    [1, MAX]", "Write [1, MAX]", Q3 "(α) live_set suffix", "Accept the
    leak"). A per-end sentinel may state `live_set(SET)` and `absent_set(SET)`
    (G05i as amended). One reading serves every guard
    (`Guard::dead_edge_reads_absent`, `Guard::live_edge_reads_live`,
    `Protocol::dead_edge_decides`, `Protocol::live_edge_decides`): a dead edge
    decides "absent" only when its values meet the live set nowhere (and,
    over the cell's values live | absent, lie inside the absent set), a live
    edge decides "live" only when its values meet the absent set nowhere;
    otherwise the edge introduces its names undecided. Without a declared
    live set the live set is rule 38's premise (every `int` but `-1`), without
    a declared absent set the guard's dead set, which is (3) case for case.
    A store to the sentinel is admitted only at a value of the absent set; the
    guard follows its cells (a creation establishes it) only where the live
    set lies inside its live edge; an undecided edge names its release
    `descriptor-cell-zero-undecided` when 0 is the value it leaves open, else
    `descriptor-cell-edge-undecided`. A creation into a cell with a declared
    live set is admitted only where the creator's pin-derived written set of
    that slot (rule 38e as amended) is in force inside it, else
    `descriptor-cell-live-set-unproved`. The written set read is the one of
    the class the cell's `#fresh` state holds in (`when #result == C`; card
    CELL-RESULT-SET-WRITTEN-SET-DERIVATION). A state under `!= C`, or under
    every return, reads none. The production ends declare
    `live_set([1, MAX])`: a DECLARED POLICY, not POSIX's `[0, MAX]`
    (descriptor 0 is observed natively on `ch_data`), under which a `> 0` dead
    edge decides "absent"; every reading it moves is attributed to that bound
    (follow-up CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY). The Lean model is
    `QCore.CellResultSet`
    ([report](../reports/cell-result-set-alignment/README.md)).

<a id="constraint-38g"></a>

38g. **A call in a record's release window passes only under the callee's
    frame.** Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (registered by
    CELL-PROTOCOL-PER-END-GUARDS; contract specification
    [§C04.6.4](../QCORE_CONTRACT_SPEC_r0_3.md#c0464-callee-frames-over-a-declared-cell-protocol),
    grammar G05k). USER DECISION 2026-11-04 (verbatim, on the options (a)
    callee frame declarations, (b) a member-actual narrowing, (c) keep the
    refusal): "(a) Callee frames". The package model is
    `crates/gateway-rules/rule-38-descriptor-cells/src/frame.rs`
    (`body_check`, `standing`, `window`, `protocol_members`, `BODY_VECTORS`,
    `WINDOW_VECTORS`); the host is `legacy_cell_frames.rs` (`settle_frames`,
    `read_body`, `call_standing`, `frame_hold_description`,
    `frame_held_call`) and rule 38c's `touch` in `legacy_descriptor_cells.rs`;
    the loader is `local_condition.rs` (`frame`, `cell_frames`) and the table
    `ResourceContracts::cell_frames`.

    (1) **The clause.** `quant cell CALLEE: TAG |-> #untouched;` stands
    beside `TAG`'s protocol on the declaration stating it. It states that
    `CALLEE`, with everything it calls, neither reads nor writes any member a
    declared protocol of `struct TAG` names (the first member of each cell
    and each guard: `ch_events`, `event`, a flag), on every object of that
    type, however the callee reaches it. It is a frame over the record's
    type, never over the call's actuals: a callee handed `&ins->_head`
    recovers `ins` by `container_of` (option (b)'s unsoundness), and the
    clause still covers it. "Reads no end member" is read with writes
    included, the stronger claim (fail-closed). A record keeps its frames
    only where it keeps its protocol and every declaration stating that
    protocol states the same callees.
    (2) **Its standing in a unit.** After the parser builds a body's CFG,
    each declared frame of a contracted callee the body calls (nested calls
    included) is checked against Clang's AST of the unit: **checked** where
    the unit defines the callee's body and every body it calls, none names a
    protocol member of `TAG`, and none uses a construct the reader does not
    read (a call through a pointer or out of the unit, recursion, a pointer
    reinterpreted as anything but a record or `void` pointer, an integer read
    as a pointer, a whole-record read or store, a scalar read through a
    pointer, `offsetof`, inline assembly, `__builtin_bit_cast`, atomic
    builtins); **refuted** where a body reached names a protocol member;
    otherwise **declared**. The standing travels on the CFG, so certificate
    rederivation reproduces it.
    (3) **Its reading.** Only where a call that may reach a record (rule 38c
    (4)) would refuse because the record does not keep its protocol (rule
    38f (5)'s window: an end closed, its sentinel not stored), and the call
    does not release the record: the call's standing is the weakest of its
    callees' (the call's own and its nested calls'). Checked: the record's
    fact passes the call unchanged. Declared: the fact passes and the call is
    held for review, ``call to `CALLEE` reaching a `struct TAG` that does not
    keep its cell protocol rests on its declared frame, which awaits human
    review (awaiting-human-review)``, a held call (`Pending`, never `Safe`;
    rule 10i, C06.1); the analysis with every held declaration in force
    (`ResourceContracts::with_human_holds_in_force`) reads it accepted, so a
    `Pending` row with another reason is `Unknown`. Refuted: the call refuses
    (`descriptor-cell-frame-refuted`). No frame: the call refuses
    `descriptor-cell-protocol-broken` as before. A record that keeps its
    protocol at the call is read as rule 38c reads it, whatever its frames.

    **Do not** read a frame from the call's actuals (a member actual such as
    `ins->processor` or `&ins->_head` reaching no end), take a frame from a
    body the unit does not analyse as checked, lift a refutation by
    acceptance, read a frame where the record keeps its protocol, or present
    any refusal or hold here as a finding. With `38` in
    `QCORE_GATEWAY_RULES_OFF` nothing here is read. Nothing here is `Unsafe`.
    Premises stated, not proved (trust boundary 28): rule 38c's declared
    protocol (`P-CELL-PROTOCOL-DECLARED`); Clang's AST of the configured unit;
    C11 6.5p7 (an `int` cell is accessed only through an lvalue naming its
    member, a whole-record access or a reinterpreted pointer) and 6.5.6p8
    (pointer arithmetic stays inside the object it points into). A declared
    frame is a deferred-review contract, never a premise. The Lean model is
    `QCore.CellProtocolReleaseWindowFrames`
    ([report](../reports/cell-protocol-release-window-frames/README.md)).

<a id="constraint-38h"></a>

38h. **An uninitialized declaration is read from the Clang AST only where the
    lexical gate cannot see that it is one.** Card MEMBER-ACCESS-PROJECTION
    (`typed_declaration.rs`, the hook `legacy_typed_declaration.rs`, called
    at the gate's two declaration-less arms in `legacy_classify.rs`). The
    gate admits a statement with no `=` whose first token is a keyword
    specifier and whose bracket bounds are decimal literals: such a
    declaration allocates automatic storage and evaluates nothing, and the
    classifier's fall-through lowers it to `NoOp`. Spelled with a typedef
    specifier (`rd_segment_t *seg, *tmp;`, `uint32_t op[32];`) or a
    constant-expression bound (`char buf[1024 * 3];`) the same declaration
    reached the token test, which reads the `*` after a comma as a
    dereference and a bracket as a subscript, and was refused as an indirect
    memory access (141 leaves on the 046accf6 `current` run). Where the old
    gate refuses a statement its own declaration-prefix test accepts, the
    statement's unique Clang node (by line and exact tokens, several told
    apart by occurrence, as rule 10t) is asked for five facts: a `DeclStmt`;
    only `VarDecl`s, at least one; no initializer (no `init`, no inner node);
    no storage class or thread-local marker; every array extent of every
    declared type, as written and desugared, a decimal constant (Clang prints
    a variable-length array's size expression and a constant array's
    evaluated size). On all five the statement is admitted and lowered
    exactly as the keyword spelling is; on any other answer, or no unique
    node, the refusal stays byte for byte. **Do not** read an initialized
    declarator here (`char *p = NULL, *q = NULL;` stays with
    W46-LEXICAL-GATE-NON-ACCESS and CG14-DECLARATION-FORMS), a `static`,
    `extern`, `register` or thread-local object, a variably modified type, a
    declaration of anything but objects,
    or a statement the old gate admitted. Premises stated, not proved:
    faithful Clang extraction of declarations, storage classes and types
    (trust boundary 28); C11 6.2.4p6, 6.7.9p10 and 6.8p3 (an automatic object
    with no initializer is indeterminate and its declaration evaluates
    nothing unless its type is variably modified). The `NoOp` keeps a
    redeclared name's earlier binding, as the keyword branch always has; the
    loop re-acquisition and alias refusals still govern such a name
    (`member_access_projection.rs` pins one). `QCore.TypedDeclarationReading`
    (`vectors_match`, `an_old_admission_stays`,
    `no_ast_answer_keeps_the_refusal`, `a_new_admission_needs_every_fact`, the
    four keeping theorems, the `NoOp` lowering);
    [report](../reports/member-access-projection/README.md).

<a id="constraint-38i"></a>

38i. **The gate's refusal of member custody through a formal parent names
    its premise: a re-attribution, never an admission.** Card
    MEMBER-EFFECT-SUMMARY step 1 (`legacy_member_custody_attribution.rs`; one
    hook in `legacy_c.rs::expression_statement`, after rule 38d's close
    admission, the last consultation of a statement), on the user's decision
    of 2026-11-05 ("Option 1 (rec.)": option 3's re-attribution first).
    MEMBER-ACCESS-PROJECTION's census found the sole-blocker mass of
    `indirect memory access is outside the resource projection` to be
    custody through a formal's members, which the leaf did not name. For
    exactly the gate's refusal of a statement with no braces that reads no
    tracked descriptor, whose base `P` is a formal no statement of the body
    writes or takes the address of (`CallerFormals`) and whose declarator
    does not spell `union` (the census's `form:union`): a whole-statement
    call `R(..)` whose contract in force (no review or pin hold) states
    `Released` at the position of an actual spelled exactly `P->F` takes `release of `P->F`, a member of the
    formal `P` (member-release-through-formal)`, and one spelled
    `&P->F[.G…]` takes `… (member-address-release-through-formal)`, each
    only when the statement classified again with every memory-reading actual
    read as at a `Borrowed` position (rule 12k's reading) **admits** the
    call; `P->F = V` with `V` a plain name the lexical reader declares a
    pointer, or a whole call whose actuals read no memory and whose contract
    in force (no hold) returns `Owned` or `Reallocated` custody, takes `store
    of a custody-carrying value into `P->F`, a member of the formal `P`
    (member-store-through-formal)`. Rule 5k is unchanged (a member release is
    admitted only when the body also releases the parent, P4). **Do not** let
    the reading lower anything, name a held call (its refusal is `Pending`),
    a second released member actual, a rebound or non-formal base, a store
    whose value reads memory, or ask the rule before any older route
    (hard invariant 3). The node stays `Unknown` with the same writes, so no
    verdict moves; parity runs record each consultation (`position:
    "member-custody"`). Twelve gateway pins that asserted the gate's text
    on such a statement were flipped deliberately to the name, each still
    `Unknown` (report section 1). `QCore.MemberCustodyAttribution`
    (`the_verdict_of_every_node_is_unchanged`,
    `the_status_of_a_function_is_unchanged`, `the_writes_are_kept`,
    `a_name_needs_every_premise`, `a_refusing_reading_keeps_a_call_shape_unnamed`,
    `a_store_needs_a_custody_value` and eleven more), bound by
    `tests/member_effect_summary.rs`;
    [report](../reports/member-effect-summary/README.md).

<a id="constraint-38j"></a>

38j. **A member release through a formal the body keeps is a member effect:
    checked like rule 5k, stated beside the contract, and read only where the
    caller's own check reads it.** Card MEMBER-EFFECT-SUMMARY step 2
    (`legacy_member_effect.rs`; schema `qcore_schemas::member_effect`,
    `ResourceContracts::{member_effect, member_effects,
    insert_member_effect}`; hooks: `BodyReading::read` beside rule 5k's
    `AdmissibleReleases` in `build_function_cfg_with`, whose release places
    join rule 1's; `minted_environment` after rule 12t's retaining
    environment; `Parser::member_effect_call` last in
    `expression_statement_lowered`; `published_effect` in
    `derive_custody_summaries`; the minted prefix `qcore_member_effect__` in
    `reserved_spelling` and `contract_dependencies`), on the user's decision
    of 2026-11-05 ("Option 1 (rec.)"). Over a body without `goto`/`setjmp`,
    per formal `B`: a release event is `B->F` whole at a `Released {
    nullable: true }` position; a call event is `B`, or `&B->F`, whole at a
    position whose member-effect summary is in force (an event on each named
    member, the address form on `F` with paths `F.G`). **P1** every other
    occurrence of `B` is `B->G`, a null test of `B`, or `R(B)` with `R`
    releasing its only parameter; **P2** each place has one event, its other
    occurrences null tests before it or null-constant stores after it;
    **P3** no loop around the event; each call event's callee is
    characterised (rule 5k's W45-SR5 reading) and, for a whole-base actual,
    touches no other place this body has an event on. With an `R(B)` every
    event needs rule 5k's P4 and an `R` that does not touch the place, and
    nothing is published; with none the body keeps `B` and states, at `B`'s
    position, each released path `stale`, or `cleared` when a null store
    follows the release in its block with no `return`/`break`/`continue`
    between. The effect is a may-effect. A body that states an effect and is
    declared (its source text declares it) states none and keeps every
    refusal (P-B08-SHALLOW). **Where the summary lives:** a body with an
    effect publishes no ordinary W4 summary (no record, no cross-unit export,
    no callback or indirect reading: each sees an uncontracted function); its
    validated contract and effect go to `member_effects` (a singleton
    component only; a recursive one publishes nothing). A whole-statement
    call to such a function, refused as uncontracted, is classified again
    with the callee renamed to its minted name (rule W44's actual reading
    where the gate refuses `&B->F`) and the function renamed back, only when
    this body's check admitted that `(callee, line)`; otherwise it is refused
    `call to `g`, whose summary releases members of an argument, outside the
    caller's member-effect premises (member-effect-call-refused)`. **Do not**
    publish an effect function's contract in the function table, admit a
    `nonnull` member release, a call event in a return, condition or nested
    position, a local (non-formal) parent, the same object at two positions
    of one call, or a callee that reaches a place this body released. Not
    in this rule: the store half of the member effect, a declared form for
    bodiless callees (a user choice, report section 4), typedef-spelled
    bases (W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS). Premises stated, not
    proved: rule 5k's (distinct members own distinct objects; a stored pointer
    is tracked only through a bound place or sub-custody) and the lexical
    reading (trust boundary 28). `QCore.MemberEffectSummary` (`body_sound`:
    every path of a checked body is free of use after release and double
    release of the place; `cascade_sound`: replacing each call event by its
    callee's trace keeps that, given the callee's stated effect, its
    characterised touches and its own `body_sound`;
    `an_ordinary_reader_sees_no_contract`; `two_releases_are_refused`,
    `a_use_after_a_call_is_refused`,
    `a_callee_touching_a_released_place_is_refused` and seven more), bound by
    `tests/member_effect_summary.rs`;
    [report](../reports/member-effect-summary/README.md).

<a id="constraint-39"></a>

39. **A lookup's miss is the NULL event of a null witness only when it is
    feasible on the path by facts the search reads.** Card
    NULL-WITNESS-GUARD-CORRELATION (contract specification C09.3;
    `null_deref/witness.rs`, `qcore_core::null_lookup`). A *lookup* is a call
    whose declared result is `resource_return_borrowed_from(P, nullable)`;
    unlike an allocation's failure, its miss is a function of the state its
    arguments reach. In the path search (rule 14r):
    (1) **tie**: when the unit holds the lookup's definition and its body is
    one `return` of a load of a place built only from its parameters, member
    accesses, `&`, `*` and casts, the result is that place, spelled as the
    caller's own read of it: the miss is named only when the path's own test
    fixed the place NULL (a finding, its condition naming the test); fixed
    non-null, tested before a call or store, or an unconstrained memory
    value (which rule 14r never names) refuses it;
    (2) otherwise the miss is named only when every argument is a constant (a
    literal, an enumeration constant, `sizeof`, a predefined name) or is
    rooted at a formal that no earlier decision, store, assignment or call
    on the path read or wrote through (a NULL test of a pointer root itself
    excepted), or at a global with, in addition, no call before; an argument
    rooted at a local (an arena the function initialised and sized), at a
    call's result or at nothing the search roots is correlated;
    (3) a refused miss is a constrained value: reaching the dereference it
    ends the path **unresolved** with the reason, and a test of it is
    inexact (rule 15r), so a refusal never validates a witness, never proves
    a candidate infeasible and never makes a row `Safe`. The tie is read only
    to name or refuse a miss, never to prove a result non-null.
    **Do not** name a lookup's miss as a NULL condition when a guard the
    path read, a state the path built or an entry memory value decides it.
    `QCore.NullLookupEvent` (`a_named_miss_is_decided_or_free`,
    `a_tied_lookup_is_named_only_when_decided_null`, `touching_never_frees`,
    `one_correlated_argument_correlates`,
    `a_correlated_miss_never_contributes`,
    `a_refused_miss_never_concludes_infeasible`; `missVectors`,
    `stateVectors`).

<!-- generated:rules:end -->
