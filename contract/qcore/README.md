# QCore contract regression corpus

These native QCore sources are the durable, source-readable form of the
external-library contracts used by the compiler's Fluent Bit integration.
They are grouped by library and kept independent so each file can be checked
directly with `qcorec`.

The regression test in `compiler/tests/qcore_contract_corpus.rs` recursively
discovers every `.qcore` file, parses its function contracts into structured
clauses, and runs the complete compiler driver. It also pins the expected
contract-bearing declarations for each source, so deleting or accidentally
weakening a fixture is a test failure.

From the repository root:

```sh
cargo test -p qcore-compiler --test qcore_contract_corpus
cargo run -p qcorec -- check \
  contract/qcore/chunkio/import_composition.qcore
```

The clauses are retained in the source AST for future semantic passes. Today,
the driver checks their grammar and preserves their spans and expressions; it
does not yet use arbitrary contract predicates to mint a safety certificate.

Two contract modules contain three bounded semantic checks against actual C source:
`fluent_bit/busy_state.qcore` and `chunkio/lock_state.qcore`. Their function
names, preconditions, return relations, and frame conditions are defined here;
the Rust checker interprets these files rather than duplicating their
postconditions in Rust tables.

The source audit runs Clang, imports the complete supported function body,
and checks every `ensures normal` clause over all four entry combinations of
the Boolean ghost fields `busy` and `lock`. `old_busy` and `old_lock` denote
entry values; `busy` and `lock` denote exit values; `result` is the C integer
return. The bounded predicate language supports integer literals, these five
ghost names plus helper-written `field_NAME` values, addition, subtraction, parentheses, and `==`/`!=`. Unknown
predicates and unsupported clause kinds are rejected, not assumed true.

The live-chunk, exclusive-access, and Boolean-domain `requires` clauses are
caller assumptions. Source checking does not establish them at arbitrary call
sites. The imported body may access the selected field and source-validated constant-field
helpers; their disjoint write footprints establish the other field's frame condition
in this bounded model. A negative C return
is still an `ensures normal` exit, not a QCore typed exception.

Run the semantic source checks with the existing Fluent Bit build database:

```sh
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- ../fluent-bit
```

Look for `qcore-contract ...: Checked` for the three field functions. The audit additionally
prints an explicit `Unknown` for every unmodeled function in `cio_chunk.c` and
`flb_input_chunk.c`. Unsupported source bodies never receive a field summary.
These field contract checks do not promote either translation unit to `Safe`.
The ordinary `qcorec check FILE.qcore` command continues to perform the compiler
and contract-syntax checks; use the audit above to bind these contracts to C.

Every contract source now begins with `module contract;` and compiles without
an artificial `main`. `module library;` similarly checks helper definitions
without a process entry. See [module modes and closed `where` bindings](../../docs/QCORE_MODULE_MODES.md)
for syntax, multi-file behavior, compiler API changes, and current boundaries.

`fluent_bit/memory.qcore` is also evaluated on executions of the actual
`include/fluent-bit/flb_mem.h`, compiled with Fluent Bit's include/define
profile and allocator observation hooks. Its five functions passed 145
recorded executions, including deterministic allocator failures. This is
runtime contract evaluation, separate from the static Boolean-field
source checks above. See the [native evaluation report](../../docs/reports/cycle19-native-contracts/README.md)
for reproducible commands, assumptions, and exclusions.

The recent Monkey execution policies also have native semantic consumers:
`monkey/callback_binding.qcore` contains 15 replay rules and
`monkey/descriptor_observation.qcore` contains five descriptor identity rules.
These 20 contracts have 56 postconditions and are executed by the Rust binding
and observation checkers through the shared predicate interpreter. They cover
scope, typed memory/symbols, complete and unique replay, call results and order,
map release, injected-failure labeling, descriptor generations and terminal
custody. Mutating any of these contracts changes the corresponding checker
result; unsupported predicates fail closed.

