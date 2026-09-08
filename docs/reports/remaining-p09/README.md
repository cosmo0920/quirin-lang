# P09 — Remove one parity-proven legacy route at a time

Workstreams W0/W5/W7. Refactor increment: **one** redundant route is deleted,
with its own re-measured parity evidence archived in the commit *before* the
deletion. Nothing is promoted: no raw Cycle 19 unit changes class, no CT case is
mapped, no contract-corpus count moves, no diagnostic code is added or changed,
and no checking rule is touched.

- Worktree branch checkpoint before this card: `c60fd91` (second delegated
  increment consolidated into the status ledger).
- Baseline reproduced before any change, from a clean worktree with the
  Libevent submodule initialised at the pinned `d82464a2…`:
  **1141 workspace tests, 56 provider tests, 43 Python tests, 23 Lean audits**,
  both reevaluation examples exit 0, `fmt` and `clippy -D warnings` clean in
  both workspaces. That matches the ledger's recorded merged gate exactly.

## 1. Dependency note: P08

The card names P08 as its landed dependency. **No `docs/reports/remaining-p08/`
exists at this checkpoint** and no P08 branch is merged into this worktree's
history, so P08's bounded composition audit is not available as a parity input.
This card therefore uses the parity dossiers that *are* landed (P04, P05,
W2 H6 provider discovery) and re-measures the selected one itself. Recorded as
an integrator decision in §8, not silently assumed.

## 2. Route selection, by measurement

The card's guidance names P04's in-process ChunkIO unsigned unit-counter route
as the strongest dossier and asks for a recorded reason if its parity scope
turns out narrower than its callers. It is. The full census is
`docs/reports/remaining-p09/route-census.json`; the summary:

| Candidate legacy route | Replacement | Selected | Measured reason |
|---|---|---|---|
| `verifier_plugins::chunkio::teardown_source::check_counter_sources` | `qcore.chunkio` 0.5.0 process capability + `CounterSourceTask` (P04) | **No** | Two live production call sites in `compiler/examples/support/cycle19_compatibility.rs` (lines 554 and 701). One feeds `MemoryCloseDependencies.counter`; the other serialises the `unsigned_counters` record into the pinned teardown artifact. P04 proves *certificate equality*; the callers need *a certificate produced with no provider process*, and P04 §8 records that no configured audit route selects the counter capability yet. Deleting it would change the pinned artifacts, and re-measuring them is impossible here (see §6). |
| `verifier_plugins::libevent::event_driven::check_event_program` | `qcore.libevent` 0.2.0 process capability + `ReactorCallbackTask` (P05) | **No** | The replacement is a witness-replay route needing a registered package and a derived evidence-v2 task; it cannot answer the in-process signature. Its callers are `external_integration.rs:230` (Monkey composition) and the whole Cycle 18 model suite. P05 itself records the moved capability has no production caller. Deleting the checker would be capability removal, not consolidation. |
| `cycle19_compatibility.rs::SourceContractConfiguration::Direct` | `SourceContractConfiguration::Process` | **No** | The archived parity (`docs/reports/w5-configured-source-contracts/audit-parity.json`) is between the two *process* selectors, not between `Direct` and `Process`. The process route additionally emits `provider_source_effect` records, so the two are not byte-identical, and re-measuring needs the same unavailable corpus. |
| **`verifier_plugins::{libevent,chunkio}::{manifest, provider}`** | `first_party::{load_first_party_manifests, load_first_party_registry}` | **Yes** | Zero production callers; the replacement is already the default everywhere; parity fully re-measurable here without the pinned corpus; and the legacy route is strictly *less* capable than its replacement (§4). |

### What the selected route was

Two per-plugin helpers, one pair per first-party plugin, left over from the
pre-discovery era:

```rust
pub fn manifest() -> Result<PluginManifest, PackageLoadError> { /* discover, then filter by literal id */ }
pub fn provider() -> Result<BuiltInProvider, PackageLoadError> { BuiltInProvider::new(manifest()?) }
```

W2 H6 (`docs/reports/w2-provider-discovery/README.md`) already replaced the
first-party manifest literals with atomic profile-package discovery and recorded
"no compiled fallback". These two pairs were the last hardcoded per-plugin
*selection* route on top of that discovery: they re-read the same validated
profile packages and then pick one plugin by a literal id, and they force the
built-in adapter regardless of what the manifest declares.

`libevent::provider()` / `chunkio::provider()` / `libevent::manifest()` /
`chunkio::manifest()` had **no** caller in `compiler/src`, `crates/`, `providers/`
or any example — only `compiler/tests/cycle20_verifier_plugins.rs` (3 lines).

