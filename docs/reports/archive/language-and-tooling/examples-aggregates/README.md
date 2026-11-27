# EXAMPLES-AGGREGATES — the published records family (wave 28)

Card brief: *"Publish an examples/records/ directory: a README plus one or more
.qcore programs demonstrating source.record.v1 (type declarations,
construction, field access). Precedent: the LOOP-EXAMPLE card."* The card is
`AGGREGATES-V1`'s named follow-on `EXAMPLES-AGGREGATES`
(`docs/reports/aggregates-v1/README.md` §12): that card shipped a test fixture
with the construct's evidence but published no example, and the two are not the
same deliverable.

Base: `baadb564` (wave-27 integration tip), with `origin/main` at the wave-27
close (`e552e59c`) merged in before the closing gate on the integrator's
direction — the card is therefore post-close, wave 28.

**What is published.** `examples/records/` — two programs and a page, held to
`EXAMPLES-SUITE`'s standard by `compiler/tests/examples_suite.rs`, which
reaches the directory structurally by its scan and owns the family's sanitized
native runs. The published family count grows **11 → 12** (the index table in
`examples/README.md` gains one row, and its native-runner table one entry).
**No crate-source file changed**: the whole diff is the two programs, the
page, the index row, four suite-owned tests in `examples_suite.rs`, this
report and one `DEVELOPMENT.md` paragraph. **No word reserved, no diagnostic
code allocated, no pinned expectation edited.**

---

## 1. The two programs, and why these two

| Program | What it shows |
|---|---|
| `point.qcore` | CT09's "distinct ordinary struct fields" scenario as a published example: three fields, three *different* values, three reads printed, then a second value of the same type read in a different order. Record-only — no buffer anywhere — so the effect row is `io` alone and the emitted C carries **no buffer prelude** |
| `pixel_writes.qcore` | A record beside a real resource: `Rgb` fields drive three stored bytes, a `Write` record drives the fourth store's **index and value** both out of field reads, the buffer owes its `drop`, and the four bytes read back are `QCor` — the loops family's own payload |

