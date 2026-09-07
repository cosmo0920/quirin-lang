# V03 — R04: admitted source signatures, results and exits (adversarial probe matrix)

**Date:** 2026-09-07. **Worktree HEAD at write time:** `ea2af0f`
(`536dc01` plus two report-only commits; no implementation commit landed
during this card). **Card type:** verify_or_repair, executed as report-only.
No tracked file outside this directory was modified, no patch was written, no
verdict was promoted and no pinned expectation was edited.

**Machine-readable record:** [`probe_results.tsv`](probe_results.tsv)
(`name<TAB>exit<TAB>first diagnostic line`). Every probe source is copied
verbatim into [`probes/`](probes).

## 1. Why this card exists

[`docs/reports/remaining-b02/README.md`](../remaining-b02/README.md) classified
R04 (*"the source driver does not enforce ordinary return-type compatibility"*)
**verified-fixed** for the admitted profile, but flagged a residual:

> Two R04 acceptance rows — inconsistent function exits, and call/result
> signature mismatch — reject at `Q500` behind the Cycle 12 executable-subset
> gate rather than by a result-type check.

This card tests that flag adversarially with live CLI probes rather than
inspection. **The flag is narrower than B02 recorded**: both rows *are*
independently exercised by precise typed diagnostics, and the `Q500`-only
residue is four specific syntactic forms, named in §5.

## 2. Method

```sh
cargo build -p qcorec
./target/debug/qcorec check --entry main PROBE.qcore     # all probes except S26
./target/debug/qcorec check S26_library_mode_unknown_types.qcore   # library mode
```

71 single-file probes, each written to be minimal. Every probe was run twice:
once against the pre-existing `target/debug/qcorec`, once after an explicit
`cargo build -p qcorec`. The two runs are byte-identical, so no result below
depends on a stale binary.

Exit codes observed across all 71 probes: **12 × exit 0, 59 × exit 1, 0 crashes**
(no exit 101, no panic message, no timeout). Acceptance criterion 3 holds at the
process level: no unsupported form crashed the driver.

### The two admitted source profiles

`qcorec check` routes a module to one of two checkers, and the probe table is
grouped accordingly.

| Profile | Selected when | Result types | Parameter types |
|---|---|---|---|
| Sequential Cycle-12 executable subset (`SequentialCleanupV1`) | default | `i32`, `unit` only (`crates/qcore-frontend/src/frontend.rs:149-155`) | structurally classified, **not resolved** (§6) |
| Scoped tasks (`ScopedOwnedTasksV1` / `ScopedCopyTasksV1`) | source mentions `qcore_buffer_new`/`qcore_buffer_read`, a task expression, or a `thread scope` (`crates/qcore-frontend/src/source_tasks.rs:13-27`) | `i32`, `unit`, `own<Buffer>` (`source_tasks.rs:103-118,264`) | `i32` or `own<Buffer> use one`, checked (`source_tasks.rs:230-251`) |

Probe fixtures for the task profile are modelled on the CT harness fixtures
(`compiler/tests/fixtures/ct/CT01/case.qcore`,
`compiler/tests/fixtures/ct/CT37/case.qcore`), per
`compiler/tests/ct_corpus.rs` and `docs/reevaluation/ct_manifest.json`.

## 3. Probe table — sequential executable profile

`expected` is the behaviour this card required before running; `classification`
records how the rejection is guarded.

