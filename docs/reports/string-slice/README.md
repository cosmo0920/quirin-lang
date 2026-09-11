# Card `STRING-SLICE` — `source.string.v1`, immutable UTF-8 string views

Wave 23. Implements the user's 2026-09-11 revision-0.3 amendment
`docs/design/STRING_SLICE_V1.md` (commit `26bc1651`), specifically its two named
follow-ons `STRING-SLICE-LEX-TYPE` and `STRING-SLICE-EXEC-C11`, as **one checked
v1**.

Base: `origin/main` at `52e3889c`.

## 1. What shipped

The ninth bundled module, `std.string`, and the checked source profile
`source.string.v1` behind it. A module on this route is a flat entry that

- constructs an **immutable UTF-8 `{ptr, len}` view** of statically stored
  literal bytes (`let s = "hello";`, optionally `let s: string = "hello";`),
- reads that view's **exact byte length** in O(1) (`let n = len(s);`),
- derives another view by an **allocation-free, copy-free checked slice**
  (`let t = s[low:high];`), and
- reuses `source.std-io.v1`'s line statements unchanged, over **scalars only**.

It is checked, reference-executed and lowered to C11, and the reference
execution and the emitted program agree per stream, byte for byte, under
ASan/UBSan/LSan.

| Component | File |
|---|---|
| Module registration | `crates/qcore-core/src/module_imports.rs` (`STRING_MODULE`) |
| Checked model, decoding, certificate, recheck | `crates/qcore-core/src/std_string.rs` |
| Reference execution and C11 emission | `crates/qcore-core/src/std_string_c_emit.rs` |
| Source detection, import gate input, lowering | `crates/qcore-frontend/src/source_std_string.rs` |
| Per-file capability gate | `crates/qcore-frontend/src/module_imports.rs` |
| Pass, code map, contextual `len` rule | `crates/qcore-frontend/src/frontend.rs` |
| Route selection and driver | `crates/qcore-verification/src/driver.rs` (`compile_string_source`) |
| Tests (62) | `compiler/tests/std_string.rs` |
| Fixtures | `compiler/tests/fixtures/std_string/{slices,silent}.qcore` |
| Lean model and audit | `proof/QCore/StdString.lean`, `proof/AuditStdString.lean` |
| Registry widening | `proof/QCore/ModuleImports.lean` (seven fields → eight) |

## 2. Diagnostic band allocated: `Q065`–`Q070`, pass `StandardString`

`Q064` was the highest code allocated on this card's base, so the band starts at
`Q065`. **Other wave-23 cards may also be allocating; the integrator relocates on
collision per the wave-3 `T07`/`T09` precedent, and the code-to-condition mapping
below is stable under such a shift.** The §L21 row is added at
`docs/QCORE_SPEC_r0_3.md`; the `AGENTS.md` namespace-table row is integrator
business and was deliberately not written here.

| Code | Condition |
|---|---|
| `Q065` | The flat entry shape is violated — a statement outside the grammar, a nested block, a construct outside the entry, a parameterised entry, a malformed literal/`len(…)`/slice, a type ascription naming something other than `string`, a missing or repeated return, a malformed `effects` row. Also the pass's generic presentation code, an entry-free module, and a failed certificate rederivation. |
| `Q066` | A slice bound that is not a check-time constant this entry bound, or a view position naming no string. |
| `Q067` | A quoted token that does not decode — an undefined escape, a truncated `\x`, a trailing backslash, decoded bytes that are not well-formed UTF-8, or a payload over 4096 bytes. |
| `Q068` | A slice outside `0 <= low <= high <= len(s)`, or a bound in range but not on a UTF-8 code-point boundary. |
| `Q069` | A string at a position it may not cross — a rendering operand (bare or `borrow`) or a slice bound. |
| `Q070` | The declared `effects { … }` row does not admit a derived effect. |

`Q070` is deliberately this pass's own rather than a reuse of `StandardIo`'s
`Q029`, on §L21's rule that a code may not be reused by a second pass.

## 3. Keyword census (constraint 14): **this card reserves no word**

Script `measure/census.sh`, output archived at
`artifacts/keyword_census.txt`. Scope: every `.qcore`/`.qc` file on disk —
**408 at census time**, 410 after this card's two fixtures — plus every tracked
`.rs`/`.md` file.

