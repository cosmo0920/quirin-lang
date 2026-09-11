# PIPE-DROP-STAGE — the releasing pipeline stage (wave 23)

Card brief: *"`buffer |> drop;` (and argue which, if any, other keyword-shaped
statement heads join the stage table). The wave-22 report deliberately deferred
this because a stage-table entry that RELEASES custody is a different kind of
entry from one that renders: make that decision explicitly and argue it."* Plus
its sibling `PIPE-INTRINSIC-EXAMPLE`: *"rewrite
`examples/threads/owned_buffer.qcore`'s reading pair as the intrinsic chain the
wave-22 card already admits."*

Filed by `PIPE-INTRINSIC-STAGE` §7, whose stage table carried the withheld entry
in a comment: *"`drop` is the entry a reader will reach for first and the one
deliberately withheld: it takes a single operand exactly as `print` does, but it
*releases custody*, and a releasing table entry is a different kind of entry from
a rendering one."* Base commit: main `52e3889c`.

**What landed.** `drop` is the closed statement-stage table's third head and its
only **releasing** one. `b |> drop;` is `drop b;`, and `7 |> qcore_buffer_new? |>
drop;` is `let ⟨t0⟩ = qcore_buffer_new(7)?; drop ⟨t0⟩;` — the allocating chain
`PIPE-INTRINSIC-STAGE` §3.4 measured as *unreleasable*, now spellable end to end,
with the pipeline releasing the temporary it invented. **No other keyword-shaped
head joined**, on a two-part criterion applied to the whole census (§2), and the
decision's real content is §3: *the difference between a releasing entry and a
rendering one is entirely in the argument for admitting it, not in the
desugaring or in any rule the pipe gained.* Terminality is the pre-existing
"only a call stage may feed a later stage" rule **instantiated**, not extended.

Twenty-five new tests and ten new audited Lean theorems, **no new diagnostic
code, no new reserved word, no checker edited, and one line of table data plus
its refusal messages in production**. Two pins moved by name, deliberately
(§6). Corpus-neutral by construction: no gateway, audit, contract or provider
file is touched.

---

## 1. The decision

A stage-table entry names a statement head, the number of closed selector words
it takes before its operand, and — as of this card — **what it does with the
value**:

```rust
const SINK_STAGES: &[(&str, usize, StatementRole)] = &[
    ("print", 1, StatementRole::Renders),
    ("log",   2, StatementRole::Renders),
    ("drop",  0, StatementRole::Releases),
];
```

That is the whole production change to the table. The role decides only what a
refusal *says*; it decides nothing about what is emitted (§3).

## 2. The census, and the criterion that closed it

The brief asked which *other* keyword-shaped heads join. The answer is **none**,
and the criterion is what makes that an argument rather than a preference. An
entry earns its place only when both hold:

1. the head's statement form is exactly `HEAD SELECTOR* OPERAND ";"`, where every
   selector is a word from a **closed, route-independent** set; and
2. some stage can actually *feed* it a value **no reader can name** — i.e. a
   route it belongs to has a call-shaped intrinsic producing what it takes.

Condition (1) is the table's mechanical shape. Condition (2) is the one that
does the work: without it an entry buys nothing but a second spelling of a
statement over a name the reader already wrote, and a second spelling with no
new expressive power is exactly what card `PIPE-INTRINSIC-STAGE` §1.1 declined
to add for `render`.

| Head | Route | (1) | (2) | Verdict |
|---|---|---|---|---|
| `drop N;` | every route with custody | yes — no selectors | **yes**: `qcore_buffer_new`, `qcore_error_payload` (task routes) and `bytes(N)` (`source.buffer.v1`) bind owned resources to hygienic temporaries | **admitted** |
| `spawn S W(ARGS)?` | task routes | **no** — the operands are neither an argument list nor a bare name | — | refused, unmoved |
| `join move H?` | task routes | **no** — same | — | refused, unmoved |
| `move NAME;` | sequential | yes | — | refused on a **third** ground: `move` already spells this grammar's own consume prefix (`\|> move STAGE`), and one spelling may not carry two meanings |
| `read N;`, `write N;`, `wait N;`, `use N;` | `source.mutex.guard.v1` | yes | **no** — that route's model has **no call-shaped intrinsic at all**, so no stage can ever produce their operand | refused, unmoved |
| `open N;`, `close N [unchanged];`, `grow N guards M;`, `read N with G;` | same, in-section | **no** — a guard name is an open name, not a closed selector word; `close`'s tail is optional, so its arity is not fixed | **no** | refused, unmoved |
| `share N` | task routes | yes | **no** — a share is a capture expression, not a value a stage produces | refused, unmoved |
| `bound`, `overhead`, `peak`, `effort`, `credit`, `retained` | epoch plan clauses | yes | **no** — twice: they are declaration clauses, and their operands must be check-time constants, which no stage result is | refused, unmoved |
| `return NAME;` | every route | yes | **no**, by the second half: a chain's result is **already** nameable, because a bound pipeline (`let NAME = … ;`) binds exactly it. An entry would be a *control-flow* entry — a third kind — bought for a spelling the language has | refused, unmoved |

