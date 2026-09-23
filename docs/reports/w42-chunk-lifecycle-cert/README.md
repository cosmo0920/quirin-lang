# W42 chunk lifecycle certificate

Date: 2026-09-23. Card: `W42-CHUNK-LIFECYCLE-CERT`. Base:
`5988be052bd86391c6f448eb3c9ec6ec03f98583`. Input baseline:
[W42 priority integration](../w42-priority-integration/README.md).

## Scope and result

Every one of the **96** functions the pinned forced-Libevent profile selects in
`flb_input_chunk.c` (**66**) and `cio_chunk.c` (**30**) now carries a *checked
lifecycle result* — a verdict about its post-state, decided in Rust, distinct
from the field-contract status the previous inventory reported and distinct from
the function-level resource verdict the corpus reports.

**All 96 results are `Unknown`, each with nine outstanding composition legs and
its own reason.** No selected function's lifecycle post-state is established by
the pinned evidence, `input_chunk_get` and `flb_input_chunk_create` included.
That is the fail-closed answer, not a placeholder: the machinery that would
promote a function exists, is audited in Lean, is exercised by a positive
control, and refuses every current row because no selected function has a
source-derived lifecycle role.

This card changes no corpus verdict. Cycle 19 units remain
**2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**; projected
functions remain **223 Safe / 84 provisional Unsafe / 4,527 Unknown**. Every
frozen artifact under `docs/reports/*/artifacts/` is byte-identical; no
sidecar schema, contract pin or corpus expectation moved.

## The verdict contract

Every published row carries the full contract, in
[`lifecycle-coverage.json`](lifecycle-coverage.json):

| Field | Value |
|---|---|
| artifact | `docs/reports/w42-priority-integration/artifacts/chunk-contracts.json.gz` |
| artifact digest | `c4f438e4…917980` (uncompressed sha256) |
| property | bounded chunk lifecycle post-state |
| semantic profile | `qcore.chunk_lifecycle.v1` |
| scope | the selected input-chunk wrapper and `cio_chunk` functions of the pinned forced-Libevent Fluent Bit build; no other translation unit, build configuration or caller |
| bounds | nine legs, seven derived from a re-run model program and two from source evidence; a delete-only operation keeps its executed partial post-state and is never `Safe`; model identities are bounded by the kernel's identity profile; field contracts are not lifecycle results |
| assumptions | faithful C extraction and Rust-model correspondence (evidence rule 28); the sidecar's field-contract domain premises; the sidecar's recorded certificates are unsealed report values and are never revalidated here; the re-derived compositions are taken over committed C fixtures |
| evidence version | `chunk-lifecycle-coverage-1` |
| source revision | `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` |

## The nine legs, and why a `Safe` needs all of them

The acceptance boundary is implemented literally. A ChunkIO `Safe` requires
context → stream → chunk → backend teardown, **both** list unlinks, counters,
error exits and outstanding wrapper obligations:

| Leg | Source |
|---|---|
| `context_teardown` | re-run model program |
| `stream_teardown` | re-run model program |
| `chunk_teardown` | re-run model program |
| `backend_teardown` | re-run model program |
| `primary_list_unlink` | re-run model program |
| `residency_list_unlink` | re-run model program |
| `context_counters` | re-run model program |
| `error_exits` | source evidence |
| `wrapper_obligations` | source evidence |

The seven model legs **cannot be asserted**. `ModelComposition::terminal`
re-runs `check_chunk_program` over the caller's program and reads the discharge
out of the accepted terminal state; `ModelComposition::prefix` executes a
composed prefix and records exactly what that execution reached, together with
the identities it leaves live. There is no constructor that takes a leg set.
An empty program discharges nothing (it is refused rather than vacuously
accepted), a program with a live identity is refused, and a delete-only program
is refused as a terminal composition.

This distinction does real work. `cio_chunk_close`'s composed memory path
reaches `chunk_teardown`, `backend_teardown`, both unlinks and
`context_counters`, and leaves `context 0, stream 0` live — because
`requires context_and_stream_outlive_chunk` is a **premise** of the composed
contract. A premise is recorded as retained, never as discharged, so the two
container legs stay outstanding and the row stays `Unknown`.

