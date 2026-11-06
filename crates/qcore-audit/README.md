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
of choosing an arbitrary build. Units are selected by located (canonical)
path, so a database spelled through a symlink to a moved checkout selects the
same units; each selected record keeps its spelling for Clang and every
report. A record spelled under a source root that resolves outside it, and two
records spelling one source file two ways (`input-path-alias`), reject
(`src/input_paths.rs`). Selected source bytes are independently
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
rewritten to name Clang. The historical corpus arbiter is `quirin-audit pinned`.
Its source adapters live in `quirin-source-analysis`, shared with the compiler
facade.

`-Wextra` is transported to Clang preprocessing, syntax checking, and AST import.
Dollar signs in a quoted C string macro are preserved literally; the adapter does
not evaluate make/shell expressions or remove the filename definition. Active
shell expansions and unquoted unresolved macro expressions still reject. Results
refer to the generated compilation database's literal arguments; equivalence to
additional Make expansion steps or general filename-builtin semantics is not
established.

## `quirin-audit pinned`: historical Fluent Bit replay

`quirin-audit pinned [--jobs N] [--checkpoint-dir D [--resume]] [fluent-bit-root]
[monkey-database] [monkey-source-report] [resource-diagnostics]
[native-observation-directory] [resource-contract-manifest]` checks the declared
source, contract and Libevent pins before replaying the Cycle 19 evidence. An
unmatched pin refuses the run. For the persistent 185-unit corpus inputs, build
the release binary and use
[`scripts/run_pinned_audit.py`](../../scripts/run_pinned_audit.py) with a fresh
output directory and `--inputs-root` when the inputs are held by another
checkout. The runner records the command, binary digest, environment and exit.

With `--checkpoint-dir D` (card CORPUS-GROUP-CHECKPOINTS) every library group
of every phase the replay runs (`first`, `refuted`, `imported`) is sealed into
`D/<library>.<phase>.qo` as soon as the group completes, and `--resume` reuses
a group whose identity (run inputs, the phase's whole-run facts, the group's
units) is unchanged; the published outputs are byte-identical to the whole
run's. The [specification](../../docs/development/audit-checkpoints.md#the-pinned-corpus-route)
lists the identity and the environments it refuses.

## `quirin-audit current`: the configured current-revision route

`quirin-audit current` audits the committed `HEAD` of a checkout: every
compilation configuration that the profile's `audit.json` library groups select from
`--compile-database`, then every unit of an optional Monkey (forced-Libevent)
database. Each configuration is analysed or explicitly Unknown, `--jobs N`
units run concurrently (default: available parallelism capped at 16, or
`QCORE_AUDIT_JOBS`; outputs are byte-identical at any count), and historical
source/native/provider evidence is written as explicit Unknown sidecars.

### Choosing the contracts: providers

Resource contracts come from exactly one of these sources:

1. **A provider selection.** Pass `--provider NAME` once per entry. `NAME` is a
   `quirin-provider-*` or `qcore-provider-*` crate on the contract load path
   (`--load-path providers/contract-load-path.json`). `NAME` alone selects the
   crate's resource view (below), in load-path order. `NAME:MODULE` selects
   one owned module.
2. **An explicit manifest** (`--contract-manifest PATH`).
3. **In contracts-only mode, all loaded contracts** when neither is given: the
   resource view of every crate on the load path (see *Contracts only*).
4. **In the full mode, the profile's declared manifest** when neither is given.

Resolution never falls back to anything else:

- An unknown provider, an unowned module or a module selected twice refuses.
- So do a stale load path, a namespace collision, a dependency cycle and a
  listed module missing on disk.
- A provider selection together with an explicit manifest refuses.
- A selection that resolves to no module refuses.
- In the full mode, with no selection, no manifest and no profile manifest,
  the command refuses: the full mode has no default provider set.

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

**Whole crates: the resource view.** Every crate on the load path declares
its `resource_view`: the owned modules the resource-contract loader accepts.
Some crates also own modules written for other checkers (lifecycle,
observation, policy and allocator-profile vocabularies), which the resource
loader refuses; those are outside the view. `python3 providers/contracts.py
sync` generates each view by loading every owned module through this crate's
own selection and loader (`cargo run -p qcore-audit --example
provider_resource_views`), and `check` refuses a view that names a refused
module or omits an accepted one. So every one of the eleven crates can be
selected whole. `quirin-provider-shared` has an empty view: named alone it
refuses, and named with others it adds nothing. The eleven views together are
exactly the 47 modules of `cycle19-resource-contracts.json`. A crate whose
record declares no view refuses as a whole-crate entry.