Each refusal is pinned by measured code in
`the_other_keyword_shaped_heads_did_not_join_the_table` and
`a_mutex_route_statement_head_is_not_a_stage`. The codes differ per head
(`Q000` for `spawn`, whose desugared form does not parse; `Q500` for the rest),
and that is the point: **the pipe states no rule about any of them.** Each keeps
the refusal it already had, from whichever reader reaches it first.

### 2.1 A census row the wave-22 card did not have

`source.buffer.v1` landed in the same wave as `PIPE-INTRINSIC-STAGE` and is not
in its census. Measured here: its acquisition `bytes(N)` and its extent read
`extent(B)` are **call-shaped**, so they are call stages under the existing
definition and `8 |> bytes |> drop;` is an allocating chain on that route with no
adapter tail at all (`the_standard_buffers_allocating_chain_is_spellable_too`).
One table entry therefore reaches every route that releases, because every one of
them spells its release with the same reserved `drop` — that word being
`OwnershipOp::Drop` itself rather than a per-route synonym.

## 3. What "a different kind of entry" turns out to mean

This is the card's finding, and it is smaller than the wave-22 note implied.

**It is not a difference in the desugaring.** A releasing stage and a rendering
stage both emit `HEAD SELECTOR* OPERAND ";"`, both are a pure permutation of the
reader's own tokens with nothing synthesized, and both bind no result. Lean:
`a_release_and_a_render_emit_by_the_same_rule` states the two rewrites side by
side with the head and selector count abstracted away, so the claim is the
mechanism's rather than an instance's.

**It is therefore not a difference in terminality either.** The brief required
that "a `drop` stage must be terminal (a released buffer cannot feed a later
stage)". It is — and the rule that makes it so is the one card `PIPE-CHAIN`
already proved, *applied*:

> `a_releasing_stage_in_a_non_final_position_refuses` is literally
> `a_non_final_stage_that_produces_nothing_refuses` instantiated at a `drop`
> stage.

A released value cannot feed a later stage **because there is no value to feed**.
The pipe needs, and gains, no notion of custody. That coincidence is load-bearing
enough to be stated rather than relied on: if a future entry ever released *and*
produced a result, this card's terminality would not follow and the entry would
need its own argument.

**It is a difference in the argument for admitting the entry**, which is §2, and
in **what a refusal says**. A reader who writes `b |> drop |> log info dec;`
needs to hear that the value is gone, not that `drop` "is a sink":

| Shape | Code | Message |
|---|---|---|
| `b \|> drop \|> log info dec;` | `Q037` | ``…`drop` releases the value piped into it and binds nothing, so it cannot feed a later stage: a released value is gone. Make it the pipeline's last stage`` |
| `let x = b \|> drop;` | `Q036` | ``…`drop` produces no value, so a bound pipeline cannot end in it`` (unchanged wording — it is the pre-existing condition) |
| `\|> move drop` | `Q036` | ``…`drop` releases the value piped into it, which is what consuming it means here; it takes no `move` `` |
| `b \|> drop dec;` | `Q036` | ``…`drop` takes the value piped into it and nothing else; this stage names 1 further word(s)`` |

No code was added, moved or changed; `Q036`–`Q038` keep their §L21 conditions,
restated through the §L21 process on the `OR-AND-LOWERING`/`PIPE-INTRINSIC-STAGE`
precedent.

**And the pipe says nothing about what happens next.** `b |> drop; b |> drop;` is
the ordinary double release, refused by the route's own checker at its own code —
measured equal to the hand-written double release's diagnostics, not merely
non-empty (`releasing_the_same_binding_twice_refuses_under_the_unedited_checker`).
That is card `PIPE-CHAIN`'s discipline holding on a releasing stage: custody
flows through the desugared statements under the **unedited** checkers, so
linearity is inherited by construction.

