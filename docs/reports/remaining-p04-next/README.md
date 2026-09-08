# P04-NEXT — Remaining in-process ChunkIO capabilities through the process boundary

Workstream W5 (Providers). Bounded increment: **two** further ChunkIO
capabilities — the intrusive-list unlink (a two-summary relation) and the
nullable-field memory-backend destructor — now travel to a separate
schemas-only process as typed path projections and come back as witnesses the
trusted core replays. Nothing is promoted: no raw Cycle 19 unit changes class,
no CT case is mapped, no contract-corpus count moves, and the checking rules
that decide both contracts are byte-identical to the ones that already ran in
process.

- Worktree cut from `main`; the branch point in this worktree is `8d0790a`
  (two commits past the `60939ff` named in the card — the `quant[| |]`
  delimiter alias merge, which is unrelated to this card and appears in the
  `git diff 60939ff..HEAD` statistics for `crates/qcore-frontend` and
  `compiler/tests/quant_index_contracts.rs`).
- Baseline re-measured at that branch point by checking the base commit out
  into its own worktree: **1752 workspace tests passed, 0 failed, 3 ignored**;
  58 provider tests; 108 Python tests; 42 Lean audits. (An earlier in-session
  reading of 1734 raced with edits already on disk and is superseded; 1752 is
  independently corroborated by main's P09-G commit message, which records the
  same count for the same base.)
- Libevent submodule initialized at the pinned revision with
  `git -c protocol.file.allow=always submodule update --init --recursive`.
- The in-process adapters are **retained**, not replaced (invariant 3). Route
  removal is a later P09 unit.

## 1. Census of the thirteen remaining capabilities

`compiler/src/verifier_plugins/chunkio/` holds fourteen modules; two of the
capabilities they expose were already process-based before this card
(`chunk_source`'s Boolean field, and P04's unsigned counter). Every remaining
capability was read and classified by **which core checker decides it**, **what
a process would have to be able to say**, and **whether its named blocker is
engineering or structure**.

| In-process route | Core checker | Class | Reason |
|---|---|---|---|
| `teardown_source::check_list_unlink_source` | `qcore-verification::list_unlink_contract` | **movable-now — moved by this card** | P04's blocker ("relates *two* summaries, so a projection must carry both paths and their caller/callee relationship") is a vocabulary size, not a structure: both bodies are complete, single-path, call-free except for the one call into the co-transported helper, and neither releases an allocation. Transporting both *inputs* (never the summaries) is a strictly larger but still closed vocabulary |
| `teardown_source::check_memory_backend_source` | `qcore-verification::field_destructor` | **movable-now — moved by this card** | P04's blocker was "the path contains a `free` call, and a portable call vocabulary is exactly the custody-bearing surface this card is told not to widen". The unlink family already forces a call record, so refusing `free` on that ground alone would be arbitrary. The widening is bounded by an explicit **closed callee table** per family (the co-transported helper; exactly `free`), not by a general call vocabulary, and the core still decides release ordering itself |
| `file_attempts::check_file_attempt_sources` / `check_file_delete_source` | `qcore-verification` call-result def/use + `contract_observation` | movable-with-a-third-vocabulary — **deliberately not taken** | Its evidence is a `PathCallUse` def/use analysis (which call results are ignored, which are guarded, and their operation order), not a path replay. Moving it means a producer reimplementing the def/use algorithm over a third transported vocabulary. That is a separate capability definition, not a widening of this one, and its certificate is consumed by the two routes below |
| `file_outcomes::check_file_outcome_sources` | `qcore-verification::cleanup_outcome` | blocked-by-dependency | Its body is `check_file_attempt_sources(...)?.iter().map(check_cleanup_outcomes)`: it consumes another certificate, so under P04's own rule it can only follow its dependency. Its certificates additionally carry descriptor/mapping residual custody and feed the native-observation composition |
| `file_delete::check_native_delete_source` | `qcore-verification::cleanup_outcome` | blocked-by-dependency | Same shape: it consumes `check_file_delete_source`'s evidence and adds guarded deletion/`unlink` residual custody |
| `traversal::check_traversal_source` | `typed_source_match::check_source_match` | **blocked-structural** | `SourceMatch` is a sealed reviewed-library wrapper with no public constructor, conversion or deserialization (W5 typed-body boundary). A process cannot propose one without breaking that boundary. Unchanged from P04 |
| `parent_teardown::check_parent_source` | `typed_source_match::check_source_match` | **blocked-structural** | Same sealed boundary |
| `memory_close::check_memory_close_source` | composition | composition-only | Takes `MemoryCloseDependencies { reset, backend, unlink, counter }` — four other certificates |
| `filesystem_chunk_close::check_filesystem_chunk_close_source` | composition | composition-only | Calls `memory_close` and consumes a `FileCloseDeletionEvidence` |
| `unsupported_close::check_unsupported_close_source` | composition | composition-only | Consumes `FilesystemChunkCloseEvidence` and rechecks it |
| `file_close::check_file_close_source` / `check_file_close_deletion_source` | composition | composition-only | Composes the file attempt/outcome/delete certificates |
| `unmap_wrapper::check_unmap_wrapper_source` | composition | composition-only | Composes the unmap outcome certificates |
| `nested_memory::check_nested_memory_source` | composition | composition-only | Takes two `TraversalSource`s, two `ParentSource`s and a `MemoryCloseCertificate` — so it is *also* blocked behind the sealed `SourceMatch` boundary |
| `nested_cleanup::check_nested_cleanup_source` | composition | composition-only | Same shape, one level higher |

