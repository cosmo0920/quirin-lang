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
    no-return call or a handle kind stays undeclared with its reason, and a
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
    taken). `X ?: B` is admitted only when `X` is reads only: the lowering
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
    every execution, W07/CG29; no liveness, W11). `quirin-audit
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
