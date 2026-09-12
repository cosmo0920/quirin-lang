# EXAMPLES-SUITE-WIRE — every published example family under one standard (wave 26)

Card brief: a small test-wiring card closing three open follow-ons —
`STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE`.
The newer published example families are checked by their own card test files
but are not held to the examples-suite standard, and `source.loop.v1` ships a
test fixture with byte-compared sanitized evidence but publishes no example.

Base commit: main `e26bb0c3` (the wave-25 close).

**What landed.** One new published family — `examples/loops/`, one program and
one page — and **12 new tests**: 11 in `compiler/tests/examples_suite.rs` and 1
in `crates/qcorec/tests/examples_cli.rs`. Every published family is now under
the suite's page-level standard, by a **directory scan** rather than by an
explicit list. Five README repairs, each listed in §5. **No semantic surface
moved**: no crate outside `tests/` is touched, no diagnostic, keyword, route,
contract module or corpus number changes, and every pre-existing example
program is byte-identical. The no-Lean close is legitimate on exactly that
ground — there is no semantic change for a Lean definition to match (§7).

---

## 1. What the suite adds that a card test file does not

Each of the three named follow-ons said the same thing in its own words: the
family is verified by its own card's test file rather than by
`compiler/tests/examples_suite.rs`, "whose list is explicit rather than a
directory scan", and folding it in is mechanical.

It is worth being precise about what was and was not missing, because the card
tests are not weak. Before this card, `string_slice_io.rs`, `string_concat.rs`,
`std_socket.rs`, `std_http.rs`, `std_event.rs`, `compose_event_lines.rs` and
`socket_event_compose.rs` already compiled each program through the real
pipeline, built the emitted C under `-Werror`-class flags, ran it under
ASan/UBSan/LSan and compared both streams with the reference execution. Four of
the seven already read a README transcript out of the file.

What the suite adds is three things none of them can:

1. **A directory scan.** Every `.qcore` program in a family directory is indexed
   on that family's page, and every `.qcore` a page links exists. An explicit
   list cannot notice a file nobody added it to — which is exactly how
   `examples/sockets/` came to be published with no row in `examples/README.md`
   at all until the HTTP card noticed and added two (recorded in
   `docs/reports/http-quirin-v1/README.md` §"integrator reconciliation").
2. **One marker convention, applied to every page.** Wave 21 established
   `<!-- transcript: NAME stream -->`; the sockets and HTTP pages adopted it;
   the strings and events pages did not, and their card tests reach their
   transcripts by splitting on a heading (`"## Transcript"` → `nth(1)` /
   `nth(2)`) or by `README.contains(…)` containment. This card added the markers
   to the strings and events pages and extracts every transcript the same way.
   The card tests' own heading splits and containment assertions are left
   working, unedited, and still run.
3. **A route assertion per program**, so a page that names its program's route
   is checked rather than believed — including the two contrasts the pages draw
   on purpose: `http/raw_request.qcore` is on `source.socket.v1`, not the HTTP
   route, and `events/socket_server.qcore` likewise.

### 1.1 The native runs are not duplicated, and that is the wave-21 precedent

The brief asked which it would be. It is **not** double-running, and the
precedent is wave 21's own treatment of `examples/threads`: that family was
owned by another test file, and the suite "was not read from, written to, or
referenced except by cross-link". The same boundary is applied here, one level
finer:

> Where a card's test file already builds and runs a program, this suite
> compares the **page** with the **model** and stops there; the card's file
> compares the model with the machine.

Together they close the triangle — page = model = native run — and each emitted
program is built exactly once per gate. The suite's transcript comparisons in
§6 call `projections()` on the compiled module's reference observation; they
invoke no C compiler.

`examples/loops/` is the exception, because it is new here and no other file
owns it: this suite performs its sanitized native run, exactly as it does for
the five wave-21 families. `examples/threads` remains `thread_examples.rs`'s in
every respect.

---

## 2. The wiring table, family by family

Every row's "suite adds" column is new in this card. "Already covered" names the
file that owned the family before it.

