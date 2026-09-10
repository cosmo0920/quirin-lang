# EXAMPLES-SUITE — runnable, tested examples for the shipped feature families (wave 21)

Card brief: *"Runnable, tested example programs for the shipped feature
families, beyond `examples/threads`. Named non-optional deliverable: std.io and
std.logger examples."* Then, in priority order: pipes, branching, declared
extern calls, unsafe linkage.

Base commit: main `45f6c74c`.

**What landed.** All five families, none omitted. Fourteen example programs in
five directories, four supporting files (one C library and its declared
contract, one uncontracted C library and its native-library manifest), six
README pages, and **32 new tests** — 27 in
`compiler/tests/examples_suite.rs` and 5 in `crates/qcorec/tests/examples_cli.rs`
— including **22 sanitized native runs**. Zero compiler, checker or backend
changes: the diff outside `examples/` is two new test files and this report,
and no existing file in the repository is modified.
No new diagnostic, no new keyword, no Lean change, corpus-neutral by
construction.

---

## 1. The bar, and how it was met

`examples/threads/README.md` and `compiler/tests/thread_examples.rs` set the
standard this card was held to, and it was copied deliberately rather than
approximated. For every example in this suite:

1. it **checks** through the real pipeline, and takes the route its page says it
   takes (asserted, per example);
2. its emitted C compiles under `-std=c11 -Wall -Wextra -Werror -pedantic` and
   runs clean under ASan/UBSan, and under LSan on Linux;
3. its native **stdout and stderr each** equal the reference execution's
   projection of the same module — per stream, never as an interleaving,
   because C models no order between two separately buffered streams;
4. every transcript printed in its README is **read out of the README file** by
   the test and compared with the real run, so neither side can move without
   the other.

Two things were added beyond the threads precedent, both because this suite
documents refusals as well as behaviour:

* **Refused programs are extracted and compiled.** The card asked for refusal
  examples "as a documented refusal, not a broken file". The refused program
  lives in a fenced block behind an HTML-comment marker in the README, and the
  test reads it out, compiles it, and asserts the exact code list. No directory
  contains a file that does not check.
* **Quoted diagnostics are compared with the real rendering.** Where a page
  quotes a compiler message it does so in a second marked block, and the test
  compares it with `format_diagnostic`'s real output, span caret included.

The marker convention is `<!-- transcript: NAME stream -->`,
`<!-- refusal: CODE -->`, `<!-- diagnostic: CODE -->` and `<!-- cli: LABEL -->`,
each immediately preceding the fenced block it names.

## 2. Family coverage

| Family | Directory | Shipped | Programs | Tests |
|---|---|---|---|---|
| `std.io` / `std.logger` / `std.fmt` **(named non-optional)** | `examples/stdio/` | yes | 4 | 8 + CLI |
| Pipes | `examples/pipes/` | yes | 3 | 6 + CLI |
| Branching | `examples/branching/` | yes | 3 | 5 + CLI |
| Declared extern calls | `examples/extern/` | yes | 3 | 5 + CLI |
| Unsafe linkage | `examples/unsafe-linkage/` | yes | 1 | 3 + 2 CLI |
| Concurrency | `examples/threads/` | **out of scope** — owned by card `COMPOSE-CONCURRENCY-REALWORLD`; not touched | — | — |

Nothing is named as an omission, because nothing was omitted. The concurrency
row is a card boundary, not a gap: `examples/threads` was not read from, written
to, or referenced except by cross-link.

### 2.1 What each program is

| Program | Route | Demonstrates |
|---|---|---|
| `stdio/renderings.qcore` | `source.std-io.v1` | `print dec`, `print hex`; the effect row is exactly `io` |
| `stdio/severities.qcore` | `source.std-io.v1` | `log info` / `warn` / `fatal`, `flush` |
| `stdio/barrier.qcore` | `source.std-io.v1` | `flush` alone; the smallest module on the route, writing nothing |
| `stdio/pipeline.qcore` | `source.std-io-extern.v1` | `value \|> render \|> log` with `succeeds INTEGER` and the `-> N` observation |
| `pipes/rendered_lines.qcore` | `source.std-io.v1` | one-stage pipes into both sinks |
| `pipes/scalar_stages.qcore` | `source.extern-call.v1` | a three-stage chain; hygienic temporaries; piped acquire and release |
| `pipes/scalar_stages_hand_written.qcore` | `source.extern-call.v1` | the same program with the temporaries named — the byte-identity twin |
| `branching/conditional_release.qcore` | `source.branch.v1` | the custody join |
| `branching/condition_tier.qcore` | `source.branch.v1` | constant binding, `\|` `^` `&`, `&&` `\|\|`, `!` |
| `branching/early_return.qcore` | `source.branch.v1` | an arm that returns (wave-20 `IF-ARMS`) |
| `extern/buffer_round_trip.qcore` | `source.extern-call.v1` | one resource's whole life; `succeeds true else raise` |
| `extern/scalar_results.qcore` | `source.extern-call.v1` | scalar results and §C04.1 |
| `extern/borrowed_render.qcore` | `source.std-io-extern.v1` | `print dec borrow NAME` of a live resource |
| `unsafe-linkage/unchecked_calls.qcore` | `source.extern-call.v1` | `unsafe extern fn` and printed waiver premises |

