# CONNECT-ADOPT-DECLARED-DEPENDENCIES: every verdict reliance, declared

Round 40, base `1ecaf97e9`, branch `r40-adopt-dependencies`. User decisions of
2026-11-26: "Every reliance", "which is related to validate whether Safe or
Unsafe"; the linkage syntax (`premises { link { quant #link CRATE::FUNCTION }
... }`, `quant #link CRATE::FUNCTION`, a module-level `#include ... as quant
#link`); "This custody linkage can be standardized by linkage_contract_v1";
and, on the census hand-back (commit `df682b93d`), "Accept all".
[PREDICTION.md](PREDICTION.md) was committed before implementation
(`64a21fefb`).

## The rule, as checked formulas

Ten CX terms were added to the C04.18.9 typed rendering (the table authority
is [scripts/cx_block.py](scripts/cx_block.py), which extends the frozen
QUIRIN-AUDIT-CONNECT copy): `cx_reads`, `cx_decides`, `cx_relies`,
`cx_co_used`, `cx_link_required`, `cx_check_passes`, `cx_links`,
`cx_inherits`, `cx_marked`, `cx_load_refused`. Thirteen formulas
(`qcore_schemas::connect::FORMULAS`, 73 in all, each checked by
`guarded_proposition` in `crates/qcore-frontend/tests/connect_formulas.rs`):

| Formula | States |
|---|---|
| `cx_relies_is_a_deciding_read`, `cx_deciding_read_relies` | a reliance is exactly a reading that can decide a `Safe` or an `Unsafe` |
| `cx_required_is_a_cross_bundle_reliance`, `cx_cross_bundle_reliance_is_required` | the rule requires exactly the cross-bundle reliances |
| `cx_reliance_is_linked` | `((#cx_link_required(#b,#g,#c,#h) && #cx_check_passes(#b)) => (#cx_links(#b,#g,#c,#h) \|\| #cx_inherits(#b,#g,#c,#h)))` |
| `cx_unlinked_reliance_fails_the_check` | an unlinked reliance fails the linkage check |
| `cx_undeciding_read_not_required` | converse: a reading that yields only `Unknown`/`Pending` is no reliance |
| `cx_co_use_is_not_reliance` | converse: two declarations used by one row are no reliance |
| `cx_same_bundle_not_required` | converse: no same-bundle link is required |
| `cx_link_needs_marker`, `cx_marker_needs_link` | a link without `requires linkage_contract_v1;`, or the marker without a link, refuses the module |
| `cx_link_is_a_dependency`, `cx_dependency_is_a_link` | the C15.2 dependencies are exactly the links and inherit directives |

The existing `cx_refuse_libc_target_elsewhere` is the libc converse. Lean
(`QCore.MeasureConnect`, audited in `AuditMeasureConnect.lean`): one theorem
per formula, and `elaboration_is_the_dependency_set` (the model's
dependencies are the elaboration of its written links),
`link_forms_elaborate_alike` (the form is not part of the dependency) and
`a_held_link_target_holds_its_dependent` (from
`held_dependency_holds_dependents`). Standard axioms only, no placeholder.

## Census

The census covers 14 crates: 11 on the shipped load path and 3 on the user
load path (libcurl, onigmo, openssl). It counts 189 modules and 1,149 distinct
declared function names per crate. Every edge is in [census.json](census.json), which
[scripts/census.py](scripts/census.py) writes. Each entry gives the clause or
reading, the target's lock identity, the Cargo edge and whether the edge can
be satisfied under C15.2. The body readings come from
[scripts/callees.py](scripts/callees.py) over
`plugins/in_storage_backlog/sb.c` and librdkafka 2.15.0 `src/rd.h`. Both
files were read from the recorded linked source,
`private/current-linked-556460e31/run/fluent-bit-clang-source`.

| Dependent crate | Target crate | Edges | Declarations | Kind | Can C15.2 satisfy it? |
|---|---|---:|---:|---|---|
| fluent-bit | libc | 6 | 6 | 5 inherit, 1 body check (`memset`) | yes (Cargo edge) |
| librdkafka | libc | 5 | 5 | 3 inherit, 2 rule-37 checked specifications | yes |
| libevent | libc | 3 | 3 | inherit | yes |
| fluent-bit | qcore-provider-libevent | 5 | 5 | 4 body checks (`mk_list_add`/`mk_list_del`), 1 call frame | **no** |
| fluent-bit | qcore-provider-chunkio | 11 | 3 | body checks (`cio_chunk_*`) | **no** |
| **total** | | **30** | **22** | 11 inherit, 18 body check, 1 frame | 16 not satisfiable |

The census finds three reliance kinds:

1. **Inherit** (11 edges): `contract inherit libc::F`. The child elaborates
   to the parent's clauses, so every discharge through the child reads the
   parent's clause. The five `#include` edges (4 modules) are all covered by
   these 11 edges, because every included module supplies an inherited
   parent.