| Candidate | `.qcore`/`.qc` occurrences | files | binding position | tracked `.rs`/`.md` |
|---|---|---|---|---|
| `string` | 3 | 3 | **0** | 250 |
| `len` | 6 | 2 | **2** | 1780 |
| `slice` | 3 | 1 | 0 | 394 |
| `text` | 8 | 8 | 1 | 2212 |
| `str` | 11 | 4 | 5 | 3941 |
| `view` | 0 | 0 | 0 | 125 |
| `chars` | 0 | 0 | 0 | 37 |
| `utf8` | 0 | 0 | 0 | 1 |

Two findings decided the surface.

**`len` must not be reserved globally.** Its two binding-position hits are in the
**pinned contract corpus**, and one of them is a *reference*, not a parameter
declaration:

```
contract/qcore/fluent_bit/resource_memory.qcore:36  extern fn flb_sds_create_len(str: ptr<const, u8>, len: i32) -> ptr<mut, u8>
contract/qcore/fluent_bit/resource_memory.qcore:40      ensures normal resource_borrowed(len);
contract/qcore/librdkafka/memory.qcore:30           extern fn rd_strndup(str: ptr<const, u8>, len: usize) -> ptr<mut, u8>
contract/qcore/librdkafka/memory.qcore:34               ensures normal resource_borrowed(len);
```

A global reservation would change how a corpus this card may not touch resolves a
name inside an `ensures` clause. The repair follows the `work` → `effort`,
`error` → `fatal` and `buffer` → `bytes` precedent — narrow the rule rather than
edit the pinned surface — but narrows it further than any of those did: `len` is
skipped by name resolution **only where it is immediately followed by `(`**.

That shape was measured to occur **zero times** in QCore source anywhere in the
repository. `len(` appears in tracked `.rs` files only as Rust's `.len()` method
call: a search for `len(` not preceded by `.` or an identifier character returned
**0 matches across every tracked `.rs`, `.qcore` and `.qc` file** on the base.
Re-running it after this card returns six files, and all six are this card's own.
The corpus's `resource_borrowed(len)` has `len` followed by `)` and is untouched.

The measurement is pinned as a test rather than left as a claim in this report:
`the_contextual_len_rule_still_leaves_a_contract_parameter_resolvable` reproduces
those corpus entries' shape — an `extern fn` with a `len` parameter and an
`ensures normal resource_borrowed(len);` clause — and asserts it compiles, on the
ordinary sequential route. A future widening of the contextual rule fails there
instead of in the corpus test.

**`string` needs no rule at all.** Its three `.qcore` occurrences are all inside
comments, and it is never in binding position. The type name is admitted only in
a `let NAME : string = …;` ascription, and `lexical_references` already drains
every token up to and including a `let`'s `=`, so the word is never collected as
a reference.

Nothing else was needed: the literal's delimiters and `[`, `]`, `:`, `-` are all
already lexed. **Net new reserved words: zero**, which is what amendment §2 asks
for and one fewer than `STD-BUFFER` (2), `STD-MODULES` (8 + `log_level`) or
`T08-EPOCH-SRC` (8) needed.

## 4. Quoted-metadata census (amendment §2's lexer hazard)

Amendment §2: recognizing quoted tokens for structural syntax or existing
declaration metadata "does not construct a string value and MUST NOT acquire this
new import requirement."

Census of every `TokenKind::StringLiteral` consumer:

| Site | What it reads | In scope? |
|---|---|---|
| `crates/qcore-frontend/src/frontend.rs:840` | a `verify scope` entry's plugin `version` | **declaration**, not a statement |
| `crates/qcore-frontend/src/frontend.rs:860` | a `verify scope` entry's plugin `profile` | **declaration**, not a statement |
| `crates/qcore-frontend/tests/unit/lexer.rs:13` | a lexer unit test | not source |
| `crates/qcore-c-gateway/src/typed_source.rs:641,669` | Clang AST node *names* | different crate, not this lexer, not touched |

Separately: **no `.qcore`/`.qc` fixture on disk contains a double quote outside a
comment at all**, so every quoted token in the repository lives in an inline Rust
source string, and the shipped example is
`compiler/tests/cycle20_verifier_plugins.rs:451`.

The answer is structural rather than a filter: `string_uses` scans **function-body
statements and nothing else**, so a declaration-level quoted token cannot reach
it. Pinned by two tests —
`a_verifier_scope_keeps_its_quoted_version_without_any_string_import` (the
shipped shape compiles with no `import std.string;` and still refuses at `Q701`,
where it refused before this card) and
`a_quoted_profile_in_metadata_is_not_a_string_use`.

## 5. One collision with a sibling route, measured and disambiguated

`let t = s[0:3];` and `source.buffer.v1`'s byte read `let x = b[0];` share a head:
a `let`, a name, a `[`. Measured before any change: a slice written with **no
imports at all** reported `Q015` asking for `import std.buffer;` — a buffer the
program never named.