Summary: **two moved**, one movable-with-a-third-vocabulary and deliberately
not taken, two blocked by dependency, two blocked structurally by the sealed
`SourceMatch` boundary, seven compositions over other certificates.

### Why these two, and why the call vocabulary is still closed

Both are genuine capabilities rather than compositions: one contract module
each (`contract/qcore/chunkio/list_unlink.qcore`,
`.../memory_backend.qcore`), no dependency on another certificate, no native
observation, no descriptor or mapping custody in the statement, and both are
live production code that the Cycle 19 compatibility adapter checks.

Calls are admitted only against an explicit closed callee table matched on
**both name and declared C signature**:

- the unlink family admits exactly the co-transported helper; a call to
  anything else refuses the projection at import;
- the destructor family admits exactly `free` at `void (void *)`.

There is no catch-all record and no general call vocabulary. A body calling any
other function keeps its existing `Unknown` — pinned by a test that replaces
`free(mf->name)` with a declared `cio_free(mf->name)` and by one that adds a
declared `mk_list_touch(entry)` call to the unlink entry.

## 2. What actually crosses the boundary

The request carries `TeardownSourceInput`: a schema version plus one
`TeardownFamilyInput`.

- `ListUnlink { entry, helper, node_c_type }` — **both bodies' inputs travel;
  neither `SourceSummary` does.** Each body is a `TeardownFunctionRecord`
  (function, source revision, imported C signature, complete path set, declared
  profile assumptions) over a bounded operation vocabulary
  (`Read`/`Write`/`LocalWrite`/`LocalEnd`/`Guard`/`Call`/`Return`) and a
  bounded expression vocabulary
  (`Parameter`/`Symbol`/`Integer`/`Field`/`Cast`/`Unary`/`Binary`).
- `FieldDestructor { destructor, root_c_type, declared_owned_fields }` — one
  multi-path body plus the owned-field inventory the pinned contract declares.

**No derived result travels with either** — no case list, no link writes, no
release order, no verdict. This is checked in three places: the core-side test
asserts the serialized projection has no `cases`/`writes` key, and the
end-to-end test asserts the same over the real transported request.

