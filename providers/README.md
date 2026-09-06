# Standalone first-party provider packages

This separate Cargo workspace builds five executable packages:
`qcore.libevent`, `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, and
`qcore.librdkafka`. Each artifact crate depends only on the published
`qcore-schemas` package and serialization libraries. The shared transport
source introduces no Rust crate dependency. The conformance harness depends
on the real verifier host and is never linked into an artifact.

All protocol-v1 responses remain Unknown. ChunkIO 0.3.0 additionally computes and proposes
source-derived Boolean-field contract witnesses for the explicit
`qcore.c_boolean_field_contract.v2` profile. The trusted receiver binds the typed
source/build snapshot, checks the computed paths and independently evaluates the contract. Requests contain a version-2 typed statement program and no expected paths. Other v2
requests remain Unknown. This bounded function property does not establish
whole-unit custody, caller provenance, general lifecycle or synchronization.
The other four packages remain Unknown-only. Clang import and helper validation stay in the trusted gateway; broader provider source analysis and aggregate Cycle20 acceptance are still open. See the
[statement execution integration](../docs/reports/w5-provider-statement-execution/README.md).

## Build, verify and reproduce

On a Unix host with Rust, Python 3 and the locked Cargo dependencies cached:

```sh
python3 providers/package.py build --output /tmp/qcore-packages
python3 providers/package.py verify /tmp/qcore-packages/qcore.libevent
cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline
cargo fmt --manifest-path providers/Cargo.toml --all -- --check
cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings
```

The builder uses release mode, a separate target directory, a locked offline
build, source-path remapping and deterministic tar/gzip metadata. It refuses
to overwrite an output directory. `--target-dir` selects a fresh build cache
for reproducibility checks. The output contains five package directories,
five deterministic `.tar.gz` archives, and `packages.json` with executable,
manifest and archive SHA-256 values plus the compiler identity and lockfile
hash. Reproducibility is relative to that recorded source, target, toolchain
and lockfile; no cross-toolchain or cross-platform byte identity is claimed.
No provider binary is checked into the source tree.

Each package contains `provider` and `verifier-plugin.json`. Its
`package_digest` is the executable's SHA-256. The executable embeds its
manifest template and verifies every installed manifest field against it,
substituting only the recomputed executable digest. Package ID/version,
capabilities, limits, targets, evidence versions and trust labels cannot be
changed independently of the binary. The all-zero digest in a source
`provider-template.json` is a build placeholder and is never an installed
package digest. Verification establishes this binding, not a signature or
source authenticity; retain the archive hash from a trusted build record.

## Invocation and installation

Normal invocation is one request on stdin followed by EOF, and one response
on stdout. The wire framing is unchanged: a four-byte big-endian payload
length followed by strict JSON, protocol version 1. Evidence versions 1 and 2
select their existing schema types. Unsupported versions, mismatched IDs,
malformed identities, nonconcrete target selectors, unsupported targets or
capabilities, oversized/truncated frames, unknown fields and trailing frames
fail with a nonzero exit and no response. The host reports these failures as
Unknown. Valid v1 responses match the built-in diagnostic and no-evidence
response exactly, including when the submitted facts are empty or false.
V2 requests outside the explicit ChunkIO source-field path return semantic-evidence-unavailable diagnostics.

Existing wildcard routes are retained; ChunkIO adds a bounded `fluent_bit` source-field target and explicit C dialects.
A request still selects a concrete nonempty revision, profile, platform and
backend. Wildcard routing is never source/revision verification or safety
coverage; every admitted source-field result still needs trusted replay.

Extract a verified archive into a versioned directory and register its
absolute package path using the existing host API:

```rust,ignore
let provider = qcore_verifier_host::ProcessProvider::from_package(package_path)?;
registry.register(std::sync::Arc::new(provider))?;
```

Explicit registration avoids silently replacing the default in-process
first-party registry. For upgrade, verify a newly built version in a separate
directory, then select it in a fresh registry. For rollback, select the
previous directory and its retained build record. CLI configuration is explicit and separate from the default registry:

```sh
qcorec plugins verify /absolute/path/qcore.libevent
qcorec plugins enable /absolute/path/qcore.libevent --config ./plugins.json
qcorec plugins list --config ./plugins.json
qcorec plugins inspect qcore.libevent --config ./plugins.json
qcorec check --plugins ./plugins.json main.qcore
qcorec plugins disable qcore.libevent --config ./plugins.json
```

Enable pins the complete manifest, including the executable digest, under its
plugin ID. Re-enable a reviewed new package path to upgrade or roll back.
Disabled entries do not enter the registry and can remain configured after
removing the package directory. An explicit config supplies the entire process
provider registry; an empty selection supplies no providers. Missing, malformed
or changed enabled packages fail before compilation, without default fallback.
Config updates use a sibling temporary file and rename. They are local edits,
not a distributed or concurrent-writer transaction protocol.

The CLI verifier is read-only and does not execute the package: it validates
manifest syntax, resolved path containment and the declared executable hash.
It establishes a snapshot identity, not a signature, runtime compatibility,
embedded-template binding or source authenticity. `package.py verify` additionally
executes the first-party `--verify-package` self-check. Keep packages protected
from mutation between verification and dispatch; no immutable-file or OS sandbox
guarantee is added. `qcorec` default provider selection is unchanged. The standalone `package.py verify` command and the
binary's `--verify-package` mode check an installed package; no new protocol
message or implicit network lookup is involved.

The process-host containment remains its existing Unix process-group,
deadline/output behavior; package creation does not add an OS sandbox or a
new memory-limit enforcement mechanism. Core/host build and tests remain
independent of this workspace and of installed provider packages.

The conformance workspace also contains `callback_proposal_writer`, a schema-only
record-exchange fixture. Its paired test uses the trusted declared-reactor
consumer and rejects authority/identity/coverage mutations. It is not one of the
five packaged providers and speaks no plugin protocol. See
[callback record evidence](../docs/reports/w5-reactor-callback-records/README.md).

For the real field-contract audit, select the checked-in configuration and durable
verified package. On first setup, build that package set with
`python3 -B providers/package.py build --output build/cycle19/provider-packages`:

```sh
mkdir -p build/cycle19/reports
python3 -B providers/package.py verify build/cycle19/provider-packages/qcore.chunkio
QCORE_SOURCE_CONTRACTS="$PWD/contract/profiles/fluent-bit/source-contracts.json" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit build/cycle19/monkey-build/compile_commands.json \
  build/cycle19/reports/source-effect-source.json \
  build/cycle19/reports/source-effect-diagnostics.json
```

The configuration maps sealed model-library identities to provider IDs and policy
paths. Relative paths resolve from its directory. `QCORE_CHUNKIO_PACKAGE` remains
a compatible package-path alias using the default bindings; setting both selectors
rejects. Neither selector changes `qcorec` plugin configuration. The audit loads
one policy snapshot and verified registry before processing units. Unsupported
functions remain in the `.source-effects.json` sidecar; explicit missing,
incompatible or rejected providers fail without fallback. Direct evaluation
remains available when neither selector is set.

`qcore_audit::source_contract::SourceContractRegistry` exposes the same service to
other audit drivers. The `sample_boolean_provider` conformance fixture exercises
a different C codebase through Clang import, explicit package binding, a real
schemas-only process and trusted replay. Its declared profile maps one Boolean
field onto an existing logical axis; it does not establish arbitrary record or
lifecycle semantics. Process tests are Unix-bound. See the
[configuration format](../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g08-source-contract-service-configuration-v1)
and [parity evidence](../docs/reports/w5-configured-source-contracts/README.md).