Delete-only operations keep their actual partial post-state.
`DeleteOnlyPostState::from_program` executes a `cio_stream_delete`-shaped
program and reads back `retained_live = [context 0, stream 0]`,
`discharged = [chunk 0]` and the recorded backing-delete outcome. A program with
no recorded content deletion, or one that retains nothing, is refused rather
than classified as delete-only, and a delete-only row is never `Safe` even with
both source legs discharged.

## Inventory, and why every row is `Unclassified`

[`inventory.json`](inventory.json) is the digest-bound extract of the pinned
sidecar, produced by
[`scripts/lifecycle_inventory_extract.py`](scripts/lifecycle_inventory_extract.py).
It transports identities and decides nothing; it refuses any artifact whose
uncompressed digest is not the pinned one, any Unknown row without a reason, and
any duplicate source-function identity.

A lifecycle **role** is never guessed from a function's name — the whole point
of the kernel model is that "close" and "delete" in a name are not evidence. A
role arrives with a rechecked certificate or the function keeps the pinned
projection's own refusal. The pinned projection classifies no selected
function's lifecycle role, so all 96 rows take the fail-closed default:

| Rows | Pinned projection said |
|---:|---|
| 74 | `unsupported-syntax`: outside single borrowed chunk / int-or-void-result source profile |
| 13 | `unsupported-syntax`: unmodeled statement `DeclStmt` |
| 4 | `unsupported-syntax`: unmodeled expression `CallExpr` |
| 2 | `incomplete-obligation-coverage`: expected one selected definition after preprocessing, found 0 |
| 3 | a **checked field contract** over 4 Boolean field states |

The last row is the one this card most needed to get right.
`cio_chunk_is_locked`, `cio_chunk_unlock` and `flb_input_chunk_release_lock`
have checked field contracts over four Boolean field states each. Their
published reason says, in those words, that a checked field contract
*establishes no lifecycle post-state*, so the distinction cannot be lost between
the adapter and a reader of the artifact. **Field-contract checks do not close
this card, and they do not promote a row.**

`input_chunk_get`'s published row:

```json
{"library": "fluent-bit-chunk", "file": "flb_input_chunk.c",
 "function": "input_chunk_get", "role": "unclassified",
 "lifecycle": {"status": "unknown",
   "reason": "no source-derived lifecycle role; pinned projection refusal (unsupported-syntax): outside single borrowed chunk / int-or-void-result source profile",
   "outstanding_legs": ["context_teardown", "stream_teardown", "chunk_teardown",
     "backend_teardown", "primary_list_unlink", "residency_list_unlink",
     "context_counters", "error_exits", "wrapper_obligations"]}}
```

## Composition of the existing ChunkIO teardown evidence

Where a result depends on destruction the composition is re-derived, not cited.
The compiler adapter turns the already-rechecked ChunkIO certificates into model
programs and hands them to the derivation:

* `compose_memory_close` places `MemoryCloseCertificate::instantiate`'s
  operations after a live context, stream and memory-backed chunk. The
  certificate supplies the operations; the adapter supplies only the identities.
  Result: a **prefix** composition, five legs, two retained identities.
* `compose_nested_teardown` runs `NestedMemorySource::instantiate` — the
  complete context → stream → chunk → backend chain — and re-checks the result
  with `check_chunk_program`. Result: a **terminal** composition, all seven
  model legs, nothing retained.
* `compose_stream_content_delete` builds the partial delete shape and reads its
  post-state back.

Both compositions run over the committed ChunkIO C fixtures through the existing
importers (`check_memory_close_source`, the traversal and parent-teardown
importers), under Clang, at the pinned revision constant. They are **not** bound
to the pinned upstream translation units — see the residual below.

## Placement