The `declared_owned_fields` deviation is deliberate and recorded: the inventory
is part of the *question* (it selects which nullable states must be
enumerated), exactly as `root_c_type` is on the counter route. It is
contract-derived, so `FieldDestructorSourceTask::derive` re-reads the
inventory from the pinned contract itself with
`qcore_verification::field_destructor::declared_owned_fields` and **refuses any
task whose projection names a different set**, before the task exists. A
producer therefore cannot choose a smaller domain to answer over.

The process (`providers/common/teardown_effect.rs`, shipped in
`qcore.chunkio` 0.6.0) walks the transported paths on its own and returns a
`TeardownProposalRecord`:

- `ListUnlinkRecord`: for each of the two neighbor-alias classes, the ordered
  link-cell writes and the number of helper calls;
- `FieldDestructorRecord`: for each realizable nullable state, which fields
  were live, which were released and in what order, and on which path.

The core (`qcore-verification::teardown_source`) then:

1. refuses any task whose target identity, dialect, revision, platform, backend
   or Clang target does not match its own import (and, for the unlink family,
   any pair of bodies that disagree on revision or identity);
2. **evaluates the pinned contract itself** with the unmodified
   `check_list_unlink` / `check_field_destructor`, over its own summaries; and
3. requires the proposal to equal the reading it computed from that
   certificate.

Acceptance therefore terminates in the core's own evaluation. Agreement alone
discharges nothing, and a Safe proposal over a false contract is refused
(exercised below in both families). Ring membership, exclusive link access,
allocation provenance, field disjointness and the `free`/libc binding remain
the contracts' declared premises, recorded in each task's assumptions — this is
not a lifecycle, aggregate-teardown or whole-unit claim.

The sealed imports live in `crates/qcore-c-gateway/src/teardown_source.rs`:
`ListUnlinkSourceImport` and `FieldDestructorSourceImport` are constructed only
by their importers, carry the importer-captured identity, have private fields
and no `Deserialize`, and each has a `compile_fail` doctest against
deserialization.

### Obligations

Each route derives **one** obligation over its subject function, under a new
property (`ListUnlinkContract`, `FieldDestructorContract`). The unlink
obligation's scope names **both** checked bodies, because both are checked.

The destructor route is the first derivation to emit
`PathBounds::BoundedPaths`: the body is multi-path and the exact path count is
bound into the task, so a re-import with a different path set is a different
task. The variant, its serde representation and its meaning are unchanged; only
the doc comment saying no derivation emits it was corrected.

## 3. Wire and schema versions

Per `crates/qcore-schemas/README.md`'s documented policy, two of the three
additions are source-incompatible for exhaustive matches and struct literals:

| Addition | Kind |
|---|---|
| `SemanticWitness::TeardownContract` | new variant on an exhaustively matched enum |
| `ObligationProperty::{ListUnlinkContract, FieldDestructorContract}` | new variants on an exhaustively matched enum |
| `VerificationRequestV2::teardown_source` | new required struct field (skipped when absent) |

Additive was preferred and is not available: no existing witness variant can
hold these readings (`ContractObservation` is a recorded-run record set;
`CounterContract` is a single-path unsigned update), and the
`NormalizedFact::subject` JSON encoding has no `deny_unknown_fields`, no
schema-version field and no `validate()` rule, so it would be a strictly weaker
boundary than the typed field the three earlier projections established.
**Flagged prominently:** this is a major schema bump.

| Boundary | Before | After | Why |
|---|---|---|---|
| `qcore-schemas` crate | 6.0.0 | **7.0.0** | Exhaustive-variant precedent from 2.0.0, 4.0.0 and 5.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Boolean / counter / reactor projections | unchanged | unchanged | Untouched |
| Teardown source input | — | 1 | New |
| List unlink semantic profile | — | `qcore.c_list_unlink_contract.v1` | New |
| Field destructor semantic profile | — | `qcore.c_field_destructor_contract.v1` | New |
| Teardown input digest key | — | `qcore:teardown-source-input:v1` | New |
| ChunkIO package | 0.5.0 | **0.6.0** | Adds the two capability routes |

