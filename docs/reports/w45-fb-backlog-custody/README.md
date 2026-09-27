# W45-FB-BACKLOG-CUSTODY: intrusive-list iteration, element identity and checked backlog specifications

Card W45-FB-BACKLOG-CUSTODY, 2026-09-26. Base `74579af6`; `main` merged in
twice: `84ebad57` (W45-FB-RETRY-LIFETIME) at `95b7399e`, then `2f9131a8`
(W45-CONTRACT-REVISION-MODE) at `d6c1f756`. The measured code is `6b07bd5d`
(the narrowed §2.5; its arbiter and slice reports are byte-identical to
`d6c1f756`'s). P1.1
of the [Fluent Bit system plan](../../plans/fluent-bit-system-coverage/README.md),
on the slice [W45-FB-SYSTEM-SELECTION §5](../w45-fb-system-selection/README.md)
chose.

Two user amendments apply:

* the obligations are **checked provider contracts** reported by
  `quirin-audit`;
* the slice is **re-pinned** from `864c3d97` to Fluent Bit master
  `046accf6aded7906f66a1baf0202da349e521736`.

[PREDICTION.md](PREDICTION.md) was frozen before implementation, on the base.
The integration review asked for one repair (§2.5), and it is in the measured
code.

## Headline

* **Four generic mechanisms**, gateway constraint 5l:
  1. **Intrusive-list iteration** (`legacy_list_iteration.rs`): the
     preprocessed `mk_list_foreach`/`cfl_list_foreach` headers, plain and
     `_safe`, are recognised on their original spelling and modelled; a plain
     iteration refuses unlinking or releasing its current element, a safe one
     its saved successor.
  2. **Element identity and its declaration**: the `container_of` GNU
     statement expression binds untracked list memory, and the declaration
     reader skips the one declaration inside it instead of refusing the whole
     function. That silent refusal was the dominant "declaration" blocker
     (criterion 2).
  3. **Checked specifications** (contract spec §C06.3, `legacy_checked_spec.rs`):
     `requires resource_body_checked;` makes a declaration a specification that
     is in force only where its body establishes it. It composes with the
     revision and review markers.
  4. **A call refused before its effects hands on its actuals** (§2.5): the
     W44 refused hand-on rule now also fires where a call is refused on an
     unresolved member-address or bound-read aggregate.
* **Contracts**: `providers/quirin-provider-fluent-bit/contracts/fluent_bit/storage_backlog.qcore`,
  twelve checked specifications of the backlog functions. They are bound by
  name and signature, reviewed, and pinned in `contracts.lock` at `046accf6`
  and at the `66910c10` corpus source; `sb.c` is identical at both.
* **Corpus** (185 units), against the baseline `main` reproduces:
  * functions **389 / 12 / 4,433 → 398 Safe / 9 provisional Unsafe / 4,427
    Unknown**;
  * units **4 / 10 / 167 / 4 → 5 / 8 / 168 / 4**;
  * leak **37 → 34**, unknown_alias 3,571 → 3,570, unsupported 42,702 →
    41,325;
  * 1,480 leaves removed and 99 added in 192 functions, all attributed;
  * **no Safe → non-Safe move**, and the new module moves **no** corpus
    diagnostic.
* **The one added leak** is `mk_http_error`'s unclosed `open()` descriptor, a
  provisional candidate and not a defect claim. §2.5 withdraws the two artifact
  leaks the first submission added, and the four standing
  `MK-LIST-ADD-BORROWED-PUBLICATION` leaves as well.
* **Slice at 046accf6**, against `main`'s slice: units **0 / 2 / 13**,
  unchanged; functions **24 / 2 / 208 / 3 → 28 / 2 / 204 / 3**; unsupported
  4,325 → 3,338. Every `mk_list_foreach` and `mk_list_entry` refusal in `sb.c`
  is gone.
* **Obligations**: none of OB-1 … OB-10 is decided. Two custody legs are held
  (`sb_allocate_chunk`, `sb_destroy_chunk`), and every obligation's relational
  part is a named residual. OB-2 and OB-5 are **not** refuted (§5).
* **Publication reach from the first merge.**
  `sb_append_chunk_to_segregated_backlog` is no longer blocked by
  publication: its one remaining blocker is the byte counter's indirect access
  (OB-8). But the retirement legs of OB-3, OB-7 and OB-10 now meet rule 10d's
  refusal of an unlink the function did not publish (§5).
* **Lean**: `QCore.ListIteration` (13 audited theorems),
  `QCore.CheckedSpecification` (7) and `QCore.RefusedCallHandOn` (5), all
  registered.

## 1. Bases, merges and re-pin

| Measurement | Result |
|---|---|
| Arbiter BEFORE on the card's base `74579af6` | Reproduces [w45-parenthesised-callee](../w45-parenthesised-callee/integration/artifacts/): diagnostics, `chunk-contracts` and `monkey-source.json` byte-identical |
| Arbiter BEFORE on `main` `2f9131a8` (a private export outside the worktree, its own target) | Reproduces [w45-fb-retry-lifetime](../w45-fb-retry-lifetime/integration/artifacts/), the unchanged baseline: diagnostics, `chunk-contracts` and `monkey-source.json` byte-identical, `verdict_bearing_identical: true` ([before-identity.json](artifacts/arbiter/before-identity.json)). The contracts sidecar differs only by `main`'s new revision section. |
| Slice BEFORE at `864c3d97` (attempt-1, base binary) | Reproduces the published figures, 0 differing function rows ([comparison](artifacts/slice/before-864c3d97-vs-published.json)) |
| Slice BEFORE at `046accf6` (attempt-2, base binary) | units 0 / 3 / 12; functions 25 / 2 / 207 / 3 ([report](artifacts/slice/before-046accf6.json.gz)) |
| Slice BEFORE at `046accf6` (attempt-2, `main` `2f9131a8` binary, run in `main`'s own tree) | units **0 / 2 / 13**; functions **24 / 2 / 208 / 3**: the post-retry figures ([report](artifacts/slice/before-2f9131a8.json.gz)) |

**Merges.** Both merges kept every card's hooks and modules.

* With W45-FB-RETRY-LIFETIME: `caller_formals` beside
  `list_facts`/`list_frames`, and both cards' modules in the crate and the
  manifest. The pins count both: 50 selected modules, 114 in the corpus, 174
  resource-v1 declarations, 50 resource views
  (`providers/tests/test_contract_load_path.py`).
* With W45-CONTRACT-REVISION-MODE: `resource_body_checked` composes with the
  revision and review markers. It is an ordinary contract-level requirement
  read after both are peeled, neither may wrap it, and an unpinned or
  withheld declaration is never a specification. This is pinned in
  `contract_body_checked.rs`, and this card's section became contract spec
  §C06.3.
* No schema version was bumped: this card adds no schema field.

**Pins.** The twelve `storage_backlog.qcore` declarations were reviewed against
the `sb.c` bodies and pinned (`scripts/pin.sh`) at `046accf6` (attempt-2's
clone) and at the `66910c10` corpus source (read-only). Every function's
identity is the same at both revisions. `pin --check` at both revisions and
`pin --verify-contracts` exit 0. `contracts.lock` gains exactly those twelve
entries.

**Re-pin.** attempt-2 is a fresh private clone and build at `046accf6`
([pin-attempt-2.json](artifacts/slice/pin-attempt-2.json): clone HEAD equals
the pin). Between the revisions, `plugins/in_storage_backlog/sb.c`,
`lib/chunkio/` and `mk_list.h` are unchanged (`sb.c` SHA-256 `705012c1…`
at both). Only `src/flb_input_chunk.c` changed among the backlog files
([diff](artifacts/slice/flb_input_chunk-864c3d97-046accf6.diff)):
`flb_input_chunk_write_header_v2`, `input_chunk_get` and
`input_chunk_append_raw`. `flb_input_chunk_map` is unchanged, and
`flb_input_chunk_destroy` is unchanged apart from a +12 line shift.

The base binary's difference from `864c3d97` is **source change measured
with one binary**, not analyzer change
([comparison](artifacts/slice/before-046accf6-vs-864c3d97.json)): one new
Unknown function, `flb_output_task_flush_error`, and diagnostic changes in 13
functions of four files.

**Native controls.** The partial-fan-out and zero-progress observations of
[W45-FB-BACKLOG-FAULT-INJECTION](../w45-fb-backlog-fault-injection/README.md)
were made at `864c3d97`. Their code (`sb.c`, ChunkIO) is identical at
`046accf6`, and so is `flb_input_chunk_map`'s `routes_mask` arm. So OB-2's A2
prediction and OB-5's BC-1 prediction still describe the re-pinned source.
They remain native observations and move no verdict.

## 2. Mechanisms

### 2.1 Intrusive-list iteration (criterion 1)

`legacy_list_iteration::recognise_header` reads the header's three clauses
**with their groups intact**:

* plain: `C = (H)->next ; C != (H) ; C = C->next`;
* safe: `C = (H)->next, N = C->next ; C != (H) ; C = N, N = C->next`;
* and the `prev` forms.

`H` must be `& R -> L… -> F`, `& S . F` or a plain `R`, typed through the
chain reader. The lowering:

* the init is `IndirectScalarAccess` over `H`'s root, with its chain
  obligation;
* the condition is supported and keeps both edges, including the exit edge;
* every cursor assignment is an `IndirectScalarAccess` whose destination is
  the cursor. The cursor is therefore unbound (untracked list memory), and a
  read through it passes the ordinary dereference obligation. A safe
  header's second increment node is `Statement::For::list_step`.

**Rule 4** (`frame_refusal`) keeps the two forms apart. In a **plain**
iteration, a statement that hands the current node (`C`, `&E->F`) to any
call, or `C` or an element bound from it to a non-`Borrowed` position, is
refused. A **safe** iteration protects `N` the same way and admits unlinking
and releasing the current element. That release then refuses on custody,
because an element of a borrowed container is untracked.

**Premises**, stated in the module and in rule 5l:

* P-ILI-BODY: the body does not write or address a cursor (declined
  otherwise);
* P-ILI-UNTRACKED: an element is no tracked identity (declined when one call
  receives both `&X->…` of a possibly tracked `X` and the iterated head);
* P-ILI-CALLEE-FRAME: a callee handed the container does not unlink the
  protected node;
* P-ILI-BRACED: the body is braced.

**`W45-FOR-CLAUSE-ORIGINAL-SPELLING`** is discharged for this header family
only. Other headers keep rule 10c(3).

### 2.2 Element identity and the declaration blocker (criteria 1–2)

Criterion 2 asked which declaration or initialiser shapes block the slice.
[shape-census-before.json](shape-census-before.json) reads every leaf's
original line. In the corpus, 326 of the 2,284 braced leaves are
`container_of` element reads (`cfl_list_entry` 198, `mk_list_entry` 99,
`*_first`/`_last` 28, one direct), and so are all 11 braced leaves of the
slice's `sb.c` functions.

Behind each is a second, silent blocker: the statement expression
`({ const typeof(((T *)0)->F) *__mptr = (P); … })` contains a declaration
the typed declaration reader cannot read, so it refused the **whole
function**, and every typed route was off in it. Two changes follow:

* `element_expression_spans` lets `read_locals` and `unique_local_names`
  skip exactly the declaration inside a recognised element expression
  (`typed_field_access.rs`, two hooks);
* `list_element_binding` lowers `E = <element expression>` to an access that
  leaves `E` unbound when `P` is one of:
  * an active cursor;
  * a `(H)->next`/`->prev` read of an admissible head (`mk_list_entry_first`);
  * an unwritten, never-bound formal (the callback idiom).

  An element bound from the protected cursor joins the frame's protected
  set.

The other braced shapes (`RD_UT_ASSERT` 731, brace initialisers 315, call
lines 540, …) are not this card's and are unchanged.

### 2.3 Checked specifications (user amendment)

**Why a new marker.** A declared contract is applied at call sites whatever
the body proves: the mode gate (constraint 8) withdraws it only against a
*validated* W4 summary, and a body that does not validate has none. So for an
audited body a plain declaration would be a trusted replacement, which the
amendment forbids.

**The marker.** `requires resource_body_checked;` (loader:
`BODY_CHECKED_MARKER`, `checked_specifications`,
`declared_checked_specifications`) changes no mode and adds nothing to the
serialized environment. Inheritance compares it.

**The gateway.** `legacy_checked_spec::settle` runs before the mode gate. It
first withdraws a specification whose body the unit does not define, or whose
body is in a recursive component. It then descends: each round withdraws
every specification whose body, analysed under it with the specifications
still standing, is not established (the W4 gate
`observations_match_contract`, and no `SummaryConflict`). A withdrawn
specification leaves the environment: callers see W4's summary or an
uncontracted call. The defining row gains `declared specification of `NAME`
is not established: REASON`. Contract spec §C06.3 and grammar G05 are
extended.

### 2.4 The contract module

[`storage_backlog.qv`](../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/storage_backlog.qv)
declares the twelve `sb.c` functions:

* `sb_allocate_chunk`: owned-nullable result, borrowed arguments;
* `sb_destroy_chunk`: `resource_released(chunk, nonnull)`;
* the other ten: every argument borrowed, no custody result.

**`nonnull` for `sb_destroy_chunk`.** `nullable` is also true of the body,
but W4 validates `nonnull`, and the refinement would stand as a
`SummaryConflict` beside it (measured; §6). A checked specification never
survives a conflict.

**Left undeclared.** `flb_input_chunk_map` is left undeclared: no v1 mode is
faithful to its three `chunk` dispositions (stored on success, closed on the
`routes_mask` arm, left open on the other NULL arms).
`flb_input_chunk_destroy` keeps its existing declaration.

**Registration.** The module is in the crate's `CONTRACTS`, the load path
(`contracts.py sync`) and `cycle19-resource-contracts.json`, so the
published slice command applies it.

**Revision-sensitive.** The declarations name no revision, line or digest.
Fragile across revisions are only the C signatures (the ABI gate refuses a
mismatch) and the reading that `sb_destroy_chunk` releases through `flb_free`.

**No other crate is touched.** Monkey's iteration is a macro expansion, not
a function, so there is nothing to declare beside `intrusive_list.qcore`.
No ChunkIO fact was needed, because no obligation becomes decidable from one.

### 2.5 A call refused before its effects hands on its actuals (integration review)

**The finding.** The first submission added two leaks the review questioned:
`mk_rconf_section_entry_add` and `mk_vhost_fdt_worker_init`. They were
artifacts, and the mechanism is not rule 10d.

In both bodies the allocation is unchecked (`new = mk_mem_alloc(…)`, no NULL
test), so the name is possibly null at `mk_list_add(&new->_head, …)`. The
member-address liveness check (O04-B09B10) refuses the call **before any
effect applies**: rule 10d never runs, so the publication is never recorded
or refused. That check reports the unresolved identity (`UnknownAlias`), not
`Unsupported`, and W44-LEAK-PIN-AUDIT's refused hand-on rule fires only on
`Unsupported`. So the object a refused call may have published stayed live,
and it was accused at the exit.

The declaration reader (§2.2) exposed this: before the card the whole
function's typed routes were off, the call was an `Unsupported` indirect
access, and the hand-on rule withdrew the leak.

**The repair** (`refused_operands::withdraw_refused_call`, called from the
refusal branch in `legacy_c::transfer`):

* a call refused there marks, before the bindings are poisoned, the live
  identities its contract lets it move with an unresolved effect: the
  actuals at custody positions, the entries of its container effects
  (`mk_list_add`'s published entry) and the bound reads it takes;
* a `Borrowed` actual with no container effect is not handed on. The refusal
  is about the read's liveness, not the call's effect, and the existing
  `borrow_residuals_boundary.rs` control, a read through a released owner,
  keeps its leak. A first version that handed on every actual withdrew that
  leak, and the gate caught it;
* it only adds unresolved effects, so it only withdraws accusations;
* the node keeps its `UnknownAlias`, so the function is never `Safe`;
* it is not widened to every `UnknownAlias`, since most report a read whose
  effect still applies.

**Pinned** in `crates/qcore-c-gateway/tests/refused_call_hand_on.rs`, 6 tests:

* both corpus shapes: a nullable object published into a caller container,
  and into a local container, are withdrawn to Unknown;
* an object never published and never freed still leaks;
* an object the refused call does not name still leaks;
* an object published into a container rule 10d accepts, then unlinked and
  leaked, still leaks;
* an object published into an accepted container is discharged (Safe).

**Reach in the corpus.** The same mechanism was behind the four standing
leaves of `MK-LIST-ADD-BORROWED-PUBLICATION`: `mk_rconf_read`,
`mk_vhost_create`, `mk_vhost_set_property` and `mk_vhost_set_single`. Each
links an unchecked or refused-path allocation into its caller's configuration
(`mk_list_add(&file->_head, &conf->includes)`, …). All four are withdrawn
(provisional Unsafe → Unknown); none is discharged to Safe.

## 3. Corpus arbiter (185 units)

Final run on `6b07bd5d`: [artifacts/arbiter/](artifacts/arbiter/), the new
baseline (`run.json`, manifest digests). BEFORE is `main` `2f9131a8`, which
reproduces the unchanged W45-FB-RETRY-LIFETIME baseline (§1). The contracts
sidecar against it is in
[contracts-delta.json](artifacts/arbiter/contracts-delta.json).

**Attribution** is two sequential diffs through an intermediate build.
`scripts/decompose.py` rewrites hook lines, builds into a private target
outside the worktree and restores the bytes, with SHA-256 asserted. Every leaf
is listed in [attribution/](artifacts/attribution/). §2.5's repair is in every
build.

| Step | Removed | Added | Functions | Status moves |
|---|---:|---:|---:|---|
| BEFORE → declaration reader (§2.2's skip) + §2.5 | 695 | 93 | 125 | 4 provisional Unsafe → Unknown, 1 Unknown → provisional Unsafe |
| → + iteration and element rules (full) | 785 | 6 | 179 | 9 Unknown → Safe |
| **Total** (BEFORE → full) | **1,480** | **99** | **192** | 9 → Safe, 4 Unsafe → Unknown, 1 → provisional Unsafe |

Two further runs:

* The iteration rules **without** the reader, §2.5 included, remove 6
  leaves in 5 functions. Two leaves and one Unknown → Safe are the iteration
  rules, as measured before §2.5 existed; four leaks and four provisional
  Unsafe → Unknown are §2.5 alone (§2.5, reach).
* With `storage_backlog.qcore` removed from the manifest, the final binary's
  `resource-diagnostics.json` is **byte-identical**
  ([module-identity.json](artifacts/attribution/module-identity.json)).

**Step 1.** In the functions whose declarations it now reads, typed routes
replace blanket refusals.

* **Removed:** 621 `unsupported`, 70 `unknown_alias` and 4 `leak` (the four
  §2.5 withdrawals).
* **Added:** 67 `unknown_alias` (reads now reaching a flow check with an
  unresolved identity: `request` 9, `map` 8, …), 25 `unsupported` and **one
  leak**.
  * 17 of the `unsupported` leaves are rule 10d's `unlink by mk_list_del of
    X, which this function has not published`, in 14 teardown functions whose
    now-admitted safe iterations unlink an element. Each is an exchange,
    fail-closed.
  * The leak is `mk_http_error` (`mk_server/mk_http.c`): a descriptor from
    `open()` at :1301 is never closed or stored before the `return` at
    :1315. The source's own comment reads "This fd seems to be leaked". It
    is a provisional `Unsafe` candidate (`W45-MK-HTTP-ERROR-FD-LEAK`), not
    an upstream defect claim, and the only added violation-class leaf.

**Step 2.**

* **Removed:** 284 element-read braced leaves, 258 header indirect accesses,
  235 header `for`-loop conditions, and 8 leaves behind callees that now
  validate a summary.
* **Added:** 6 exchanges (five head-root read obligations, and one
  `nullable resource st` in `cio_stream_create`).
* **Nine read-only traversals become Safe:** `cio_memfs_scan_dump`,
  `cio_stats_get`, `cio_stats_print_summary`, `cio_stream_get`,
  `cio_stream_size_chunks_up`, `cmt_labels_count`, `is_file_included`,
  `mk_rconf_meta_get` and `mk_vhost_lookup`.

**Units.** `cio_stats.c` Unknown → Safe; `mk_server/mk_http.c` Unknown →
provisional Unsafe. `mk_core/mk_rconf.c`, `mk_server/mk_lib.c` and
`mk_server/mk_vhost.c` go provisional Unsafe → Unknown, because their only
accusations were the §2.5 withdrawals.

**Shape reach** ([shape-census-after.json](shape-census-after.json)):

* element-read braced leaves 326 → 43;
* list-header `for` leaves 269 → 34.

**Rule 4 in the corpus** adds no refusal. Of 17 plain iteration bodies with
an unlink or release call ([plain-unlink-census.json](plain-unlink-census.json)),
the two read release another object or unlink a second link. The other 15
were not read one by one; the audit reports no rule-4 leaf in them.

**Earlier measurements, kept for the record** (each is superseded by the
table above):

| Measured at | BEFORE baseline | Functions | Leak | Leaves |
|---|---|---|---|---|
| Card's base (tip `0a677729`) | pre-retry | 390 / 12 / 4,432 → 399 / 15 / 4,420 | 37 → 40 | 1,459 removed, 83 added |
| First merge (tip `95b7399e`), before §2.5 | retry | 389 / 12 / 4,433 → 398 / 15 / 4,421 | 37 → 40 | 1,476 removed, 101 added |

Those two runs carried the two artifact leaks §2.5 removes.

## 4. System slice at `046accf6`

Command (from the repository root; `scripts/run-current.sh`):

```sh
QCORE_CLANG=/usr/lib/llvm-21/bin/clang quirin-audit current \
  --checkout OUT/attempt-2/fluent-bit-clang-source \
  --profile-directory contract/profiles/fluent-bit \
  --compile-database OUT/attempt-2/fluent-bit-clang-production/compile_commands.json \
  --selection contract/profiles/fluent-bit/system-selection.json \
  --report OUT/attempt-2/slice-after-6b07bd5d/resource-diagnostics.json --jobs 8
```

`OUT` is `/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-backlog-custody`.
The profile's manifest (`cycle19-resource-contracts.json`) loads
`storage_backlog.qcore`, so this is the `quirin-audit` route that checks the
specifications. The obligation table is `scripts/ob_table.py` over that report
([ob-table.json](ob-table.json)).

**Tally**, against `main` `2f9131a8`'s slice (the post-retry figures; the pinned contracts are in force at `046accf6`):

* units 0 / 2 / 13, unchanged;
* distinct functions 24 / 2 / 208 / 3 → **28 / 2 / 204 / 3**;
* leaves: unsupported 4,325 → 3,338, unknown_alias 685 → 717, counted_user
  4 → 4.

The four functions that become Safe are list traversals, each in both
variants: `flb_sched_timer_coro_get`, `sched_timer_coro_get_by_id` and
`sched_timer_coro_get_id` (`flb_scheduler.c`), and `dlq_stream_total_size`
(`flb_storage.c`).

**The module's own share** is exactly the ten `not established` leaves on ten
rows, with no status change ([module-effect.json](artifacts/slice/module-effect.json)).

Backlog slice functions (`sb.c`, one unit; `flb_input_chunk.c` identical in
both variants). BEFORE is `main` `2f9131a8`'s slice. The AFTER rows are
unchanged by the second merge and by §2.5:

| Function | BEFORE | AFTER | Specification | Remaining blockers (AFTER) |
|---|---|---|---|---|
| `sb_allocate_chunk` | Safe (0) | Safe (0) | **held** | — |
| `sb_destroy_chunk` | Safe (0) | Safe (0) | **held** | — |
| `sb_append_chunk_to_segregated_backlog` | Unknown (1) | Unknown (2) | not established | one indirect access: the byte counter's saturation test (`backlog->ins->fs_backlog_chunks_size`, OB-8) |
| `sb_append_chunk_to_segregated_backlogs` | Unknown (15) | Unknown (9) | not established | uncontracted `cio_chunk_get_real_size`, `flb_input_chunk_get_tag`, `sb_append_chunk_to_segregated_backlog`; indirect accesses; a branch condition |
| `sb_segregate_chunks` | Unknown (25) | Unknown (19) | not established | element `chunk` released by `cio_chunk_close` (unresolved identity ×5); uncontracted `cio_chunk_lock`, `cio_error_get`, `sb_get_context`, own helpers; branch conditions |
| `sb_allocate_backlogs` | Unknown (7) | Unknown (8) | not established | its iteration declined by P-ILI-UNTRACKED (`mk_list_add(&backlog->_head, …)` beside the head); uncontracted `sb_destroy_backlogs` |
| `sb_remove_chunk_from_segregated_backlog` | Unknown (10) | Unknown (5) | not established | rule 10d: `unlink by mk_list_del of chunk, which this function has not published`; element release; a branch condition |
| `sb_remove_chunk_from_segregated_backlogs` | Unknown (4) | Unknown (2) | not established | uncontracted callee (its helper's specification is withdrawn) |
| `sb_destroy_backlog` | Unknown (4) | Unknown (2) | not established | one indirect access |
| `sb_destroy_backlogs` | Unknown (6) | Unknown (4) | not established | rule 10d's unlink refusal (`backlog`); element release; uncontracted `sb_destroy_backlog` |
| `sb_release_output_queue_space` | Unknown (12) | Unknown (6) | not established | uncontracted `sb_find_segregated_backlog_by_output_plugin_instance`, own helper; element `underlying_chunk` release |
| `cb_queue_chunks` | Unknown (42) | Unknown (26) | not established | uncontracted `flb_input_chunk_map`, `flb_input_chunk_get_event_type`, own helper; element releases; branch and loop conditions; indirect accesses |
| `flb_input_chunk_map` | Unknown (91) | Unknown (91) | undeclared | per-arm disposition (§2.4) |
| `flb_input_chunk_destroy` | Unknown (32) | Unknown (19) | existing declaration | indirect accesses, `ic` aliases |

Counts in parentheses are the row's leaves, including the one
`not established` leaf.

## 5. Obligations OB-1 … OB-10

Reading rule (`ob_table.py`): a function's v1 leg is **held** when its row is
`Safe`, **refuted** when the row is provisional `Unsafe` on a violation that is
not the registered publication class, **blocked-publication** when every
violation is that class, and **unknown** otherwise. An obligation is decided
only when all its legs are and a clause states its relation. No v1 clause
states a relation between a wrapper, a queue and a chunk.

| OB | Relation | BEFORE | AFTER | Blocker / residual |
|---|---|---|---|---|
| OB-1 | wrapper linked into exactly one queue, or never created | Unknown | **Unknown**; creation leg held (`sb_allocate_chunk`) | Publication (rule 10d, merged) now models the link and withdrew the old publication leak. The insertion leg fails only on the byte counter's indirect access (OB-8); "exactly one queue" is not stated by a clause (`W45-FB-BACKLOG-PUBLICATION-BLOCKED`, re-scoped). |
| OB-2 | no chunk closed while a wrapper refers to it (the `-3` arm) | Unknown | **Unknown** (not refuted) | Needs a wrapper→chunk reference relation, a return-conditioned clause and element identities; the chunk is an untracked list element in both functions (`W45-FB-BACKLOG-REFERENCE-VOCABULARY`) |
| OB-3 | at most one wrapper per chunk per queue | Unknown | **Unknown**; retirement leg held (`sb_destroy_chunk`) | List membership uniqueness (same residual). The unlink in `sb_remove_chunk_from_segregated_backlog` is refused by rule 10d: it unlinks a wrapper another function published (`W45-FB-BACKLOG-FOREIGN-UNLINK`). |
| OB-4 | after a successful map, no wrapper refers to the chunk | Unknown | **Unknown** | `flb_input_chunk_map` per-arm disposition (`W45-FB-BACKLOG-MAP-ARMS`) |
| OB-5 | chunk closed exactly once on a NULL map return | Unknown | **Unknown** (not refuted) | Same. BC-1 stays a source-reading candidate. No v1 mode states "closes on one NULL arm", so neither the double close nor its absence is decidable. |
| OB-6 | no wrapper used after a callee frees it | Unknown | **Unknown** | the callee frees what its argument reaches (`W45-FB-BACKLOG-REACHABLE-RELEASE`) |
| OB-7 | eviction retires every wrapper before the close; the saved successor stays valid | Unknown | **Unknown** | same, the foreign-unlink refusal inside the callee, and P-ILI-CALLEE-FRAME (a stated premise, not a proof) |
| OB-8 | byte counter equals the sum over linked wrappers | Unknown | **Unknown** | quantitative clauses (FB-BACKLOG-ACCOUNTING) |
| OB-9 | each chunk ends the pass in exactly one state | Unknown | **Unknown** | ChunkIO return-code summaries (`W45-FB-BACKLOG-CIO-RETURN-CODES`) |
| OB-10 | teardown retires every wrapper, touches live outputs only | Unknown | **Unknown**; retirement leg held | shutdown ordering (`W45-FB-SYNC-PREMISES`), reachable release, and the foreign-unlink refusal in `sb_destroy_backlogs` |

**Why OB-2 and OB-5 are not refuted.** A provisional `Unsafe` needs a
validated reachable violation in the resource projection, and neither is
expressible there.

* **OB-2's chunk** is an element of `stream->chunks`, which the projection
  does not track. The partial insertion happens inside a loop whose failing
  call has no return-conditioned summary.
* **OB-5's double close** needs `flb_input_chunk_map` to say which NULL arm
  closed the chunk.

The native partial-fan-out observation (ASan use-after-free) and the BC-1
reading stand as native and source evidence. Per the brief, they move no
verdict.

## 6. Controls and gates of the rules

`crates/qcore-c-gateway/tests/intrusive_list_iteration.rs`, 20 tests, every
fixture preprocessed from the pinned macro spellings.

**Accepting:**

* a plain read-only iteration (Safe);
* a head through a member chain (Safe);
* a safe iteration unlinking each element (Safe);
* a safe iteration that unlinks and frees, refused only on the element's
  custody (`UnknownAlias e`, no iteration refusal);
* removal before the close of a tracked resource (Safe);
* `mk_list_entry_first` (Safe);
* a leak inside an iteration kept (provisional Unsafe);
* a call handed only the head does not decline.

**Refusing:**

* unlinking the element, or the cursor, in a plain iteration;
* freeing the element in a plain iteration;
* unlinking the saved successor of a safe iteration;
* a body writing its cursor;
* linking a tracked object into the iterated list (declined);
* the partial-fan-out shape (a wrapper linked for a chunk, then the chunk
  released) and a stale wrapper read after its close: both never Safe.

The partial-fan-out pair is not *separated* by v1, because it has no
reference relation. What the tests pin is that neither shape certifies.

**Checked specifications:**

* an established `Released(nonnull)` specification is held, and its caller
  is Safe;
* a contradicted `Borrowed` specification is withdrawn: the row is Unknown,
  and the caller is Safe on the validated summary, where a *trusted*
  `Borrowed` declaration leaves the caller non-Safe;
* a specification without a body in the unit is never applied;
* a recursive one is withdrawn.

A `nullable` specification of the same body is withdrawn beside W4's
`nonnull` `SummaryConflict`. That was measured, and it is why the module
declares `nonnull`.

**Hand-on of a refused call** (§2.5):
`crates/qcore-c-gateway/tests/refused_call_hand_on.rs`, 6 tests.

**Other tests:**

* `crates/qcore-contract/tests/contract_body_checked.rs`, 5 tests: the
  marker, duplicate, argument and `ensures` spellings, an environment
  without texts, and the composition with the revision and review markers;
* 6 unit tests in `legacy_list_iteration.rs`;
* `compiler/tests/qcore_contract_corpus.rs`: pins +12 declarations, corpus
  113 → 114 modules on the merged tree.

## 7. Lean

`proof/QCore/ListIteration.lean`, audited in `AuditListIteration.lean`:

* `unlinking_preserves_distinctness`, `the_saved_successor_stays_linked`
  and `safe_unlinking_visits_each_once`: safe iteration preserves the ring
  and visits every node once under unlinking the current node;
* `plain_unlinking_reads_a_cleared_link`: plain iteration does not survive
  it;
* `unlinking_the_successor_reads_a_cleared_link`: neither does safe
  iteration unlinking its successor;
* `an_admitted_body_never_reads_a_cleared_link`, `plain_keep_visits`,
  `safe_keep_visits`, `plain_unlinking_the_next_node_runs` and
  `a_refused_body_has_a_stuck_ring`: rule 4 is exact on the model;
* `element_binding_moves_no_custody`.

`proof/QCore/CheckedSpecification.lean`, audited in
`AuditCheckedSpecification.lean`:

* `settle_sound`: everything in force is established under what is in force;
* `settle_subset` and `never_without_a_body`;
* `settle_is_a_fixed_point_within_its_bound`: within the Rust's
  `S.length + 1` rounds;
* `a_row_is_safe_only_if_established`;
* two non-vacuity theorems.

`proof/QCore/RefusedCallHandOn.lean`, audited in
`AuditRefusedCallHandOn.lean`:

* §2.5 is W44's `RefusedHandOnCustody.apply` at a wider refusal signal
  (`is_the_hand_on_rule`);
* it only withdraws (`it_only_withdraws`);
* a site the call does not name keeps its leak
  (`an_unnamed_site_keeps_its_leak`);
* the named object is withdrawn (`the_named_object_is_withdrawn`);
* the node's refusal keeps the function out of `Safe`
  (`a_refused_call_is_never_safe`).

No placeholders and no project axioms. The lexical recognition, the stated
premises and the W4 gate's correspondence are Rust boundaries pinned by the
tests above.

## 8. Prediction against outcome

The prediction was frozen on the base `74579af6`, before the merges and §2.5.
Final figures, with the pre-merge ones in brackets where they differ:

| Prediction | Outcome |
|---|---|
| 200–269 of 269 list-header `for` leaves withdrawn | **235** |
| 220–326 of 326 element braced leaves withdrawn | **283** |
| 10–60 new plain-form refusals | **0**. Miss: no plain body in the corpus or the slice hands its current element to a releasing position (§3). |
| unsupported falls by several hundred, unknown_alias rises by tens | unsupported −1,377; unknown_alias −1. Miss on the sign: the declaration reader also removed 70 aliases. |
| Unknown → Safe 0–15; Safe → non-Safe 0 | **9**; **0** |
| Unknown → provisional Unsafe 0–10; leak pin +0–10 | **1**; **37 → 34** [3; 37 → 40]. Miss in direction: §2.5 withdrew the two artifacts and four standing leaves of the publication class. |
| the module moves no corpus leaf | **confirmed**, byte-identical |
| slice: no new `sb.c` Safe beyond the two | **as predicted**; every `sb.c` iteration and element refusal gone; 4 other slice functions became Safe |
| OB-1 blocked on publication; OB-2, OB-5 Unknown, not refuted; at most the two helper legs held | **as predicted** pre-merge. After the first merge, OB-1 is blocked on accounting instead (§5). |

## 9. Follow-ups (`followups.json`)

**Registered:**

* `W45-FB-BACKLOG-REFERENCE-VOCABULARY` (OB-2, OB-3): a wrapper→chunk
  reference relation, return-conditioned clauses and per-element identity;
  measured reach in this slice 0 without the last;
* `W45-FB-BACKLOG-MAP-ARMS` (OB-4, OB-5): per-arm disposition of
  `flb_input_chunk_map`;
* `W45-FB-BACKLOG-REACHABLE-RELEASE` (OB-6, OB-7, OB-10): objects released
  through what an argument reaches;
* `W45-FB-BACKLOG-CIO-RETURN-CODES` (OB-9);
* `W45-FB-BACKLOG-PUBLICATION-BLOCKED`, re-scoped after the merge. OB-1's
  link is now modeled. What stays open is the counter's indirect access
  (OB-8) and the absence of a clause stating "exactly one queue".
* `W45-FB-BACKLOG-FOREIGN-UNLINK`: rule 10d refuses an unlink of a wrapper
  another function published. That blocks OB-3's, OB-7's and OB-10's
  retirement legs in `sb.c`, and 17 corpus leaves in 14 teardown functions
  reached through this card's safe iterations.
* `W45-ILI-PREMISES`: P-ILI-UNTRACKED's lexical guard, P-ILI-CALLEE-FRAME
  and P-ILI-BRACED; residual element leaves 43 and header leaves 34;
* `W45-MK-HTTP-ERROR-FD-LEAK`: the provisional candidate, for a native or
  source discharge.

**`W45-FOR-CLAUSE-ORIGINAL-SPELLING`** is narrowed to the other headers, not
retired.

**For the integrator:**

* `MK-LIST-ADD-BORROWED-PUBLICATION`: §2.5 withdraws its four standing leaves
  (`mk_rconf_read`, `mk_vhost_create`, `mk_vhost_set_property`,
  `mk_vhost_set_single`) to Unknown, and the two it would have gained never
  appear. No leaf of the class stands. None is discharged, though: each
  function is Unknown, not Safe. Retiring the entry is the integrator's
  call; this card does not.
* `W45-FB-SINGLEPLEX-CUSTODY-DECLARED` (the retry card's custody half applied
  at callers while the bodies do not validate) could be answered by the
  `resource_body_checked` marker of §2.3.
* The retry card's `W45-FB-OR6-ELEMENT-IDENTITY` is adjacent: rule 3 binds a
  `container_of` element as untracked memory, not as an identity.

## 10. Gate

`scripts/gate.sh` runs the standing gate of `docs/agents/validation.md` and the
provider gate (constraint 18, including `providers/tests`). Each exit is
captured to its own file, `CARGO_INCREMENTAL=0`, and `cargo clean --profile
dev` runs after the test step. `scripts/pin.sh` records the pinning. Commands,
exits and counts are in [gate.json](gate.json); logs are under the
worktree-private `private/w45-fb-backlog-custody/`.

Final gate on `ab87c458` (the merged tree with §2.5): every step exited 0.

| Step | Result |
|---|---|
| submodule; `cargo fmt --check`; `cargo clippy -D warnings`; both reevaluation probes | 0 each |
| Rust suite (`scripts/run_test_binaries.py`) | 0: 305 units, 315 summaries; **5,877 passed**, 0 failed, 11 ignored |
| `cargo clean --profile dev` | 0 |
| Lean audits | 0: **201** passed |
| Root Python tests | 0: **358** tests |
| `build_docs_index.py --check` | 0 |
| Provider gate: `contracts.py check`, `package.py build` + `verify qcore.libevent`, `providers/tests`, provider `cargo test`, fmt and clippy | 0 each: 15 Python tests; 124 Rust tests passed, 0 failed |
| Pins (`scripts/pin.sh`): pin at `046accf6` and `66910c10`, `pin --check` at both, `pin --verify-contracts` | 0 each |

**Discarded, recorded in `gate.json`:**

* `c63f1a08`: the first version of §2.5 withdrew `borrow_residuals_boundary`'s
  pinned leak, and the rule was narrowed.
* The first attempt at `9e76fc64`: a private export sat inside the worktree.

**Earlier green gates:** `9e76fc64` (first merge) and `316cf766` (the card's
own base).

## Reproduce

```sh
docs/reports/w45-fb-backlog-custody/scripts/prepare-attempt.sh attempt-N BINARY 046accf6aded7906f66a1baf0202da349e521736
python3 docs/reports/w45-fb-backlog-custody/scripts/pin_check.py OUT/attempt-N 046accf6aded7906f66a1baf0202da349e521736 pin.json
docs/reports/w45-fb-backlog-custody/scripts/run-current.sh attempt-N RUN BINARY --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
python3 docs/reports/w45-fb-backlog-custody/scripts/ob_table.py OUT/attempt-N/RUN/resource-diagnostics.json BEFORE_REPORT ob-table.json
python3 docs/reports/w43-return-expression/scripts/run_audit.py FRESH_DIR COMMIT --jobs 8
python3 docs/reports/w45-fb-backlog-custody/scripts/arbiter_compare.py AFTER BEFORE OUT.json --source-root PINNED_SOURCE
python3 docs/reports/w45-fb-backlog-custody/scripts/decompose.py declarations-only FRESH_DIR PRIVATE_TARGET
python3 docs/reports/w45-fb-backlog-custody/scripts/without_module.py FRESH_DIR
```
