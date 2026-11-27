# WAVE21-QUIRIN-RENAME — the language is promoted as Quirin

The language is presented as **Quirin**. This card renames the two installed
commands, moves the branding surfaces (README, CLI text, document titles) onto
that name, and retires the `.qc` alternative source extension. Nothing else
moves: no semantics, no diagnostics, no emitted-C surface, no crate names, no
corpus. The card is corpus-neutral by construction — no gateway, audit-logic,
contract or provider code is touched.

## 1. What changed, in one table

| Surface | Before | After |
|---|---|---|
| Language driver command | `qcorec` | **`quirin`** |
| Configured audit command | `qcore-audit` | **`quirin-audit`** |
| Source extension | `.qcore` and `.qc` | **`.qcore` only** |
| Product name in README / doc titles | "QCore-C" / "QCore …" | **"Quirin …"** |

Both renames are `[[bin]] name = "…"` declarations inside the **existing**
crates. The packages are still `qcorec` and `qcore-audit`, and the libraries are
still `qcore_audit`, `qcore_compiler`, `qcore_core`, and so on.

## 2. The boundary — what deliberately did NOT move

This boundary was fixed when the card was reserved and is not widened here. Each
row states the ground, because "we could have renamed it" is the wrong test; the
test is whether a rename would move a pin or a semantic surface for no gain.

| Kept | Ground |
|---|---|
| Crate and package names (`qcore-compiler`, `qcore-schemas`, `qcore-audit`, `qcorec`, `qcore-core`, `qcore-frontend`, `qcore-contract`, `qcore-c-gateway`, `qcore-verifier-host`, `qcore-verification`) | User rule: the core component of the language stays seated as "qcore". Renaming would also churn `Cargo.lock`, the provider workspace's lock, `compiler/tests/workspace_layering.rs`'s pinned layering table and every `use qcore_*` path, for zero semantic gain. |
| Rust namespaces `qcore_*` | Same rule; every module path, doc comment and Lean cross-reference names them. |
| Emitted-C surface: `QCORE_IO_WRITE`, `QCORE_IO_FLUSH`, `qcore_task_*`, `qcore_buffer_*`, `QCORE_BUFFER_*` / `QCORE_TASK_*` trace vocabulary | These bytes are compared. Renaming them moves F02 trace digests and every byte-compared emission pin. The gain would be cosmetic; the cost is a re-pin of pinned artifacts, which this card is not scoped to do. |
| Specification and document **filenames** (`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_CONTRACT_SPEC_r0_3.md`, `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`, `docs/QCORE_MODULE_MODES.md`) | They are load-bearing cross-references from frozen reports under `docs/reports/` and `docs/reviews/`, and from Rust doc comments (`legacy_literal_premise.rs`, `borrow_liveness.rs`, `extern_c_emit.rs`, `mutex_c_emit.rs`, `report.rs`, `reason.rs`, `source_mutex.rs`). Only the H1 title lines inside them changed. |
| Diagnostics codes and namespaces (`Q000`–`Q703`, `QP001`–`QP016`, `<plugin-id>.<local-code>`, the `qcore.libevent.L00`–`L12` provider codes) | Codes are stable by AGENTS constraint 1; a code change is semantic. |
| The `.qcore` extension itself | It is the surviving extension, not a casualty of the branding. |
| Plugin identifiers (`qcore.libevent`, `qcore.chunkio`, …) | They are declared in provider manifests and compared field-for-field against the template embedded in each provider executable (AGENTS constraint 18). Changing one would require rebuilding and re-verifying all five packages for a cosmetic gain. |
| The repository name `qcore-c` and the `contract/qcore/` corpus directory | Paths appear in pinned compile-task identities, frozen reports and `docs/CYCLE19_BUILD_INPUTS.md`. |
| The emacs mode name, file and function prefix (`qcore-mode`, `qcore-mode.el`) | Same "core component" rule; the README now states it explicitly so a reader does not read it as a missed rename. |
| Frozen archives under `docs/reports/` and `docs/reviews/` | AGENTS constraint 21: never edited in place. 397 `qcorec` occurrences and 51 `.qc` occurrences live there and are untouched. |
| `docs/cycles/CYCLE_*.md` | Per-cycle **acceptance records**: they record what a bounded cycle accepted at the time it closed. `CYCLE_12.md` §"The `qcorec` binary is the stable command-line boundary" and `CYCLE_20.md`'s `qcorec plugins list` are records of that cycle's surface, in the same class as a frozen report. Listed as a follow-on below rather than rewritten. |
| `docs/design/W2_PROFILE_MANIFEST_SCHEMA.md` line 514 | It quotes the evidence sentence of a frozen report (`docs/reports/w2-profile-providers/README.md`). Editing the quote would make it stop matching its source. |
| `docs/reevaluation/*.log` | Raw archived run logs (they contain `qcorec-<hash>` test-binary names). Archives, not prose. |

