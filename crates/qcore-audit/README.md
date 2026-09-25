# qcore-audit

A configured L3 driver for bounded existing-C resource projection and optional
process-based source-contract checking.
It loads W2 source profiles and W1 contracts, selects compiled units from data,
checks pins/source identities, runs the C gateway and writes diagnostics plus
an assumption sidecar. Its normal dependency graph contains no compiler or
provider crate.

The crate is `qcore-audit`; the command it installs is **`quirin-audit`**
(`quirin-audit CONFIG.json`). From a checkout, the package spelling still
selects it:

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
cargo run -p qcore-audit -- prepare clang-build
# Run quirin-audit with the absolute cmetrics-audit.json.config path printed above.
```

Preparation (`quirin-audit prepare clang-build`, see
[below](#preparing-the-fluent-bit-build)) defaults to local `HEAD`, accepts
`--revision COMMIT_OR_TAG`, and uses an independent build under
`build/cycle19/revisions/<resolved commit>`.
`--output DIRECTORY` overrides that location. It does not fetch upstream or
modify the developer's checkout. Newer revisions get fresh resource projections;
the historical revision retains its configured source-contract evidence route.
Supply the original commit with `--revision` to reproduce that route even after
the upstream checkout advances. Existing audit configurations default to
`"source_revision_policy": "profile_pin"`, preserving historical evidence gates.
The configured current-revision route is `quirin-audit current` (see
below). It requires every selected compilation database to use the configured
Clang C driver: a GCC database (`/usr/bin/cc`) is rejected once, with
preparation instructions, before any per-unit work, and is never silently
rewritten to name Clang. The compiler example
`cycle19_fluent_bit_libraries_audit` keeps its configured route as a thin
wrapper over the same code, with the Fluent Bit profile and its declared
manifest; its `--pinned-evidence` route (historical replay, the corpus arbiter)
is unchanged.

`-Wextra` is transported to Clang preprocessing, syntax checking, and AST import.
Dollar signs in a quoted C string macro are preserved literally; the adapter does
not evaluate make/shell expressions or remove the filename definition. Active
shell expansions and unquoted unresolved macro expressions still reject. Results
refer to the generated compilation database's literal arguments; equivalence to
additional Make expansion steps or general filename-builtin semantics is not
established.

## `quirin-audit current`: the configured current-revision route

`quirin-audit current` audits the committed `HEAD` of a checkout: every
compilation configuration that the profile's `audit.json` library groups select from
`--compile-database`, then every unit of an optional Monkey (forced-Libevent)
database. Each configuration is analysed or explicitly Unknown, `--jobs N`
units run concurrently (default: available parallelism capped at 16, or
`QCORE_AUDIT_JOBS`; outputs are byte-identical at any count), and historical
source/native/provider evidence is written as explicit Unknown sidecars.

### Choosing the contracts: providers

Resource contracts come from exactly one of three sources:

1. **A provider selection.** Pass `--provider NAME` once per entry. `NAME` is a
   `quirin-provider-*` or `qcore-provider-*` crate on the contract load path
   (`--load-path providers/contract-load-path.json`). `NAME` alone selects every
   module the crate owns, in load-path order. `NAME:MODULE` selects one module.
2. **An explicit manifest** (`--contract-manifest PATH`).
3. **The profile's declared manifest**, used when neither is given.

Resolution never falls back to a default:

- An unknown provider, an unowned module or a module selected twice refuses.
- So do a stale load path, a namespace collision, a dependency cycle and a
  listed module missing on disk.
- A provider selection together with an explicit manifest refuses.
- With no selection, no manifest and no profile manifest, the command refuses:
  there is no default provider set.

A selection supersedes the profile's declared manifest. The sidecar records
that manifest as not read.

**Dependencies.** The selected crates' dependency closure is resolved and
recorded. It is also where the selected modules' `#include` directives
resolve. A dependency's own modules are not selected just because it is a
dependency. For example, `quirin-provider-fluent-bit` resolves
`quirin-provider-libc`, and its `resource_memory.qcore` inherits
`libc::malloc` from there. Name `quirin-provider-libc` as well to audit with
libc's own contracts. Add `--quirin-providers-only` to refuse any
`qcore-provider-*` crate anywhere in the closure.