Existing wire bytes are unchanged: `teardown_source` is an `Option` that
defaults to absent and is skipped on serialization, exactly like the three
projections before it. `crates/qcore-schemas/tests/teardown_request_wire.rs`
pins that a request carrying none of the four projections serializes with none
of the four keys and round-trips.

All five provider packages were rebuilt and verified under the new schema
version (`§7`).

**Deliberate pin updates:**

- `providers/conformance/tests/packages.rs` pinned the ChunkIO package manifest
  at 0.5.0; it now pins 0.6.0. Both new profiles are reached through the
  `chunkio` target, whose profile list is already `*`, so the certificate
  checker, Lean family, capabilities, limits, trust class and every other
  target entry are unchanged.
- `providers/conformance/tests/composed_five_providers.rs` pinned the same
  version in its shipped-package inventory; it now pins 0.6.0.
- `compiler/tests/fixtures/w3/provider/{Cargo.toml,Cargo.lock}` require
  `qcore-schemas` 7.0.0 so the pinned standalone-provider fixture's `--locked`
  build resolves.

No gate was weakened and no expectation was deleted.

## 4. Parity evidence (old routes retained)

The in-process adapters are **not removed**, so parity is archived as an
executable comparison: `compiler/tests/teardown_route_parity.rs` (4 tests)
imports the same C bodies through both routes.

- **Unlink:** the serialized `ListUnlinkCertificate` from the old route
  (`teardown_source::check_list_unlink_source`) is **equal** to the one the new
  route's `ListUnlinkSourceTask::evaluate()` recomputes. Both neighbor-alias
  classes survive, with all four writes in order
  (`(2,next,3) (3,prev,2) (1,prev,null) (1,next,null)`) and exactly one helper
  call per class.
- **Destructor:** the serialized `FieldDestructorCertificate` from
  `check_memory_backend_source` is **equal** to the one
  `FieldDestructorSourceTask::evaluate()` recomputes: nine realizable nullable
  states, with the fully-live state releasing
  `backend__name, backend__buf_data, backend__meta_data, backend`.
- **Refusal agreement, unlink** — four changed bodies the old route refuses are
  refused by the new route too (projection lost at import, or contract refused
  at replay): the helper relinking the wrong neighbor, the entry forgetting to
  null one of its own link cells, the entry writing a neighbor cell the helper
  owns, and the helper never being called.
- **Refusal agreement, destructor** — four changed bodies: a retained declared
  field, a retained backend, releasing the parent before a live child, and
  releasing one field twice.

## 5. Mutation and refusal matrices

Measured totals, in the E02 style, counted from the test sources rather than
estimated: **25 forged readings** driven through the two core checkers
(12 unlink + 13 destructor, each asserted to actually differ from the accepted
reading before it is submitted), **24 further core-side refusals**
(identity, binding, false contracts, derivation, import, request admission),
**34 provider-side refusals** over the real projected bytes, and **9
end-to-end Unknown outcomes** through the real built package — 92 rows, all
refused. No mutation was admitted, so no gate was changed and no repair was
required by the matrix itself.

### Core-side (`crates/qcore-verification/tests/teardown_source.rs`, 7 tests)

**Unlink, twelve forged readings, each refused against the core's own
certificate:** emptied cases; a dropped alias case; a duplicated case; swapped
alias classes; a relabelled alias flag; an inflated helper-call count; an
erased helper call; a dropped entry null write; reordered writes; a retargeted
link write; a changed written value; a renamed helper. Each forgery is first
asserted to actually differ from the accepted reading, so no row is vacuous.

**Destructor, thirteen forged readings:** emptied cases; a dropped fully-live
state; a duplicated state; reordered states; a claimed release that did not
happen; a hidden real release; a reordered release sequence; a relabelled live
set; a retargeted source path; a widened declared inventory; a renamed
function; a moved revision; a changed root C type.

