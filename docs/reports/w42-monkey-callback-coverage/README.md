# W42 Monkey callback coverage accounting

Date: 2026-09-20. Baseline: `f7b010de`. Card:
`W42-MONKEY-CALLBACK-COVERAGE`.

## Result

The forced-Libevent Monkey profile selects three callbacks. Their imported
source bodies contain eight paths: four for `cb_event`, three for `cb_timeout`
and one for `cb_wait_2_timeout`. The existing expanded native replay checks all
eight paths, including the labeled injected send failure. Before this card the
source sidecar carried the source results and the optional binder carried the
eight-path replay, but the normal audit with no native binding directory emitted
no callback-specific Cycle 18 disposition.

The normal and replay-enabled routes now publish one accounting row for every
callback selected by the loaded profile. The denominator and path totals are
derived from the selected callback map and imported source summaries. Missing,
duplicate or unselected source/evidence rows reject the accounting operation;
so do an empty source revision, an empty path inventory, an empty Unknown
reason, an out-of-range checked path, native evidence attached to a
source-Unknown callback, and an execution count that cannot support the set of
distinct checked paths.

Every current row is explicitly **Unknown** for Cycle 18 composition. With no
native directory, each row says the native callback binding evidence was not
supplied. With the expanded replay, all eight paths are checked and each row
says:

> all selected source paths have checked native execution evidence, but no
> source-to-Cycle18 EventProgram projection or composed certificate exists

There is no `Composed` variant in the current Rust status vocabulary. The
reusable verifier validates accounting structure; the production adapter
supplies the profile-selected inventory, private imported summaries and optional
native replay report. Neither layer accepts a certificate-authority input.
Consequently an unsealed public report value is never described as revalidated
certificate evidence, and complete replay cannot manufacture `Safe`.

## Placement and audit output

Reusable accounting lives in
`crates/qcore-verification/src/callback_coverage.rs`, the target verification
adapter crate. The Monkey compiler module only maps the sealed source results,
profile assumptions and optional binder report into that adapter. It also uses
the profile-derived callback set when checking binder completeness; the former
literal three-name membership test and literal count are gone.

The pinned compatibility audit's Monkey source sidecar advances from schema 2
to schema 3 and adds `callback_coverage`. The field is written whether or not
the optional native binding directory was supplied. It retains the source
profile assumption map, including profile digest and source pin; the containing
sidecar retains the exact compile entry, command, file and source revision.
Raw source summaries and wrappers remain present.

`QCore.CallbackCoverage` models the implemented route rather than a future one:
every current evidence row is Unknown. Its audited theorems state that result
and the exact-membership rejection of omitted, duplicate and unselected rows.
The module is registered in both `QCore.lean` and `AuditCycle19.lean`.

## Measured acceptance and limitations

Focused tests measure:

- profile-derived source accounting without native evidence: **3/3 callbacks,
  8 selected paths, 0 checked paths, 3 callback-specific Unknown rows**;
- expanded replay accounting: **3/3 callbacks, 8 selected paths, 8 checked
  paths, 3 callback-specific Unknown rows**;
- five generic verifier tests covering no-evidence, complete replay,
  source-Unknown, partial evidence and malformed accounting families;
- nine Monkey binder tests, including the normal no-directory accounting and
  the expanded eight-path evidence.

This card does not derive a Cycle 18 `EventProgram`, bind the native observation
stream to an event-program trace, authenticate observer provenance, prove
allocation provenance, close wrapper/map custody, or promote any callback or
translation unit. `MONKEY-CALLBACK-CYCLE18-COMPOSITION` remains open with this
discharge condition: derive an event program from the selected source and bind
the same execution evidence to a successful Cycle 18 composition recheck. A
new certificate status would require a Rust implementation and matching Lean;
the present all-Unknown enum is not a reserved optimistic branch.

## Validation

All logs are private, run-scoped files under
`measure/w42-monkey-gate-20260920-escalated/`. Debug information was disabled
for Rust dev/test profiles and build concurrency was one after relocating this
worktree's `target` to the home-disk private build directory.

