# W1: clause vocabulary and interpreter boundary

Date: 2026-09-06. Status: W1's last open item — the richer clause vocabulary —
is closed. This is representation work only: no contract changed meaning, no
`.qcore` file changed, no verdict moved, no pinned expectation was edited and
no fail-closed gate was weakened. The remaining unification of the resource
loader with the observation/lifecycle interpreters is a **W5-adjacent** item,
not a W1 item; see "What is deliberately not here".

## Inventory

The corpus has 56 modules and **160 declarations** (141 `extern fn`, 11
`resource type`, 7 `error`, 1 `axis`). Each declaration was submitted to
`load_resource_contract_sources` in isolation, so a module's first failure
could not mask its siblings.

The result is a clean split: **`requires resource_contract_v1` is exactly the
selector.** All 19 marked declarations load; all 141 unmarked declarations are
rejected. No marked declaration fails and no unmarked declaration loads.

| Class | Clause forms | Declarations | Disposition |
| --- | --- | ---: | --- |
| mapped (v1) | `requires resource_contract_v1`; `resource_borrowed/owned/consumed/released/out/stored/removed`; `resource_return_none/owned/reallocated` | 19 | load |
| **(a)** bounded loader-vocabulary extension | C ABI scalar parameter/result types outside the old eight-entry table | 0 today | **landed** (see below) |
| **(b)** owned by another interpreter | see the four families below | 141 | reject, now pinned by test |
| **(c)** genuinely ambiguous | none found | 0 | — |

### Class (b), by owning interpreter

| Family | Examples | Owner | Decls |
| --- | --- | --- | ---: |
| integer/Boolean ghost predicates | `result == 0`, `field_error_n`, `busy`, `lock`, `allocations`, `releases`, `open_epochs`, `reference_count == old(reference_count) + 1` | `contract_observation` and the ChunkIO/source interpreters | 68 |
| named atomic lifecycle propositions | `chunk_is_live`, `arena_is_discharged`, `entry_has_list_custody`, `descriptor_custody_is_discharged_once`, `exact_parent_teardown` | the mechanism/lifecycle interpreters | (same declarations, mixed with the above) |
| QCore language-level resource syntax | `own<T>` / `borrow<'a, _, T>` parameters, `use one` annotations (65 parameters), `effects { .. }` rows (11 declarations), `quant` clauses, `ensures any`, `ensures error(E)`, `result<T, E>` returns | the QCore front end and checker | 54 |
| non-function declarations | `resource type` (11), `error` (7), `axis` (1) | the QCore front end | 19 |

These are not approximable into custody modes without dropping predicates the
loader cannot check, which is precisely what the loader refuses to do. The
established alternative is a **separate v1 projection module**:
`fluent_bit/resource_memory.qcore` projects `fluent_bit/memory.qcore`, and
`chunkio/resource_open_close.qcore` projects the ChunkIO destructors. Writing
new projections is contract authoring, not vocabulary work, and none was done
here.

### Class (c)

Empty. Every rejected form has an identifiable owner. One form is
well-defined but **unrepresentable rather than ambiguous**, and is recorded as
such rather than forced into a class: **non-null error sentinels** (a resource
returned as `MAP_FAILED` or `-1` instead of null). The plan's W1 task 1 lists
them, but `ResourceReturnMode` records only `nullable: bool`, so expressing
them requires extending the *target interface* in `qcore-schemas` and the
analyzer that consumes it — not the loader's clause vocabulary. No corpus
module declares one. They stay rejected, and the module header now says why.

## Vocabulary added (class (a))

The old loader carried one flat eight-entry `allowed_types` array
(`unit, i32, u32, u8, usize` and three pointer shapes). It is now two explicit,
documented tables with distinct roles:

- `SCALAR_ABI_TYPES` = `bool, i8, i16, i32, i64, isize, u8, u16, u32, u64,
  usize` — opaque projections of sizes, counts, flags, descriptors and
  offsets, **borrowed only**. Every custody mode on a scalar is rejected, as
  is a non-`None` result on a scalar return type.
- `POINTER_ABI_TYPES` = `ptr<mut, u8>, ptr<const, u8>, ptr<mut, ptr<mut, u8>>`
  — unchanged; the custody-bearing projections. `resource_out` still requires
  `ptr<mut, ptr<mut, u8>>`.
- `unit` remains a result type only, never a parameter type.

This adds `bool, i8, i16, i64, isize, u16, u64` to what a v1 signature may
**declare**. It carries no semantic content: a borrowed scalar maps to
`Borrowed` regardless of width, so no loaded contract's meaning changes and no
existing module's result changes. Floats stay out — no resource API in scope
passes one, and the loader adds no vocabulary without a use.

