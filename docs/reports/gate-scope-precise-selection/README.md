# GATE-SCOPE-PRECISE-SELECTION: the scoped gate selects what a change reaches

Round 19, card A (2026-11-05). Requirements R1, R2, R6 and R9 of the
[gate-time plan](../../plans/gate-time-reduction/README.md); follow-ups
GATE-SCOPE-RUNTIME-READS-HEURISTIC, GATE-SCOPE-FULL-CRATES-TEST-ONLY,
GATE-SCOPE-PROVIDER-LOCK and CARD-GATE-NEW-AUDIT-MATRIX-COUNT. Base
`25508ccaf`. Tooling and tests only: no analyzer, contract or Lean change, so
no prediction and no declared measurement. The rule text is in
[validation.md](../../agents/validation.md#scoped-card-gate).

## 1. What changed

- **Test-target granularity (R2).** `scripts/gate_scope.py` now reads each
  crate's cargo targets and the module closure of each target (`mod`, `#[path]`).
  A reader compiled only into integration-test targets claims for those targets
  alone, and nothing depends on a test target, so no reverse dependency
  follows. A path a crate owns selects only test targets when it is a Rust file
  only test targets compile, or a non-Rust file whose readers in the crate name
  it (an include, a literal naming the file, a walker or a manifest) and are all
  test targets. `src/`, `Cargo.toml`, `build.rs`, and a file nothing in the
  crate names select the whole crate, as before. A FULL_CRATES member forces
  full mode only when it is reached whole.
- **Runner.** `scripts/run_test_binaries.py --targets PKG::test::NAME,...`
  builds and captures those members with cargo's own `-p PKG --test NAME`
  filters, beside `--packages` members run whole. It refuses, before building,
  a malformed id, a non-member, a name that is not an integration test, a
  member also given whole, and `--batch-size`. It fails closed if a named target
  is not built and launched, or anything outside the selection is. Clippy runs
  per crate with the same filters.
- **Run-time reads (R1).** A literal naming a tracked file claims that file.
  Two scanner gaps found by the census are closed: a test id
  `path.rs::name` names `path.rs`, and a bare literal naming a tracked
  top-level file (`cycle19-resource-contracts.json`) counts. A directory literal
  claims only what [`scripts/gate_scope_walkers.json`](../../../scripts/gate_scope_walkers.json)
  declares for that site. An undeclared one still claims its whole directory
  (fail-closed), and the census test fails until it is declared.
  Data-derived reads have two forms. Every tracked JSON file that names a
  tracked path (relative to itself or the root) lists it, and its readers read
  what it lists, transitively: the profiles, `cycle19-resource-contracts.json`,
  the load path and its contract roots. The table's `expansions` add what a
  manifest implies but does not spell: the load path's crates' locks. A walker
  entry with literal `*` attaches reads to a source that has no literal, such as
  a whole-tree `tracked_files` listing filtered by extension, or `cargo metadata`.
- **The read census.** [`scripts/gate_read_census.py`](../../../scripts/gate_read_census.py)
  builds and captures the root suite exactly as the runner does. It runs every
  test executable under `strace -ff --seccomp-bpf -y` on the path system calls,
  and reduces the traces to the repository files each unit opened, the
  directories it listed and its `git ls-files` pathspecs.
  [read-census.json](read-census.json) is that record. Probe-only paths (a
  tracked-file listing stats every path it names) are counted, not listed.
  [`scripts/tests/test_gate_scope_census.py`](../../../scripts/tests/test_gate_scope_census.py)
  holds the model to it:
  every observed open selects its unit; every walker entry is walked (a
  matching open, or a listing of its directory); every directory literal is
  declared; no entry is stale.
- **Provider lock (R6).** The scoper adds a step
  `cargo metadata --locked --offline --manifest-path providers/Cargo.toml` when
  the root `Cargo.toml`/`Cargo.lock`, or the `Cargo.toml` of a root crate in
  the provider workspace's transitive path-dependency closure, changes and the
  provider gate is not already selected. The RULE32A landing's paths
  (`c82895503..62da8997d`) select it in a test.
- **Card matrix drift (R6).** On a card gate the Python step gets
  `QUIRIN_CARD_GATE_BASE=<base>`. `test_build_matrix_sections.py` then accepts a
  committed Lean-audit list that differs from the tree by the card's own added
  or removed audits and the drift its base had, nothing else. The standing gate
  and `gate_scope.py --landing` keep the exact check.

## 2. The walker table

109 walker entries and 1 expansion (the counts are in the file):

| Kind | Entries | Meaning |
|---|---:|---|
| literal `*` | 7 | the 3 whole-tree extension censuses (`qr_lexical_selector_census`, `qr_print_route_census`, `std_socket_inet`), `rule_switch_entry_points`' manifest and entry-point listing, two `cargo metadata` tests, the w3 fixture crate build |
| base path `[]` | 27 | no unit compiling the site opens a file under it that something else does not attribute; 1 hand-written: `pin_window.rs`'s NOTE prose |
| extension-filtered | 1 | `bounds_contract.rs`: `providers/**/*.qv` |
| whole directory `DIR/**` | 74 | its units list the directory (a `read_dir` or `git ls-files` covering it), or open files under it with no filter the source spells, or the site is provider-workspace code the census does not trace |

The census-backed entries (`why` starting `census:`) were drafted by
[scripts/draft_walkers.py](scripts/draft_walkers.py) and then reviewed. Each
site starts at its narrowest declaration and widens only on evidence: a
listing, a drop the rebuilt model makes, or provider code. It repeats until
nothing observed is dropped. The draft could not explain seven reads, all
fixed by hand or by the JSON scan: the w3 fixture crate, and
`guarded_decomposition`'s and `production_symbolic_adoption`'s reads named by
docs JSON. Every test entry is held by the census test. A `DIR/**` entry claims
what the old scanner claimed, but only for the test targets of the site.

## 3. Census

Recorded at `a9de4e8be` (the base's Rust tree; the commit differs from
`25508ccaf` in scripts only), all 610 root units, `--jobs 6`, under the
`rust-suite` slot: waited 604 s, ran 11:07–11:21 UTC (receipt
`private/r19a/census-slot.json` in the worktree). 9,825 opens of 2,817
distinct tracked files.

45 units exited nonzero under the tracer. 43 logs show LeakSanitizer's
"fatal error" under ptrace. The other two failed in sanitized native runs
(`detach_lookup_membership`: LSan cannot read `/proc/<pid>/task`;
`ct_corpus`: a sanitized case's assertion). libtest ran every other test of
those units, so their reads are kept. A failure can only shorten a read set;
the sanitized runs read generated C under `/tmp`, outside the repository.

Seen while recording, not fixed (follow-up): a qcore-audit test leaves
`crates/qcore-audit/private/audit-retained-products/` in the checkout.

## 4. Probes of the plan page's section 1 (this tree)

[probes.json](probes.json), from [scripts/probes.py](scripts/probes.py):

| Path | Old | New |
|---|---|---|
| `crates/qcore-c-gateway/tests/libc_gap_declarations.rs` | full, 22 packages (reaches qcore-c-gateway) | **scoped**: 0 packages, 8 test targets (`libc_gap_declarations`, `workspace_layering`, `composition_bounds`, `rule_switch_entry_points`, `ct_corpus`, measure's `host`/`measure`/`units`) |
| `providers/quirin-provider-libc/contracts/libc/stdlib.qv` | full (qcore-c-gateway, qcore-contract) | **scoped**: 6 packages (qcore-audit, qcore-compiler, qcore-verification, qcorec, measure, quirin-source-analysis: libraries that read the load path or the profiles naming it), 16 test targets (11 gateway tests that include or load it, 5 qcore-contract tests that walk providers/) |
| `proof/QCore/WrittenSetDerivation.lean` | full (reaches qcore-core) | **scoped**: 3 test targets (`qcore-core::test::written_set`, `float_ieee_vectors`, `proof_leg_digests`), Lean: 1 module, 1 audit |
| `compiler/tests/contract_loader.rs` | scoped: qcore-compiler, qcorec | scoped: 3 test targets (`contract_loader`, `proof_leg_digests`, `rule_switch_entry_points`) |

R1's acceptance: the `.qv` probe selects neither qcore-c-gateway nor
qcore-contract as a crate. It selects their test targets that read it
(`include_str!`, the load path, or a declared `.qv` walk). R2's acceptance: the
first probe is scoped. The Lean probe is scoped too: `written_set`'s
`include_str!` is a test target's read.

## 5. R9 replay

[replay.json](replay.json), from [scripts/replay.py](scripts/replay.py). For
each card the tree at each commit is extracted, and both scopers plan
`git diff base...commit` over it: the base's `gate_scope.py`, and this one with
this table. "Units" counts the test executables the selection runs, without
the doc-test unit; full mode runs all of them. Red commits and their failing
units come from gate records (recorded) or from the next commit, which edits
only that test's pins or literals (inferred).

| Card | Commit | Kind | Paths | Old | New | Failing units at the red commit, all run by the new selection |
|---|---|---|---:|---|---|---|
| LIBC-GAP-DECLARATIONS | `aa5e4e6f8` | tip | 20 | full 602 | full 602 | |
| | `fbca407e2` | red (recorded) | 16 | full 602 | full 602 | `qcore-compiler::test::contract_loader` |
| | `b019deaea` | red (inferred) | 15 | full 602 | full 602 | `qcore-compiler::test::contract_loader` |
| CELL-PROTOCOL-RELEASE-WINDOW-FRAMES | `8a36914e2` | tip | 42 | full 602 | full 602 | |
| | `fa1c10b43` | red (inferred) | 38 | full 602 | full 602 | `qcore-c-gateway::test::contract_dependencies` |
| CFL-MPACK-EXTERNAL-DECLARATIONS | `17e27141c` | tip | 22 | full 602 | **scoped 199** (3 packages + 8 targets) | |
| | `58a95463d` | red (inferred) | 18 | full 602 | **scoped 199** | `contract_loader`, `qcore-c-gateway::test::cfl_mpack_external_declarations` |
| W45-AUDIT-SUBCOMMAND-PLUGINS | `52b35ae8a` | tip | 95 | full 317 | full 317 | |
| | `d9df130a2` | red (recorded) | 91 | full 317 | full 317, **+ provider lock check** | `workspace_layering`, `cycle20_verifier_plugins`; the provider steps failed on the stale lock: old selects no provider step, new the lock check |
| MEMBER-ACCESS-PROJECTION | `c077c4101` | tip | 28 | full 607 | full 607 | no red commit recorded |
| LIBRDKAFKA-HEADER-INLINES (extra) | `a58debf9b` | tip | 22 | full 604 | **scoped 205** (3 + 14) | |
| | `68c8065e1` | red (inferred) | 16 | full 604 | **scoped 205** | `provider_selection`, `neighbours`, `prod_contracts` |
| | `bf29932d3` | red (inferred) | 22 | full 604 | **scoped 205** | `provider_selection` |
| LOG-PRINT-BODY-CONTRACTS (extra) | `5e44bfa5a` | tip | 27 | full 604 | full 604 | |
| | `f04c3075d` | red (inferred) | 20 | full 604 | full 604 | `contract_loader`, `provider_selection`, `neighbours`, `prod_contracts` |

No failing unit is dropped: every one is attributed by the new plan, not only
run because the plan is full. The two contract-only cards that were full only
because of string literals and test-file includes now run about a third of
the units. The others stay full on a real crate-level reach. LIBC-GAP and
LOG-PRINT reach qcore-contract's or the gateway's library tests
(`tests/unit/*.rs` compiled into the library under `#[path]`). CELL-PROTOCOL
and MEMBER-ACCESS change gateway `src/`. W45 changes the workspace manifests.

## 6. Limits

The census is one commit's observation. A test that later starts reading a
path through data, or a code path no test took, is not seen until the census is
recorded again (`gate_read_census.py record`, then `reduce`, then
`draft_walkers.py`). The landing gate stays the arbiter, and validation.md says
so. The provider workspace is not traced; its sites keep their whole
directory. A library's `#[cfg(test)]` modules (`tests/unit/*.rs` under
`#[path]`) are still crate-wide, which is conservative (follow-up below). The
census test takes about 6 s of the Python suite.

## 7. Gate

See [gate.json](gate.json) and section 8. Two Python failures belong to the
base, not this card, and stay for the integrator: the base's committed matrix
lists 408 Lean audits against 410 in the tree (strict mode; the card-mode check
excuses exactly that base drift), and
`test_gateway_registry.test_committed_tables_equal_generated`
(`crates/qcore-c-gateway/src/gateway_rule_registry.rs` stale at `25508ccaf`;
this diff touches neither the registry nor the gateway).

## 8. Follow-ups

- GATE-SCOPE-LIB-UNIT-TEST-MODULES: files a library compiles only under
  `#[cfg(test)]` (`tests/unit/*.rs` via `#[path]`) reach only that library's
  unit-test executable, not its dependents; today they select the crate whole,
  which keeps LIBC-GAP- and LOG-PRINT-shaped contract cards full.
- GATE-READ-CENSUS-REFRESH: the census and the drafted table describe
  `a9de4e8be`; nothing yet re-records them when tests change (a scheduled
  re-record, or a cheaper per-unit tracer the card gate can run on the units it
  selects).
- TEST-WRITES-INTO-CHECKOUT: a qcore-audit test writes
  `crates/qcore-audit/private/audit-retained-products/` into the checkout.
