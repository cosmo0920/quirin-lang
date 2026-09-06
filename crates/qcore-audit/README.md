# qcore-audit

A provider-independent L3 driver for bounded existing-C resource projection.
It loads W2 source profiles and W1 contracts, selects compiled units from data,
checks pins/source identities, runs the C gateway and writes diagnostics plus
an assumption sidecar. Its normal dependency graph contains no compiler or
provider crate.

```sh
cargo run -p qcore-audit -- /path/to/audit.json
```

The configuration uses paths relative to its own directory:

```json
{
  "schema_version": 1,
  "profile_directory": "profiles/sample",
  "checkout": "checkout",
  "compile_database": "compile_commands.json",
  "libraries": ["sample"],
  "report": "output/report.json"
}
```

The profile directory supplies `profile.json`, including the source pin,
source selections, file digests and reference to a resource-contract manifest.
Compilation database records accept either `command` or `arguments`, with an
absolute compilation directory. Selected conflicting commands reject instead
of choosing an arbitrary build. Selected source bytes are independently
compared with their pinned Git blob, including files omitted from a profile's
audited-file list.

The configured command accepts only compile options that its gateway can
preserve: source/output names, `-c`, include directories, macro definitions
and an explicitly supported `-std=` dialect. Other options, unresolved shell
expansions, absent contracts, empty selections and unsupplied required evidence
reject. This conservative bound can be widened only alongside gateway support.

Reports preserve Safe, provisional Unsafe and Unknown separately; successful
command completion means the audit ran, not that every unit was Safe. Assumption
sidecars identify the profile, contracts, compilation database and selected
source bytes. The driver performs resource projection only, without observation
or provider composition.

The retained Cycle19 command uses a compatibility adapter for its pinned
compiler-option projection and specialized source/native evidence stages. Its
shared projection/report code lives here; its provider-specific checks do not.