## 4. What is admitted, and how it is pinned

Every positive claim below is **byte-equality of emitted C against the
hand-written statement**, plus equality of the reference observation — never "it
compiles".

| Test | What it measures |
|---|---|
| `a_release_is_a_pipe_stage_on_the_task_route` | `returned \|> drop;` ≡ `drop returned;` on `ScopedOwnedTasksV1` |
| `the_release_pipeline_and_its_hand_written_twin_are_the_same_program` | `syntax_equivalent`: the whole post-parse token stream is equal |
| `a_release_is_a_pipe_stage_on_the_composed_task_route` | the same on `ComposedStdIoTasksV1` |
| `a_release_is_a_pipe_stage_on_the_sequential_route` | the same on `SequentialCleanupV1`, reached through the statement reader `PIPE-INTRINSIC-STAGE` §4 repaired |
| `a_release_is_a_pipe_stage_on_the_standard_buffer_route` | the same on `StandardBufferV1` |
| **`the_allocating_chain_is_spellable_end_to_end`** | `7 \|> qcore_buffer_new? \|> drop;` ≡ `let scratch = qcore_buffer_new(7)?; drop scratch;` — **the payoff** |
| `an_error_payload_take_can_be_released_in_the_same_chain` | `failure \|> qcore_error_payload \|> drop;` — the census's one custody-*transferring* call-shaped intrinsic, taken and released in one chain |
| `the_standard_buffers_allocating_chain_is_spellable_too` | `8 \|> bytes \|> drop;` on `source.buffer.v1` |
| `the_rendering_stages_are_unchanged` | the two entries that were in the table keep their arity, their last-stage-only rule and their **own** message |

### 4.1 The refusals the brief named, each by name

| # | Shape | Refusal | Test |
|---|---|---|---|
| a | drop in a non-final position | `Q037`, releasing message | `a_release_cannot_feed_a_later_stage` |
| b | drop feeding a drop | `Q037` — refused by terminality alone, before any checker sees a statement | `a_release_cannot_feed_a_later_release` |
| c | **double drop** across two statements | the route's own `Q600`, **equal to the hand-written double release's** | `releasing_the_same_binding_twice_refuses_under_the_unedited_checker` |
| d | **drop of a borrow** | `ScopedReadLoansV1`'s own refusal, equal to the hand-written form's | `releasing_a_borrow_is_the_routes_own_refusal` |
| e | drop of a scalar | `Q500` "drop requires an owned buffer" — the route's question, not the pipe's | `releasing_something_that_is_not_owned_is_the_routes_own_refusal` |
| f | bound pipeline ending in a release | `Q036`, pre-existing condition | `a_bound_pipeline_cannot_end_in_a_release` |
| g | `\|> move drop` | `Q036`, releasing message | `a_release_stage_takes_no_move_prefix` |
| h | `\|> drop dec` | `Q036`, releasing arity message | `a_release_stage_names_no_further_word` |
| i | `\|> flush` | `Q037` unchanged — the operandless table is untouched | `the_operandless_stage_is_still_refused` |
| j | an allocating chain **not** ending in a release | `Q600`, unchanged: the entry admits a release, it does not weaken the leak rule | `the_allocating_chain_still_leaks_when_it_does_not_end_in_a_release` |
| k | `move returned \|> drop;` — a *moved operand* | `Q013`, see §5.3 | `a_moved_operand_piped_into_a_release_refuses` |

(c) and (d) are stated as *equality with the hand-written form's diagnostics*
rather than as an expected code, because what the card claims is that the pipe
changed nothing about them.

## 5. Two measurements the fixtures forced, recorded rather than absorbed

**5.1 A propagating allocation mid-body refuses for a reason that is not the
pipe's.** The first spelling of the payoff test placed
`7 |> qcore_buffer_new? |> drop;` inside a match arm where another buffer was
live, and it refused at `Q600`. The cause is the `?`: its propagation path
abandons whatever the enclosing arm already owns, exactly as the hand-written
pair does there. The fixture was moved to the **entry**, the one point where
nothing else is live, and the helper says so in its doc comment. Recorded because
the wave-22 leak fixture (§3.4) sits in the ambiguous position, and its claim —
that the temporary is what leaks — is therefore not isolated by that test alone.
This card's `the_allocating_chain_still_leaks_when_it_does_not_end_in_a_release`
is the isolated form of the same measurement.

