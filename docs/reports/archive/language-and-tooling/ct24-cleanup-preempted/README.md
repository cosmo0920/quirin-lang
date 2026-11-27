# CT24-CLEANUP-PREEMPTED — the preempting refusal carries the exit rule's own judgment

Wave 30, card `CT24-CLEANUP-PREEMPTED`, the follow-on the wave-26 CT lane named
at `docs/reports/ct-wire-lane/README.md` §3.5 and §10. Base: `main` at
`30aee0b2` (the wave-29 close).

**What moved, in one sentence.** The `Q600` `ScopeBusy` refusal that preempts
CT24's cleanup shape — an early exit inside a thread scope while a spawned
child still holds a read share of the frame's owner — is now rendered from the
exit rule's own loan judgment, so its message names the owner the preempted
cleanup would destroy and the borrower whose join it precedes (the construct
CT24 names, destroy-before-join); before this card that judgment's result was
discarded and the refusal read `ScopeBusy(5)` bare, attributable only to the
scope.

**Nothing is admitted, nothing stops refusing, and nothing is reordered.** The
refused set is unchanged — proved, not asserted: `QCore.TaskExecution` gains
the attribution relations, and `attribution_never_moves_the_refusal` pins that
whether the exit refuses is the scope obligation's own decision, whatever the
loan judgment says. Zero diagnostic codes allocated, zero codes moved, zero
conditions widened (the Q-space icebox is untouched), zero reserved words, no
parser change, no ledger change, no backend change. The task obligation that
fires first **still fires first**; what changed is that its refusal no longer
throws the CT24 fact away.

---

## 1. The problem, as the wave-26 lane measured it

CT24 reads *"Cleanup destroys an owner before child join"*, expected
**Reject** (spec section 55; T00 bucket `DECIDABLE-NOW (with a distinguishing
fixture)` — the distinction being that scope-exit cleanup is a different
mechanism from an explicit `drop`). The lane's two measured shapes (§3.5):

| Shape | Measured (wave 26) |
|---|---|
| explicit `drop buffer;` before the join, child holding `read buffer share 512` | `Q600`: `LoanOutstanding { resource: 1, outstanding: 512, borrowers: [2] }` — attributable to the `drop`, not to cleanup |
| a `?` inside the thread scope with the child unjoined — the shape whose scope exit would run cleanup over the owner | `Q600`: `ScopeBusy(5)`, at the `?` itself — the task obligation, nothing about the owner |

The lane's conclusion: the distinguishing fixture is unwritable, because the
cleanup path is refused first on the task obligation and the refusal is never
attributable to the construct CT24 names.

**The judgment already existed and was being discarded.** The reference
execution's exit rule (`task_execution.rs::State::exit`) carries its own loan
backstop — *"return/error leaves resource N lent to an unjoined child"* —
recorded since card T04 as a redundant backstop precisely because the scope
rule reaches the refusal first (`read_loans.rs`, the test
`a_path_that_never_joins_its_borrower_cannot_leave_the_scope` said so in its
doc comment). That backstop's search is exactly the CT24 fact: a frame
resource with an outstanding read share, whose borrower is unjoined by the
ledger's own transaction (`join` reclaims the child's shares in the same
transaction that consumes the handle; a failed join keeps its scope
inexitable). This is the CT47 shape one wave later: the judgment computes the
right refusal at the very construct, and the rendering throws it away for a
catch-all — there the backend's `Q500`, here the bare `Debug` of the ledger
error.

## 2. What refuses where — before → after

Every row measured through the real pipeline (`quirin check`, and the same
shapes pinned by `crates/qcore-frontend/tests/cleanup_preempted.rs`):

