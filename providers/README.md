# Standalone first-party provider packages

This separate Cargo workspace builds five executable packages:
`qcore.libevent`, `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, and
`qcore.librdkafka`. Each artifact crate depends only on the published
`qcore-schemas` package, serialization libraries and the workspace library
[`quirin-provider-shared`](quirin-provider-shared/README.md), which itself
depends only on `qcore-schemas`, serialization and the five data-only shared
contract crates listed under [contract ownership](#contract-ownership). That library holds the
shared protocol runtime and the schemas-only capability evaluators, and each
provider's `main` names exactly the evaluators it offers. The conformance
harness depends on the real verifier host and is never linked into an artifact.

All protocol-v1 responses remain Unknown. ChunkIO 0.5.0 and CMetrics 0.2.0
compute Boolean-field contract witnesses using a shared schemas-only statement
interpreter. Integer-result source uses `qcore.c_boolean_field_contract.v2`;
void-result source uses `.v3`, an explicit unit ABI and two field prestates. The
trusted receiver binds source/build/profile/policy identities, compares the
computed paths and independently evaluates the contract. Unit contracts cannot
mention a scalar `result`. Unsupported requests remain Unknown.

ChunkIO 0.5.0 additionally reads bounded unsigned unit counters under
`qcore.c_counter_contract.v1`. The request transports one typed path
projection with no derived result; the process reports which field the path
updates, in which direction and over which unsigned storage type. The trusted
receiver recomputes the same reading from its own import and evaluates the
pinned counter contract itself, so an agreeing proposal discharges nothing on
its own. Caller storage, exclusive access, the entry range and the unsigned
C/`usize` ABI binding remain the contract's declared premises. See the
[capability transport evidence](../docs/reports/remaining-p04/README.md).

Libevent, CTraces and librdkafka remain Unknown-only. Clang import and helper
validation stay in the trusted gateway. The CMetrics configured audit checks
`cmt_counter_allow_reset` under a separately declared analysis build; it does not
establish whole-unit custody, counter behavior, caller validity or synchronization.
See the [unit-provider evidence](../docs/reports/w5-cmetrics-unit-provider/README.md).

## Contract ownership

User-defined contracts live in the user's own crates, outside this
repository; [`quirin-provider-plugin`](quirin-provider-plugin/README.md) is
their SDK (card W45-USER-CONTRACT-PLUGINS). It owns no contracts and is not on
the load path.

Provider crates own their supplied Quirin contracts under `contracts/`, beside
`src/` and `provider-template.json`. `contracts.json` records their package paths
and SHA-256 values. The ownership map is `providers/contracts.py`.

| Crate | Owned contract families |
|---|---|
| `qcore-provider-chunkio` | `chunkio/` and `fluent_bit/busy_state.qv` |
| `qcore-provider-cmetrics` | `cmetrics/` |
| `qcore-provider-ctraces` | `ctraces/` |
| `qcore-provider-libevent` | `monkey/` |
| `qcore-provider-librdkafka` | `librdkafka/` |

Edit the provider-owned source, then synchronize and check:

```sh
python3 providers/contracts.py sync
python3 providers/contracts.py check
cargo test -p qcore-compiler --test qcore_contract_corpus
```

`sync` updates each provider's `contracts.json` and `contract-load-path.json`,
and each load-path crate's `contract-inventory.lock` (card CONTRACT-LOCKS):
per module its SHA-256, its `extern fn` declarations with their elaborated
clause counts, its `contract inherit` edges and its premise manifest, plus
the crate's views, computed by `cargo run -p qcore-contract --example
contract_inventory`. The corpus tests read these locks instead of literal
tables, so a contract change commits its regenerated lock beside it and the
review reads the lock diff. `check` refuses a lock that differs by a byte.
`check` compares its exact path and SHA-256 inventory with the canonical
`contracts/` tree, and the load path with its sources. Each load-path crate
record carries a `resource_view`: the owned modules the resource-contract
loader accepts, which `quirin-audit --provider NAME` selects. `sync` computes it
by running the loader (`cargo run -p qcore-audit --example
provider_resource_views`, so it needs the repository workspace), and `check`
refuses a view that names a module the loader refuses or omits one it accepts
(card W44-PROVIDER-RESOURCE-VIEW). Each record also carries a `bounds_view`,
the owned modules the bounds contract v1 loader accepts (disjoint from the
resource view), which the audit routes load when `QCORE_BOUNDS_PROPERTY`
selects the bounds property; their pins are the crate's
`bounds-contracts.lock`, written by `quirin-audit pin --bounds --source-root
<checkout>` after review (card CHUNKIO-META-LEN-BOUNDS-CONTRACTS). Each record
also carries an `outer_view`, the owned modules the outer contract v1 loader
accepts (disjoint from both other views, loaded together per crate), which the
audit routes load when `QCORE_OUTER_PROJECTION` selects the runtime projection;
their pins are the crate's `outer-contracts.lock`, written by `quirin-audit pin
--outer --source-root <checkout>` (card OUTER-CONTRACT, gateway rule 31). Both actions
refuse symbolic links, invalid or unowned paths, and any reintroduced
`contract/qcore/` legacy tree. Intentional declaration/clause changes still
require the corpus pin updates.

Shared language, support-library and application policies without a provider
owner live in six library crates of this workspace (five data-only, plus the
shared runtime crate):

| Crate | Shared contract modules |
|---|---|
| [`quirin-provider-libc`](quirin-provider-libc/README.md) | `libc/` (16): the only home for libc contracts, enforced by the placement test in `scripts/tests/test_contract_corpus.py` |
| [`quirin-provider-cfl`](quirin-provider-cfl/README.md) | `cfl/` (6) |
| [`quirin-provider-mpack`](quirin-provider-mpack/README.md) | `mpack/` (1) |
| [`quirin-provider-msgpack`](quirin-provider-msgpack/README.md) | `msgpack/` (1) |
| [`quirin-provider-fluent-bit`](quirin-provider-fluent-bit/README.md) | `fluent_bit/` (16, one of checked specifications); the first crate with a contract-use Cargo edge (to `quirin-provider-libc`), whose `resource_memory.qcore` and `resource_string.qcore` inherit libc contracts |
| [`quirin-provider-shared`](quirin-provider-shared/README.md) | `language/` (1), `library/` (3); re-exports the five above as one bundle |

Each exposes its modules as `CONTRACTS` (logical path, source text), pinned to
its `contracts/` tree by a unit test. The logical corpus is assembled from the
roots listed in `contract/corpus.json`; provider contracts remain canonical in
their artifact crates and are not copied into a compatibility tree.

## Consuming contract data without a process

The existing `contracts.json` plus `contracts/` tree is independently consumable
with `qcore_contract::bundle::ContractBundle::load(manifest, expected_sha256,
expected_owner)`. Supply the inventory hash from the consumer's reviewed
configuration; do not derive the expected hash from the input being checked.
Copy that inventory and tree together to relocate the data. No executable,
plugin manifest or provider registration is required by this data loader.

The [source-contract service version 2](../docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#version-2-pinned-contract-bundle-selection)
selects a bundle object instead of a policy filename. Its CMetrics configuration
is the initial migrated consumer. After an intentional inventory change, update
that consumer's manifest pin deliberately. Digest equality identifies declared
bytes, not authenticity or satisfied predicates; the existing interpreter and
source-evidence gates still decide those obligations. The package verifier's
executable-to-inventory binding remains a separate check.

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
hash. Reproducibility is relative to that recorded source, toolchain and
lockfile; no cross-toolchain or cross-platform byte identity is claimed.
No provider binary is checked into the source tree.

The bytes do not depend on where the checkout, `CARGO_HOME`, the target
directory or the toolchain lie (card
[PACKAGE-BYTES-DEPEND-ON-CHECKOUT](../docs/reports/package-bytes-depend-on-checkout/README.md)).
Cargo hashes a path package by its path relative to the workspace root, or by
its absolute path when it lies outside, and `qcore-schemas` lies outside
`providers/`. So the build compiles a staged copy under the target directory,
`<target>/qcore-package-source/`: its `Cargo.toml` is this workspace's manifest
with members and path dependencies rebased under `providers/`, beside a copy of
`Cargo.lock` and of every path package at its repository-relative path. Unchanged
files keep their modification times, so a reused target rebuilds only what
changed. The stage, the target directory, `CARGO_HOME` and the toolchain's
`rust-src` are remapped to `/qcore`, `/target`, `/cargo` and
`/rustc/<commit-hash>`, and the build refuses an executable that still contains
any of those paths, the repository's or the home directory.
`python3 providers/package.py reproduce --output DIR` checks it: it builds into
`DIR`, builds again by running the `package.py` of a copy of the checkout
(without `docs/` and the submodule) at another path with another `CARGO_HOME`
holding only the locked registry files, and fails unless every output file,
`packages.json` included, is byte-identical. CI runs it in place of `build`.

Each package contains `provider`, `verifier-plugin.json`, `contracts.json` and
the provider-owned `contracts/` tree. Its
`package_digest` is the executable's SHA-256. The executable embeds its
manifest template between the markers `<<<qcore.provider-template.v1:begin>>>`
and `<<<qcore.provider-template.v1:end>>>`, exactly once, so a host can read it
back out of the executable's bytes. Both the running package and the read-only
CLI verifier compare every installed manifest field against that template,
substituting only the recomputed executable digest. Package ID/version,
capabilities, limits (including every `limits.additional` kind an enforced
execution profile installs), targets, filesystem inputs, evidence versions and
trust labels cannot be changed independently of the binary; a divergence is
refused by field name, and an absent, duplicated, truncated or unparseable
envelope is refused distinctly. The all-zero digest in a source
`provider-template.json` is a build placeholder and is never an installed
package digest. Verification establishes this binding, not a signature or
source authenticity: whoever can rebuild or forge the executable controls its
embedded template too. Retain the archive hash from a trusted build record.

The new executable also embeds a separate `qcore.provider-contracts.v1`
inventory envelope. `package.py verify` compares the installed `contracts.json`
with that embedded inventory, checks the exact recursive contract file set and
every SHA-256, and refuses missing, changed, extra or symbolic-link assets.
`--contract-manifest` prints the executable's inventory. The inventory identifies
supplied bytes; it is not a signature or a claim that every shipped policy is
implemented by that process provider.

The existing core `plugins verify`/registry gate continues to check executable
and plugin-manifest identity. Contract-bundle checking is the additional
`package.py verify` gate; consumers still select contract paths explicitly and
retain their existing policy/trust checks. For example, the CMetrics policy is
available at `<package>/contracts/cmetrics/counter_reset.qv`. Legacy packages
without a contract bundle remain supported.

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
let provider = qcore_verifier_host::verify_executable_package(package_path)?;
registry.register(std::sync::Arc::new(provider))?;
```

