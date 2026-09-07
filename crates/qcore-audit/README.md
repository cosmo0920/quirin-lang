# qcore-audit

A configured L3 driver for bounded existing-C resource projection and optional
process-based source-contract checking.
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
source bytes. Resource projection stays separate from the optional function-contract
evidence below; neither changes the other's verdict or discharges native-observation
requirements.

The retained Cycle19 command uses a compatibility adapter for its pinned
compiler-option projection and specialized source/native evidence stages. Its
shared projection/report code lives here; its provider-specific checks do not.

## Configured source-contract processes

Add an explicit `source_contracts` object to the audit configuration:

```json
{
  "schema_version": 1,
  "profile_directory": "profiles/sample",
  "checkout": "checkout",
  "compile_database": "compile_commands.json",
  "libraries": ["sample_widgets"],
  "report": "output/report.json",
  "source_contracts": {
    "configuration": "source-contracts.json",
    "source_rules": "source_projection.qcore",
    "libraries": [{"library": "sample_widgets", "source_root": "src"}]
  }
}
```

`configuration` selects the existing source-contract package/policy format:

```json
{
  "schema_version": 1,
  "packages": ["packages/sample-provider"],
  "bindings": [{
    "library": "sample_widgets",
    "provider": "qcore.sample_audit",
    "contract": "sample.qcore",
    "frame_helpers": [{
      "source": "sample_reset.c",
      "function": "sample_reset",
      "record": "sample_record",
      "contract": "sample_reset.qcore"
    }]
  }]
}
```

Package and policy paths resolve relative to that configuration file. The audit's
`source_rules` path resolves relative to the audit configuration and supplies the
existing bounded source-projection acceptance rules. Each helper source path
resolves beneath its explicit `source_root`, relative to the checkout. Omit
`frame_helpers` when none are used. Selected source-contract libraries must be
unique, selected by the resource audit, have declared Boolean field models, and
match the complete binding inventory. Other resource-audit libraries can remain
outside this explicit source-contract selection.

Each helper requires one compile entry, supported flags, actual pinned source
bytes and a checked frame policy. Main units use their actual pinned source and
strict compile options. The command imports them through the trusted gateway,
dispatches the selected process and independently replays its typed witnesses.
There is no built-in provider fallback. Missing/incompatible configuration rejects
before report publication. Unsupported imports, missing policy coverage and
unadmitted process evidence remain Unknown; raw and build-excluded definitions
stay visible in the function inventory.

The main report's optional `source_contracts` object contains loaded package and
policy snapshots, selected roots/revisions, source rules, helper import identities,
per-unit functions, admitted tasks/witnesses and explicit checked/Unknown counts.
A checked function contract is not a Safe resource unit. Omitting the option
retains the original resource report shape. Command success still means the audit
ran; the report can contain Unknown or provisional Unsafe results.

This route currently supports the declared single-field Boolean model with
live/exclusive/domain and C ABI premises. It does not establish arbitrary object
lifecycles, native observations, whole-program safety or recursive C layout/linker
equality. Libraries declaring additional required evidence still reject until a
separate route supplies it. Package/compiler/input stability during execution
remains a caller premise; package verification is a snapshot, not an OS sandbox.
See [the end-to-end command evidence](../../docs/reports/w5-configured-process-audit/README.md).