The pairing is deliberate: the two programs are **each other's leak control**,
and the page says so in those words. `point.qcore` allocates nothing at all,
so its sanitized run is a *negative* control — LSan reporting a leak would
mean the aggregate had acquired something the model says it cannot (standing
constraint 15k's field-custody measurement, visible in a published program).
`pixel_writes.qcore` allocates one real buffer and releases it, so its run is
a *positive* control — a lost release is a finding, and the two records beside
the buffer change nothing about that (`Q054` still refuses the drop-less
variant, checked by a suite test rather than asserted).

`point.qcore` is also the first *published* program to exercise the
conditional buffer prelude `AGGREGATES-V1` §4.3 landed: its emitted C contains
no `qcore_bytes_new` at all, and a suite test pins that beside the
neighbouring program's positive case so the contrast is a fact about the
programs rather than about the backend.

### 1.1 The spelling rule, program by program

`examples/README.md`'s rule is *pipe the flow, parenthesize the assembly*, and
both programs are spelled to it:

* **Piped.** Every once-read scalar flows straight into one sink and is piped:
  `a |> print dec;` (and four siblings) in `point.qcore`; `first |> print
  dec;` (and three siblings) in `pixel_writes.qcore`.
* **Statements.** A construction (`let p = Point { … };`) is the assembly half
  — a value built from several pieces — and stays a statement; the construct
  is a `let` form and has no pipeline spelling, which is the honest shape. A
  buffer store (`pixels[i] = v;`) has two operands, an index and a value, so
  it too is assembly and stays a statement; the program's comment says so.
* **Fan-out.** `p`, `q`, `c` and `w` are each read by several field reads and
  stay on the statement side; a field read is itself a `let` statement by
  grammar, so nothing needed un-piping.

The pipe spellings were **measured, not assumed**, to change nothing: the
piped programs emit byte-identical C to statement-form drafts (the desugaring
is a token permutation), verified during authoring with `quirin emit` and
`cmp`.

### 1.2 The selector words, avoided and said

Residual `AGGREGATE-SELECTOR-WORDS`: a record type or field named one of the
six whole-token-stream route-selector words (`spawn`, `join`,
`qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`,
`qcore_error_payload`) selects the concurrency route from its own spelling and
refuses fail-closed — `Q015` declared, `Q000` constructed. Every published
type name (`Point`, `Rgb`, `Write`) and field name (`x`, `y`, `weight`, `red`,
`green`, `blue`, `at`, `value`) avoids all six, and the page states the
boundary with the residual's name so a reader extending the family knows it is
a collision, not a style choice.

---

## 2. The page, held to the suite standard

Everything `examples/records/README.md` claims is read out of the page by a
test rather than restated:

* **Transcripts.** Three marked blocks (`transcript: point stdout`,
  `transcript: point stderr`, `transcript: pixel_writes stdout`), compared
  with the reference execution's projections **and** with the native runs'
  streams. No order is claimed between stdout and stderr, per the family
  convention.
* **The emitted struct.** An `emitted: point-struct` block publishes the
  `struct qcore_record_Point` definition; a test asserts the backend really
  writes it, that constructions initialize in *declared* order however the
  source wrote them (`{ weight: 9, y: 2, x: 1 }` → `.f_x, .f_y, .f_weight`),
  and the conditional-prelude contrast of §1.
* **The refusal.** A `refusal: Q051` block (an incomplete construction) is
  compiled and its `diagnostic: Q051` block compared with the compiler's real
  diagnostic, span caret included — and the same program with the field
  supplied is compiled to admission, so what is pinned is the totality rule
  rather than the shape.
* **The side table.** Four more refusals, each earned by exactly the statement
  the table writes and asserted as the *only* diagnostic: `drop` on a record
  (`Q052`, with the effect row admitting `alloc` so nothing else fires), a
  field the type does not declare (`Q052`), a construction of an undeclared
  type (`Q013`, name resolution's), and the published program without its
  release (`Q054`, the unedited ownership checker's).

**No `quirin` command line is published, deliberately.**
`crates/qcorec/tests/examples_cli.rs` pins the published `quirin check`/`emit`
command-line counts at exactly 6/6 across every family page; the records page
follows the loops precedent and publishes `cargo test` lines only. The CLI
suite passes unchanged.

## 3. Test support

Four tests were added to `compiler/tests/examples_suite.rs` (38 → 42 in that
file), all in the file's own idiom, plus rows in four existing scan/list
tests:

| Test | What it owns |
|---|---|
| `the_record_examples_run_clean_and_match_their_reference_execution` | both native runs under ASan/UBSan (LSan on Linux), both streams against the reference projection and the page's transcripts |
| `the_records_page_publishes_the_struct_the_backend_really_writes` | the emitted-struct block, declared-order initialization, the no-prelude/with-prelude contrast |
| `the_documented_incomplete_construction_is_the_refusal_the_compiler_gives` | the `Q051` block and its quoted diagnostic, caret included, with the completed construction as the non-vacuity control |
| `the_record_refusals_the_page_names_beside_it_hold` | the four side-table refusals, each a single-code assertion, and every code the page names being one the page spells |

Existing tests extended by rows only: the route assertion (both programs
`SourceExecutionProfile::StandardBufferV1` — the only route v1 admits the
construct on, including for the buffer-less `point.qcore`), the
every-program-emits list, the transcript-projection walk and the
changed-line tripwire. The directory-scan tests
(`every_published_program_is_indexed_and_every_indexed_program_exists`,
`the_examples_index_names_every_family_and_only_real_ones`) needed no edit at
all — the new family is covered structurally the moment the directory exists,
which is that card's design working as intended.

## 4. Deviations and findings

1. **The ground moved mid-card and the merge is recorded rather than hidden.**
   Wave 27 closed on `main` (`dda43a3e`, then `e552e59c`) while this card was
   in flight; `origin/main` was merged into the card branch before the closing
   gate on the integrator's direction. The Lean audit baseline at the close is
   **114**, and this card expects exactly 114 since it changes no semantics.
2. **The refusal block was shaped by the cascade.** A refused construction
   leaves its binding unbound, so a program that then reads the record earns a
   second `Q052` (`AGGREGATE-REFUSAL-CASCADE`, recorded by the construct's
   card). The published refusal program deliberately never reads `p`, so the
   page's diagnostic block is one diagnostic and the test can compare it
   exactly — the same discipline `AGGREGATES-V1`'s own refusal suite used.
3. **`drop` on a record needed its effect row widened to isolate the rule.**
   With `effects { io }`, `drop p;` earns `Q052` *and* a `Q057`
   (`Alloc` undeclared) beside it; with `effects { io, alloc, mut }` the
   refusal is exactly `["Q052"]`. The side-table test uses the latter so it
   measures the custody rule it names, and nothing else.
4. **No AGENTS.md edit.** The entry-points section's published-examples bullet
   now undercounts by one family (records is the twelfth) and its family list
   does not name `records/`; per the sync rule the headline update is
   integrator surface, flagged in §6 rather than edited here.

## 5. Gate

Run at the merged tip (this card's commits plus `origin/main` at `e552e59c`),
exit codes written by the processes themselves into worktree-private
`measure/` files.

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run first (checked out `d82464a277d0f42703702c4dfd9af6af38595a83`) |
| `cargo test --workspace --no-fail-fast` | **4,380 passed, 0 failed, 8 ignored** across 214 suites reporting `test result:`, exit **0** (read from `measure/workspace.exitcode`, written by the process itself) |
| `cargo test -p qcore-compiler --test examples_suite` | **42 pass** (38 + this card's 4), both native runs included |
| `cargo test -p qcorec --test examples_cli` | **6 pass**, unchanged — the 6/6 command-line pin holds |
| `cargo fmt --all -- --check` | clean, exit **0** |
| `cargo clippy --all-targets -- -D warnings` | clean, exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **"Passed 114 Lean audits"** (the text line, per that script's own behaviour), exit 0 — the wave-27 close's baseline, unmoved, as a card changing no semantics must leave it |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, OK** — no derived-value tripwire, since this card adds no audit file and edits no derived fact |

The workspace arithmetic reconciles: the wave-27 close's ledger records 4,376
workspace tests (8 ignored); 4,376 + 4 = **4,380**, and 4 is precisely this
card's new tests — the card adds its own and moves no other count. None of
standing constraint 16's five load-sensitive rows tripped; no assertion was
weakened and no test re-run in isolation to obtain a pass.

## 6. For the integrator

* `examples/README.md`'s family table and native-runner table each gain one
  row; the published family count is now **twelve**. `AGENTS.md`'s
  entry-points bullet still says eleven and lists the families by name —
  integrator surface, not edited by this card.
* `compiler/tests/examples_suite.rs` gains 4 tests; the workspace total moves
  4,376 → 4,380 by exactly those.
* No crate under `crates/`, no `compiler/src/` file, no contract, no provider,
  no proof file and no pinned table changed. The corpus pins (73 modules, 185
  units, leak 266), the CT manifest and the Lean audit count are all the
  close's, unmoved.
* Residuals this card leaves: none of its own. The construct's residuals
  (`AGGREGATE-*` family, `docs/reports/aggregates-v1/README.md` §12) stand
  unchanged; the page points at four of them by name where a reader would
  otherwise mistake a boundary for a bug (`AGGREGATE-FIELD-WRITE`,
  `AGGREGATE-BOUND-FROM-FIELD`, `AGGREGATE-ROUTE-WIDENING`,
  `AGGREGATE-SELECTOR-WORDS`).
