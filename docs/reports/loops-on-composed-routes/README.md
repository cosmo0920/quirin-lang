# Card `LOOPS-ON-COMPOSED-ROUTES` (wave 39) — a `for` beside a borrow or a lend, with the window computed over iterations

**Status: closed (2026-09-18).** Base `e878b362` (`origin/main` at branch
time). Rust plus audited Lean (`proof/QCore/LoopWindows.lean`,
`proof/AuditLoopWindows.lean`, registered in `proof/QCore.lean`); mutations
shipped (`mutations.sh`, `mutations.log`); emission census shipped
(`emission_census.py`, `emission-census.json`); gate in `gate.json` beside this
file with the steps' exits under `gate/`.

## 1. Scope

Before this card both routes that hold a loan refused `source.loop.v1`'s
`for` beside it, by name:

* the buffer route, in an entry that lends — `SLICE-PERMISSIONS-LOOP-WINDOW`,
  `Q051`, the refusal `check_buffer_program` raised *before* `lend_windows` ran;
* the composed string/buffer route — `STRING-CONCAT-LOOP-WINDOW`, `Q068` at the
  lowering (a `for` is a nested block) and `Q077` in the model's own guard.

Each refusal defended one premise, stated in standing constraint 15h: *the
window is exact because the route is one flat statement sequence*. The window
was an interval of **top-level** statement indices, `[opening, through]`, with
`through` the last top-level statement naming the loan; a `for` body contributed
nothing to it. `STABILITY-REEVAL-3` §4 measured what lifting either refusal
without rewriting that computation would admit: a loan used only inside a loop
would close before the loop opened, and every owner write in the body would
pass as "outside every live loan" — the admitting arm.

This card replaces the computation by the **window function over iterations**
(§3), proves it sound relative to `LOOP-V1`'s unrolling (§4), lifts both
refusals exactly where the theorem covers (§5), refuses by name the two body
shapes it does not cover (§11), and leaves `WRITE-LEND-RELEND` open, deliberately
(§11): a sub-lend is a second borrow node nested in the first's body, which the
one-pass walker cannot see, and nothing about loops touches it.