## 3. Census: every site that invokes or names a binary

The census was taken with `git grep -n 'qcorec'` and
`git grep -n 'qcore-audit\|qcore_audit'` over the whole tree, then split by
whether the occurrence names the **binary** (renamed) or the **package / crate /
library** (unchanged, because those names did not move).

Totals: **93** live `qcorec` occurrences outside frozen archives (397 more
inside them, untouched); **77** live `qcore-audit` / `qcore_audit` occurrences.
Of those, **44** name a binary and were dispositioned; the rest are package,
crate, module or path references that are correct as they stand.

### 3.1 Binary-naming sites that changed

| Site | Occurrences | Change |
|---|---|---|
| `crates/qcorec/Cargo.toml` | 1 | `[[bin]] name = "qcorec"` → `"quirin"`; `description` rebranded; comment records that the package name is deliberately kept |
| `crates/qcore-audit/Cargo.toml` | 0 → 1 | new explicit `[[bin]] name = "quirin-audit"` (the target previously defaulted to the package name) |
| `crates/qcorec/src/main.rs` | 12 | `USAGE` (10 command spellings + the version banner line) and the `--version` line; one comment |
| `crates/qcore-audit/src/main.rs` | 2 | both `usage: qcore-audit CONFIG.json` lines |
| `crates/qcorec/tests/cli.rs` | 15 | `env!("CARGO_BIN_EXE_qcorec")` → `CARGO_BIN_EXE_quirin` |
| `crates/qcorec/tests/plugin_packages.rs` | 1 | same |
| `crates/qcore-audit/tests/configured_cli.rs` | 1 | `env!("CARGO_BIN_EXE_qcore-audit")` → `CARGO_BIN_EXE_quirin-audit` |
| `crates/qcore-audit/tests/root_source_set.rs` | 1 | same |
| `crates/qcore-audit/tests/second_source_set.rs` | 1 | same |
| `README.md` | 5 | `cargo build --bin qcorec` → `--bin quirin`; two `./target/debug/qcorec …` invocations; two prose mentions |
| `contract/qcore/README.md` | 2 | `checked directly with qcorec`; `The ordinary qcorec check FILE.qcore command` |
| `docs/QCORE_SPEC_r0_3.md` | 3 | two profile-table rows; the CLI section now names `quirin` as the installed command and keeps the package spelling for checkout use |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | 1 | `the ordinary qcorec check command` |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | 3 | `qcorec check` / `qcorec emit`; `qcorec --native-library MANIFEST` |
| `docs/QCORE_MODULE_MODES.md` | 1 | `qcorec emit reports that boundary` |
| `docs/GENERALIZATION_PLAN.md` | 2 | `qcorec plugins` tooling; `qcore-audit CONFIG.json` |
| `docs/ROADMAP_AFTER_CYCLE_9.md` | 1 | `qcore-audit CONFIG.json` |
| `docs/design/UNSAFE_LINKAGE_V1.md` | 2 | `qcorec check prints them`; the `UNSAFE-LINKAGE-CLI` follow-on description |
| `providers/README.md` | 8 | six `qcorec plugins …` command lines, two prose mentions |
| `crates/qcore-audit/README.md` | 2 | two `# Run qcore-audit with …` comment lines; a new sentence naming `quirin-audit` as the installed command |
| `compiler/tests/unsafe_linkage.rs` | 1 | comment naming what the CLI can do |
| `examples/threads/build-windows.ps1` | 1 | the failure message text (`QCore emission failed` → `Quirin emission failed`); a comment records why `-p qcorec` is still correct |

### 3.2 Binary-invoking sites that needed **no** change, and why

This is the part worth reading, because "CI invokes the binary" turned out to be
false: every automated invocation selects the **package**, not the binary name.