**5.2 `NAMED-TEMPORARY-IN-EMITTED-C`: the hygienic temporary's name is
observable on `source.buffer.v1`.** `PIPE-INTRINSIC-STAGE` §2.1 could compare a
chain against a twin binding any name a reader would choose, because the task
backend names slots and the source identifier reaches no emitted byte. That is
**not true of every backend**: `source.buffer.v1` derives its C identifiers from
the source binding, so `8 |> bytes |> drop;` emits
`unsigned char *b_qcore__pipe0 = …` and the reserved prefix reaches the emitted
C. The byte-equality pin on that route is therefore against the twin that spells
`qcore__pipe0`, and the difference is pinned in its own test
(`the_temporarys_name_is_observable_on_the_standard_buffer_route`) — the two
programs differ in **exactly** that identifier and in nothing else, and the
reference execution does not observe it at all. This is a statement about the
backend, not about the pipe, and it weakens nothing: follow-on
`PIPE-BUFFER-TEMPORARY-NAME`.

**5.3 `MOVED-OPERAND-NAMES-A-KEYWORD`: `move returned |> drop;` refuses at
`Q013`.** A statement stage appends the incoming tokens verbatim, so that
pipeline is `drop move returned;` — not the release statement, and it must
refuse, since admitting it would be a second spelling of `returned |> drop`. It
does refuse, and for a true reason, but **at name resolution**: a `drop`'s
operand is read as its first word, which here is the token `move`, so `Q013`
reports `move` as an unknown binding. The message therefore names a token the
reader wrote as a keyword. It is recorded rather than repaired because the
obvious repair — having the pipe inspect a *releasing* stage's operand — is a
rule the two rendering entries do not have and would be asymmetric with. Pinned
by measured code so a later change to it is deliberate. Follow-on
`PIPE-MOVED-OPERAND-MESSAGE`.

## 6. `PIPE-INTRINSIC-EXAMPLE`, and the two pins moved by name

**The example.** `examples/threads/owned_buffer.qcore`'s reading pair
`let local = qcore_buffer_read(second); local |> log info dec;` is now the chain
`second |> qcore_buffer_read |> log info dec;`, and its `drop second;` is now
`second |> drop;`. `PIPE-INTRINSIC-STAGE` §5 asserted the first would "compile to
identical C"; that claim is **verified here rather than carried**
(`the_published_example_emits_what_its_hand_written_form_emits` compiles the
published file and the form it replaced and compares `c_source` and
`reference_observation`). The example's five byte-compared tests and both
transcripts are reconfirmed unchanged — `thread_examples.rs` (9 tests) and
`obs_reordering.rs` (10 tests) both pass, including
`every_published_example_matches_its_reference_execution_under_sanitizers` and
`the_published_examples_still_match_byte_for_byte`.

The **post-join** read is deliberately left as a statement pair, with a comment
in the file saying why: the number is used twice — rendered and returned — and a
pipeline is linear, so a chain consuming it could not also return it. Pinned by
`the_published_example_still_carries_the_statement_form_where_a_chain_cannot_reach`,
so a later card cannot "finish" the rewrite without meeting the reason.

**The two pins.** Both failed, and both were **rewritten rather than deleted**,
on the `TRACE-STDIO` precedent:

* `compiler/tests/pipe_intrinsic_stage.rs::drop_is_not_a_pipe_stage_on_the_task_route`
  → `drop_is_now_a_pipe_stage_on_the_task_route`, which compiles §3.4's chain and
  re-asserts that terminality is the sink rule.
* `proof/QCore/PipeDesugaring.lean::keywordShapedIntrinsics` lost `"drop"`, and
  `no_intrinsic_is_a_sink_or_operandless` therefore covers a shorter list. The
  removal is documented in the definition's own docstring, and
  `the_release_is_the_only_keyword_shaped_head_in_the_table` is the theorem that
  now says both halves — the rest of the census is outside the table, the
  releasing list is inside it, and it has length one.

## 7. What did not move

* **No diagnostic code was added, moved or changed**, and no reserved word was
  added — `drop` has been reserved since the language had one (`frontend.rs`'s
  `is_reserved_source_word`), so standing constraint 14's fixture census does not
  apply.