## 3. The named non-optional deliverable, item by item

The brief listed four things `examples/stdio/` had to carry. All four landed,
and each is checked:

| Asked for | Where | Checked by |
|---|---|---|
| `print` with both renderings (`dec`, `hex`) | `renderings.qcore` | route, single-emission-route, per-stream native run, README transcript |
| `log` at every severity (`info`, `warn`, `fatal`) | `severities.qcore` | same, on stderr |
| the README explains the `fatal` spelling — the `error` collision, the recorded decision | `README.md` §"Severities, and why the third one is called `fatal`" | `the_page_is_right_about_why_the_severity_is_spelled_fatal` compiles the `log error` spelling and asserts `Q030` **and** that the message names the collision |
| `flush` as sink handoff, README stating the no-durability boundary | `barrier.qcore`, README §"`flush` is a sink handoff, not a durability barrier" | route, empty-transcript native run, `flush 1;` → `Q030` |
| the composed literal chain `value \|> render \|> log` with an extern callee, exercising `succeeds INTEGER` and the `-> N` observation | `pipeline.qcore` | composed-route assertion, `if (v1 != 56)` / `return 70;` in the emitted C, the `-> 56` trace line beside the `56` rendering, both README transcripts |

The `fatal` paragraph states the decision in the repository's own terms: `error`
is the reserved error-type declaration keyword, the severity moved rather than
the pinned surface, and this follows the `work` → `effort` precedent
(constraint 14). The test makes that account falsifiable rather than
decorative.

## 4. Decisions worth recording

### 4.1 The refusal in a README is compiled, not transcribed

The pipes page publishes `total |> print dec |> log info dec;` as a refused
program and quotes its diagnostic. Both blocks are extracted by
`the_documented_pipe_refusal_is_the_refusal_the_compiler_gives`, and the
diagnostic comparison is an equality against `format_diagnostic`'s output — file
name, line, column, code, message, source line and caret. The same shape covers
the branching page's `Q046` conditional leak and its `Q045` parenthesization
refusal.

This is the mechanism that makes a "documented refusal" worth more than prose:
if the message text, the span or the code moves, the page fails rather than
becoming quietly wrong.

### 4.2 The lowering table is a byte identity, not a claim

`examples/branching/README.md` publishes a three-row table saying what `||`,
`&&` and `!` lower to. `every_row_of_the_documented_lowering_table_is_a_byte_identity`
compiles both spellings of each row and compares the emitted C byte for byte.
The arms are made **distinguishable** (one carries a `qcore_check` the other
does not, which leaves the ledger alone and so satisfies the join), so none of
the three equalities can pass by the arms being identical.

### 4.3 The CLI boundary is stated and pinned, not worked around

`qcorec`/`quirin` selects no extern contract environment, so five of the
fourteen examples cannot be checked through the driver: they are refused at
`Q023`. Three pages say so in those words and name the follow-on
`UNSAFE-LINKAGE-CLI`.

Rather than leave that as prose,
`the_contracted_examples_are_refused_by_the_driver_rather_than_admitted` runs
the driver over all five and asserts the refusal, and
`every_example_the_pages_say_checks_through_the_driver_does` runs it over the
eight that the pages say do check. A page claiming a boundary is only worth
having if something notices when the boundary moves — in either direction.

The driver binary is named through
`match option_env!("CARGO_BIN_EXE_quirin") { Some(p) => p, None => env!("CARGO_BIN_EXE_qcorec") }`
so the in-flight `qcorec` → `quirin` rename cannot break this file; README
command lines are spelled `quirin`.

