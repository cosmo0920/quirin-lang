# Standalone first-party provider packages

This separate Cargo workspace builds five executable packages:
`qcore.libevent`, `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, and
`qcore.librdkafka`. Each artifact crate depends only on the published
`qcore-schemas` package and serialization libraries. The shared transport
source introduces no Rust crate dependency. The conformance harness depends
on the real verifier host and is never linked into an artifact.

These are **Unknown-only executable packages**. They reproduce the existing
first-party protocol-v1 adapter responses; they do not extract the specialized
Libevent/Monkey, ChunkIO, CMetrics, CTraces or librdkafka source/model checkers.
Every supported v2 capability explicitly returns Unknown without evidence.
Capabilities and theorem-family labels in the manifests remain routing
metadata, not claims of implemented witness generation. Existing in-process
and Cycle 19 routes remain required. W5 semantic extraction and aggregate
Cycle 20 acceptance remain open.

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
Valid v2 requests return explicit semantic-evidence-unavailable diagnostics.

The existing manifest target wildcards are preserved for routing parity.
A request still selects a concrete nonempty revision, profile, platform and
backend. Wildcard routing is never source/revision verification or safety
coverage; these packages always return Unknown.

Extract a verified archive into a versioned directory and register its
absolute package path using the existing host API:

```rust,ignore
let provider = qcore_verifier_host::ProcessProvider::from_package(package_path)?;
registry.register(std::sync::Arc::new(provider))?;
```

Explicit registration avoids silently replacing the default in-process
first-party registry. For upgrade, verify a newly built version in a separate
directory, then select it in a fresh registry. For rollback, select the
previous directory and its retained build record. `qcorec` default provider
selection is unchanged. The standalone `package.py verify` command and the
binary's `--verify-package` mode check an installed package; no new protocol
message or implicit network lookup is involved.

The process-host containment remains its existing Unix process-group,
deadline/output behavior; package creation does not add an OS sandbox or a
new memory-limit enforcement mechanism. Core/host build and tests remain
independent of this workspace and of installed provider packages.
