# W44-QUIRIN-AUDIT-PROVIDERS: providers for `quirin-audit`, `quirin-audit current`, contracts only

Card W44-QUIRIN-AUDIT-PROVIDERS, 2026-09-25, on base `a2a63ced` ("Record the
w44-collection-allocation integration and archive the combined baseline").
Mid-card, the user widened the scope twice: first to retire
`scripts/prepare_cycle19_clang_audit.py`, then to add a contracts-only route.
Both are part of this report.

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

The same selection as JSON, run with `quirin-audit current --config FILE`.
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

### 2.8 Dependency rule

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

TODO-SERIES2

### 3.5 The pinned arbiter

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
spells its manifest's location. With the natural spelling,
`--load-path providers/contract-load-path.json`, the same selection gives the
same verdicts under different digests. TODO-NATURAL

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
* `tests/current_cli.rs`, 10 tests, which run the binary against a Clang
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
    `providers` with contracts-only is recorded.
* `tests/dependency_graph.rs`, 2 tests: the resolved normal closure, and every
  manifest table.
* `tests/source_contract_config.rs`, +2 tests: `package_root` selects and
  verifies by id; a missing package, both inventories, neither inventory and
  an empty root all refuse.
* `current_cli` unit tests, 3.
* `scripts/tests/test_retired_scripts.py`, 2 tests.

**Lean.** `proof/QCore/ProviderSelection.lean`, with 24 theorems audited in
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
- contracts-only, which admits only contract evidence, refuses a mixed
  request, and never narrows.

Rust–Lean correspondence is by review (constraint 28).

## 6. Gates

TODO-GATES

## 7. Files

* Code:
  * new in `crates/qcore-audit/src/`: `providers.rs`, `evidence.rs`,
    `current_route.rs`, `current_cli.rs`;
  * changed in `crates/qcore-audit/src/`: `current.rs`, `lib.rs`,
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
