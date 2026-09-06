# Schema 1.0 compatibility and package evidence

The public schema crate is now **qcore-schemas 1.0.0**. Its compatibility policy
requires a major version when public fields or serde shapes change. The earlier
Boolean source-input transition removed the result-bearing `summary` field and
introduced required version-2 statement-program fields while the crate still
claimed 0.2.0. This change records that breaking event explicitly; it does not
change analysis or witness admission semantics.

Every workspace consumer and standalone provider now requires 1.0.0. Three
tracked lockfiles pin it: the root workspace, the provider workspace and the
isolated W3 process fixture. The combined workspace gate exposed the initially
missed fixture lock; it was regenerated offline with only the schema version
changed before the successful gate rerun. `lock-identities.json` records all
three final lock identities.

Protocol version 1, evidence versions 1 and 2, Boolean input/profile version 2,
and provider package versions retain their separate meanings. ChunkIO package
0.3.0 already identified the independent statement-execution change. Rebuilding
against the corrected crate identity changes executable/package digests, which
are recorded rather than hidden behind the old artifact identity.

## Publishable crate and consumer checks

`cargo package -p qcore-schemas --allow-dirty --offline` produced and verified
the schema 1.0.0 archive. Locked, offline checks passed for all targets in both
workspaces. A separate consumer compiled against the extracted packaged crate,
round-tripped the new statement input, and rejected an attached old `summary`
field. The same consumer requiring `^0.2.0` failed Cargo version selection as
expected. This is compatibility evidence, not a registry publication.

The archived consumer fixtures and reproducible script retain both expected
outcomes. The script uses a fresh temporary directory, rebuilds the package,
and adjusts the fixture dependency path to the current checkout:

```sh
python3 -B docs/reports/schema-1-compatibility/reproduce_consumers.py
```

`consumer-replay.json`, the consumer/package logs and `validation.json` record
successful execution of this script. The Cargo package command reports only
optional missing documentation/homepage/repository metadata; archive verification
succeeds.

## Independent executable builds

All five providers were built from scratch into separate target caches and
initial output directories:

```sh
python3 -B providers/package.py build \
  --output build/cycle19/schema1-packages-a \
  --target-dir /tmp/qcore-schema1-packages-target-a
python3 -B providers/package.py build \
  --output build/cycle19/schema1-packages-b \
  --target-dir /tmp/qcore-schema1-packages-target-b
```

Build output directories must be absent; the packager refuses overwrite.
Each package in each set passed executable/manifest verification. The two
indexes are byte-identical, including all five executable, manifest and archive
hash triples, Rust toolchain identity and provider lock digest. See
`packages-a.json`, `packages-b.json`, `package-reproducibility.json` and the
archived manifests. These are reproducible local builds, not signatures or an
operating-system sandbox claim.

The audited A set was subsequently promoted by two directory renames:

1. `build/cycle19/provider-packages` →
   `build/cycle19/provider-packages-schema0.2` preserves the previous set.
2. `build/cycle19/schema1-packages-a` →
   `build/cycle19/provider-packages` installs the exact audited set.

`package-promotion.json` records the before/after index hashes. All artifact
hashes were rechecked before promotion; no symlinks or temporary/build-output
paths occur in the durable index. The independent B set remains at
`build/cycle19/schema1-packages-b`. Renaming did not rebuild the artifacts or
require a second audit.

## Pinned audit parity

The real A ChunkIO process was selected explicitly for the full Fluent Bit audit,
using the durable Monkey source/configuration inputs:

```sh
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages/qcore.chunkio \
  cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  /tmp/qcore-schema1-source.json /tmp/qcore-schema1-diagnostics.json
```

The recorded run used the A directory before its exact promotion. The command
above names that same artifact at its final durable location.

Against `d938970` and `../w4-contextual-recursion`, source projections, resource
diagnostics and the existing chunk-contract sidecar are **byte-identical**.
The source-effect sidecar differs at exactly three JSON leaves:
`/results/{16,24,63}/evidence/reproducibility/package_digest`. Each changes the
old ChunkIO executable digest to the newly verified artifact digest. Replacing
only that exact digest makes the entire sidecar byte-identical. Task identities,
source/build bindings, witnesses, contracts, assumptions, bounds, statuses and
provider version remain unchanged. `audit-parity.json` records every changed
leaf and both raw file hashes; the archived sidecar retains the actual new
artifact identities.

```sh
python3 -B docs/reports/schema-1-compatibility/check_audit_parity.py
```

The audit remains **185 units: 2 Safe / 47 provisional Unsafe / 132 Unknown,
plus four data-only units**. Three field contracts check over four Boolean-state
combinations each; the other 93 selected functions remain Unknown. No raw unit
is promoted. This schema compatibility repair does not complete general
source-language, lifecycle, custody, concurrency or provider composition work.

The coordinating agent's combined standing gate passed 636 workspace tests,
nine provider tests, 43 Python tests and 19 Lean axiom audits; the detailed gate
logs are in `../w6-source-task-foundations`. This report records the independent
schema/package/consumer/audit checks for the same change set.