| Site | Spelling | Why it still works |
|---|---|---|
| `.github/workflows/ci.yml:139` | `cargo test -p qcorec --test cli` | Package selector; package name unchanged. |
| `.github/workflows/ci.yml:157` | `cargo run -p qcorec -- emit …` | Package selector; the package has exactly one binary target, so cargo resolves it without a `--bin` flag. |
| `.github/workflows/ci.yml:192` | `cargo test -p qcore-audit --test second_source_set` | Package selector. |
| `examples/threads/build-windows.ps1:28` | `cargo run --manifest-path … -p qcorec -- emit …` | Same; comment added so the next reader does not "fix" it. |
| `examples/threads/README.md:52,53,90` | `cargo run -p qcorec -- …` | Same. |
| `contract/qcore/README.md:18`, `docs/QCORE_MODULE_MODES.md:46,48`, `docs/QCORE_SPEC_r0_3.md` code block | `cargo run -p qcorec -- …` | Same. |
| `crates/qcore-audit/README.md:11,88`, `docs/CYCLE19_BUILD_INPUTS.md:69,87` | `cargo run -p qcore-audit -- …` | Same. |
| `Cargo.toml` workspace members, `compiler/tests/workspace_layering.rs`, `compiler/Cargo.toml`, `providers/conformance/Cargo.toml`, `Cargo.lock`, `providers/Cargo.lock` | `crates/qcorec`, `qcore-audit = { path = … }` | Path and package identity; unchanged by design. |
| `scripts/tests/test_sidecar_reason_inventory.py` (2 sites), `crates/qcore-schemas/src/reason.rs`, `crates/qcore-schemas/tests/reason_codes.rs`, `proof/QCore/{ReasonCarrier,ReasonCategory,SourceRootAdmission}.lean`, `proof/AuditReasonCarrier.lean` | `qcore-audit` / `qcore_audit` | Crate and module references, not commands. Lean is untouched, as required. |

**No CI workflow edit is required by this card.** That is a census finding, not
an omission: had CI spelled `cargo run --bin qcorec`, it would have broken.

### 3.3 The alias decision

**No transitional `qcorec` or `qcore-audit` alias binary ships.** Ground:

1. The census above is complete and every in-repository caller is either
   updated or package-scoped, so nothing in the tree needs the old spelling.
2. Both packages are pre-1.0 (`0.1.0`) and neither is published to crates.io,
   so there is no external installed-command contract to honour.
3. No CI cache, pinned artifact or frozen archive names an executable **file**
   `qcorec` or `qcore-audit`. The archived run logs under `docs/reevaluation/`
   name `qcorec-<hash>` *test* binaries, which cargo derives from the package
   name — unchanged — not from the `[[bin]]` name.
4. An alias binary is not free: a second `[[bin]]` target in `crates/qcorec`
   would make `cargo run -p qcorec --` ambiguous, breaking exactly the CI and
   documentation invocations that §3.2 shows are the ones actually in use. That
   inverts the cost the alias was supposed to save.

The stale `target/debug/qcorec` from a pre-rename build is a build artifact, not
a shipped alias; a clean checkout produces only `quirin` and `quirin-audit`.

## 4. Census: every `.qc` claim

`git grep -n '\.qc\b'` finds **99** occurrences. They fall into four classes.

| Class | Count | Disposition |
|---|---|---|
| Real `.qc` **files** on disk | 1 | `examples/cycle12.qc` → `git mv` to `examples/cycle12.qcore`. The tree now holds 385 `.qcore` files and **zero** `.qc` files. |
| Live **extension registrations and claims** | 6 | All retired (table below). |
| **Synthetic in-memory unit names** in Rust tests (`SourceFile::new("main.qc", …)`, `translation_unit: "unit.qc"`, …) | 39 | Deliberately **kept** — see §4.2. |
| **Frozen or historical** text (archives under `docs/reports/`, `docs/reviews/`, `docs/reevaluation/`, plus two recorded censuses) | 53 | Untouched (AGENTS constraint 21). |

### 4.1 Live claims retired