### 4.4 The manifest digest is all zeros, deliberately

`examples/unsafe-linkage/unchecked_demo.qcorelib` states
`hash sha256:0000…0000`. This repository ships no prebuilt archive, and the
manifest identity is *a pin, not a probe* — nothing is hashed at check time, no
file is opened, and whether the linker used that artifact is the builder's act.
Writing a plausible-looking digest would have dressed a placeholder as a
measurement. The zeros are the honest spelling, the manifest comment and the
README both say what they stand for, and the README names the builder-side
follow-on `UNSAFE-LINKAGE-ARTIFACT-PROBE`.

### 4.5 A measured correction to a claim this card first wrote

The extern page originally said "move the release above **either** rendering and
the file refuses at `Q043`". Measured: moving it above the *first* rendering
refuses at `["Q024", "Q024"]` instead, because the intervening `demo_fill` and
`demo_sum` calls then use a consumed identity and are refused before the
rendering is reached. Only the second rendering isolates this feature's own
refusal.

The page and the example's own comment were corrected to say exactly that —
naming both refusals and which one belongs to the borrow feature — rather than
keeping the tidier sentence. `a_borrowed_render_after_the_release_refuses`
asserts the corrected form as an exact `["Q043"]`.

### 4.6 Shared support files rather than four copies

`examples/extern/buffer_api.c` and `examples/extern/contracts/buffer_api.qcore`
are used by the extern, stdio and pipes families. The extern directory owns them
because that is the family whose subject they are; the other two pages link to
them and say so. The callees are named `demo_*` rather than the fixtures'
`qcore_extern_*`, so a reader is not invited to think they are builtins.

### 4.7 A positive leak control in every extern family run

Each of the five programs run natively against `buffer_api.c` allocates a real
buffer and releases it, so the LSan check is a positive control rather than a
vacuous one:
a lost release would be reported. The borrowed-render example strengthens it —
it renders the identity twice while the obligation is open, so a render that had
moved, duplicated or discharged custody would show up as a leak or a double
free rather than as a passing test.

## 5. What this suite does **not** establish

Stated here in the same words the pages use, because the pages are the surface a
reader meets:

* **Nothing about the C behind the declared contracts.** Every admitted callee
  is carried as `DeclaredContract("<name>")`, a premise the certificate records
  and the driver prints. A sanitized run can *refute* such a premise for these
  programs; no run establishes one in general.
* **Nothing whatever about the uncontracted callees.** The unsafe-linkage run
  establishes that the calls happened across a real link boundary in the checked
  order and left nothing behind. `demo_hidden_effect` mutates a private global
  and writes a line the checked observation does not mention — deliberately, so
  a reader can see the waiver is not a formality.
* **No extent is transported.** `NoExtentTransport` stands over every extern
  program here: that `demo_fill` writes eight bytes is a property of the C, not
  of any declaration.
* **No stream interleaving.** Every comparison is per stream.
* **No scheduling, no concurrency.** That family is `examples/threads`.

## 6. Gate

Run in this worktree at the card's tip, on Linux, with the Libevent submodule
initialised.

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | `third_party/libevent` at `d82464a2…`, the pinned revision |
| `cargo test --workspace` | **3,294 passed, 0 failed, 8 ignored**, exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **88 Lean audits passed**, unchanged by this card |
| `python3 -m unittest` over `scripts/tests` (21 modules) | **193 passed**, exit 0 |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **122 passed, 0 failed**, exit 0 — run for completeness; no provider file is touched, so constraint 18 is not triggered and no constraint-16 row needed isolation |

Two of those numbers are larger than `AGENTS.md`'s current headline row, and the
difference is not this card's: the ledger's "2,986 workspace tests · 80 Lean
audits" is the **wave-19 partial checkpoint**, and waves that merged since have
grown both. The pre-card baseline in this worktree is 3,262 workspace tests and
88 Lean audits; this card adds **32 workspace tests and no Lean audit**. The
integrator should reconcile against the current main, not against 2,986.

Corpus neutrality is trivial: this card adds no checker code and no contract
module, so the 185-unit corpus baseline, the 272 leak diagnostics and the
69-module contract corpus are untouched by construction.

The Python suites need the namespace-package form CI uses
(`python3 -m unittest scripts.tests.<module>`); `unittest discover` cannot start
in `scripts/tests`, which has no `__init__.py`. That is pre-existing and not a
finding of this card.