## 3. Re-measured parity (archived before the deletion)

Commit `3369b94` adds a temporary harness,
`compiler/tests/first_party_route_parity.rs`, and archives its output as
`docs/reports/remaining-p09/route-parity.json`. Reproduce with:

```sh
QCORE_P09_PARITY_OUT="$PWD/docs/reports/remaining-p09/route-parity.json" \
  cargo test -p qcore-compiler --test first_party_route_parity
```

Measured, for `qcore.libevent` and `qcore.chunkio`:

| Property | Old route | New route | Result |
|---|---|---|---|
| Manifest as a record | `libevent::manifest()` / `chunkio::manifest()` | `load_first_party_manifests(profiles_root())`, filtered | **equal** |
| Manifest as serialised JSON | 711 bytes / 710 bytes | same | **byte-identical** |
| Manifest carried by the constructed provider | `provider().manifest()` | discovered manifest | **equal** |
| v1 `verify` outcome, every declared capability | 4 (libevent: resource_contracts, event_traces, concurrency, observation_hooks) + 3 (chunkio: resource_contracts, chunk_lifecycle, translation_unit_projection) | `load_first_party_registry(profiles_root())` | **equal on all 7**; every one `Unknown`, no certificate, diagnostic `<plugin>.semantic_evidence_unavailable` |
| Foreign target (`not-a-declared-library`) | refused | refused | **equal**, `Unknown` |
| Undeclared capability | refused | refused | **equal**, `Unknown` |
| Absent profile root | — | `load_first_party_manifests` and `load_first_party_registry` both return `PackageLoadError` | no fallback |

The archived JSON records the per-capability verdicts, diagnostics and manifest
byte counts, plus the profile root, protocol version and evidence version the
measurement ran under.

Two of these properties were already partially pinned in-tree: the pre-existing
`extracted_plugins_preserve_manifests_and_fail_closed_for_every_capability`
asserted `registry.verify(&request) == first_party.verify(&request)` for the
same capability set. The harness widened that to manifest bytes, the foreign
target, the undeclared capability and the entrypoint gap, and archived it.

## 4. Why the legacy route was worth removing, not just redundant

The hardcoded route always constructs a `BuiltInProvider`. The discovery route
matches on `manifest.entrypoint` and constructs a `ProcessProvider` for
`PluginEntrypoint::Executable`. Given the *same* manifest with an executable
entrypoint and a valid package digest, the measurement records
`BuiltInProvider::new(...) == Err(ManifestError::InvalidTarget)`.

That is a **capability gap, never a promotion**: the legacy route fails closed
(a refusal, or an always-`Unknown` built-in adapter), it never admits anything
the discovery route would refuse. But it became a live foot-gun when P04 and
P05 shipped executable packages `qcore.chunkio` 0.5.0 and `qcore.libevent`
0.2.0: a caller reaching for `chunkio::provider()` against a profile root
carrying those packages gets a refusal or an inert built-in, where the surviving
route gets the real process capability. Removing the route removes that choice.

## 5. The deletion

Commit `<deletion>` (separate, and separately reviewable):

| File | Change |
|---|---|
| `compiler/src/verifier_plugins/libevent/mod.rs` | `manifest()`, `provider()` and their `BuiltInProvider`/`PluginManifest` imports deleted; a pointer comment to the surviving route added |
| `compiler/src/verifier_plugins/chunkio/mod.rs` | same |
| `compiler/tests/cycle20_verifier_plugins.rs` | the one consumer retargeted at `first_party_manifests()` / `first_party_registry()`, keeping every existing assertion, and gaining a pin that an executable manifest is refused by `BuiltInProvider::new` |
| `compiler/tests/first_party_route_parity.rs` | the temporary harness removed (its measurement is archived) |
| `compiler/Cargo.toml`, `crates/qcorec/Cargo.toml`, `Cargo.lock` | `qcore-compiler` `0.1.0` → **`0.2.0`** |

Totals: 7 files changed, 47 insertions, 266 deletions. Production deletion is
50 lines across the two `mod.rs` files; the rest is the temporary harness and
the retargeted test.

### Public API: explicitly versioned