The first sandboxed workspace run is preserved separately at
`measure/w42-monkey-gate-20260920/cargo-test.log`. It was interrupted after 89
result summaries when ten binaries reported LeakSanitizer's explicit
`does not work under ptrace` fatal error; this is not hidden as a source failure.
The exact gate was restarted with escalation and leak detection unchanged.

The completed standing gate is:

| Command | Exit | Result | Private log |
| --- | ---: | --- | --- |
| `git submodule update --init --recursive` | 0 | Libevent at `d82464a277d0f42703702c4dfd9af6af38595a83` | command output retained by the task; `git submodule status --recursive` rechecked the pin |
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1 cargo test --workspace --no-fail-fast` | 0 | **4,851 passed, 0 failed, 10 ignored; 249 result summaries** | `measure/w42-monkey-gate-20260920-escalated/cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/w42-monkey-gate-20260920-escalated/fmt.log` |
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1 cargo clippy --all-targets -- -D warnings` | 0 | warnings denied, clean | `measure/w42-monkey-gate-20260920-escalated/clippy.log` |
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1 cargo run -p qcore-compiler --example reevaluation_probe` | 0 | **8/8 expectations pass** | `measure/w42-monkey-gate-20260920-escalated/probe.log` |
| `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=1 cargo run -p qcore-compiler --example reevaluation_followup` | 0 | **11/11 expectations pass** | `measure/w42-monkey-gate-20260920-escalated/followup.log` |
| `LAKE_JOBS=1 python3 scripts/check_lean_audits.py` | 0 | **148/148 audits pass** | `measure/w42-monkey-gate-20260920-escalated/lean-audits.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | **305/305 pass** | `measure/w42-monkey-gate-20260920-escalated/python-tests.log` |
| `clang --version` | 0 | `Ubuntu clang version 21.1.8 (6ubuntu1)` | `measure/w42-monkey-gate-20260920-escalated/clang-version.txt` |

The successful Rust workspace run used escalation so LeakSanitizer ran outside
the sandbox ptrace restriction; no sanitizer option was disabled. Its measured
revision before the report-only close was `af258e53`.

The standard gate summary is serialized in `gate.json`. Exact commands,
environment, exits, counts, source identity and private log paths are in
`gate-facts.json`; `clang-version.txt` is the actual output captured by this
worktree's successful gate.

## Integrator reconciliation list

1. **Acceptance matrix:** register
   `MONKEY-CALLBACK-CYCLE18-COMPOSITION` as an open residual. Ground: the
   selected callback/path denominator and native replay are now total, but no
   source-to-`EventProgram` projection or composed certificate exists.
2. **Current status:** no Safe/Unsafe/Unknown corpus count changes are proposed.
   This card changes a separate callback accounting sidecar and keeps all three
   selected callbacks explicitly Unknown.
3. **Current increment:** record **3/3 selected callbacks accounted, 8/8 source
   paths checked in the optional expanded replay, 0/3 Cycle 18 composed, 3/3
   explicit Unknown**. Also record that the ordinary no-binding audit emits
   3/3 callback-specific Unknown rows with 0/8 checked paths.
4. **Integration evidence pending:** rerun the ordinary production audit after
   merge and confirm schema-3 `callback_coverage` is present without a binding
   directory. The isolated production run was stopped during the shared tmpfs
   pressure event before it reached the Monkey sidecar writer; integration owns
   the required combined AFTER and artifact parity check.

## Integration confirmation — 2026-09-20

Merged through `9233f8b0b50f07bc4ac5367b1859fbdd4cf621a6`; combined source `f3f99df5d2dcdf27e9b73ecd81dd7fe126419ca4`
passed **4,863 Rust / 149 Lean / 313 Python**, zero failures,
with fmt, Clippy, both probes and the isolated host checks green. The
[integration gate](integration-gate.json) retains the combined measurement;
its colocated Clang record is explicitly borrowed from this card, while the
[combined report](../w42-priority-integration/README.md) carries the actual
combined recording, pinned AFTER, exact attribution and remaining limits.
The isolated checkpoint above remains unchanged.
