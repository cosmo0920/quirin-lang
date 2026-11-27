# C-EMIT-MAIN-UNIFY — one injected entry point, and a modular emission architecture

Wave 24, phase 2. User-directed, verbatim: *"The newly landed std_socket_c_emit
could define duplicated main entry points so we need to unity the main entry
point to inject and need to use a modular architecture."*

This card is the prerequisite for phase 3 (`HTTP-QUIRIN-V1`), which will compose
the socket and string routes into **one** emitted program and must not collide
on `main`.

---

## 1. The census, measured first

### 1.1 Where `main` was written

Ten source files each wrote their own entry point, in **four different byte
spellings**:

| # | Emitter | spelling at the site | prologue |
|---|---|---|---|
| 1 | `crates/qcore-core/src/c_emit.rs:164` (sequential) | `int main(void)\n{\n` | — |
| 2 | `crates/qcore-core/src/composed_std_io_c_emit.rs:176` | `\nint main(void)\n{\n` | — |
| 3 | `crates/qcore-core/src/task_c_emit.rs:513` | `int main(void) {\n` | the six `(void)&…` discards |
| 4 | `crates/qcore-core/src/std_string_c_emit.rs:224` | `\nint main(void)\n{\n` | — |
| 5 | `crates/qcore-core/src/std_socket_c_emit.rs:321` | `\nint main(void)\n{\n` | `qcore_socket_startup()` |
| 6 | `crates/qcore-core/src/mutex_c_emit.rs:341` | `int main(void) {\n` | `int qcore_exit = 0;` |
| 7 | `crates/qcore-core/src/extern_c_emit.rs:423` | `\nint main(void)\n{\n` | — |
| 8 | `crates/qcore-core/src/std_buffer_c_emit.rs:232` | `\nint main(void)\n{\n` | — |
| 9 | `crates/qcore-core/src/std_event_c_emit.rs:530` | `\nint main(void)\n{\n` | endpoint/watch arrays |
| 10 | `crates/qcore-core/src/std_io_c_emit.rs:200` | `\nint main(void)\n{\n` | the scalar literals |

Ten emitters, and exactly ten call sites in `driver.rs` — one in each
`compile_*_source`, each returning immediately, so no function calls two. The
epoch and composed-epoch routes publish `c_source: None` and emit nothing at all,
which is why they are absent from the table.

**After this card: one.** `qcore_core::c_program::ENTRY_SIGNATURE` is the only
`int main(` spelling in the crate, and `c_program::assemble` is the only code
that writes it. The four spellings survive as data — `Entry::lead` and
`BraceStyle` — rather than as four code paths, because both brace styles are
pinned by the F02 emission digests and normalizing them would be a byte move for
no stated gain.

### 1.2 Was the collision reachable today? **No — and the card is still justified.**

Measured, not assumed. The driver's route dispatch in
`crates/qcore-verification/src/driver.rs` (≈ lines 994–1160) is a chain of
mutually exclusive `if … { return compile_*_source(…) }` arms: a module carrying
an epoch scope never reaches the mutex arm, a module that sockets *and* buffers
stays with the buffer route, and so on. Exactly one `compile_*_source` runs per
program, so exactly one backend ever emitted, so exactly one `main` was ever
written.

Confirmed by emitting 33 programs covering all ten routes and counting: **1
`int main(` per emitted translation unit, 0 with two.**

