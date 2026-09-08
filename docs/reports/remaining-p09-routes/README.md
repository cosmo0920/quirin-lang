# P09-ROUTES — disposition of the seven named legacy routes P09-A..G

Workstreams W0/W5/W7. This card takes the seven follow-on routes
[P09](../remaining-p09/README.md) §10 named after its own removal, re-measures
each one against hard invariant 3 ("parity before deletion"), removes the ones
whose replacement is live and whose parity can be shown, and records a precise
missing-parity statement for the ones whose replacement cannot serve their
callers.

**One route is removed (P09-G) with an archived parity measurement. Six are
retained (P09-A..F), each with a named gap measured here rather than inherited
from the earlier report.** Nothing is promoted: no verdict, diagnostic code,
contract, profile, pinned count or wire byte changes, and no pinned artifact is
touched. One public API is removed, so `qcore-compiler` is versioned
**0.2.0 → 0.3.0** (flagged in §6).

- Worktree branch checkpoint: cut from `60939ff` (wave-7 consolidation); the
  worktree's own base is `8d0790a`, which carries two later main commits (the
  `quant[| |]` delimiter alias and its merge). The recorded workspace-test
  baseline below is therefore 1752, not wave 7's 1747; the delta is those two
  commits, measured before any change of mine.
- Libevent submodule initialised at the pinned `d82464a2…`.

## 1. What was measured, and how to re-measure it

```sh
python3 -B docs/reports/remaining-p09-routes/census.py > \
  docs/reports/remaining-p09-routes/route-census.json
```

[`route-census.json`](route-census.json) is mechanical: for each route it
records the definition site, every textual reference in `compiler/`, `crates/`
and `providers/` split into production / example / test, whether the symbol its
recorded replacement needs exists in the tree, and which pinned artifacts the
route writes into. Judgments live here in prose; the JSON holds only what a
reader can reproduce.

The parity measurement for the removed route is
[`g-route-parity.json`](g-route-parity.json), produced by a temporary harness
committed **before** the deletion (`368a641`) and removed with it (`372039b`):

```sh
QCORE_P09G_PARITY_OUT="$PWD/docs/reports/remaining-p09-routes/g-route-parity.json" \
  cargo test -p qcore-compiler --test p09g_zero_helper_parity
```

## 2. Disposition table

