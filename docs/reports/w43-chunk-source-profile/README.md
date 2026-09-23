# W43 chunk source profile

Date: 2026-09-23. Card: `W43-CHUNK-SOURCE-PROFILE`. Base:
`ffdddbbb9ab306d6afdec335c25c1de7efa0416f` (contains `d7b39fac`). Input
baseline: [W42 chunk lifecycle certificate](../w42-chunk-lifecycle-cert/README.md),
its [`inventory.json`](../w42-chunk-lifecycle-cert/inventory.json) and
[`lifecycle-coverage.json`](../w42-chunk-lifecycle-cert/lifecycle-coverage.json).
The W42 private gate logs no longer exist and are not relied on.

## Result in one paragraph

Two of the 96 selected functions — `cio_chunk_close` and
`cio_chunk_close_stream` — now carry a lifecycle composition derived from
**their own pinned translation unit**, re-derived on every gate run and bound to
the digest the fluent-bit profile pins for `cio_chunk.c`. **Both stay
`Unknown`**, and so do the other 94: the distribution moves from
96 unclassified to 2 destruction-dependent + 94 unclassified, with **0 Safe /
96 Unknown and nine outstanding legs on every row, before and after**.
`input_chunk_get` **did not move**; it reaches **0 of 9** legs, for reasons
measured from its own pinned body below. The widening the brief proposed —
admitting more ABI shapes into the pinned Boolean-field projection — was
measured first and **cannot move any lifecycle row**, so it was not made.
`CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND` is **narrowed, not retired**; one
residual is registered.

## Scope

In scope: the lifecycle accounting of the 96 functions the pinned
forced-Libevent profile selects in `cio_chunk.c` (30) and `flb_input_chunk.c`
(66), revision `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`. Untouched: every
corpus verdict and frozen artifact, the Boolean-field importer
(`crates/qcore-c-gateway/src/boolean_source.rs`), the kernel lifecycle model
(`qcore_core::chunk_lifecycle`), the C gateway projection (card
W43-RETURN-EXPRESSION) and CT fixtures (card CT-SHARED-FIXTURE-DISCLOSURE).

## Step 1 — which profile dimension the 74 refusals hit

The W42 report's largest refusal group is **74** rows with
`unsupported-syntax: outside single borrowed chunk / int-or-void-result source
profile`. That reason is emitted by one line of the **Boolean-field** importer
(`boolean_source.rs`, `models.get(abi)`): the pinned audit selects a function
into its field-contract projection only when its full Clang `qualType` is
exactly `int (struct R *)` or `void (struct R *)` for a record `R` with a
declared Boolean field model (`cio_chunk.lock`, `flb_input_chunk.busy`).

[`scripts/measure_refusal_shapes.py`](scripts/measure_refusal_shapes.py) ran
Clang over each selected function with its unit's own pinned compile-database
entry, filtering the AST exactly as `typed_c_ast::function_ast` does. It refuses
an inventory not bound to the pinned sidecar digest and a unit whose bytes are
not the pinned bytes. Output: [`refusal-shapes.json`](refusal-shapes.json) (all
96 rows, ABI, parameters, direct callees).

| Result type of the 74 | Rows | …of which take exactly one chunk pointer |
|---|---:|---:|
| `int` | 40 | 0 |
| `void` | 12 | 0 |
| other scalar (`size_t`, `ssize_t`, `flb_sds_t`) | 14 | 7 |
| pointer | 8 | 2 |
| **total** | **74** | **9** |

| Parameter shape of the 74 | Rows |
|---|---:|
| a chunk pointer first, plus 1–5 more parameters | 24 |
| exactly one chunk pointer | 9 |
| a non-chunk first parameter (instance, config, task, output, raw bytes) | 40 |
| no parameter (`cio_version`) | 1 |

**No refused row has an ABI any declared model admits** (`admitted_boolean_abi:
0`). Both dimensions fail together or apart: all 52 `int`/`void` rows fail on
their parameters, and every single-chunk row fails on its result. The most a
Boolean-field ABI widening could admit is 9 rows (result widening) or 22
(`int`/`void` with a chunk first and extra parameters).