2. **Body-checked specification** (18 edges).
   * `resource_body_checked` on 8 storage-backlog declarations. A
     specification is withdrawn unless the gateway's body reading discharges
     it, and that reading applies each callee's declaration.
   * Rule 37 `rd_calloc` and `rd_strdup`. Their nonnull results are
     established through `libc::calloc` and `libc::strdup`.

   Every one of these is a reading that can decide a verdict. The edge is
   required only if the specification can discharge. A specification whose
   body also calls an undeclared function (for example `cio_chunk_lock`,
   `cio_error_get` or `mk_list_init`) may never discharge. If so, its edges
   are only `Unknown`-yielding readings and are not required. Before the
   re-pin, each specification's established status at `1ecaf97e9` must be
   confirmed from a measured report.
3. **Call frame of another crate's function** (1 declaration, 2 clauses):
   `flb_config_set_program_name` holds `quant cell mk_list_del:
   flb_input_instance |-> #untouched` and the same clause for
   `flb_output_instance`. `legacy_cell_frames` applies these frames at every
   `mk_list_del` call. A `Safe` across that call rests on the frame and on
   `libevent::mk_list_del` together.

Readings that are not required, each with its ground:

* **libevent observation records.** The libevent crate names libc symbols
  in strings: `callback_task_handoff.qv` `effect_send`, `effect_perror` and
  `effect_errno_location`; the `timeout_teardown.qv` `path_calls`; and the
  `native_platform_matrix.qv` `backend == "poll"`. These name a C symbol and
  read no libc clause. Follow-up CONNECT-LIBC-EFFECT-CLAIMS: decide whether
  task and defer effect claims about libc callees belong in
  `quirin-provider-libc`.
* **Gateway built-ins.** `__assert_fail` and the other no-return names in
  `null_deref/ast.rs` are built into the gateway and are not declarations.
  The rule-37 failure arm uses them.
* **Shared words.** `close`, `time`, `read`, `write`, `access`, `listen`,
  `shutdown` and `exit` appear as clause vocabulary or parameter names. They
  only spell libc function names.
* **Type and member names.** Struct types and member paths, such as the
  mpack types in cmetrics and ctraces and the fluent-bit types, are not
  declarations.
* **Co-use in one function row.** A report's per-row
  `contract_dependencies.declarations` lists both declarations, but
  `cx_co_use_is_not_reliance` excludes this.
* **Same-crate readings.**
  * The storage-backlog bodies call `flb_calloc` and `flb_free`. These are
    fluent-bit declarations that reach libc only through the inherit edges.
  * `rd_realloc` is written out and is not body-checked. libc declares no
    `realloc`.
* **Duplicate `mk_list_del`.** `chunkio/list_unlink.qv` also declares
  `mk_list_del`, but that module is not in the resource view. The resource
  route reads `monkey/intrusive_list.qv`, so the target is
  `libevent::mk_list_del`.
* **User crates.** onigmo, libcurl and openssl read no other crate's
  declaration. Their checkers compare declarations with documentation
  snapshots. The QUIRIN-AUDIT-CONNECT trial's `onig_new -> libc::malloc` was
  a scratch demonstration, not a reliance.

**Re-pin count.** 22 declarations, under the stop bound of about 150; with D2
the 11 inherit children keep their text and pins, so 11 are re-pinned.

## Decided ("Accept all") and applied

* **D1 (a).** A dependency resolves through the load path's contract-level
  graph: the dependent's crate and the target crate on one load path, the
  graph acyclic (`QI014`, `cx_cycle_holds`); Cargo dependencies are recorded,
  no longer read (`qcore_schemas::connect::local_failures`; the
  `cx_declared_crate` gloss changed, the formulas did not). An unrecorded
  digest now matches nothing (`Pending`), so an identity-less link can never
  resolve.
* **D2.** A `contract inherit NS::NAME;` directive is its own link: the
  manifest records it (`form: inherit`) with the parent's identity; the 11
  children keep their text and pins. Inheritance copies neither a link nor
  the marker (`contract_dependency::strip_linkage`).
* **D3.** The body-checked edges stay: a specification's own row is analysed
  under its entry custody even when it is not established, so its body
  reading of the callee can yield an `Unsafe` (it can never yield `Safe`).
  Measured evidence: the Priority 0 chain
  (`docs/reports/priority0-remeasurement/chain/chain.json.gz`) has
  `sb_remove_chunk_from_segregated_backlog` `Unknown -> Unsafe` at
  `9aed1913a` (system route), a leak at `sb.c:226`, the `mk_list_del`
  unlink. In the same chain `sb_allocate_backlogs`, `sb_destroy_backlogs` and
  `sb_segregate_chunks` are `Unknown` at all three sides; the other rows did
  not move, so their status is the base record's. No edge was dropped; each
  establishment status was not remeasured at `1ecaf97e9` on this host.
* **Syntax (G1-G7, grammar G05m: `G05l` was already allocated).** `quant #link
  NS::NAME;`; `premises { ... };` (an optional grouping: its clauses are read
  in place, no existing premise moved) holding `link { quant #link ...; };`;
  `;` terminators, blocks closed `};`; `#link` reserved only at the start of a
  clause; namespace spelling in all forms; `#include NS::NAME as quant
  #link;` only makes `NAME` nameable (the elaborator writes bare links
  qualified, so their digest is the qualified one); no link in `[[local]]`.