| Family | Programs | Already covered by | What the suite adds | Native run owner |
|---|---|---|---|---|
| `stdio/` | 4 | `examples_suite.rs` (wave 21) | directory scan; index row | `examples_suite.rs` |
| `pipes/` | 3 | `examples_suite.rs` (wave 21) | directory scan; index row; published `quirin` command lines executed | `examples_suite.rs` |
| `branching/` | 3 | `examples_suite.rs` (wave 21) | directory scan; index row; published `quirin` command lines executed | `examples_suite.rs` |
| `extern/` | 3 | `examples_suite.rs` (wave 21) | directory scan; index row | `examples_suite.rs` |
| `unsafe-linkage/` | 1 | `examples_suite.rs`, `examples_cli.rs` (wave 21) | directory scan; index row; published `quirin` command lines executed | `examples_suite.rs` |
| `threads/` | 4 | `thread_examples.rs` | directory scan; index row **only** — the wave-21 boundary is kept | `thread_examples.rs` |
| `events/` | 4 | `std_event.rs`, `compose_event_lines.rs`, `socket_event_compose.rs` | four transcript markers + extraction; route assertion ×4; the check-only arm pinned as the only program that emits no C | unchanged (three card files) |
| `strings/` | 2 | `string_slice_io.rs`, `string_concat.rs` | two transcript markers + extraction; route assertion ×2; the CRLF framing pinned on the projection | unchanged (two card files) |
| `sockets/` | 2 | `std_socket.rs` | transcript extraction under the existing markers; route assertion ×2; published `quirin` command lines executed | unchanged (`std_socket.rs`) |
| `http/` | 2 | `std_http.rs` | transcript extraction under the existing markers; route assertion ×2; published `quirin` command lines executed | unchanged (`std_http.rs`) |
| `loops/` | 1 | **nothing — new here** | everything, including the sanitized native run | `examples_suite.rs` |
| `cycle12.qcore` (no family) | 1 | nothing | route assertion; emits C; named on the index page for what it is | — |

Program counts are the scan's, not a list's: **29** `.qcore` programs across 11
family directories, plus `cycle12.qcore` at the root — 30 in all — and one
support contract module under `extern/contracts/` that the scan deliberately
does not treat as a published program.

## 3. Exact test names

`compiler/tests/examples_suite.rs` — 11 new tests (27 → 38):

| Test | What it establishes |
|---|---|
| `every_published_program_is_indexed_and_every_indexed_program_exists` | the directory scan, both directions, over every family |
| `the_examples_index_names_every_family_and_only_real_ones` | `examples/README.md` ↔ the directory listing, both directions |
| `the_newer_families_take_the_routes_their_pages_say_they_take` | 12 route assertions, including the two deliberate contrasts |
| `the_check_only_example_is_the_only_one_that_emits_no_c` | `events/write_interest.qcore` emits nothing; every other program here does |
| `the_newer_pages_publish_the_transcripts_their_programs_produce` | 10 transcript equalities against the reference projection (9 programs, one with two streams), plus `computed_length`'s CRLF case |
| `the_newer_transcript_checks_would_notice_a_changed_line` | 12 non-vacuity pins on the extraction above |
| `the_loop_example_runs_clean_and_matches_its_reference_execution` | the new family's sanitized native run, both streams, against the model and the page |
| `the_loop_page_publishes_the_c_the_backend_really_writes` | the page's emitted-C block, extracted and found in the real output; body emitted once |
| `the_documented_zero_bound_refusal_is_the_refusal_the_compiler_gives` | the page's refused program compiled and its quoted `Q098` compared with `format_diagnostic` |
| `the_loop_refusals_the_page_names_beside_it_hold` | `Q097` ×2, `Q099`+`Q054`, `Q013` |
| *(modified)* `block` | made newline-agnostic — see §5.6 |

`crates/qcorec/tests/examples_cli.rs` — 1 new test (5 → 6):

| Test | What it establishes |
|---|---|
| `every_published_driver_command_line_names_a_real_file_and_works` | every `quirin check`/`quirin emit` line any family page publishes names a file that exists; all six published `check` lines run and succeed |

That last one is the card's one unplanned finding, and it is a small one: six
pages publish a `quirin check` command line and **none of the six was executed
by anything** before this test. Wave 21's
`every_example_the_pages_say_checks_through_the_driver_does` checks eight
examples by an explicit list, which overlaps but is not the same statement — it
does not read the command line off the page. All six pass; the counts (6 and 6)
are pinned so that a page adding or dropping a command line is a thing somebody
looks at.

