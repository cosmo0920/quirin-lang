# Gateway rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.
Per-card history compacted out of rules 5c, 7 and 8 at wave 45, and 5l's corrected
sentence, are kept verbatim
in the [wave-45 compaction checkpoint](../development/increments/wave-45-compaction-checkpoints.md#gateway),
which is not a second authority.

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
    **Do not admit an external-linkage definition**: a C99 `inline` definition
    may be bypassed for the external one at any call (C11 6.7.4p7) and an
    external definition may be interposed, so its body need not be the body the
    call runs (`mpack_*`, 86 leaves, stay refused). **Do not drop the reserved
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
    function's own contract in two places: the protected formals (every formal
    it does not declare `Borrowed`) and the descriptor names. The protected set
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

<a id="constraint-5"></a>

<!-- preserved:constraint-5:start -->
5. The typed scalar-field whitelist's last premise — the base is a name this
   function never binds to a resource identity — exists because dropping it
   turned a read through a released pointer into `Safe`. The fail-closed pin
   caught it and **was not weakened**; keep it that way. `remaining-c02`.
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
    condition is desugared only when an operand needed this reader; one
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
    undeclared (rule 10j). **Do not** apply a loop-owner declaration on a
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
    paths, never a path end). A summary is read from the normal exits only;
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
    `data`, is the contract's word, `CG22-CALLBACK-REGISTRY-CONTRACT`), an
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
    `legacy_relational_summary.rs`, thin hooks in `legacy_c.rs`
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
    boundary `;`, `{`, `}` before `NAME`). An out slot a class leaves
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
    infeasible, or treat `resource_borrowed` on an out slot as unwritten
    outside a disposition. Premises stated, not proved: faithful Clang
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
    bounds contracts has the same identity, accesses and verdict. **Do not**
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
