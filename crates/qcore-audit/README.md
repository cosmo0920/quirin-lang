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
preserve: exactly one source input and `-c`, at most one output, distinct `-I`,
`-isystem` and `-iquote` directories, ordered `-D`/`-U`, and a supported `-std=`
dialect. The explicit `-Wall`, `-g`, `-fPIC` and `-Wa,--noexecstack` flags are also
transported to Clang preprocessing, syntax validation and typed AST import.
These flags require an explicit dialect and an executable matching the configured
Clang C driver; C++/CL driver aliases and unproven compiler substitutions reject.
Assembler requests are recorded but do not certify generated code or stack policy.
Joined and separate preprocessing arguments are accepted. The source
parent participates only in quoted include lookup. Empty arguments retain their
positions and invalid commands reject before any partial report is published.
Commands without these frontend flags still default to GNU17 when no standard
is specified; this is an analysis assumption, not discovery of another compiler's
default. Source filename builtins still see stdin; compiler equivalence and
arbitrary build flags are not established. Other options, unresolved shell
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

## Auditing a moving Fluent Bit checkout

Set `"source_revision_policy": "checkout_head"` in an audit configuration to
analyze the checkout's current commit without editing historical profile pins.
The command resolves HEAD once, verifies selected source bytes against that
commit, and records the commit, source hashes and complete compile-task identities.
Moving HEAD during the run rejects report publication. The existing clean-path,
source selection, compiler-option and conflicting-command checks still apply.
Local edits to selected source are rejected; commit the intended source first or
use the isolated build preparation below, which clones committed bytes only.

This mode consumes profile paths and declared resource contracts. Historical
file digests and construct witnesses are not admitted, and source-specific
provider/native evidence is explicitly Unknown. `source_contracts` cannot be
combined with this mode, and libraries requiring unsupplied evidence still reject.
Raw Safe/Unknown/provisional Unsafe classifications remain scoped to resource
projection under the recorded contract assumptions. They are not whole-library
safety claims. A changed library layout requires an updated source selection;
missing selected libraries never silently disappear from the audit.

The checked-in `contract/profiles/fluent-bit/current-audit.json` selects the
five normal Fluent Bit library groups from the sibling checkout:

```sh
cargo run -p qcore-audit -- contract/profiles/fluent-bit/current-audit.json
```

It requires a supported, unambiguous compilation database. The existing GCC
build and the fresh full Clang CMake database currently contain unsupported
arguments; the revision policy does not erase those arguments to obtain a result.
For an executable current-revision CMetrics counter audit:

```sh
python3 -B scripts/prepare_cmetrics_clang_audit.py
# Run qcore-audit with the absolute cmetrics-audit.json.config path printed above.
```

Preparation defaults to local `HEAD`, accepts `--revision COMMIT_OR_TAG`, and
uses an independent build under `build/cycle19/revisions/<resolved commit>`.
`--output DIRECTORY` overrides that location. It does not fetch upstream or
modify the developer's checkout. Newer revisions get fresh resource projections;
the historical revision retains its configured source-contract evidence route.
Supply the original commit with `--revision` to reproduce that route even after
the upstream checkout advances. Existing audit configurations default to
`"source_revision_policy": "profile_pin"`, preserving historical evidence gates.
The existing Cycle19 command defaults to the current checkout and now requires
both selected compilation databases to use the configured Clang C driver. A GCC
database is rejected once with preparation instructions, before per-unit work.
It is never silently rewritten to name Clang. Prepare explicit Clang CMake inputs
and run the complete current resource audit with:

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py --clang clang --clangxx clang++ --run
```

This orchestrates the independent Fluent Bit clone/build and the forced-Libevent
Monkey input preparation. CMake receives explicit C/C++ compilers, a C standard,
and `CMAKE_C_FLAGS=-std=gnu17` so repeat configuration cannot omit the dialect;
the analysis uses the final dialect in each generated command. The helper records
`audit-command.json`, sets `QCORE_CLANG` to the selected compiler, and writes
reports under `build/cycle19/clang/<commit>/reports`. Without `--run`, it prints
the exact audit command to execute. `--fluent-bit`, `--revision`, and `--output`
select another source or build location. The developer's existing GCC build is
left intact. The native build step builds `cmetrics-static`; this is not a full
native Fluent Bit executable build or a GCC/Clang equivalence claim.

The example accepts `--compile-database PATH` before positional arguments, so it
can consume that independently generated main database. Pass `--pinned-evidence`
instead for historical replay. Distinct compile configurations stay visible.

`-Wextra` is transported to Clang preprocessing, syntax checking, and AST import.
Dollar signs in a quoted C string macro are preserved literally; the adapter does
not evaluate make/shell expressions or remove the filename definition. Active
shell expansions and unquoted unresolved macro expressions still reject. Results
refer to the generated compilation database's literal arguments; equivalence to
additional Make expansion steps or general filename-builtin semantics is not
established.

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

Ordered preprocessing is transported through `CPreprocessorOption`, including
configured frame helpers. Source-contract identity binds the complete typed
sequence even if its preprocessed output is unchanged. Legacy callers with no
ordered options keep their existing identity bytes. The Cycle19 compatibility
adapter retains its historical projection (including flattened include classes)
for pinned parity; it does not establish arbitrary build equivalence. Evidence:
[W2 preprocessing transport](../../docs/reports/w2-preprocessor-transport/README.md).

The CMetrics example consumes an unmodified Clang GNU17 CMake database:

```sh
python3 -B scripts/prepare_cmetrics_clang_audit.py
# Run qcore-audit with the generated configuration path printed by preparation.
```

The script creates an independent source clone and a persistent build under
`build/cycle19/revisions/<resolved commit>`, configures CMake and builds `cmetrics-static`. Upstream
configure-time writes stay in that isolated clone. It records the original
compile entry, full database hash, source/cache identities and library hash.
Reproducing the historical source-contract route also needs the CMetrics process package at
`build/cycle19/provider-packages-unit` (build with `providers/package.py`). A custom
`--clang` must match `QCORE_CLANG` when running the audit. This is an explicit
Clang build; the original GCC command is not silently reinterpreted.
See [CMake build evidence](../../docs/reports/w2-clang-cmake-build/README.md).