**Identity and binding:** another family's proposal never satisfies this
checker; another witness kind (`ContractObservation`) never satisfies it; a
witness bound to another task digest is refused before replay; a task the
checker did not derive is refused even with a real witness.

**Anti-vacuousness / agreement is not enough:** a contract asserting three
writes instead of four, and a contract permitting a retained backend, are both
refused by the core's own evaluation, and the honest reading a provider would
compute cannot discharge either.

**Derivation refusals:** six incompatible target identities (the other family's
profile, `c99`, a moved revision, a foreign platform, an emulated backend, an
empty library) plus an empty translation unit, an empty contract and an empty
declared QCore type. For the destructor, three projected inventories that are
not the pinned contract's own (too small, too large, duplicated) are refused —
and the extracted contract inventory is separately asserted equal to the four
declared fields.

**Task identity:** changed source bytes, changed build definitions and a
changed contract each produce a different task digest, and the cross-task
witness is refused **in both directions**; a different declared QCore node type
is a different task.

**Import refusals (function stays Unknown):** an unadmitted callee in the
unlink entry; a helper whose arity is not the declared two-cell helper; an
unadmitted callee (`cio_free`) in the destructor; naming one function as both
entry and helper; an empty node C type; an empty field inventory.

**Request admission:** exactly one obligation of the projection's own family
for the function it names; the other family's obligation is refused; two
projections never travel together; an unsupported projection version and an
absent projection with a live obligation both refuse.

### Provider-side (`providers/common/teardown_effect_tests.rs`, 7 tests)

The two fixtures are the **real projections** the gateway produces for these
bodies (`providers/common/fixtures/*.json`), and a conformance test asserts the
live import still equals them, so the matrix cannot drift into a shape the
importer no longer emits.

- Accepted readings: both alias classes with their exact writes and helper
  counts; all nine realizable nullable states with the exact release order.
- **Unlink refusals (18):** wrong schema version; empty function; entry equal to
  helper; mismatched revisions; wrong entry signature; wrong helper signature;
  empty node type; an empty body; a missing trailing return; a second entry
  path; an oversized body; a foreign callee; swapped call arguments; a dropped
  call argument; a non-void call result; a read of a cell with no supplied live
  binding; a guard the family does not model; a second helper body.
- **Destructor refusals (16):** wrong schema version; wrong signature; empty
  root type; empty inventory; a duplicated field; an empty field name; an
  oversized inventory; a dropped declared field (the freeing path then reads an
  unmodeled field); releasing the parent while a declared child is live; a
  truncated path set; a duplicated path; a missing return; an oversized body; a
  duplicated release; a write inside a destructor path; a flipped guard.
- **Precision, not silence:** a helper that relinks the wrong neighbor and a
  destructor that stops releasing two fields both stay *readable*, and the
  reading changes — so the core's comparison and its own contract evaluation
  see the change rather than a normalized result. A declared field the body
  never touches is likewise reported as never released rather than refused.

### End-to-end through the real built package (`providers/conformance/tests/teardown_capability.rs`, 2 tests)

`python3 providers/package.py build` is run, the real `qcore.chunkio` package is
registered as a `ProcessProvider`, and:

- **positive, both families**: `ProposedVerdict::Safe`, certificate bound to the
  task, witness kind `teardown_contract`, the unlink inventory naming
  `mk_list_del`/`__mk_list_del` with two cases of four writes and one helper
  call each, and the destructor inventory with nine cases and the exact release
  order;
- the transported request contains `paths` for both bodies and **no** `cases`;
- **missing evidence**: no core checker → Unknown with QP016 and no
  certificate; no registered provider → Unknown; disabled provider → Unknown;
  the shipped `qcore.libevent` package, which does not implement this
  capability → Unknown with no certificate;
- **tampered transport**: a well-formed helper body the core never imported
  (`prev->next = prev`) is refused by the host's digest binding with **QP001**;
  rebinding it to a recomputed task digest gets past that and is then refused
  by the core's own comparison, with no certificate;