| Layer | Path |
|---|---|
| Reusable decision procedure | `crates/qcore-verification/src/chunk_lifecycle_coverage.rs` |
| ChunkIO half (selection, projection reasons, composition recipes) | `compiler/src/verifier_plugins/chunkio/lifecycle_coverage.rs` |
| Lean model and audited theorems | `proof/QCore/ChunkLifecycleCoverage.lean`, registered in `QCore.lean` and `AuditCycle19.lean` |
| Published inventory and result | `docs/reports/w42-chunk-lifecycle-cert/{inventory,lifecycle-coverage}.json` |

The kernel model it composes (`qcore_core::chunk_lifecycle`) is untouched.

## Lean parity

`QCore.ChunkLifecycleCoverage` states the layer above an accepted teardown:
when a *function's* result may be `Safe`. Fifteen theorems are registered in
`AuditCycle19.lean` and pass the audit with only `propext` / `Quot.sound`:

* `safe_is_terminal_and_complete`, `safe_discharges_every_required_leg` —
  an accepted result is destruction-dependent, carries a **terminal**
  composition, and discharges all nine legs;
* `a_missing_leg_stays_unknown` — any missing leg keeps the row `Unknown` and
  the published row names that leg;
* `only_destruction_dependent_rows_can_be_safe`, `delete_only_is_never_safe`,
  `unclassified_is_never_safe`, `non_destructive_is_never_safe`;
* `a_prefix_composition_is_never_safe`, `no_composition_is_never_safe` —
  the premise/discharge distinction, stated;
* `delete_only_retains_its_partial_post_state`,
  `only_destruction_dependent_rows_carry_a_composition` — the well-formedness
  the Rust adapter enforces as an operation error;
* `requiredLegs_length` — the nine;
* `omitted_selected_function_is_not_accounted`,
  `duplicate_function_is_not_accounted`,
  `unselected_function_is_not_accounted` — exact accounting.

These are abstract rule theorems over the implemented route, not extraction of
the Rust analyzer or of C. Trust boundary 28 is unchanged.

## Negative controls

**Source-level** (`compiler/tests/unit/lifecycle_coverage.rs`, Clang-gated). The
real ChunkIO C fixture is mutated and each mutation must fail to compose *any*
leg:

| Mutation | Result |
|---|---|
| `cio_chunk_counter_total_sub(ctx);` removed | composition refused |
| `mk_list_del(&ch->_state_head);` removed | composition refused |
| `if(type==CIO_STORE_MEM){cio_memfs_close(ch);}` emptied | composition refused |
| `free(ch->name);free(ch);` → `free(ch->name);` | composition refused |

Each mutation is asserted to change the source before it is compiled, so an
inert replacement cannot pass as a control.

**Model and accounting**
(`crates/qcore-verification/tests/chunk_lifecycle_coverage.rs`, 29 tests):
empty program, live-identity program, delete-only-as-terminal, outstanding
route, empty program digest, complete-teardown-as-delete-only, prefix that
discharges nothing, omitted row, duplicated row, unselected row, empty
selection, a result published without its assumptions, unbound artifact digest,
unclassified row carrying evidence, unclassified row claiming source legs,
unclassified row without a reason, delete-only row without a post-state,
non-destructive row carrying a teardown certificate, missing wrapper leg,
missing error-exit leg, missing model binding.

**Adapter controls** (9 tests): classification outside the selection,
duplicated classification, caller-supplied `Unclassified`, unexecutable prefix.

**Positive control.** `a_complete_composition_reaches_safe` builds a terminal
composition plus both source legs and reaches `Safe` with all nine legs
discharged. The `Safe` arm is therefore reachable, and every `Unknown` above is
a measured refusal rather than an unreachable branch. No production row reaches
it.

## Parity and artifact attribution

* No frozen artifact changed. `docs/reports/w42-priority-integration/artifacts/`
  and `docs/reports/b-series-integration/artifacts/` are read-only inputs here;
  their `chunk-contracts.json.gz` have the same uncompressed digest
  `c4f438e4…917980`, and a Python test measures that both extract identically.