| Route | What it is | Disposition | Decisive measurement |
|---|---|---|---|
| **P09-A** | `verifier_plugins::chunkio::teardown_source::check_counter_sources` | **Retained** | One of two recorded blockers has cleared (CORPUS-REPIN landed), the other has not: **no configured audit route selects the counter capability.** `qcore-audit`'s `SourceContractRegistry::verify` derives only `BooleanFieldSourceTask`; the string `counter` does not occur anywhere in `crates/qcore-audit/src/`. The route's two production call sites (`cycle19_compatibility.rs:554`, `:701`) write into a pinned artifact (`chunk-contracts.json`, `backend_teardown.unsigned_counters`). |
| **P09-B** | `verifier_plugins::libevent::event_driven::check_event_program` | **Retained** | 8 production and 50 test references. The portable route reproduces the certificate *for accepted programs* (P05's `reactor_route_parity.rs`), but its in-process caller — `external_integration.rs:230`, which consumes both the certificate and the diagnostic vector as `MonkeyCompositionDiagnostic::Event(errors)` — has no `TargetProfile` identity to derive a task from, and `compiler/tests/unit/event_driven.rs` alone calls the adapter 23 times, 14 of those pinning refusal text. |
| **P09-C** | `SourceContractConfiguration::Direct` in the pinned audit example | **Retained** | Correction to the earlier note (§4.1): the pinned artifacts are produced in **Process** mode, so this is not an artifact-parity problem. It is a documented default: `docs/CYCLE19_BUILD_INPUTS.md` states "the default audit without either selector remains the direct comparison route", and `scripts/prepare_cycle19_clang_audit.py --run` runs the audit with no selector. Deleting `Direct` makes a verified provider package mandatory for a shipped script. |
| **P09-D** | `QCORE_CHUNKIO_PACKAGE` selector alias | **Retained** | The alias is spelled in 10 committed files (9 documents plus an archived validation script), including the reproduction commands of the **current** pinned baseline (`docs/reports/remaining-o04-arms/README.md`) and of CORPUS-REPIN. Removing it invalidates the recorded way to reproduce the artifacts that are pinned today. |
| **P09-E** | `providers/conformance` out-of-protocol record-exchange fixture | **Retained** | Unchanged blocker, re-measured: `MissingOperation`, `DuplicateOperation` and `UnsupportedVersion` occur in `providers/` only in `reactor_callbacks.rs`. P05's `reactor_callback_capability.rs` matrix still does not reach them, so deleting the fixture deletes the only process-boundary coverage of those three refusals. |
| **P09-F** | the eleven compiler compatibility facades | **Retained** | Correction to the earlier note (§4.2): they are **not** all pure re-exports, and they are not free-standing. 40 production references, 26 test references; 7 of the 11 host their crate's unit tests through `#[cfg(test)] #[path = "../tests/unit/…"]`; and `chunk_contract.rs` (54 lines) is a sealed wrapper with its own type, not a facade at all. |
| **P09-G** | `chunk_source::import_chunk_source` zero-helper wrapper | **Removed** | Parity measured over all seven source shapes its consumers pass; every case byte-identical. See §5. |

## 3. P09-A, in detail — one blocker cleared, one standing

The earlier report blocked P09-A on two things.

1. *"CORPUS-REPIN, so the resulting artifact diff can be measured rather than
   asserted."* **Cleared.** The four artifacts were re-measured and deliberately
   re-pinned by CORPUS-REPIN and again by O04-ARMS, and both records carry the
   exact preparation and audit commands.
2. *"A configured audit route that selects the `qcore.chunkio` counter
   capability."* **Still open, measured here.** `qcore-audit`'s source-contract
   service has exactly one derivation
   (`crates/qcore-audit/src/source_contract.rs:473`, `BooleanFieldSourceTask`),
   gated on `VerifierCapability::ResourceContracts` and on the Boolean witness
   checker; there is no counter task, no counter binding vocabulary, and no
   occurrence of the word `counter` in that crate's sources.

A second option was considered and rejected: retargeting the example's two call
sites onto the **in-process** replacement pair
(`qcore_c_gateway::counter_source::import_counter_sources` +
`qcore_verification::counter_source::CounterSourceTask::derive(…).evaluate()`),
whose certificate equality P04 already pins in
`compiler/tests/counter_route_parity.rs`. That was rejected on two measured
grounds, not on effort:

- P04's parity is certificate equality on a **synthetic** four-function source,
  not on the real `cio_chunk.c`, and the certificates are serialised into a
  pinned artifact. Deleting on that basis would assert the artifact diff instead
  of measuring it — precisely what invariant 3 forbids.
- The replacement call needs the function list, record list, root C type, root
  QCore type and a full `TargetProfile` supplied by its caller. Moving those from
  `compiler/src` into `compiler/examples` relocates the hardcoding rather than
  removing it, and leaves the example, not the configured audit, holding the
  capability selection the follow-on asks for.

**Named gap (P09-A-GAP).** A configured audit route that selects the ChunkIO
counter capability by binding, so the example obtains the certificate the way
the Boolean-field route already does; then a two-sided 185-unit re-measurement
through the canonical pinned route (Fluent Bit `66910c10`, explicit Clang),
diffing `chunk-contracts.json`'s `unsigned_counters` and `memory_chunk_close`
blocks.

## 4. Corrections to the P09 report's per-route notes

### 4.1 P09-C is not blocked on artifact parity

The earlier note says the process route "is currently known *not* to be
byte-identical: the process route adds `provider_source_effect` records", and
files that under CORPUS-REPIN. Measured here: the pinned baseline
`docs/reports/remaining-o04-arms/artifacts/source-effects.json.gz` **contains 96
`provider_source_effect` records** — the archived artifacts were produced with
`QCORE_CHUNKIO_PACKAGE` set, i.e. in Process mode. So the `Direct` arm is not on
the path that produces the pinned artifacts at all, and removing it would not
move them.

What it would move is the default. `docs/CYCLE19_BUILD_INPUTS.md` documents the
no-selector default as the direct comparison route, and
`scripts/prepare_cycle19_clang_audit.py` runs the audit with `QCORE_CLANG` only,
so `--run` would begin requiring a built, verified provider package. That is a
user-facing change to a shipped script and a documented flow, which belongs in a
change that says so, not in a route deletion.

**Named gap (P09-C-GAP).** Decide and record whether a verified provider package
becomes mandatory for the compatibility example; if yes, update
`prepare_cycle19_clang_audit.py`, `docs/CYCLE19_BUILD_INPUTS.md` and the current
route's archived command in the same change, and re-measure the current-route
inventory (`current-route-diagnostics.json` in CORPUS-REPIN was produced by a
`Direct` run).

### 4.2 P09-F is not eleven pure re-exports

Measured facade-by-facade in `route-census.json` (`p09_f_facade_detail`):

| Module | Lines | Pure re-export | Hosts attached unit tests | In-tree consumers |
|---|---|---|---|---|
| `verifier_plugin` | 10 | yes | no | 9 (incl. `qcorec` production) |
| `external_source` | 6 | yes | **yes** | 1 |
| `counter_source` | 7 | yes | no | 1 |
| `research_extensions` | 6 | yes | **yes** | 4 |
| `cleanup_outcome` | 2 | yes | no | 7 |
| `source_call_use` | 2 | yes | no | 1 |
| `typed_source` | 8 | yes | **yes** | 20 |
| `list_unlink_contract` | 6 | yes | **yes** | 4 |
| `field_destructor` | 6 | yes | **yes** | 3 |
| `counter_contract` | 6 | yes | **yes** | 8 |
| `chunk_contract` | **54** | **no** | **yes** | 9 |

Three consequences the earlier note does not capture:

1. `chunk_contract.rs` is not a compatibility facade. It defines
   `ChunkContractCertificate`, a sealed wrapper over
   `qcore_contract::chunk_contract::ChunkContractEvaluation` that only the typed
   importer's summaries can produce, and it is consumed by production code
   (`memory_close.rs`) and by the pinned audit example. It has no canonical
   replacement to redirect to; deleting it would delete a boundary.
2. Seven facades are the compiler crate's *test hosts*: the unit tests under
   `compiler/tests/unit/` are attached to them with `#[cfg(test)]
   #[path = …] mod tests` and are written against `crate::` paths. Deleting a
   facade orphans its tests, and they cannot simply move to the canonical crate:
   they exercise ChunkIO/Libevent provider fixtures that live in the compiler
   crate, which the canonical crates must not depend on.
3. `crates/qcorec` reaches the host only through
   `qcore_compiler::verifier_plugin` and declares no dependency on
   `qcore-verifier-host`. Removing that facade adds a dependency edge to the CLI,
   which the layering guard test governs.

**Named gap (P09-F-GAP).** P09-F is a test-rehoming refactor plus a
multi-path public API break, not a deletion, and its own recorded sequencing
places it after the analysis routes A–C, all of which are retained here. It
should land as one deliberate versioned unit that (a) rehomes the seven attached
unit-test modules, (b) states what happens to `chunk_contract.rs` — which is not
in scope for "facade removal" — (c) decides the `qcorec` → `qcore-verifier-host`
edge, and (d) updates `compiler/tests/workspace_layering.rs`'s compatibility
pins deliberately, keeping their load-bearing negative halves.

## 5. The removal: P09-G

`chunk_source::import_chunk_source(unit, revision)` was a convenience overload
whose whole body was
`import_chunk_source_with_helpers(unit, source_revision, &[])`.

**Parity, measured before the deletion** (`368a641`,
[`g-route-parity.json`](g-route-parity.json)). The harness runs both entry
points over every source shape the wrapper's two in-tree consumers actually
pass, and compares the serialised result vector, not just the verdict:

| Case | Outcome | Serialised bytes | Identical |
|---|---|---|---|
| `release_summary` | 1 result | 653 | yes |
| `busy_summary` | 1 result | 674 | yes |
| `lock_summary` | 1 result | 664 | yes |
| `volatile_pointer_is_unknown` | 1 result (`Unknown`) | 107 | yes |
| `definition_absent_after_preprocessing` | 1 result (`Unknown`) | 103 | yes |
| `definition_present_after_preprocessing` (`-DINCLUDE_F`) | 1 result (`Summary`) | 642 | yes |
| `revision_mismatch_rejects` | rejected | 70 | yes, same text |

The harness also asserts the measurement is not vacuous: six cases must reach a
real import and exactly one must reject, so a shared early error could not pass
it. Both routes ran through the real Clang gateway
(`"clang" "--driver-mode=gcc"`).

**The deletion** (`372039b`):

| File | Change |
|---|---|
| `compiler/src/verifier_plugins/chunkio/chunk_source.rs` | `import_chunk_source` deleted; a pointer comment naming the surviving entry point and this parity archive left in its place |
| `compiler/tests/unit/chunk_source.rs` | six call sites retargeted to `import_chunk_source_with_helpers(…, &[])`; every assertion kept, including the revision-mismatch rejection and both preprocessing-conditional cases |
| `compiler/tests/unit/chunk_contract.rs` | one call site and its import retargeted |
| `compiler/tests/p09g_zero_helper_parity.rs` | temporary harness removed, its measurement archived |
| `compiler/Cargo.toml`, `crates/qcorec/Cargo.toml`, `Cargo.lock` | `qcore-compiler` `0.2.0` → **`0.3.0`** |

Re-measured after the deletion: **1752 workspace tests passed, 0 failed, 3
ignored** — exactly the pre-change count, the harness having been added and
removed inside this card. No test was deleted, weakened or renamed.

The one remaining textual occurrence of the removed name in the tree is the
pointer comment left at its former site; `route-census.json` reports it, rather
than filtering it out.

## 6. Public API and pinned expectations

**Public API removal (flagged).** `qcore_compiler::chunk_source::import_chunk_source`,
also reachable as
`qcore_compiler::verifier_plugins::chunkio::chunk_source::import_chunk_source`,
is gone. Under the crate's stated policy this is a breaking change, so
`qcore-compiler` moves `0.2.0` → `0.3.0` and its only dependent,
`crates/qcorec`, is updated to match. Every other compatibility path is
untouched — the `pub use` surface in `compiler/src/lib.rs`, the eleven P09-F
facades and the layering guard's pinned compatibility function pointers all
still resolve, and `compiler/tests/workspace_layering.rs` passes unchanged.

**Deliberate pin updates: none.** No contract-corpus count, submodule revision,
archived artifact, CT row, diagnostic vector or golden surface moved. The version
bump is the only expectation this card changes, and it changes no test: the
assertion that once read `env!("CARGO_PKG_VERSION")` was already retargeted at
the manifest's declared package version by P09 itself, so this bump does not
touch it.

**Fail-closed gates:** none weakened. Nothing in this card can turn an `Unknown`
into a `Safe`; the removed function had no decision logic of its own.

## 7. Lean

**Judged not applicable, recorded.** No semantic rule, model, checker,
certificate shape or diagnostic changed. The deleted function performed argument
defaulting on an importer call; it appears in no Lean definition or theorem, and
`proof/` contains no reference to it. The gate was run anyway and is unchanged at
**42 audits**, with no new axiom.

## 8. Gate results

| Gate | Baseline (`8d0790a`, before this card) | After |
|---|---|---|
| `cargo test --workspace` | 1752 passed, 0 failed, 3 ignored | **1752 passed, 0 failed, 3 ignored** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | 58 passed | **58 passed** |
| `python3 -B -m unittest discover -s scripts/tests` | 108 passed | **108 passed** |
| `python3 -B scripts/check_lean_audits.py` | 42 audits | **42 audits** |
| `cargo fmt --all -- --check` | clean | **clean** |
| `cargo clippy --all-targets -- -D warnings` | clean | **clean** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | **exit 0** |

The workspace row was measured on this worktree both before and after the
change (and once more with the temporary harness in place, at 1753). The other
rows were measured after the change and compared against the wave-7 recorded
ledger, which they reproduce exactly; nothing in this card touches a provider,
a Python script or a Lean file.

`lake` is not on the default `PATH` on this host; the Lean gate was run with
`PATH="$HOME/.elan/bin:$PATH"`. The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`.

Provider packages were **not** rebuilt: no provider, schema or manifest bytes
changed in this card, and the packaging gate is unaffected by a
`qcore-compiler` version bump (`providers/` is a separate workspace whose lock
file does not name `qcore-compiler`).

## 9. What this card does not claim

- It does not claim the compatibility layer is gone. Six of the seven named
  routes survive, and the largest of them (P09-F) is now measured to be a bigger
  unit of work than its one-line description suggested.
- It does not claim any Cycle 19 result moved. The four pinned 185-unit
  artifacts were **not** re-measured and were not eligible to move: the removed
  route has no production or example caller, and the census records that as a
  measurement rather than an assumption.
- It does not claim the retained routes are unremovable — only that on this
  checkout their replacements cannot yet serve their callers, with the specific
  missing piece named per route in §2, §3 and §4.

## 10. Named follow-ons

- **P09-A-GAP** — configured audit selection of the ChunkIO counter capability,
  then a two-sided 185-unit re-measurement (§3).
- **P09-B-GAP** — a derived-task identity for the Monkey composition caller, and
  a decision on whether the 14 pinned adapter refusal assertions move with it.
- **P09-C-GAP** — the mandatory-package decision for the compatibility example
  and its script/documentation consequences (§4.1).
- **P09-D-GAP** — follows C: retiring the alias requires rewriting the archived
  reproduction commands of the artifacts pinned today.
- **P09-E-GAP** — extend P05's `reactor_callback_capability.rs` matrix to reach
  `MissingOperation` / `DuplicateOperation` / `UnsupportedVersion` through the
  process boundary, or record that the protocol route refuses earlier and cannot
  reach them, before deleting the legacy fixture.
- **P09-F-GAP** — the four-part sequencing in §4.2.