| # | Source (verbatim) | Expected | Exit | Actual diagnostic | Classification |
|---|---|---|---|---|---|
| S01 | `fn main() -> bool { return 2; }` | reject | 1 | `Q020` executable result profile supports only i32 and unit | precise typed-result |
| S02 | `fn main() -> u32 { return 2; }` | reject | 1 | `Q020` same | precise typed-result |
| S03 | `fn main() -> Widget { return 0; }` | reject | 1 | `Q020` same | precise typed-result |
| S04 | `fn main() -> own<Buffer> { return 0; }` | reject | 1 | `Q020` same | precise typed-result (owned results are not admitted in this profile) |
| S05 | `fn main() -> own<Item> { return 0; }` | reject | 1 | `Q020` same | precise typed-result |
| S06 | `fn main() -> own<own<Buffer>> { return 0; }` | reject | 1 | `Q020` same | precise typed-result |
| S07 | `fn main() -> i32 { return 2; }` | **accept** | 0 | `checked 1 function(s); entry main` | positive control |
| S08 | `fn main() -> unit { return; }` | **accept** | 0 | `checked 1 function(s); entry main` | positive control |
| S09 | `fn main() -> i32 { }` | reject | 1 | `Q202: MissingReturn` | precise exit check (lowering pass, **not** `Q500`) |
| S10 | `fn leaf() -> unit { return; }` + `fn main() -> i32 { let x: i32 = leaf(); return 0; }` | reject | 1 | `Q500` ``let`` must use `qcore_acquire(...)` | **Q500-gated only** (row Q500-1, §5) |
| S11 | `fn leaf(x: i32) -> unit { return; }` + `fn main() -> i32 { leaf(); return 0; }` | reject | 1 | `Q020` executable source calls must have zero arguments | precise signature check |
| S12 | `fn main(f: fn() -> i32) -> i32 { return 0; }` | reject | 1 | `Q020` the emitted entry function must not have parameters | precise signature check |
| S13 | `fn helper(x: Widget) -> i32 { return 0; }` + `fn main() -> i32 { return 0; }` | reject | **0** | accepted | **boundary B-1** (§6): unresolved parameter type on an uncallable function |
| S14 | `fn main() -> unit { return 0; }` | reject | 1 | `Q020` a unit result requires a bare return | precise exit/value check |
| S15 | `fn main() -> i32 { return; }` | reject | 1 | `Q020` an i32 result requires a signed 32-bit integer literal return | precise exit/value check |
| S16 | `fn main() { return 0; }` | reject | 1 | `Q000` function requires `->` return type | parse gate |
| S17 | `fn helper(x: Widget) -> i32 { drop x; return 0; }` + main | reject | 1 | `Q103` unknown resource binding `x` | ownership gate (using an unresolved-typed parameter fails) |
| S18 | `fn helper(x: own<Buffer> use one) -> i32 { return 0; }` + main | reject | 1 | `Q100` variable `x` requires usage [1,1], but has demand [0,0] | usage gate |
| S19 | `fn helper() -> Widget { return 0; }` + `fn main() -> i32 { return 0; }` | reject | 1 | `Q020` executable result profile … | precise typed-result — result types are checked on **every** function, called or not |
| S20 | `fn helper() -> i32 { }` + `fn main() -> i32 { helper(); return 0; }` | reject | 1 | `Q202: MissingReturn` | precise exit check on a non-entry function |
| S21 | `fn main() -> i32 { if (true) { return 1; } }` | reject | 1 | `Q500` structured control flow is not in the Cycle 12 executable subset | **Q500-gated only** (row Q500-2, §5) — this is a *branching* gate, not an exit-consistency gate |
| S22 | `error E { Failed; }` + `fn main() -> i32 { let x: Widget = qcore_acquire(1, true, E); drop x; return 0; }` | reject | **0** | accepted | **boundary B-2** (§6): unresolved local type annotation |
| S23 | `error E { Failed; }` + `fn main() -> i32 { let x = qcore_acquire(1, true, E); drop x; return 0; }` | **accept** | 0 | `checked 1 function(s)` | positive control (documented grammar) |
| S24 | S22 without the `drop` | reject | 1 | `Q202: LiveResourcesAtNormalReturn([1])` | resource obligations survive the ignored annotation (evidence for §6) |
| S25 | `error E { Failed; }` + `fn main() -> i32 { let x: i32 = qcore_acquire(1, true, E); drop x; return 0; }` | reject | **0** | accepted | **boundary B-2** (§6): scalar annotation on a resource binding |
| S26 | `module library;` + `fn helper(x: Widget) -> Widget { return 0; }` | reject | 1 | `Q020` executable result profile … | precise typed-result also in library mode |
| S27 | `error E { Failed; }` + `fn main() -> i32 { let x = qcore_acquire(1, true, E); move x; return 0; }` | accept (documented) | 0 | accepted | documented `CustodyPayloadProfile::SeparateIdentityLedger` (`crates/qcore-verification/src/driver.rs:83-88`) |
| S28 | same with `-> unit` / bare `return;` | accept (documented) | 0 | accepted | same |
| S29 | `fn main() -> i32 { let x = qcore_acquire(1, true, Missing); drop x; return 0; }` | reject | 1 | `Q013` unknown name `Missing` | name resolution |
| S30 | `fn main() -> i32 { missing(); return 0; }` | reject | 1 | `Q013` unknown name `missing` | name resolution |
| S31 | `fn main() -> i32 { return 0; return; }` | reject | 1 | `Q020` an i32 result requires a signed 32-bit integer literal return | precise exit check |
| S32 | `fn leaf() -> unit { return; }` + `fn main() -> i32 { return leaf(); }` | reject | 1 | `Q020` an i32 result requires a signed 32-bit integer literal return | **precise call/result mismatch check** (see §5) |
| S33 | `fn leaf() -> bool { return 1; }` + `fn main() -> i32 { leaf(); return 0; }` | reject | 1 | `Q020` executable result profile … | precise typed-result on the callee |
| S34 | `fn main() -> result<i32, Failure> { return 0; }` | reject | 1 | `Q020` executable result profile … | precise typed-result |
| S35 | `error Failure { Failed; }` + `fn leaf() -> i32 { let r = qcore_acquire(8, true, Failure); move r; return 0; }` + `fn main() -> unit { leaf(); return; }` | reject | 1 | `Q200` whole-function summary exceeds its declared resource bounds | resource-bound gate (callee payload cannot escape unencoded) |