- **false contracts**: a Safe proposal over a three-write unlink contract and
  over a destructor contract permitting a retained backend are both rejected
  with **QP009**;
- changed source bytes and changed build options each retire the checker's
  context.

## 6. Custody and audit invariants

No allocation, descriptor or mapping custody path was touched. `check_list_unlink`,
`check_field_destructor`, the in-process `teardown_source` adapter, and the
`cleanup_outcome`, `file_*`, `*_close`, `nested_*`, `traversal` and
`parent_teardown` routes are **unedited**. The only edits to existing
production files are additive: a new schemas module, a new optional request
field that serializes away, one witness variant, two obligation properties, one
host digest check, one `predicate_for` arm, one additive read-only contract
extractor (`declared_owned_fields`), and a new gateway/verification/compiler
module each.

**The canonical CMetrics audit was re-measured, before and after, in this
worktree** on a freshly prepared pinned clone (Fluent Bit `66910c10`, explicit
Clang), with the five provider packages rebuilt at each side:

```
diff cmetrics-audit-before.json cmetrics-audit-after.json
45c45   "package_digest": 156a792b… → 193faaba…
134c134 "package_digest": 156a792b… → 193faaba…
```

Two lines, both the rebuilt `qcore.cmetrics` executable's digest — the same
effect REASON-CODES measured when it rebuilt the packages. **Every verdict,
diagnostic, source digest, compile-task identity and the checked/Unknown split
are byte-identical**: one checked function contract, six Unknown functions,
`resource diagnostics: leak=2, unknown_alias=2, unsupported=35`, and a
provisional Unsafe raw unit. The complete before/after reports and the diff are
archived beside this file.

The four re-measured corpus artifacts are not re-pinned by this card: they are
identical to the base, so the archived baseline under
`docs/reports/remaining-o04-arms/artifacts/` stands unchanged.

The `chunkio-core` second source set and the synthetic configured-audit routes
are covered by `crates/qcore-audit/tests/second_source_set.rs`,
`root_source_set.rs`, `configured_cli.rs`, `source_contract_config.rs`,
`compile_options.rs` and `compile_task_identity.rs`, and by the
`configured_audit`, `configured_source_contract` and `cmetrics_unit`
conformance tests, all of which pass unchanged.

**Nothing is promoted.** Raw Cycle 19 units remain 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only units; the contract corpus remains 60
modules; full language conformance remains false. CT coverage was 24/50 mapped
at this branch's base commit and is 28/50 on `main` after wave-8's
T08-SRC-CT — this card maps no CT case and changes neither number.

## 6.1 The 185-unit corpus, re-measured before and after

The full pinned corpus was additionally re-measured on both sides, over one
freshly prepared explicit-Clang tree (Fluent Bit `66910c10`, Monkey with forced
Libevent), from the **same working directory** with
`QCORE_CLANG=/usr/bin/clang` fixed on both runs:

| Run | Result |
|---|---|
| Base commit `8d0790a` | 185 source files, 186 configurations: 2 Safe / 43 provisional Unsafe / 137 Unknown / 4 data-only / 0 gateway failures |
| This branch | 185 source files, 186 configurations: 2 Safe / 43 provisional Unsafe / 137 Unknown / 4 data-only / 0 gateway failures |
| `diff -r` of the four serialized artifacts | **byte-identical** |

`diff -r before2/ after/` over `resource-diagnostics.json` (16 MB),
`resource-diagnostics.contracts.json`,
`resource-diagnostics.chunk-contracts.json` and `monkey-source.json` reports
**no difference at all**. The 43 is the current-route **configuration** count
that CORPUS-REPIN already reconciled with the archived 42 **files**:
`src/flb_input_chunk.c` appears twice under G01's recorded
compile-configuration ambiguity. No unit verdict moves in either direction and
no provisional Unsafe unit is newly surfaced.

