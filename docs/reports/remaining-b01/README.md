# B01 — Revision-bound baseline reproduction — 2026-09-07

Reproduction of the standing validation baseline at `6c2782f`
(*Record halted-agent handoff and reconcile remaining v0.3 requirements*),
checked out clean on branch `claude/qcore-execution-audit-plan-24520f`.
A docs-only commit from a concurrent sibling card landed on the branch mid-run;
it changes no code and does not affect these results (finding F4).

Every reported count reproduced exactly: **819 workspace tests, 45 provider
tests, 43 Python tests, 20 Lean audits**, and the canonical CMetrics audit's
**1 checked function contract / 6 Unknown** with a **provisional Unsafe** raw
unit. No gate was weakened and no source or test file was edited. The machine
-readable record is `run_record.json`; trimmed logs (last 100 lines each) are
under `logs/`.

## Result summary

| Suite | Command | Expected | Actual | Exit | Duration |
|---|---|---|---|---|---|
| Submodule | `git -c protocol.file.allow=always submodule update --init --recursive` | pinned rev | `d82464a2` (`release-2.2.2-alpha-11`) | 0 | — |
| Workspace tests | `cargo test --workspace` | 819 | **819 passed / 0 failed** | 0 | 69.3 s |
| Format | `cargo fmt --all -- --check` | clean | clean | 0 | 0.6 s |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | clean | 0 | 8.1 s |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | exit 0, all cases `pass: true` | 0 | 0.1 s |
| Followup | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | exit 0, all cases `pass: true` | 0 | 0.5 s |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | 45 | **45 passed / 0 failed** | 0 | 18.4 s |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | clean | 0 | 0.7 s |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | clean | 0 | 4.6 s |
| Python | `python3 -B -m unittest discover -s scripts/tests -v` | 43 | **43 ran, OK** | 0 | 0.8 s |
| Lean build | `(cd proof && lake build)` | clean | 62 jobs built | 0 | 5.4 s |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | 20 | **20 audits passed** | 0 | 4.1 s |
| Provider packages | `python3 -B providers/package.py build --output build/cycle19/provider-packages-unit` | 5 packages | 5 packages | 0 | 20.7 s |
| CMetrics prepare | `scripts/prepare_cmetrics_clang_audit.py` (see below) | build + database | succeeded | 0 | 30.2 s |
| CMetrics audit | `cargo run --quiet -p qcore-audit -- …/cmetrics-audit.json.config` | 1 checked / 6 Unknown | **1 checked / 6 Unknown** | 0 | 1.8 s |
| 185-unit audit | `cycle19_fluent_bit_libraries_audit` | — | rejected at pin gate (fail-closed) | 1 | 0.2 s |

Durations exclude cargo build time where the binary was already compiled.

**No deviation from the reported 819 / 45 / 43 / 20 baseline.**

## Toolchain

| Component | Version |
|---|---|
| rustc | 1.98.0 (88d9e12ae 2026-08-18) |
| cargo | 1.98.0 (797e8a9bc 2026-08-05) |
| clang | Ubuntu clang version 21.1.8 (6ubuntu1) |
| cmake | /usr/bin/cmake |
| python | 3.14.4 |
| lean | pinned `leanprover/lean4:v4.33.1` |

## CMetrics canonical audit

Existing prepared directories were inspected before anything was run. The
persistent build managed by `scripts/prepare_cmetrics_clang_audit.py` lives in
the **main checkout** at `/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/`;
this worktree had none. Rather than overwrite the existing tree, the script was
run against a **fresh output** in the session scratchpad:

```sh
python3 -B scripts/prepare_cmetrics_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-source \
  --output <scratchpad>/cycle19
cargo run --quiet -p qcore-audit -- \
  <scratchpad>/cycle19/fluent-bit-clang-production/cmetrics-audit.json.config
```

`--fluent-bit` selects the existing pinned, clean, alternates-free clone because
the sibling upstream checkout is off-pin (see findings).

Order matters. In a fresh worktree the audit must be preceded by the provider
package build, or it rejects fail-closed with
`audit rejected: source-contract package: No such file or directory (os error 2)`
— `contract/profiles/cmetrics-counter/source-contracts.json` binds the
`qcore.cmetrics` package at `build/cycle19/provider-packages-unit/`. This was
observed on the first attempt and resolved by running
`providers/package.py build` first, exactly as the `w2-clang-cmake-build`
reproduction section prescribes. The freshly prepared
build reproduces the recorded pinned source hash for `cmt_counter.c` exactly:

- `source_sha256 = e507481702ceea626ea71e1043b7534ae98b7c2f5d71366ee916c47e25ce88f0`
  — identical to the value in the main checkout's `build-provenance.json`.

The `compile_commands.json`, `CMakeCache.txt` and `libcmetrics.a` hashes differ
from the main checkout's, as expected: those files embed absolute source/build
root paths, and the report for `w2-clang-cmake-build` already states that debug
object and archive bytes are not claimed reproducible across directory names.

Audit outcome, matching the reported baseline exactly:

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=36
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

The raw unit stays **provisional Unsafe** and unpromoted.

The audit config referenced by the card, `contract/profiles/cmetrics-counter/audit.json`,
does exist, but its relative paths resolve to `<repo>/build/cycle19/…`, which is
absent in a fresh worktree. The generated `cmetrics-audit.json.config` written by
the prepare script is the correct input for a non-default output, exactly as the
`w2-clang-cmake-build` reproduction section documents.

## Pinned 185-unit artifacts