`verify_executable_package` is the checked route: it applies path containment,
the declared digest and the embedded-template binding, and it keeps the
executable's descriptor open so every later spawn can confirm the file has not
been substituted (card `P01-F3`). `ProcessProvider::from_package` now delegates
to it, so there is no unverified way to build a registrable provider:
`ProcessProvider` is produced by verification alone.

A caller that genuinely wants an unchecked load asks for it by name, and gets a
type that says so — `UnverifiedProvider`, which is not a `VerifierProvider` and
therefore cannot be registered. The conformance harness uses it to drive
deliberately mutated packages. `UnverifiedProvider::into_verified` is the one
bridge back, and it runs the same gate.

The identity re-check narrows the window between verification and dispatch; it
does not close it, and it is not signature verification. An actor who can
produce the executable controls its embedded template too. Keep the operational
rule: retain the archive hash from a trusted build record.

Explicit registration avoids silently replacing the default in-process
first-party registry. For upgrade, verify a newly built version in a separate
directory, then select it in a fresh registry. For rollback, select the
previous directory and its retained build record. CLI configuration is explicit and separate from the default registry:

```sh
quirin plugins verify /absolute/path/qcore.libevent
quirin plugins enable /absolute/path/qcore.libevent --config ./plugins.json
quirin plugins list --config ./plugins.json
quirin plugins inspect qcore.libevent --config ./plugins.json
quirin check --plugins ./plugins.json main.qcore
quirin plugins disable qcore.libevent --config ./plugins.json
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
manifest syntax, resolved path containment, the declared executable hash, and
the binding of every other declaration to the template embedded in those same
bytes. It establishes a snapshot identity, not a signature, runtime
compatibility or source authenticity. `package.py verify` additionally executes
the first-party `--verify-package` self-check, which reaches the same verdict
from inside the process. Keep packages protected
from mutation between verification and dispatch; no immutable-file or OS sandbox
guarantee is added. `quirin` default provider selection is unchanged. The standalone `package.py verify` command and the
binary's `--verify-package` mode check an installed package; no new protocol
message or implicit network lookup is involved.

The process-host containment under the default `unenforced-legacy` execution
profile remains its existing Unix process-group, deadline/output behavior;
package creation adds no OS sandbox. A host may instead select the
target-qualified `linux-rlimit-v1` profile, which installs the manifest's
declared `limits` with `setrlimit` before `exec` and refuses any package
declaring a limit kind it cannot enforce; see
[enforced process limits](../docs/reports/remaining-p02/README.md). Neither of
those two profiles confines the filesystem or the network.

A host may instead select the target-qualified `linux-confined-v1` profile,
which installs everything `linux-rlimit-v1` installs **and** two kernel
confinement mechanisms in the child before `exec`: a Landlock ruleset admitting
only the package root and the paths an explicit `ConfinementPolicy` names, and
`unshare(CLONE_NEWUSER | CLONE_NEWNET)` for a fresh network namespace. It has no
default policy — a caller that supplies none is refused before spawning — and a
policy handed to a non-confining profile is refused in the other direction. All
five shipped packages answer identically under it; see
[filesystem and network confinement](../docs/reports/remaining-p03/README.md)
for the mechanisms, the measured fixtures and the exact trust boundary. This is
confinement of two named resource classes, not a general OS sandbox: no seccomp
filter and no cgroup is installed, and every non-completed provider outcome
remains a scoped `Unknown` under `QP006`. Core/host build and tests remain
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
  cargo run --quiet -p qcore-audit -- current \
  --checkout ../fluent-bit --profile-directory contract/profiles/fluent-bit \
  --monkey-database build/cycle19/monkey-build/compile_commands.json \
  --monkey-source-report build/cycle19/reports/source-effect-source.json \
  --report build/cycle19/reports/source-effect-diagnostics.json
```

The configuration maps sealed model-library identities to provider IDs and policy
paths. Relative paths resolve from its directory. `QCORE_CHUNKIO_PACKAGE` remains
a compatible package-path alias using the default bindings; setting both selectors
rejects. Neither selector changes `quirin` plugin configuration. The audit loads
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

Bindings may now select separate-TU constant metadata helpers with `frame_helpers`.
The generic loader checks exact compile-entry selection, supported options,
record/model identity and disjoint writes before the existing process executes
`FrameWrites`. The sample fixture exercises this complete path. No provider binary,
manifest, wire shape or schema version changes for helper selection.
Provider diagnostic codes now require the complete selected plugin ID plus one
1–128-byte ASCII local component; dotted/control/empty suffixes reject at the host.
See [helper evidence](../docs/reports/w5-configured-frame-helpers/README.md) and
[diagnostic attribution](../docs/reports/w7-diagnostic-attribution/README.md).