* No analysis route was removed, so constraint 3 does not apply.
* No contract, clause-count, submodule or CT pin moved. The Lean **audit file**
  count is unchanged at 165: the new module registers in the existing
  `AuditCycle19.lean` rather than adding an `Audit*.lean`.
* Two new files are published under this report: `inventory.json` (derived, and
  regenerated by the Python suite) and `lifecycle-coverage.json` (decided in
  Rust, and regenerated and byte-compared by
  `compiler/tests/chunk_lifecycle_coverage.rs`). Neither is a corpus artifact.
* **`docs/reports/INDEX.md` was already stale before this card.**
  `python3 scripts/build_docs_index.py --check` failed on the base commit
  `5988be05`; regenerating is all-or-nothing, so the committed index adds three
  rows: this report plus `confined-exit-precedence` and
  `native-platform-test-isolation`, whose own cards landed without regenerating
  it. Those two rows are not this card's work and are attributed here rather
  than absorbed. `--check` is not a standing-gate row, which is why nothing was
  red.

## What stayed Unknown, and why

Everything. All 96 rows, all nine legs each. The three distinct grounds:

1. **93 rows** — the pinned resource projection refuses the function's body, so
   there is no lifecycle projection to compose with (74 profile refusals, 13
   `DeclStmt`, 4 `CallExpr`, 2 preprocessing-coverage).
2. **3 rows** — a checked bounded *field* contract exists; a field contract
   ranges over Boolean field states and establishes no post-state.
3. **Every row** — no composition is attached, because attaching one would
   require deriving it from that function's own pinned source (see the first
   residual). The nested teardown evidence the artifact records is 21 backend
   teardown records across `cio_destroy`, `cio_stream_destroy`,
   `cio_stream_destroy_all`, `cio_chunk_close_stream`, `cio_file_close`,
   `cio_file_native_delete`, the list-unlink and counter certificates and the
   nested memory/cleanup/unsupported sources. Those are *unsealed published
   values*; revalidating one as certificate evidence is exactly what this
   repository refuses, so none of them promotes a row here.

## Validation

All logs are worktree-private, run-scoped files under
`measure/w42-chunk-lifecycle-final/`. The run used `CARGO_BUILD_JOBS=4` and
`RUST_TEST_THREADS=2`; no sanitizer option was disabled and no assertion,
deadline or confinement requirement was weakened. Clang was measured **in this
worktree for this run**; `measure/w42-chunk-lifecycle-final/clang-version.txt`
is that recording, not a borrowed one.