Motivation is a real signature, not speculation. `monkey/wrapper_paths.qcore`
declares `_mk_event_timeout_create(..., seconds: i64, nanoseconds: i64, ...)`
for the C `time_t`/`long` pair, and `chunkio/chunk_destructor.qcore` declares
`cio_chunk_close(chunk, delete_backing: bool)`. A future v1 resource
projection of the Monkey descriptor/timeout wrappers — an open Cycle 19 19D
gate — could not have been *written* under the old table. Note honestly that
**no corpus module consumes the new scalars today**; the round-trip tests are
the only current consumers, and no `contract/qcore` file was edited, so the
pinned corpus count tables in `compiler/tests/qcore_contract_corpus.rs` and
`contract/qcore/README.md` are deliberately untouched.

## Boundary made explicit

`crates/qcore-contract/src/contract_loader.rs`'s module header now opens with
an **interpreter-selection section** carrying a clause-family → owner table,
and states that an unmarked declaration is *rejected, not skipped*. The same
table is mirrored for humans in `contract/qcore/README.md` under "Which
interpreter owns which clause family", alongside the two ABI type tables.

## Tests

`compiler/tests/contract_loader.rs` grows from 8 to 10 tests:

- `every_abi_scalar_is_a_borrowed_only_projection` — round-trips each of the
  11 scalars as a borrowed argument (→ `Borrowed`) and as a result type
  (→ `None`), pins both ABI tables verbatim, and asserts rejection of
  `resource_owned` / `resource_released` / `resource_out` on a scalar, of an
  owned result on a scalar return, and of `unit`, `f32`, `f64` and
  `ptr<const, ptr<const, u8>>` parameters.
- `every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` —
  walks all 56 corpus modules, asserts per module **and per declaration** that
  loadability agrees exactly with the presence of the marker, pins the
  `(5 modules, 19 loaded declarations, 141 rejected declarations)` split and
  the manifest's 19 functions, and shows the marker is a selector rather than
  a bypass: injecting `requires resource_contract_v1` into
  `chunkio/chunk_destructor.qcore` still rejects, because the rest of that
  declaration belongs to the language front end.

The second test is the fail-closed regression the boundary needed: a future
change that made the loader skip clauses it cannot express, instead of
rejecting them, fails here against the real corpus rather than against a
synthetic mutation.

No production or example site was blocked on the old vocabulary. The
[literal-site record](../w1-literal-sites/README.md) already found zero, and
a fresh sweep confirms no Rust-literal contract table remains outside the
loader's own construction, the schema definitions and the loader's mapping
tests (`legacy_summary.rs`'s `insert_contract` is W4's validated inference,
not a literal table). There was nothing to migrate.

## Parity

Representation only; the pinned audit must not move, and it does not.

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit BUILD/compile_commands.json CALLBACKS.json DIAGNOSTICS.json
```

Run on this HEAD immediately before and after the change with identical
inputs: the resource-diagnostics JSON and the callback-source JSON are
**byte-identical**, and the audit log differs only in the echoed output
filename. Verdicts at this HEAD are 185 units, 4,775 raw source functions,
4,688 projected, 4 data-only, 3 Safe / 98 provisional Unsafe / 80 Unknown,
0 gateway failures — identical before and after.

That 3/98/80 split differs from the 2/98/81 pinned in the older
[contract-loader record](../cycle19-contract-loader/README.md) because the W4
validated-custody-summary work merged into this branch after that report was
archived; the difference is not attributable to this change, which is exactly
byte-neutral. Projection coverage is not safety coverage, and provisional
`Unsafe` classifications are not confirmed upstream defects.

## Standing gate

- `git -c protocol.file.allow=always submodule update --init --recursive`: exit 0.
- `cargo test --workspace`: exit 0 — 436 passed, 0 failed.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo run -p qcore-compiler --example reevaluation_probe`: exit 0.
- `cargo run -p qcore-compiler --example reevaluation_followup`: exit 0.
- `cd proof && lake build`: exit 0 (52 jobs). Lean is untouched: no semantic
  change, so no matching definition or theorem was required.
- Contract corpus counts and the Libevent submodule pin are unchanged and green.

## What is deliberately not here

- **Interpreter unification.** The observation and ChunkIO/lifecycle
  interpreters keep their own vocabularies. Merging them behind one clause
  dispatcher depends on the W5 task 3 chunk-summary schema split (the same
  dependency that keeps `chunk_contract` in `qcore-compiler`), so it is a
  **W5-adjacent** item, not remaining W1 scope.
- **Error sentinels** — target-interface work in `qcore-schemas` plus the
  analyzer, as described above.
- **New projection modules** for the lifecycle mechanisms
  (`cfl_list_add`, `rd_kafka_q_enq0`, the Monkey descriptor wrappers). Their
  shapes are already expressible; authoring them is Cycle 19 19D contract
  work with its own evidence requirements, not vocabulary work.
- **Pointer-shape widening** (`ptr<const, ptr<const, u8>>` and friends) was
  considered and not added: no corpus or production signature needs it.