| Shape | Before | After |
|---|---|---|
| `?` inside the scope, child unjoined, owner lent (`read buffer share 512`) | `Q600`: `ScopeBusy(5)` | `Q600`, same span: `ScopeBusy(5): cleanup preempted — this exit would run the frame's cleanup over resource 1 while task(s) [2] still hold a read share of it, destroying the owner before the child's join; the scope refuses the exit with that loan outstanding` |
| early `return` inside the scope, same loan outstanding | `Q600`: `ScopeBusy(5)` | the same refined message — an early return's teardown is the same preempted cleanup point |
| explicit `drop buffer;` before the join | `Q600`: `LoanOutstanding { resource: 1, outstanding: 512, borrowers: [2] }` | **byte-identical, untouched** (attributable to the `drop`, which is exactly why the lane declined to map CT24 on it) |
| `?` inside the scope, child unjoined, **no loan** (CT13's shape) | `Q600`: `ScopeBusy(4)` | **byte-identical** — no owner's cleanup is preempted, nothing is attributed |
| join, `drop buffer;`, then the same `?` | admitted | admitted — **unchanged** (the boundary control: supplying the join and the release ahead of the early exit is exactly what flips the verdict) |

The two standing substring pins on `"ScopeBusy"` moved deliberately to the
refined messages (`read_loans.rs` — the whole message verbatim — and
`read_shares.rs`, where the first join has already reclaimed its share so the
attribution names the second borrower alone, `task(s) [3]`). Every other
`ScopeBusy` pin in the tree (CT13's fixtures in `ct_corpus.rs` and
`source_tasks.rs`, the core IR test, the prelude doc test) is a no-loan shape
and still matches byte-identically.

## 3. The change, mechanically

`crates/qcore-core/src/task_execution.rs`, two small moves:

* The exit rule's loan search is factored into
  `State::outstanding_lent_resource` — the first frame resource with
  `lent_share > 0`, plus its borrowers from `loan_borrowers` — and `exit`'s
  backstop now calls it, its message **byte-identical**. One judgment, two
  readers, so the fact named at the preempting gate is provably the fact the
  backstop would name.
* The `ExitScope` instruction's refusal is rendered by
  `State::scope_exit_refusal`: on `TaskLedgerError::ScopeBusy`, if
  `outstanding_lent_resource` resolves, the message is the refined one above
  (prefix `ScopeBusy(..)` kept, so the refusal's identity is still legible);
  every other ledger error, and a busy scope with no lent frame resource,
  keeps `format!("{error:?}")` byte for byte.

The ledger (`scoped_tasks.rs`) is untouched: `exit_scope` still refuses
exactly what it refused, and the interpreter state it is read against is the
pre-refusal state (`exit_scope` is transactional and rolls back on error).
The lowering is untouched: every exit form — `?`'s synthesized error block,
`raise`, `return`, the allocation- and spawn-failure arms — already runs the
same synthesized `ExitScope` chain, which is why the attribution covers them
uniformly.

### Attribution or semantics?

**Attribution only, and stated as such** (the card brief asks which). The
refused set of programs did not move, no code moved, no condition widened, no
span moved: this is constraint 1's "fixing where a diagnostic is attributed is
not a code change" clause — narrower than CT47, which moved `Q500 → Q600` and
was semantic on constraint 1's own terms. The message text is the only
observable change, and the §L21 `Q6xx` row states the non-widening explicitly
rather than taking it silently. Lean parity is supplied anyway (below),
because the claim "the attribution cannot move a verdict" is exactly the kind
of claim this project proves rather than asserts.

## 4. Lean (invariant 6)

`proof/QCore/TaskExecution.lean` gains the attribution relations —
`ExitAttribution`, `outstandingLentResource` (the exit rule's own first-lent
search, as a recursive definition in the file's own style), and `scopeExit`
(the judgment consulted only inside a refusal the obligation already made).
Registered in the existing `proof/AuditTaskExecution.lean`; the audit count
stays **118**.

| Theorem | What it pins |
|---|---|
| `attribution_never_moves_the_refusal` | `(scopeExit busy lent resources).isSome = busy` — whether the exit refuses is the scope obligation's decision alone; the loan judgment selects only the message |
| `an_admitted_exit_is_never_attributed` | the judgment is read only inside a refusal (`scopeExit false … = none`, definitional) |
| `a_named_owner_is_a_lent_frame_resource` | the judgment cannot invent an owner: what it names is a frame resource with an outstanding share |
| `the_preempting_gate_reads_the_backstops_own_judgment` | the refined attribution appears **exactly** where the backstop's judgment resolves a resource — the two gates read one judgment |
| `an_unlent_frame_keeps_the_plain_refusal` | a busy scope with no lent frame resource renders plain `scopeBusy` — CT13's shape stays byte-identical by theorem, not by luck |
| `ct24s_cell_is_attributed_to_the_destroy_before_join` | **CT24's cell**: a refused exit whose frame holds a lent owner is always attributed to the destroy-before-join, naming a genuinely lent frame resource |

## 5. Scope: what is landed and what is measured boundary

**Landed:** the attribution, for every exit form the task route can spell —
`?`, `return`, `raise`, and the synthesized failure arms all run the same
exit chain, so the refined refusal covers the whole family. That is the
re-attribution half the card brief prefers, and the measurement supported it:
no composition slice was needed, because the preempting refusal and the CT24
judgment already meet in one place.

**Measured boundary, published rather than papered over:**

* **This route still has no cleanup construct.** The checked task profile
  destroys nothing at scope exit — releases are explicit `drop`s, and an exit
  holding an owned buffer is refused as a discard (measured: moving the `?`
  after the join but before the `drop` refuses with *"return/error discards a
  handle, unconsumed Result or owned buffer"*). The refusal this card refines
  is therefore about the exit that **would** run cleanup over the owner — the
  wave-26 lane's own words for the shape — not about an executed destroy. A
  lane that requires a literal executed cleanup-destroy (a `Q202`-band
  CleanupPack composed with tasks) will not find one here; that composition
  remains unlanded and is the honest residual of this card
  (`CT24-CLEANUP-TASK-COMPOSITION` would be its name if a future card takes
  it; CT25 wants the same composition, per §3.6 of the lane report).
* The preemption itself is **kept**: the task obligation still refuses the
  scope exit first, deliberately — the card was forbidden from reordering it,
  and the exit rule's backstop stays in place, unedited in behaviour, for the
  exits that reach it.
* The explicit-drop shape keeps its `LoanOutstanding` refusal untouched; it
  remains attributable to the `drop` and remains the wrong shape to map CT24
  on, exactly as the lane recorded.

## 6. CT24 readiness statement

For a future mapping lane (this card edits neither
`docs/reevaluation/ct_manifest.json` nor
`docs/reviews/r0_3/acceptance_matrix.json` — CT24 is a key of the derived
`newly_decidable_not_mapped_reasons` map, constraint 25, and its row handling
belongs to a mapping lane):

* The reject half of CT24 — *"cleanup destroys an owner before child join"
  must refuse* — is now attributable at the construct: `Q600`, fixture shape
  a `?` (or early `return`) inside `thread scope` with
  `spawn s reader(read buffer share 512)` unjoined, message naming the owner
  the preempted cleanup would destroy and the borrower whose join it
  precedes. The wave-26 shapes are re-measured and pinned by
  `crates/qcore-frontend/tests/cleanup_preempted.rs`: the drop shape
  byte-identical, the `?` shape carrying the judgment's message, the no-loan
  busy scope byte-identical, the join-then-drop-then-`?` control admitted.
* An attribution control and a boundary control in the wave-26 style come
  free: the same program with the join and release ahead of the `?` is
  admitted whole, and the no-loan variant shows the attribution is the loan's
  and not the scope's.
* What a lane must still decide: whether a refusal at the exit that *would*
  run cleanup satisfies CT24's "cleanup destroys" premise, given that this
  route's checked semantics never executes a destroy at scope exit (the
  measured boundary above). If the lane requires the literal executed
  destroy, CT24 stays unmapped until a cleanup/task composition lands, and
  that is the honest answer. Both facts are published here and in
  `acceptance.json`'s hand-kept CT24 reason (updated by this card; the
  derived matrix copy is a mapping lane's to regenerate).

## 7. Files

* `crates/qcore-core/src/task_execution.rs` — `outstanding_lent_resource`
  (the factored judgment; `exit`'s backstop message byte-identical),
  `scope_exit_refusal`, and the `ExitScope` arm rendering through it.
* `crates/qcore-frontend/tests/cleanup_preempted.rs` — the card's five pinned
  shapes (new file).
* `crates/qcore-frontend/tests/read_loans.rs`,
  `crates/qcore-frontend/tests/read_shares.rs` — the two standing `ScopeBusy`
  substring pins moved deliberately to the refined messages.
* `proof/QCore/TaskExecution.lean`, `proof/AuditTaskExecution.lean` — §4.
* `docs/QCORE_SPEC_r0_3.md` §L21 `Q6xx` row — the non-widening, stated.
* `docs/reviews/r0_3/acceptance.json` — the hand-kept CT24
  `newly_decidable_not_mapped_reasons` row updated to the post-card
  measurement (staleness-checked only against the manifest; no derived field
  touched).
* `DEVELOPMENT.md` — the wave-30 increment paragraph.
* `measure/` (worktree-private, not committed) — the four probe programs and
  their `quirin check` transcripts.

## 8. What this does not establish

* **No mapping.** CT24 stays unmapped (42/50 mapped, unchanged); mapping is a
  mapping lane's decision and its row is in a derived, test-compared field
  this card may not regenerate.
* **No cleanup construct on the task route.** Nothing here composes the
  `Q202` cleanup machinery with tasks; the route's exits still discharge
  custody explicitly or refuse.
* **No new capability, no admitted program.** The admitted set is exactly
  what it was; the boundary control was admitted before this card too.
* **Nothing else moved.** No gateway, contract, audit or provider file is
  touched; the Cycle 19 corpus rows and the leak pin (266) are untouched by
  construction.

## 9. Gate

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | **4,450 covered (4,445 pre-card + this card's five new tests), 8 ignored: 4,449 passed in the single run, 1 failed** (cargo exit 101 from that one row) — `enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`, the `Io(Broken pipe)`-under-load shape of constraint 16's recorded rows in that same binary, with sibling worktree agents' suites running on the machine during the measurement. Re-verified in isolation per that constraint: `cargo test -p qcore-verifier-host --test enforced_process_limits -- --test-threads=1` → **12 passed, 0 failed**. Both facts reported; no assertion touched |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 118 Lean audits** (the baseline count; the new theorems register in the existing `AuditTaskExecution.lean`) |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** (no matrix file touched; no audit file added) |
