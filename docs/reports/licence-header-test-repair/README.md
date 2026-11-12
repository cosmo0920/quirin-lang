# LICENCE-HEADER-TEST-REPAIR

Card LICENCE-HEADER-TEST-REPAIR (2026-11-12) repairs the test fallout of
main's two licence-header commits, `838bd777d` (APACHE-2.0-SOURCE-HEADERS) and
`74105dd0e` (APACHE-2.0-CONTRACT-HEADERS). Both commits landed without the
build gate. On base `74105dd0e`, 33 tests in 11 binaries of `qcore-audit`,
`qcore-c-gateway` and `qcore-contract` failed with Clang present. The repair changes no analyser logic, no contract
semantics, no Lean definition or theorem and no frozen report. Each test now
reads the same bytes, digests and verdicts as before `838bd777d`. No verdict
or measurement moves. `quirin-audit measure` inputs are not prepared on this
host, so no corpus measurement was attempted, and none is predicted to move.

## Root cause and repair, per failing test

Repair classes follow the brief. (a) The test assumed a file opens with its
module header, so it now tolerates the leading licence notice and nothing
else. (b) A live pin was missed by the checksum refresh, so it is re-pinned
deliberately. (c) The header changed an input whose exact bytes are recorded
by frozen evidence that a test replays, so the file's pre-header bytes are
restored and it is excluded from the header.