**Why no widening of that profile was made.** It would not move one lifecycle
row. An admitted row yields at best a *field contract* over a Boolean field,
and W42 established — in its published reasons and in Rust — that a field
contract establishes no lifecycle post-state. The three rows that already have
checked field contracts (`cio_chunk_is_locked`, `cio_chunk_unlock`,
`flb_input_chunk_release_lock`) are still `Unclassified` for exactly that
reason. Widening it would also re-measure the frozen `chunk-contracts` sidecar
(gateway rule 7) for no lifecycle gain. The widening that *does* bear on the
lifecycle result is of the **lifecycle** source profile: which pinned bodies
project into the lifecycle model at all. That is what this card widens.

## Step 2 — the lifecycle source profile, widened to the pinned units

W42's two compositions ran over committed distilled fixtures, so no row could
carry one. This card admits a function into the lifecycle model when an
existing rechecked importer accepts **its complete pinned body** against its
reference — never from its name — and attaches the resulting composition to
**that** row.

| Admitted shape | Pinned function | Importers run over the pinned units | Composition |
|---|---|---|---|
| `void (struct cio_chunk *, int)` null/memory chunk close | `cio_chunk_close` | frame helper `cio_error_reset` (`cio_error.c`), memory backend `cio_memfs_close` (`cio_memfs.c`), list unlink `mk_list_del`/`__mk_list_del`, total-chunk counter decrement, memory-close composition (`cio_chunk.c`) | prefix, 4 operations; must reach identical legs for `delete` = 0 and ≠ 0 |
| `void (struct cio_stream *)` chunk-ring close | `cio_chunk_close_stream` | the same close certificate plus the chunk-ring traversal (complete typed body, saved next pointer, only the current child passed, `delete` argument zero) | prefix, 6 operations over a two-member memory ring |

The derivation lives in
[`compiler/src/verifier_plugins/chunkio/pinned_lifecycle.rs`](../../../compiler/src/verifier_plugins/chunkio/pinned_lifecycle.rs);
the new ring composition (`compose_stream_ring_close`) and the program-digest
binding are in `lifecycle_coverage.rs` beside W42's compositions.

**Where the pinned bytes come from.** The gate cannot read the out-of-tree
pinned checkout, so
[`scripts/vendor_pinned_chunk_units.py`](scripts/vendor_pinned_chunk_units.py)
copied, byte for byte, exactly what Clang reads for the three units: the units,
their in-tree include closure measured with `clang -M` under each unit's own
pinned compile entry, and the two headers the pinned build generates
(`cio_info.h`, `cio_version.h`) — 24 files including four upstream licence
files, under `compiler/tests/fixtures/pinned/fluent-bit/` with a
[manifest](../../../compiler/tests/fixtures/pinned/fluent-bit/manifest.json) of
digests, origins, and each unit's recorded definitions and include-root order.
No byte is edited. The loader refuses any file whose digest differs, any
unlisted file (which could shadow a header), any include root outside the tree,
and a `cio_chunk.c` whose digest differs from the one
`contract/profiles/fluent-bit/profile.json` pins independently. The preprocessing
configuration is the pinned audit's own: the compatibility projection's
historical standard-input transport and main-source definition scope.