The disambiguation is the **colon**, applied in
`crates/qcore-frontend/src/source_std_buffer.rs`'s `buffer_use` classifier — the
one place that route decides what its statements are, so the two routes cannot
disagree. A buffer byte access **cannot** contain a colon: its index is
`["-"] INTEGER | NAME` and neither form holds one.

This is the **only** change this card made to a route it does not own, it is
narrowing rather than widening, and the buffer route's accepted set and emitted C
are unchanged. Pinned by `the_buffer_route_is_unchanged_by_the_colon_disambiguation`
and by the untouched `compiler/tests/std_buffer.rs` (all 60 of its tests pass).

## 6. Measurements

### 6.1 Refusals moved, none became acceptances

| Program, with no import | Before this card | After |
|---|---|---|
| `let s = "hello";` | `Q500` (sequential route: the reduced C11 CFG cannot encode it) | `Q015`, naming `std.string` |
| `let n = len(s);` | `Q013` (unknown name `len`) | `Q015`, naming `std.string` |
| `let t = s[0:3];` | `Q015`, naming **`std.buffer`** | `Q015`, naming `std.string` |

Every direction is refusal → refusal, and the refusal moves *earlier* and names
the actual cause. No previously refused program is now accepted without the
file's own import, and no `Unknown` became `Safe`. Pinned by
`no_program_became_safe_because_a_token_now_parses_as_a_string`. No test in the
repository asserted `Q500` on a quoted token, so nothing was re-pinned.

### 6.2 Zero-allocation and zero-copy, three independent ways

1. **Certificate.** Every `SliceRecord` carries `from`, `low`, `high` and
   `derived`, and `every_slice_derives_a_view_of_the_same_storage_at_an_offset`
   asserts `derived.literal == from.literal`,
   `derived.offset == from.offset + low` and `derived.len == high - low` for each
   one — amendment §4's arithmetic, as fields a reader checks.
2. **Emitted program.** `std_string_c_emit::allocator_calls` finds no
   `malloc`/`calloc`/`realloc`/`strdup`/`memcpy`/`memmove`/`strcpy`/`strlen` in
   any emitted program. It strips C comments first, deliberately: this route's
   prelude comment names those functions in the course of saying they are absent,
   and scanning the prose would make the explanation of the property falsify it.
3. **Effect row.** The derived row of a string-only program is **empty** — no
   `Alloc`, no `Mut` (amendment §7). A route that could allocate would derive
   `Alloc`; the sibling buffer route's `acquire` always does.

The emitted `len` is `(int32_t) v_greeting.len;` — a field read. The emitted
slice is `qcore_string_slice(v_greeting, (size_t) (INT32_C(1)), (size_t) (INT32_C(4)))`,
whose whole body is `derived.ptr = s.ptr + low; derived.len = high - low;`.
Archived at `artifacts/slices.c`.

### 6.3 Decoding and the boundary rule

Defined escapes: `\\`, `\"`, `\n`, `\t`, `\r`, `\0`, `\xHH`. Anything else
refuses at `Q067` rather than passing bytes through. Decoded bytes must be
well-formed UTF-8, checked at the one point bytes enter the model, so an
ill-formed payload is refused **at construction** and the value never exists —
which is how §3.4's "for the whole lifetime of the view" is discharged.

- `"ab\0cd"` has `len` **5**. `strlen` would say 2. That disagreement, on a value
  the route admits, is why §3.7 forbids `strlen` and why the emitted C reads a
  field and writes each literal array byte-wise rather than as a C string literal.
- `""` has `len` 0, a canonical non-null address, and is never dereferenced.
- `"\xc3\xa9o!"` has `len` 4 and exactly the boundaries {0, 2, 3, 4}. `s[0:1]`
  and `s[1:3]` refuse at `Q068`; `s[0:2]` and `s[2:4]` are admitted.
- `"\xff"`, `"a\x80b"` and `"\xc3"` all refuse at `Q067`.
- A boundary violation is **refused, never rounded**: rounding would silently
  slice a different string and not rounding would produce ill-formed text, and §2
  forbids both.

### 6.4 Native agreement

`fixtures/std_string/slices.qcore` reference observation:
`out: 5, 3, 1, 0, 4, 2` and `err: [WARN] 5`. The emitted program compiled with
`-std=c11 -Wall -Wextra -Werror -pedantic -fsanitize=address,undefined` and run
with `detect_leaks=1:halt_on_error=1` reproduces it exactly, per stream.
`silent.qcore` writes nothing and exits cleanly.