* **The checkers are unedited.** No custody, effect, linearity, concurrency or
  emission rule changed. Every refusal in §4.1 except (a), (b), (g) and (h) —
  which are the pipe's own shape rules — predates this card.
* **`produces_a_result` is unchanged.** It still reads "is this a call stage",
  and a releasing head answers `false` through the same table lookup a rendering
  head does.
* **The operandless table is unchanged**: `flush`, and nothing else.
* **Corpus-neutral.** No gateway, audit, contract, provider or `docs/reports`
  artifact is touched; leak diagnostics stay at **272** by construction, since no
  C-gateway file is in the diff.

## 8. Deviations from the brief, recorded

1. **The brief allowed for a new diagnostic code ("otherwise allocate via §L21
   and record"). None was needed.** `Q037` already refuses a non-final stage that
   binds no result, and a release binds no result. Taking a new code would have
   asserted a distinction the mechanism does not make. What §L21 records is a
   **restatement** of the `Q036`/`Q037` conditions, through the §L21 process.
2. **The brief's "§3.4's allocating chain becomes spellable end to end" is
   narrower than it sounds, and the report says so.** §3.4's *own* shape —
   `7 |> qcore_buffer_new? |> qcore_buffer_read;` — is **still refused**, and must
   be: a pipeline is linear, so a chain cannot both read a buffer and release it;
   the buffer would have to be named twice. What became spellable is the
   allocate-and-release chain (`… |> drop`), which is the chain that gives the
   unnameable temporary a release for the first time. Reading *and* releasing one
   resource in one chain is the already-named follow-on `PIPE-TAP`.
3. **The sibling example card grew one line beyond its brief.** Converting
   `drop second;` to `second |> drop;` is this card's own demonstration of the new
   entry in a published program, not `PIPE-INTRINSIC-EXAMPLE`'s ask. It is
   byte-identical and pinned like the rest.

## 9. Named follow-ons

| Name | What it is, and what it needs |
|---|---|
| `PIPE-TAP` | *(existing, sharpened)* a stage that uses the piped value and passes it on. It is what would make **read-then-release** one chain; §3.4's own shape needs it, not a table entry. |
| `PIPE-BUFFER-TEMPORARY-NAME` | §5.2. `source.buffer.v1` publishes `qcore__pipe0` into the emitted C's identifiers. Needs a decision: either that backend names slots as the task one does, or the reserved prefix is declared observable and the hygiene note says so. Nothing is unsound; a reader of the emitted C sees a name no source line spells. |
| `PIPE-MOVED-OPERAND-MESSAGE` | §5.3. `move b \|> drop;` refuses at `Q013` naming `move` as an unknown binding. Needs a decision about whether a statement stage may inspect its operand at all — which is a question for `print` and `log` too, not only for the releasing entry. |
| `PIPE-RELEASE-CENSUS-DRIFT` | §2's criterion is enforced by a test over five heads and by one Lean list. A route landing a **new** call-shaped intrinsic changes condition (2) for the heads of *that* route, and nothing fails if it is not revisited. Needs the census to be derived from the route table rather than written down. |
| `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY`, `PIPE-ASSIGN`, `PIPE-BINDER-SPAN`, `PIPE-BORROW-SINK`, `SEQ-SCALAR-RESULT` | *(unchanged)* |
| `COMPOSE-WORKER-LINES`, `COMPOSE-TASK-BORROW`, `COMPOSE-TASK-EXTERN` | *(unchanged)* |
| `PIPE-DROP-STAGE`, `PIPE-INTRINSIC-EXAMPLE` | **retired** by this card. |

## 10. Evidence

### 10.1 The standing gate

Run in this worktree on main `52e3889c` plus this card, as one whole-workspace
invocation (no sibling card was competing for the host during the run).

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | checked out `d82464a2…`; **needed** — a fresh worktree has no submodule, and `libevent_submodule_matches_the_audited_revision` fails until it does. Environmental, not a regression |
| `cargo test --workspace` | **exit 0** · 198 suites · **3,605 passed, 0 failed, 8 ignored** (baseline 3,580 + this card's 25) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | exit 0 · **95 Lean audits passed**, no project axiom (the ten new theorems use only `propext`, `Classical.choice` and `Quot.sound`) |
| `python3 -m unittest discover -s scripts/tests` | **193 passed** |

No constraint-16 flake was tripped; nothing needed an isolated re-run. The whole
suite ran in one invocation, so the four load-sensitive provider/host rows were
exercised under exactly the load the gate normally gives them and passed.

A doc-comment-only edit to `crates/qcore-frontend/src/pipe.rs` landed after that
whole-workspace invocation started. `fmt`, `clippy` and the six suites that read
the pipe, the published examples and the buffer route (`pipe_drop_stage`,
`pipe_intrinsic_stage`, `pipe_operator`, `thread_examples`, `obs_reordering`,
`std_buffer` — 157 tests) were re-run against the final tree and pass; no
executable line changed between the two states.

### 10.2 Artifacts

| Artifact | Where |
|---|---|
| Tests | `compiler/tests/pipe_drop_stage.rs` (25 tests) |
| Moved pin | `compiler/tests/pipe_intrinsic_stage.rs::drop_is_now_a_pipe_stage_on_the_task_route` |
| Production change | `crates/qcore-frontend/src/pipe.rs` — the `SINK_STAGES` table, `StatementRole`, and the role-dependent refusal messages |
| Example | `examples/threads/owned_buffer.qcore`, `examples/threads/README.md` (its row) |
| Lean | `proof/QCore/PipeDesugaring.lean` §5.7 (10 theorems), registered in `proof/AuditPipeDesugaring.lean` |
| Spec | `docs/QCORE_SPEC_r0_3.md` §L21 `Q036`–`Q038` row (restatement) |
| Grammar | `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02h |

## 11. Draft increment paragraph

> The releasing pipe stage (PIPE-DROP-STAGE, with PIPE-INTRINSIC-EXAMPLE):
> `drop` joins `source.pipe.v1`'s closed statement-stage table as its third head
> and its only **releasing** one, so `b |> drop;` is `drop b;` and
> `7 |> qcore_buffer_new? |> drop;` is the pair
> `let ⟨t0⟩ = qcore_buffer_new(7)?; drop ⟨t0⟩;` — the allocating chain card
> `PIPE-INTRINSIC-STAGE` §3.4 measured as unreleasable, now spellable end to end,
> with the pipeline releasing the temporary it invented. The decision the wave-22
> card withheld turns out to be **smaller than its framing**: a releasing entry
> and a rendering entry emit by one rule and bind no result by one rule, so a
> released value cannot feed a later stage *because there is no value to feed* —
> terminality is `a_non_final_stage_that_produces_nothing_refuses` instantiated,
> not extended — and what actually differs is the argument for admitting the
> entry and what a refusal says. **No other keyword-shaped head joined**, on a
> two-part criterion applied to the whole census: the statement form must be
> `HEAD SELECTOR* OPERAND` with closed selectors, and some stage must be able to
> feed it a value no reader can name. That excludes the mutex heads (their route
> has no call-shaped intrinsic at all), the epoch plan clauses (check-time
> constant operands), `return` (a bound pipeline already names a chain's result)
> and `move` (which already spells the pipe's own consume prefix), each keeping
> the refusal it had, at whichever reader reaches it first. Custody flows through
> the **unedited** checkers: a piped double release and a drop of a borrow refuse
> with exactly the diagnostics their hand-written forms produce, and an
> allocating chain that does not end in a release still leaks at `Q600`. Two
> measurements are recorded rather than absorbed — a `?` mid-body abandons what
> the enclosing arm owns, which is why the payoff fixture sits at the entry, and
> `source.buffer.v1`'s backend **publishes the hygienic temporary's name into the
> emitted C** where the task backend does not (`NAMED-TEMPORARY-IN-EMITTED-C`,
> follow-on `PIPE-BUFFER-TEMPORARY-NAME`). `examples/threads/owned_buffer.qcore`
> now spells its parent-side read as the intrinsic chain and its release as a
> stage, with the wave-22 claim that this emits identical C **verified rather
> than carried**, and keeps the post-join read as a statement pair because the
> number is used twice and a pipeline is linear. Two pins failed and were
> rewritten by name, not deleted. Twenty-five new tests, ten new audited Lean
> theorems, **no diagnostic code added, moved or changed and no reserved word**;
> §L21's `Q036`–`Q038` row and grammar §G02h restate the condition in the same
> change. Corpus-neutral by construction. Evidence:
> `docs/reports/pipe-drop-stage/README.md`.