**The manifest the selection becomes.** A selection is lowered to the schema-2
manifest you would write beside `providers/`. A module that uses no inclusion
is written as a file path. A module that uses inclusion is written as a
`crate:module` reference. The unchanged manifest loader then loads that text.
The sidecar records it as `contract_manifest`, and every compile-task identity
digests it. A selection equal to `cycle19-resource-contracts.json` (its 47
modules, in its order) therefore lowers to that file byte for byte. Naming the
eleven crates whole selects the same 47 modules, elaborated identically, but
in load-path order, so its manifest digest differs from that file's. The
sidecar's `provider_selection` records the following:

- `selection`: `explicit`, or `all-loaded` for the contracts-only default;
- the entries and which forms declared them;
- the load path and its SHA-256;
- the closure, with each crate's name, namespace, version, role, dependencies,
  resource view, selected-module digests and published-module pins;
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
  --provider quirin-provider-fluent-bit:fluent_bit/resource_memory.qv \
  --provider qcore-provider-chunkio:chunkio/chunk_residency.qv \
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
    "quirin-provider-fluent-bit:fluent_bit/resource_memory.qv",
    "qcore-provider-chunkio:chunkio/chunk_residency.qv"
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
writes the same kind of invocation record. In contracts-only mode with a
`load_path` and no `providers` it loads all contracts too.

### Choosing the units: an explicit selection

`--selection FILE` (or `"selection"` in the configuration; both forms must
agree) replaces the profile's `audit.json` groups with exact files. The
profile is otherwise unchanged, so a unit keeps its configuration identity
under either route; only the report label differs:

```json
{
  "schema_version": 1,
  "name": "fluent-bit-system",
  "groups": [
    {"library": "fluent_bit", "label": "fluent-bit-system",
     "files": ["plugins/in_storage_backlog/sb.c", "src/flb_task.c"]}
  ]
}
```

Every distinct compilation configuration of a selected file is its own unit,
so static and shared variants stay separate. A selected file with no
compilation record, or missing from the checkout, is one `Unknown` unit with
`reason_code: "environment-unavailable"`; it is never dropped. Paths must be
plain relative `.c` paths, distinct across the file, and every group's library
must be declared by the profile. The invocation record (`selection`) and the
`.contracts.json` sidecar (`unit_selection`) name the file by path, SHA-256
and contents. `contract/profiles/fluent-bit/system-selection.json` is the
Fluent Bit system slice ([W45-FB-SYSTEM-SELECTION](../../docs/reports/w45-fb-system-selection/README.md)).

### Per-library contract scoping and slice execution

To bound peak memory during multi-library audits, `quirin-audit` can scope
loaded contracts to each library slice (`providers` plus the transitive
`dependent_libraries` closure) and execute slices sequentially, absorbing unit
outcomes and clearing the cached contract premises and hypothesis graphs at
the end of each slice before loading the next slice's contracts.

Per-library scopes can be declared in `--library-providers FILE` (for example
`contract/profiles/fluent-bit/library-providers.json`), inline via
`library_providers` in `CurrentConfig` / `AuditConfig`, or on each group in
`selection.json` or `audit.json`:

```json
{
  "schema_version": 1,
  "libraries": {
    "chunkio": {
      "providers": ["quirin-provider-libc", "qcore-provider-chunkio"],
      "dependent_libraries": []
    },
    "fluent_bit": {
      "providers": ["quirin-provider-libc", "quirin-provider-fluent-bit"],
      "dependent_libraries": ["cmetrics", "ctraces", "chunkio", "librdkafka", "monkey-libevent"]
    }
  }
}
```

Resolution computes the transitive `dependent_libraries` closure (refusing
cycles and unknown libraries), deduplicates provider entries in closure order,
and resolves them against `--load-path`. Both `<report>.invocation.json` and
`<report>.contracts.json` / `<report>.assumptions.json` record
`library_provider_selection` (`mode: "per-library-slice"` and per-library
resolved closures, modules and digests).

### Contracts only

`--contracts-only`, or `"mode": "contracts-only"` in either configuration,
audits using only Quirin contracts. It runs the resource projection from the
`.qcore` modules elaborated from the loaded contracts, and nothing else.

**Contracts only loads all contracts.** With no `--provider` / `providers` and
no `--contract-manifest`, a contracts-only run selects every crate on the load
path whole: all eleven resource views, which are the Cycle 19 manifest's 47
modules. This is the only default provider set, and it exists only in
contracts-only mode. It needs the load path (`--load-path` or `load_path`); the
profile's declared manifest is never a silent substitute. An explicit
selection or manifest narrows it. The sidecar and `<report>.invocation.json`
record `"selection": "all-loaded"`. The simplest contracts-only run:

```sh
QCORE_CLANG=$(command -v clang) cargo run --release -p qcore-audit -- current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --load-path providers/contract-load-path.json \
  --contracts-only \
  --report "$out/reports/contracts-only.json"
```

A contracts-only run admits none of the following:

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
`.qcore` contracts. The opt-in `--quirin-providers-only` refuses any
`qcore-provider-*` crate in the resolved closure; it refuses the all-loaded
default rather than narrowing it, so combine it with a selection of Quirin
crates, as the JSON below does:

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

### Preparing `libevent-core`

The `libevent-core` inputs are prepared by the external subcommand
`quirin-audit libevent prepare` (crate `quirin-audit-plugin-libevent`), from
the pinned `third_party/libevent` submodule, byte for byte as the retired
preparation script did ([parity](../../docs/reports/w45-audit-subcommand-plugins/README.md)):

```sh
cargo build -p qcore-audit -p quirin-audit-plugin-libevent
target/debug/quirin-audit --plugin-path target/debug --output ../out/libevent \
  libevent prepare --clang clang --clangxx clang++
```

It writes `libevent-clang-build/` under the output: CMake's unmodified
`compile_commands.json`, `libevent-core-audit.json.config` and
`libevent-build-provenance.json`. An output inside `build/cycle19` or
`third_party` is refused. `--write-profile` writes the regenerated audited-file
table to `libevent-clang-build/profile.json` for review instead of editing the
committed profile.

## External subcommands

`quirin-audit [--plugin-path DIR]... --output DIR NAME ARGS...` runs the
executable `quirin-audit-NAME`, cargo-style, when `NAME` is not built in.
Only the declared search path is searched (`--plugin-path`, then
`QUIRIN_AUDIT_PLUGIN_PATH`; never `PATH`), the environment is cleared except
an allow-list, and the plugin runs under wall-time and output quotas in its
output directory. Its verdict claims pass through the checker-plugin firewall,
so an uncertified `Safe` or `Unsafe` is `Pending` and any failure `Unknown`.
`--list-plugins` lists what the search path holds. The report is
`<output>/quirin-audit-NAME.report.json` (or `--report FILE`), with
`<report>.invocation.json` beside it. How to write one:
[audit subcommand plugins](../../docs/development/audit-subcommand-plugins.md).

## `quirin-audit contracts`: contracts only, without build settings

`quirin-audit contracts` audits C sources with the loaded contracts alone. It
reads no compilation database, prepared build, checkout pin or source
profile. Point it at a source tree:

```sh
cargo run --release -p qcore-audit -- contracts --source-root ../fluent-bit
```

Run it from the repository root: with no `--load-path` and no
`--contract-manifest`, it reads `providers/contract-load-path.json` from the
working directory and loads every crate's resource view (`"selection":
"all-loaded"`, recorded with `load_path_source: "default"`). A missing file
refuses. It is a separate subcommand, not a `current` flag, because it shares
none of `current`'s inputs: no checkout `HEAD`, no profile library groups and
no database commands.

**Selection.** Name files and directories after the flags, relative to the
source root (`--source-root`, default the working directory), or name nothing
to walk the whole root. A directory contributes its regular `*.c` files in
sorted order. The walk does not enter directories whose name starts with `.`
and does not follow symbolic links. The first selection of a file wins. A
directory with no unit, a path outside the root and an empty selection refuse.
A unit's report `library` is the argument that selected it (its parent
directory for a named file), and its `file` is relative to the root.

**Assumed configuration.** Each unit is preprocessed with only what you
supply, `-I DIR`, `-D NAME[=VALUE]` and `--std c11|gnu99|gnu11|gnu17`, plus
three stated defaults:

- the `gnu17` dialect;
- the unit's own directory for quoted includes;
- the host Clang's system headers.

Nothing is searched for or guessed: no include directory discovery, no
configuration header, no build-system reading. An include directory must exist.
A definition must be a C identifier with an optional value, and no `$`. Headers
are read as found on disk, tracked or not. So a header that an earlier in-tree
configure left in the checkout is read like any other.

**Units that cannot be analysed.** A unit that fails to preprocess or parse
under these settings is `Unknown`. It is never dropped. Its row keeps its raw
function denominator, `reason_code: "environment-unavailable"`, and under
`assumptions` a reason code with the first Clang diagnostic. The reason codes
are:

| Code | Meaning |
|---|---|
| `missing-include` | an included file was not found |
| `unresolved-configuration` | an `#error` directive fired |
| `preprocessing-error` | any other preprocessing error |
| `syntax-error` | the preprocessed text does not parse |
| `unsupported-input-identity` | the input identity is refused |
| `clang-unavailable` | Clang could not run |
| `unreadable-source`, `not-utf8` | the source file could not be read as text |