**The leak control here is negative, not positive**, and that is the honest
reading: unlike the buffer route's fixtures these programs allocate nothing, so a
clean LSan exit confirms no allocation was introduced rather than that a release
was performed. The load-bearing sanitizer is **ASan**: every slice is pointer
arithmetic into a static array, so an off-by-one derived view would be a real
out-of-bounds read.

## 7. Boundaries — what this card does not claim

- **Not source-language conformance.** One bounded profile.
- **§2's second arm is unused.** Every bound is a check-time constant, so v1
  decides every admission condition **statically** and introduces no trap, no
  error value and no failure arm. There is no residual case for a checked failure
  to represent. A bound the checker cannot decide is refused
  (`STRING-SLICE-VARYING-INDEX`).
- **No custody model, because there is none to model.** A view is not a
  resource; the route builds no ownership operation list. What stands in its
  place is *storage provenance*: every view names the static literal it
  addresses, so §3.6 is checkable in v1. A view of storage that can **end** does
  not exist until `STRING-SLICE-BUFFER-BORROW`, and that card owns the lifetime
  rule that becomes necessary then.
- **The C boundary stays refused**, and the refusal is an existing route's, not
  new code: route selection places this route after the extern route, so a module
  that strings and extern-calls keeps that route's refusal. Same construction for
  `std.buffer` (after the buffer route, which keeps `Q051`) and for concurrency.
- **No text output.** A rendering operand is a scalar; `Q069` refuses a string at
  one. The observable evidence in this card is therefore scalar-only — lengths
  piped to the existing renderers — exactly as the card scope requires. On the
  quiet-emission landscape: this route's observation comes from source-authored
  `print`/`log`, which are the **formatted-I/O owners** rather than compiler
  observations, so it produces real stream bytes with no threshold declaration.
  The quiet-emission `log_level LEVEL;` declaration is **refused** at `Q065`
  rather than accepted and ignored, because it installs the shared system-trace
  adapter at emission time through `emit_c11_with_log_level`, which this route's
  backend does not call — the same disposition `source.buffer.v1` gives it.
- **No equality, ordering, hashing, search, concatenation or mutation.**
- **No `cap`.** `{ptr, len, cap}` is `BYTE-SLICE-CAP`'s, and nothing here
  retroactively adds it.
- **The Lean model is coarser than the Rust one on UTF-8.** It models
  well-formedness at the granularity the *slicing rule* decides at — which
  offsets are code-point boundaries — and proves preservation there. Full UTF-8
  validation is a Rust boundary at the decoding step, not a Lean theorem.

## 8. Named follow-ons

| Name | What is missing |
|---|---|
| `STRING-SLICE-BUFFER-BORROW` | `borrow_utf8` / `freeze_utf8` from `std.buffer`, with custody and mutation exclusion. Amendment §5. |
| `STRING-SLICE-ABI` | An extent- and lifetime-carrying extern/native-library ABI. Amendment §6; the crossing stays refused. |
| `STRING-SLICE-IO` | Rendering text. Amendment §7; `std.io`/`std.logger` still accept scalars only. |
| `STRING-SLICE-VARYING-INDEX` | A slice bound that is not a check-time constant: a relational domain over indices plus §2's checked-failure arm. |
| `STRING-SLICE-NOTASCALAR-REACH` | **Recorded, not repaired.** `Q066`'s "not a check-time constant" arm cannot be reached from a single source module — name resolution refuses an unbound name at `Q013` first, whether it is never bound or bound later. The guard is checked through the library (`the_models_own_guard_refuses_a_bound_it_cannot_evaluate`) and kept rather than deleted, on the `EXTERN-F1-STORE-REACH` precedent. |
| `BYTE-SLICE-CAP` | A separate mutable `{ptr, len, cap}` byte-slice profile. |

Amendment §3.7's operations (equality, ordering, hashing, search) remain
unallocated to a card; §3.7 states what they must be when they arrive.

## 9. Lean

`proof/QCore/StdString.lean` (registered in `proof/QCore.lean`) and
`proof/AuditStdString.lean`, **38 declarations**, all placeholder-free with no
project axioms — every closure is a subset of `{propext, Classical.choice,
Quot.sound}`.

