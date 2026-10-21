# Reproduce the bounded source witness

Use the exact input paths, source revision, four source hashes and selected compile entries in `input-inventory.json`. The original source tree and compile database are read-only. This report's Rust adapter accepts no caller-specified producer or unchecked checked-receipt constructor.

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test -p quirin-source-analysis --lib plugin_source_witnesses -j1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo clippy -p quirin-source-analysis --lib --example plugin_source_witnesses -j1 -- -D warnings
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo run -p quirin-source-analysis --example plugin_source_witnesses -j1 -- /absolute/fresh/private/output
```

The example makes one actual observation and an independent complete observation/replay. It then runs refusal controls against actual presented observations. Its output includes exact canonical Context, checked source facts, bounded four-operator flow results and control names. Each producer directory retains original ASTs, preprocessed inputs, dependency files, generated unchanged-body projections, actual DSOs/executables and every compiler/native command with exit. The host compiler is `/usr/bin/clang`; its bytes/banner/target and all527 actual dependency-byte identities are bound. Rust sets `SOURCE_DATE_EPOCH=1761004800` for this explicitly reproducible local fixture build. That environment is not a declaration about deployed plugin builds.

For only this new Lean module and its audit, from `proof`:

```sh
lake env lean -o .lake/build/lib/lean/QCore/PluginSourceWitnesses.olean QCore/PluginSourceWitnesses.lean
lake env lean AuditPluginSourceWitnesses.lean
```

The full standing/provider gate, packaging and six paired production routes belong to the integrator's final joint source campaign. This focused reproduction is not their substitute. `focused-validation.json` lists actual scoped commands/exits and retained failures; `actual-native-commands.json.gz` contains the two actual producer command streams. Earlier native/control attempts remain private under `private/plugin-premise-source-witnesses/` and selected failure logs are archived in `focused/`.
