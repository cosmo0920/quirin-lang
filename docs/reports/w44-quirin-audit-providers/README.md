# W44-QUIRIN-AUDIT-PROVIDERS: providers for `quirin-audit`, `quirin-audit current`, contracts only

Card W44-QUIRIN-AUDIT-PROVIDERS, 2026-09-25, on base `a2a63ced` ("Record the
w44-collection-allocation integration and archive the combined baseline").
Mid-card, the user widened the scope three times:

1. retire `scripts/prepare_cycle19_clang_audit.py`;
2. add a contracts-only route;
3. give every unit-analysing `quirin-audit` route parallel execution.

All three are part of this report.

## Headline

* **Providers by name.** `quirin-audit` selects resource contracts by provider
  crate: repeatable `--provider NAME[:MODULE]`, or a JSON `providers` array.
  Selection accepts `quirin-provider-*` and `qcore-provider-*` crates alike and
  resolves them, with their dependency closure, through
  `providers/contract-load-path.json`. Resolution fails closed.
* **One route.** The configured current-revision route moved from the
  compiler example into `qcore_audit::current_route`, and
  `quirin-audit current` runs it. The example's configured route is now a thin
  wrapper over that code.
* **Script retired.** `scripts/prepare_cycle19_clang_audit.py` is deleted. Its
  provenance record moved into `<report>.invocation.json`.
* **Contracts only.** `--contracts-only` (or `"mode": "contracts-only"`) is
  recorded in the report and the sidecar. It refuses package and
  source-contract inputs.
* **Sidecar.** The sidecar records the resolved provider set.
* **Packages by id.** A source-contract configuration can name packages by
  provider id through `package_root`.
* **Identity.** Each run below is compared with the base binary's run of the
  example's configured route on the prepared Clang build:
  * the example itself, now a wrapper, is byte-identical in all five
    artifacts;
  * `quirin-audit current` with the Cycle 19 manifest is byte-identical;
  * the 47-module provider selection is byte-identical, command-line form and
    JSON form, except for the new `provider_selection` key in the sidecar;
  * the contracts-only run is byte-identical except for the new `evidence`
    key in the report and sidecar.
  The contract elaboration from the selection equals the manifest's, module
  for module, and the selection lowers to `cycle19-resource-contracts.json`
  byte for byte. The pinned arbiter's verdict-bearing artifact is
  byte-identical to the W44-COLLECTION-ALLOCATION integration baseline.
* **Jobs.** Both `quirin-audit current` and `quirin-audit CONFIG.json` take
  `--jobs N`, a JSON `"jobs"` field and `QCORE_AUDIT_JOBS`, in that order of
  precedence. The plain route now folds units in parallel too. On the corpus,
  the outputs are byte-identical at 1 and 16 jobs for `current` (full and
  contracts-only) and for the plain route (325 s → 67 s). The plain route at
  16 jobs is also byte-identical to the base binary's sequential run.
* **Measured residual.** Only 4 of the 11 provider crates can be selected
  whole. Exactly 47 of 111 owned modules are resource-v1 modules, and they are
  exactly the Cycle 19 manifest's modules (`W44-PROVIDER-RESOURCE-VIEW`).

## 1. What a user runs

A command line:

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
  --report "$out/reports/resource-diagnostics.json" --jobs 16
```

A JSON configuration (the contracts-only example), run with `quirin-audit current --config FILE`.
Paths are relative to the file:

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

Both examples were run on the prepared build in this card (§3.4).
[The crate README](../../../crates/qcore-audit/README.md) has the preparation
sequence that replaces the retired script:
`prepare_cmetrics_clang_audit.py`, then `prepare_cycle19_monkey.py`, then
`quirin-audit current`, with the `build/cycle19/clang/<rev>` paths the old
script produced. It also describes the plain `quirin-audit CONFIG.json`
route's `providers`, `load_path`, `quirin_providers_only` and `mode` fields.
[`scripts/manifest_to_providers.py`](scripts/manifest_to_providers.py) writes
the provider selection equal to a schema-2 manifest.

## 2. Decisions

### 2.1 Entries and the dependency closure

An entry is either `NAME`, which selects every module the crate owns in
load-path order, or `NAME:MODULE`, which selects one module. `NAME:MODULE` is
the spelling an `#include` target already uses. Entries keep their order.

Resolution first validates the whole load path through the existing reader.
The reader refuses a namespace collision (`QI017`), a dependency cycle, a
missing dependency, a listed module that is missing on disk, and a stale
publication pin. Resolution then refuses each of these:

- a name without a provider prefix;
- a crate not on the load path, with the error listing the known providers;
- a module the crate does not own;
- a crate with no module;
- a module selected twice;
- an empty selection.

**Closure semantics: a deviation from the brief's wording, stated.** The brief
says that selecting `quirin-provider-fluent-bit` "pulls in" libc and shared.
Here the closure is resolved, validated and recorded, and it is the scope in
which the selected modules' `#include`s resolve. The elaborator follows each
include over one declared edge. A dependency's own modules are **not**
selected because they are a dependency. There are three reasons:

- A contract-use edge means that one crate's contracts inherit from another's,
  not that the other crate's contracts should be audited too.
- Provider crates are data that nothing loads implicitly.
  `quirin-provider-shared`'s own documentation says so.
- Selecting a crate that way would pull in whole dependencies. A selection
  could then never express a manifest's module subset, and the identity
  acceptance could not hold. For example, `qcore-provider-chunkio` would pull
  in all of `quirin-provider-shared`, and none of its four modules is a
  resource module (§4.1).

One correction to the brief's example: on the load path,
`quirin-provider-fluent-bit` depends only on `quirin-provider-libc`.
`quirin-provider-shared` is the crate that depends on fluent-bit.

### 2.2 Lowering to a manifest

A selection is lowered to the schema-2 manifest a person would write beside
the load path's directory. A module that uses no inclusion is written as a file
path, which loads exactly as a schema-1 entry does. A module that uses
inclusion is written as a `crate:module` reference. `qcore_contract` gains
`load_resource_contract_manifest_text`, the unchanged manifest loader applied
to text already in hand. It also gains `uses_contract_inclusion`, the
predicate on which a path entry is refused. So a provider selection and an
equal hand-written manifest are one load, not two implementations of one.

The lowered text is recorded as `contract_manifest` and digested into every
compile-task identity, exactly as a manifest file is. Loading all 111 owned
modules one at a time confirmed the spelling rule on the real tree: every
module the Cycle 19 manifest spells as a reference uses inclusion, and no
other module does. With a load path spelled like the manifest's location, the
Cycle 19 selection lowers to the committed manifest byte for byte (test
`the_cycle19_provider_selection_is_the_cycle19_manifest_module_for_module`).

### 2.3 Command line and configuration: reject a conflict

Every setting of `quirin-audit current` can be a flag, a JSON field or both.
When both forms give a setting, they must agree. For providers, agreeing means
that they resolve to the same lowered manifest: the same modules in the same
order, on the same load path, with the same guard. Otherwise the command
refuses. Nothing is merged or overridden.

This rule was chosen over union for two reasons:

- A union has no natural order, and order is part of the recorded input: it
  is the manifest's digest.
- A union would let a configuration file silently widen a command line.

Under this rule a run is described completely by either form, and the sidecar
names which forms declared it (`provider_selection.sources`).

### 2.4 The contract source

Contracts come from exactly one declared source:

- a provider selection;
- an explicit manifest (`--contract-manifest` / `contract_manifest`);
- when neither is given, the manifest the profile declares.

A provider selection **supersedes a profile's declared default**. The sidecar
records that default as `profile_contract_manifest`, not read. A selection
**together with an explicit manifest refuses**, as two declarations of one
input. With no selection, no manifest and a profile that declares none, the
command refuses: there is no default provider set.

The plain `quirin-audit CONFIG.json` route follows the same rule with its
`providers` field. That route has always used the profile's manifest, and it
still does when `providers` is absent.

### 2.5 Contracts only

`contracts-only` admits resource projection from the elaborated `.qcore`
modules. It admits none of the following:

- packages or source-contract checks;
- verifier-host execution;
- compiler-crate plugin evidence;
- native or observed evidence;
- pinned historical evidence.

A configuration that names `source_contracts` (in the plain route) or
`packages` / `source_contracts` (in the current route's configuration) refuses.
Nothing is narrowed silently.

The report and the sidecar gain an `evidence` block. It records the mode, the
excluded kinds, and every contract module with the SHA-256 of its elaborated
text. Included parents stay listed under `contract_inclusion`.

**The configured current route used no process packages before this card**:
`run_current` built only a `CurrentAudit` and wrote Unknown placeholders.
On the same inputs, a contracts-only run therefore differs from a plain one
only in the `evidence` block and one console line (§3.3).

Provider selection is the same in both modes and admits both crate
families. The opt-in `--quirin-providers-only` guard refuses any
`qcore-provider-*` crate anywhere in the resolved closure. The Cycle 19
selection's closure holds five such crates, so the guard refuses it. **The
user may prefer to make the guard the default for contracts-only runs.** That
is a one-line change, left to the user because the brief called the question
ambiguous.

### 2.6 The route, the example and the retired script

`current_route.rs` holds the moved route, unchanged in behaviour. The following
moved with it verbatim: selection, the Clang driver check (including the
`/usr/bin/cc` refusal), the Monkey source-copy check, the ordered parallel
fold, and the Unknown sidecars. The example's pinned route now imports the
moved Monkey helpers and `source_weight` rather than keeping copies.

**The example's configured route stays as a thin wrapper, not removed.**
`compiler/tests/cycle19_current_command.rs` drives it. W44-AUDIT-PARALLEL-UNITS
and older reports document its command line. Keeping it costs about 40 lines,
and the wrapper-versus-base identity (§3.1) now tests the move itself.
Printing is the one difference. The CLI prints the source-set name and points
historical replay at the example (`fluent-bit …` / `… the pinned replay is
cycle19_fluent_bit_libraries_audit --pinned-evidence`). The wrapper keeps
`Fluent Bit …` / `… use --pinned-evidence …`.

`scripts/prepare_cycle19_clang_audit.py` is deleted. Its `--jobs` and
`--release` duties are `quirin-audit current --jobs N` and
`cargo run --release -p qcore-audit -- current …`. Its two preparation steps
remain their own scripts, unchanged.

Its `audit-command.json` fields now live in `<report>.invocation.json`:

- `source_revision` → `source_revision`;
- `clang` → `clang`, the identity that ran;
- `clangxx` → preparation's `build-provenance.json` and the Monkey marker,
  where the C++ compiler is used; the audit never runs it;
- `compile_database` and `monkey_database` → the same keys, each with a
  SHA-256;
- `cwd` and `command` → `cwd` and `argv`;
- `profile` → `build_profile`;
- `jobs` → `jobs.requested` and `jobs.effective`;
- `environment` → `environment`;
- `scope` → `scope`.

The record also gains the configuration file and its digest, and the contract
source with the loaded manifest's digest. It is provenance of the run, so it
is excluded from identity comparisons.

Every live reference was updated: the refusal text in `compile_db.rs` and its
pins in `report.rs` and `qcore-schemas/tests/reason_codes.rs`, the crate
README and `architecture.md`. The refusal's reason code is unchanged; it is
still classified by its unchanged prefix. `scripts/tests/test_retired_scripts.py`
fails if the script returns or if any live file names it. Frozen reports and
the increment history are exempt.

### 2.7 Packages by provider id

A source-contract configuration may give `"package_root": "DIR"`, a
`providers/package.py build` output directory, instead of `packages`. Each
binding's provider id names `DIR/<id>`, which is verified exactly as a listed
package is. Exactly one of the two fields is required. This was cheap: one
function in `source_contract.rs` and two tests. `load_with_packages` and every
existing configuration are unchanged.

### 2.8 Parallel units on every route

The user required parallel execution on every unit-analysing route, as the
retired script's `--jobs` gave. Both routes use W44-AUDIT-PARALLEL-UNITS's
`qcore_audit::parallel` without reimplementing it: `run_ordered`, the unit-order
fold, first-error-in-unit-order, `effective_jobs` (an active census sink forces
one job) and `heaviest_first` dispatch.

- `quirin-audit current` already used them through the moved route.
- `quirin-audit CONFIG.json` (`run_config`) was sequential and now folds the
  same way. Each unit records into its own report fragment, tally and console
  buffer, and source identities and compile tasks are folded in unit order. The
  source-contract step may dispatch a process provider, so it stays in the
  fold, one unit at a time, at the loop's position.

The count comes from these sources, in order of precedence:

1. `--jobs N` (the plain route takes `quirin-audit [--jobs N] CONFIG.json`);
2. the configuration's `"jobs": N`;
3. `QCORE_AUDIT_JOBS`;
4. the host's available parallelism, capped at 16.

`parallel::jobs_with_precedence` implements this, next to `resolve_jobs`. A
zero is an error, not a fallback. This precedence is the one exception to the
both-forms rule of §2.3: the job count changes wall time only, so a flag
overriding a configuration cannot change a run's meaning.

The requested count, the effective count and the source of the count are
recorded in `<report>.invocation.json`, on both routes. The brief asked for the
count "in the sidecar". It is kept out of the assumption sidecar deliberately:
that sidecar is part of what must be byte-identical across job counts, as it
has been since W44-AUDIT-PARALLEL-UNITS. The invocation record is the
provenance sidecar written next to it.

### 2.9 Dependency rule

`qcore-audit`'s normal dependency closure, as `cargo metadata` resolves it,
contains no `qcore-compiler`, `qcorec`, `qcore-provider-*` or
`quirin-provider-*` package (`tests/dependency_graph.rs`). The existing
`workspace_layering` guards stay green. Providers reach the audit only as data
(the load path and its `.qcore` texts) or as verified package directories.

## 3. Identity

All runs were made in this worktree with `QCORE_CLANG=/usr/bin/clang` and
16 jobs. They all used the same inputs as W44-AUDIT-PARALLEL-UNITS's
configured route:

- the prepared current-revision Clang build `build/cycle19/current-revision`
  (Fluent Bit `d1ab8abf`, 185 source files, 186 configurations);
- the pinned Monkey Clang database
  (`build/cycle19/c-gateway-wave-pin/monkey`, 29 configurations).

`build/cycle19/clang/<rev>` holds older preparations. The brief's pointer
resolves to this directory, the one the audit-parallel card actually ran on.

The **base** is the base commit's release example binary (`5315f09d…`):
3 Safe / 10 provisional Unsafe / 169 Unknown / 4 data-only / 0 gateway
failures.

Comparisons use [`scripts/compare_outputs.py`](scripts/compare_outputs.py). It
checks the five artifacts: the report, `.contracts.json`,
`.chunk-contracts.json`, `.source-effects.json` and `monkey-source.json`. It
also compares `audit.log` line by line and always masks the `preparation:`
wall-clock line, as W44-AUDIT-PARALLEL-UNITS documented. The script prints any
other masked or inserted line. Results are in [identity/](identity/).

### 3.1 The move

| Run | Binary | Report | `.contracts.json` | Unknown sidecars ×3 | `audit.log` |
|---|---|---|---|---|---|
| example (wrapper) | branch | identical | identical | identical | identical after `preparation:` |
| `quirin-audit current --contract-manifest` | branch | identical | identical | identical | + lines 1–2 (label, replay note) |

The manifest was passed spelled as the example spells it:
`<worktree>/compiler/../contract/profiles/fluent-bit/../../../cycle19-resource-contracts.json`.
With that spelling the file-path contract keys, and every digest derived from
them, are the same strings. §4.2 covers other spellings.

### 3.2 Providers

| Run | Report | `.contracts.json` | Other artifacts | `audit.log` |
|---|---|---|---|---|
| 47 × `--provider crate:module` (CLI) | identical | identical after removing `provider_selection` | identical | lines 1–2 |
| the same 47 in `providers` (JSON) | identical | identical after removing `provider_selection` | identical | lines 1–2 |

The CLI and JSON sidecars differ only in `provider_selection.sources`
(`["command_line"]` against `["configuration"]`). Their reports are
byte-identical to each other. The selection was
`manifest_to_providers.py cycle19-resource-contracts.json`: every manifest
entry as `crate:module`, in manifest order. The load path was spelled like the
manifest's location.

The module-for-module claim is a unit test on the real tree
(`provider_selection.rs`). The lowered text equals the committed manifest.
`source_assumptions` (the 47 keys and elaborated texts), `inclusion_evidence`
and the loaded function table all equal those of
`load_resource_contract_manifest(cycle19-resource-contracts.json)`. The
selection's closure is the ten selected crates plus `quirin-provider-shared`.

### 3.3 Contracts only

The JSON selection plus `--contracts-only` gives these results against the base:

- the report is identical after removing `evidence`;
- `.contracts.json` is identical after removing `provider_selection` and
  `evidence`;
- the other artifacts are identical;
- `audit.log` gains one inserted line 3 (`evidence: contracts-only (…)`) and
  differs in lines 1–2.

Against the manifest run of §3.1, the same holds with lines 1–2 equal.

### 3.4 Confirmation on the final binaries, and the README examples

Series 1 used the first branch binaries. Series 2 repeated all five runs on
the binaries built from `7d9c8efa`: example `5e138998…` and quirin-audit
`1c8ba761…`. All six comparisons gave the same result as in series 1, exit 0.
These are the five against the base plus manifest-against-contracts-only
(`identity/compare-*2.json`).

The README's two examples ran on the same build:

| Example | Units | Contract modules | Closure |
|---|---|---|---|
| CLI (libc whole + 2 modules) | 3 Safe / 3 provisional Unsafe / 176 Unknown / 4 data-only over 186 configurations | 18 | libc, fluent-bit and chunkio selected; cfl, mpack, msgpack and shared recorded as dependencies |
| JSON (contracts-only, `quirin_providers_only`, no Monkey database) | 2 Safe / 1 provisional Unsafe / 150 Unknown / 4 data-only over 157 configurations | 18 | libc, mpack and msgpack |

With fewer contracts, verdicts move towards Unknown (10 provisional Unsafe
against 3), not towards Safe. An undeclared callee is Unknown, never assumed.

### 3.5 Jobs

Late in the card, the user required `--jobs` on every unit-analysing route.
Both routes now fold with `qcore_audit::parallel` (§2.8). Identity at 1 and 16
jobs on the real corpus:

| Route | Jobs compared | Artifacts | `audit.log` | Wall time |
|---|---|---|---|---|
| `current`, 47 providers (JSON) | 1 vs 16 | all five byte-identical | identical but `preparation:` | 482 s vs 71 s |
| `current`, 47 providers, contracts-only | 1 vs 16 | all five byte-identical | identical but `preparation:` | 353 s vs 70 s |
| plain `CONFIG.json`, 47 providers | 1 vs 16 | report and `.assumptions.json` byte-identical | identical but `preparation:` | 325 s vs 67 s |
| plain `CONFIG.json`, profile manifest | base binary (sequential) vs branch at 16 | report and `.assumptions.json` byte-identical | identical but `preparation:` | — |

Plain-route inputs: a checkout-head configuration over the same prepared
build. The configuration selects four of the profile's five library groups
(155 units: 2 Safe / 5 provisional Unsafe / 144 Unknown / 4 data-only). The
`fluent_bit` group is left out because the production database compiles
`src/flb_input_chunk.c` twice, once into `fluent-bit-static` and once into
`fluent-bit-shared`. The plain route refuses that ambiguity ("conflicting
compilation commands"), and `compile_target` cannot narrow it: its object
prefix would also drop every library record.

The last row compares the base commit's own `quirin-audit`, built from an
archive of `a2a63ced`, with the branch binary at 16 jobs. It shows that
rewriting `run_config`'s loop into the fold changed no byte. The `current` j1
runs used quirin-audit `57759861…`, which is the final code, and the j16 runs
used `1c8ba761…`. `run_config`'s change does not touch the `current` route.

The job count's resolution is tested in
`parallel::tests::a_configured_route_prefers_flag_then_configuration_then_environment`
(pure precedence, including zeros and a malformed environment value). It is
also tested in `current_cli.rs::jobs_resolve_by_precedence_and_never_change_an_output`.
That test runs both routes on a six-unit fixture; the invocation record names
the source of the count (`command_line`, `configuration`, `environment`), and
the reports and sidecars are byte-identical at 1, 2, 3, 4 and 6 jobs.

[Digests of every run's artifacts](identity/run-digests.txt). Ten runs share
the base report's SHA-256 `1c2076fc…`: the base, both examples, both manifest
runs, both CLI and both JSON provider runs, and the JSON provider run at one
job. The three contracts-only runs share `7e5206da…`.

### 3.6 The pinned arbiter

`run_audit.py private/arbiter 7d9c8efa --jobs 16` ran on the final example
binary (`5e138998…`) and exited 0. `baseline_identity.py` against
`docs/reports/w44-collection-allocation/integration/artifacts`, with this
worktree aliased to the baseline's, gives:

- diagnostics, chunk-contracts and monkey-source: **byte-identical**;
- contracts: identical after the root alias;
- source-effects: differs only in the checkout-path-bound digests, as every
  run in another worktree does (W44-AUDIT-PARALLEL-UNITS §4.1);
- verdict-bearing identical: exit 0.

The analyzer is unchanged; no gateway, projection or contract text moved.

## 4. Measured residuals

### 4.1 Whole-crate selection (`W44-PROVIDER-RESOURCE-VIEW`)

Each crate was selected whole, alone. The run stops at a deliberately absent
compile database, which is read only after the contracts load. Four crates
load: `quirin-provider-libc`, `-mpack`, `-msgpack` and `qcore-provider-ctraces`.
Seven refuse on their first module that is not resource-v1.

Every one of the 111 owned modules was also selected alone:

| Crate | Resource-v1 modules | Refused |
|---|---:|---:|
| qcore-provider-chunkio | 3 | 41 |
| qcore-provider-cmetrics | 3 | 1 |
| qcore-provider-ctraces | 1 | 0 |
| qcore-provider-libevent | 3 | 12 |
| qcore-provider-librdkafka | 5 | 3 |
| quirin-provider-cfl | 2 | 2 |
| quirin-provider-fluent-bit | 12 | 1 |
| quirin-provider-libc | 16 | 0 |
| quirin-provider-mpack | 1 | 0 |
| quirin-provider-msgpack | 1 | 0 |
| quirin-provider-shared | 0 | 4 |
| **total** | **47** | **64** |

The 47 are exactly the Cycle 19 manifest's modules
([probe](identity/each-module.log)). The refusals are correct and fail-closed:
those modules are lifecycle, observation, policy and allocator-profile views
for other checkers. They do mean that `--provider NAME` alone is usable for
only four crates today. The follow-up asks crates to declare their resource
view in the load path.

### 4.2 Contract keys name the path spelling

This is not registered: it is a property of the existing loader that
W44-AUDIT-PARALLEL-UNITS already met as the arbiter's root alias. A file-path
module is keyed by the path as reached, and those keys feed the
`resource_contract_sources` digest and every compile-task identity.

The identities of §3 therefore depend on spelling the load path as the example
spells its manifest's location. The same 47-module selection was run with the
natural spelling, `--load-path <worktree>/providers/contract-load-path.json`
([leaf buckets](identity/leaf-buckets-base-vs-natural-report.json)). The report
then differs from the base in exactly 558 leaves:

- 372 are `resource_contract_sources` digests, two per configuration: the
  dependency map and the compile-task identity;
- 186 are `inventory_sha256` digests.

Every verdict, row and count is equal, and the lowered manifest's own digest
is equal. The sidecar differs only in the path spelling of its `contracts`
keys and in the new `provider_selection` key.

## 5. Tests and Lean

**Rust** (in `crates/qcore-audit`):

* `tests/provider_selection.rs`, 10 tests:
  * whole-crate order and lowering;
  * the closure is recorded but not selected, with roles, versions, published
    pins and the load-path digest;
  * unknown provider, non-provider name, missing module, duplicate and empty
    selection;
  * namespace conflict, dependency cycle, and a listed module missing on disk;
  * the guard, including a `quirin-provider-*` crate whose dependency is a
    `qcore-provider-*` crate;
  * the both-forms rule: two spellings of one set agree; another set, or the
    same set in another order, conflicts;
  * a selection loads exactly what the equal hand-written manifest loads;
  * the real Cycle 19 selection is the manifest, module for module.
* `tests/current_cli.rs`, 11 tests, which run the binary against a Clang
  fixture:
  * the command-line, configuration and both-form runs are identical;
  * conflicting forms refuse;
  * an unknown provider refuses before any report is written;
  * the contract-source rule, including no default provider set;
  * the reproducibility sidecar: the recorded manifest reloads the recorded
    texts, and the digests match the files;
  * the invocation record;
  * contracts only: mode, modules, excluded kinds, and nothing else changed;
  * the guard;
  * `/usr/bin/cc` refuses with the new instructions;
  * a current configuration naming packages or source contracts refuses;
  * the plain route: contracts-only with `source_contracts` refuses, and
    `providers` with contracts-only is recorded;
  * job precedence (flag, configuration, environment) on both routes, with
    byte-identical outputs across job counts on a six-unit fixture.
* `tests/dependency_graph.rs`, 2 tests: the resolved normal closure, and every
  manifest table.
* `tests/source_contract_config.rs`, +2 tests: `package_root` selects and
  verifies by id; a missing package, both inventories, neither inventory and
  an empty root all refuse.
* `current_cli` unit tests, 3; `parallel` unit test for the job precedence, 1.
* `scripts/tests/test_retired_scripts.py`, 2 tests.

**Lean.** `proof/QCore/ProviderSelection.lean`, with 31 theorems audited in
`proof/AuditProviderSelection.lean`, models what the Rust implements:

- expansion soundness;
- the closure walk: it contains every selected crate, is closed under
  dependencies, names only load-path crates, and refuses on exhaustion;
- `resolve_sound`, which gathers every fact an admitted resolution carries;
- refusal of an empty selection, an unknown provider, a missing module and a
  duplicate;
- the guard, and its refusal of a closure containing `qcore-provider-*`;
- lowering, which keeps the selection and spells references exactly for
  inclusion;
- the both-forms rule (`agree_ok_iff`);
- the contract-source choice, including `no_default_provider_set`;
- the job count: flag first, then configuration, then environment, then
  the capped default; zeros refuse, and an admitted count is positive;
- contracts-only, which admits only contract evidence, refuses a mixed
  request, and never narrows.

Rust–Lean correspondence is by review (constraint 28).

## 6. Gates

Measured on `04fee19a` in this worktree with `CARGO_INCREMENTAL=0`. Every exit
was captured to a file and no step was piped ([gate.json](gate.json); logs are
in the worktree-private `measure/gate/`, and the harness is
[`scripts/harness/gate.sh`](scripts/harness/gate.sh)). No many-job audit ran
during a test step.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `python3 scripts/run_test_binaries.py` | **101** | 5,624 passed / **1 failed** / 11 ignored. The failure was `std_socket_inet::this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`, which walks the worktree for `.qcore` examples and found this card's own measurement copy of the base tree under `private/base-src`. Attempt kept at `measure/gate/test.log`. |
| test (rerun, copy removed) | `python3 scripts/run_test_binaries.py` | 0 | **5,625 passed / 0 failed / 11 ignored**, 298 summaries over 288 units; one single-job audit ran beside it (load 3.0) |
| clean | `cargo clean --profile dev`, after each test step | 0, 0 | 21.5 GiB the second time |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings`; `cargo clippy --workspace --all-targets -- -D warnings` | 0, 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py --jobs 4` | 0 | 189 audits (+1: `AuditProviderSelection.lean`, 31 theorems) |
| offline build | `cargo build --offline --workspace` | 0 | no dependency added |
| python | `python3 -m unittest discover -s scripts/tests` | 1, then 0 | 351 tests. Before the matrix regeneration the two matrix self-consistency tests failed, as they must (`named_followups` 391 → 392, `lean_audits` 188 → 189). After `build_matrix.py . --gate …/gate.json` (exit 0), all 351 pass. |
| docs index | `python3 scripts/build_docs_index.py`; `--check` | 0, 0 | |
| packages | `python3 providers/package.py build --output measure/packages`; `verify` × 5 | 0; 0 × 5 | |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 124 passed / 0 failed / 0 ignored, 39 summaries |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0, 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | provider data unchanged |

Constraint 16's load-sensitive rows all passed in both test runs.

## 7. Files

* Code:
  * new in `crates/qcore-audit/src/`: `providers.rs`, `evidence.rs`,
    `current_route.rs`, `current_cli.rs`;
  * changed in `crates/qcore-audit/src/`: `current.rs`, `lib.rs` (the
    plain route's parallel fold, providers, mode, jobs), `parallel.rs`
    (`jobs_with_precedence`),
    `main.rs`, `report.rs`, `source_contract.rs`, `compile_db.rs`;
  * `crates/qcore-contract/src/{contract_loader.rs, contract_inclusion.rs}`;
  * `compiler/examples/support/cycle19_compatibility.rs`.
  Deleted: `scripts/prepare_cycle19_clang_audit.py`.
* Tests:
  * new: `crates/qcore-audit/tests/{provider_selection.rs, current_cli.rs,
    dependency_graph.rs}` and `scripts/tests/test_retired_scripts.py`;
  * extended: `crates/qcore-audit/tests/source_contract_config.rs`;
  * re-pinned refusal text: `crates/qcore-schemas/tests/reason_codes.rs` and
    the test in `report.rs`.
* Lean: `proof/QCore/ProviderSelection.lean`,
  `proof/AuditProviderSelection.lean`, `proof/QCore.lean`.
* Docs:
  * `crates/qcore-audit/README.md` (usage, JSON, contracts-only, preparation
    and provenance);
  * `docs/development/architecture.md`;
  * this report, `followups.json`, `scripts/` and `identity/`;
  * the wave-44 increment paragraph and `docs/STATUS.md` (ledger count).
