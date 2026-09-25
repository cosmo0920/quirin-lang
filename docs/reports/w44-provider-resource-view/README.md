# W44-PROVIDER-RESOURCE-VIEW: whole-crate provider selection, and contracts only loads all contracts

Card W44-PROVIDER-RESOURCE-VIEW, 2026-09-25, on base `fc6a0f14` ("Record the
w44-quirin-audit-providers integration gate"). It discharges the follow-up
[`W44-PROVIDER-RESOURCE-VIEW`](../w44-quirin-audit-providers/README.md#41-whole-crate-selection-w44-provider-resource-view).

Mid-card the user replaced the brief's second item. The brief asked for
`--quirin-providers-only` to become the contracts-only default. The user's
definition instead reads: "Contract only should be meant as using all of
loaded contracts without build settings." This card implements the first half:
the guard stays opt-in, and a contracts-only run with no selection loads every
crate's resource view. The second half, running without a compile database,
is a separate card for the integrator; `current` still reads the database.

## Headline

* **Resource views, declared and checked.** Every crate record in
  `providers/contract-load-path.json` carries `resource_view`: the owned
  modules the resource-contract loader accepts. `providers/contracts.py sync`
  generates it by running the loader, and `check` recomputes it. `check`
  refuses a view that names a refused module, omits an accepted one, or is
  missing. No list is kept by hand.
* **`--provider NAME` selects the view.** All eleven crates can now be
  selected whole. `quirin-provider-shared` has an empty view, so selecting it
  alone refuses; named with other crates it adds nothing. `NAME:MODULE` is
  unchanged.
* **Contracts only loads all contracts.** A contracts-only run with no
  `--provider`/`providers` and no `--contract-manifest` selects every crate
  on the load path whole. The sidecar and `<report>.invocation.json` record
  `"selection": "all-loaded"`. An explicit selection or manifest narrows the
  run. The full mode is unchanged.
* **Identity.** All eleven views together are exactly the Cycle 19 manifest's
  47 modules. They elaborate byte for byte as the manifest does: module keys,
  texts, inclusion evidence and function table. On the prepared Clang build,
  two runs are byte-identical, apart from the sidecar's `provider_selection`,
  to a manifest holding the same 47 entries in whole-crate order: all eleven
  crates named whole, and the all-loaded contracts-only default. Against the
  committed manifest, whose entries are in a different order, every verdict,
  row, count and console line is equal. Only the 558 order-bound digest leaves
  differ (§3.2). The pinned arbiter is verdict-bearing identical to the
  W44-COLLECTION-ALLOCATION baseline.

## 1. What a user runs

The simplest contracts-only run needs no provider:

```sh
QCORE_CLANG=$(command -v clang) cargo run --release -p qcore-audit -- current \
  --checkout "$out/fluent-bit-clang-source" \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database "$out/fluent-bit-clang-production/compile_commands.json" \
  --load-path providers/contract-load-path.json \
  --contracts-only \
  --report "$out/reports/contracts-only.json"
```

The same run as JSON is `{"schema_version": 1, …, "load_path":
"providers/contract-load-path.json", "mode": "contracts-only"}`. A whole
crate is named `--provider quirin-provider-fluent-bit`, which selects its twelve
resource modules. The eleven crates named whole select the Cycle 19 set in
the full mode too. The [crate README](../../../crates/qcore-audit/README.md)
has both examples, and the Quirin-only JSON example with
`--quirin-providers-only`.

## 2. Decisions

### 2.1 Where the view lives, and how it is checked

The view is a field of each crate's record in the load path, not of its
`contracts.json`. Three reasons:

- Only the five `qcore-provider-*` crates have a `contracts.json`, but all
  eleven crates are on the load path.
- The load path is what `quirin-audit` already reads.
- Package inventories, and so packages, stay unchanged: build and verify
  have nothing new to carry.

**What "the loader accepts" means.** `qcore_audit::providers::probe_resource_views`
defines it, and `cargo run -p qcore-audit --example provider_resource_views`
prints the result. For every owned module, the probe resolves `NAME:MODULE`
exactly as `quirin-audit` does. It lowers that module to a one-entry manifest
and loads it with the unchanged manifest loader. A module is in its crate's
view exactly when that load succeeds. The probe then loads each view as a
whole, and all views together, which is the all-loaded environment. If either
load fails, the probe refuses. A resolution failure, such as a stale load path,
is the probe's own error and is never recorded as a module refusal. The first
draft of the probe had that defect: a relative load-path spelling failed
resolution, and all 111 modules were recorded as "refused". The probe now
resolves an absolute path, and resolution errors propagate.

**`sync` and `check`.** `sync` writes the regenerated load path without views,
runs the probe over it, and writes the load path with the views. `check`
compares every other field first. It then runs the probe over the committed
file and compares each view, naming the difference:

```text
stale resource view in …/contract-load-path.json: quirin-provider-app declares
modules the resource loader refuses: ['app/policy.qcore']; run `python3
providers/contracts.py sync`
```

`contracts.py` stays in the provider workspace but now needs the repository
workspace to run the probe. `package.py build` runs `check`, so it needs it
too. The Rust load-path reader checks only the view's shape: sorted, unique
and owned, refusing with `QI014` otherwise. A missing view is not read as
"every module": a whole-crate entry for such a crate refuses.

The views, as the probe computes them
([probe output](identity/probe.json), with each refusal's loader message):

| Crate | Owned | View | Refused |
|---|---:|---:|---:|
| qcore-provider-chunkio | 44 | 3 | 41 |
| qcore-provider-cmetrics | 4 | 3 | 1 |
| qcore-provider-ctraces | 1 | 1 | 0 |
| qcore-provider-libevent | 15 | 3 | 12 |
| qcore-provider-librdkafka | 8 | 5 | 3 |
| quirin-provider-cfl | 4 | 2 | 2 |
| quirin-provider-fluent-bit | 13 | 12 | 1 |
| quirin-provider-libc | 16 | 16 | 0 |
| quirin-provider-mpack | 1 | 1 | 0 |
| quirin-provider-msgpack | 1 | 1 | 0 |
| quirin-provider-shared | 4 | 0 | 4 |
| **total** | **111** | **47** | **64** |

This reproduces W44-QUIRIN-AUDIT-PROVIDERS §4.1 module for module. No crate
needed a contract-text or loader change, so the brief's legitimate stop did
not arise.

### 2.2 Whole-crate semantics

`NAME` expands to the crate's view, in view order, which is sorted. A crate
with an empty view contributes nothing and is still recorded in the closure
with role `selected`. A selection that resolves to no module refuses. That
replaces the old refusal of a crate that owns no module; `contracts.py`
never generates one, because it refuses an empty contract directory. `NAME:MODULE` still accepts any owned module, and the
loader then refuses one outside the view. The closure record gains each
crate's `resource_view`.

### 2.3 Contracts only loads all contracts

`resolve_all_loaded` names every crate on the load path, in load-path order,
and resolves them like any selection. It records `selection: "all-loaded"`
and `sources: ["default"]`. Explicit selections record `selection:
"explicit"`. The rules, identical on `quirin-audit current` and the plain
`quirin-audit CONFIG.json` route:

- The default applies only in contracts-only mode, and only when there is no
  provider selection and no explicit manifest. This is the one default provider
  set.
- It needs the load path. Without one the run refuses. The profile's declared
  manifest is **not** a silent substitute, which is a behaviour change: before
  this card, a contracts-only run with no selection read the profile's
  manifest.
- Like any selection, it supersedes the profile's default. The sidecar
  records that manifest as not read, and says why.
- `--quirin-providers-only` stays opt-in. With the default it refuses, because
  five crates are `qcore-provider-*`. It never narrows.
- In the full mode, a load path without a selection still refuses. So does no
  selection, no manifest and no profile manifest.

`<report>.invocation.json`'s `contracts` object gains `selection`. It is
`all-loaded`, `explicit`, or `null` for a manifest. The plain route's record
gains the same key.

### 2.4 Order: why whole-crate identity is shown against a canonical manifest

A selection lowers to a manifest in selection order, and every compile-task
identity digests that text (`resource_contract_manifest`). The committed
`cycle19-resource-contracts.json` interleaves crates.
`qcore-provider-libevent`'s `intrusive_list.qcore` comes before
`msgpack/packer.qcore` and `mpack/writer.qcore`, and its `memory.qcore` and
`logging.qcore` come after them. Libc's modules are not sorted either. So no
selection of whole crates, in any crate order, lowers to that file's bytes.

Instead,
[`canonical_manifest.py`](scripts/canonical_manifest.py) writes the same 47
entries, spelled as the manifest spells them, in whole-crate order. The
whole-crate and all-loaded runs record exactly that text as
`contract_manifest`. Against it they are byte-identical. Against the committed
manifest they differ only in the digests of that text (§3.2). The loaded
environment does not depend on order: the Rust test compares the full loaded
state. Reordering the committed manifest would move the pinned arbiter's
manifest digest, so it is out of scope here and filed as
`W44-CONTRACT-MANIFEST-ORDER-DIGEST`.

## 3. Identity

Every run used the inputs of W44-QUIRIN-AUDIT-PROVIDERS §3, 16 jobs and
`QCORE_CLANG=/usr/bin/clang`:

- the prepared current-revision Clang build `build/cycle19/current-revision`
  (Fluent Bit `d1ab8abf`, 185 source files, 186 configurations);
- the pinned Monkey database.

The profile directory and load path are spelled as the example spells them:
`<worktree>/compiler/../contract/profiles/fluent-bit/../../../…`. The
[harness](scripts/harness/) is `build.sh`, `series.sh`, `base-and-arbiter.sh`
and `compare-all.sh`. Comparisons use W44-QUIRIN-AUDIT-PROVIDERS's
`compare_outputs.py`, which masks only the `preparation:` wall-clock line.

Series 1 ran on the first branch binaries. Series 2 is the evidence below: it
ran on the final code, `7df7b916`, with quirin-audit `ef82bcba…`, example
`c88622fc…` and base quirin-audit `57759861…`. The base digest is the
digest W44-QUIRIN-AUDIT-PROVIDERS reported for its final code. Results are in
[identity/](identity/).

### 3.1 Byte identity

Every run gives 3 Safe / 10 provisional Unsafe / 169 Unknown / 4 data-only /
0 gateway failures over 186 configurations. That is the manifest run's result.

| Run | Against | Report | `.contracts.json` | Other three artifacts | `audit.log` |
|---|---|---|---|---|---|
| all eleven crates whole (`whole2`) | canonical-order manifest (`canonical2`) | identical | identical after removing `provider_selection` | identical | identical but `preparation:` |
| contracts-only, no selection (`all-loaded2`) | canonical-order manifest, contracts-only (`canonical-co2`) | identical | identical after removing `provider_selection` | identical | identical but `preparation:` |

The canonical-order manifest was written by `canonical_manifest.py` beside
`cycle19-resource-contracts.json`, at the worktree root, for the series only.
It is archived as [identity/cycle19-canonical-order.json](identity/cycle19-canonical-order.json).
Both the whole-crate run and the all-loaded run record it as
`contract_manifest`, byte for byte
([lowered-is-canonical2.json](identity/lowered-is-canonical2.json)). The
all-loaded sidecar records `selection: "all-loaded"` over 47 modules.
Series 1 gave the same whole-against-canonical result on the first branch
binaries.

### 3.2 Against the committed Cycle 19 manifest

| Run | Against | Report | `.contracts.json` | Other three | `audit.log` |
|---|---|---|---|---|---|
| `whole2` | `manifest2` | 558 leaves | `contract_manifest` only (after removing `provider_selection`) | identical | identical but `preparation:` |
| `all-loaded2` | `manifest-co2` | 558 leaves | `contract_manifest` only (after removing `provider_selection`) | identical | identical but `preparation:` |
| `canonical2` (a plain manifest, no provider code) | `manifest2` | 558 leaves | `contract_manifest` only | identical | identical but `preparation:` |

The 558 leaves fall into two buckets
([leaf buckets](identity/leaf-buckets-manifest-vs-whole2.json)):

- 372 `resource_contract_manifest` digests, two per configuration: the
  dependency map and the compile-task identity;
- 186 `inventory_sha256` digests.

No verdict, row, count or console line differs. The third row shows that
reordering a hand-written manifest alone produces exactly this difference:
the difference is the order, not the provider route.

### 3.3 Full mode unchanged

`base2` is the base commit's `quirin-audit` (`57759861…`). It ran with
`--contract-manifest cycle19-resource-contracts.json`. `manifest2` is the
branch binary on the same inputs. All five artifacts are byte-identical, and
`audit.log` differs only in the masked `preparation:` line
([comparison](identity/compare-base-vs-manifest2.json)). The Rust tests
cover the other full-mode rules (§4).

The base binary refuses the new load path. Its reader denies unknown fields
and reports `unknown field resource_view`. Series 1's base run shows this, and
it is kept as `private/runs/base1`. So the base run alone read the base
commit's load path: `base-and-arbiter.sh` rewrote the file's bytes for that one
run, rewrote them back, and asserted the SHA-256 round trip. No other run was
in flight. An older `quirin-audit` therefore refuses a newer load path,
fail-closed. The new reader accepts a load path with no views, and then
refuses only whole-crate entries.

### 3.4 The pinned arbiter

`run_audit.py private/arbiter2 <tip> --jobs 16` ran on the final example
binary (`c88622fc…`) and exited 0 ([run](identity/arbiter-run2.json)).
`baseline_identity.py` compared it with
`docs/reports/w44-collection-allocation/integration/artifacts`, with this
worktree aliased to the baseline's
([result](identity/baseline-identity-arbiter2.json)):

- diagnostics, chunk-contracts and monkey-source are **byte-identical**;
- contracts are identical after the root alias;
- source-effects differ only in the checkout-path-bound digests, as every run
  in another worktree does (W44-AUDIT-PARALLEL-UNITS §4.1);
- `verdict_bearing_identical: true`.

No analyzer, gateway, projection or contract text changed.

## 4. Tests and Lean

**Rust** (`crates/qcore-audit`):

* `tests/provider_selection.rs`, 17 tests (+7):
  * a whole crate selects its view, not every owned module, and the probe
    computes that view;
  * an empty view contributes nothing, and a selection of nothing refuses;
  * a crate that declares no view cannot be selected whole, but its modules
    can be named one by one;
  * an unsorted view, a duplicate entry or an unowned module refuses the
    whole load path (`QI014`);
  * all-loaded equals naming every crate, and the guard refuses it;
  * on the real tree, each of the 11 crates selected whole is exactly its
    probe-computed view and loads (shared refuses as empty, 47 in total);
  * all eleven whole and all-loaded give the same lowered text. It holds the
    manifest's 47 entries with the same spellings and load path, and
    elaborates the manifest's `source_assumptions`, `inclusion_evidence` and
    `functions` exactly.
* `tests/current_cli.rs`, 12 tests (+1). The new test covers all-loaded on the
  command line and in JSON:
  * byte-identical to naming the crates;
  * `selection` recorded in the sidecar and the invocation record;
  * narrowing by a selection and by a manifest;
  * refusal without a load path;
  * the guard's refusal;
  * the unchanged full mode: a load path without a selection refuses, and the
    profile default is read with no selection;
  * the plain route's default and refusal.

  The fixture load paths gained views.

**Python** (`providers/tests/test_contract_load_path.py`, 15 tests, +2). The
synthetic trees use a stand-in loader. The new tests cover:

- `sync` writes the view, and `check` refuses a declared refused module, an
  omitted accepted one and a missing view. The file is restored by rewriting
  bytes, and the round trip is asserted.
- A probe that cannot run is an error, not an empty view. So are an unowned
  view and a probe that omits a crate.

The real-workspace test runs the real probe and checks 47 modules in total,
an empty `quirin-provider-shared` and 16 libc modules.

**Lean** (`proof/QCore/ProviderSelection.lean`, 45 theorems audited in
`AuditProviderSelection.lean`, +14):

- `Crate` gains `view : Option (List Module)` and `ViewsOwned`.
- `expand (.whole c)` is the view. `expand_whole_is_view` and
  `expand_whole_without_view_refused` prove it.
- `resolve` refuses an empty resolution (`resolve_empty_view_refused`). With
  `ViewsOwned`, `resolve_sound` now also yields non-emptiness.
- `allLoaded`, and `resolve_allLoaded_is_allViews`: with distinct names and
  every view declared, it resolves to the concatenated views in load-path
  order. `findCrate_self` and `expandAll_whole_crates` support it.
- `chooseSourceIn` per mode:
  - `full_mode_unchanged` (by `rfl`);
  - `contracts_only_default_is_all_loaded`;
  - `contracts_only_all_loaded_needs_load_path`;
  - `contracts_only_explicit_narrows`;
  - `allLoaded_only_when_contracts_only`.

It has no placeholders and no project axioms. The Rust–Lean correspondence is
by review (constraint 28).

## 5. Gates

Measured on `d9c2de54` in this worktree with `CARGO_INCREMENTAL=0`. Every
exit was captured to a file and no step was piped ([gate.json](gate.json);
harness [`scripts/harness/gate.sh`](scripts/harness/gate.sh); logs in the
worktree-private `measure/gate/`). No audit ran during the test step.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `python3 scripts/run_test_binaries.py` | 0 | **5,633 passed / 0 failed / 11 ignored**, 298 summaries over 288 units (+8: provider selection +7, current CLI +1); load 16.8 at the start |
| clean | `cargo clean --profile dev` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings`; `cargo clippy --workspace --all-targets -- -D warnings` | 0, 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py --jobs 4` | 0 | 189 audits (`AuditProviderSelection.lean`: 45 theorems, +14) |
| python | `python3 -m unittest discover -s scripts/tests` | 0, then 0 | 351 tests, before and after the matrix regeneration (`build_matrix.py . --gate …/gate.json`, exit 0; `named_followups` 392 → 393) |
| offline build | `cargo build --offline --workspace` | 0 | no dependency added |
| contracts | `python3 providers/contracts.py check` | 0 | views recomputed by the loader and equal |
| provider Python | `python3 -m unittest discover -s providers/tests` | 0 | 15 tests (+2) |
| packages | `python3 providers/package.py build --output measure/gate/packages`; `verify` × 5 | 0; 0 × 5 | inventories unchanged |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 124 passed / 0 failed / 0 ignored, 39 summaries |
| isolated mutation test | `cargo test --manifest-path providers/Cargo.toml -p qcore-provider-conformance --test packages --locked --offline -- --exact real_packages_preserve_unknown_and_reject_mutations --test-threads=1` | 0 | 1 passed |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0, 0 | |
| docs index | `python3 scripts/build_docs_index.py`; `--check` | 0, 0 | |

Constraint 16's load-sensitive rows all passed. The mutation check that
`contracts.py check` refuses a wrong view is the Python suite's (§4). The
real file was never mutated for it.

## 6. Files

* Code:
  * `crates/qcore-contract/src/load_path_manifest.rs`: the optional,
    shape-checked `resource_view` and `LoadPathManifest::resource_views`;
  * `crates/qcore-audit/src/providers.rs`: view expansion,
    `resolve_all_loaded`, `probe_resource_views` and the record;
  * `crates/qcore-audit/src/current_cli.rs` and `lib.rs`: the contracts-only
    default and the invocation `selection`;
  * `crates/qcore-audit/examples/provider_resource_views.rs`;
  * `providers/contracts.py`;
  * the regenerated `providers/contract-load-path.json`.
* Tests: `crates/qcore-audit/tests/{provider_selection,current_cli}.rs` and
  `providers/tests/test_contract_load_path.py`.
* Lean: `proof/QCore/ProviderSelection.lean` and
  `proof/AuditProviderSelection.lean`.
* Docs:
  * `crates/qcore-audit/README.md`: its providers and contracts-only sections
    only;
  * `providers/README.md`;
  * the resource-view amendment in `docs/agents/providers.md`;
  * `docs/development/architecture.md`;
  * `docs/STATUS.md`;
  * the wave-44 increment paragraph;
  * this report, `followups.json`, `scripts/` and `identity/`.

## 7. Follow-ups

* Retired: `W44-PROVIDER-RESOURCE-VIEW`. Its discharge condition is met:
  - each crate declares its view, generated by `sync` and checked by `check`;
  - `NAME` selects the view;
  - a test shows that the ten Cycle 19 crates selected by name load exactly
    the manifest's environment. The eleventh, `shared`, has an empty view.
* New: `W44-CONTRACT-MANIFEST-ORDER-DIGEST` (§2.4).
* New: `W44-RESOURCE-VIEW-REFUSAL-UNCLASSIFIED`. A view excludes every module
  the loader refuses, so a broken resource module would be excluded at `sync`
  time rather than flagged. `check` and the pinned 47 guard the committed
  state.
* Not filed, because the integrator owns it: running contracts-only without
  build settings (no compile database).