**Whole crates.** Today only four crates can be selected whole:
`quirin-provider-libc`, `quirin-provider-mpack`, `quirin-provider-msgpack` and
`qcore-provider-ctraces`. The other seven also own modules written for other
checkers, which the resource loader refuses. For those, select modules with
`NAME:MODULE`. The report's follow-up `W44-PROVIDER-RESOURCE-VIEW` tracks this.

**The manifest the selection becomes.** A selection is lowered to the schema-2
manifest you would write beside `providers/`. A module that uses no inclusion
is written as a file path. A module that uses inclusion is written as a
`crate:module` reference. The unchanged manifest loader then loads that text.
The sidecar records it as `contract_manifest`, and every compile-task identity
digests it. A selection equal to `cycle19-resource-contracts.json` (its 47
modules, in its order) therefore lowers to that file byte for byte. The
sidecar's `provider_selection` records the following:

- the entries and which forms declared them;
- the load path and its SHA-256;
- the closure, with each crate's name, namespace, version, role, dependencies,
  selected-module digests and published-module pins;
- each module's digest and spelling.

### Command line and JSON configuration

Every setting can be given as a flag, in a JSON configuration (`--config FILE`,
with paths relative to the file), or both. When both give a setting, the two
must agree. For providers, agreeing means resolving to the same modules in the
same order. If they disagree, the command refuses; nothing is merged or
overridden.

**Parallel units.** The job count is the one exception to that rule, because
it changes wall time only. Its sources, in order of precedence:

1. `--jobs N` on the command line;
2. the configuration's `"jobs": N`;
3. `QCORE_AUDIT_JOBS`;
4. the host's available parallelism, capped at 16.

A zero from any source is an error, not a fallback, and `N = 1` is the
sequential loop. Both routes analyse units with W44-AUDIT-PARALLEL-UNITS's
ordered fold (`qcore_audit::parallel`): results are folded in unit order, the
first error in unit order is the one reported, and an active census sink
forces one job. So the report, the sidecars and every console line except the
`preparation:` timing sums are byte-identical at any count. The plain route's
source-contract step, which may dispatch a process provider, runs in the fold,
one unit at a time.

The requested count, the effective count and the source of the count are
recorded in `<report>.invocation.json`. They are not recorded in the
assumption sidecar, which is part of what stays identical across job counts.

A worked command line: the Fluent Bit libraries with libc, the
Fluent Bit wrappers and ChunkIO's residency contract, over a prepared build
(`$out` as below):

```sh
QCORE_CLANG=$(command -v clang) cargo run --release -p qcore-audit -- current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$out/monkey/monkey-build/compile_commands.json" \
  --load-path providers/contract-load-path.json \
  --provider quirin-provider-libc \
  --provider quirin-provider-fluent-bit:fluent_bit/resource_memory.qcore \
  --provider qcore-provider-chunkio:chunkio/chunk_residency.qcore \
  --monkey-source-report "$out/reports/monkey-source.json" \
  --report "$out/reports/resource-diagnostics.json" \
  --jobs 16
```

The same run as JSON (`quirin-audit current --config current.json`, the file at
the repository root):

```json
{
  "schema_version": 1,
  "checkout": "build/cycle19/clang/REV/fluent-bit-clang-source",
  "profile_directory": "contract/profiles/fluent-bit",
  "compile_database": "build/cycle19/clang/REV/fluent-bit-clang-production/compile_commands.json",
  "monkey_database": "build/cycle19/clang/REV/monkey/monkey-build/compile_commands.json",
  "load_path": "providers/contract-load-path.json",
  "providers": [
    "quirin-provider-libc",
    "quirin-provider-fluent-bit:fluent_bit/resource_memory.qcore",
    "qcore-provider-chunkio:chunkio/chunk_residency.qcore"
  ],
  "monkey_source_report": "build/cycle19/clang/REV/reports/monkey-source.json",
  "report": "build/cycle19/clang/REV/reports/resource-diagnostics.json",
  "jobs": 16
}
```