| Command | Exit | Result | Private log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a277d0f42703702c4dfd9af6af38595a83`, rechecked with `git submodule status --recursive` | `submodule.log` |
| `clang --version` | 0 | `Ubuntu clang version 21.1.8 (6ubuntu1)` | `clang-version.txt` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,085 passed / 0 failed / 11 ignored**; 271 result summaries over 260 running targets and 9 doc-test sections (two doc sections print two summaries each) | `cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | **8/8 expectations pass** | `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | **11/11 expectations pass** | `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | **165 audits pass**; no project axioms or placeholders; the 15 new theorems depend only on `propext` / `Quot.sound` | `lean-audits.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | **340 tests** (336 + this card's 4) | `python-tests.log` |
| five isolated core host-sensitive rows | 0 each | **1 passed / 0 failed each**: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, `a_confined_fixture_cannot_write_outside_the_admitted_paths`, `memory_growth_beyond_the_declared_limit_is_bounded`, `a_declared_process_limit_stops_the_fixture_from_forking`, `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` | `isolated/*.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `provider-fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `provider-clippy.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | **123 passed / 0 failed / 0 ignored**, 27 summaries | `provider-test.log` |
| isolated `real_packages_preserve_unknown_and_reject_mutations` | 0 | **1 passed** in 43.7 s | `isolated/package_mutation.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | **5 tests** | `package-python.log` |
| `python3 providers/package.py build --output …` | 0 | five packages built | `package-build.log` |
| `python3 providers/package.py verify …` ×5 | 0 each | `qcore.chunkio`, `qcore.cmetrics`, `qcore.ctraces`, `qcore.libevent`, `qcore.librdkafka` | `package-verify-*.log` |

The `closed_input…` row is the one rule 16 says a report must **state which it
saw**: it passed in isolation, 1 passed / 0 failed, and also inside the full
workspace run. No sanitizer, LeakSanitizer-under-ptrace or namespace refusal
appeared in either run.

**Failed and incomplete attempts, kept rather than deleted:**

* An earlier full workspace run
  (`measure/w42-chunk-lifecycle-incomplete/cargo-test-interrupted.log`) reached
  **96 result summaries with 0 failures** and was then **deliberately stopped by
  me**, not by a failure: I had since tightened the accounting to refuse a
  result published without assumptions and reworded an `Unknown` reason, so the
  run no longer measured the tree being reported. It is retained and is **not**
  counted as validation.
* `cargo fmt --all -- --check` exited **1** once, on a single over-long
  `assert!` in the new verification test. `cargo fmt --all` repaired it and the
  recheck exited 0.
* `compiler/tests/chunk_lifecycle_coverage.rs` failed **twice by design** while
  the published result was being produced: once against a placeholder file, once
  after the reason wording changed. Each time the test wrote the regenerated
  bytes to `CARGO_TARGET_TMPDIR` and named the path; the committed artifact was
  replaced with those bytes and the test then passed. That is the intended
  regeneration loop, and it is why a stale published result cannot survive a
  gate run.
* `python3 scripts/build_docs_index.py --check` exited **1 on the base commit**,
  before this card changed anything. See the attribution above.

## Remaining work

`W42-CHUNK-LIFECYCLE-CERT` is retired against this report. Two narrower
residuals replace it, bringing the registered ledger from **281** to **282**:

* **`CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND`** — the two re-derived
  compositions run over the committed distilled C fixtures, not over the pinned
  upstream `cio_chunk.c` / `flb_input_chunk.c` units, so no published row
  carries a composition derived from its own pinned source. Discharge: run the
  pinned source importer over the upstream units and attach the resulting
  composition to the selected function's row.
* **`CHUNK-LIFECYCLE-COVERAGE-OUTSIDE-THE-PINNED-SIDECAR`** — the accounting
  reaches its decision through a digest-bound committed extract rather than
  through the audit that produced the sidecar. Discharge: emit a
  `lifecycle_coverage` block from the audit run at the next corpus arbiter and
  compare it with `lifecycle-coverage.json`.

Out of scope and untouched: general indirect access, static-helper summaries,
`MONKEY-CALLBACK-CYCLE18-COMPOSITION`, the filesystem and invalid-backend close
paths that `memory_close.qcore` leaves uncovered, and every corpus verdict.

## Integrator reconciliation list

1. **Apply.** Retire `W42-CHUNK-LIFECYCLE-CERT` and register
   `CHUNK-LIFECYCLE-COMPOSITION-FIXTURE-BOUND` and
   `CHUNK-LIFECYCLE-COVERAGE-OUTSIDE-THE-PINNED-SIDECAR`; the matrix's
   `dimensions.named_followups` must read **282**.
2. **Apply.** `docs/STATUS.md` carries the new chunk-lifecycle coverage row and
   the note that function-level safety and the lifecycle result are separate
   dimensions.
3. **Note.** If a later arbiter re-measures the corpus, the pinned digest in
   `scripts/lifecycle_inventory_extract.py` must be re-pinned deliberately and
   `lifecycle-coverage.json` regenerated; the Rust test fails closed until both
   happen.
4. **Note.** The Lean audit file count stays 165; the new theorems live in
   `AuditCycle19.lean`.
5. **Apply.** `docs/reports/INDEX.md` is regenerated. Two of its three new rows
   belong to `CONFINED-EXIT-PRECEDENCE` and `NATIVE-PLATFORM-TEST-ISOLATION`,
   which left the index stale; consider adding
   `python3 scripts/build_docs_index.py --check` to the standing gate so the
   next card cannot leave it stale either.