## 4. The loop example

`examples/loops/buffer_copy.qcore`, on `source.buffer.v1`, which is the only
route v1 admits `for` on.

**Shape.** The brief named it: the buffer fill/copy shape card `LOOP-V1` landed
as a test fixture. The program fills a four-byte `src` with `81 67 111 114`
(`QCor`), takes `let n = extent(src);`, and copies into `dst` with

```qcore
for i in 0..n {
    let v = src[i];
    dst[i] = v;
}
```

adding **no byte statement**: the read and the write are `source.buffer.v1`'s
own. A second loop over a literal bound renders its own index, so the page shows
both bound tiers (`extent(NAME)` and a literal) and both body kinds (a byte
write and a line). The bound reaches the logger's stream as `[INFO] 4`, so the
example writes both streams and the comparison below is per stream.

**Comparison result.** The emitted C builds under
`-std=c11 -Wall -Wextra -Werror -pedantic` with `-fsanitize=address,undefined`
and runs clean (LSan on Linux), and both streams are **equal** to the reference
execution's projection and to the transcript read out of the page:

```text
stdout: 0 1 2 81 67 111 114        stderr: [INFO] 4
```

The point of that equality is `LOOP-V1`'s own: the reference execution
**unrolled** the loop and the emitted C **rolled** it — the page publishes
`for (int32_t s_i = INT32_C(0); s_i < INT32_C(4); s_i += INT32_C(1)) {`, the
test finds that exact line in the real output, and pins that the copy's store is
emitted **once**. So the native run is a check of the unrolling rather than a
restatement of it. The leak control is positive: two buffers allocated and
released, one of the four writes into the second performed only by the loop
body.

**The page's refusals.** One refused program is published and compiled — a bound
of zero, `Q098` — with its diagnostic quoted and compared against
`format_diagnostic` including the span caret. Three more are named in a table
and checked by code: `break;` → `Q097`, `return;` → `Q097`, an unreleased
`let t = bytes(2);` → `Q099` then the unedited ownership checker's `Q054`. And
one that is deliberately *not* this route's: naming the index after the loop is
`Q013`, unknown name, because the binding is body-local.

**Constructs used.** Only what is on main at `e26bb0c3`: `for … in 0..BOUND`,
`bytes`, `[i]`, `extent`, `drop`, `print dec`, `log info dec`, `flush`. No
unroll attribute, no inet address — the two parallel in-flight cards are neither
used nor anticipated.

## 5. README repairs

Each was found by writing the test that would have caught it, and each is fixed
in the newline-agnostic style.

1. **`examples/sockets/README.md` published a retired gap as a live one.** Its
   "What these examples do not show, because it does not exist in v1" list said
   "No composition with `source.event.v1` — registering a socket with the
   reactor is `SOCKET-EVENT-COMPOSE`". That card landed in wave 25:
   `std_socket.rs`'s own test is named
   `a_module_that_dispatches_and_sockets_reaches_this_route_and_is_now_admitted`,
   and `examples/events/socket_server.qcore` is the published example. The row
   now says the composition landed, says why that example is indexed on the
   events page, and links it.
2. **`examples/README.md` claimed every program runs.** Its opening paragraph
   said every program "emits C11 that builds under `-Wall -Wextra -Werror`, and
   runs under the sanitizers". One does not:
   `events/write_interest.qcore` is the event route's check-only arm, accepted
   and checked in full and deliberately not executed — its own page says so, and
   the index did not. The paragraph is corrected and the exception is named.
   `the_check_only_example_is_the_only_one_that_emits_no_c` pins **both**
   halves: that one emits nothing and that every other one emits C.
3. **`examples/README.md` said "the first five directories"** and listed test
   commands for four families out of eleven, omitting `sockets/`, `http/`,
   `string_concat`, `compose_event_lines` and `socket_event_compose`. Replaced
   with a per-directory native-runner table, so a reader can find the file that
   owns any family's run.
4. **`examples/README.md` indexed no `loops/` row and did not mention
   `cycle12.qcore`.** The row is added; `cycle12.qcore` — which predates the
   directories and belongs to no family — is now named for exactly what it is,
   with the one thing the suite holds it to (it still checks).