| Site | Claim | Change |
|---|---|---|
| `editors/emacs/qcore-mode.el:199` | `auto-mode-alist` entry `"\\.\\(?:qcore\\|qc\\)\\'"` | → `"\\.qcore\\'"`, with a comment naming the retirement |
| `editors/emacs/qcore-mode-tests.el:19` | ERT `qcore-mode-file-associations` asserted `cycle12.qc` selects the mode | now asserts `program.qcore` / `contract.qcore`; a **new** ERT test `qcore-mode-retired-qc-extension` asserts `.qc` does *not* select it |
| `editors/emacs/qcore-mode-tests.el:130` | corpus smoke test globbed `\\.\\(?:qcore\\|qc\\)\\'` | → `\\.qcore\\'` |
| `editors/emacs/README.md:4` | "selects QCore mode for `.qcore` and `.qc` files" | `.qcore` only, plus an explicit retirement sentence |
| `editors/emacs/README.md` validation section | described the file-selection regression | now names the negative `.qc` check |
| `README.md:120` | documentation-map row `.qcore` / `.qc` | → `.qcore` |
| `README.md:93,94` | invoked the CLI on `examples/cycle12.qc` | → `examples/cycle12.qcore` |

**There is no CLI or loader extension check.** `quirin check`/`emit` take any
path and `compile_files` never inspects an extension, so retiring `.qc` is not a
behaviour change in the driver — it is a change in what the editor claims and
what the tree contains. The only extension filters in the tree are
`extension() != "c"` guards in the audit examples, which are about C sources.

### 4.2 The in-memory unit names, kept on stated grounds

Thirty-nine occurrences are string literals naming a source that never exists on
disk: `SourceFile::new("main.qc", source)` in `compiler/tests/unit/driver.rs`,
`translation_unit: "unit.qc"` in `compiler/tests/w3_evidence_v2.rs`,
`crates/qcore-verification/tests/*`, `crates/qcore-verifier-host/tests/*`,
`crates/qcore-schemas/{src/obligation.rs,tests/*}`, and
`"scope.qc"` / `"missing.qc"` in the Cycle 20 plugin tests and the
`reevaluation_followup` example.

They are kept, because they are **not extension claims**:

* Nothing reads them as a file. No loader, no `auto-mode-alist`, no glob.
* They are opaque identifiers **inside digest and evidence inputs**. A
  `translation_unit` string is hashed into obligation digests and compared in
  witness replay and composition relations; `"main.qc"` is asserted to appear
  verbatim in an emitted `#line` directive. Changing them would move digest
  inputs and pinned strings — a semantic change, which this card forbids.
* The corresponding *claim* — "the toolchain accepts `.qc` sources" — is
  retired where it was actually made (§4.1).

Recorded as **`QC-SYNTHETIC-UNIT-NAMES`**: a later card that re-pins the digests
for its own reasons may normalise these to `.qcore` in the same change. Doing it
here would be a re-pin disguised as a rename.

### 4.3 Recorded censuses left alone

`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md:1421` and
`docs/design/SOURCE_BRANCH_V1.md:77` both say "census across all 369
`.qcore`/`.qc` files". These are statements about a census that was taken at a
past moment, with a count that was true then. They are left exactly as written:
rewriting the extension inside a recorded census would silently change what the
census is claimed to have covered while leaving its number unchanged.

## 5. Behaviour neutrality — the pins

`quirin` was built from the renamed target and compared against the
pre-rename `qcorec` binary built from `d9a453e`.

| Pin | Baseline (SHA-256) | After rename | Verdict |
|---|---|---|---|
| `emit -o … examples/threads/two_workers.qcore` | `54c9ea21c1d64ea399a8466f405cc0c5f8291c04c957325a4f5314dedaeb405f` | identical | byte-identical |
| `check examples/threads/core_context.qcore` (stdout+stderr) | `a1d47c2806969d2338a60527ddffc2a4e3e836a997996e62b816af04c505b5d0` | identical | byte-identical, exit 0 |
| `check examples/cycle12.{qc → qcore}` | `3d462d73561e45fb754b8b9c436e106eed7d38e702d5df428724d151cd4968f1` | identical | byte-identical, exit 0 |
| `emit examples/cycle12.{qc → qcore}` | see note | `02dea5a4479bdff21dfda6bb43b211d7aa661d1165c59e856a7b64e0d418c9cd` | identical **after** normalising the input path |

The last row is the one honest caveat. The emitted C carries a
`#line 3 "examples/cycle12.qc"` directive naming the input as spelled, so
renaming the input file changes one byte-range of the output. Normalising both
spellings to a common token makes the two emissions hash identically
(`02dea5a4…` on both sides), which is the statement that matters: **the compiler
did not change**; the input file's name did. This is the same
`__FILE__`-preservation property AGENTS constraint 11 protects — the spelled
input path is passed through, not normalised, and this card does not move it.