## 4. Probe table — scoped-task profile and exotic result forms

| # | Source shape | Expected | Exit | Actual diagnostic | Classification |
|---|---|---|---|---|---|
| T01 | CT01-shaped `own<Buffer>` round trip | **accept** | 0 | `checked 2 function(s); … scoped owned-buffer tasks (Linux C11)` | positive control |
| T02 | CT37-shaped worker allocating and returning `own<Buffer>` | **accept** | 0 | same | positive control |
| T03 | worker declares `-> own<Buffer>`, returns `0` | reject | 1 | `Q020` return value does not match the declared task function result | precise typed-result |
| T04 | worker declares `-> i32`, returns `move buffer` | reject | 1 | `Q020` same | precise typed-result |
| T05 | worker `match` arm falls through with no `return`/`raise` | reject | 1 | `Q020` task functions require an explicit return or raise on every path | **precise inconsistent-exit check** (see §5) |
| T06 | `spawn s worker(copy value)` into an `own<Buffer>` parameter | reject | 1 | `Q020` capture type does not match worker parameter | precise call-argument check |
| T07 | `spawn s worker()` for a 1-parameter worker | reject | 1 | `Q013` spawn worker `worker` expects 1 captures, received 0 | precise arity check |
| T08 | worker `-> own<Item>` | reject | 1 | `Q020` task functions support i32, unit and own<Buffer> results | precise owned-result shape check |
| T09 | worker parameter `own<Item> use one` | reject | 1 | `Q020` task parameters require i32 or own<Buffer> use one | precise parameter-shape check |
| T10 | worker `-> own<own<Buffer>>` | reject | 1 | `Q020` task functions support i32, unit and own<Buffer> results | precise nested-own rejection |
| T11 | `let returned: i32 = join move handle?;` from an `own<Buffer>` worker | reject | 1 | `Q020` local result does not match its declared scalar type | **precise call-result mismatch check** |
| T12 | worker `-> bool` | reject | 1 | `Q020` task functions support i32, unit and own<Buffer> results | precise typed-result |
| T13 | worker parameter `value: Widget` | reject | 1 | `Q020` task parameters require i32 or own<Buffer> use one | precise unresolved-parameter rejection |
| T14 | task entry `fn main(value: i32)` | reject | 1 | `Q020` task execution entry must have no parameters | precise signature check |
| T15 | entry `-> unit` returning a value | reject | 1 | `Q020` return value does not match the declared task function result | precise exit/value check |
| T16 | entry `-> unit` with bare `return;` | **accept** | 0 | `checked 1 function(s); … scoped owned-buffer tasks` | positive control |
| T17 | `let buffer: Widget = qcore_buffer_new(42)?;` | reject | 1 | `Q500` unsupported task local type | **Q500-gated only** (row Q500-3, §5) |
| T18 | `let buffer: i32 = qcore_buffer_new(42)?;` | reject | 1 | `Q020` local result does not match its declared scalar type | precise local-annotation check |
| T19 | `spawn s worker(move buffer)` into an `i32` parameter | reject | 1 | `Q020` capture type does not match worker parameter | precise call-argument check |
| T20 | `spawn s worker(copy buffer)` into `own<Buffer> use one` | reject | 1 | `Q500` only copy i32 and move own<Buffer> captures are executable | **Q500-gated only** (row Q500-4, §5) |
| T21 | `spawn s missing(move buffer)` | reject | 1 | `Q013` unknown name `missing` (+ `Q013` must resolve to a first-order source function) | name resolution |
| X01 | `fn main() -> (i32, i32)` | reject | 1 | `Q020` executable result profile … | precise typed-result |
| X02 | `fn main() -> [i32; 4]` | reject | 1 | `Q000` function declaration requires a body | parse gate (the `;` closes the declaration) — explicit, no crash |
| X03 | `fn main() -> &[i32]` | reject | 1 | `Q020` … | precise typed-result |
| X04 | `fn main() -> *i32` | reject | 1 | `Q020` … | precise typed-result |
| X05 | `fn main() -> !` | reject | 1 | `Q020` … | precise typed-result |
| X06 | `fn main() -> &Buffer` | reject | 1 | `Q020` … | precise typed-result |
| X07 | `fn main() -> Count<i32>` | reject | 1 | `Q020` … | precise typed-result |
| X08 | `fn main() -> fn() -> i32` | reject | 1 | `Q020` … | precise typed-result |
| X09 | `fn main() -> task<i32>` | reject | 1 | `Q020` … | precise typed-result |
| X10 | `fn main() -> mutex<i32>` | reject | 1 | `Q020` … | precise typed-result |
| X11 | `fn main() -> own<>` | reject | 1 | `Q020` … | precise typed-result |
| X12 | `fn main() -> own` | reject | 1 | `Q020` … | precise typed-result |
| X13 | `fn main() -> own<own<own<Buffer>>>` | reject | 1 | `Q020` … | precise typed-result |
| X14 | `fn helper(f: fn() -> i32) -> i32 { return 0; }` + `fn main() -> i32 { return 0; }` | reject | **0** | accepted | **boundary B-1** (§6): function-typed parameter on an uncallable function |
| X15 | `fn main() ->  { return 0; }` | reject | 1 | `Q000` function requires a nonempty balanced result type | parse gate |