5. **Markers added where the convention had not reached.** Six
   `<!-- transcript: … -->` markers: two on `examples/strings/README.md`, four
   on `examples/events/README.md`. One `<!-- emitted: copy-loop -->` on the new
   loops page. No existing marker was moved or renamed, and the four card tests
   that read those two pages by heading split or containment still pass
   unedited.
6. **`block()` in `examples_suite.rs` was not newline-agnostic.** It split on a
   spelled `"-->\n"`, which a CRLF checkout would not match. Normalised to
   `\r\n` → `\n` first, on `string_slice_io.rs`'s recorded
   `WINDOWS-NATIVE-SOCKETS-HARNESS` precedent — `examples_cli.rs`'s copy already
   did this. On a Unix checkout the normalisation is the identity, so nothing
   the file already pinned moves. (The suite file stays `#![cfg]`-gated to Linux
   and macOS; `EXAMPLES-SUITE-WINDOWS` is unchanged and still open.)

Four pages additionally gained one sentence each naming the suite beside their
card test — `strings/`, `sockets/`, `http/`, `events/` — because the suite now
checks those pages and saying so is the honest form of the claim those pages
already make about themselves.

## 6. The three follow-ons, dispositioned

| Follow-on | Disposition |
|---|---|
| `STRING-SLICE-IO-EXAMPLE-SUITE` | **Closed.** `examples/strings/` is under the directory scan, both programs' routes are asserted, both transcripts carry markers and are extracted and compared with the reference execution. `string_slice_io.rs` and `string_concat.rs` keep their sanitized native runs, unedited; the suite does not build either program a second time. |
| `HTTP-V1-EXAMPLE-SUITE` | **Closed.** Same, for `examples/http/`, plus the page's deliberate route contrast (`raw_request.qcore` is `source.socket.v1`'s) asserted, plus its published `quirin check`/`emit` command lines executed. `std_http.rs` keeps its native run and its wire-bytes comparison. |
| `LOOP-EXAMPLE` | **Closed.** `examples/loops/` is published — one program, one page held to this row's standard, wired into the suite including the sanitized native run the suite owns for it. |

The sibling gap the same rows named — `examples/sockets/` was never indexed and
has the same arrangement — is closed in the same way, and closed **structurally**:
the scan is what keeps it closed, not this card's diligence.

## 7. Why there is no Lean

Stated plainly rather than assumed. The close standard is Rust plus audited
Lean *for a card that moves a semantic surface*. This card moves none:

* no file outside `compiler/tests/`, `crates/qcorec/tests/`, `examples/` and
  `docs/reports/` is touched — no crate source, no contract module, no schema;
* no diagnostic code, message or attribution changes; no new keyword (the census
  discipline of constraint 14 is not engaged, because no word is reserved);
* no route admits anything it did not admit at `e26bb0c3`, and no `Unknown`
  becomes `Safe`;
* the one new `.qcore` program uses only constructs already checked and already
  proved, and every claim its page makes is asserted by a test rather than by a
  new definition.

There is therefore nothing for a Lean definition or theorem to match. The Lean
audit count is unchanged by this card, and `AuditAll.lean`'s axiom closure is
untouched.

## 8. Gate

Run in this worktree at the card's tip, on Linux, with the Libevent submodule
initialised at the pinned revision `d82464a2…`, `$HOME/.elan/bin` on `PATH`.

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | `third_party/libevent` at `d82464a277d0f42703702c4dfd9af6af38595a83`, the pinned revision |
| `cargo test --workspace` | **4,003 passed, 1 failed, 8 ignored** across 186 test binaries — the one failure is the constraint-16 row below; **4,004 pass** with it re-run in isolation |
| *(constraint 16)* `cargo test -p qcore-verifier-host --test confined_process_execution -- --test-threads=1` | **12 passed, 0 failed**, exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **107 Lean audits passed**, exit 0 — unchanged by this card |
| `python3 -m unittest` over `scripts/tests` (21 modules, two batches) | **114 + 79 = 193 passed**, exit 0 — run for completeness; no script is touched |

**The one failure is a recorded one, and it is reported rather than absorbed.**
`a_confined_fixture_cannot_write_outside_the_admitted_paths` failed under the
full workspace run with

