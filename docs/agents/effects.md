# Effects rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-15j"></a>

<!-- preserved:constraint-15j:start -->
15j. **A flat epoch's credit is conserved across the creator's own error edges,
    and the disposition is declared while the edge is derived.** For every task
    of an admitted plan `restored + spent = credit` and `restored ≤ credit`,
    componentwise on both axes, and over the epoch
    `restored_credits + spent_credits = granted_credits ≤ allocation`. Both
    directions are **refusals**, not conventions: an abandoned grant — neither
    restored nor spent — is always refused (`Q635`, and this is CT23), and a
    restoration larger than its grant is refused per task and per axis (`Q636`),
    because it would put credit into the epoch that the epoch never granted. The
    split is spelled and checked, never inferred both ways: the creator's error
    edges are **derived** from the checked task IR (`TrySpawn.spawn_error`, an
    eliminated `TryOutcome` join), while what the edge does with the grant is
    **declared** and then checked against the grant the epoch actually makes — so
    a plan can neither claim an error path the program does not have nor restore
    credit that was never granted. Three things must stay as they are.
    **`Q637` is deliberately not extended**: because `spent ≤ granted`, the credit
    committed on every error edge is bounded by the success exit's, which `Q637`
    already bounds by the allocation, so a second allocation check there would be
    an unreachable condition rather than a gate (proved as
    `the_error_arm_commits_no_more_than_the_granted_total`). **Restored credit is
    not re-granted** — making the committed total path-sensitive would *admit*
    programs `check_flat_epoch` refuses today, the forbidden direction
    (`CT23-RESTORE-REUSE`). And the **declarative** route has no error edge at
    all, so a clause written there is refused by the owning band
    (`ErrorArmWithoutErrorEdge`, `Q630`) rather than by a grammar that pretends
    the word does not exist; an elided arm is `error(drop);`, which is exactly the
    accounting that existed before, and `qcore_core::epoch_accounting` is
    untouched. `ct23-credit-error-path`.
<!-- preserved:constraint-15j:end -->

<a id="constraint-15l"></a>

<!-- preserved:constraint-15l:start -->
15l. **An admitted quantitative region erases from emission, and the sealed pack
    is a linear capability consumed exactly once.** `source.region.v1` —
    `error region NAME { … } catch (ERROR) quant open NAME as BINDING { discharge
    BINDING; }` — lands on the **sequential** route, is not a route of its own and
    selects nothing, which is what gives it the existing certificate, reference
    execution, C emission and native comparison unchanged. Every outcome is a
    check-time literal, so the region has one path and compiles to
    **byte-identical C with the hand-written straight-line program it is**
    (measured against a twin, the whole difference being the `#line` directive
    naming the other file): a region buys checking, not code. Every *semantic*
    refusal is the unedited `qcore_core::error_region` model's own at the code
    that pass already allocates (`Q303`–`Q309`, plus `Q300` for an undeclared
    error type); only the reader's own **shape** refusals land on `Q302`. Two
    things must not be "repaired". `RegionOp::QuantOpen` — the operation
    **variant** — is **unreachable from a valid program by design**: the admitting
    constructor is the `error_region::QuantOpen` **struct** on a catch clause, and
    the variant is retained so an opening outside a catch has an explicit `Q302`
    (`REGION-QUANTOPEN-OP-VARIANT`). And the v1 **path** bound is refused by name
    rather than admitted silently — no error edge, a second catch clause, a second
    region, an uncaught raise, a failing acquisition as the edge, a region in a
    branch arm, a handler statement other than a discharge, a payload binder —
    because `analyze_error_regions` calls `handle_catch` only for an error exit
    that reaches a matching clause, so a clause on a path the region does not take
    is a clause **nothing decides**, and admitting one would be silent rather than
    fail-closed (invariant 1). `quant-region-source`.
<!-- preserved:constraint-15l:end -->

<a id="constraint-15m"></a>

