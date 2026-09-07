# Archived `libevent-core` attempt (does not load)

These files are **evidence for findings F1 and F2** in the parent report, not
an installed source set. They are deliberately kept out of
`contract/profiles/` because `profile.json` is rejected by the profile loader:
Libevent's C sources live at its checkout root, and
`SourceRootSelection::root` admits only normal relative path components.

Reproduce the rejection from the repository root (the directory name must stay
`libevent-core` for the source-set name check to pass first):

```sh
cargo run --quiet -p qcore-audit -- docs/reports/remaining-g03/libevent-core/audit.json
# audit rejected: inconsistent profile manifest:
#   library `libevent-core` source root `.` must be relative
```

| File | What it is |
|---|---|
| `profile.json` | the attempted schema v1 manifest, pinned to the `third_party/libevent` submodule at `d82464a277d0f42703702c4dfd9af6af38595a83`, selecting six compiled top-level units by digest |
| `audit.json` | the audit configuration as it would sit at `contract/profiles/libevent-core/` |
| `contracts.json`, `libevent_memory.qcore` | declared resource contracts for Libevent's documented `event_mm_*` allocator API (parses and compiles: `qcorec check` reports 5 declarations) |
| `prepare_libevent_audit.py` | the build script: verifies the submodule pin and audited digests, CMake-configures and natively builds `event_core_static` with Clang, retains CMake's unmodified database, and writes an executed supported-flag database |
| `build-provenance.json` | the real executed build: Clang 21.1.8, the exact CMake configure/build commands, the 55-entry CMake database digest, and the per-file list of CMake options the gateway cannot preserve |
| `cmake-database-sample.json` | one verbatim CMake entry showing the 18 unsupported options F2 describes |

The build script and the contract module were never needed for a verdict: the
profile is rejected before any unit is selected. Repairing F1 is a schema
decision (`PROFILE_MANIFEST_VERSION`), and repairing F2 means arguing
preservation for each additional compiler option — both are out of scope for a
configuration-only exercise, and neither was worked around by weakening a gate.