```text
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

which is character-for-character the third row of standing constraint 16
(P02-FLAKE): "the confined-fixture write test (`Io(Broken pipe)` under load)".
It passes in isolation, with all twelve tests of its binary, on the same binary
and the same tree. Nothing about it was weakened, and no other constraint-16 row
tripped.

`AGENTS.md`'s headline row reads 3,580 workspace tests at the wave-23
fixed-scalar checkpoint; waves 24 and 25 have grown it since. The figure this
card should be reconciled against is the current main's, not 3,580. This card
adds **12** tests (11 + 1) and **no** Lean audit; the Lean count here (107) is
the merged tree's at `e26bb0c3`, above the ledger's 95 for the same reason.

**Corpus neutrality** is by construction: this card adds no checker code, no
contract module and no analysis route, so the 185-unit corpus baseline, the
**272** leak diagnostics (standing constraint 7), the **72**-module contract
corpus and the CT mapping are untouched. Constraint 16's five load- or
parallelism-sensitive rows were not tripped; no provider file is touched, so
constraint 18 is not engaged.

Every pre-existing example program is byte-identical — `git diff --stat` over
`examples/` shows five README files and nothing else, beside the new
`examples/loops/` directory.

## 9. Follow-ons

Named, not silently deferred.

1. **`EXAMPLES-SUITE-WINDOWS`** (pre-existing, unchanged) — `examples_suite.rs`
   is still `#![cfg(any(target_os = "linux", target_os = "macos"))]`, so section
   6 runs on the same two arms as the rest of the file. The page-level checks in
   it need no C compiler and would run anywhere; splitting them out from the
   `cfg` gate is the shape that card would take, and this one deliberately did
   not widen a `cfg` it was not asked to.
2. **`EXAMPLES-LOOPS-CLI`** — the loops page publishes a `cargo test` command
   line and no `quirin` one, unlike six of its neighbours. The driver does check
   the program; the reason the page does not say so is that adding a seventh
   command line would move the counts
   `every_published_driver_command_line_names_a_real_file_and_works` pins, and
   that is a decision for whoever also decides whether every page should carry
   one.
3. **`EXAMPLES-INDEX-TEST-COMMANDS`** — the per-directory native-runner table
   added to `examples/README.md` in repair 3 names nine `cargo test` targets.
   Nothing checks that those targets exist. A test that runs
   `cargo test --list` per target, or reads the file names out of
   `compiler/tests/`, would close it; it was out of this card's scope.
4. **`EXAMPLES-ERROR-REGION`** (pre-existing, unchanged) — typed recoverable
   errors and checked cleanup still have no family directory.
5. **`STD-BUFFER-EXAMPLE`** (pre-existing) — **not** closed by this card, and
   the distinction matters. `examples/loops/buffer_copy.qcore` is a *loop*
   example that happens to run on the buffer route; it shows a fill and a copy
   and nothing else of `source.buffer.v1`. A buffer family page would be about
   the owned extent-carrying byte buffer itself.

## 10. Integrator reconciliation

* **Files added:** `examples/loops/buffer_copy.qcore`,
  `examples/loops/README.md`, this report.
* **Files modified:** `compiler/tests/examples_suite.rs` (section 6 appended;
  `block()` made newline-agnostic; header note),
  `crates/qcorec/tests/examples_cli.rs` (one test appended),
  `examples/README.md`, `examples/events/README.md`, `examples/http/README.md`,
  `examples/sockets/README.md`, `examples/strings/README.md`.
* **No example program is modified.** The wave-21, wave-22, wave-24 and wave-25
  programs are byte-identical.
* **Headline numbers:** `AGENTS.md`'s merged-gate workspace test count grows by
  **12** from whatever the current main figure is. No other headline number
  moves: not the corpus, not the leak count, not the CT mapping, not the Lean
  audit count, not the contract corpus, not any version.
* **`DEVELOPMENT.md`:** one "Increment records" paragraph, per the sync rule.
  This card did not write it (integrator-owned file).
* **Matrix:** `docs/reviews/r0_3/acceptance_matrix.json` carries
  `STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE` in
  the `EXAMPLES-SUITE` row's narrative. This card did not edit it — a parallel
  card owns that file — and §6 above is the disposition the integrator should
  carry across.