`python3 docs/reports/w44-quirin-audit-providers/scripts/manifest_to_providers.py
cycle19-resource-contracts.json` prints the 47 entries equal to the Cycle 19
manifest. Its `--config OUT.json KEY=VALUE...` form writes them into such a
configuration.

The plain `quirin-audit [--jobs N] CONFIG.json` route takes the same
`providers`, `load_path`, `quirin_providers_only`, `mode` and `jobs` fields, and
writes the same kind of invocation record.

### Contracts only

`--contracts-only`, or `"mode": "contracts-only"` in either configuration,
audits using only Quirin contracts. It runs the resource projection from the
`.qcore` modules elaborated from the selected providers (or the manifest), and
nothing else. It admits none of the following:

- process packages or source-contract checks;
- verifier-host execution;
- compiler-crate verifier-plugin evidence;
- native or observed evidence;
- pinned historical evidence.

Anything that needs that evidence stays Unknown. A configuration that also
names `source_contracts` or `packages` refuses; nothing is silently ignored.
The report and the sidecar carry an `evidence` block. It gives the mode, the
excluded evidence and every contract module, with the SHA-256 of its elaborated
text, that the verdicts rest on. Included parents are listed under
`contract_inclusion`. A contracts-only verdict is therefore distinguishable
from a full audit's.

The configured current route admits no process packages today. On the same
inputs, a contracts-only run differs from a plain run only in that block and one
console line. Provider selection is the same in both modes and admits
`quirin-provider-*` and `qcore-provider-*` crates alike, since both carry
`.qcore` contracts. If only `quirin-provider-*` crates should be admitted, add
`--quirin-providers-only`.

```sh
quirin-audit current --config current.json --contracts-only --quirin-providers-only
```

```json
{
  "schema_version": 1,
  "checkout": "build/cycle19/clang/REV/fluent-bit-clang-source",
  "profile_directory": "contract/profiles/fluent-bit",
  "compile_database": "build/cycle19/clang/REV/fluent-bit-clang-production/compile_commands.json",
  "load_path": "providers/contract-load-path.json",
  "providers": ["quirin-provider-libc", "quirin-provider-mpack", "quirin-provider-msgpack"],
  "mode": "contracts-only",
  "quirin_providers_only": true,
  "report": "build/cycle19/clang/REV/reports/contracts-only.json"
}
```

### Preparing the Fluent Bit build

`quirin-audit prepare` prepares the inputs, and `--run` chains the audit.
Run it from the repository root (or pass `--repository DIR`). It leaves the
developer's GCC build intact, works in an independent clone of the local
checkout and reads no network:

```sh
cargo run --release -p qcore-audit -- prepare --fluent-bit ../fluent-bit \
  --clang clang --clangxx clang++ --run --jobs 16
```

This prepares `build/cycle19/clang/<commit>/` (commit of `--revision`,
default `HEAD`; `--output DIR` overrides it) and then runs
`quirin-audit current` with `QCORE_CLANG` set to the prepared C compiler.
Arguments after `--` are passed to that run, for example
`-- --load-path providers/contract-load-path.json --provider quirin-provider-libc`
or `-- --contracts-only`. Without them, the profile's declared manifest
(`cycle19-resource-contracts.json`) selects the contracts. An explicit
`QCORE_CLANG` naming another compiler refuses before the build. Without
`--run`, an equivalent audit, naming the same manifest explicitly, is:

```sh
out=build/cycle19/clang/$(git -C ../fluent-bit rev-parse HEAD)
QCORE_CLANG=$(command -v clang) cargo run --release -p qcore-audit -- current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --monkey-database "$out/monkey/monkey-build/compile_commands.json" \
  --contract-manifest cycle19-resource-contracts.json \
  --monkey-source-report "$out/reports/monkey-source.json" \
  --report "$out/reports/resource-diagnostics.json"
```