<!-- preserved:constraint-15m:start -->
15m. **A composed region on the epoch task route opens its pack after every
    join, and the pack's duties are the parent's own owners.**
    `REGION-CREDIT-JOINS` (wave 28) let a `source.epoch.tasks.v1` entry carry
    one `error region … catch (E) quant open … { discharge …; }` whose
    `AbandonCredit` publishes the parent-local unused credit — CT22's scenario.
    `REGION-JOINS-DUTIES` (wave 29) **deliberately superseded** the duty-free
    premise this constraint recorded: the composed body also admits the task
    route's own acquisition and release — `let NAME = qcore_buffer_new(INT)?;`
    sealing a real `Destroy` duty, `drop NAME;` of a region owner erasing one —
    and the discharge destroys the remaining owners in reverse acquisition
    order, CT25's duty half. `REGION-JOINS-LOAN` (wave 30) **deliberately
    discharged `T08-COMPOSE-LOAN`** and landed the loan half: a worker may
    capture a counted read share of a creator owner (`read PLACE share N` at
    the spawn; the place-free `read;` clause in the plan), admitted because the
    lender on this route is always the creator and creator-scope bytes are
    outside the epoch allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan has
    no epoch residency; and the region body also admits the **adoption**
    `move NAME;`, which takes a live entry owner's cleanup duty into the pack —
    an acquisition from the pack's side, erasing completely, its release
    written into the blanks after every join. "After joins and loan expiry" is
    the composition of unedited rules: the share returns at the join under the
    task ledger, the joins precede the region (`Q600`), and a destruction while
    a share is live refuses as the ledger's own `LoanOutstanding` — do not
    re-implement any of the three. Premises that must survive refactors, each a
    refusal today: every `TrySpawn`/`Join` span of the checked task IR
    **precedes** the region (`Q600` in both directions — children live at the
    opening stays `CheckedRegionTaskCreditV1`'s territory, `REGION-JOINS-T07`);
    the region **erases into its straight-line twin** — a duty-free region
    blanks completely (emitted C, source map and reference observation
    byte-identical with and without it, the wave-28 measurement kept), a
    duty-carrying one keeps its acquisitions and body releases at their own
    byte offsets in the task half with one `drop NAME;` per remaining duty
    written into the blanked space after them, first-fit, newlines never
    crossed, refusing when it does not fit (`REGION-JOINS-ERASURE-FIT`), and an
    adoption keeps nothing (the owner's acquisition already stands in the
    entry's own text) — so the composed checkers (the scoped-task route and its
    ledger, `check_flat_epoch` + `EpochCreditRecoveryV1`,
    `analyze_error_regions`) stay **unedited**, each refusing at its own codes.
    **Do not re-implement a custody rule beside the erasure**: two
    simultaneously live owners refuse at the task route's existing custody
    `Q600` — the second acquisition's `?` failure edge discards the first, the
    erased program's real leak path (`REGION-JOINS-MULTI-OWNER`, reachable
    since wave 30 with an adopted owner beside an acquisition) — and an adopted
    name that is not a live owned Buffer in the erased program refuses at the
    task route's own codes (`Q500` for a non-buffer, `Q600` for custody). Both
    published figures are re-derived before publication — `abandoned_credit`
    against the reader's quantity fold, `released` against its duty fold, over
    the model's own cleanup trace. An **acquired** owner's name is the region's
    own (a collision with an entry binding refuses, since the erasure flattens
    the body's block into the task scope); an **adopted** owner's name is the
    entry's own (a name the entry does not bind refuses); the sequential
    `qcore_acquire` spelling names no task-route resource and stays refused;
    and the declarative epoch route gains no error path (constraint 15j's
    shape). Zero new codes: `Q302` condition widenings and the one stated
    `Q600` narrowing, in §L21. `region-credit-joins`, `region-joins-duties`,
    `region-joins-loan`.
<!-- preserved:constraint-15m:end -->

<!-- generated:rules:end -->

## Amendment to 15m — CT24 cleanup/task composition

A composed handled region is either the creator's direct epilogue after
every creation and join, or one direct statement in the exact
`Result::Ok` arm of a direct `match spawn` in that creator task scope. In
the second form, its straight-line erasure places every cleanup release at
the region's own source position; the unedited scoped-task ledger checks
that release before any later join. Arbitrary branches and loops, the
spawn-error arm, worker regions and every other nested placement remain
refused. Region strict credit is creator-local and neither settles nor
abandons a child's epoch grant; published region evidence in a success arm
is conditional on that arm, while spawn-failure recovery stays the task
route's own.

This supersedes only the blanket after-all-joins placement premise above.
[Evidence and controls](../reports/ct24-cleanup-task-composition/README.md).