The renamed CLI's own text did change, deliberately (`quirin 0.1.0 — the Quirin
language driver`, ten `quirin …` usage lines, `usage: quirin-audit CONFIG.json`).
No test pinned those strings before or after; the census confirmed no assertion
anywhere reads `USAGE` or the `--version` line.

## 6. Gate

Run from this worktree at the card's tip.

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | libevent checked out at the pinned `d82464a277d0f42703702c4dfd9af6af38595a83` |
| `cargo test --workspace` | see §6.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean, exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | exit 0, but **`lake` is not installed on this machine**: the script printed `Lean checks failed: [Errno 2] No such file or directory: 'lake'` and exited 0. Reported honestly as `environment-unavailable`, not as a pass — see §6.2 |
| `python3 -m unittest discover -s scripts/tests` | 193 tests |
| `emacs -Q --batch -L editors/emacs -l qcore-mode-tests.el -f ert-run-tests-batch-and-exit` | 10 tests, 0 unexpected (was 9; the retirement adds one) |

### 6.1 Test counts

`cargo test --workspace` exited **0**: **3,236 passed, 0 failed, 8 ignored**
across **185 test binaries**. Summed from the 185 `test result:` lines of
`measure/workspace_tests.log`.

The card adds and removes **zero** Rust test functions
(`git diff | grep -c '^[+-].*#\[test\]'` is 0 on both signs), so this number is
the branch's own baseline carried through unchanged, not a number this card
moved. The eight ignored tests are the standing eight.

Outside the workspace suite:

| Suite | Count |
|---|---|
| `python3 -m unittest discover -s scripts/tests` | 193 tests, OK |
| Emacs ERT (`qcore-mode-tests.el`) | 10 tests, 0 unexpected — **9 before**; the retirement adds `qcore-mode-retired-qc-extension` |
| Lean audits | 80 expected, **not run here** (§6.2); `proof/` diff is empty |

**Machine load.** A sibling session's `cargo test --workspace` ran concurrently
with this one for most of its duration (two full workspace runs, ~23 GB
available). That is exactly the AGENTS constraint-16 load scenario, so it is
recorded rather than glossed.

Of the four constraint-16 rows, the ones inside the **core** workspace passed in
the shared run and needed no isolation re-run:
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (the wall-clock row,
measured elsewhere to fail at load 50) passed, as did
`crates/qcore-verifier-host/tests/{enforced_process_limits,execution_quota}.rs`
and `tests/confined_process_execution.rs`. Had any tripped, it would have been
re-verified alone before being called green.

`real_packages_preserve_unknown_and_reject_mutations` and the `RLIMIT_NPROC`
conformance row live in `providers/conformance/`, a **separate workspace
excluded from the core one**, so `cargo test --workspace` never ran them and
this report does not claim a result for them. That is not an omission: AGENTS
constraint 18 requires the provider rebuild-and-verify pass only after a change
to the manifest template or to a `qcore-schemas` major version, and this card
changes neither — `providers/` is untouched except for the six `quirin plugins`
command spellings in its README, which no test reads. The integrator's usual
`providers/package.py` step is unaffected.

### 6.2 The Lean audit step, stated honestly

`scripts/check_lean_audits.py` exited 0, but it did **not** check anything: this
machine has no `lake`, `lean` or `elan` on `PATH`, and the script reported
`Lean checks failed: [Errno 2] No such file or directory: 'lake'` before exiting
0. Recording that exit code as "80 Lean audits pass" would be exactly the
optimistic fallback AGENTS invariant 1 forbids, so it is recorded as
`environment-unavailable`, which says nothing about the audits.

What can be asserted without the toolchain is stronger than a run would be for
this card anyway: **`git diff -- proof/` is empty in both the working tree and
the index.** No `.lean` file, no `Audit*.lean` registration and no axiom surface
changed, so the audit count cannot have moved. The card requires no Lean parity
— a binary rename and an extension retirement are not semantic changes — and it
introduces none.

The integrator should re-run the step on a machine with the pinned
`proof/lean-toolchain` before merging, and expects the unchanged **80**.

## 7. What the integrator owes

`AGENTS.md`, `CLAUDE.md`, `DEVELOPMENT.md` and
`docs/reviews/r0_3/acceptance_matrix.json` are integrator-owned and were not
touched. The reconciliation list:

**`AGENTS.md`**

1. Line 50, repository map: ``| `crates/qcorec/` | CLI binary `qcorec` plus its
   process-level tests |`` → the binary is now `quirin`; the crate name is
   unchanged. Suggested: "CLI binary `quirin` (package `qcorec`) plus its
   process-level tests".
2. Line 390, the per-change gate paragraph: ``qcorec check`/`emit` and
   generated-C tests work without Clang`` → `quirin check`/`emit`.
