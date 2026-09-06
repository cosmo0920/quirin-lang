# W2 H6: provider manifest discovery

Date: 2026-09-07. H6 is implemented independently of provider process extraction.
Five first-party metadata records now come from the validated W2 profile JSON
packages. The previous first-party Rust manifest construction tables are removed
only after the W2 equality guard and the frozen `baseline-manifests.json`
confirm field-for-field parity. Registry order is lexical plugin-id order.

`compiler/src/first_party.rs` discovers source-set directories, loads profiles
atomically, rejects duplicate providers across sets and registers each validated
manifest through the existing built-in or process adapter. Executable roots are
bound to the directory of the actual manifest read by `qcore-contract`, without
re-reading or guessing their location. No provider-specific dispatch table remains
in the host registration path. The five shipped entrypoints remain built-ins.

Explicit `load_first_party_manifests` and `load_first_party_registry` APIs return
load errors. CLI `plugins` commands expose those errors. Compatibility default
helpers return an empty manifest set/registry on invalid discovery, making every
requested source scope report its provider absent; there is no fallback to
compiled metadata. Sources without provider scopes remain package independent.
Provider-specific `manifest()` and `provider()` helpers now return `Result`.

## Preserved trust boundary

Manifest capabilities remain routing declarations, not verified support. All
five built-in v1 adapters still return Unknown for every capability; the v1 Safe
firewall refusal is unchanged. Existing process framing/containment/firewall
logic is untouched. This closes H6 metadata routing, not W5 standalone provider
extraction or W3 semantic dispatch.

## Validation and parity

- Full 185-TU Cycle19 resource diagnostics and Monkey source JSON are
  byte-identical to the prior main/gateway baseline, with zero verdict changes.
  `parity.json` binds hashes and counts; compressed output and audit log are
  archived here. Audit exited 0.
- The five manifest records match the independently frozen pre-discovery data;
  existing provider tests preserve every built-in capability outcome.
- New tests exercise runtime metadata changes, stable ordering, malformed or
  missing packages, duplicate ids, exact package directories and CLI failures
  without compiled fallback.
- All standing gates pass: **478 workspace tests**, fmt, clippy, both
  reevaluation examples (exit 0), pinned submodule init and Lean (52 jobs).
  Logs are archived alongside this report.

Command for the pinned audit:

```sh
cargo run -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/h6-source.json /tmp/h6-diagnostics.json
```

Typed Monkey imports, Boolean field routing, configured audit orchestration and
actual provider crate/process extraction remain separate work. Raw production
units remain unpromoted under the Cycle19 evidence gates.