These parameterless functions are validation helpers over interpreter-derived
ghost facts. Concrete C resource APIs and callback postconditions remain in
`monkey/composition.qcore`. Rust retains the schema decoder, expression and
state-transition implementation, representation bounds, and conservative source
grammar. Compiling a QCore file alone checks syntax; the native runner connects
these rules to recorded executions. The [native-rule report](../../docs/reports/cycle19-native-rules/README.md)
records eight exercised callback paths, the labeled injected failure, bounded
wrapper source projections and remaining composition gates.

## Declarative resource-projection contracts

`../cycle19-resource-contracts.json` explicitly selects five modules for the
legacy-C audit: `libc/memory.qcore`, `monkey/memory.qcore`,
`librdkafka/memory.qcore`, `fluent_bit/resource_memory.qcore`, and
`chunkio/resource_open_close.qcore`. These 19 declarations replace the audit's
Rust table. They are trusted resource-projection assumptions, separate from the
more detailed execution and lifecycle profiles above. In particular the
ChunkIO close declaration does not itself provide a nested teardown certificate.

The version-1 vocabulary begins with `requires resource_contract_v1`. Normal
postconditions use `resource_borrowed(p)`,
`resource_{owned,consumed,released,out}(p, nullable|nonnull)`, or
`resource_{stored,removed}(p, owner, nullable|nonnull)`. An aggregate owner must
be a distinct owned parameter. Each parameter has exactly one mode; out requires
a pointer-to-pointer. Results use `resource_return_none()`,
`resource_return_owned(nullable|nonnull)`, or
`resource_return_reallocated(p, nullable|nonnull)` with an owned input.

Signatures are opaque C ABI projections, not checked C layouts, and their type
vocabulary is an explicit table rather than a wildcard. The custody-bearing
pointer projections are `ptr<mut, u8>`, `ptr<const, u8>` and
`ptr<mut, ptr<mut, u8>>`. The scalar projections — `bool`, `i8`, `i16`, `i32`,
`i64`, `isize`, `u8`, `u16`, `u32`, `u64`, `usize` — carry sizes, counts,
flags, descriptors and offsets, so they are **borrowed only**: every custody
mode on a scalar is rejected. `unit` is a result type only. Float scalars are
deliberately absent; no resource API in scope passes one, and the loader adds
no vocabulary without a use.

This is the existing abstract resource model: borrowed arguments have no
custody requirement, null reallocation failure preserves old custody, and
successful replacement invalidates it once. Unsupported requirements,
effects rows, error/any/quant clauses, non-null error sentinels, unknown modes,
duplicates and incomplete mappings reject the entire selected environment.
Do not add these clauses to an observation module and expect either interpreter
to ignore clauses it cannot check.

### Which interpreter owns which clause family

`contract/qcore` is shared by three consumers with disjoint vocabularies, and
`requires resource_contract_v1` is the selector between them. Of the corpus's
160 declarations, 19 carry the marker and load; the other 141 are **rejected**
by the resource loader, not skipped, so asking it for a `ResourceContracts`
environment over an observation or lifecycle module fails closed instead of
quietly returning a weaker table.

| Clause family | Owner |
| --- | --- |
| `requires resource_contract_v1` and the `resource_*` predicates above | `contract_loader` (this section) |
| integer/Boolean ghost predicates — `result == 0`, `field_NAME`, `busy`, `lock`, `allocations`, `releases`, `old(...)` arithmetic | `contract_observation` and the ChunkIO/source interpreters |
| named atomic lifecycle propositions — `chunk_is_live`, `arena_is_discharged`, `descriptor_custody_is_discharged_once`, … | the mechanism/lifecycle interpreters |
| `own<T>` / `borrow<'a, _, T>` parameters, `use` annotations, `effects { .. }` rows, `quant` clauses, `ensures any`, `ensures error(E)`, `result<T, E>` returns, and `resource type` / `error` / `axis` declarations | the QCore language front end and checker |