The brief's file ownership: this card owns `std_buffer.rs`'s `lend_windows` /
`Repeat` arms and `std_string.rs`'s `last_view_uses`; it also touched, for the
loop alone, `std_buffer_c_emit.rs` and `std_string_c_emit.rs` (one published
block emitter and one line count), `source_std_buffer.rs` (one published loop
lowering entry point and the constants type it needs) and `source_std_string.rs`
(the composed `for` arm). None of these is a sibling card's file
(`ownership.rs` and `std_buffer.rs`'s **record** arms are
`AGGREGATE-OWNED-FIELD`'s and were not touched; `ownership.rs` did not move).

**Rebased onto `e80b58d6`** (`AGGREGATE-OWNED-FIELD` and
`CT07-TWO-THREAD-PROOF-LEG` merged ahead) at the integrator's request; the
overlaps and their resolutions, file by file, are in §12 item 9, and the
merged card's two by-name refusals are pinned against this card's windows in
§5.

## 2. What is true today (re-derived, not inherited)

Re-derived on `e878b362` before any edit.

* **Buffer route.** `lend_windows` (`std_buffer.rs:965`) computed `last_use`
  over top-level events only, matching `Store | Load | Extent | Release |
  Lend` and nothing nested; `through = last_use.get(binding).unwrap_or(index)`;
  `crossed_lend_windows` and the nested-acquire scan were over top-level
  indices; and `check_buffer_program` refused any top-level `Repeat` in an
  entry that lends **before** calling `lend_windows`. The lend-structured
  projection's `segment` had `Repeat` as an explicit no-op ("unreachable from
  the route"). All confirmed by reading; the brief's line numbers match
  `crates/qcore-core/src/`, not `compiler/src/`.
* **Composed string route — one premise of the measurement was wrong, and the
  hole was elsewhere.** `last_view_uses` (`std_string.rs:1057`) was indeed a
  top-level scan. But a `for` body on this route holds `BufferEvent`s only —
  it is lowered by `source.buffer.v1`'s reader — and **no buffer event names a
  view**, so "a view used only inside a `Repeat` body" is a program that
  cannot be written, and `last_view_uses` was exact as it stood. The real
  admitting arm was the `Repeat` arm of `check_string_program`: the `conflict`
  match judged a `Store` and a `Release` and gave a `Repeat` `None`, and
  `BufferEvaluation::apply` steps over a `Repeat` — so, had the by-name refusal
  been removed, every body store would have been admitted **unjudged and
  unrun**, which is worse than the measurement said (the body's writes would
  also have been dropped from the observation). The custody list
  (`string_custody_operations`) likewise ignored bodies, so a body store after
  a release would have reached no checker. Both are repaired (§3.2).
* **The refusals were correctly placed and are not reordered**: the by-name
  refusals this card keeps sit in exactly the positions the blanket ones did,
  before any window is computed.
* `SLICE-PERMISSIONS-LOOP-WINDOW` is **not** registered in `named_followups`
  (the wave-38 close deliberately left it out as a constraint-named rule; its
  sibling's entry says so). `STRING-CONCAT-LOOP-WINDOW` is registered, by
  `WAVE-38-CLOSE`, with the discharge "a window over iterations,
  `last_view_uses` walking `Repeat` bodies under a runs-many-times semantics …
  and amend 15h's sentence deliberately when it lands". `WRITE-LEND-RELEND` is
  registered by `WAVE-35-CLOSE`.

## 3. The window function over iterations

### 3.1 Buffer route (`crates/qcore-core/src/std_buffer.rs`)

```rust
pub fn last_uses<'a>(events: &'a [BufferEvent], bindings: &BTreeSet<&str>)
    -> BTreeMap<&'a str, usize>
```

For each loan binding, the index of the last **top-level** event that names it
**at any depth** (`custody_names_at_any_depth`: a `Repeat` contributes every
name its body names, however deep). A loan a loop body touches is live across
the whole loop; a lend at top-level index `e` gets the window
`[e, last_uses[binding]]` (or `[e, e]` when nothing names it — the old
`unwrap_or(index)`, unchanged). `lend_windows` is the same function it was,
reading `last_uses` in place of its inline top-level scan.

The consumers of the window were rewritten to the same reading:

* **The projection** (`lend_custody_operations::segment`) projects a `Repeat`
  by **unrolling its body under the loans live at the loop** — the same loans
  map, `count` times, the index bound in the lowering-visible tier so
  `part[i] = v` states the footprint `low + i` of the iteration it runs in;
  body bindings scoped out afterwards; nested loops by the same rule. The
  flat list the inherited IR decides is therefore `LOOP-V1`'s unrolled program
  under the loan (§4, `the_unrolling_is_loop_v1s`), and a body access is judged
  by exactly the rules a top-level one is (`the_projection_unrolls_the_body_inside_the_borrow_node`;
  the out-of-lend probe gets the flat spelling's codes, `["Q054", "Q053"]`).
* **`crossed_lend_windows`** is unchanged in code: windows are top-level
  intervals, and two loans last used inside one loop share a `through` and
  read as **nested**, whatever order their body uses come in — both are live
  across the loop, and the encoding is faithful for the iteration windows.
* **The nested-acquire scan** is depth-aware (`event.any(Acquire)`).

### 3.2 Composed string route (`crates/qcore-core/src/std_string.rs`)

* `last_view_uses` reads `view_uses_at_any_depth`, which recurses into `Repeat`
  bodies and, by the exhaustive `match`, returns nothing for a buffer event —
  the measured fact of §2 written as code, so a later body grammar that admits
  a view read lengthens the window here rather than closing it early.
* The `Repeat` arm of `check_string_program` judges the body **before it
  runs** (`loop_body_conflict`): the loop is one statement at index `k`; the
  loans live at `k` are those with `through ≥ k`, i.e. exactly the loans still
  read after the loop; the body is walked once per iteration with the index
  bound (nested loops likewise), and every store is judged against every live
  loan of its buffer by `ranges_overlap` over `[i, i + 1)` — an index outside
  the lowering-visible tier (a byte or a field read) being *not provably
  disjoint* and conflicting with every live loan, the fail-closed direction.
  Then the body runs through **`BufferRun::repeat`** — the buffer route's own
  runner, published for this purpose, which owns the index binding, the
  body-local scoping and the shadowing refusal — and its accesses, lines,
  flushes and renderings are published in program order. The loop's own
  refusals arrive as the new `StringError::Loop` at `Q097`–`Q099`, the
  `BufferError::Loop` precedent.
* `string_custody_operations` unrolls a body `count` times (accesses only), so
  a body store after a release is the inherited `UseAfterDrop`.

### 3.3 Lowering and emission

* `source_std_string.rs`: a `for` in a **composed** module (one that acquires a
  buffer) is lowered through the new `source_std_buffer::lower_for_statement`
  — the same reader and the same bound tier the uncomposed route uses — with
  the buffer reader's `Q051` body refusals re-attributed to `Q077` and the
  loop's `Q097`–`Q099` kept; every other nested block, and a `for` in a
  string module holding no buffer, keeps `Q068` word for word. The lowering
  keeps a `LoweringConstants` so a bound may name the entry's literal `let`s
  and `extent` bindings.
* `std_string_c_emit.rs`: an embedded `Repeat` is emitted by
  `std_buffer_c_emit::buffer_loop_block` — `emit_events`'s own text, a real C
  `for` whose body is written once — and the prelude scan and the line count
  are depth-aware. The buffer route's emitter is **unchanged**: `emit_events`
  already carried the loan environment into a body.

## 4. The Lean theorem (`proof/QCore/LoopWindows.lean`, audited)

A program is a list of `Stmt α` — a leaf event or `loop count body`.
`tagged prog` unrolls it exactly as `QCore.SourceLoop.unroll` does and tags every
unrolled event with the top-level index it is attributed to; `lastUse uses prog`
is the largest tagged origin of any event `uses` holds of — the Rust
`last_uses`.

| Theorem | Statement |
|---|---|
| `every_use_lies_inside_the_window` | every `(i, a) ∈ tagged prog` with `uses a` has `i ≤ lastUse uses prog` |
| `origins_are_monotone` | `tagged prog` is `Pairwise (·.1 ≤ ·.1)`: unrolling never moves an event in front of one that stood before it at the top level |
| `the_unrolled_window_lies_inside_the_iteration_window` | split `tagged prog = pre ++ opening :: (mid ++ use :: post)` with `uses use.2`: every `q ∈ mid` has `opening.1 ≤ q.1 ≤ lastUse` — everything the **exact** window contains in the unrolled program is attributed inside the iteration window |
| `tagged_forgets_to_the_unrolling`, `the_unrolling_is_loop_v1s` | `(tagged prog).map snd = unrollStmts prog = QCore.SourceLoop.unrollList (toItems prog)` on `StdBuffer.Event` — the unrolling the window is stated over is `LOOP-V1`'s, not a second one |
| `the_window_is_exact_over_a_flat_program` | on a loop-free program `taggedFrom i (events.map leaf) = flatTagged i events`: the iteration window **is** the flat window the routes always computed |
| `a_top_level_only_window_misses_a_body_use` | `STABILITY-REEVAL-3`'s scenario `[leaf 0, loop 2 [leaf 1]]`: the replaced computation gives `0`, the new one `1`, and `(1, 1)` is a use outside the old window — non-vacuous, by `decide` |
| `a_longer_live_set_refuses_no_less`, `a_longer_live_set_admits_no_more_lends`, `a_longer_live_set_blocks_the_drop_no_less` | `QCore.WriteLend`'s `ownerBlocked`, `admitsLend`, `hasLoan` are monotone in the live loans in the refusing direction — what makes an over-approximation of liveness safe |
| `the_admitted_body_shapes_are_exactly_these` | the body shapes admitted beside a loan, as one list (`access`, `plain`) |

So the iteration window **over-approximates the exact window in the refusing
direction**: every loan the exact window holds live at an unrolled position,
the iteration window holds live there too, and the IR refuses no less under
the longer live set. The audit prints `propext` and `Quot.sound` only.

## 5. Admitted and refused shapes per route

Pinned in `compiler/tests/loops_on_composed_routes.rs` (23 tests) and the
three re-pinned suites (`write_lend.rs`, `slice_permissions.rs`,
`string_concat.rs`, each test renamed to say what it now pins).

**Buffer route (entry that lends).**

| Shape | Verdict | Code / test |
|---|---|---|
| Loan used only inside the loop; owner writes a lent byte in the body (the REEVAL scenario) | **refused** | `Q054`, `a_loan_used_only_inside_the_loop_is_live_across_it_and_the_body_write_refuses` |
| Loop filling the loan's range through the loan, owner writing a kept byte in the same body | admitted; `through` = the loop's index | `a_loop_filling_a_lend_is_admitted_and_its_window_runs_through_the_loop` |
| Owner write to a lent byte before the loop / after the loop | refused `Q054` / admitted | `an_owner_write_before_the_loop_is_inside_the_window_and_one_after_it_is_outside` |
| Two loans last used inside one loop (disjoint / overlapping) | admitted, nested / refused `Q054` at the second lend | `two_loans_last_used_inside_one_loop_nest_and_their_disjointness_is_still_decided` |
| Nested loops through a loan; a step past the loan | admitted / refused with the flat spelling's codes `["Q054","Q053"]` | `a_nested_loop_is_projected_by_the_same_rule_at_every_depth` |
| Loop before any lend | admitted | `a_loop_before_the_lend_writes_the_owner_freely` |
| Body index read out of a byte, under a live loan | refused `Q054` (not provably disjoint) | `a_body_index_read_from_a_byte_conflicts_with_every_live_loan` |
| Lend inside a body | **refused by name** `Q051` `LOOP-PER-ITERATION-LOAN` | `a_lend_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name` |
| Acquire or release inside a body | **refused by name** `Q051` `LOOP-WINDOW-BODY-CUSTODY`; the same body in an entry with no lend keeps `LOOP-V1`'s admission | `a_body_that_acquires_or_releases_refuses_by_name_where_a_loan_is_held` |
| (after the rebase onto `e80b58d6`) a lend whose loan a `for` uses, beside a record with an owned field; an owned-field statement in a `for` body of an entry that lends; an owned-field statement in a `for` body beside a loop over the owner | **refused by name** `Q051` — `OWNED-FIELD-LEND-WINDOW` (the merged card checks its lend premise first), `OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY`; all three sit in that card's "half zero-and-a-quarter", before this card's lend block, so no iteration window is computed for them | `the_owned_field_refusals_survive_a_for_beside_a_loan` (24th test) |

**Composed string route.**

| Shape | Verdict | Code / test |
|---|---|---|
| Body write into a live loan's bytes (the REEVAL scenario) | **refused** | `Q078`, `a_body_write_into_a_borrowed_range_refuses_on_the_composed_route` |
| Loop before the borrow, filling the buffer the view reads | admitted | `a_loop_filling_a_buffer_before_it_is_borrowed_is_admitted` (the program `string_concat.rs` pinned refused) |
| Loop inside a loan's window writing outside its range | admitted | `a_loop_writing_outside_a_live_loans_range_is_admitted_on_the_composed_route` |
| Body index read out of a byte, under a live loan | refused `Q078` | `a_body_index_read_from_a_byte_conflicts_on_the_composed_route_too` |
| Body store after the release | refused `Q078` (`UseAfterDrop`, inherited) | `a_body_store_after_the_release_is_the_inherited_use_after_drop` |
| Zero bound / shadowing body binding / `return` in body | `Q098` / `Q097` / `Q097` — the loop pass's own | `the_loops_own_refusals_keep_their_codes_on_the_composed_route` |
| Scalar line in a body | admitted, once per iteration, in program order | `a_body_line_renders_once_per_iteration_on_the_composed_route` |
| `borrow_utf8` inside a body | **refused by name** `Q077` `LOOP-PER-ITERATION-LOAN` | `a_borrow_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name` |
| Release inside a body | **refused by name** `Q077` `LOOP-WINDOW-BODY-CUSTODY` (model level too) | `a_body_that_acquires_or_releases_refuses_by_name_where_a_loan_is_held`, `a_body_release_under_a_live_loan_is_refused_by_the_model_by_name` |
| Text line / literal / `len` inside a body | **refused by name** `Q077` `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY` | `a_string_construct_inside_a_body_on_the_composed_route_refuses_by_name` |
| `for` in a string module holding no buffer | still `Q068`, "opens a nested block" | `a_for_in_a_string_module_without_a_buffer_still_refuses_as_a_nested_block` |

## 6. Falsification (mutations)

`mutations.sh` (run from the repository root; log `mutations.log`) applies six
source mutations, one at a time, runs the named test, expects it red, restores
the file and checks it sha256-equal. Results in `mutations.log`, summarised:

Run at HEAD `4cc580bb` (2026-09-18T12:13:29Z, the tree rebased onto
`e80b58d6`), exit 0 — **baseline green before and after, six of six red, three
files restored sha256-equal** (`std_buffer.rs` `87eda8f8a503c7d4`,
`std_string.rs` `4158d84e4ddb544e`, `source_std_string.rs`
`49e77c407429327c`, before and after). The pre-rebase run (HEAD `7aff9fcf`,
08:12Z) gave the same six verdicts; a first post-rebase run reported M1
**VACUOUS** because the rebase had put a comment between M1's anchor lines
and the mutation was never applied — the harness's exactly-once anchor check
doing its job — and M1 was re-anchored on the `Repeat` arm alone.

**A finding about the harness itself, recorded rather than smoothed.** The
first run (HEAD `ac2a6b84`, 07:40Z) reported the same six red rows but left
**stale build artifacts**: it restored each file with `mv` from a backup whose
mtime predated the mutated build, so cargo's fingerprint read the restored file
as unchanged and kept the rlib compiled from the *last* mutation of each crate
(`qcore-core` at M5, `qcore-frontend` at M6). The first full gate then failed
exactly three rows of this card's own test file — no custody unrolling, and
the composed lowering running past its by-name refusal — against those
artifacts (`gate/run1/`, kept as evidence; the source tree was byte-identical
to the committed one throughout). The harness now `touch`es every restored
file and runs the whole test file **green before the first mutation and after
the last restore**, so a stale artifact or a red baseline is reported as a
failure of the harness and cannot pass as non-vacuity; the run recorded here is
the corrected one. The verdicts of the first run were not wrong — each row
did go red under its own mutation — but a harness that cannot tell that from
a red baseline had not shown it, and earlier waves' harnesses restore the same
way (§12 item 8).

| Mutation | Arm mutated | Named test | Result |
|---|---|---|---|
| M1 `top-level-only-window` | `custody_names_at_any_depth`'s `Repeat` arm contributes nothing (the replaced computation) | `a_loan_used_only_inside_the_loop_is_live_across_it_and_the_body_write_refuses` | RED — the REEVAL scenario is **admitted** |
| M2 `composed-body-not-judged` | `loop_body_conflict` judged against an empty live set | `a_body_write_into_a_borrowed_range_refuses_on_the_composed_route` | RED — the body write into the loan's bytes is **admitted** |
| M3 `per-iteration-lend-admitted` | the `LOOP-PER-ITERATION-LOAN` refusal removed | `a_lend_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name` | RED — the per-iteration lend's stores reach the checker as no operation and the program is **admitted** |
| M4 `body-custody-unnamed` | the `LOOP-WINDOW-BODY-CUSTODY` refusal removed (buffer route) | `a_body_that_acquires_or_releases_refuses_by_name_where_a_loan_is_held` | RED — still refused, by the depth-aware nested-acquire scan under its own name; the **name** is load-bearing |
| M5 `composed-custody-not-unrolled` | `string_custody_operations` ignores bodies again | `a_body_store_after_the_release_is_the_inherited_use_after_drop` | RED — the body store after the release is **admitted** |
| M6 `per-iteration-borrow-unnamed` | the composed lowering's by-name refusal skipped | `a_borrow_inside_a_body_is_a_per_iteration_loan_and_refuses_by_name` | RED — still refused, by the buffer reader's own message; the **name** is load-bearing |

The Lean side's non-vacuity is `a_top_level_only_window_misses_a_body_use`,
which is M1 as a theorem.

M1 is the brief's requirement: the REEVAL scenario admitted under the window
computation put back to top-level only. M2 is its composed-route twin. M3 and
M5 show two *admitting* holes the by-name refusal and the custody unrolling
close. M4 and M6 show only that the residual **name** is load-bearing — under
those mutations the program is still refused, by a different arm and message —
and are recorded as such rather than claimed as more.

## 7. Emission census

`emission_census.py --base measure/census/quirin-base --card target/debug/quirin`
over every `.qcore` in the repository, re-run on the tree rebased onto
`e80b58d6` (529 programs; the base binary built from `e878b362`'s code — the
base of this card, which predates both `AGGREGATE-OWNED-FIELD` and
`CT07-TWO-THREAD-PROOF-LEG` — exit 0):

| Verdict | Programs |
|---|---|
| admitted by both, C **byte-identical** | **139** |
| admitted by both, C differing | **0** |
| admitted by base only | **0** |
| admitted by card only | **3** — `examples/loops/lend_fill.qcore`, `examples/strings/looped_fill.qcore` (this card's own) and `examples/records/owned_field.qcore` (`AGGREGATE-OWNED-FIELD`'s, new on main since the base) |
| refused by both | 387 |

Unrolling is unchanged and every program admitted before the card emits
byte-identical C. The pre-rebase run (527 programs, 139 / 0 / 0 / 2 / 386)
gave the same verdict on every program it shared. Full rows in
`emission-census.json`.

## 8. Examples and sanitized native runs

One new program per route, each indexed on its family page with its transcript
read out of the page by `examples_suite.rs` (42 tests green) and its sanitized
native run (`-std=c11 -Wall -Wextra -Werror -pedantic -fsanitize=address,undefined`,
LSan on Linux) in `loops_on_composed_routes.rs`:

* `examples/loops/lend_fill.qcore` — a loop filling a lend's range through the
  loan while the owner writes byte 7; stdout `81 67 111 114 1`; the C is one
  `for` with one store through `l_part` (page block `emitted: lend-loop`).
* `examples/strings/looped_fill.qcore` — a loop before the borrow and a loop
  inside a live loan's window writing outside its range; stdout `aaaa`, `aa`,
  `aabb`.

Linux arm only. The `windows_c` path is carried unchanged in the test file and
no Windows or Apple run is claimed — the `WRITE-LEND-WINDOWS-RUN` footing
(`CLAUDE.md`, platform records), which that platform record already covers for
this family; no new platform residual is coined.

## 9. Diagnostics (§L21 narrowings) and the keyword census

No `Q` code is allocated. Three §L21 rows carry an amendment each, stated as
**narrowings** with the exact admitted shape:

* `Q051` — a `for` in an entry that lends is admitted (the shape in §5); still
  covers, by name, `LOOP-PER-ITERATION-LOAN` and `LOOP-WINDOW-BODY-CUSTODY`.
* `Q068` — narrows by exactly one statement: a `for` in a composed module.
* `Q077` — the `for` refusal lifted; still covers, by name, the three residual
  shapes; `Q078` **not widened** (reached from a body statement, reporting the
  sentence a top-level one reports); `Q097`–`Q099` reached from the composed
  route unwidened.

**Constraint 14 census, run: +0 words, +0 tokens.** The card admits an existing
statement in two positions and spells nothing new;
`git diff --stat e878b362 -- crates/qcore-frontend/src/lexer.rs crates/qcore-frontend/src/parser.rs`
is empty; the composed route reads the loop through the buffer route's reader.

## 10. Gate

Three detached full gates (`measure/run-gate.sh`: `setsid -f`, no
`trap '' TERM`, every step's exit to its own file, no pipe; exits copied to
`gate/` beside this file, `gate.json` built from them by a script that reads
only those files). **The figures are run 3's**, on the tree rebased onto
`e80b58d6`; runs 1 and 2 (pre-rebase) are kept below because each records a
fact.

**Run 3 — the figures (`gate/`, HEAD `4cc580bb`, this card's final code on
`e80b58d6`; later commits are documentation, `gate.json` and the
regeneration only).** Started 12:14:08Z; `git submodule update --init
--recursive` exit 0; one uninterrupted `cargo test --workspace
--no-fail-fast` of 16m34s, exit 0: **4,701 passed / 0 failed / 9 ignored**
over 237 result summaries (226 `Running` targets + 9 doc-test sections; the
loose `^test result` grep gives 239, the two-line discrepancy the closes
record); fmt 0; clippy 0; both probes 0; `scripts/check_lean_audits.py`
exit 0, **140 Lean audits** (main's 139 at `e80b58d6` + `AuditLoopWindows.lean`,
measured on disk after the rebase); Python (`unittest discover -s
scripts/tests`) **exit 1 in the detached run, deliberately reported**: 303
run, 2 failures, both tripwires against the not-yet-regenerated matrix
(`D02-DIMENSIONS-FOLLOWUP-COUNT`'s `232 != 234`, and the Lean-count check's
`139 != 140`) — the wave-36/37/38 closes' shape — and **303 OK, exit 0**
re-run after the regeneration with `--gate` (`gate/python-post.exit`).
`clang --version` exit 0, `Ubuntu clang version 21.1.8 (6ubuntu1)`,
`QCORE_CLANG` unset (`gate/clang-version.txt`). Gate ended 12:31:17Z; disk
161 GB free before, 256 GB after.

Constraint 16: all five named rows that live in this workspace were **green**
under full-suite load in run 3 — `a_declared_process_limit_stops_the_fixture_from_forking`,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`memory_growth_beyond_the_declared_limit_is_bounded`,
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
and the unnamed `a_forced_override_never_weakens_an_execution`, plus
`an_admitted_path_still_works_under_confinement` (red in run 1, below); a
full-suite run on a machine carrying sibling builds, not a quiet-host number.
`real_packages_preserve_unknown_and_reject_mutations` lives in `providers/`
and is not among them. Provider **122 carried, not re-measured**: nothing under
`providers/` or `crates/qcore-schemas/` moved and `qcore-schemas` is still
14.0.0 (constraint 18 not engaged); a Unix figure.

**Run 2 (pre-rebase, HEAD `7aff9fcf`, base `e878b362`) — exit 0**: 4,662 / 0
/ 9 over 235 summaries, 138 Lean, 303 Python after regeneration; superseded
by run 3 and not carried into `gate.json`.

**Run 1 (`gate/run1/`, HEAD `ac2a6b84`) — exit 101, recorded, not
smoothed.** 4,658 passed / **4 failed** / 9 ignored: three rows of this card's
own test file red against the stale artifacts the first mutation harness left
(§6 — the source tree was byte-identical to the committed one, the rlibs were
not), and one constraint-16 row, `an_admitted_path_still_works_under_confinement`
(the confined-fixture write test, `Io(Broken pipe)` under load), red under
the load of the sibling gates and **green in runs 2 and 3** — both facts
recorded. Every other step of run 1 was exit 0.

Delta: this card's own file contributes **+24** workspace tests (23 plus the
owned-field pair pinned after the rebase; the three re-pinned tests in
`write_lend.rs`, `slice_permissions.rs` and `string_concat.rs` are renamed,
not added) and **+1** Lean audit, Python +0, ignored unchanged; the rest of
the distance from the wave-38 close's 4,639 / 137 is the two cards merged
ahead of this one and is theirs to attribute. Nothing is promoted by this
card: no corpus count, unit status or function status moves; contract corpus
88; CT and proof counts as main carries them; full language conformance
false.

## 11. Residuals and retirements

**Retired (exact name, verified registered first):**

* `STRING-CONCAT-LOOP-WINDOW` — registered by `WAVE-38-CLOSE`; discharge met
  word for word: the window computation revisited for a loop construct
  (`view_uses_at_any_depth`, and the body judged over iterations), the
  refusal kept in front until it landed, and 15h's sentence amended
  deliberately (§12).

**Not retired, with the reason:**

* `SLICE-PERMISSIONS-LOOP-WINDOW` — **no entry exists** to retire (§2); the
  lifting is recorded in the amended constraint 15h and here.
* `WRITE-LEND-RELEND` — left open, deliberately: orthogonal to loops (§1).

**Registered (`recorded_by: LOOPS-ON-COMPOSED-ROUTES`, each with a discharge):**

* `LOOP-PER-ITERATION-LOAN` — a lend or a borrow opened inside a body.
* `LOOP-WINDOW-BODY-CUSTODY` — a body that acquires or releases where a loan
  is held (lending buffer route; composed route).
* `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY` — a string construct, text line,
  `len`-bound header or unrolling licence inside a composed `for` body.

`named_followups`: 232 (main at `e80b58d6`) → **234** (+3, −1; 228 → 230 on
the pre-rebase base).

## 12. For the integrator

1. **Headline numbers moved in this tree** (after the rebase onto
   `e80b58d6`): Lean audits **139 → 140** (`AuditLoopWindows.lean`);
   `named_followups` **232 → 234**; workspace tests **4,701 / 0 / 9** per
   `gate.json` (see §10). `AGENTS.md`'s gate row and followups line are
   edited accordingly.
2. **Constraint 15h amended deliberately, in the constraint's own text**: the
   sentence "Do not widen the window's computation without revisiting it: the
   interval is exact because the route is one flat statement sequence, and a
   loop construct invalidates that premise — which is why the composed route
   **refuses** a `for` today (`STRING-CONCAT-LOOP-WINDOW`)" and the clause "so
   the window-exactness argument and the `for` refusal at `Q068` are untouched
   there (`STRING-CONCAT-LOOP-WINDOW` re-measured, not assumed)" plus the
   listed refusal "a `for` in an entry that lends
   (`SLICE-PERMISSIONS-LOOP-WINDOW`)" are quoted as superseded in place, and
   the amendment paragraph at the end of 15h carries the new rule. Landed in
   my tree; quoted in full in §12.1 below.
3. **Constraint 14 ledger**: `LOOPS-ON-COMPOSED-ROUTES` +0 (census run), landed.
4. **Q-space note**: the card recorded as the sixteenth instance, the first
   whose movements are narrowings; landed.
5. **§L21**: three row amendments (`Q051`, `Q068`, `Q077`–`Q079`), landed.
6. **Matrix**: regenerated with `--gate docs/reports/loops-on-composed-routes/gate.json`.
7. **Decisions I could not take**: (a) whether the composed route should
   accept an unrolling licence before a `for` (recorded under
   `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`); (b) whether
   `SLICE-PERMISSIONS-LOOP-WINDOW`'s absence from the tracker should now be
   recorded as a retired *rule* somewhere other than 15h — I left it in 15h's
   amendment only.
8. **A harness hazard for the integrator to decide on** (§6): a mutation
   harness that restores a source file by `mv` from a `cp` backup leaves the
   file with an mtime older than the mutated build, and cargo then keeps the
   rlib compiled from the mutation. This card's first run did exactly that and
   the first full gate measured stale artifacts (`gate/run1/`, three red rows
   of this card's own file). Shipped harnesses of earlier waves that restore
   the same way (the `W36-W37-MUTATION-HARNESSES-UNSHIPPED` family, and any
   `mutations.sh` using `cp`/`mv`) would leave the same artifacts behind
   whenever their last mutation is not followed by a `cargo clean` or a
   touch; the fix is one `touch` after each restore plus a green baseline
   before and after (this card's `mutations.sh`). I did not audit or edit
   other cards' harnesses; whether to register this as a residual
   (`MUTATION-HARNESS-STALE-ARTIFACTS`) is the integrator's call — it is
   named here, not registered, because it is a finding about other cards'
   files.
9. **Rebase onto `e80b58d6`, overlaps resolved so both cards' arms survive**
   (one line per file):
   * `crates/qcore-core/src/std_buffer.rs` — its record arms and
     `FieldStore`/`FieldLoad`/`FieldExtent`/`FieldRelease` kept everywhere;
     my `Repeat` arm in the lend projection kept beside its catch-all
     comment; `BufferRun` carries its `moves` field; my two exhaustive
     matches (`custody_names_at_any_depth`, the projection's body walk)
     treat the four field-path events as naming no loan (their statements
     refuse by name before either runs).
   * `crates/qcore-core/src/std_string.rs` — its exhaustive-match arms kept
     in `buffer_view_uses` and `loop_body_conflict` (a field-path load binds
     a name outside the tier; the rest no-ops), my `Vec`-returning form kept.
   * `crates/qcore-core/src/std_buffer_c_emit.rs` — my `buffer_loop_block`
     initialises its two new `RecordEnvironment` fields.
   * `compiler/tests/examples_suite.rs` — both cards' example constants kept.
   * `docs/QCORE_SPEC_r0_3.md` — the `Q051` row is its amended row with my
     amendment appended after it.
   * `docs/reviews/r0_3/acceptance_matrix.json` — main's (232) taken, my
     `register_followups.py` re-run: **234** (+3, −1), then regenerated with
     `--gate`.
   * `AGENTS.md` — its 15k amendment and Q-space instance kept beside my 15h
     amendment; the followups line reads 234 with both chains; my Q-space
     instance renumbered **seventeenth** (its `AGGREGATE-OWNED-FIELD` is the
     sixteenth and took the first single narrowing, `Q055`; mine is the first
     whose movements are all narrowings); the gate row rewritten from the
     post-rebase gate.
   * `docs/development/increments/wave-39.md` — union, this card's paragraph
     first, then the two already on main.
   * No admitted/refused row of §5 changed; one row was **added** (the
     owned-field pair). `lean_audits` re-measured on disk after the rebase:
     **140** (main's 139 + `AuditLoopWindows.lean`).

### 12.1 The 15h amendment (verbatim, as landed)

> **Amended, card `LOOPS-ON-COMPOSED-ROUTES` (wave 39; deliberate — it
> replaces the "one flat statement sequence" premise this constraint carried
> from `STRING-CONCAT`, the two sentences quoted as superseded above, by a
> theorem).** A loan's window is computed **over iterations**: on both
> routes it runs from the loan's opening to the last **top-level** statement
> naming the loan **at any depth** (`std_buffer::last_uses`;
> `std_string::view_uses_at_any_depth`), so a `for` whose body names a loan
> is that statement and the loan is **live across every iteration** of it.
> The computation is proved sound relative to `source.loop.v1`'s own
> unrolling in `QCore.LoopWindows`: every use in the unrolled program lies
> inside the window (`every_use_lies_inside_the_window`), unrolled positions
> are attributed to non-decreasing top-level indices, so everything the exact
> unrolled window contains the iteration window contains
> (`the_unrolled_window_lies_inside_the_iteration_window`), the unrolling is
> `QCore.SourceLoop.unroll`'s (`the_unrolling_is_loop_v1s`), a loop-free
> program's window is the flat one the routes always computed
> (`the_window_is_exact_over_a_flat_program`), and the replaced top-level-only
> computation demonstrably missed a body use
> (`a_top_level_only_window_misses_a_body_use`). The over-approximation is in
> the **refusing** direction because the IR's owner-access, lend-admission
> and drop decisions are monotone in the live loans
> (`a_longer_live_set_refuses_no_less` and siblings) — do not replace the
> window by an exact per-position one without re-proving both halves. A
> `for` is therefore **admitted** beside a lend (`SLICE-PERMISSIONS-LOOP-WINDOW`
> lifted, `Q051` narrowed) and inside a composed string/buffer entry
> (`STRING-CONCAT-LOOP-WINDOW` discharged, `Q068` and `Q077` narrowed), with
> the body decided **under the loans live at the loop**: on the buffer route
> the lend-structured projection unrolls the body inside the borrow node
> with the index bound, so the IR decides every body access as it decides a
> top-level one; on the composed route every body store, at every
> iteration, is judged against every loan still read after the loop
> **before the body runs** (`loop_body_conflict`), the body then running
> through the buffer route's own `BufferRun::repeat` — the one iteration
> rule, published for this — and the composed custody list unrolling it so a
> body store after a release is the inherited `UseAfterDrop`. An index
> outside the lowering-visible tier (a byte or a field read) is **not
> provably disjoint** and conflicts with every live loan of its buffer, on
> both routes. Three premises must survive refactors, each a refusal **by
> name** that sits **before** the window is computed and must not be
> reordered behind it: a lend or a `borrow_utf8` inside a body is a
> per-iteration loan (`LOOP-PER-ITERATION-LOAN`, `Q051`/`Q077`); a body that
> acquires or releases where a loan is held (`LOOP-WINDOW-BODY-CUSTODY`,
> `Q051` in an entry that lends, `Q077` on the composed route); and a string
> construct, text line, `len`-bound header or unrolling licence inside a
> composed `for` body (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`, `Q077`).
> The release arm stays whole-buffer, the crossed-window and (now
> depth-aware) nested-acquisition refusals stand, `WRITE-LEND-RELEND` is
> untouched, an entry with no loan keeps `source.loop.v1`'s admission byte
> for byte, and every program admitted before the card emits byte-identical
> C (a 527-program census, 139 admitted, 0 moved).

The two superseded sentences are quoted in place in 15h with the words
"Superseded deliberately by card `LOOPS-ON-COMPOSED-ROUTES`".