Preparation has two steps, each also a subcommand of its own. They run the
same git, CMake and build commands, with the same arguments, as the retired
Python preparation scripts, and write the same files
([parity evidence](../../docs/reports/w44-quirin-audit-prepare/README.md)):

- `prepare clang-build` clones the checkout (`git clone --no-hardlinks
  --no-checkout`, detached at the revision) into `fluent-bit-clang-source`,
  configures `fluent-bit-clang-production` with explicit `--clang` and
  `--clangxx`, `-DCMAKE_C_FLAGS:STRING=-std=gnu17`, `FLB_DEV` and
  `FLB_EXAMPLES`, and builds `cmetrics-static` (not a full native Fluent Bit
  executable, and not a GCC/Clang equivalence claim). It writes
  `cmetrics-audit.json.config` and `build-provenance.json`, which records the
  configure command, the C++ compiler included. Its default output is
  `build/cycle19/revisions/<commit>`.
- `prepare monkey --fluent-bit DIR` copies the checkout's tracked `lib/monkey`
  files into `monkey-source` and configures the forced-Libevent Monkey build in
  `monkey-build`. It writes the `qcore-monkey-inputs.json` marker, with source
  hashes and the CMake command, and `configure.log`. Its default output is
  `build/cycle19`. `prepare` runs it on the clone, into `<out>/monkey`.

`prepare` also writes `<out>/prepare.json`: the argv, both compilers (path,
resolved path and `--version`), CMake, the revision and the digests of every
step record. Each step refuses a reused output whose identity changed; choose
a fresh `--output`. `--cmake PATH` selects a CMake executable. `QCORE_CLANG`
must name the compiler the database names. Add `--jobs N` for the job count,
and keep `--release` for an optimised build. Neither changes a result.

**Provenance.** Beside `--report`, the command writes
`<report>.invocation.json`. This replaces the `audit-command.json` that the
retired wrapper wrote. Each old field now lives here:

- `source_revision` → `source_revision` (the checkout's audited `HEAD`, also in
  the sidecar);
- `clang` → `clang` (the executable, version and target that ran, captured
  from the configured `QCORE_CLANG`);
- `clangxx` → preparation's `build-provenance.json`, Monkey marker and
  `prepare.json`, where it is used; the audit never runs the C++ compiler;
- `compile_database` and `monkey_database` → the same keys, each with its
  SHA-256;
- `cwd` and `command` → `cwd` and `argv`;
- `profile` → `build_profile` (`dev` or `release`);
- `jobs` → `jobs` (`requested` and `effective`);
- `environment` → `environment` (`QCORE_CLANG`, `QCORE_AUDIT_JOBS`);
- `scope` → `scope`.

The record also names the configuration file and its digest, and the contract
source with the digest of the manifest that was loaded. It is provenance of the
run, not an input to any verdict, so it is not part of any identity comparison.

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

`configuration` selects the existing source-contract package/policy format.
Instead of `packages`, a configuration may give `"package_root": "DIR"`, a
`providers/package.py build` output directory. Each binding's provider id then
names its package, `DIR/<provider id>`, and that package is verified exactly as
a listed one is. Exactly one of `packages` and `package_root` is required.

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
cargo run -p qcore-audit -- prepare clang-build
# Run quirin-audit with the generated configuration path printed by preparation.
```

The step creates an independent source clone and a persistent build under
`build/cycle19/revisions/<resolved commit>`, configures CMake and builds `cmetrics-static`. Upstream
configure-time writes stay in that isolated clone. It records the original
compile entry, full database hash, source/cache identities and library hash.
Reproducing the historical source-contract route also needs the CMetrics process package at
`build/cycle19/provider-packages-unit` (build with `providers/package.py`). A custom
`--clang` must match `QCORE_CLANG` when running the audit. This is an explicit
Clang build; the original GCC command is not silently reinterpreted.
See [CMake build evidence](../../docs/reports/w2-clang-cmake-build/README.md).