A resource projection of a lifecycle module is written as its own v1 module —
`fluent_bit/resource_memory.qcore` projects `fluent_bit/memory.qcore`, and
`chunkio/resource_open_close.qcore` projects the ChunkIO destructors — and is
never inferred by dropping predicates the loader cannot check. Non-null error
sentinels (a resource returned as `MAP_FAILED` or `-1` rather than null) stay
unrepresentable: the target interface records only nullability, so they need
interface work rather than loader vocabulary.

`contract_loader::load_resource_contract_manifest` loads these files at runtime;
`load_resource_contract_files` and `load_resource_contract_sources` expose the
same strict loader for other selections and immutable snapshots. Certificates
retain the exact sources and reload them during independent checking. The audit
writes the same inputs to `RESOURCE_REPORT.contracts.json` alongside a requested
resource inventory; its optional sixth positional argument overrides the module
selection manifest. Missing or unloadable input terminates the audit as an
error, without reporting a safety result from a default environment.

## Source-validated chunk helper dependencies

The bounded chunk-field importer can compose a helper only after independently
checking its complete body as constant writes to ordinary int metadata fields,
with a matching external declaration, matching direct record field layout, and
the same borrowed parameter identity. The pinned audit supplies the source,
build and cross-unit ABI identity; the importer does not prove linker resolution
or complete ABI equivalence. Helper source text travels in the field summary
and native contract certificate. This is a bounded dependency check, not general
SCC/fixed-point custody inference.

`chunkio/lock_state.qcore` now includes `cio_chunk_unlock`. Its `field_error_n`
postcondition checks the source-validated `cio_error_reset` write on both exits.
Every helper-written field requires a `field_NAME` postcondition; absent or
unsupported ghost values reject the contract. Const borrows reject helper writes
as well as direct field writes. `ChunkState` models busy/lock lifecycle effects;
metadata writes remain in the source paths and native contract result.

When a resource inventory path is supplied, the pinned audit also writes
`RESOURCE_REPORT.chunk-contracts.json`, containing all 96 selected native
chunk-function results, exact checked QCore contracts and dependency sources.
Unknown results remain in that inventory. The generic resource report and its
Safe/Unsafe/Unknown classifications are separate and unchanged by these checks.

## Extracted wrapper and source-policy contracts

The latest source findings are retained in three additional modules:

- `library/source_projection.qcore`: ten **executable** policies for evaluated
  enum values, scan effects/initialization, typed symbols and induction scope,
  source type matching, read-only scan regions, and frame-write/declaration/
  dependency checks. Both source importers consume these rules through the
  shared predicate interpreter. AST decoding and operational semantics stay
  in Rust; arbitrary C safety is not inferred from policy acceptance.
- `chunkio/error_state.qcore`: the **executable** `cio_error_reset` frame
  contract. `check_chunk_frame_contract` checks the reset constant, explicit
  write footprint and preserved lock against the source-validated helper.
  It requires an explicit mutable Chunk borrow with `use one`, unit result,
  live-storage and exclusive-access premises. Changed values, unmentioned
  writes, unsupported predicates and signature mismatches reject the check.
- `monkey/wrapper_paths.qcore`: **syntax-preserved behavioral specifications**
  for all 15 selected wrappers, including allocation/registration failures,
  descriptor close calls, timed waits and the injection scan. These predicates
  do not yet have behavioral/custody replay and do not issue a safety certificate.
  A close/free call is not asserted to have discharged its resource obligation.

The audit sidecars retain the exact executable source-policy/helper inputs and
label the wrapper behavioral specifications as unverified. These three modules
are intentionally outside the declarative resource-projection manifest: their
predicates are not approximated as acquire/release annotations. Corpus tests pin
all declarations and clauses across 25 modules; mutation regressions exercise
every extracted policy. See the [extraction report](../../docs/reports/cycle19-contract-extraction/README.md).