## 7. Test inventory

`compiler/tests/examples_suite.rs` — 27 tests, Linux/macOS only (the sanitized
native runs are Unix; the file is `#![cfg]`-gated exactly as
`thread_examples.rs` is):

| Section | Tests | Sanitized native runs |
|---|---|---|
| stdio | 8 | 8 |
| pipes | 6 | 4 |
| branching | 5 | 3 |
| extern | 5 | 6 |
| unsafe linkage | 3 | 1 |

`crates/qcorec/tests/examples_cli.rs` — 5 tests: the two published premise
listings compared with real driver output, the eight CLI-checkable examples
checked, the five contracted examples refused at `Q023`, and one `emit`
round trip.

## 8. Follow-ons

Named, not silently deferred:

1. **`EXAMPLES-SUITE-WINDOWS`** — the suite is Unix-only. `thread_examples.rs`
   has the same boundary; `std_modules.rs` and `std_io_compose.rs` show the
   `tests/support/windows_c.rs` route that would lift it. The card's brief did
   not ask for it and the sanitized-run harness would need a second arm.
2. **`EXAMPLES-CONTRACT-SLICE`** — `examples/unsafe-linkage/` ships an
   `unchecked fn` manifest only. The contract-slice level (L2), where a
   manifest's entries are `contract fn … from …`, is reachable only through the
   library API today for the same `Q023` reason, so an example of it would be a
   page with no runnable command line until `UNSAFE-LINKAGE-CLI` lands.
3. **`UNSAFE-LINKAGE-CLI`** (pre-existing) — five of the fourteen examples are
   library-API-only until it lands. This card pinned the boundary in both
   directions; when the flag arrives, `examples_cli.rs`'s two multi-family lists
   are the place the change shows up.
4. **`UNSAFE-LINKAGE-ARTIFACT-PROBE`** (pre-existing) — would let the example
   manifest carry a real digest instead of a documented placeholder.
5. **`EXAMPLES-QC-EXTENSION`** — every file this card added uses `.qcore`. One
   pre-existing file, `examples/cycle12.qc`, still carries the retired `.qc`
   extension. It is **not** renamed here: two archived reports
   (`docs/reports/remaining-p01/README.md` row 09 and
   `docs/reports/remaining-v05/README.md`) cite that exact path in pinned
   command lines, and constraint 21 forbids editing a frozen archive, so the
   rename needs a card scoped to publish provenance beside those archives
   rather than an incidental `git mv`.
6. **`EXAMPLES-ERROR-REGION`** — typed recoverable errors and checked cleanup
   appear here only incidentally (`else raise Failure`, `drop`). A family
   directory for them is the obvious next one and was outside this card's named
   priority list.

## 9. Integrator reconciliation

* **Files added, all new:** `examples/README.md`; `examples/stdio/` (4 programs
  + README); `examples/pipes/` (3 + README); `examples/branching/` (3 + README);
  `examples/extern/` (3 + `buffer_api.c` + `contracts/buffer_api.qcore` +
  README); `examples/unsafe-linkage/` (1 + `unchecked_api.c` +
  `unchecked_demo.qcorelib` + README); `compiler/tests/examples_suite.rs`;
  `crates/qcorec/tests/examples_cli.rs`; this report.
* **Files modified:** none.
* **`examples/threads/` untouched** — the `COMPOSE-CONCURRENCY-REALWORLD`
  boundary. `examples/README.md` links to it and cites its test command; if that
  card renames the directory or its test, the index table and one command line
  need updating.
* **Headline numbers:** `AGENTS.md`'s merged-gate test count grows by 32
  (27 + 5) **from whatever the current main figure is** — see §6 on why the
  ledger's 2,986 is a wave-19 checkpoint that later waves have already
  outgrown. No other headline number moves: not the corpus, not the CT mapping,
  not the Lean audit count, not the contract corpus, not any version.
* **`DEVELOPMENT.md`:** one "Increment records" paragraph, per the sync rule.
  This card did not write it (integrator-owned file).
* **Rename collision:** `crates/qcorec/tests/examples_cli.rs` resolves the
  driver binary through `option_env!("CARGO_BIN_EXE_quirin")` with an
  `env!("CARGO_BIN_EXE_qcorec")` fallback, so it survives the `qcorec` →
  `quirin` rename in either merge order. README command lines already say
  `quirin`.