| Test binary (failures at base) | Root cause (panic at base) | Repair |
|---|---|---|
| `qcore-audit::contract_dependencies_report` (2), `qcore-contract::hypothesis_bindings` (1) | The frozen `docs/reports/premise-evidence-contracts/binding-manifest.json` pins the SHA-256 of `proof/QCore/CjsonRepresentationPremises.lean` and `proof/AuditPremiseRefinement.lean`, so `binding: missing or mismatched independently supplied artifact` | (c): restore both Lean files |
| `qcore-audit::source_contract_config` (1) | `contract/profiles/cmetrics-counter/source-contracts.json` pins the digest of `providers/qcore-provider-cmetrics/contracts.json`. `74105dd0e` refreshed that inventory's file checksums but not this pin, so `contract inventory digest mismatch` | (b): re-pin `5b193802…` to `00be95c3…`, the current inventory digest. `73d59a294` set the precedent: a live configuration pin, not frozen evidence |
| `qcore-c-gateway::hash_contracts` (5) | `publication_fragment()` strips a leading `module contract;\n` from the production publication module, which now opens with the notice | (a): strip only the notice first (the integrator's hand-off patch, narrowed to the notice) |
| `qcore-c-gateway::neighbours` (5), `qcore-c-gateway::prod_contracts` (4) | `declarations_of()` asserts that each concatenated production module starts with `module contract;\n` | (a): the same, through the shared helper |
| `qcore-c-gateway::libc_reading_site_evidence` (3), `qcore-c-gateway::reading_site_domain` (1) | `docs/reports/libc-reading-site-evidence/bindings.json`, compiled into the gateway as `reading_site_evidence::BINDINGS`, pins three gateway sources and four Lean modules by SHA-256, so `reading-site: artifact reference mismatch` | (c): restore `crates/qcore-c-gateway/src/{legacy_condition_desugar,typed_field_access,variadic_nested}.rs` and `proof/QCore/{LibcReadingPremises,LibcReadingSiteEvidence,ScalarAccessorCondition,VariadicNested}.lean`. `3bb265c64` set the precedent by keeping the pinned `legacy_condition_desugar.rs` byte-identical |
| `qcore-contract::guarded_contract_adoption` (2), `qcore-contract::guarded_decomposition` (8) | The frozen `docs/reports/guarded-contract-adoption/adoption-plan.json` binds the exact whole source of librdkafka `cjson.qv` (`guarded bridge: version or exact whole-source inventory mismatch`). With that restored, the `premise-guarded-decomposition/applications/cjson.json` provenance and the plugin binding manifests still pinned `GuardedContractApplications.lean` and `PluginPremiseDecomposition.lean` / `AuditPluginPremiseDecomposition.lean` | (c): restore `cjson.qv` and those three Lean files |
| `qcore-contract::production_symbolic_adoption` (1) | The same plan and plugin-manifest pins (`:360`, `:138`). After those, `quant-contract-hypotheses/plugin/migration.json` pins six librdkafka modules and libc `errno.qv` (`unregistered source change`). `premise-symbolic-adoption/inventory-before.json` then requires every original byte of the eight inventory files, including fluent-bit `logging.qv` (`all old bytes must remain`) | (c): restore librdkafka `{client,kafka_mem,logging,memory,null_deref,queue_destructor}.qv`, libc `errno.qv` and fluent-bit `logging.qv` |
| `qcore-audit::configured_cli` (1, `pic_build_flag_…`) | Not reproduced. It passes at base on this host (20/20 in 0.59 s; the other card's run took 14.41 s). Its input is an inline C string and it reads no tracked file. This is a host or load observation, not header fallout | none |
| `compiler::task_route_tsan` (3) | Host-only: ThreadSanitizer frames are not symbolized with the borrowed LLVM 21 toolchain, and the test reads no file the headers changed | none (see the gate section) |

The other card's run panicked `production_symbolic_adoption` at
`tests/support/tracked_files.rs:102`, `git tracks no file under providers`.
That panic did not occur here. In this worktree `git ls-files` works and the
test reaches the header assertions above, so the earlier panic belongs to that
worktree's git environment.

Restored files: 21 in all. Nine are Lean, three Rust and nine contract
modules. Each is checked out from `838bd777d^` (Lean, Rust) or `74105dd0e^`
(`.qv`), so its bytes equal the pre-header blob. That is the exact inverse of
the header insertion, which both commits state reproduces the prior content.
`providers/contracts.py sync` (exit 0, twice) rewrote only the `sha256` lines
of the restored modules in `qcore-provider-librdkafka/contract-inventory.lock`
and `contracts.json`, and in the `quirin-provider-libc` and
`quirin-provider-fluent-bit` locks, back to their `74105dd0e^` values. No
clause count, declaration, view or load-path entry changed. The test-side
helper is `strip_licence_notice` in
`crates/qcore-c-gateway/tests/support/text.rs`. It skips a block only when the
text opens with `// SPDX-License-Identifier: `, and then only that `//` block
and one blank line, so a module header behind any other text still fails the
caller's assertion.

The exclusion is recorded here, beside the commits' own exclusions ("pinned
contracts and fixtures … and frozen report evidence"). A tracked file whose
exact bytes are bound by a frozen manifest, sidecar, migration record or
compiled-in binding manifest stays header-free until that evidence is
re-recorded by a card that owns it. The census below and the failing tests
identify the 21 files above as the live instances.

## Sweep beyond the listed tests

- Census ([`pin_census.py`](pin_census.py), run on `74105dd0e`): every file
  modified by the two commits was hashed before the change,
  raw and newline-normalized, and every tracked file at `74105dd0e` was
  searched for those digests. There were 553 source files with hits, almost all
  in historical report records that no test replays. The hits read by code or
  tests were the manifests above plus three more. (1)
  `contract/evaluations/cmetrics-counter-configured/evaluation.json` pins a
  revision-tagged historical copy, so it was left alone. (2)
  `crates/qcore-core/tests/data/o01_records.txt` and (3)
  `docs/reports/remaining-o01/logs/records.txt` record a historical
  observation; their readers (`test_file_integrity_fixtures.py`,
  `chunk_integrity.rs`) passed in the full gate.
- The full Rust suite with Clang, the Python suite and the reevaluation
  examples are the arbiter for anything the census could not see, such as
  line or span pins. Results are below.

## Gate (full standing gate plus provider gate, Clang 21.1.8)

GATE_SECTION

## Follow-ups

- `LICENCE-HEADER-PINNED-INPUT-EXCLUSIONS` (registered): the 21 restored
  files carry no licence notice because frozen evidence binds their bytes.
  Giving them the notice needs the owning evidence re-recorded (new manifests
  or bridge plans, a new symbolic-adoption inventory) by a card that can
  re-derive it, or a header-aware digest convention. Neither is a test edit.
- The header commits' tooling lacked a pin census. A future bulk text edit
  should run the digest census above (old blob digests against tracked files)
  before landing.