`chunkio/memory_backend.qcore` is checked against the complete typed
`cio_memfs_close` body over nine nullable ownership states. The shared field
replay derives `old_live__PATH`, `live__PATH`, `released__PATH` and
`release_rank__PATH` facts from `owned_or_null__PATH` premises. Double underscores
separate field components, so `backend__buf_data` denotes `chunk->backend->buf_data`.
The borrowed chunk and its pointer fields remain unchanged; source-derived
release does not make a dangling backend pointer reusable. The
[memory backend report](../../docs/reports/cycle19-memory-backend/README.md)
records the provenance/libc assumptions and remaining enclosing teardown gates.

`chunkio/counters.qcore` binds four unsigned counter helpers to their selected
field, source-derived unit `delta`, updated return and write footprint.
`counter_positive`/`counter_below_maximum` and range/ABI clauses are caller
premises; the C source does not guard against wraparound. The certificate checks
concrete values against a supplied target maximum when instantiated. See the
[counter report](../../docs/reports/cycle19-counters/README.md).

`chunkio/list_unlink.qcore` checks `mk_list_del` and its complete
`__mk_list_del` dependency under live reciprocal/exclusive link premises.
Both distinct and equal neighbor identities are replayed; the entry is distinct
from its neighbors. The contracts check link clearing/reconnection and preserve
object custody. See the [list unlink report](../../docs/reports/cycle19-list-unlink/README.md).

`chunkio/memory_close.qcore` composes the checked dependencies for the null and
memory-backed `cio_chunk_close` paths. It binds both embedded links and the saved
context counter after chunk release; filesystem/other paths stay uncovered.
The caller must establish context/stream lifetime, distinct memberships, owned
provenance and source/ABI equality. The backend contract now explicitly requires
owned fields to be distinct from the borrowed root. See the
[memory close report](../../docs/reports/cycle19-memory-close/README.md).

`chunkio/filesystem_attempts.qcore` contains four executable source result-use
policies. They record ignored cleanup results in `cio_file_close` and guarded
results in the unmap/Unix close helpers. These are producer-use facts, not OS
success or custody certificates. The [filesystem attempts report](../../docs/reports/cycle19-filesystem-attempts/README.md)
retains all 20 source paths and the unresolved failure obligations.

`chunkio/filesystem_outcomes.qcore` checks the null, absent, syscall-error and
syscall-success bookkeeping transitions of the Unix close and unmap helpers.
The typed symbolic interpreter binds actual syscall arguments and every field
write. External outcome/framing premises remain explicit; an error preserves C
fields but leaves OS custody unresolved. See the
[filesystem outcomes report](../../docs/reports/cycle19-filesystem-outcomes/README.md).

`chunkio/filesystem_observation.qcore` checks endpoint observations from actual
compiled Unix helpers with controlled syscall outcomes. Eighteen sanitized
executions cover both helpers' null/absent/error/success behavior and record
call identity, diagnostics and field frames. These hooks do not release OS
resources. See the
[cleanup observation report](../../docs/reports/cycle19-cleanup-observations/README.md).

`chunkio/unmap_wrapper.qcore` composes the source-derived native unmap and
up-counter helpers across all eight `munmap_file` paths. Only successful native
unmap clears wrapper sizes and decrements the saved parameter context. Sync/log
frames and provenance remain explicit premises. See the
[unmap wrapper report](../../docs/reports/cycle19-unmap-wrapper/README.md).

`chunkio/file_close.qcore` binds the non-deleting backend close chain to its
checked unmap/counter and descriptor helpers. It retains unresolved OS custody
through the subsequent backend allocation release: 26 of 43 conditional cases
fail the nested-discharge gate. Deletion paths remain uncovered. See the
[file close report](../../docs/reports/cycle19-file-close/README.md).