Two measurement artifacts are recorded rather than hidden, because both were
found by doing the comparison rather than assuming it:

1. An earlier attempt ran the base side without `QCORE_CLANG`, which moved
   every recorded `program` field from `/usr/bin/clang` to `clang` and, with
   it, every derived `inventory_sha256`. That is an environment difference, not
   a code difference.
2. A second attempt ran the two sides from two different worktree directories.
   The contract-source map is keyed by **absolute path**, so every key, the
   `resource_contract_sources` digest over them and the derived
   `inventory_sha256` moved — 2,246 lines. Filtering that diff for any line
   that is *not* one of those three mechanical fields returned **zero lines**:
   no verdict, no diagnostic, no coverage number, no source digest and no
   compile-task argument differed. Both sides were then re-run from the same
   directory, which is the byte-identical result above.

That the recorded compile-task identity is sensitive to the checkout's absolute
path is a property of the existing identity scheme, not something this card
introduces; it is noted here because it makes naive cross-worktree artifact
comparison misleading.

## 7. Lean inventory

`proof/QCore/TeardownAdmission.lean` and `proof/AuditTeardownAdmission.lean`
add **26 audited theorems** in a new `QCore.TeardownAdmission` namespace, with
**no new axiom** (only `propext`, `Classical.choice` and `Quot.sound`, the
allowed closure). The moved relation's content is the admission rule, so that
is what is modelled:

- *the core's own evaluation decides* (5): a refusing core admits nothing;
  every admission is backed by an evaluation the core ran; an admitted proposal
  **is** the core's reading; two admitted witnesses cannot disagree; changing
  what the core computes retires the witness.
- *identity binding* (5): cross-task, foreign-task and cross-family refusals;
  admission implies all three identity facts; and non-vacuity — the core's own
  reading is admitted.
- *the two-summary question* (4): with any injective identity recorder, a
  changed helper, entry or contract is a different question, so an entry
  witness does not survive a helper substitution. This is the fail-closed
  direction the card names for the two-summary relation.
- *the declared inventory* (8): the memory-backend inventory has exactly nine
  realizable states (decided, matching the measured certificate); dropping one
  declared field leaves five, so a smaller inventory is a strictly smaller
  question; a state naming an undeclared field and a child without its parent
  are unrealizable; the empty state and the full inventory are realizable; only
  the contract's own inventory is accepted.
- *both families inhabited* (4): each family admits a reading and neither
  answers for the other.

Judged **not applicable** and recorded rather than modelled: the transported
vocabulary's faithfulness to Clang's typed AST (a gateway/importer premise, as
on every existing source route), and the contract-evaluation bodies themselves,
which are unedited and already covered by `QCore.ListUnlink` and
`QCore.FieldDestructor`.

## 8. Gate results

Run in this worktree on Linux with Clang available:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1768 passed, 0 failed, 3 ignored** (1752 baseline + 7 core replay + 4 parity + 3 schema wire + 2 gateway `compile_fail` doctests = 1768, exactly) |
| `cargo fmt --all -- --check` | clean (workspace and `providers/`) |
| `cargo clippy --all-targets -- -D warnings` | clean (workspace and `providers/`) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **43 audits passed**, no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **67 passed, 0 failed** (58 baseline + 7 provider unit + 2 conformance) |
| `python3 -B -m unittest discover -s scripts/tests` | 108 passed |
| `python3 providers/package.py build` + `qcorec plugins verify` × 5 | all five packages built and verified (exit 0) |
| Canonical CMetrics audit, before vs after | 2 lines, both rebuilt `package_digest` |
| 185-unit corpus, before vs after, same directory | all four artifacts **byte-identical** |

Every row above was re-measured after the worktree was recreated mid-card (see
§8.2), so the recorded numbers are from the worktree as it stands.

## 8.2 Process note

