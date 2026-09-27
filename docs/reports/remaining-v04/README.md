# V04 — R05 adversarial probe matrix: unconditional declaration checks

**Date:** 2026-09-07. **Worktree HEAD:** `536dc01`. **Card:** V04 (`verify_or_repair`).
**Raw log:** [`probe_run.log`](probe_run.log). **Probe sources:** [`probes/`](probes/).

Report-only card. No tracked file was modified, no semantic patch was written, no
verdict was promoted and no gate was weakened. Raw Cycle 19 units remain
**unpromoted**; full source-language conformance remains false.

## Scope

Revision-0.3 requirement **R05** ("Axis declarations can bypass dimension checking
when unused", `docs/reviews/r0_3/supplied/REVIEW.md:91`) states:

> **Required rule:** declaration well-formedness must not depend on later type usage.
> Build and validate dimension/axis declarations unconditionally, then resolve each
> type against that validated environment.
>
> **Acceptance:** reject unknown dimensions, duplicate axes and invalid declarations
> both with and without a quantitative type. Preserve valid nominal `Count<T>`
> behavior according to the existing dimension rules; do not conflate a nominal
> count with a requirement for a runtime allocation type.

B02 (`docs/reports/remaining-b02/README.md:56`) classified R05 **verified-fixed** on
one library test invocation and one live probe. This card re-derives that
classification from a 27-source adversarial matrix run end-to-end through the real
CLI (`./target/debug/qcorec check`), not through the library API.

## Implementation under probe

`crates/qcore-frontend/src/frontend.rs:1645-1671` (`elaborate_source`) is the R05
repair site. It is explicitly unconditional:

```rust
// Declaration validity is independent of whether a quantitative type uses it.
```

It slices every `DeclarationKind::Dimension | DeclarationKind::Axis` declaration out
of `program.source`, joins the slices with `\n`, reparses that reconstructed source,
and runs `crate::check::StaticEnvironment::check` over it. A failure is pushed as
`Q016` / `SemanticPass::Index` with `span: Span { start: 0, end: 0 }`, and
`index_environment` stays `StaticEnvironment::default()` (empty), so every later
quantitative type also fails to resolve. That is fail-closed.

`crates/qcore-frontend/src/check.rs:32-60` is the validator. It collects dimension
names first (so forward references are legal by construction), then checks axes.
Only `DimensionRef::Named` is required to exist; `DimensionRef::Count(_)`
(`crates/qcore-core/src/ast.rs:35-38`) is nominal and carries no dimension
obligation — this is the "do not conflate" clause of the acceptance, and it is
implemented as a structural distinction, not as a special case.

Three `CheckError` variants exist: `DuplicateDimension`, `DuplicateAxis`,
`UnknownDimension`. `check` returns on the **first** error.

## Probe matrix

All rows were run as `./target/debug/qcorec check <probe>`; sources under
`probes/`, verbatim output in `probe_run.log`. "±use" is the R05 axis: `nouse`
sources contain **no** quantitative type at all (the exact condition the pre-repair
code used to skip validation); `use` sources append
`quant axis slots : credit<Count<Slot>>; type T = quant<credit, { slots = n } where n := 1>;`.
Every source ends with `fn main() -> i32 { return 0; }` unless it declares a module
mode, so that accepted rows reach exit 0 rather than `Q401` (no entry point).

| # | Probe | Source (essential part) | Expected | Actual diagnostic (code @ reported span) | Exit | Class |
|---|---|---|---|---|---|---|
| 1 | `P01-unknown-dim-nouse` | `quant axis bad : credit<Missing>;` — **no quant type** | reject | `Q016 @ 1:1` unknown dimension `Missing` | 1 | pass (R05 core) |
| 2 | `P02-unknown-dim-use` | same + quant type | reject | `Q016 @ 1:1` unknown dimension `Missing`; `Q016 @ 1:84` unknown quant axis `slots` | 1 | pass |
| 3 | `P03-dup-axis-nouse` | `dimension Bytes …; quant axis x …; quant axis x …;` | reject | `Q010 @ 1:64` duplicate declaration `x`; `Q016 @ 1:1` duplicate quant axis `x` | 1 | pass |
| 4 | `P04-dup-axis-use` | same + quant type | reject | `Q010 @ 1:64`; `Q016 @ 1:1` duplicate quant axis `x`; `Q016 @ 1:143` unknown quant axis `slots` | 1 | pass |
| 5 | `P05-nominal-count-nouse` | `quant axis objects : live<Count<UndeclaredNominal>>;` | **accept** | `checked 1 function(s); entry main` | 0 | pass (positive control) |
| 6 | `P06-nominal-count-use` | same + quant type | **accept** | `checked 1 function(s); entry main` | 0 | pass (positive control) |
| 7 | `P07-valid-named-nouse` | `dimension Bytes : Idx + Ordered; quant axis memory : credit<Bytes>;` | **accept** | `checked 1 function(s)` | 0 | pass (positive control) |
| 8 | `P08-valid-named-use` | same + `extern fn f(q: quant<n where n := 5>) -> unit;` | **accept** | `checked 1 function(s)` | 0 | pass (positive control) |
| 9 | `P09-forward-dim-ref` | axis **before** its `dimension` declaration | **accept** | `checked 1 function(s)` | 0 | pass (declared forward-reference rule) |
| 10 | `P10-decl-after-use-site` | `extern fn f(q: quant<…>) …;` **then** dimension + axis | **accept** | `checked 1 function(s)` | 0 | pass (declaration after use site) |
| 11 | `P11-dup-dimension-nouse` | `dimension Bytes …; dimension Bytes …;` | reject | `Q010 @ 1:34` duplicate declaration `Bytes`; `Q016 @ 1:1` duplicate dimension `Bytes` | 1 | pass |
| 12 | `P12-dup-dimension-use` | same + quant type | reject | `Q010 @ 1:34`; `Q016 @ 1:1` duplicate dimension `Bytes`; `Q016 @ 1:116` unknown quant axis `slots` | 1 | pass |
| 13 | `P13-axis-self-reference` | `quant axis bad : credit<bad>;` | reject | `Q016 @ 1:1` unknown dimension `bad` | 1 | pass (adversarial) |
| 14 | `P14-axis-refs-other-axis` | `quant axis other : credit<memory>;` where `memory` is an **axis** | reject | `Q016 @ 1:1` unknown dimension `memory` | 1 | pass (adversarial; axis names do not leak into the dimension namespace) |
| 15 | `P15-count-of-unknown` | `quant axis bad : credit<Count<Missing>>;` | **accept** | `checked 1 function(s)` | 0 | pass (nominal count is not a dimension obligation) |
| 16 | `P16-unknown-mechanism` | `quant axis bad : missing<Count<T>>;` | reject | `Q016 @ 1:1` `expected axis mode at bytes 17..24` | 1 | pass (reject), see finding F2 |
| 17 | `P17-cost-profile-unknown` | `quant axis alloc : cost.total<Missing>;` | reject | `Q016 @ 1:1` unknown dimension `Missing` | 1 | pass (validation is mechanism-independent) |
| 18 | `P18-library-mode-unused` | `module library; quant axis bad : credit<Missing>;` — no functions at all | reject | `Q016 @ 1:1` unknown dimension `Missing` | 1 | pass (the review's "otherwise-unused module") |
| 19 | `P19-contract-mode-unused` | `module contract; quant axis bad : credit<Missing>;` | reject | `Q016 @ 1:1` unknown dimension `Missing` | 1 | pass |
| 20 | `P20-two-bad-axes` | `quant axis a : credit<M1>; quant axis b : credit<M2>;` | reject | `Q016 @ 1:1` unknown dimension `M1` **only** | 1 | pass (reject), see finding F3 |
| 21 | `P21-span-line4` | bad axis on **line 4** of a 5-line file | reject, span on line 4 | `Q016 @ 1:1`, caret under `dimension Bytes : Idx + Ordered;` (line 1) | 1 | reject correct, **span wrong** (F1) |
| 22 | `P22-span-line4-dupaxis` | duplicate axis on **line 4** | reject, span on line 4 | `Q010 @ 4:1` (correct); `Q016 @ 1:1` (wrong) | 1 | reject correct, **span split** (F1) |
| 23 | `P23-parse-error-later-line` | bad axis mode on line 3 after a blank-free preamble | reject | `Q016 @ 1:1` `expected axis mode at bytes 83..90` | 1 | pass (offsets coincidentally match here) |
| 24 | `P24-no-axis-declarations` | `fn main() -> i32 { return 0; }` only | **accept** | `checked 1 function(s)` | 0 | pass (empty reconstructed source does not spuriously fail) |
| 25 | `P28-byte-offset-divergence` | bad axis mode at original byte 84, preceded by a blank line and a function | reject | `Q016 @ 1:1` `expected axis mode at bytes 50..57` | 1 | reject correct, **offsets are synthetic** (F2) |
| 26 | `P25`+`P26` multi-file | dimension in file A, `credit<Missing>` axis in file **B** | reject, blamed on B | `Q016` attributed to **`P25-multifile-a.qcore:1:1`** | 1 | reject correct, **wrong file** (F1) |
| 27 | `P26`+`P25` reversed | same pair, reversed argument order | reject | `Q016` attributed to `P26-multifile-b-badaxis.qcore:1:1` | 1 | attribution follows argument order, not the offender (F1) |
| 28 | `P25`+`P27` multi-file | dimension in A, axis referencing it in B | **accept** | `checked Library module: 2 declaration(s)` | 0 | pass (cross-file environment is one program; no false rejection) |

Rows 1–20 discharge the R05 acceptance list in full: unknown dimensions, duplicate
axes, duplicate dimensions and invalid axis modes all reject **both with and without
a quantitative type**, and every valid nominal / forward-reference / declaration-
after-use control is accepted. Row 18 is the review's own scenario — a module that
contains the offending axis and *nothing that could use it* — and it rejects.

## Existing test coverage per row

| Rows | Existing test | Location |
|---|---|---|
| 1–6, 9, 11–12, 16 | `review_r05_checks_axes_even_without_quantitative_types` (loops the same prefixes over an empty suffix and a quant-type suffix) | `crates/qcore-frontend/tests/unit/frontend.rs:335-373` |
| 1, 13, 14, 17 | `rejects_unknown_named_dimension` | `crates/qcore-frontend/tests/unit/check.rs:21-28` |
| 5, 6, 7, 15 | `validates_named_and_count_dimensions` | `crates/qcore-frontend/tests/unit/check.rs:6-19` |
| 9, 10 | `allows_forward_dimension_references` | `crates/qcore-frontend/tests/unit/check.rs:30-38` |
| 8, 10 | `shorthand_and_vector_where_bindings_elaborate_to_dimensioned_values`, `bindings_are_type_local_and_natural_operators_are_checked` | `compiler/tests/quant_index_contracts.rs:13-45`, `:111-140` |
| 2, 4, 12 (`Q016` on the type side) | `rejects_false_cyclic_runtime_and_overflowing_index_bindings`, `where_binders_cannot_shadow_visible_runtime_locals` | `compiler/tests/quant_index_contracts.rs:47-72`, `:148-159` |

Executed here: `cargo test -p qcore-frontend --lib review_` → 3 passed, 0 failed;
`cargo test -p qcore-frontend --lib check::` → 3 passed, 0 failed.

**Test-level gaps (behaviour is correct; only the unit coverage is indirect):**

- `check.rs`'s own unit tests cover `UnknownDimension` and forward references but
  have **no** direct case for `CheckError::DuplicateDimension` or
  `CheckError::DuplicateAxis`. Rows 3/4/11/12 are covered only transitively through
  the frontend R05 test, which asserts `is_err()` without pinning the code.
- Rows 13, 14, 17, 18, 19, 20, 24 and the multi-file rows 26–28 have **no** existing
  test at any level. They pass, but nothing pins them against regression.

## Findings

None of the findings below is an R05 acceptance failure: every one of them occurs on
a source that is **correctly rejected with exit 1**. They are diagnostic-attribution
defects on the rejecting path, so they cannot flip an `Unknown` to `Safe` and cannot
weaken a gate. They are recorded, not repaired (this card may not modify tracked
files).

### F1 — the unconditional check reports a degenerate span (`0..0`)

`frontend.rs:1668` emits `span: Span { start: 0, end: 0 }` for every declaration
error found by the unconditional pass. Consequences, all reproduced above:

- **Row 21 (minimal reproducer):** the offending axis is on line 4; the diagnostic
  is rendered at `1:1` with line 1's text under the caret.
- **Row 22:** the same source produces a correctly located `Q010 @ 4:1` from the
  name-resolution pre-pass and a mislocated `Q016 @ 1:1` from the R05 pass — so the
  two passes disagree about where the same duplicate lives.
- **Rows 26/27:** in a multi-file build, offset 0 lands in whichever file was listed
  first, so a bad axis in file B is blamed on file A. Reversing the argument order
  moves the blame without changing the source.

Minimal reproducer (`probes/P21-span-line4.qcore`):

```qcore
dimension Bytes : Idx + Ordered;
quant axis good : credit<Bytes>;

quant axis bad : credit<Missing>;
fn main() -> i32 { return 0; }
```

```
$ ./target/debug/qcorec check probes/P21-span-line4.qr
probes/P21-span-line4.qcore:1:1: Q016: unknown dimension `Missing`
dimension Bytes : Idx + Ordered;
^
exit 1
```

Expected location: line 4. The information needed to fix this exists — the failing
`CheckError` names the offending dimension/axis, and each contributing declaration's
original `Span` is in hand at the slice site — but the current code discards it.

### F2 — parse-failure messages carry offsets into a synthetic source

When the reconstructed axis source fails to *parse* (rather than fails
`StaticEnvironment::check`), the parser's message is forwarded verbatim, including
byte offsets that refer to the joined slice buffer, not to the user's file.

Row 25 (`probes/P28-byte-offset-divergence.qcore`): the token `missing` is at
original byte **84**; the diagnostic says `at bytes 50..57`, which is its position in
the reconstructed source. Row 23 only *appears* correct because that file happens to
have the same layout as the reconstruction (one declaration per line, no gaps).

### F3 — first-error-only reporting

`StaticEnvironment::check` returns on the first `CheckError`, so row 20's second bad
axis (`credit<M2>`) is never mentioned. This is fail-closed and consistent with the
validator's `Result<Self, CheckError>` signature; it is noted so the row-20 output is
not mistaken for `M2` being accepted.

## Conclusion

**R05 is fully covered — closing, no patch.**

The unconditional-validation requirement holds under adversarial probing. Across 28
rows: 9 acceptance-required rejections fire identically with and without a
quantitative type; 8 positive controls (valid named dimension, nominal `Count<T>`,
`Count<Unknown>`, forward reference, declaration after use site, no declarations at
all, cross-file split declaration) are accepted; and the review's own
"unused module" scenario rejects in executable, library and contract modes. The
B02 `verified-fixed` classification is confirmed on a much wider matrix than the
single probe it cited.

Three diagnostic-attribution defects (F1–F3) were found on the rejecting path. They
are **not** R05 failures — no source is wrongly accepted, no gate is weakened, no
verdict changes — but F1 in particular is a real defect: it mislocates within a file
and misattributes across files. It is recorded here for a future card rather than
patched, per this card's report-only constraint.

Remaining boundary, unchanged: the closed index/profile limits recorded in
`docs/reviews/r0_3/acceptance.json` for R05 still apply. `quant<cost,…>` remains a
future total/peak profile and `quant<access,…>` remains skipped by the closed-index
parser; neither is source-level verification.