The console tallies them (`not prepared: missing-include=…`). Such units are
not gateway failures, so they do not fail the command.

**The publication rule.** Every verdict of this route is relative to the
recorded assumptions and to nothing else. `Safe` means Safe for the unit as
preprocessed under this dialect, these include directories and definitions,
its own directory and the host system headers. A no-build verdict is **never
presented as equivalent to a build-configured one**: a different configuration
can select different code. The route enforces this in three ways:

- Every report row carries `assumptions`, holding the effective settings,
  `outcome`, `reason` and `verdict_scope`.
- The report's `evidence` block states `evidence: "contracts-only-no-build"`,
  `build: "none"` and `equivalence_to_build_configured: "none: …"`.
- The assumption record, together with the selection, is each compile task's
  `source_profile` dependency. Every Safe certificate's trust context digests
  it, so a certificate minted under assumed settings cannot be admitted for a
  configured compile task.

Lean model: `QCore.NoBuildScope`. It includes the witness that a Safe under
assumptions and an Unsafe under a build can coexist for one source.

**JSON and precedence.** `--config FILE` takes the same settings. Its
`mode: "contracts-only"` and `build: "none"` are required. Paths are relative to
the file, except `sources`, which are relative to the source root.
`compile_database`, `checkout`, `profile_directory`, `source_contracts` and
`packages` refuse. The rule is `current`'s: when both forms give a setting, the
two must agree, and nothing is merged. The job count is the exception: flag,
then `jobs`, then `QCORE_AUDIT_JOBS`, then the capped host default.

```json
{
  "schema_version": 1,
  "mode": "contracts-only",
  "build": "none",
  "source_root": "../fluent-bit",
  "sources": ["lib/ctraces/src", "lib/chunkio/src"],
  "include_dirs": ["../fluent-bit/lib/ctraces/include"],
  "defines": [],
  "std": "gnu17",
  "load_path": "providers/contract-load-path.json",
  "report": "private/no-build/report.json",
  "jobs": 16
}
```

`--jobs N` analyses units with the same ordered fold as `current`. The report
and the `.contracts.json` sidecar are byte-identical at any count. The
sidecar records the selection, the assumed configuration, the contracts, the
outcome tallies, and every file with its settings and raw SHA-256.
`<report>.invocation.json` records the jobs, the form that declared the
dialect, and the load path and its source. On Fluent Bit, every unit that both
this route and the build-configured run analyse gets the same verdict; the
[measurement](../../docs/reports/w44-contracts-only-no-build/README.md)
attributes every difference.

## Pinned and flexible contracts: `quirin-audit pin` and `compare`

A contract is **pinned** unless it says `requires resource_revision(flexible);`
(contract specification C06.2). A pinned contract is in force only where its
contract text is the pinned one and the audited source matches one of the
**reviewed identities** its crate's generated `contracts.lock` records, each
the function's sites at the revisions it was reviewed at; every route above,
and the corpus arbiter, compares the pins before analysing any unit, withholds
each mismatch (calls relying on it are held under `contract-pin-mismatch`, and a
function resting only on held calls is `Pending`)
and publishes a `contract_revision` section in the contract sidecar with a
one-line tally. Flexible contracts bind by name and signature; the libc
provider's are one standing assumption.

```sh
# after reviewing the contracts against a checkout: add (or refresh) its
# identity in every lock, keeping those reviewed at other revisions
quirin-audit pin --source-root /path/to/fluent-bit
# per revision, never writing: recorded / in force through another revision /
# not in force; exit 1 unless every entry is recorded at this revision
quirin-audit pin --source-root /path/to/fluent-bit --check
quirin-audit pin --source-root /path/to/fluent-bit --check --revision 66910c10
# add one reviewed function only; drop a revision; rebuild from one revision
quirin-audit pin --source-root /path/to/fluent-bit --function flb_input_chunk_destroy
quirin-audit pin --drop-revision 66910c10 --crate quirin-provider-fluent-bit
quirin-audit pin --source-root /path/to/fluent-bit --regenerate
# the change report between two runs (two revisions or two contract sets)
quirin-audit compare --before A/resource-diagnostics.json --after B/resource-diagnostics.json --output change.json
```