* **L1-L6.** `requires linkage_contract_v1;` per declaration; C15.2 is the
  `linkage_contract_v1` section; the new loader-owned `QL001`-`QL009`
  namespace (`docs/agents/diagnostics.md`); a change is v2; the vocabulary is
  in every manifest entry, report graph entry, `quirin-contract-composition/2`
  and (through the report graphs) `quirin-measure-chain/2`; qcore-schemas
  30.0.0; the written `resource_contract_dependency(...)` is retired (`QL005`)
  and survives only as the elaborated record (`DeclaredDependency::clause`).

**Contracts changed (no other clause).** `git diff 1ecaf97e9 --
providers/*/contracts` adds only `requires linkage_contract_v1;` and `quant
#link ...;` lines:

| Crate | Declarations | Lines added |
|---|---|---:|
| quirin-provider-fluent-bit (`storage_backlog.qv`) | 8 storage-backlog specifications | 24 |
| quirin-provider-fluent-bit (`teardown_destroy.qv`) | `flb_config_set_program_name` | 2 |
| qcore-provider-librdkafka (`memory.qv`) | `rd_calloc`, `rd_strdup` | 4 |

**Re-pins.** `quirin-audit pin --source-root` at exactly the revisions each
entry was reviewed at before (the 8 storage-backlog entries at 046accf6 and
66910c10; `flb_config_set_program_name`, `rd_calloc` and `rd_strdup` also at
34155955; script `private/r40/pin.sh`, log `private/r40/pin-1.log`). A
comparison of every lock entry before and after: only `contract_sha256` and
`pinned_at` of the 11 entries changed; every identity's revision-to-sites map
is unchanged; no other entry changed.

**Lock and manifest diff.** `providers/contract-load-path.json`: 30
`contract_dependencies` entries (fluent-bit 5 inherit + 1 link to libc, 5
links to libevent, 11 to chunkio; librdkafka 3 inherit + 2 links to libc;
libevent 3 inherit to libc), each with `since`, `sha256`, `form` and
`vocabulary`. `contract-inventory.lock`: fluent-bit `storage_backlog.qv` and
`teardown_destroy.qv`, librdkafka `memory.qv` clause counts (+2 or more per
changed declaration; declaration counts unchanged); librdkafka
`contracts.json` the module digest. `contracts.lock`: the 11 re-pins above.

**Enforcement.** `providers/contracts.py check` runs `check_linkage` against
the reviewed record [providers/linkage-reliances.json](../../../providers/linkage-reliances.json)
(15 declarations: the 14 `resource_body_checked` specifications, 4 of them
with no cross-crate callee, and the frame): it refuses a body-checked
declaration or a frame of another crate's function the record lacks, a
recorded reliance without its link, and a link the record does not list.

## Fixtures

* Frontend (`crates/qcore-frontend/tests/linkage_syntax.rs`, 5): the user's
  example shape (`premises { link { ... }; quant live ...; };`) parses to flat
  clauses; a single link and a bare link; misplacement (`link` outside
  `premises`, nested `premises`, a non-link in `link`, an empty block, a
  missing `};`, a `.` terminator, two targets, a nested qualifier, `#link` as
  a condition, a link in `[[local]]`); `#link` as a binder in a proposition is
  unaffected; the module-level import directive and its malformed spellings.
* `qcore_contract::contract_dependency` (7): targets, the marker rule
  (`QL001`, `QL002`, `QL004`), the retired form (`QL005`), no inheritance of
  links, the block-free token reconstruction, a module scan reading all three
  forms and inherit directives, import refusals (`QL003`, `QL006`).
* `qcore_schemas::connect` (13): the contract-level graph (no Cargo edge
  needed; a dependent off every load path unresolved), the link spelling per
  form, the vocabulary in the identity, a record without the new fields read
  as a v1 link.
* `qcore_audit::declared_dependencies` (8): a resolved link; a stale digest
  after a target re-pin holds `Pending`; out of range, a missing crate and an
  unloaded target hold `Unknown`; a link the manifest does not record is held
  `Pending`; a dependent off every load path `Unknown`; a recorded inherit
  directive resolved like a link; adopting a link changes the digest; the
  retired form refused.
* `crates/qcore-audit/tests/linkage_shipped_resolution.rs`: every shipped link
  resolves on the cycle19 module set with the shipped manifest: held set
  empty, no cycle, 11 inherit + 19 link dependencies.
* Providers Python (`providers/tests/test_contract_load_path.py`):
  `ContractDependencies` (3: the entries with identity in text order; an
  unknown crate or function `QL007`, the retired form `QL005`, a repeated or
  unused import `QL006`, a bare name without import `QL003`) and
  `LinkageCheck` (3: a wrapper release resting on libc `free` must link it;
  a body-checked declaration must be recorded; a reading that only yields
  `Unknown` requires no link and a link to it is refused). A libc name under
  another crate is refused by `sync` (`QL008`) and by the resolver
  (`dependency-libc-elsewhere`); a cycle by the manifest (`QI014`) and the
  existing `a_cycle_is_held_unknown`.