So `std_socket_c_emit` did **not** in fact duplicate an entry point, and the
report says so rather than manufacturing a defect. What the user's concern names
correctly is the *architecture*: composition in the old shape meant writing a
new whole-program backend with a new `main`, which is exactly how
`composed_std_io_c_emit` (route #2 above) came to exist — it is the composition
of routes #7 and #10, hand-written, with its own eleventh entry point and its own
hand-copied prelude list. `HTTP-QUIRIN-V1` composing socket × string would have
been the twelfth. The card removes the reason to ever write one again.

### 1.3 Fragments before this card

One fragment mechanism already existed, and it is the in-repo precedent this card
generalizes: `io_emit::TEXT_PRELUDE`, added by `STRING-SLICE-IO`, is a
conditional prelude fragment inserted **only** into a program that renders text —
which is precisely what keeps every program that renders none byte-identical to
what it was before that card. Everything else was a `String` built by
`push_str` in the order each backend happened to choose.

---

## 2. The architecture, after

`crates/qcore-core/src/c_program.rs` (new). A backend now contributes two things:

* a `Vec<Fragment>` — file-scope text, each piece under a **key** that names the
  *thing* rather than the route that asked for it, and a `Section` that declares
  its position (`Banner < Config < Compat < Includes < Declarations < Prelude <
  FileScope`, the order C requires);
* one `Entry { lead, brace, prologue, body, epilogue }`.

`assemble(&[Fragment], &Entry) -> String` writes the deduplicated fragments, then
the entry. **The single-`main` guarantee is a type, not a check:** `assemble`
takes exactly one `Entry` by reference, so there is no call that produces two.
Two routes in one program merge two fragment lists and interleave two bodies into
the one entry — which is what `composed_std_io_c_emit` already did by hand for
its statements, now generalized to the preamble as well.

Also published, in the style of `io_emit::direct_stdio_calls` — decision
procedures the backends do not call and the tests and the Lean model do:

| Procedure | Decides |
|---|---|
| `deduplicate` | first-wins on the key alone, across sections |
| `conflicting_keys` | keys two contributors spell with **different** text — the check that first-wins discarded nothing anyone meant |
| `out_of_order_fragments` | positions where a fragment's section precedes its predecessor's |
| `merge` | the composition path: concatenate, deduplicate, then order stably by section |
| `entry_definitions` | entry definitions in emitted text — the single-`main` analogue of `direct_stdio_calls` |
| `preamble_text` | everything `assemble` writes before the entry (the task route's source-map offset reads it) |

### 2.1 Architecture map: each emitter → its fragment contributions

Shared fragments are named through **one** constructor each, so two routes
contributing one prelude spell one key: `io_emit::io_prelude_fragment`
(`qcore_io.prelude`), `io_emit::text_prelude_fragment`,
`io_emit::stdio_include_fragment` (`include.stdio`),
`diagnostic_log_c_emit::{diagnostic_log_fragment, adapter_declaration_fragment,
adapter_definition_fragment}`.

| Route | `preamble` fn | Fragments, in contribution order | Entry |
|---|---|---|---|
| sequential | `c_emit::preamble(log_level)` | `sequential.includes` · *(adapter decl)* · `qcore.diagnostic_log` · *(adapter def)* · `sequential.definitions` (error ABI, result struct, runtime helpers, function bodies) | lead `""`, Allman |
| `source.std-io.v1` | `std_io_c_emit::preamble()` | `std_io.includes` · `qcore_io.prelude` · `std_io.prelude` | lead `"\n"`, Allman, prologue = scalar literals |
| `source.extern-call.v1` | `extern_c_emit::preamble(program)` | `extern.banner` · `extern.includes` · `qcore.diagnostic_log` · `extern.prototypes` | `Entry::source_route` |
| composed std-io × extern | inline in `emit_composed_c11` | `composed.banner` · `composed.includes` · *(adapter decl)* · `qcore.diagnostic_log` · *(adapter def)* · `qcore_io.prelude` · `extern.prototypes` | `Entry::source_route` |
| `source.buffer.v1` | `std_buffer_c_emit::preamble(program)` | `buffer.includes` · `qcore_io.prelude` · `buffer.prelude` | `Entry::source_route` |
| `source.string.v1` | `std_string_c_emit::preamble(program)` | `string.includes` · `qcore_io.prelude` · *(`qcore_io.text_prelude`, only when the program renders text)* · `string.prelude_header` · `string.prelude` · `string.literals` | `Entry::source_route` |
| `source.socket.v1` | `std_socket_c_emit::preamble()` | `socket.compat` · `socket.includes` · `qcore_io.prelude` | lead `"\n"`, Allman, prologue = `qcore_socket_startup` |
| `source.event.v1` | `std_event_c_emit::preamble(cert, writes_lines)` | `event.compat` · `event.includes` · *(adapter decl)* · *(`include.stdio`, only without an adapter)* · *(`qcore_io.prelude`)* · `qcore.diagnostic_log` · *(adapter def)* · `event.prelude` | lead `"\n"`, Allman, epilogue `}\n` only |
| `CheckedMutexInvariantV1` | `mutex_c_emit::preamble()` | `c11_threads.compat` · `mutex.includes` · `qcore.diagnostic_log` · `mutex.prelude` | lead `""`, K&R, prologue `int qcore_exit = 0;` |
| checked tasks | `task_c_emit::preamble(cap, ev_cap, level, writes)` | `task.capacities` (Config) · `c11_threads.compat` · `task.includes` · *(adapter decl)* · `qcore.diagnostic_log` · *(adapter def)* · *(`include.stdio` · `qcore_io.prelude`)* · `task.runtime` · `task.definitions` | lead `""`, K&R |

Every conditional in that table is the **contributor's**, unchanged from before
this card. The assembler emits what it is handed; it does not decide whether a
program renders text or selected a system-trace adapter. Moving those decisions
into the assembler would have replaced measured facts with unconditional
fragments the assembler then had to remove again.

### 2.2 Why insertion order, not section order

`assemble` does **not** sort by `Section`. That was decided by measurement.

Sorting would have moved one program's bytes: the task backend places its
`#include <stdio.h>` *after* `DIAGNOSTIC_LOG_PRELUDE` rather than beside its
other includes, so a stable sort would have lifted that line above the logger
prelude. The move is inert in C — the logger prelude uses no stdio — but it is
still a byte move in a program the two F02 emission digests pin, and this card's
premise is that no such move happens silently.

The inversion is reported rather than hidden. `out_of_order_fragments` names it,
and `the_task_routes_late_stdio_include_is_the_only_section_inversion` pins that
it is the **only** one across every route's contribution, and that `merge`
repairs it — so the composition path, where two routes genuinely have no relative
insertion order, gets the order C requires.

Recorded as `C-EMIT-TASK-LATE-STDIO`: a stated architectural fact, not a defect,
repairable by a deliberate re-pin of the two F02 digests whenever a card has
reason to move them for its own sake.

---

## 3. Byte identity: the hard requirement, met everywhere

**Every existing program shape emits byte-identical C. No pin moved.**

Measured against a **pristine checkout of the base commit** rather than against
memory: `git archive a759abf8` was unpacked into a worktree-private `measure/`
directory, the same temporary harness (removed before the commit) was built in
both trees, and each compiled the same inputs through the real pipeline and wrote
the emitted C to disk. Result:

* **33 emitted translation units, 33 identical MD5 digests, 0 differing bytes.**
  `diff -rq` over the two directories reports nothing. The digest table is
  archived at `artifacts/emission_digests.txt`.
* **Every head unit still carries exactly one `int main(`**, as every base unit
  did.
* Coverage: **all ten routes**, with at least one program each —
  `branching/*` + `cycle12` (sequential, 4), `stdio/*` + `pipes/*` (std-io, 4),
  `extern_calls` fixtures (extern alone, 6), `stdio/pipeline` + `extern/*` +
  `unsafe-linkage/*` (composed, 5), `std_buffer` fixtures (buffer, 2),
  `std_string` fixtures + `strings/status_line` (string, 3), `sockets/*`
  (socket, 2), `events/*` (event, 2), a guarded fixture (mutex, 1),
  `threads/*` (task, 4).

The three routes no published example takes — extern-alone, buffer and mutex —
were reached through their own test fixtures precisely so that the byte claim
covers every emitter rather than only the ones `examples/` happens to exercise.

The pins that would have caught a move, all **unchanged and passing**:

| Pin | Where | Status |
|---|---|---|
| F02 emission digests `6524b41d0ccb910f`, `0719a8dcb0192e7e` | `compiler/tests/f02_trace_relation.rs:67` | unmoved |
| the same two, `\r\n`-normalised | `compiler/tests/windows_threads.rs:26` | unmoved |
| `the_scalar_only_fixture_is_byte_identical` | `compiler/tests/string_slice_io.rs:639` | passing |
| exactly one `fwrite(`, exactly one `fprintf(` | `compiler/tests/string_slice_io.rs:223` | passing |
| `direct_stdio_calls` empty, five call sites | `examples_suite.rs`, `std_modules.rs`, `std_socket.rs`, `string_slice_io.rs`, `std_buffer.rs`, `compose_event_lines.rs` | passing |
| `emitted_c_contains` | `fixtures/ct/CT34/expected.json`, `CT26/expected.json` | passing |
| emitted-C indentation and column-0 rules | `crates/qcore-core/tests/unit/c_emit.rs:100`, `:125` | passing |
| `examples/threads` transcripts, `examples/sockets` transcripts | `thread_examples.rs`, `std_socket.rs:1062` | passing |
| `source_map` line numbers | `epoch_tasks_source.rs:228`, `module_modes.rs:145` | passing |

The one place the refactor could have shifted a number without shifting a byte is
the task route's **source map**: its mappings are generated line numbers counted
from the top of the file, and the preamble is no longer spliced into the same
`String` they are measured against. They are shifted once, by
`preamble_text(&fragments).lines().count()`, which reproduces the old numbers
exactly — every fragment ends in a newline, so no line splices at the boundary.
`epoch_tasks_source.rs`'s `assert_eq!(composed_module.source_map, plain.source_map)`
is the pin that would have caught an off-by-one, and it passes.

## 4. The single-emission-route property, re-verified

`TRACE-STDIO`'s property — every print site on one `QCORE_IO` route — is
unchanged and was re-checked after the refactor at all five `direct_stdio_calls`
assertion sites plus the two exact-count assertions from `STRING-SLICE-IO`.

`QCore.SingleEmissionRoute`'s theorems still describe the real emitter, and the
refactor **strengthened rather than moved** what they quantify over:
`concatenation_is_violation_free_iff_both_are` said a program assembled from a
prelude and several backend fragments is violation-free exactly when every
fragment is — which was the formal licence for installing the prelude once and
routing each write. That was a statement about an architecture the code did not
yet have. It now does: `io_emit::io_prelude_fragment` is a single named fragment
with a single key, and `a_shared_key_survives_a_merge_once` proves a composed
program carries one copy. No Lean was changed; none needed to be.

## 5. Lean

`proof/QCore/CProgram.lean`, registered in `proof/AuditCProgram.lean` and
`proof/QCore.lean`. Placeholder-free; axiom closure `propext`,
`Classical.choice`, `Quot.sound` only.

* **Deduplication** — `deduplicate_keys_nodup` (no repeat survives),
  `deduplicate_idempotent` (assembling an assembled contribution is the same
  assembly), `deduplicate_is_a_sublist_in_membership`.
* **One entry point** — `assemble_contains_the_entry` (non-vacuity),
  `assemble_writes_exactly_one_entry` and
  `an_empty_contribution_still_has_one_entry`, with the honest converse
  `a_smuggled_entry_is_counted_too`: the assembler guarantees it *writes* one,
  not that no contributor carried another, which is exactly why
  `entry_definitions` exists beside the type and why the Rust suites scan emitted
  text.
* **Composition** — `a_shared_key_survives_a_merge_once`,
  `a_merged_program_still_has_one_entry`,
  `merge_preserves_the_union_of_keys` (nothing vanishes silently, so a
  fail-closed reading is justified),
  `merging_puts_the_compatibility_header_first`, `merging_is_idempotent`.
* **Two-sided domain tie** — `the_domain_outcomes_are_pinned` pins
  `[0,1,1,2,2,1]` (fragments surviving deduplication) and `[1,1,1,1,1,2]` (entry
  definitions) over six fragment lists; `the_domain_outcomes_agree_with_the_lean_model`
  in `crates/qcore-core/tests/unit/c_program.rs` computes the same two vectors
  independently from the Rust procedures and compares them. The trailing `2` is
  the stated boundary made observable.

**Not proved.** That the assembled text is valid C — this is concatenation in a
declared order, and each fragment's correctness is its contributor's. That the
Rust backends call the modeled subject — they are tested, not modeled. That any
program's *bytes* are unchanged — that is §3's measurement, which a model cannot
establish.

## 6. Boundaries and what did not change

* **No new diagnostics.** No `Q` code added, moved or reused. `Q090`–`Q099`
  stays reserved for `HTTP-QUIRIN-V1`.
* **No new keywords**, so constraint 14's fixture census is untouched.
* **No behaviour change.** No program that compiled stops compiling; none that
  refused starts compiling. Every route's refusals, conditionals and selection
  order are unmoved — `driver.rs` was not edited at all.
* **Corpus-neutral.** `crates/qcore-c-gateway`, `contract/qcore` and `providers/`
  are byte-untouched. Leak diagnostics stay **272** (constraint 7); the 185-unit
  corpus is not re-measured and not moved.
* `entry_definitions` is a **lexical** scan, and its boundary is stated on both
  sides exactly as `direct_stdio_calls`' spelling table is: a translation unit
  could name an entry point in a way the scan does not spell — a differently
  declared `main`, or one reached through a macro.

## 7. Named follow-ons

* **`C-EMIT-TASK-LATE-STDIO`** — the task backend's `#include <stdio.h>` sits
  among its preludes rather than beside its includes (§2.2). Reproduced, pinned
  as the only inversion, and repairable only together with a deliberate re-pin of
  the two F02 emission digests.
* **`C-EMIT-COMPOSED-VIA-MERGE`** — `composed_std_io_c_emit` still spells its own
  fragment list inline rather than calling
  `merge(&[extern_c_emit::preamble(…), std_io_c_emit::preamble()])`. Doing so
  would move bytes (the two routes' include blocks and banner differ from the
  composed route's own), so it is a deliberate follow-on with a re-pin, not a
  cleanup to slip in here.
* **`C-EMIT-ENTRY-SCAN-COMPLETENESS`** — the sibling of
  `TRACE-STDIO-ROUTE-COMPLETENESS`: `entry_definitions` decides a spelling, and a
  program could declare an entry point some other way.
* **`C-EMIT-CONFLICT-GATE`** — `conflicting_keys` is a decision procedure the
  tests run, not a gate the assembler enforces. Making it fail-closed inside
  `assemble` needs an error type across ten backends' refusal enums; worth doing
  when a composition first has two contributors that could plausibly disagree.
* **`C-EMIT-PROGRAM-DEPENDENT-PREAMBLE-PINS`** — four routes' preambles
  (`string`, `buffer`, `event`, and the task route's `task.definitions`) are
  built from a checked program or certificate, and the composed route's list is
  still inline, so the three unit-level architecture tests
  (`no_two_routes_disagree_about_a_fragment_they_share`,
  `the_task_routes_late_stdio_include_is_the_only_section_inversion`,
  `no_routes_preamble_defines_an_entry_point_of_its_own`) walk the constant
  preambles plus the task route's. The others are covered by §3's byte
  comparison and by their own suites, not by those three tests.

## 8. The standing gate

Run in full on this worktree, after the refactor and with the harness removed:

| Leg | Result |
|---|---|
| `cargo test --workspace` | **exit 0** — 202 suites, **3,883 passed, 0 failed, 8 ignored**, 0 `FAILED`/`panicked` lines |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **"Passed 102 Lean audits"**, `AuditCProgram.lean` among them, no failures |

The Lean leg is read from the TEXT line rather than the exit code, per the
script's own behaviour. `AuditCProgram.lean`'s 23 theorems print closures inside
`propext` / `Classical.choice` / `Quot.sound` only.

Constraint 16's five load-flaky rows were not tripped: no failure of any kind
appeared, so none needed isolated re-verification.

## 9. Files

New: `crates/qcore-core/src/c_program.rs`,
`crates/qcore-core/tests/unit/c_program.rs`, `proof/QCore/CProgram.lean`,
`proof/AuditCProgram.lean`, this report and `artifacts/emission_digests.txt`.

Changed: the ten emitters above, plus `crates/qcore-core/src/io_emit.rs` and
`crates/qcore-core/src/diagnostic_log_c_emit.rs` (fragment constructors and keys
for the shared preludes), `crates/qcore-core/src/lib.rs` (module registration),
`proof/QCore.lean` (import), `compiler/tests/examples_suite.rs` (one test).