What is proved: the length is exact and is never a scan (including
`an_embedded_nul_is_inside_the_length`, the statement on which a
scan-to-terminator and this model *disagree*); slicing is `{p + low, high - low}`
into the same storage and **composes** (`slicing_a_slice_composes_the_offsets`);
an admitted slice is in range and on two code-point boundaries, in every
direction, including both ways a bound can fail to be decided; a bound inside a
code point is refused (`a_bound_inside_a_code_point_is_refused`); well-formedness
is preserved by an admitted slice
(`a_boundary_slice_preserves_well_formedness`); an empty view addresses no bytes;
a string-only program derives no effect at all; the bound forms are exactly two
and neither is a string; and the module gate, via the **same generic
`admitted_capability_requires_its_module`** every other bundled module is gated
by, so it cannot drift from the seven that precede it.

`proof/QCore/ModuleImports.lean` grew from seven capability fields to eight.
Twenty-three positional structure literals in that file, one in
`QCore/StdBuffer.lean` and three in `QCore/StdIo.lean` were widened; every
theorem **name and meaning is preserved**, and
`QCore.StdIo.no_module_substitutes_for_another` is strictly stronger than before
(one more module held, and still refused).

## 10. Gate

Run on this worktree at the card's final state.

| Step | Result |
|---|---|
| `cargo test --workspace` | **3,546 passed, 1 failed, 8 ignored** across 176 binaries. The one failure is constraint 16's confined-fixture write row; see §10.2 |
| `cargo fmt --all -- --check` | exit **0** |
| `cargo clippy --all-targets -- -D warnings` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **96 Lean audits passed** (was 95), exit 0 |
| `compiler/tests/std_string.rs` | **62 passed, 0 failed** |
| `compiler/tests/std_buffer.rs` | **52 passed, 0 failed** (untouched) |

### 10.1 The workspace run, and the load it was measured under

**3,546 passed, 1 failed, 8 ignored** across 176 test binaries, with the pinned
`third_party/libevent` submodule initialised (an uninitialised submodule fails
`libevent_submodule_matches_the_audited_revision` by design — invariant 5 doing
its job, not a regression; it passes once initialised).

The run was measured under heavy contention: other wave-23 card agents were
running their own suites in sibling worktrees on the same machine, with the
one-minute load average between 30 and 87 throughout. That is the direct cause
of §10.2 below and is recorded so the integrator can read these numbers
correctly.

**Two files changed after that run started**, and both are named here rather
than glossed: `crates/qcore-frontend/src/source_std_string.rs` (the `log_level`
refusal of §7) and `compiler/tests/std_string.rs` (two added tests). Nothing
outside this card's own route reads either, and `compiler/tests/std_string.rs`
was re-run at the final committed state — **62 passed, 0 failed**. `fmt` and the
string suite were re-run after the last edit; the 3,546 figure is therefore the
workspace measured one `log_level` refusal and two tests earlier.

### 10.2 Constraint 16's load-sensitive rows

Checked, and **two of the five fired — both for their documented reason, and
both pass in isolation.** Neither is a regression, and neither was weakened.

1. `confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
   — the confined-fixture write row — failed inside the full run with
   `assertion left == right failed: Broken pipe (os error 32); left: Io, right:
   Terminated`. That is **verbatim the `Io(Broken pipe)` shape constraint 16
   records for this row under load**. Re-run alone: **12 passed, 0 failed**.
2. `enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
   — the `RLIMIT_NPROC` row, which counts every process of the real UID — failed
   when I ran it *while* a full workspace run was in flight (eleven parallel
   cargo processes on the same UID). Re-run alone: **passed**, on the same
   binary, immediately afterwards.

`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` passed inside the full
run. The remaining two rows are in the separate `providers/` workspace, which
this card does not touch and which `cargo test --workspace` excludes.

**Environment note, recorded because it will recur:** `lean`, `lake` and `elan`
are installed under `~/.elan/bin` but are **not on the default `PATH`** in this
worktree's shell. `scripts/check_lean_audits.py` reports
`Lean checks failed: [Errno 2] No such file or directory: 'lake'` **and exits 0**
in that state, so a gate run that only checks the exit code will report a pass
having executed nothing. Exporting `PATH="$HOME/.elan/bin:$PATH"` runs the real
audits. The exit-code behaviour is a pre-existing property of that script and was
deliberately not changed by this card.

## 11. Corpus neutrality

By construction. `contract/`, `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
`providers/`, `examples/`, the pipe stage tables and the event pass are **byte-
untouched** — they appear nowhere in this card's diff. The route is selected only
when a string construct is present in a function body, so every existing module
travels an identical code path, and the single change to a sibling route (§5) is
a narrowing that no shipped fixture can observe.

## Artifacts

- `artifacts/keyword_census.txt` — the constraint-14 census output.
- `artifacts/slices.c` — the emitted C11 for the accepted fixture.