**Not rerun.** The four serialized artifacts are archived under
`docs/reports/w2-clang-cmake-build/` and compared by `audit-parity.json`. All
four were verified by hash against that record — **all match**:

| Artifact | Uncompressed bytes | sha256 (prefix) | Matches recorded parity |
|---|---|---|---|
| `source.json.gz` | 1,663,331 | `baef422337aed1b3…` | yes |
| `diagnostics.json.gz` | 11,006,128 | `6628bf3bcef7c093…` | yes |
| `chunk-contracts.json.gz` | 4,141,225 | `df92980655aa3662…` | yes |
| `source-effects.json.gz` | 114,434 | `4a09358a8d51b605…` | yes |

`audit-parity.json` records hashes of the **uncompressed** JSON, not of the
`.gz` archives; the archive hashes differ and that is not a discrepancy.

A rerun was attempted and correctly refused (see finding F3). Auditing from the
pinned clone instead would need a full Fluent Bit CMake build for all 185
translation units plus a forced-Libevent Monkey build, which exceeds this
card's time budget.

## Findings

F1–F3 are **environment preconditions** and F4 is a **process/concurrency
observation**. None is a source verdict or a code defect, and none was worked
around by weakening a gate.

### F1 — `git submodule update --init --recursive` fails as documented

The bare command from the standing gate fails on this host:

```
fatal: transport 'file' not allowed
fatal: clone of '/home/cosmo/Documents/GitHub/libevent' into submodule path '…/third_party/libevent' failed
Failed to clone 'third_party/libevent' a second time, aborting
```

`.gitmodules` declares the upstream HTTPS URL, but the effective
`submodule.third_party/libevent.url` is a local path, and git blocks the `file`
transport by default (CVE-2022-39253 hardening). Re-running with a
per-invocation `-c protocol.file.allow=always` succeeded and checked out the
pinned `d82464a277d0f42703702c4dfd9af6af38595a83`. No persistent git config was
changed. Environment/config issue.

### F2 — Lean toolchain is not on the default `PATH`

`lake`, `lean` and `elan` are not on `PATH`; they live in `~/.elan/bin`. Both
Lean gates require `PATH="$HOME/.elan/bin:$PATH"`. With that prefix, `lake build`
and `check_lean_audits.py` both pass. Environment issue.

### F3 — Upstream Fluent Bit checkout is off-pin and dirty

`/home/cosmo/Documents/GitHub/fluent-bit` is at
`d411765eaa40a6ca590ffbcef07a61ce0094650d` with modified tracked files
(`plugins/out_s3/*`), while the profile pin is
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`. Two consequences:

1. `scripts/prepare_cmetrics_clang_audit.py` with its default `--fluent-bit`
   would reject at the pin check. (The default also resolves to
   `<worktree-parent>/fluent-bit`, which does not exist for a git worktree.)
2. The 185-unit audit **rejects fail-closed**, which is the correct behavior:

```
Fluent Bit library audit failed: Fluent Bit pin check failed:
pin `fluent-bit`: expected 66910c10a4d7eafa810b84229cf2dcf7b0f26f97,
found d411765eaa40a6ca590ffbcef07a61ce0094650d
```

The pinned commit object is still present in that repository, and an
independent clean clone at the pin exists at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-source`,
which is what the CMetrics reproduction used.

### F4 — Branch advanced mid-run by a concurrent sibling card

`HEAD` was `6c2782f` with a clean tracked tree when this run started. At
14:15:11 +0900, partway through the run, commit
`6072947685b2563ceb687e64349f52b068fa9f5c` (*Record B00 checkpoint
reconciliation ledger*) appeared on the same branch. **B01 did not create it**;
it comes from a concurrent sibling card sharing this worktree.

It is docs-only:

```
docs/reports/remaining-b00/README.md   | 255 ++++++++++++
docs/reports/remaining-b00/ledger.json | 721 +++++++++++++++++++++++++++++++++
2 files changed, 976 insertions(+)
```

`git diff --name-only 6c2782f 6072947` yields nothing outside `docs/`. No code,
test, contract, profile or build-input path is touched, and both files are new,
so every suite result recorded here still reflects the `6c2782f` source tree.
`run_record.json` records `revision` as `6c2782f` and carries the interloping
commit under `concurrent_commit` and `head_at_end_of_run`.

Two operational notes: concurrent cards sharing one worktree can race, and any
future comparison against this record should pin `6c2782f` rather than branch
tip.

## Scope and non-claims

This card reproduces a **validation baseline**; it establishes nothing new about
the checker. Specifically:

- No raw Cycle 19 unit is promoted. The CMetrics raw unit remains **provisional
  Unsafe**, and a provisional Unsafe is not an upstream defect claim.
- CT coverage is unchanged at 11/50 mapped.
- The four 185-unit artifacts were **hash-verified, not regenerated**; this is
  archive integrity, not a fresh parity run.
- The fresh CMetrics build reproduces the pinned **source** hash only. Object
  and archive bytes are not claimed reproducible across directory names, and no
  GCC equivalence, code-generation or whole-library safety claim is made.
- No sanitizer runs were executed under this card.

## Files written

Only these paths were created. Nothing was committed, pushed, reset or cleaned.

- `docs/reports/remaining-b01/README.md` (this file)
- `docs/reports/remaining-b01/run_record.json`
- `docs/reports/remaining-b01/logs/` (trimmed logs, last 100 lines each)
- `build/cycle19/provider-packages-unit/` — gitignored (`/build/cycle19/` is in
  `.gitignore`); required by the CMetrics audit's source-contract package binding
- Session scratchpad `cycle19/` tree — outside the repository