The isolated worktree for this card was pruned by an external cleanup while the
card was paused, which briefly left the shell in the main checkout and moved
that checkout onto this branch. It was restored to `main` at `92a7032`, the
worktree was recreated at the same path from the same branch, and every commit
survived (`git log` shows all five). No work was lost and no other worktree or
branch was touched. The full gate was then re-run from the recreated worktree,
which is what §8 reports. `build/cycle19/` scratch trees for the corpus
measurement live under the main checkout's ignored `build/` directory, where
the preparation scripts write by design.

`lake` is not on the default `PATH` on this host; it was run from
`~/.elan/bin`.

## 8.1 For the integrator: overlap with `main` after wave 8

This branch is based on `8d0790a`; `main` has since advanced to `92a7032`
(P09-G, T08-SRC-CT, CT 28/50). The overlap is small and recorded here rather
than resolved on this branch:

- **P09-G removed `chunk_source::import_chunk_source`** (the zero-helper
  convenience wrapper) and moved `qcore-compiler` to 0.3.0. This card never
  calls that function and adds nothing to `chunk_source`; the new modules are
  `teardown_source` under new names in the gateway, the verification adapter
  and the compiler facade. **No textual overlap with the removed wrapper.**
- `compiler/Cargo.toml` is touched by both: this branch changes only the
  `qcore-schemas` requirement line (`6.0.0` → `7.0.0`); P09-G changes only the
  crate's own `version` line (`0.2.0` → `0.3.0`). Different lines, same file —
  take both.
- `crates/qcorec/Cargo.toml` is touched by P09-G only; this branch does not
  edit it. Its `qcore-compiler` requirement should stay at main's `0.3.0`.
- The schema bump to **7.0.0** rewrites the `version = "6.0.0"` requirement in
  six manifests plus `providers/Cargo.toml`,
  `compiler/tests/fixtures/w3/provider/{Cargo.toml,Cargo.lock}` and
  `providers/Cargo.lock`. If any wave-8 card also bumped a schema requirement,
  those single lines are the only contention.
- Three request-literal sites gain `teardown_source: None`
  (`crates/qcore-verification/src/{verify_dispatch,counter_source,source_effect,reactor_source}.rs`
  and five test files). Any new `VerificationRequestV2` literal added on main
  will need the same field.
- The report's "CT coverage" line records 24/50 at this branch's base and
  28/50 on main; this card maps no CT case.

## 9. Open items and named blockers

- The in-process adapters for both moved capabilities are retained. Their
  removal, after this parity record, is a later unit (P09).
- **Neither moved capability has a configured audit route yet.** They are
  exercised by the parity, replay and conformance tests, not by the Cycle 19
  compatibility adapter, so no production report changes and no raw unit moves.
  Wiring them into a configured audit is the substantive next step and is not
  done here.
- `file_attempts` is movable with a third transported vocabulary (call-result
  def/use) and was deliberately not taken; it is recorded as **P04-NEXT-A**.
  `file_outcomes` and `file_delete` follow it (**P04-NEXT-B**).
- `traversal` and `parent_teardown` stay blocked by the sealed `SourceMatch`
  boundary. They need a different design — a reviewed-body attestation the core
  can re-derive — not a bigger vocabulary. Recorded as **P04-NEXT-C**.
- The seven composition modules can only follow their dependencies; two of them
  (`nested_memory`, `nested_cleanup`) additionally sit behind `SourceMatch`.
- The two routes bind no profile-declared model library: they select functions
  by explicit name, as the in-process routes do, so `target.library` is checked
  for presence but not against a declared field or link model. Profile-declared
  teardown selection is open (**P04-NEXT-D**).
- The destructor projection carries the contract-declared owned-field
  inventory. It is cross-checked against the pinned contract before a task
  exists, but it is still a datum the request carries that is derived from the
  contract rather than from the source; that deviation from P04's "no derived
  result" phrasing is recorded here rather than hidden.
- The 185-unit corpus was not re-pinned by this card — it was re-measured
  byte-identical on both sides (§6.1), so the archived baseline stands; see §6
  for what was measured instead.
