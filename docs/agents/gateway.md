# Gateway rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

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

<!-- preserved:constraint-5c:start -->
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
    header's successors (lines 3325 and 5177 at `c33c7640`; the wave-36
    close's 3312 and 5161 had already been moved by `SCALAR-ROUTE-EXIT-EDGE`,
    so the citation is by identifier and the line numbers name the commit
    they were measured at — `DOCS-COMPACT-7` §7.3 item 5). It is exact
    (C11 §6.8.5.3) and `break` still reaches `continuation`, but it carries
    neither a Lean statement nor a pin, and it is not owned by card
    `C-GATEWAY-CONSTANT-DO-WHILE` (`FOREVER-FOR-EXIT-EDGE-UNOWNED`).
    The 32 `unmodeled call,
    aggregate or indirect access in do-while condition` leaves are a different
    question and stay refused — measured unmoved at 32 across the re-pin.
    Proved as `QCore.ConstantDoWhile`: `constant_do_while_is_its_body` the
    `≡ S` equality, `removing_the_back_edge_only_removes_paths` and
    `the_successor_list_never_grows` the conservativity for **every**
    condition, `the_repeated_release_is_a_path_the_c_does_not_have` the
    non-vacuity. Separately, and **not owned by that card**, a pre-existing
    route already decides a `do`-`while` from a *modelled scalar value*; it
    answers conservatively wherever measured and is pinned by
    `a_modeled_scalar_condition_is_a_separate_pre_existing_route`
    (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`).
    That neighbour is now owned by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37), and
    "conservatively wherever measured" was measured only on positively spelled
    headers. The value route decides a loop header from a **modelled value**,
    and deleting a loop header's **exit** edge orphans the function's exit
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
    is unchanged (154 corpus prunings), and `for (;;)` carries no exit
    successor for the rule to reach. Proved as
    `QCore.ScalarRouteExitEdge.the_exit_edge_is_never_removed`, with
    `the_rule_never_promotes_to_safe` for the other direction. Finally, removing a back edge
    **makes a function acyclic**, and that changes which route
    `legacy_joined_provenance::derive` takes for it (constraint 10b) — so this
    rule silently changes *that* pre-pass's reach: **654** of 4,771 analysed
    functions at this card's re-pin, with a measured diagnostic effect of zero.
    Any widening of the key therefore moves two subsystems and must re-measure
    both (`docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py`,
    `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`).
    **Two clauses of this paragraph are the integrator's, not the card's**, and
    the wave-35 close's `reconciliation.json` carries both before-and-after
    texts: the card wrote that `derive` "returns no conditional-provenance facts
    at all for a cyclic CFG", which wave 35's **second** arbiter
    (`JOINED-PROVENANCE-CYCLES`) falsified later in the same wave by solving a
    cyclic CFG with a greatest fixed point — landing that sentence verbatim
    would have put a statement contradicting 10b into this file — and the card's
    script path was its own gitignored `measure/`, replaced by the committed
    copy it archived. The rule itself is unchanged, and the residual it names is
    **answered** by that second arbiter's §8, which reproduced the 654
    independently and measured the flip's cost at six functions and 20 leaves of
    servable population — reach that moved to the topological pass, not reach
    lost. `c-gateway-constant-do-while`, `joined-provenance-cycles`.
<!-- preserved:constraint-5c:end -->

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

<a id="constraint-6"></a>

<!-- preserved:constraint-6:start -->
6. **C02-P-ALIAS-LEAK-ORDER:** `report_leaks` on intermediate iterates is
   scheduling-sensitive; a pre-pass keeps it scheduling-neutral with
   `AbstractState` byte-identical. Recorded, not repaired — do not disturb it
   incidentally. `remaining-c02-p-alias`, `remaining-borrow-liveness`,
   `remaining-hygiene-14`.
<!-- preserved:constraint-6:end -->

<a id="constraint-7"></a>

<!-- preserved:constraint-7:start -->
7. Leak diagnostics are **272** across the pinned 185-unit corpus. A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. Through wave 30 it had moved four times, each deliberately and each
   either in the *accusing* direction or by **deciding** a leaf. The current
   six-row increase is attributed after that historical chain below.
   271 → 272 (wave 17): `flb_input_chunk_get_direct_routes` L19399,
   accusing, on a function and a unit already `Unsafe`, held provisional as
   `CONSTRUCTOR-RESIDUALS-LEAK-271`. 272 → 273 (wave 20): `input_chunk_get`
   L19849, accusing — modelling the constructor exposed a release through the
   then-uncontracted `flb_input_chunk_destroy`. 273 → 272 (wave 22): that same
   leaf decided by the faithful `flb_input_chunk_destroy` contract — a
   **withdrawal**, not a discharge, since the release refuses at the call on
   joined provenance and `input_chunk_get` goes provisional `Unsafe` →
   `Unknown`; residual `AOM-DESTROY-JOINED-PROVENANCE`. 272 → **266**
   (wave 23): the faithful **`nullable`**
   `flb_input_chunk_destroy_direct_routes` contract (`nullable` where its
   already-declared neighbour carries `nonnull`, because that body's third
   statement is `if (!routes) { return; }`) retires six leaves of
   `flb_input_chunk_get_direct_routes` — L19384, L19392, L19399, L19434,
   L19489, L19505 — again by **withdrawal-by-refusal**: at each preceding call
   site the projection cannot resolve the actual it is handed, so the site is
   marked through the unresolved-effect mechanism and `report_leaks` skips it.
   The cause is that every refused statement poisons every pointer binding in
   the state, so an acquisition's binding does not survive to its release —
   residual `AOM-SIBLINGS-BINDING-POISON`, whose repair moves those sites
   toward the *discharging* arm, a direction invariant 1 constrains. That move
   **DECIDES — withdraws** `CONSTRUCTOR-RESIDUALS-LEAK-271` at a measured cost
   of **five** standing claims, not the six wave 17 recorded: the sixth leaf,
   L19544, is on the success path where no destroy call runs, so it **stands**,
   the function keeps its leak and its provisional `Unsafe`, and the pin reads
   266 rather than 265. Nothing is promoted by any of the four: zero unit status
   moves, zero function status moves. At the wave-25 re-measurement there was
   **no fifth move**: the L19399 leaf wave 23 decided by *withdrawal* is decided instead by the **discharging** arm of that same contract, its first
   firing anywhere in the pinned corpus: a strictly better answer to the same
   question, changing no count, the pin still **266** with not one leak leaf
   added or removed. The wave-27 arbiter also made **no further move**: it
   re-pinned *presentation* only, and the 266 leak rows are exactly the 266 whose
   description text was rewritten — each verified to be the substitution of the
   **site alone**, the text on both sides of the phrase byte-identical. A leak
   leaf's identity is still its preprocessed line.
   `remaining-o04-b08`, `corpus-repin`,
   `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`, `wave22-aom-destroy`,
   `wave22-corpus`, `aom-destroy-siblings` §6 and §6.1,
   `aom-binding-poison` Headline and §5.2, `gateway-c-location` §5.3.
   The current `C-GATEWAY-WAVE` re-pin advances **266 → 272**: six added
   provisional leak leaves, no baseline leak leaf withdrawn. Two arise from
   complete-expression frames and four from nullness refinement; every site is
   listed in `docs/reports/c-gateway-wave/README.md`'s final measured result.
   Four intermediate missing leaves were restored by the scalar-assignment
   repair before publication. The original worklist schedule is retained;
   neither the leak rows nor the new constant-loop repeated-release row is a
   validated upstream defect claim.
   At the wave-34 `C-GATEWAY-RETURN-CALL-W4` arbiter the pin is **unmoved at
   272** — the first gateway *admission* card in this chain to move no leak leaf
   in either direction. That is not luck and is worth the sentence: the exits it
   newly models are allocator wrappers that retain nothing, so exposing them
   accuses nobody. Admitting a previously unmodeled exit remains a move in the
   accusing direction by construction, because `report_leaks` **skips** an
   unmodeled exit; a future admission card reaching bodies that do retain should
   expect the pin to rise, and that rise is a finding to attribute, not a
   regression. `c-gateway-return-call-w4` §5.
   Card `W36-CORPUS-CONTRACTS-3` (wave 36) moved the number **not at all**:
   272 → 272 across a re-pin that added six declared contracts and moved 21
   function statuses. A declarative card that decides 204 uncontracted-call
   leaves is therefore not automatically a leak-pin event, and the pin was
   re-verified rather than re-derived.
   Card `W37-CORPUS-CONTRACTS-4` (wave 37) left the number unmoved a second
   consecutive time: 272 → 272 across a re-pin adding ten declared contracts
   and moving eight function statuses. Two declarative arbiters in a row have
   now decided several hundred uncontracted-call leaves without touching the
   leak pin, which is the expected shape when the declarations release
   nothing; the pin was re-verified rather than re-derived.
<!-- preserved:constraint-7:end -->

<a id="constraint-8"></a>

<!-- preserved:constraint-8:start -->
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
   The mode gate's `SummaryConflict`-only arm for a **nullability
   refinement** was exercised for the first time by a *declaration* rather
   than by the corpus: card `W36-CORPUS-CONTRACTS-3`'s `rd_kafka_mem_free`
   declares `Released{nullable}` where W4 derives `Released{nonnull}`, and
   the declaration is the faithful one — `rd_free(NULL)` reaches
   `free(NULL)`. The declaration governs every call site, the defining unit's
   own function moves `Safe → Unknown`, and **that is the gate working**: a
   disagreement is reported, never silently resolved, and declaring `nonnull`
   to silence it would assert a precondition the body does not have
   (constraint 4). `w36-corpus-contracts-3` §5.2.
   The ABI reader's **withdrawal** arm was exercised for the first time by a
   *production declaration* in card `W37-CORPUS-CONTRACTS-4` (wave 37):
   `rd_kafka_topic_partition_list_sort`'s C declaration spells its comparator
   as an **inline function-pointer parameter declarator**, which the reader
   does not parse — it resolves the function-pointer **typedef** form that
   card `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF` taught it, and nothing else — so
   the declaration is withdrawn at every call site with
   `abi-unreadable-declaration`. The withdrawal is **leaf-neutral**: all
   eleven rows are same-position substitutions and no function's leaf count
   changed. This is the
   gate failing closed on a *reader* boundary rather than on a disagreement,
   which is `LOADER-ABI-UNDECLARED`'s neighbour and is named
   `LOADER-ABI-INLINE-FNPTR-DECLARATOR`.
<!-- preserved:constraint-8:end -->

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