`chunkio/filesystem_delete_attempt.qcore` checks guarded unlink result use in the
four source paths of `cio_file_native_delete`. The shared importer now preserves
short-circuit fd/mapping guards and skipped reads/calls. Actual deletion outcome
and custody still require proof. See the
[short-circuit report](../../docs/reports/cycle19-short-circuit/README.md).

The filesystem wrapper policies now also state storage separation explicitly:
unmap's chunk/backend/context/stream storage is disjoint, and file close's owned
backend/path are disjoint from the borrowed chunk root. Both policies have 17
clauses. These remain caller provenance obligations; see the
[cleanup frame report](../../docs/reports/cycle19-cleanup-frames/README.md).

`chunkio/filesystem_delete.qcore` checks native deletion eligibility and both
unlink outcomes without consuming record/path allocations. Its nonnull record,
readable path and external-call frames remain explicit premises. See the
[native deletion report](../../docs/reports/cycle19-delete-outcomes/README.md).

`chunkio/file_close_delete.qcore` extends the checked close chain to delete one.
Thirty conditional cases retain blocked, failed, successful and absent-backend
outcomes through the following frees. Both delete modes together cover all four
source paths; success still needs bound OS/caller premises. See the
[deleting close report](../../docs/reports/cycle19-deleting-close/README.md).

`chunkio/filesystem_delete_observation.qcore` evaluates 16 controlled executions
of the actual pinned native-delete body. It checks eligibility, path forwarding,
status/diagnostic counts and record/path endpoint preservation. The returning
unlink hook performs no deletion. See the
[native delete observation report](../../docs/reports/cycle19-delete-observations/README.md).

`chunkio/filesystem_chunk_close.qcore` carries filesystem backend outcomes through
the enclosing chunk's two list removals, name/chunk frees and saved-context total
decrement. Its 75 conditional cases retain 39 unresolved OS cleanup outcomes;
only eight cases establish conditional backing deletion. Memory plus filesystem
coverage leaves unsupported backend kinds open. See the
[filesystem chunk close report](../../docs/reports/cycle19-filesystem-chunk/README.md).

The nested chain now has four additional executable policy modules:
`chunkio/teardown_traversal.qcore` records saved-next traversal and exact child
arguments; `chunkio/parent_teardown.qcore` records child-first parent allocation
release; `chunkio/nested_memory_teardown.qcore` binds complete ordered model
identities to the checked memory chain; and `chunkio/nested_cleanup.qcore`
preserves each filesystem residual and up-counter obligation through parent free.
All component and native caller/provenance/OS premises remain explicit.
These policies do not turn source matching, selected conditional outcomes or a
checked lifecycle prefix into a whole-unit Safe verdict. See the
[nested cleanup report](../../docs/reports/cycle19-nested-cleanup/README.md).

`chunkio/teardown_observation.qcore` checks five bounded native memory executions.
Its adapter derives ownership, reciprocal rings, nested release order and exact
counters from the pinned observer's allocation generations and event stream.
The observer/build and exclusive execution are trusted. The contract is runtime
evaluation only; it cannot establish a static certificate. See the
[native teardown report](../../docs/reports/cycle19-native-teardown/README.md).

`chunkio/constructor_failure_observation.qcore` evaluates the nine injected
allocation failures and one control on the same bounded memory path. Exact
attempt/rollback records and the surviving ownership tree account for every
observed allocation. Null optional fields, a null chunk name on a successful
return, and unchanged zero error output on backend failure remain explicit
observations, not general API guarantees. See the
[constructor failure report](../../docs/reports/cycle19-constructor-failures/README.md).

`chunkio/unsupported_close.qcore` specifies the conditional partial release for
all non-memory/non-filesystem integer backend kinds. The source interpreter
uses an excluded-constant symbolic class and rejects any further split of that
domain. Both links, name and chunk are released and the total counter decrements;
backend custody remains unresolved. This 12-clause policy cannot establish a
reachable caller, successful backing deletion or nested discharge. See the
[unsupported close report](../../docs/reports/cycle19-unsupported-close/README.md).