3. Lines 415–416, key entry points: ``qcorec check <file>``,
   ``qcorec emit -o out.c <file>``,
   ``qcorec plugins list|inspect <id>|verify <package>|enable|disable`` →
   `quirin …` throughout.
4. Line 417, configured audits: ``qcore-audit CONFIG.json`` → `quirin-audit
   CONFIG.json`. The crate reference on line 49 stays as it is.
5. Line 424, editor entry point: ``gives lexical `.qcore`/`.qc` highlighting``
   → `.qcore` only.
6. Consider a one-line naming statement near the top of "What this project is":
   the language is Quirin, the core component stays `qcore`, so an agent does
   not read the surviving `qcore_*` names as an incomplete rename. The precise
   boundary is §2 of this report.

**`DEVELOPMENT.md`**

7. Line 34: ``| crates/qcorec/ | CLI binary `qcorec` over the compiler library |``
   → binary `quirin`.
8. Line 42: "the CLI binary `qcorec`" → `quirin`.
9. Line 233: "`qcorec check` / `qcorec emit` do not require Clang" → `quirin`.
10. Line 339–341, the Windows quick-start block: ``cargo build --bin qcorec`` →
    ``--bin quirin``; ``.\target\debug\qcorec.exe`` → ``.\target\debug\quirin.exe``;
    **and** ``examples\cycle12.qc`` → ``examples\cycle12.qcore`` (twice) — that
    file was renamed by this card, so those two lines are now broken paths.
    This is the one reconciliation item that is a *correctness* fix, not
    branding.
11. Line 461: "`qcorec check` prints the premises" → `quirin check`.
12. Line 750: "`qcorec plugins verify PACKAGE`, `enable …`" → `quirin plugins …`.
13. Line 841: "normal `qcorec check`/`emit`" → `quirin`.
14. Line 939: "`qcore-audit CONFIG.json` now connects" → `quirin-audit`.
15. Line 281 (`cargo run -p qcorec -- emit …`) and line 2853 (the layering
    paragraph naming the `qcorec` CLI crate) are package/crate references and
    need **no** change.
16. A new "Increment records" paragraph for this card.

**`CLAUDE.md`**

17. The "Pointers" section's Emacs entry names "`editors/emacs/qcore-mode.el`,
    with installation and ERT checks" — no `.qc` claim there, so no edit is
    forced; but if the naming statement of item 6 lands in `AGENTS.md`, a
    one-line echo here would keep the two files consistent.

**`docs/reviews/r0_3/acceptance_matrix.json`**

18. No derived field in the matrix reads a binary name or an extension, so no
    regeneration is required by this card. If the integrator wants the rename
    tracked as a named follow-on, add `QC-SYNTHETIC-UNIT-NAMES` (§4.2) to
    `named_followups`.

## 8. Follow-ons

* **`QC-SYNTHETIC-UNIT-NAMES`** — 39 synthetic `*.qc` unit names inside Rust
  test digest inputs are retained (§4.2). A card that re-pins those digests for
  its own reasons may normalise them; doing so alone would be a re-pin
  masquerading as a rename.
* **`CYCLE-RECORD-COMMAND-SPELLINGS`** — `docs/cycles/CYCLE_12.md` (lines 28–32)
  and `docs/cycles/CYCLE_20.md` (line 172) record `qcorec` command spellings as
  the accepted surface of a closed cycle. They read as stale to a newcomer. The
  repair is to publish the current spelling *beside* the record (the same
  dual-publication pattern AGENTS constraint 21 prescribes for frozen
  archives), not to edit the record. Not done here: this card is not scoped to
  decide the editability of cycle acceptance records.
* **`UNSAFE-LINKAGE-CLI`** (pre-existing, re-spelled here) — the follow-on for a
  `quirin` flag selecting a contract environment, in
  `docs/design/UNSAFE_LINKAGE_V1.md`.
* **Provider plugin identifiers** stay `qcore.*` (§2). If a later card ever
  wants `quirin.*`, it must rebuild and re-verify all five provider packages in
  the same change (AGENTS constraint 18).
