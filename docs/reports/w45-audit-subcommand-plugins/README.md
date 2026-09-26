# W45-AUDIT-SUBCOMMAND-PLUGINS: external `quirin-audit` subcommands, their SDK, and the libevent plugin

Card W45-AUDIT-SUBCOMMAND-PLUGINS, 2026-09-26, on base `37e1b82d` ("Record
the w45-refuted-contract-withdrawal integration gate"); `main` moved during the
card and is not merged (§10). The user's decisions of 2026-09-26: `quirin-audit` gains
cargo-style external subcommands with an SDK crate `quirin-audit-plugin`;
plugins contribute verdicts only through the certificate firewall of
W45-USER-CONTRACT-PLUGINS; and the libevent preparation script becomes the
first plugin, in its own crate `quirin-audit-plugin-libevent`, and is retired.
The [prediction](PREDICTION.md) was frozen before implementation.

The normative text is contract specification
[C11.2](../../QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins);
the standing rule is gateway [constraint 10h](../../agents/gateway.md#constraint-10h);
the user page is [audit subcommand plugins](../../development/audit-subcommand-plugins.md).

## 1. Dispatch

`crates/qcore-audit/src/subcommand_plugins.rs` is the new module; `main.rs`
gains one hook after every built-in and before the `CONFIG.json` form, and one
usage line.

```text
quirin-audit [--plugin-path DIR]... --output DIR [HOST-OPTIONS] NAME [ARGUMENTS...]
quirin-audit [--plugin-path DIR]... --list-plugins
```

* **Built-ins always win.** `BUILTINS` lists the 16 names `main.rs` matches,
  and a unit test reads `main.rs` and pins the two sets equal. The hook runs
  after every built-in, so `quirin-audit plugin …` reaches the built-in even
  with a `quirin-audit-plugin` executable on the search path; with host
  options in front (`quirin-audit --plugin-path D plugin`), the route refuses
  the built-in name as a usage error. `find` never looks a built-in name up,
  and `--list-plugins` marks such an executable "shadowed by a built-in; never
  run".
* **Explicit search.** `--plugin-path DIR` (repeatable, in order), then
  `QUIRIN_AUDIT_PLUGIN_PATH` (`split_paths` syntax). **`PATH` is not
  searched** — no deviation. The first executable `quirin-audit-NAME` wins.
* **Clean arguments and environment.** Everything after `NAME` passes
  verbatim (a host flag after the name is the plugin's). The environment is
  cleared except `PATH`, `HOME`, `TMPDIR`, `LANG`, `LC_ALL`, `LC_CTYPE` and
  `TZ` (`quirin_audit_plugin::ENVIRONMENT_ALLOW_LIST`) and each
  `--plugin-env NAME`; the invocation record lists every name passed. `PATH`
  is passed so a plugin can run `git`, `cmake` and `clang`; it is never used
  to find a plugin.
* **Unknown names.** A name found nowhere is a usage error (exit 2) listing
  the search path and saying `PATH` is not searched.
* **What is a name.** `[a-z0-9][a-z0-9-]*`. A configuration path contains a
  `.` or a `/`, so `quirin-audit CONFIG.json` is unaffected.

Exit status: 0 the plugin completed; 1 the run failed (report written,
`Unknown`); 2 a usage error or refused input (nothing ran).

## 2. Protocol and SDK

`crates/quirin-audit-plugin` (library `quirin_audit_plugin`) defines protocol
`quirin-audit-plugin` version 1, versioned JSON over standard input and
output, every struct `deny_unknown_fields`.

* **Request:** subcommand and arguments; `invocation_directory`; `output`
  (canonical, the plugin's working directory); `report`; `load_path` (path and
  SHA-256, validated with the core load-path reader) and `providers` (each on
  it); `identity` (revision, contract-set digest, refutation-table digest, from
  the `--qo` checkpoints, which are verified whole and must agree); the
  `--audit-report` and each checkpoint with their digests; `quotas`.
* **Response:** `outcome` (`"completed"` or `{"failed": REASON}`), `artifacts`
  (path, SHA-256, kind), `diagnostics`, and `claims`: the tuple of hard
  invariant 2 (artifact, property, semantic profile, scope, bounds,
  assumptions, evidence version), a proposed `Safe`/`Unsafe`/`Unknown` (there
  is no `Pending` proposal; that is the host's classification), a reason and
  certificate references.
* **Version.** `parse_response` reads `protocol` and `protocol_version`
  before the shape, so another version is `protocol-version-mismatch`, never a
  shape error.
* **SDK surface:** `Request::from_slice`, `run` (read, answer, write; exit 0
  or 1), `Response::completed`/`failed`/`push_artifact` (hashes the file)/
  `push_diagnostic`/`push_claim`/`validate`, `verify_artifacts`, the
  harness `testing::invoke` and `Invocation::verified`, `json_text` (`JsonText`,
  the ordered, indented JSON text writer of W44-QUIRIN-AUDIT-PREPARE, public
  here; `W45-JSON-TEXT-DUPLICATE`), and the example plugin `quirin-audit-example`.
  It depends on `qcore-schemas` for SHA-256 only, `serde` and `serde_json`,
  like `quirin-provider-plugin`; it links no core analysis crate and contains
  nothing library-specific.

## 3. Quotas and confinement

The host spawns the plugin with `process_group(0)` and captures its PID. A
reader thread takes standard output up to its quota; standard error goes to
`<report>.stderr.log`, whose size is polled. A wall-time breach, oversize
standard output or oversize standard error kills the group
(`kill(-pgid, SIGKILL)`), and so does the end of every run, so nothing the
plugin started outlives it. On Linux the host observes the exit with
`waitid(WNOWAIT)`, leaving the leader unreaped while it kills the group, so the
group ID cannot name another group — only captured PIDs are signalled. The
working directory is the declared output; every artifact must resolve inside
it (after `canonicalize`), must not be one of the host's three records, and
must have its declared SHA-256. The verifier host's process transport was not
reused: it is bound to a verified package manifest and frame codec; the
quota semantics (deadline, output bound, group kill) are the same. This is
containment, **not an OS sandbox** (constraint 26): a plugin can write
elsewhere and a `setsid` descendant escapes the kill
(`W45-SUBCOMMAND-CONFINEMENT`). The route refuses on a non-Unix host
(`W45-SUBCOMMAND-NON-UNIX`). `qcore-audit` now depends on `libc` on Unix, the
version `qcore-verifier-host` already locks; no new crates.io dependency.

## 4. Firewall, join and provenance

`classify_claim` maps a claim onto `qcore_verifier_host::PluginClaim` and
`PluginClaim::verdict` gives its value — the same function that classifies a
checker plugin's answer:

| Claim | `PluginClaim` | Verdict |
|---|---|---|
| `Safe`, no certificate | `UncertifiedSafe` (`QP008`) | `Pending` (`plugin-claim-uncertified`) |
| `Safe`, certificate references | `UncertifiedSafe` (`QP016`) | `Pending` |
| `Unsafe` | `UnvalidatedUnsafe` (`QP010`) | `Pending` |
| `Unknown` | `Undecided` | `Unknown` |
| run failed | `Failure` (`plugin-spawn`, `plugin-timeout`, `plugin-output-limit`, `plugin-stderr-limit`, `plugin-escaped-descendant`, `plugin-exit`, `protocol-version-mismatch`, `malformed-response`, `plugin-reported-failure`, `plugin-artifact`) | `Unknown` |

`CertifiedSafe` needs a `CoreCertificate`, which has no public constructor and
is issued only by the registry's witness replay, so **no subcommand claim can
reach `Safe`**; no checker exists for these claims today
(`W45-SUBCOMMAND-CERTIFICATE-ROUTE`), so none reaches `Unsafe` either. A failed
run's claims are recorded (`"joined": false`) and joined nowhere.

`join`: each row (one artifact) is the join of the host's verdict for it —
from `--audit-report`, whose unit rows' `status` by `file` are the host's
verdicts (`Pending` only with a human-resolvable `reason_code`, else
`Unknown`; `data_only` carries none) — the run's (`Unknown` if it failed) and
its classified claims. The report's `verdict` is the join of every row and the
run, and `null` for a completed run with no row, so a plugin that claims
nothing publishes no vacuous `Safe`.

Records: `<report>` (default `<output>/quirin-audit-NAME.report.json`) holds
the plugin identity (`name`, `executable`, `sha256`, `label` =
`quirin-audit-NAME@<sha256[..16]>`), both protocol versions, the request, the
run (outcome, code, reason, exit code, signal, wall time, byte counts, stderr
log), the verified artifacts, the diagnostics, every claim with the plugin's
identity and its firewall classification, the rows and the verdict.
`<report>.invocation.json` holds the argv, cwd, search path, the plugin's path,
SHA-256, protocol version and exit, the environment names passed, the quotas
and the request's SHA-256. **The route writes no `.qo`**, so no `.qo` schema
changed; `--qo` inputs are verified with `CheckpointReader` and passed by
content digest.

## 5. The libevent plugin and the retired script

`crates/quirin-audit-plugin-libevent` (binary `quirin-audit-libevent`,
depending on the SDK, `qcore-schemas` and `serde_json` only) performs the
script's steps in order: the pin check against `profile.json`, the two
clean-checkout checks, the `PATH` lookup of both compilers and the Clang
banner check, the CMake cache reuse checks, the configure and `-j2`
`event_core_static` build with one merged pipe per log, the unit selection
(the final `.`-suffix of a name, non-strict path resolution), the declared
audited-file set and digests, and the three records through the ordered JSON
text writer (`quirin_audit_plugin::json_text::JsonText`). Its process, path
and text operations are the module `src/audit.rs`. (By the user's naming
correction, modules and identifiers are named for what they do in the audit,
not after the retired script's language: `src/python.rs` became
`src/audit.rs`, `python_strip` `strip_whitespace`, and the SDK's
`python_json::PyJson` `json_text::JsonText`, with `dumps_indent2`
`to_indented_text`; behaviour unchanged, parity re-measured below.)
Two deliberate differences, neither reaching a prepared byte: the output is
the host's required `--output` (an output inside `build/cycle19` or
`third_party` is refused), and `--write-profile` writes the regenerated profile
to `libevent-clang-build/profile.json` as an artifact for review instead of
overwriting the committed one. The plugin claims no verdict.

**Parity** ([harness](scripts/parity.sh), [comparator](scripts/compare_trees.py),
[log explanation](scripts/explain_logs.py); evidence in [parity/](parity/)).
Pinned `third_party/libevent` `d82464a2`, Ubuntu clang 21.1.8
(`/usr/lib/llvm-21/bin/clang{,++}`), CMake 4.2.3, the script from base
`37e1b82d`, the plugin at `3a08df5f` (after the module rename); outputs in
`/tmp/claude-1000/w45-asp-parity-3/`, outside the repository.

| Pass | What | Files compared | Identical | After root normalisation | Differ (all explained) |
|---|---|---:|---:|---:|---:|
| 1 | different roots | 152 | 104 | 41 | 7 |
| 2 | same path, one after the other | 152 | **147** | — | 5 |
| 3 | `--write-profile` | 1 | `cmp` exit 0 against the committed `profile.json` | | |

In pass 2 every prepared file is byte-identical, SHA-256:

| File | SHA-256 (script = plugin) |
|---|---|
| `compile_commands.json` | `6be53a5faabfe386f7546cccc298c963ddc40ec01d5f711aaaa5cca3fcd7d225` |
| `libevent-core-audit.json.config` | `2eef66cd4e9c94119338d0869434ffd76b5677432827d3e79cf64ab25939e6fb` |
| `libevent-build-provenance.json` | `125493b4548f1743c6f6b6128f7ff2c721fbfb7f3bc7d8a914f7345785bbf67f` |
| `CMakeCache.txt` | `ff440c0b0dcf31623c9eb0aa6c654d64ce34055faf749ab6ee4ff470adde6000` |
| `lib/libevent_core.a` | `2a14b00576aa2566d47748513ce03974e76462436ada1f67f2663b7c89109935` |

The 5 that differ are logs, each explained by
[`explain_logs.py`](scripts/explain_logs.py) ([result](parity/p2-explained.json)):
`CMakeError.log` and `CMakeOutput.log` by CMake's random try-compile names;
`configure.log` by those and step timings; `native-build.log` by make's `-j2`
progress percentages and line interleaving; `CMakeConfigureLog.yaml` by those
and by the one relative entry of this host's `PATH` (`./.cabal-sandbox/bin`),
which CMake resolves against its working directory — the repository for the
script, the output for the plugin. That entry changed no tool CMake chose
(`CMakeCache.txt` is identical). Pass 1's 7 add `CMakeRuleHashes.txt` and the
provenance record, whose digests are of files that carry the output root.
Every step of the harness exited 0 except the intended refusal
([commands and exits](parity/)). Two earlier runs of the same harness, at
`00ed15b3` (before the log-explanation step was added) and at `d9df130a`
(before the module rename), measured the same counts in every pass: 147
identical and the same 5 logs in pass 2. The
three host records (report, invocation, stderr log) exist only on the plugin's
side. The protected-output refusal ran end to end (exit 1, `plugin-exit`, the
libevent plugin's refusal naming `build/cycle19`), and
`git -C third_party/libevent status --porcelain --untracked-files=all` was
empty after every pass.

The script is deleted; `scripts/tests/test_retired_scripts.py` lists it, and
`docs/development/architecture.md` (crate map and entry points),
`crates/qcore-audit/README.md` and the user page name the subcommand. Frozen
reports (`remaining-g03-f`, `w44-quirin-audit-prepare`) and the wave-44
increment keep their text.

## 6. Tests

| File | Tests | What |
|---|---:|---|
| `crates/qcore-audit/tests/subcommand_plugins.rs` | 14 | through the `quirin-audit` binary, shell-script plugins: a plugin runs, its artifact, request, working directory and invocation record (path, SHA-256, protocol version, exit) are recorded; built-in shadowing refused (`plugin`, `prepare`, `current`; neither the host-option form nor the variable runs them) and listed; a missing plugin and a plugin only on `PATH` are usage errors listing the search path; a version mismatch, a timeout (its background child gone after the group kill), oversize standard output and standard error, a nonzero exit, malformed JSON and an artifact outside the output are each `Unknown`; uncertified `Safe` and `Unsafe` claims are `Pending` (and a certificate reference is `QP016`, still `Pending`); a failed plugin joined with a `Safe` host row is `Unknown`; no `Safe` or `Unknown` claim lowers an `Unsafe` host row; the environment does not leak (a secret and `CARGO_*` absent, only allow-listed names seen, `--plugin-env` passes one); a provider without a load path and a missing `--output` are refused |
| `crates/qcore-audit/src/subcommand_plugins.rs` | 7 | names vs options vs built-ins; `BUILTINS` equals `main.rs`; verbatim arguments; `PATH` never searched; no claim certified; the join never lowers a host verdict and a claim-less run has no verdict; audit-report rows as host verdicts |
| `crates/quirin-audit-plugin` | 7 + 4 + 1 | protocol round trip, version before shape, `Pending` proposals and unknown fields malformed, bad digests; `json_text` order and escapes; the harness running the example plugin, a failed response, a refused request version, a tampered artifact; the Cargo closure: the SDK links `qcore-schemas` alone of the workspace's crates, the libevent plugin only the SDK besides |
| `crates/quirin-audit-plugin-libevent` | 7 | flag parsing, `selected_units` (prefix, `-o`, `arguments`, the `.c` suffix), protected outputs, only `prepare` answered, the `audit` module's text and cache readers |

`compiler/tests/workspace_layering.rs` admits `qcore-audit` →
`quirin-audit-plugin` and pins the two new crates' workspace dependencies
(`qcore-schemas`; the SDK and `qcore-schemas`); `providers/Cargo.lock`
records `qcore-audit`'s two new dependencies, since the provider workspace
path-depends on it. The integration binary is in the test runner's `EXCLUSIVE` table (its timeout
row is a wall-clock budget).

## 7. Lean

`proof/QCore/SubcommandPlugins.lean`, audited by
`proof/AuditSubcommandPlugins.lean` (11 theorems), reuses
`QCore.VerdictSemigroup`'s `V`, `join`, `fold`, `Proposal` and `firewall`. A
run is `failure` or `completed claims`; a row is
`join host (join run.value (fold (claims.map firewall)))`.
`a_row_is_safe_only_with_accepted_certificates` (a row is `Safe` iff the host
is, the run completed, and every claim is a `Safe` whose certificate the
audit's checker accepted), `no_plugin_output_is_safe_without_an_accepted_certificate`,
`without_a_checker_only_an_unclaimed_safe_row_is_safe` (the command line),
`an_uncertified_claim_makes_a_safe_row_pending`, `a_failed_plugin_is_unknown`,
`the_join_never_lowers_the_host`, `no_claim_lowers_an_existing_unsafe`,
`adding_a_claim_never_lowers`, `a_row_is_monotone_in_the_host`,
`the_result_is_safe_only_when_every_row_is`, and `join_eq_safe_iff`. Only
`propext` and `Quot.sound` are used. Which run is a failure and that no Rust
path builds a certified claim are pinned by the tests of §6, not proved.

## 8. Corpus arbiter

No analysis code changed; the arbiter never reaches the new module.

* **BEFORE** (`run_audit.py private/asp/arbiter-before 37e1b82d --jobs 8`,
  exit 0) against `docs/reports/w45-container-root-reach/integration/artifacts/`
  ([identity](artifacts/before-identity.json)): diagnostics, chunk-contracts
  and source byte-identical, `verdict_bearing_identical: true`; the contracts
  sidecar differs, after the root alias, in exactly one leaf, the additive
  empty `refuted_contracts` section the brief names
  ([leaves](artifacts/before-contracts-leaves.txt)); source-effects differs by
  its checkout-bound digests, as every W45 card discloses.
* **AFTER** (`run_audit.py private/asp/arbiter-after e3d15aa5 --jobs 8`,
  release binary rebuilt at the tip, exit 0): all five artifacts
  (diagnostics, chunk-contracts, contracts, source-effects, source) are
  **byte-identical to BEFORE** (compared with `cmp`), and `audit.log` differs
  only in the line naming the output directory. Against the baseline it gives
  the same identity result as BEFORE ([identity](artifacts/after-identity.json),
  [leaves](artifacts/after-contracts-leaves.txt)); run records
  [after](artifacts/after-run.json) and [before](artifacts/before-run.json).

Functions 398 / 9 / 4,427, units 5 / 8 / 168 / 4, leak 34, Pending 0 —
unmoved, as predicted. No Safe → non-Safe move, no leak or violation leaf
added, so there is nothing to classify. The Fluent Bit slice was not run:
nothing on its route changed (prediction 2).

## 9. Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. corpus: no movement | held (§8) |
| 2. slice: not run, identity by construction | as stated |
| 3. parity: every prepared file byte-identical, only logs differ, each named | held: 147/152 identical, the 5 logs named (§5); one cause was not foreseen, the relative `PATH` entry resolved against the working directory, which reaches only `CMakeConfigureLog.yaml` |
| 4. no subcommand claim reaches `Safe`; uncertified is `Pending`, failure `Unknown`, nothing lowers a host verdict | held (§4, §6, §7) |
| 5. nothing retired; follow-ups registered | held: four registered (§11) |

## 10. Gate

Measured at `96f6b715` by [`scripts/gate.sh`](scripts/gate.sh) into the
worktree-private `private/asp/gate-96f6b715/`, every step's exit captured to its
own file, nothing piped, `CARGO_INCREMENTAL=0` ([gate.json](gate.json),
[Clang recording](clang-version.txt), this run's).

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| test | `python3 scripts/run_test_binaries.py` | 0 | **5,993 passed / 0 failed / 11 ignored**, 330 summaries over 318 units (`private/test-binaries/20260926T151551Z-1043578/`) |
| clean | `cargo clean --profile dev` | 0 | |
| fmt / clippy | `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings` | 0, 0 | |
| probe / followup | `reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py` | 0 | **207** audits (+1, `AuditSubcommandPlugins.lean`) |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 358 tests (after `build_matrix.py .`) |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | |
| packages | `providers/package.py build` (temporary directory) / `verify …/qcore.libevent` | 0, 0 | |
| provider Python | `python3 -m unittest discover -s providers/tests` | 0 | 15 tests |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 129 passed / 0 failed / 1 ignored, 43 summaries |
| provider fmt / clippy | | 0, 0 | |

**The discarded attempt** at `d9df130a` failed three ways, all repaired or
named: `compiler/tests/workspace_layering.rs` refused the new
`qcore-audit` → `quirin-audit-plugin` edge (§6); `providers/Cargo.lock` lacked
`qcore-audit`'s two new dependencies, failing the package build and the
provider test and clippy steps under `--locked`; and constraint 16's
load-sensitive row `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (its
500 ms wall-clock arm) failed in the exclusive phase. Re-run alone at a
1-minute load of 27–38 with sibling cards building, it passed **3 of 5**
(`private/asp/isolated/run1..5.log`), and it passed in the final gate. No code
on its path changed and nothing was weakened; this row is the one constraint
16 already names.

**`main` not merged.** Local `main` moved to `02e10476` (W45-MEMO-MEMORY
integrated) while the card ran. Merging it into this branch was refused by the
session's permission classifier, so the branch stays on `37e1b82d`; the
overlapping files are listed in the card's reply.

## 11. Follow-ups

Registered in `named_followups` (458 → 462) from [followups.json](followups.json)
by [scripts/update_ledger.py](scripts/update_ledger.py), then
`build_matrix.py .` (exit 0; gate figures carried, not supplied):
`W45-SUBCOMMAND-CERTIFICATE-ROUTE` (no checker for a subcommand claim's
certificate), `W45-SUBCOMMAND-CONFINEMENT` (containment, not a sandbox),
`W45-SUBCOMMAND-NON-UNIX` (the route refuses off Unix) and
`W45-JSON-TEXT-DUPLICATE` (the ordered JSON text writer exists twice). Nothing is
retired: no registered follow-up named the script or external subcommands.