The change report keeps *violation detected* (a move to `Unsafe`) apart from
*coverage lost* (a move from `Safe` to `Pending` or `Unknown`) and lists contracts in force,
pin mismatches, assumptions, sources and per-function obligations
([report](../../docs/reports/w45-contract-revision-mode/README.md)); since
W45-USER-CONTRACT-PLUGINS it also classes moves between `Unknown` and `Pending`
and tallies each run's four verdicts.

**Tip identities** (card CONTRACT-PIN-TIP-MODE, C06.2). A pinned contract
written for a fix that exists only on a branch can track the branch's tip
instead of a reviewed revision: its lock entry (format 3) names the
repository, the ref and the commit reviewed, and an audit keeps it in force
exactly where the audited sites equal those at the ref's current commit,
resolved read-only in the local repository (`QUIRIN_TIP_REPOSITORIES=NAME=PATH`,
else the path the lock records; never fetched). An unresolvable tip is held
(`tip-unresolved`). Every command that meets one prints a notice that the
contract targets a future, unreleased release (`info` in force, `warning`
not); an in-force tip is the run's named assumption `tip-tracked:FUNCTION`.

```sh
# track the tip of a fix branch for one reviewed function (no source change)
quirin-audit pin --source-root /path/to/monkey --crate qcore-provider-libevent \
    --function mk_vhost_set --track-tip cosmo0920-missing-va_end-on-null \
    --repository-name monkey --release-label "next Monkey release"
# back to a reviewed pin of this checkout's HEAD
quirin-audit pin --source-root /path/to/monkey --crate qcore-provider-libevent \
    --function mk_vhost_set --untrack-tip
```

## User contract crates and checker plugins

Card W45-USER-CONTRACT-PLUGINS (contract specification C11.1). Every verdict
is one of `Safe < Pending < Unknown < Unsafe` and every roll-up is their join;
`Pending` (awaiting a named human act) is printed in every tally and never
counted as `Safe`. A user's own contract crate, outside this repository, is
listed in a **user load path** that `extends` the shipped one and selected like
any provider:

```sh
quirin-audit plugin views --load-path work/user-load-path.json --write
quirin-audit pin --load-path work/user-load-path.json --source-root SOURCE
quirin-audit contracts --load-path work/user-load-path.json \
    --provider quirin-provider-acme --source-root SOURCE --report out/r.json
```

A user module declaring a function a shipped crate declares is refused unless
the user load path overrides it explicitly (recorded under the selection's
`user_load_path`). A **checker plugin** bound to a user module runs once per
run as a verified process package (`user_plugins.rs`); the host's firewall
counts its `Safe` only with a certificate the audit's trusted checker replays
and its `Unsafe` only with a validated witness, and otherwise either is
`Pending`; a failure is `Unknown`. Its verdict joins every function that
applies a bound contract, and the sidecar carries a `plugin_checks` section.
The SDK is [`providers/quirin-provider-plugin`](../../providers/quirin-provider-plugin/README.md).

## Checkpoints: per configuration, resume, compose

`quirin-audit current … --checkpoint-dir D` seals one complete `.qo` for
each source translation unit and compile configuration as its worker finishes.
Two compile records for one C source get distinct files. Completed files survive
an interrupted run even when ordered output waits for an earlier slow unit;
`--resume` checks each file's analyzer, contract, environment, revision,
compile-task and preparation identity and recomputes only changed or missing
units. The report is then composed in plan order and is byte-identical to the
whole run's. Checkpointing is off by default and never changes a verdict.

```sh
quirin-audit current --checkout … --profile-directory … --report out/r.json \
  --checkpoint-dir out/qo [--resume] --jobs 16
quirin-audit checkpoint verify out/qo/*.qo
quirin-audit checkpoint compose out/qo/*.qo --report again/r.json
quirin-audit checkpoint decompose out/qo/UNIT.qo --format yaml --output unit.yaml
quirin-audit checkpoint recompose unit.yaml --format yaml --output unit.qo
```

A schema-3 `.qo` file has ten separately digested binary sections (identity, plan, global,
preparation, lowering, verdicts, certificates, contracts, result, provenance).
It encodes integers, finite float bits and common result statuses numerically,
with UTF-8 for names, paths and messages. Each file contains the full run plan
and global publication inputs, so it is self-contained. A truncated, tampered or version-mismatched file is refused
whole; composition refuses missing, duplicate or mixed-run files. Every section
but `provenance` (wall time, peak memory, timings) is deterministic, so runs
at any `--jobs` agree on the content digest `verify` prints. The run's verdict
joins every unit's verdict (`Safe < Pending < Unknown < Unsafe`). The full
[checkpoint specification](../../docs/development/audit-checkpoints.md)
describes the binary and YAML/TOML formats and identity rules.

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