The existing `nested_cleanup` policy also counts unclassified backend children
in its residual inventory when matching symbolic close evidence is supplied.
Opaque stream/chunk/backend-kind identities survive parent teardown separately
from typed OS residuals; either inventory blocks discharge. The original
memory/filesystem cases retain their exact results. See the
[opaque parent report](../../docs/reports/cycle19-nested-unsupported/README.md).

`chunkio/unsupported_constructor_observation.qcore` evaluates four recorded
unsupported integer stream types. Native stream creation preserves each value;
chunk open returns null with unchanged error output and rolls back both local
allocations. The surviving empty stream/context is fully released. The
11-clause runtime policy and exact trace adapter establish only these executions,
not all caller inputs or universal reachability of the conditional close arm.
See the [constructor report](../../docs/reports/cycle19-unsupported-constructors/README.md).

`chunkio/filesystem_os_observation.qcore` checks twelve real Linux cleanup
executions against exclusive temporary files, descriptors and mappings. An
independent adapter checks raw arguments, errno, record frames, endpoint probes
and inode hardlink witnesses. The 13 clauses preserve the separation between
observed cleanup, static safety and durability. See the
[OS observation report](../../docs/reports/cycle19-file-os/README.md).

`chunkio/filesystem_teardown_observation.qcore` checks six native one-file tree
executions, including three suppressed-syscall EIO cases. Its 16 clauses require
complete allocation release while retaining mapping, descriptor and up-counter
residuals separately. Neither successful `msync` nor released parent allocations
prove durability. See the
[filesystem tree report](../../docs/reports/cycle19-file-tree/README.md).

`chunkio/filesystem_constructor_failure_observation.qcore` checks eleven ENOMEM
injections and a control on the bounded filesystem constructor path. Its 16
clauses conserve acquired generations across temporary, rollback and teardown
releases plus outstanding prefix generations. Sanitizer-interrupted processes
cannot claim completed teardown or leak checking; fixture cleanup is separate.
See the [filesystem failure report](../../docs/reports/cycle19-file-constructor-failures/README.md).

`chunkio/filesystem_os_constructor_observation.qcore` checks eight native
constructor OS-failure/control processes. Its 17 clauses separate allocation
release from descriptor custody after null returns and parent teardown. POSIX
allocation return/errno, file-size fallback and failed-mapping sentinels remain
distinct; harness reclamation cannot be mistaken for constructor rollback.
See the [OS constructor report](../../docs/reports/cycle19-file-os-constructors/README.md).

`chunkio/filesystem_existing_observation.qcore` checks seven native existing-file
profiles. The adapter binds generation/ring provenance, exact rollback/teardown,
original inode/bytes, distinct errors and descriptor residuals after suppressed
chmod/close failures. Read-only flags do not bypass configured chmod in these
observations. The 17-clause policy does not promote raw units or infer general
caller, corruption, ACL, OS-failure or durability behavior.

`chunkio/filesystem_transition_observation.qcore` checks five native residency,
transaction and lock sequences. It retains logical rollback separately from
mapped-header state and records successful direct write while locked. A new
mapping/open epoch is required on reload regardless of numeric address/fd reuse.
The 16-clause policy is a bounded runtime observation gate, not a universal
transaction, synchronization or production-caller guarantee.

`chunkio/filesystem_descriptor_failure_observation.qcore` checks four bounded
native down/reopen failure sequences. Its 20 clauses separate allocation release,
down-ring membership, descriptor epochs and parent residuals; harness reclamation
cannot count as library cleanup. Suppressed close retains a descriptor even when
down; suppressed reopen creates no new epoch. These runtime-only facts do not
establish arbitrary kernel failures, production reachability or durability. See
the [descriptor failure report](../../docs/reports/cycle19-file-descriptor-failures/README.md).
