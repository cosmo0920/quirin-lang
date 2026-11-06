# GATE-SCOPE-LIB-UNIT-TEST-MODULES: a library's cfg(test) code selects its unit-test executable

Round 23, card A (2026-11-06). The follow-up GATE-SCOPE-PRECISE-SELECTION
registered under the gate-time plan
([R2, R9](../../plans/gate-time-reduction/README.md)), refilled under the
user's standing three-card rule after the user's request (verbatim,
2026-11-05): "Launch three cards of gate-time reductions." Base `13337d0af`.
Tooling, tests and the walker table only: no analyzer, contract or Lean change,
so no prediction and no declared measurement. The rule text is in
[validation.md](../../agents/validation.md#scoped-card-gate).

## 1. What changed

- **cfg(test) items.** `scripts/gate_scope.py` tokenizes each Rust source
  (strings, raw strings, characters, lifetimes, nested comments) and finds the
  span of every item under a test-only outer `#[cfg]`: `test`,
  `all(.., test, ..)`, or `any` of test-only predicates (`cfg_test_spans`). The
  span runs from the first attribute of the item's attribute run to its `;` or
  closing `}`; `const`/`static`/`use`/`type`/`let` end at their `;`.
- **Module closure with test edges.** Each target's closure now records which
  files it reaches through ordinary edges and which only through a `mod` or
  `#[path]` declared inside such a span. A file a library reaches only through
  test edges is compiled into the library's unit-test executable alone
  (`PKG::lib::NAME`): dependents and doc-tests build the library without
  `cfg(test)`. It selects that unit, with no reverse dependency. This includes a
  `src/` module (`#[cfg(test)] mod test_support;`, `src/task_execution/tests.rs`).
- **Reads inside cfg(test) items.** An `include_str!`/`include_bytes!`/`include!`,
  a `#[path]` or a string literal inside a `#[cfg(test)]` item claims for the
  unit-test executables of the targets holding the file: the library's unit,
  plus any integration test that compiles the same file. The literal dedup is
  now keyed by the units too, so a test-only literal never hides the same
  literal in ordinary code.
- **Fail-closed.** What the scanner cannot classify keeps the old, crate-wide
  selection: an inner `#![cfg(test)]` (adding one removes a module from
  dependents' build, and the scoper reads only the new tree); any other
  predicate (`any(windows, test)`, `not(test)`); an attribute on a match arm, a
  field, a parameter or a block (only an item keyword or `let` opens a span); an
  item whose end is not found; `cfg(test)` code of a binary, an example or a
  build script; a library with `test = false`; and every file of a crate whose
  library shows `include!` of Rust code, a `mod $name` in a macro or
  `cfg_attr(.., path = ..)` (`Model.opaque`; no crate of this tree is opaque).
  A file also reached by an ordinary edge stays the crate's.
- **`#[path]` with other attributes.** `PATH_ATTR_RE` now accepts other outer
  attributes between `#[path]` and `mod`. Two modules were outside every
  closure before: `qcore-verification/tests/unit/cycle18_callback_obligations.rs`
  (`#[cfg_attr(windows, ..)]` in between; it was crate-wide, now the
  verification library's unit) and `qcore-contract/examples/support/premise_flow_fixture.rs`,
  which `qcore-c-gateway/tests/premise_flow_dependencies.rs` compiles
  (`#[allow(dead_code)]` in between; the gateway test was not attributed to it).
- **Runner.** `run_test_binaries.py --targets` accepts `PKG::lib::NAME`, the
  library's unit-test executable exactly as the build names it, and builds and
  captures it with cargo's `--lib` filter beside that member's `--test`
  filters. It refuses a wrong library name or kind, a binary's unit, a library
  with `test = false`, and a member `--packages` already runs whole, before
  building. The fail-closed checks (named and not launched, launched and not
  named) are unchanged. The scoper lints a library unit with
  `cargo clippy -p PKG --lib --profile test` (plain `--lib` skips cfg(test)
  code; checked on a scratch crate: an unused variable in a `#[cfg(test)]`
  module passes `--lib` and fails `--lib --profile test`).
- **Walker table.** Nine `census:` base-path entries widen
  (`scripts/gate_scope_walkers.json`; [scripts/widen_walkers.py](scripts/widen_walkers.py),
  section 3).

178 tracked files now select only a library unit (qcore-c-gateway 69,
qcore-core 51, quirin-source-analysis 20, qcore-compiler 9, qcore-frontend 7,
the provider crates 10, others 12), and 382 compile-time and 130 run-time
readers claim for a library unit instead of a crate.

## 2. Tests

[`scripts/tests/test_gate_scope_lib_units.py`](../../../scripts/tests/test_gate_scope_lib_units.py),
33 tests: the span finder (predicates in and out, inner attributes, braces in
strings, raw strings, characters and comments, semicolon items, `const fn` and
generic items, attributes on arms, fields, parameters and blocks, unterminated
items, nesting); the selection (a `#[path]` module, its children, a `src/`
test module, includes and literals in cfg(test) items, a Lean file read by a
test item, a literal also in ordinary code, a file also reached plainly, an
integration test sharing the file, the clippy and suite steps); every
fail-closed case above; the runner's `PKG::lib::NAME` parsing, filters and
refusals (two before building, on this tree); and this tree's tests/unit
modules. `test_gate_scope_precise.py`'s runner tests now expect the
`(kind, name)` pairs and refuse `a::lib::wrong`, `b::bin::b`, `a::rlib::a`,
`a::lib::x` and a library member run whole instead of `a::lib::a`.

## 3. The census caught nine entries the narrowing exposed

`test_gate_scope_census.py` (every observed open selects its unit) failed with
647 dropped reads after the narrowing, all under `contract/profiles/`. The
reads were real; their attribution had been an accident.
qcore-verification's library tests walk `contract/profiles` inside
`#[cfg(test)]` items (`src/cycle18_composition.rs:1210`,
`src/monkey_wait_projection.rs:746`, `tests/unit/cycle18_callback_obligations.rs:20`).
Read crate-wide, those walkers also claimed every test of qcore-verification
and of its reverse dependencies. That covered the qcore-compiler and qcorec
tests, which read every profile through `first_party_registry()`. That function
lists `quirin_source_analysis::profiles_root()` (`compiler/src/first_party.rs`),
a path no literal in qcore-compiler shows. The census draft had given those sites `reads: []` ("a
base path ... already attribute"), because the draft charged a site only with
its own crate's units. `scripts/widen_walkers.py` starts from the committed
table and widens by the draft's rule, but charges a site in ordinary library
code with its reverse dependencies' units too. It widened nine entries, each
to its whole directory (no source spells `.json`):
`quirin-source-analysis/src/source_profiles.rs` `contract/profiles` (14
dropped reads), six qcore-verification integration tests' `contract/profiles`,
and two `contract/profiles/fluent-bit`. Nothing was unexplained, and the
census test passes.

## 4. Probes (this tree)

[probes.json](probes.json), from [scripts/probes.py](scripts/probes.py) with
`--old-rev 13337d0af` (the old scoper and its table):

| Path | Old | New |
|---|---|---|
| `crates/qcore-c-gateway/tests/libc_gap_declarations.rs` | scoped: 9 test targets | scoped: the same 9 |
| `providers/quirin-provider-libc/contracts/libc/stdlib.qv` | scoped: 6 packages, 16 targets | scoped: 5 packages, 48 targets (qcore-verification is no longer whole: its library unit and the integration tests that read the profiles) |
| `proof/QCore/WrittenSetDerivation.lean` | scoped: 3 targets, Lean 1 module, 1 audit | the same |
| `compiler/tests/contract_loader.rs` | scoped: 3 targets | the same |
| `crates/qcore-contract/tests/unit/contract_observation.rs` | **full** (22 packages; reaches qcore-contract) | **scoped**: `qcore-contract::lib::qcore_contract` and 8 tree walkers' test targets |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | **full** | **scoped**: the gateway's library unit and 8 walkers |
| `crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qv` | **full** | **scoped**: 2 packages (qcore-compiler, qcorec: their libraries read it), 79 targets with the gateway's library unit |
| `crates/qcore-core/tests/unit/usage.rs` | **full** | **scoped**: the core library unit and 8 walkers |
| `proof/QCore/NullDeref.lean` | **full** (the gateway's `src/null_premise.rs` test module includes it) | **scoped**: 4 targets with the gateway's library unit, Lean 13 modules, 13 audits |

The first four are the plan page's section 1 probes; they were already scoped
at `13337d0af` and keep their selection. The last row is the R4 shape
(`include_str!` of a Lean module from a gateway `#[cfg(test)]` module): it no
longer selects full mode.

## 5. R9 replay

[replay.json](replay.json), from [scripts/replay.py](scripts/replay.py)
(`--old-rev 13337d0af`). The cases are GATE-SCOPE-PRECISE-SELECTION's, which
already hold LOG-PRINT-BODY-CONTRACTS and LIBRDKAFKA-HEADER-INLINES, plus
CONTRACT-COUNT-PINS-FROM-LOCKS (branch tip `7b5524c78`, read through `git`
only). Its red runs are from its README: the first scratch-declaration gate
(THROWAWAY `410d2e552` on card tip `105f46326`) and the four hand-edited
`quirin-provider-libc/contract-inventory.lock` runs at `f336ef8ba`, replayed
as that one path with the binaries each run's `rust.log` shows failing. The
extracted trees now carry their JSON files, so both scopers see the manifests.
That is why old counts differ from the previous report (CFL-MPACK 199 there,
269 here). "Units" counts the test executables the selection runs (no
doc-test unit).

| Card | Commit | Kind | Paths | Old | New | Failing units at the red commit, all run and attributed by new |
|---|---|---|---:|---|---|---|
| LIBC-GAP-DECLARATIONS | `aa5e4e6f8` | tip | 20 | full 602 | **scoped 264** | |
| | `fbca407e2` | red (recorded) | 16 | full 602 | **scoped 264** | `contract_loader` |
| | `b019deaea` | red (inferred) | 15 | full 602 | **scoped 264** | `contract_loader` |
| CELL-PROTOCOL-RELEASE-WINDOW-FRAMES | `8a36914e2` | tip | 42 | full 602 | full 602 | |
| | `fa1c10b43` | red (inferred) | 38 | full 602 | full 602 | `contract_dependencies` |
| CFL-MPACK-EXTERNAL-DECLARATIONS | `17e27141c` | tip | 22 | scoped 269 | scoped 254 | |
| | `58a95463d` | red (inferred) | 18 | scoped 269 | scoped 254 | `contract_loader`, `cfl_mpack_external_declarations` |
| W45-AUDIT-SUBCOMMAND-PLUGINS | `52b35ae8a` | tip | 95 | full 317 | full 317 | |
| | `d9df130a2` | red (recorded) | 91 | full 317 | full 317 | `workspace_layering`, `cycle20_verifier_plugins`; provider lock check in both |
| MEMBER-ACCESS-PROJECTION | `c077c4101` | tip | 28 | full 607 | full 607 | no red commit recorded |
| LIBRDKAFKA-HEADER-INLINES | `a58debf9b` | tip | 22 | scoped 267 | scoped 252 | |
| | `68c8065e1` | red (inferred) | 16 | scoped 267 | scoped 252 | `provider_selection`, `neighbours`, `prod_contracts` |
| | `bf29932d3` | red (inferred) | 22 | scoped 267 | scoped 252 | `provider_selection` |
| LOG-PRINT-BODY-CONTRACTS | `5e44bfa5a` | tip | 27 | full 604 | full 604 | |
| | `f04c3075d` | red (inferred) | 20 | full 604 | full 604 | `contract_loader`, `provider_selection`, `neighbours`, `prod_contracts` |
| CONTRACT-COUNT-PINS-FROM-LOCKS | `7b5524c78` | tip | 53 | full 610 | full 610 | |
| | `410d2e552` | red (recorded) | 3 | scoped 269 | scoped 254 | `contract_loader` |
| | `83aab9fc9` | scratch, green | 3 | scoped 269 | scoped 254 | |
| | lock: declaration count + 1 | red (recorded) | 1 | scoped 267 | scoped 252 | `qcore_contract_corpus` |
| | lock: function mode loses a parameter | red (recorded) | 1 | scoped 267 | scoped 252 | `contract_loader`, `qcore_contract_corpus` |
| | lock: resource view 69 -> 68 | red (recorded) | 1 | scoped 267 | scoped 252 | `contract_loader`, `provider_selection`, `neighbours`, `prod_contracts`, `qcore_contract_corpus` |
| | lock: retaining 115 -> 114 | red (recorded) | 1 | scoped 267 | scoped 252 | the same five |

No failing unit is dropped (`dropped failing units: []`, exit 0), and every
failing unit is attributed by the new plan, not run because of full mode. No
row adds a unit. LIBC-GAP, which the previous replay kept full because its
`.qv` files are included by qcore-contract's library tests, now runs
`qcore-contract::lib::qcore_contract` instead of the crate and its
dependents: 264 of 602 units. Every scoped row drops the same 15
qcore-verification integration tests. qcore-verification used to be selected
whole only through the cfg(test) walkers of section 3. Its integration tests
that read the profiles stay selected, by their own walker entries.
The full rows stay full on a real crate-level reach: CELL-PROTOCOL,
MEMBER-ACCESS and LOG-PRINT change gateway `src/` (LOG-PRINT:
`src/callback_member_closure.rs`, not its `tests/unit` includes as the
previous report supposed), W45 the workspace manifests and the runner, and
CONTRACT-COUNT-PINS's tip an example of qcore-contract
(`examples/contract_inventory.rs`).

## 6. Compile-time reads against rustc's dep-info

The read census observes run-time opens only. For compile-time reads,
[scripts/depinfo_check.py](scripts/depinfo_check.py) reads rustc's own
record: the dep-info file of each library build in the gate's
`build.stdout.jsonl`. Each file lists every source and `include_*!` file
rustc read, once for the library as dependents link it and once for its
unit-test executable. [depinfo.json](depinfo.json): all 21 root libraries; 793
tracked files the ordinary libraries read, every one selecting its crate whole
(or full mode, for the root `Cargo.toml` rustc lists); 298 files only the
unit-test builds read, every one selecting the library's unit, 286 of them as
that unit alone. The other 12 stay crate-wide for another reader (a JSON
manifest listing them, a binary, an example, a crate's own `Cargo.toml`). Exit 0:
no ordinary-library read is narrowed, and no test-only read is unattributed.
The same check over the clippy step's check-mode artifacts gave the same
result before the gate ran.

## 7. Gate

`CARGO_BUILD_JOBS=6 python3 scripts/gate_scope.py --base 13337d0af --out <worktree>/private/gate`
on `3ae556489`, a clean tree ([gate.json](gate.json)). The sections 6-8 text,
depinfo.json, gate.json, the amendments and the dep-info script's artifact
lookup were added after it. The mode was full by design: the diff changes the
scoper and the runner. There was no Lean, provider or provider-lock step:
nothing under `proof/`, `providers/` or a manifest changed.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1, none moved |
| fmt, Clippy | 0, 0 | |
| Rust suite | 0 | 612 units, 653 summaries; 9,243 passed, 0 failed, 16 ignored, 135 filtered. Slot wait 36,326 s (10.1 h) behind an integrator measurement holding `audit/0` and then another card's audit census holding `audit/3`. Build 108 s, capture 8 s, run 1,366 s |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Python suite | 0 | 658 tests, 0 failures, run with `QUIRIN_CARD_GATE_BASE=13337d0af…` |

Logs: `<worktree>/private/gate/` (`rust-suite/summary.json`, `slots/rust-suite.json`).
Console log outside the worktree:
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/r23a/gate-console.log`.
The card's own tests (`test_gate_scope_lib_units`, `test_gate_scope_precise`,
`test_gate_scope`, `test_gate_scope_census`, `test_run_test_binaries`) pass
inside the Python suite.

## 8. Follow-ups

No new id. Recorded on existing entries through the amendments:

- GATE-READ-CENSUS-REFRESH (append): a re-draft of the walker table must charge
  a library site with its reverse dependencies' units (section 3), or it
  rebuilds base paths that rest on accidental attribution.
- GATEWAY-TESTS-CROSS-TREE-INCLUDES (amend): includes from the gateway's
  `#[cfg(test)]` modules no longer force full mode; what remains crate-wide is
  that entry's.

Seen, not changed: a binary's `#[cfg(test)]` code (`PKG::bin::NAME`) still
selects its crate whole (fail-closed; no runner selection for it); `mod` and
`#[path]` inside inline modules resolve against the file, not the inline
module path, as before (a module missed that way stays crate-wide).