**Parity with the pinned build.** The ignored test
`the_vendored_units_derive_what_the_pinned_build_derives` builds the same three
units from the pinned checkout through its own compilation database
(`compiler_options_compatibility`, the pinned audit's adapter) and requires
byte-identical unit digests, rows and rendered result. It was run once for this
card, with the pinned checkout supplied, and passed (below). `--check` mode of
the vendoring script also re-measured all 24 copies against the checkout.

## Step 3 — the accounting rule the widening needed

Attaching a composition to a row made two new failure modes possible, so the
accounting now refuses them structurally (evidence version
`chunk-lifecycle-coverage-2`; published under this report).

1. **Own pinned source.** A row that carries a composition or a delete-only
   post-state must carry a `SourceDerivation` whose library, file, function
   and revision equal the row's own, and whose unit digest equals the
   binding's pinned digest for that file. Anything else — a fixture's bytes,
   another function's composition, another revision, a file with no pinned
   digest — is an operation error, not an `Unknown`. Only `cio_chunk.c` is
   bound: `flb_input_chunk.c` has no independent digest pin, so no composition
   can attach to one of its rows.
2. **Path completeness.** A composition's model legs count for the function
   only when it composes **every** source path. `cio_chunk_close` has four
   source paths; the certificate composes path 0 (null chunk) and path 1
   (memory-backed chunk). Paths 2 and 3 — the filesystem-backed close, whose
   nested cleanup keeps unresolved OS custody, and the unsupported-backend
   close, whose backend custody is opaque — are published as uncovered, each
   with its reason. So the five legs the composition *reaches*
   (`chunk_teardown`, `backend_teardown`, both unlinks, `context_counters`) are
   published as `reached_legs` on the row and **discharge nothing** for the
   function. Without this rule, attaching the memory-path composition would have
   read as five discharged legs for a function whose filesystem path is
   unmodelled — the fail-closed rule would have been weakened by the widening.

No existing check was relaxed: every W42 rejection still fires, the Safe arm
additionally requires own-source and complete paths, and W42's positive control
still reaches `Safe` once its derivation is complete.

## Per-row before/after

[`row-delta.json`](row-delta.json), rendered by
[`scripts/row_delta.py`](scripts/row_delta.py) from the two Rust-decided
results and checked by the Python suite:

| Distribution | Before (W42, `-1`) | After (W43, `-2`) |
|---|---:|---:|
| unclassified, Unknown, 9 legs outstanding | 96 | 94 |
| destruction-dependent, Unknown, 9 legs outstanding, pinned-source composition | 0 | 2 |
| Safe | 0 | 0 |

| Row | Before | After |
|---|---|---|
| `cio_chunk_close` | unclassified; projection refusal (profile) | destruction-dependent; prefix composition from pinned `cio_chunk.c` (`sha256:34c27133…`); reaches 5 legs on paths 0–1; retains `context 0`, `stream 0`; paths 2–3 uncovered; **9 outstanding** |
| `cio_chunk_close_stream` | unclassified; projection refusal (profile) | destruction-dependent; prefix composition of the ring traversal and the close (`sha256:ba8c3ac6…`); reaches the same 5 legs over a two-member memory ring; retains `context 0`, `stream 0`; a member closed through path 2 or 3 uncovered; **9 outstanding** |
| the other 94 | unclassified | unchanged, **reason and role byte-identical** (Python-checked) |

## `input_chunk_get`: which legs it reaches, and why not the rest

It reaches **none**. Its row is unchanged: `Unclassified`, all nine legs
outstanding. Measured from its own pinned body (`refusal-shapes.json`): it is
`struct flb_input_chunk *(struct flb_input_instance *, int, const char *, int,
size_t, int *)` — a pointer result, four non-chunk leading parameters — and it
calls `flb_hash_table_get`, `pthread_mutex_lock`/`unlock`,
`cio_chunk_is_locked`, `cio_chunk_is_up`, `cio_chunk_up_force`,
`flb_input_chunk_destroy_corrupted`, `flb_input_chunk_create`,
`flb_input_chunk_get_projected_write_size`, `flb_routes_mask_is_empty`,
`flb_input_chunk_place_new_chunk` and `flb_input_chunk_destroy` (plus logging).

* Its destructive paths go through `flb_input_chunk_destroy` and
  `flb_input_chunk_destroy_corrupted`, both themselves `Unclassified` rows with
  no composition; `flb_input_chunk_destroy` is a 17-callee wrapper
  (routes, hash tables, locks, chunk trace, then `cio_chunk_close`).
* Its constructive path goes through `flb_input_chunk_create` →
  `cio_chunk_open`, neither of which has a lifecycle importer (the model has an
  `OpenChunk` operation, but no pinned body is projected onto it).
* `error_exits` and `wrapper_obligations` need source evidence about the
  wrapper's busy/lock/task/route custody that no importer produces for
  `flb_input_chunk.c`, and that unit has no independent digest pin to bind one to.

So no leg is reachable without body importers for those callees first; the
ordering is recorded in the narrowed residual.

## What stayed Unknown, and why

* **All 96 rows.** `Safe` needs all nine legs and a terminal, own-source,
  path-complete composition; no row has one.
* **2 composed rows**: both compositions are prefixes (context and stream live
  by the close's premise) and both leave two source paths uncovered; neither
  has error-exit or wrapper evidence.
* **94 unclassified rows**: no importer projects their bodies into the
  lifecycle model — 74 refused at the Boolean ABI, 13 at `DeclStmt`, 4 at
  `CallExpr`, 2 at preprocessing coverage (`sb_*`, absent from the pinned
  build), 3 with field contracts only.

## Negative controls

**Source-level, on the pinned bytes** (`compiler/tests/unit/pinned_lifecycle.rs`,
Clang-gated). Each mutation of the vendored pinned `cio_chunk.c` is asserted to
change the source and must derive **no row**:

| Mutation | Refusal |
|---|---|
| `mk_list_del(&ch->_state_head);` removed | `unbound or out-of-order memory close call free` |
| `cio_chunk_counter_total_sub(ctx);` removed | `observed contract violation: not chunk_was_nonnull or saved_context_counter_decremented` |
| `cio_memfs_close(ch);` removed from the memory branch | `unbound or out-of-order memory close call mk_list_del` |
| `free(ch);` removed | `unbound or out-of-order memory close call cio_chunk_counter_total_sub` |
| ring close passes `CIO_TRUE` | `cio_chunk_close_stream differs from the complete typed reference body` |
| ring iterates with `mk_list_foreach` (no saved next) | `cio_chunk_close_stream differs from the complete typed reference body` |

Also: a unit one byte away from the pinned `cio_chunk.c` is refused as the
pinned unit; a unit presented under another path is refused; a vendored tree
with one changed header byte, or with an extra unlisted header, is refused.

**Accounting** (`crates/qcore-verification/tests/chunk_lifecycle_coverage.rs`,
11 new; 40 total): a path-incomplete terminal composition with both source
legs stays `Unknown` with all seven model legs outstanding (and the same row
with complete paths reaches `Safe`, so the refusal is the rule and not an
unreachable branch); a composition without a derivation; a derivation without
a composition; another function's derivation; another revision; a non-pinned
unit digest; a file with no pinned digest; empty, reasonless and duplicate
paths; an unclassified row with a derivation; a delete-only post-state without
its own source.

**On the published route** (`compiler/tests/chunk_lifecycle_coverage.rs`): the
real `cio_chunk_close` row re-labelled with the W42 fixture's digest, given
`cio_chunk_close_stream`'s derivation, or stripped of its derivation is refused.

## Lean parity

`QCore.ChunkLifecycleCoverage` gains `ownSource` and `pathsComplete` on a
composition, `Composition.admits`, and makes `Evidence.modelLegs` empty for a
path-incomplete composition; `isSafe` requires an admitted composition, and
`WellFormed` gains a fourth conjunct requiring own source. All 15 W42 theorems
keep their statement text; the two that assume `WellFormed` now assume that
stronger hypothesis.
Eleven new theorems are registered in `AuditCycle19.lean` (audit file count
unchanged): `safe_is_admitted_and_complete`, `admits_facts`,
`safe_composition_is_own_source_and_path_complete`,
`a_path_incomplete_composition_discharges_no_model_leg`,
`a_path_incomplete_composition_leaves_model_legs_outstanding`,
`a_path_incomplete_composition_is_never_safe`,
`a_composition_not_from_its_own_pinned_source_is_never_safe`,
`a_well_formed_composition_is_from_its_own_pinned_source`,
`another_function_is_not_its_own_source`,
`another_unit_digest_is_not_its_own_source`,
`another_revision_is_not_its_own_source`. They are abstract rule theorems
over the implemented route, not extraction of the Rust analyzer or of C; trust
boundary 28 is unchanged.

## The verdict contract

| Field | Value |
|---|---|
| artifact | `docs/reports/w42-priority-integration/artifacts/chunk-contracts.json.gz` (`c4f438e4…917980`), through the W42 inventory |
| property | bounded chunk lifecycle post-state |
| semantic profile | `qcore.chunk_lifecycle.v1` |
| scope | unchanged from W42 |
| bounds | W42's four plus: a composition counts only when derived from the function's own pinned unit and complete over its source paths |
| assumptions | W42's four, with `composition_binding` rewritten, plus `vendored_build_configuration` and `composition_premises` |
| evidence version | `chunk-lifecycle-coverage-2` |
| pinned units | `cio_chunk.c` → `8f6064c7…efd6c8` (the profile pin) |

## Parity and artifact attribution

* **BEFORE**: at base, before any Rust edit,
  `cargo test -p qcore-compiler --test chunk_lifecycle_coverage` exited 0,
  3 passed — the W42 result regenerated byte-for-byte
  (`measure/w43-before/compiler-lifecycle-test.log`).
* **Frozen**: `docs/reports/w42-chunk-lifecycle-cert/lifecycle-coverage.json` is
  unchanged and no longer regenerated; the current result is
  [`lifecycle-coverage.json`](lifecycle-coverage.json) here. Every change
  between them is the per-row delta above plus: `evidence_version` 1 → 2,
  a fifth `bounds` entry, two new and one rewritten `assumptions`, the new
  `pinned_units` binding, and the `model.discharged` field renamed
  `reached_legs` (it never meant *discharged for the function*).
* **Corpus**: no file under `docs/reports/*/artifacts/` changed; no corpus
  verdict, sidecar schema, contract pin, clause count or submodule moved.
  Constraint 7 is not engaged. No analysis route was removed (constraint 3).

## Validation

All logs are worktree-private, run-scoped files under `measure/w43-gate/`
(driver `run.sh`; each step's exit is written to `<step>.exit`, nothing is
piped), plus `measure/w43-before/` and `measure/w43-parity/`. The gate ran on
tip `4910a179` with a clean tree, `CARGO_BUILD_JOBS=4` and
`RUST_TEST_THREADS=2`, while two other cards ran gates on the same host. No
sanitizer option was disabled and no assertion or deadline was weakened.
Clang was measured **in this worktree for this run**
(`measure/w43-gate/clang-version.txt`).

| Command | Exit | Result | Private log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a277d0f42703702c4dfd9af6af38595a83` (`git submodule status --recursive`) | `submodule.log`, `submodule-status.log` |
| `clang --version` | 0 | `Ubuntu clang version 21.1.8 (6ubuntu1)` | `clang-version.txt` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,102 passed / 0 failed / 12 ignored**, 271 result summaries. Against W42's 5,085 / 0 / 11: +11 accounting controls, +5 pinned-unit unit tests, +1 published-route test, +1 ignored parity test | `cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | **8/8**, 0 failed expectations | `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | **11/11**, 0 failed expectations | `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | **165 audits pass** (file count unchanged); the 11 new theorems depend on `propext` alone (7) or on no axiom (4) | `lean-audits.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | **317 tests** = 313 on the base (340 less the 27 guard tests `MAIN-GUARD-REMOVAL` deleted) + this card's 4 | `python-tests.log` |
| `python3 scripts/build_docs_index.py --check` | 0 | index current (it was also current on the base) | `docs-index.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `provider-fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `provider-clippy.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | **123 passed / 0 failed / 0 ignored**, 27 summaries; `real_packages_preserve_unknown_and_reject_mutations` passed in-suite (over 60 s) | `provider-test.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | **5 tests** | `package-python.log` |
| `python3 providers/package.py build --output measure/w43-gate/packages` | 0 | five packages | `package-build.log` |
| `python3 providers/package.py verify …` ×5 | 0 each | `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, `qcore.libevent`, `qcore.librdkafka` | `package-verify-*.log` |

Rule 16's host-sensitive rows — `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths`,
`memory_growth_beyond_the_declared_limit_is_bounded`,
`a_declared_process_limit_stops_the_fixture_from_forking` and
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` —
all passed **inside the full concurrent workspace run**; none failed, so no
isolated rerun was needed and none is claimed.

Card-specific measurements, also private:

| Command | Exit | Result | Log |
|---|---:|---|---|
| BEFORE, base tree: `cargo test -p qcore-compiler --test chunk_lifecycle_coverage` | 0 | 3 passed; the W42 result regenerated byte-for-byte | `measure/w43-before/compiler-lifecycle-test.log` |
| `measure_refusal_shapes.py` over the pinned checkout | 0 | 74 profile refusals, 0 admitted ABIs (`refusal-shapes.json`) | `measure/w43-parity/refusal-shapes.log` |
| `vendor_pinned_chunk_units.py --check` | 0 | 24 vendored files and the manifest match the pinned checkout | `measure/w43-parity/vendor-check.log` |
| `QCORE_W43_PINNED_SOURCE=… QCORE_W43_PINNED_DATABASE=… cargo test -p qcore-compiler --test chunk_lifecycle_coverage -- --ignored` | 0 | 1 passed: vendored and pinned-build derivations byte-identical | `measure/w43-parity/parity.log` |

**Failed and incomplete attempts, kept rather than deleted:** the published-route
test failed **twice by design** while the result was produced — once against a
`{}` placeholder, once after the fifth `bounds` entry was added — each time
writing the regenerated bytes to `target/tmp/lifecycle-coverage.json`, which were
then committed; the test passes. One focused run of the verification suite
failed once (`a_missing_model_binding_keeps_every_model_leg_outstanding`)
because the test stripped a row's composition but not its derivation, which the
new rule correctly refuses; the test was corrected to strip both, not the rule.
The only edit after the gate is this report's text.

## Remaining work

* **`CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND`** — **narrowed**, not
  retired. The fixture binding is gone for every row that has a composition, and
  a fixture-derived composition is now refused as row evidence; but 94 selected
  functions still have no composition from their own pinned source, so the
  residual is not discharged for every selected function. Its text now names
  the 94 and the importer order `input_chunk_get` needs.
* **`CHUNK-LIFECYCLE-CLOSE-PATHS-UNCOVERED`** — **new**. The filesystem and
  unsupported-backend paths of `cio_chunk_close` (and of ring members closed
  through them) are not composed, so the two composed rows discharge nothing.
* **`CHUNK-LIFECYCLE-COVERAGE-OUTSIDE-THE-PINNED-SIDECAR`** — text updated to
  compare against this report's result; otherwise unchanged.

Ledger: **276 → 277**.

Disclosed limitation, not a new residual: parity between the vendored units and
the pinned build is an ignored test run explicitly (it needs the out-of-tree
checkout). The gate re-derives from the vendored bytes on every run and pins
`cio_chunk.c` to the profile digest, so a re-pin of the profile fails the gate
closed until the copies are re-vendored and parity re-run.

## Integrator reconciliation list

1. **Apply.** Narrow `CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND`, register
   `CHUNK-LIFECYCLE-CLOSE-PATHS-UNCOVERED`, update the comparison target of
   `CHUNK-LIFECYCLE-COVERAGE-OUTSIDE-THE-PINNED-SIDECAR`;
   `dimensions.named_followups` reads **277**.
2. **Apply.** `docs/STATUS.md`: registered follow-ups 277; the chunk lifecycle
   row keeps **0 Safe / 96 Unknown** and now cites this report's result.
3. **Note.** `docs/STATUS.md`'s "Latest validated gate" paragraph still says
   "**282** follow-ups are registered", which `MAIN-GUARD-REMOVAL` (276) had
   already made stale before this card; it is not edited here.
4. **Note.** A re-pin of the fluent-bit profile must re-run
   `scripts/vendor_pinned_chunk_units.py` and the ignored parity test; the gate
   fails closed until then.