## 5. Resolution of B02's flag

B02 recorded two R04 acceptance rows as reachable only through the `Q500`
executable-subset gate. Both rows have precise, independently exercised checks:

| R04 acceptance row | Precise check found | Probes |
|---|---|---|
| **inconsistent function exits** | `Q202: MissingReturn` in the sequential profile (entry and non-entry); `Q020` *task functions require an explicit return or raise on every path* in the task profile; `Q020` on a second, differently-shaped `return` | S09, S20, S31, **T05** |
| **call/result signature mismatch** | `Q020` on a call in return position; `Q020` on the callee's own declared result; `Q020` capture/parameter type mismatch; `Q013` capture arity; `Q020` on a mismatched joined-result annotation | **S32**, S33, S11, **T06**, **T19**, **T07**, **T11** |

`Q500` is therefore not the only guard for either row. The **remaining
`Q500`-only rows** — recorded as a named boundary of the admitted profile, not
as a defect — are exactly four syntactic forms, all fail-closed:

| Row | Probe | Diagnostic | Why the typed check is not reached |
|---|---|---|---|
| Q500-1 | S10 | ``let`` must use `qcore_acquire(...)` in the executable subset | in the sequential subset `let` is reserved for acquisition, so a `let`-bound call is rejected before any result typing runs |
| Q500-2 | S21 | structured control flow is not in the Cycle 12 executable subset | branching itself is unsupported; this is a control-flow gate, and exit *consistency* is separately checked (S09/S20/T05) |
| Q500-3 | T17 | unsupported task local type | an unresolved local type name in the task profile is rejected as an unsupported form rather than as a type mismatch (its *resolved-but-wrong* sibling is `Q020`, T18) |
| Q500-4 | T20 | only copy i32 and move own<Buffer> captures are executable | a wrong *capture mode* on a correctly typed argument is a subset gate; a wrong capture *type* is `Q020` (T06/T19) |

## 6. Named boundary: unvalidated parameter and local type annotations in the sequential subset

Three probes were accepted where this card expected a rejection (S13, X14 →
**B-1**; S22, S25 → **B-2**). Both are reported as **named boundaries of the
admitted profile, not as demonstrated defects**, because no false verdict
follows from either; the evidence for that is in the table itself.

**B-1 — parameter type names are not resolved in the sequential subset.**
`fn helper(x: Widget) -> i32` and `fn helper(f: fn() -> i32) -> i32` are
accepted when `helper` is otherwise well-formed. Parameter types are
*structurally classified*, never name-resolved
(`crates/qcore-frontend/src/frontend.rs:114-130,999`), and only `Borrow`/`Owned`
kinds reach the ownership model
(`crates/qcore-verification/src/driver.rs:751-758`). The accepted function is
nevertheless unreachable in this profile: calls must be zero-argument (S11,
`Q020`) and the emitted entry must have no parameters (S12, `Q020`), so no call
can supply a value of the unresolved type; and any *use* of such a parameter
fails (S17 `Q103`, S18 `Q100`). The scoped-task profile rejects the same
declarations outright (T09, T13 — `Q020`).

**B-2 — local type annotations are accepted and not interpreted in the
sequential subset.** `let x: Widget = qcore_acquire(...)` (S22) and
`let x: i32 = qcore_acquire(...)` (S25) are both accepted, with the annotation
neither resolved nor compared against the acquired resource. Two facts bound
this:

* The resource obligations are unaffected — deleting the `drop` from S22 is
  still rejected with `Q202: LiveResourcesAtNormalReturn([1])` (S24). The
  annotation is inert, so it cannot license a leak, a wrong exit or a wrong
  result.
* The task profile checks the same position: an unresolved annotation is `Q500`
  (T17) and a resolved-but-wrong annotation is `Q020` (T18).

The reason to record B-2 explicitly is a documented asymmetry, not a soundness
claim. `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md:49` gives the sequential
acquisition form as `"let", IDENT, "=", "qcore_acquire", ...` — with **no
annotation slot** — and line 61 states that local `use` annotations "are
explicitly rejected here rather than ignored", which the driver implements
(`crates/qcore-verification/src/driver.rs:510-522`). A local *type* annotation
is the one declared-syntax position in this subset that is accepted and
silently discarded. Against acceptance criterion 3 ("unsupported types remain
explicit rejections"), this is the single position where the sequential subset
does not hold: it holds unconditionally for every **result** type (S01–S06,
S19, S26, S33, S34, X01–X13) and for task parameters/locals/results (T03–T15,
T18).

Neither B-1 nor B-2 is a demonstrated false `Safe`, a crash or an escaped
obligation, so neither triggers this card's stop-and-report rule. Repairing
them is not attempted here (report-only card) and would be a frontend change,
not a V-card repair.

## 7. Existing tests that already pin rows in this table

Run and passing at this checkout:

* `compiler/tests/unit/driver.rs:317` `review_r04_checks_declared_results_before_lowering`
  pins S01, S03-equivalent (`-> Missing`), S05-equivalent (`-> own<File>`),
  S34, S15, S14, S09, S31, S33, S32, and the S07/S08-class positives.
  `cargo test -p qcore-compiler --lib review_r04` → 2 passed.
* `compiler/tests/unit/driver.rs:374` `review_r04_does_not_emit_unencoded_callee_payloads`
  pins S35.
* `crates/qcore-frontend/tests/unit/frontend.rs:376,415`
  `review_r04_retains_result_annotation_and_profile` and
  `review_r04_preserves_quantitative_result_comparison_syntax`.
  `cargo test -p qcore-frontend --lib review_r04` → 2 passed.

**Not pinned by any existing test:** every task-profile row in §4. A grep for
the exact task diagnostic strings (`return value does not match the declared
task function result`, `task functions require an explicit return or raise on
every path`, `capture type does not match worker parameter`, `task functions
support i32, unit and own<Buffer> results`, `task parameters require i32 or
own<Buffer> use one`, `local result does not match its declared scalar type`)
finds them only in `crates/qcore-frontend/src/source_tasks.rs`. The task tests
that exist (`compiler/tests/source_tasks.rs`,
`compiler/tests/owned_source_tasks.rs`, `compiler/tests/ct_corpus.rs`) assert
`Q600`, `Q500` and `Q013` rejections, not these `Q020` signature rows. The
probe set in [`probes/`](probes) is currently the only executed record of them.
Likewise B-1/B-2 (§6) are not pinned by any test in either direction.

## 8. Conclusion

**Classification: covered — with four named `Q500`-only rows and one named
acceptance boundary. No genuine semantic failure was demonstrated, so no
reproducer-and-stop applies and no patch was written.**

* R04's five acceptance rows (bool/integer mismatch, unknown result type,
  invalid owned-result shape, inconsistent exits, call/result signature
  mismatch) all have precise diagnostics in the currently admitted profiles.
  B02's residual flag is superseded: exits are guarded by `Q202`/`Q020` and
  call/result mismatch by `Q020`/`Q013`, not only by `Q500`.
* The `Q500`-only residue is exactly rows Q500-1 … Q500-4 (§5). Each is
  fail-closed and each names a construct the profile does not admit at all,
  rather than a type check that was skipped.
* Unsupported and exotic result types are explicit rejections in every probe
  (`Q020`, or `Q000` where the form does not parse); no probe crashed the
  driver.
* One acceptance boundary is recorded honestly rather than repaired:
  unresolved **parameter** types on uncallable functions (B-1) and
  accepted-but-uninterpreted **local type annotations** (B-2) in the sequential
  Cycle-12 subset. Both are inert in that subset and both are rejected by the
  scoped-task profile.

This card claims nothing about full language conformance, promotes no raw
Cycle 19 unit and changes no CT mapping (still 11 mapped / 39 unmapped, pinned
by `compiler/tests/ct_corpus.rs`).
