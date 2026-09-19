# Buffers Records rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

<a id="constraint-15h"></a>

<!-- preserved:constraint-15h:start -->
15h. **A `concat` result is static storage the compiler computed, and a
    `borrow_utf8` view is the buffer's own — since wave 27, a view of a
    *range* of it (amended below).** `source.string.v1` composes text
    from **check-time-known** pieces into a new `static const` payload of the
    emitted program: no allocation, no run-time copy, no custody, and a length
    that is the **sum of the pieces' lengths** and is itself a check-time
    constant. Do not make the composition materialise into an owned buffer —
    that was measured and rejected, because it would add an allocation, a copy
    and a release to an operation whose result the compiler already holds, and
    would turn `len` of the result into a run-time quantity, which is the one
    property the operation exists to preserve (`STRING-SLICE-CONCAT-RUNTIME`).
    The `std.buffer` bridge is **borrow only**: an owned string is a new custody
    shape the `{storage, offset, len}` view has nowhere to carry, and composing
    a borrowed view already launders its bytes into static storage, so nothing
    is blocked on the freeze (`STRING-SLICE-FREEZE`). A borrow's UTF-8 validity
    is decided **statically**, out of the buffer's own write history —
    `source.buffer.v1` admits only constant bytes at constant indices — and its
    **window** runs from the borrow to the last statement reading the view or
    **anything derived from it**, a slice of a borrow being itself a borrow of
    the same buffer. **Superseded deliberately by card `LOOPS-ON-COMPOSED-ROUTES`
    (wave 39; the amendment at the end of this constraint):** the sentence this
    constraint carried from `STRING-CONCAT` to wave 38 — "Do not widen the
    window's computation without revisiting it: the interval is exact because
    the route is one flat statement sequence, and a loop construct invalidates
    that premise — which is why the composed route **refuses** a `for` today
    (`STRING-CONCAT-LOOP-WINDOW`)" — is replaced by the iteration-window
    theorem: the window is computed **over iterations**, and the composed route
    **admits** a `for`. A borrow
    contributes an ordinary `Read` to the cycle-3 ownership operation list; do
    not re-implement the custody rules beside it. **Route selection changes only
    for the bridge construct** — a module holding a buffer without spelling
    `borrow_utf8` keeps `source.buffer.v1` — and a composed module is one that
    **acquires** a buffer, which is what keeps `drop s;` on a string refusing at
    `Q068` where it always did. `string-concat`.
    **Amended, card `CT07-SLICE-PERMISSIONS` (deliberate; it narrows a sentence
    15h already carried rather than taking a new number).**
    `borrow_utf8(NAME, LOW, HIGH)` lends the bytes `[LOW, HIGH)`, and the
    whole-extent spelling is that form at `[0, extent)` — an **equation**, proved
    against the shipped definition and measured on the certificate, the
    observation and the emitted bytes, not an abbreviation. The window is
    unchanged; what changed is what it **excludes**: a write is refused when its
    byte lies inside a live loan and **admitted when it lies outside every one of
    them**, with disjointness decided by `max low < min high`, which is the
    set-theoretic question itself (`overlaps_iff_a_shared_byte`) because every
    bound on this route is a check-time constant. Three asymmetries must survive
    refactors. A **release is refused for any live loan however narrow**, because
    it ends the storage all of them address — do not make that arm range-wise. An
    index the route cannot resolve to a constant is **not provably disjoint** and
    conflicts with every live loan, because the exclusion is decided *before*
    `source.buffer.v1` judges the statement; falling through would make an
    unresolvable index an admission at this gate. And two overlapping **borrows**
    are deliberately **not** refused: both are immutable, they conflict on
    nothing, and a slice of a loan is a fully overlapping second loan of the same
    buffer by construction, so refusing overlap would refuse the mechanism this
    constraint already rests on. There **is now a write lend** — landed by the
    card the old "no write lend" sentence's own residual named
    (`SLICE-PERMISSIONS-WRITE-LEND`, wave 29), superseding that sentence
    **deliberately** — on the **uncomposed buffer route only**:
    `let NAME = lend(BUFFER, LOW, HIGH);` transfers write authority over
    `[LOW, HIGH)` for the loan's lexical window, decided by the lend-extended
    cycle-3 ownership IR rather than by a route rule — a `LendRange` borrow node
    carries the half-open byte interval `STD-BUFFER-SLICE` predicted (that
    recorded blocker was measured to be about the **write** direction, and this
    is it) — and the IR decides that two live lends are admitted exactly when
    their ranges are disjoint, that an owner access is refused exactly where its
    stated footprint meets a live lend and admitted outside every one (reads
    included; an extent query's empty footprint meets nothing), that an access
    through a loan must state a rebased footprint inside the lend (rangeless
    conflicts with everything, `WRITE-LEND-COMPUTED-INDEX`), and that a release
    under any live lend refuses however narrow, by the existing drop rule. **Do
    not move these rules out of the IR into a route, and do not give the release
    arm a range.** On the composed string route every sentence of this
    constraint stands unchanged: a `lend(…)` beside `borrow_utf8`'s read loans
    refuses by name at `Q077` (`SLICE-PERMISSIONS-COMPOSED-LEND`). **Superseded
    deliberately by card `LOOPS-ON-COMPOSED-ROUTES` (wave 39):** the clause that
    followed — "so the window-exactness argument and the `for` refusal at
    `Q068` are untouched there (`STRING-CONCAT-LOOP-WINDOW` re-measured, not
    assumed)" — and the first of the three refusals the next sentence listed —
    "a `for` in an entry that lends (`SLICE-PERMISSIONS-LOOP-WINDOW`)" — are
    replaced by the amendment at the end of this constraint. On the buffer
    route the window's soundness is now defended by **two** refusals that must
    survive refactors, plus the amendment's own two: a crossed pair of lend
    windows (`SLICE-PERMISSIONS-CROSSED-WINDOWS`) and an
    acquisition inside a window (`WRITE-LEND-NESTED-ACQUIRE`), the latter
    scanned at any depth. The lend's bound
    tier is the loop bound's (`WRITE-LEND-BOUND-FROM-BYTE`); a loan is never
    released and never re-lent (`WRITE-LEND-RELEND` stays open and is
    orthogonal to loops — a sub-lend is a second borrow node nested in the
    first's body, which the one-pass walker cannot see); and a module with no
    lend builds the custody operation list it always built.
    **Appended, card `WRITE-LEND-TASKS` (wave 31; it supersedes nothing above,
    discharging the sibling residual the wave-29 card named).** The write lend
    now also **crosses the spawn boundary**, as a range write lease:
    `lend PLACE LOW..HIGH` in a spawn capture transfers write authority over
    `[LOW, HIGH)` of an owned Buffer's payload (`BUFFER_PAYLOAD_BYTES = 4`, the
    `int32_t`'s real bytes) to one worker for exactly spawn..join, decided by
    the **lease-extended scoped-task ledger** (`ResourceState::WriteLeased`)
    rather than by a route rule. The disjointness decision is
    `ownership::axes_overlap` — made `pub` and **called** by
    `scoped_tasks::LeaseRange::overlaps`; do not re-implement it in the ledger
    and do not move it out of the IR. Three rules must survive refactors: the
    owner holds **no** whole-payload authority, read included, while any lease
    is out (stricter than both the read-share route and the sequential lend's
    per-byte exclusion, because the owner's only spellable footprint here is
    the whole payload); an **empty** lease refuses (`InvalidLease`) where the
    sequential empty lend admits, the crossing of no authority being vacuous
    and failing closed; and only the creator's join reclaims a lease, in the
    same transaction that collects the capsule. The judgment's mode is
    `LendWrite`, deliberately distinct from the still-refused `write` capture
    (`aWriteCaptureIsRefusedByName` re-decided over the grown matrices), and
    exclusivity is audited as
    `QCore.WriteLendTasks.no_byte_has_two_write_authorities`.
    **Appended, card `WRITE-LEND-TASKS-EXERCISE` (wave 36); it supersedes the
    "no operation admits a lease in a worker body" sentence deliberately,
    discharging the residual of that name by landing the three things that
    sentence demanded together.** A worker body now **exercises** its lease:
    `qcore_buffer_write(LEASE, INDEX, VALUE)` writes one byte and
    `qcore_buffer_read(NAME, INDEX)` reads one, at an index **relative to the
    authority's own base** — a lease's `low`, or 0 for an owner — which is the
    sequential lend's rebasing law (`WRITE-LEND-COMPUTED-INDEX`) crossing the
    spawn boundary. The decision is **in the IR and split across the two places
    its halves are known**, and must stay split: the worker's body states the
    exclusive bound it touches through its lease, because a body is checked
    without knowing where its lease sits; the **creator's spawn site** decides
    whether that bound fits the real `[low, high)`. Do not move the fit into
    the worker (it cannot be decided there) or the footprint into the creator
    (it is a property of the body). The equivalence of that one comparison with
    every per-index containment fact is proved
    (`QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law`),
    which is what licenses checking a summary once instead of every write at
    every creation. Four rules must survive refactors. A byte index is a
    **check-time constant or nothing** — an unresolvable index is not *provably*
    inside its authority and refuses, and the refusal is non-vacuous because a
    nonempty lease always has indices it does not admit
    (`some_index_is_always_outside`). The **owner gains no byte write**: the
    whole-payload write stays its only write footprint
    (`WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`), so wave 31's "the owner holds no
    whole-payload authority while a lease is out" is inherited unchanged rather
    than re-measured against a second owner-side footprint. A **read share
    admits no byte view** (`WRITE-LEND-EXERCISE-LOAN-BYTE-READ`). And the
    owner's byte **read** takes the *same* ledger gate its whole-payload read
    takes, so a lease outstanding refuses it by the existing rule and not by a
    second one written here. Exclusivity is re-audited about the writes
    themselves — `QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties`
    — and exhibited natively: two threads writing distinct bytes of one
    `int32_t` payload under ASan/UBSan/LSan with stdout byte-identical to the
    reference run. The emitted worker writes
    `((unsigned char *)&buffer->payload)[base + index]`, a distinct memory
    location in the sense of C11 3.14; the lease's **base** travels in the task
    value's otherwise-unused scalar field, which is the one fact the holder
    must not be able to choose. Three boundaries are **measured, not claimed**,
    and the first has since been **superseded by measurement** and is stated
    here in its current form: wave 36 measured ThreadSanitizer killing every
    program this route emits, lease or not, and read the fault as the route's
    (`TASK-ROUTE-TSAN-SEGV`); wave 37's `TASK-ROUTE-TSAN` measured that it is
    **not** the route's — TSan ships no `thrd_create` interceptor, so an
    emitted thread reaches `start_thread` unregistered — retired that residual,
    and bound `thrd_create` to `pthread_create` through one `-include`, under
    which TSan does run these programs — wave 39's `TSAN-ROWS-2` runs **this
    flagship itself**
    (`fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore`) clean,
    unforced and under both forced cohort schedules, Linux only, through
    `compiler/tests/task_route_tsan.rs`. What stands is
    narrower and still load-bearing: a clean detector is evidence about the
    schedules realised and never a proof of race freedom, so this constraint's
    exclusivity claim rests on C11 3.14, on the ASan/UBSan/LSan run and on the
    allocation identity, with the TSan rows beside them and not in their place
    (`docs/reports/{task-route-tsan,tsan-rows-2}/README.md`). A stray byte at `payload + 4` lands in
    `struct qcore_buffer`'s own tail padding, which the allocation owns, so no
    memory tool sees it — which is why the emitted helper carries its **own**
    extent guard rather than delegating to a sanitizer, and that guard must not
    be removed; and a program whose observation depends on the payload's object
    representation is host-dependent (`WRITE-LEND-EXERCISE-BYTE-ORDER`), the
    published flagship being representation-independent by construction rather
    than by luck. No Windows or Apple run is claimed
    (`WRITE-LEND-EXERCISE-WINDOWS-RUN`).
    **Amended, card `LOOPS-ON-COMPOSED-ROUTES` (wave 39; deliberate — it
    replaces the "one flat statement sequence" premise this constraint carried
    from `STRING-CONCAT`, the two sentences quoted as superseded above, by a
    theorem).** A loan's window is computed **over iterations**: on both
    routes it runs from the loan's opening to the last **top-level** statement
    naming the loan **at any depth** (`std_buffer::last_uses`;
    `std_string::view_uses_at_any_depth`), so a `for` whose body names a loan
    is that statement and the loan is **live across every iteration** of it.
    The computation is proved sound relative to `source.loop.v1`'s own
    unrolling in `QCore.LoopWindows`: every use in the unrolled program lies
    inside the window (`every_use_lies_inside_the_window`), unrolled positions
    are attributed to non-decreasing top-level indices, so everything the exact
    unrolled window contains the iteration window contains
    (`the_unrolled_window_lies_inside_the_iteration_window`), the unrolling is
    `QCore.SourceLoop.unroll`'s (`the_unrolling_is_loop_v1s`), a loop-free
    program's window is the flat one the routes always computed
    (`the_window_is_exact_over_a_flat_program`), and the replaced top-level-only
    computation demonstrably missed a body use
    (`a_top_level_only_window_misses_a_body_use`). The over-approximation is in
    the **refusing** direction because the IR's owner-access, lend-admission
    and drop decisions are monotone in the live loans
    (`a_longer_live_set_refuses_no_less` and siblings) — do not replace the
    window by an exact per-position one without re-proving both halves. A
    `for` is therefore **admitted** beside a lend (`SLICE-PERMISSIONS-LOOP-WINDOW`
    lifted, `Q051` narrowed) and inside a composed string/buffer entry
    (`STRING-CONCAT-LOOP-WINDOW` discharged, `Q068` and `Q077` narrowed), with
    the body decided **under the loans live at the loop**: on the buffer route
    the lend-structured projection unrolls the body inside the borrow node
    with the index bound, so the IR decides every body access as it decides a
    top-level one; on the composed route every body store, at every
    iteration, is judged against every loan still read after the loop
    **before the body runs** (`loop_body_conflict`), the body then running
    through the buffer route's own `BufferRun::repeat` — the one iteration
    rule, published for this — and the composed custody list unrolling it so a
    body store after a release is the inherited `UseAfterDrop`. An index
    outside the lowering-visible tier (a byte or a field read) is **not
    provably disjoint** and conflicts with every live loan of its buffer, on
    both routes. Three premises must survive refactors, each a refusal **by
    name** that sits **before** the window is computed and must not be
    reordered behind it: a lend or a `borrow_utf8` inside a body is a
    per-iteration loan (`LOOP-PER-ITERATION-LOAN`, `Q051`/`Q077`); a body that
    acquires or releases where a loan is held (`LOOP-WINDOW-BODY-CUSTODY`,
    `Q051` in an entry that lends, `Q077` on the composed route); and a string
    construct, text line, `len`-bound header or unrolling licence inside a
    composed `for` body (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`, `Q077`).
    The release arm stays whole-buffer, the crossed-window and (now
    depth-aware) nested-acquisition refusals stand, `WRITE-LEND-RELEND` is
    untouched, an entry with no loan keeps `source.loop.v1`'s admission byte
    for byte, and every program admitted before the card emits byte-identical
    C (a 527-program census, 139 admitted, 0 moved).
    `ct07-slice-permissions`, `slice-permissions-write-lend`,
    `write-lend-tasks`, `write-lend-tasks-exercise`,
    `loops-on-composed-routes`.
<!-- preserved:constraint-15h:end -->

### 15h — amendment, card `COMPOSED-WINDOW-REACH` (wave 41)

Outside the preserved block above **deliberately**: that block reconstructs the
pre-restructure guide byte for byte and
`docs/reports/docs-structure-compression/verify.py` pins its `sha256`, so it is
a frozen archive in constraint 21's sense. The first draft of this amendment was
written inside it and the verifier caught it; the pin was **not** edited. This
section is current rule text and carries the same authority as the block it
amends.

It **narrows a clause** the `LOOPS-ON-COMPOSED-ROUTES` amendment carries and
**adds a refusal**; it supersedes nothing else.

**First, the two routes' window computations are not parallel.** On the
**buffer** route `std_buffer::last_uses` is the deciding rule and its "at any
depth" is live: re-running wave 39's own M1 against that card's 24 test rows
reddens **7**, every one a lend row. On the **composed string** route
`std_string::view_uses_at_any_depth` decides at **top level only** — its depth
half is empty for every input, so a `for` contributes nothing to any view's
window, because a composed body holds `BufferEvent`s and no `BufferEvent` names
a view — and **no** composed row of wave 39's file is anchored by it. Read the
clause that names those two functions as naming two functions of *different
standing*: on the composed route the window is **defence in depth**, and the
deciding rule is `check_string_program`'s `Repeat` arm judging every body store
against every loan live at the loop before the body runs, which the preserved
text already says two sentences later. `COMPOSED-VIEW-WINDOW-INERT`'s own title
("returns the empty vector for every input") is **wrong** and its content is
right: the function names a view at every reading position.

**Second, what makes the inert recursion safe rests on three mechanisms, not
one**, and all three must survive refactors: a composed body's type; the `Q077`
vocabulary refusal for the view-*constructing* forms; and — newly named, because
it is a **rendering** rule doing a window's work and a later card could remove it
without reading this constraint — the buffer route's own "`print` names a
`std.fmt` rendering" refusal, which is what stops a **bare** `print s;` in a
body, `StringUse::Render` deliberately not claiming a bare name
(`COMPOSED-BODY-BARE-RENDER-INCIDENTAL`, pinned by
`a_bare_render_of_a_view_inside_a_composed_body_is_refused`).

**Third, and the one change to code:** a name the window's key space **cannot
express** is now refused by the window function's own gate rather than dropped.
`custody_names_at_any_depth` classifies what it meets (`WindowName::Binding` /
`WindowName::FieldPath`), and `window_unnameable_events` turns a field path —
`AGGREGATE-OWNED-FIELD`'s four `Field*` events, which address a buffer as
`RECORD.FIELD` — into a refusal by name at `Q051` (a condition extension of the
band this constraint already names for the lend's window shapes; **no code
allocated**). Before the card those four arms returned nothing, correct only
because `OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY` run first, so
lifting either would have silently **shortened** a window — the admitting
direction (`WINDOW-FIELD-ARM-UNREACHED`). Both of those refusals stand and still
fire first; this gate is a second, independent line, and whoever lifts them must
give the four arms real windows and retire it deliberately
(`WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED`). Two arms that residual counted with
them, `Construct` and `Field`, are **measured reachable today** — a record whose
fields are all `i32` is admitted beside a lend and inside a loop body of a
lending entry — and are correctly empty.

The Lean is now **bound** to the Rust rather than linked by a doc comment:
`QCore.LoopWindows` carries `LendEvent`, the window-relevant shape of the Rust
`BufferEvent`; the unrolling is instantiated there
(`the_lend_alphabets_unrolling_is_loop_v1s_recursion`, sharing a right-hand side
with `loop_v1s_unrolling_is_the_same_flattening`); and eight shared vectors are
decided in Lean (`the_agreement_table_holds`) and replayed against `last_uses`
by `compiler/tests/composed_window_reach.rs`, so neither side can be edited
alone — a **finite** binding, and that is its boundary
(`LOOPWINDOWS-BINDING-IS-FINITE`).

Nothing this card lands is reachable from source: a 535-program census admits
146 on both sides, byte-identical, with **0** verdicts moved either way.
Evidence: `docs/reports/composed-window-reach/README.md`.

<a id="constraint-15k"></a>

<!-- preserved:constraint-15k:start -->
15k. **A record of scalars is a plain copyable value; a record with a `bytes`
    field is a custody shape the ownership IR addresses by path — and the four
    rules that make it one live in the IR, not in a route.** `source.record.v1`'s
    `type NAME { FIELD: i32, … }`, `let p = NAME { FIELD: OPERAND, … };` and
    `let a = p.FIELD;` are admitted on `source.buffer.v1` only, with every other
    route's refusal measured and pinned rather than widened. A record whose every
    field is an `i32` contributes **no** `qcore_core::ownership` operation, opens
    no obligation and derives **no effect**, and its consequences are the
    admitting route's **existing** rules — `drop p;` is `Q052`, a leak beside it
    is still the inherited checker's `Q054`; construction totality is what makes
    projection total, which is what makes the field read a function rather than a
    partial one. **Amended, card `AGGREGATE-OWNED-FIELD` (wave 39; deliberate —
    it supersedes this constraint's former "do not give a field custody without
    the four rules that do not exist", by landing them).** The cycle-3 ownership
    IR now addresses an owner by a **path** (`ownership::Place::Field`, one level:
    `NAME.FIELD`, measured sufficient for the record route and recorded as the
    boundary `AGGREGATE-NESTED` beyond), and the four rules are the IR's own and
    must stay there (15h's precedent forbids route-side custody rules): a
    `Compose` **moves** each named buffer into its field, so the source binding
    is a use after move from the construction on; a field read of an owned field
    is the **partial move** (`MoveOut`), leaving a hole a second read refuses;
    `Drop` of the record releases **every live owned field in declaration order
    and nothing twice**, tolerating a hole (`drop_through`; the Lean
    `QCore.AggregateOwnedField.drop_through_releases_exactly_the_live_fields`
    and `…nothing_twice`); `NormalPathLeak` names an unreleased field **by its
    path**; and `Copy` of a record declaring an owned field refuses whatever the
    state of its fields. The record itself is never a plain owner — a read,
    write, whole move, transfer, borrow or lend of it is
    `AggregateAtOwnerPosition` — and a field path is a **type**, not a dotted
    spelling: `Read("p.buf")` is an unknown binding. On the route, the field type
    is spelled **`bytes`** — the acquisition's own reserved word; the task
    route's `Buffer` names a different resource with a different model and was
    measured and not taken — and `RECORD.FIELD` stands at every custody position
    a buffer name does; `drop RECORD;` of a record of scalars keeps its `Q052`.
    **`Q055` is narrowed, the one lift**: a buffer at a field position is
    refused only where the field is declared `i32`. Two shapes are refused by
    name rather than modelled and must stay so until their card:
    `OWNED-FIELD-LEND-WINDOW` (a lend in an entry that composes such a record —
    the lend projection addresses owners by plain name) and
    `OWNED-FIELD-LOOP-BODY` (an owned-field statement in a `for` body — the
    unrolled walk scopes acquisitions by name and a composed record's fields by
    path). Every program written before the card builds the operation list it
    always built and emits byte-identical C, measured over the whole tree
    (`docs/reports/aggregate-owned-field/README.md` §3, §6); the record of
    scalars is exactly what `aggregates-v1` measured. `AGGREGATE-VIEW-FIELD`
    remains the view-field step constraint 15h's exact borrow window cannot
    reach through. Two emission facts are load-bearing: the
    `qcore_record_`/`f_`/`r_` prefixes, so a QCore name that is a C keyword still
    emits compilable C and a record, a buffer and a scalar of one source name are
    three distinct C objects; and **a program that acquires no buffer carries no
    buffer prelude**, forced by `-Wall -Wextra -Werror` on an unused `static` and
    byte-neutral for every previously admitted program by an argument about which
    programs the emitter could reach (among admitted programs, reading or writing
    a byte *implies* acquiring a buffer — **re-derived at wave 39 with the owned
    field enumerated**: a field holds only a buffer `bytes(N)` acquired, and a
    construction moves rather than acquires, so no new acquisition path exists and
    the argument stands) — measured at 80 emitting programs byte-identical at
    wave 27 and re-measured at wave 39. `aggregates-v1`, `aggregate-owned-field`.
<!-- preserved:constraint-15k:end -->