Removing `qcore_compiler::verifier_plugins::{libevent,chunkio}::{manifest,
provider}` is a breaking change to a public path, so `qcore-compiler` is
versioned `0.1.0` → `0.2.0` (acceptance 3's "explicitly versioned" branch) and
the only dependent, `crates/qcorec`, is updated to match. Every other public
compatibility path is untouched: the `pub use` surface in `compiler/src/lib.rs`,
the `verifier_plugin`/`external_source`/`counter_source`/`chunk_contract`
facades and the layering guard's pinned compatibility function pointers all
still resolve, and `compiler/tests/workspace_layering.rs` passes unchanged
(11 tests).

### Deliberate pin correction

`reproducibility_metadata_does_not_establish_semantic_evidence` asserted
`evidence.reproducibility.provider_version == env!("CARGO_PKG_VERSION")`. The
value the code publishes is the plugin **package** version from the manifest
(`0.1.0`), which coincidentally equalled this crate's version. Versioning the
crate exposed the coincidence. The assertion is **retargeted at
`manifest.package_version`** — the value the code actually publishes — with the
reason recorded in the test. No expectation is deleted and nothing is relaxed:
the test still pins that reproducibility metadata is metadata, and the
`package_digest` assertion is untouched.

## 6. What was deliberately *not* re-measured

The four pinned 185-unit Cycle 19 artifacts were **not** re-measured, and are
untouched by construction rather than by measurement. In this worktree the
developer Fluent Bit checkout is at `90a010910`, off the profile pin, there is
no prepared Clang CMake build tree, and the audit's own pin gate refuses to run.

That is exactly why the counter and `Direct` routes were not selected: deleting
either changes what the pinned artifacts contain, and invariant 3 forbids
deletion without the diff. The selected route has **zero** reachability from
`compiler/examples/` — verified by search — so the audit artifacts cannot depend
on it. CORPUS-REPIN remains open and now also names P09 as a route change made
without re-measurement (a no-op one, by construction).

## 7. Why Lean parity is unaffected

No semantic rule, checker, model, diagnostic code or certificate shape changed.
The deleted functions performed manifest lookup and provider construction; they
sit above every checked model and appear in no Lean definition or theorem. The
surviving route's behaviour is unchanged — the parity measurement is exactly the
claim that its outputs are the ones the deleted route produced — so every
existing proof that covers a first-party provider's *result* still covers it:
the v1 refusal discipline (`QCore.PluginProtocol` family, `Composition`
namespace from E03), the reactor model (`QCore.EventDriven`,
`QCore.EventComposition`) and the ChunkIO contract families are all stated over
model and certificate objects, not over which Rust helper constructed the
provider. Nothing in `proof/` references the deleted symbols.

The gate was re-run anyway: `lake build` completes 67 jobs and
`check_lean_audits.py` passes **23 audits**, unchanged from baseline, with no
new axiom.

## 8. Integrator decisions

1. **P08 unavailable.** Recorded in §1; the card proceeded on P04/P05/W2-H6
   dossiers plus its own re-measurement rather than waiting or pretending.
2. **Route reselected away from P04's counter route**, with a measured reason
   (§2) rather than the guidance's default. The card explicitly allows this.
3. **Both plugin instances deleted in one commit.** They are two instances of a
   single route (the same four-line pattern, the same replacement, the same
   parity measurement); deleting only one would leave an asymmetric public API
   for no benefit. This is not batching several *different* routes, which the
   card forbids.
4. **`qcore-compiler` versioned rather than kept compatible.** Keeping a
   deprecated shim would have retained the route the card asks to remove.
5. **One pinned assertion retargeted** (§5), recorded rather than silently
   changed.
6. **AGENTS.md / CLAUDE.md / `ct_manifest.json` / `acceptance.json` /
   `acceptance_matrix.json` were not edited**, per the card. Proposed ledger
   text is in §10.

## 9. Gate results (after the deletion)

| Gate | Baseline | After |
|---|---|---|
| `cargo test --workspace` | 1141 passed, 0 failed | **1141 passed, 0 failed** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | 56 passed | **56 passed** |
| `python3 -B -m unittest discover -s scripts/tests` | 43 passed | **43 passed** |
| `cd proof && lake build` | 67 jobs ok | **67 jobs ok** |
| `python3 -B scripts/check_lean_audits.py` | 23 audits | **23 audits** |
| `cargo fmt --all -- --check` (both workspaces) | clean | **clean** |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean | **clean** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | **exit 0** |

The workspace test count is unchanged at 1141: the temporary harness (+1) was
added and removed within this card, and the retargeted consumer test is still
one test. Provider-absent and dependency behaviour after the deletion:
`workspace_layering` (11 tests) passes unchanged, and the absent-selection tests
(`absent_selections_contracts_and_unsupported_flags_fail_closed`,
`an_absent_provider_package_rejects_the_configured_source_contract_route`,
`absent_ambiguous_or_unsupported_field_models_never_gain_fallback_routes`,
`absent_callbacks_and_wrong_revision_do_not_create_summaries`) pass. P04's
`counter_route_parity` (2 tests) and P05's `reactor_route_parity` (4 tests) pass
unchanged — both retained routes are untouched.

`lake` is not on the default `PATH` on this host; it was run from `~/.elan/bin`.
The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`.

## 10. Remaining legacy routes (named follow-ons)

Each is a separate future P09 unit, with the blocker that must clear first:

- **P09-A · ChunkIO in-process counter route** (`check_counter_sources`).
  Blocked on (i) a configured audit route that selects the `qcore.chunkio`
  counter capability, so the Cycle 19 example can obtain the certificate from
  the process route, and (ii) CORPUS-REPIN, so the resulting artifact diff can
  be measured rather than asserted.
- **P09-B · Libevent in-process event-program route**
  (`event_driven::check_event_program`). Blocked on a replacement that can serve
  an in-process caller — i.e. the Monkey composition and the Cycle 18 suite
  moving onto derived tasks and replayed witnesses. Until then the process
  capability is an addition, not a replacement.
- **P09-C · `SourceContractConfiguration::Direct`** in the legacy audit example.
  Blocked on Direct-vs-Process artifact parity being measured (it is currently
  known *not* to be byte-identical: the process route adds
  `provider_source_effect` records) and on CORPUS-REPIN.
- **P09-D · `QCORE_CHUNKIO_PACKAGE` selector alias.** Parity between the two
  process selectors is archived
  (`docs/reports/w5-configured-source-contracts/audit-parity.json`), but the
  alias is documented in `docs/CYCLE19_BUILD_INPUTS.md`, `providers/README.md`
  and several archived validation records; removing it is a user-facing change
  that should follow, not precede, the `Direct` removal.
- **P09-E · Legacy out-of-protocol reactor record-exchange fixture**
  (`providers/conformance/src/bin/callback_proposal_writer.rs` +
  `tests/reactor_callbacks.rs`). Deliberately **not** deleted: its
  `MissingOperation` / `DuplicateOperation` / `UnsupportedVersion` refusals are
  not covered by P05's `reactor_callback_capability.rs` matrix, so deleting it
  today would delete pinned expectations. It becomes a candidate once that
  matrix is extended.
- **P09-F · Compiler compatibility facades** (`verifier_plugin.rs`,
  `external_source.rs`, `counter_source.rs`, `chunk_contract.rs`,
  `research_extensions.rs`, `cleanup_outcome.rs`, `source_call_use.rs`,
  `typed_source.rs`, `list_unlink_contract.rs`, `field_destructor.rs`,
  `counter_contract.rs`). These are pure re-exports whose canonical crate paths
  are pinned by `workspace_layering.rs`; removing them is a public-API
  consolidation that should be one deliberate versioned unit after the analysis
  routes above, not interleaved with them.
- **P09-G · `import_chunk_source`** (the zero-helper convenience wrapper in
  `chunk_source.rs`). Test-only today; trivially removable, but it is a
  convenience overload rather than a route, so it is listed for completeness.

## 11. Proposed ledger text (not applied here)

For the AGENTS.md / CLAUDE.md status paragraph of the wave that merges this
card:

> - Route removal (P09): the per-plugin hardcoded first-party provider
>   selection route `verifier_plugins::{libevent,chunkio}::{manifest, provider}`
>   is deleted after its parity with profile-package discovery was re-measured
>   and archived in the preceding commit — equal manifest records and manifest
>   bytes, equal v1 outcomes across all seven declared capabilities, equal
>   foreign-target and undeclared-capability refusals, and no fallback when the
>   profile root is absent. The deleted route forced a built-in adapter and
>   refused an executable entrypoint with `ManifestError::InvalidTarget`, a
>   capability gap that went live when P04 and P05 shipped executable packages;
>   the surviving route honours the declared entrypoint. `qcore-compiler` is
>   explicitly versioned 0.1.0 → 0.2.0 for the removed public path, with
>   `crates/qcorec` updated. One pinned assertion was deliberately retargeted
>   from `env!("CARGO_PKG_VERSION")` to the manifest's declared package version,
>   the value the code publishes. P04's ChunkIO counter route and P05's Libevent
>   event-program route were examined and **not** deleted: their replacements
>   cannot serve their remaining callers, and the pinned 185-unit artifacts
>   cannot be re-measured in this worktree. Gate unchanged at 1141 workspace
>   tests, 56 provider tests, 43 Python tests, 23 Lean audits. Evidence:
>   `docs/reports/remaining-p09/README.md`.

Nothing is promoted by this card: raw Cycle 19 units stay 2 Safe / 42
provisional Unsafe / 137 Unknown plus four data-only units, CT coverage stays
15/50 mapped, the contract corpus stays 59 modules, `qcore-schemas` stays 5.0.0
and full language conformance stays false.
