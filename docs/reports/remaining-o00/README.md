# O00 — Source contracts needed for caller/constructor provenance — 2026-09-07

An obligation map for **one** concrete constructor-to-caller path in the pinned
185-unit Cycle 19 corpus, separating the **trusted** live / exclusive /
ownership / cleanup premises from what is actually derivable at the selected
call sites.

This card is **discovery and report-only**. It promotes no verdict, asserts no
upstream defect, and changes no analysis, checker, contract, profile, test or
pinned artifact. The machine-readable ledger is
[`premises.json`](premises.json).

## Verdicts carried forward unchanged

| Item | Value | Changed by O00 |
|---|---|---|
| `flb_input_chunk.c` raw unit | **provisional Unsafe** (leak 10, unknown_alias 109, unsupported 946) | no |
| `cio_chunk.c` raw unit | **Unknown** | no |
| `cio_scan.c` raw unit | **Unknown** | no |
| `cio_file.c` raw unit | **provisional Unsafe** | no |
| Corpus aggregate | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | no |
| CT coverage | 11/50 mapped | no |
| Contract corpus | 59 modules, 19 loaded `resource_contract_v1` declarations | no |

Diagnostics are read from the hash-verified archived artifact
`docs/reports/w2-clang-cmake-build/diagnostics.json.gz` (pinned Fluent Bit
`66910c1…`), **not regenerated**. Its line numbers are Clang `-E -P`
preprocessed coordinates; every mapping onto original source lines below was
reproduced locally from the archived `compile_commands.json.gz` and is recorded
in [`logs/line_anchors.txt`](logs/line_anchors.txt).

## 1. The selected path

**Constructor:** `cio_chunk_open`
(`lib/chunkio/src/cio_chunk.c:30-109`, archived status **Unknown**, 33 diagnostics).

**Caller:** `flb_input_chunk_create`
(`src/flb_input_chunk.c:2506-2621`, archived status **Unknown**, 44 diagnostics).

**Deferred caller cleanup:** `flb_input_chunk_destroy`
(`src/flb_input_chunk.c:2704`, release sites 2797–2805).

**Counter-example caller (discard):** `cio_scan_stream_files`
(`lib/chunkio/src/cio_scan.c:105`).

`cio_chunk_open` has exactly **two production call sites inside the 185-unit
corpus** — these two. (`src/flb_storage.c:950` and `src/flb_fstore.c:176` also
call it but are not corpus units; the remaining hits are tests and tools. Full
enumeration: [`logs/call_sites.txt`](logs/call_sites.txt).) One keeps the
result and cleans up on four arms; the other discards it. Between them they
cover successful construction, every partial-failure rollback arm, caller
discard and caller cleanup, which is why this path was chosen over
`cmt_counter_create`: the CMetrics constructor has no descriptor or mapping
identity at all, so it cannot exercise the identity separation this card exists
to measure.

## 2. Every conditional acquisition/teardown exit, from the pinned source

### 2.1 Constructor `cio_chunk_open`

| Exit | Kind | Source line | pp line | Condition | Released on the exit | Live at the exit |
|---|---|---|---|---|---|---|
| E1 | guard | 40 | 2018 | `!st` | — | — |
| E2 | guard | 45 | 2022 | `!name` | — | — |
| E3 | guard | 51 | 2027 | `strlen(name) == 0` | — | — |
| E3b | guard, **compiled out** | 56 | — | unsupported backend | — | — |
| E4 | allocation failure | 64 | 2032 | `!ch` | — | — |
| E5 | **partial-failure rollback** | 92 | 2055 | `!backend` | `_head` unlink (89), chunk name (90), chunk object (91) | — |
| E6 | **success** | 108 | 2065 | `backend != NULL` | — | chunk object, chunk name, both list links, backend child, backend path, incremented counter |

E3b is absent from the pinned preprocessed unit (`CIO_HAVE_BACKEND_FILESYSTEM`
is defined); it is the arm the four native executions in
`docs/reports/cycle19-unsupported-constructors/` observe.

E6 has three sub-cases that differ **only** in the descriptor and mapping
identities:

| Sub-case | Source | Descriptor | Mapping |
|---|---|---|---|
| E6a filesystem, resource-limit arm | `cio_file.c:702-714` | **not acquired** (`fd == -1`) | **not acquired** (`map == NULL`) |
| E6b filesystem, full open | `cio_file.c:718-781` | acquired | acquired (`total_chunks_up` incremented inside `mmap_file`) |
| E6c memory backend | `cio_chunk.c:83-86` | n/a | n/a |

**E6a is the reason the caller exists in the shape it has.** A successful
construction can hand back a live chunk with no descriptor and no mapping,
which is what `cio_chunk_is_up` / `cio_chunk_up_force` / `set_down` at
`flb_input_chunk.c:2536-2543` and `cio_chunk_down` at 2602 are for.

The child constructor's own arms feed E5 (all of them free their own
allocations before returning `NULL`):

| `cio_file_open` arm | Released there |
|---|---|
| 654 filename check | — |
| 659 path compose | — |
| 668 `calloc` failed | path |
| 726 native open failed | path, `cf` |
| 740 size update failed | **descriptor**, path, `cf` |
| 763 acl/settings failed | **descriptor**, path, `cf` |
| 776 `mmap_file` failed | **descriptor**, path, `cf` |

### 2.2 Caller `flb_input_chunk_create`

| Exit | Kind | Line | Condition | Released on the exit | Escapes |
|---|---|---|---|---|---|
| C1 | constructor returned NULL | 2530 | `!chunk` | — | — |
| C2 | **caller cleanup** | 2541 | `up_force == -1` | chunk, via `cio_chunk_close(chunk, CIO_TRUE)` at 2540 | — |
| C3 | **caller cleanup** | 2550 | header write `== -1` | chunk, via close at 2549 | — |
| C4 | **caller cleanup** | 2558 | `ic` calloc NULL | chunk, via close at 2557 | — |
| C5 | **caller cleanup** | 2587 | `routes_mask` calloc NULL | chunk, via close at 2585; `ic` via `flb_free` at 2586 | — |
| C6 | **success + publication** | 2620 | — | descriptor and mapping via `cio_chunk_down` at 2602, **only if** `set_down` | `ic` |
| C7 | deferred cleanup | 2797-2805 (`flb_input_chunk_destroy`) | — | chunk, list link, `routes_mask`, `ic` | — |
| C8 | **not enumerated by the caller** | falls through 2543 | `up_force` returned `CIO_RETRY` (-2) or `CIO_CORRUPTED` (-3) | — | continues into C6 with `set_down = FLB_TRUE` |

C8 is a source observation, not a defect claim: the caller tests `ret == -1`
only (2539) while `chunkio.h:50-53` defines three distinct negative results.
Any exit enumeration that omits it is incomplete.

C6 makes four separate publications of `ic`: the aggregate store `ic->chunk =
chunk` (2569), the instance list (2599), the msgpack callback (2598), and one
of four hash tables (2606/2610/2614/2617).

### 2.3 Discard caller `cio_scan_stream_files`

`cio_chunk_open(ctx, st, ent->d_name, ctx->options.flags, 0, &err);`
(`cio_scan.c:105`) discards the returned pointer entirely. The pinned C is
correct because the constructor already linked the chunk into `st->chunks`
(`cio_chunk.c:75`) and `cio_stream_destroy` releases it later — but **that
transfer appears in no contract, profile or model in this repository.**

## 3. Identity map — which identity is created and released where

Twelve identities are tracked separately in [`premises.json`](premises.json).
The load-bearing separation:

| Identity | Created | Released |
|---|---|---|
| **ID-P** chunk object | `cio_chunk.c:61` (`malloc`) | E5 rollback (91); C2–C5 close; C7 close |
| **ID-N** chunk name | `cio_chunk.c:66` (`strdup`, **NULL never tested**) | E5 rollback (90); inside `cio_chunk_close` |
| **ID-L** stream containment | `_head` at 75, `_state_head` at 102/105 | `mk_list_del` at 89 (rollback, `_head` only) and inside close |
| **ID-B** backend child | `cio_file.c:663` / `cio_memfs_open` | the child's own arms; `cio_file_close:990` |
| **ID-Q** backend path | `cio_file.c:657` | every child arm; `cio_file_close:989` |
| **ID-D** descriptor | `cio_file_native_open` (`cio_file.c:718` or `_cio_file_up:845`) | child arms 740/763/776; `_cio_file_up:879` on CORRUPTED/RETRY; `cio_file_down:951`; `cio_file_close:976` |
| **ID-M** mapping | `mmap_file` → `cio_file_native_map` | `munmap_file`; `cio_file_close:973` |
| **ID-C** counters | `cio_chunk.c:98`, `mmap_file` up-add | close, `munmap_file` |
| **ID-IC** caller object | `flb_input_chunk.c:2554` | C5 (2586); C7 (2805) |
| **ID-RM** routes_mask | `flb_input_chunk.c:2580` | C7 (2801) |
| **ID-PB** publications | 2569, 2598, 2599, 2606/2610/2614/2617 | — |
| **ID-E** `*err` out slot | `cio_chunk.c:84`, child `*err` writes | scalar, no custody |

Three asymmetries matter and none of them is modeled today:

1. **ID-D and ID-M are acquired conditionally at construction (E6a vs E6b) and
   again by the caller's own `up_force`.** The declared constructor contract
   says nothing about either.
2. **ID-B is published into `ch->backend` at `cio_file.c:690` before every one
   of the child's failure arms**, which free `cf` without clearing the field.
   That is only safe because `cio_chunk.c:91` frees the parent on the same arm
   — a cross-function invariant no pass states.
3. **`_state_head` is linked only on E6**, which is why E5's single
   `mk_list_del` is complete and `cio_chunk_close`'s two are not redundant.

## 4. How the three current routes treat this path

### 4.1 The declared `.qcore` route — assumed, and mostly inert

`contract/qcore/chunkio/resource_open_close.qcore` declares both
`cio_chunk_open` (`resource_return_owned(nullable)`, everything else borrowed)
and `cio_chunk_close` (`resource_released(chunk, nonnull)`). Both are among the
19 loaded `requires resource_contract_v1` declarations.

The constructor contract **never reaches either corpus call site.** In
`crates/qcore-c-gateway/src/legacy_c.rs` the `unsupported_access` check
(1618-1637, using `contains_unmodeled_access` at 1798) runs *before* `find_call`
(1640) and before contract lookup (1650). Both call statements read an argument
through `->`:

- `flb_input_chunk.c:2525` — `storage->cio`, `storage->stream` → archived
  diagnostic *"indirect memory access is outside the resource projection"*.
- `cio_scan.c:105` — `ctx->options.flags` → same diagnostic.

The destructor contract fares better but not well: of the **9** corpus call
sites of `cio_chunk_close`, **7** are syntactically direct and **2** are inert
(`cio_chunk_close(ic->chunk, del)` at 2690 and 2797). At the 7 direct sites the
contract does fire — onto an *unresolved* identity, because `chunk` was never
bound at the constructor call. Archived: `unknown_alias: chunk` at the
preprocessed lines corresponding to 2540, 2549, 2557 and 2585.

The richer premise vocabulary — `chunk_is_live`, `chunk_is_not_busy`,
`chunk_has_no_task_or_route_custody` (`chunk_destructor.qcore:16-18`),
`entry_is_live` / `exclusive_link_access` / `entry_has_live_reciprocal_neighbors`
(`list_unlink.qcore:19-22`), `counter_is_live` / `exclusive_counter_access` /
`c_unsigned_matches_usize` (`counters.qcore:11-16`) — carries no
`requires resource_contract_v1` marker and is therefore **not loaded** by the
resource loader. Those are exactly the trusted live/exclusive premises this
card was asked to separate out, and they are assumed and unenforced.

`mk_list_add` has **no declaration anywhere** in `contract/qcore`.

### 4.2 The W4 summary route — structurally excluded

`crates/qcore-c-gateway/src/legacy_summary.rs:21-23` publishes a summary only
when the body re-analyzes `Safe` with **zero** diagnostics. `cio_chunk_open`
has 33, `cio_chunk_close` 10, `flb_input_chunk_create` 44. No function on this
path can ever be summarised by the current pass. This is the gate working as
designed, not a defect.

### 4.3 The source-contract dispatch route — out of profile

`crates/qcore-c-gateway/src/boolean_source.rs:716` rejects with *"outside single
borrowed chunk / int-or-void-result source profile"*. `cio_chunk_open` and
`flb_input_chunk_create` both return pointers, so neither is expressible.

The evidence-v2 lifecycle inventory does contain `cio_chunk_open` →
`ApiShape::OpenChunk` (`crates/qcore-verification/src/obligation_derivation.rs:328`),
but that derivation runs over a `qcore_frontend::frontend::SourceProgram` — a
QCore-language model — **not** over the pinned C translation unit. It is a
model-only route with respect to this path.

The typed-source chunk-contract layer
(`docs/reports/w2-clang-cmake-build/chunk-contracts.json.gz`) covers `cio_destroy`,
`cio_chunk_close_stream`, `cio_file_close`, `cio_stream_destroy` and friends —
i.e. the **teardown** side. There is no constructor entry. Constructor evidence
exists only as fixed native injected runs
(`cycle19-constructor-failures`, `cycle19-file-constructor-failures`,
`cycle19-unsupported-constructors`), taken at a harness caller.

## 5. Probes were executed, not asserted

Every claim in §4 was checked through the real gateway entry point
`qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit`, loading the real
`contract/qcore/{libc/memory, fluent_bit/resource_memory,
chunkio/resource_open_close}.qcore`, from a scratchpad-only probe crate. No
repository file was modified. Full output:
[`logs/gateway_probe.log`](logs/gateway_probe.log); case sources:
[`logs/probe_cases.rs.txt`](logs/probe_cases.rs.txt).

| Case | Shape | Result |
|---|---|---|
| O-A | caller with member-read constructor arguments | **Unknown** — indirect access at the call; `unknown_alias: chunk` at the close |
| O-B | *identical control*, plain-local arguments | **Safe** |
| O-C | discard with plain-local arguments | **Unknown** — *"discarded owned result of resource API `cio_chunk_open`"* |
| O-D | pinned discard shape, one member-read argument | **Unknown** — indirect access only; the discard is **masked** |
| O-E | `cio_chunk_close(ic->chunk, del)` | **Unknown** — the destructor contract never applies |
| O-F | four-arm cleanup, plain locals + borrowed contracts for the intervening callees | **Safe** |
| O-G | success exit with `ic->chunk = chunk` | **Unknown** — indirect access; `unknown_alias: ic` |
| O-H | same success exit with an explicit `resource_stored` transfer | **Safe** |

O-A/O-B is the load-bearing pair: a single member-read argument is the *entire*
difference between the constructor contract being inert and the cleanup arm
discharging. O-C/O-D is the second: the declared owned-result mode, if it ever
reached the scan loop, would report a discarded owned result there. O-G/O-H
shows the success exit needs a `Stored` transfer that C02's scalar whitelist
cannot supply.

O-F and O-H additionally load a **scratchpad-only** hypothetical `.qcore`
module (reproduced at the end of the probe log). No repository contract was
added or edited.

## 6. The premise ledger

38 rows in [`premises.json`](premises.json), each with its identity, exact
source site, origin and status.

| Status | Rows |
|---|---:|
| **assumed** (trusted premise, reaches its site) | 13 |
| **assumed_inert** (declared but never reaches any corpus call site) | 2 |
| **checked_bounded** (fixed native observation at a harness caller only) | 4 |
| **not_modeled** | 19 |

| Origin | Rows |
|---|---:|
| `derivable_from_source` | 13 |
| `declared_contract_v2_unloaded` (premise vocabulary present, loader rejects) | 8 |
| `declared_contract_v1` (loaded) | 6 |
| `native_observation` | 4 |
| `absent` (no declaration anywhere) | 4 |
| `profile_trust` | 2 |
| `model_only` | 1 |

**Assumed vs derivable.** 15 rows are trusted premises the analysis takes on
faith; 4 are checked only by fixed injected native runs at a harness caller;
19 are not modeled at all. Of the 19, **11 are derivable at the selected call
sites behind the named features in §8** and **8 require a premise that exists
nowhere today**. Counting by derivability across all 38 rows: **16 derivable
with a named feature, 22 not derivable without a new premise.** Seven rows also
carry a bounded native observation leg.

Representative rows:

| Row | Premise | Origin | Status |
|---|---|---|---|
| R01 | the constructor hands the caller owned custody of a nullable chunk | declared v1 | **assumed, inert** |
| R03 | the returned chunk is *also* linked into `st->chunks` | derivable | **not modeled** |
| R07 | `chunk_has_no_task_or_route_custody` at close | declared v2, unloaded | **assumed** |
| R11 | the rollback arm frees name then object, exactly once each | native observation | **checked, bounded** |
| R13 | `mk_list_add` transfers containment and conserves live resources | **absent** | **not modeled** |
| R16 | `ch->backend` is published before the child's failure arms | derivable | **not modeled** |
| R19 | a successful construction may yield no descriptor and no mapping | derivable | **not modeled** |
| R22 | the caller tests only `ret == -1` | derivable | **not modeled** |
| R30 | `ic->chunk = chunk` transfers chunk custody into the caller's aggregate | derivable | **not modeled** |
| R38 | W4 can summarise any function on this path | derivable | **not modeled** (zero-diagnostic gate) |

## 7. Named blockers

Unresolved aliases and callbacks stay named, with exact locations.

| # | Blocker | Exact site | Kind |
|---|---|---|---|
| **B01** | the constructor call statement is rejected before the contract is consulted | `legacy_c.rs:1618-1637` before `find_call` at 1640 and contract lookup at 1650, hitting `flb_input_chunk.c:2525` and `cio_scan.c:105` | gate |
| **B02** | constructor publication into `st->chunks` is unmodeled and in tension with the declared owned result | `cio_chunk.c:75` | contract soundness |
| **B03** | `mk_list_add` has no declaration in `contract/qcore` | `cio_chunk.c:75/102/105`, `flb_input_chunk.c:2599` | missing contract |
| **B04** | `ch->backend` published before the child's own failure arms and left dangling by them | `cio_file.c:690` | cross-function invariant |
| **B05** | a successful construction may yield no descriptor and no mapping | `cio_file.c:702-714` | conditional acquisition |
| **B06** | `CIO_RETRY` / `CIO_CORRUPTED` reach the success path unenumerated | `flb_input_chunk.c:2539` | incomplete exit enumeration |
| **B07** | the success exit's `cio_chunk_down` releases identities whose acquisition is unmodeled | `flb_input_chunk.c:2602` | conditional teardown |
| **B08** | **unresolved alias** — pointer-field custody transfer `ic->chunk = chunk` | `flb_input_chunk.c:2569` | alias |
| **B09** | **unresolved callback** — `msgpack_packer_init(&ic->mp_pck, ic, flb_input_chunk_write)` retains `ic` | `flb_input_chunk.c:2598` | callback |
| **B10** | **unresolved alias** — `flb_hash_table_add(..., ic, 0)` under four alternative event types | `flb_input_chunk.c:2606, 2610, 2614, 2617` | alias |
| **B11** | the chunk-lifecycle obligation inventory is derived over QCore source, not over this C | `obligation_derivation.rs:325-339` | architectural |

## 8. O04 recommendation

**Single bounded path: per-arm constructor-to-caller cleanup discharge on the
four early-return arms of `flb_input_chunk_create` — exits C2, C3, C4, C5.**

**Property.** On each of those arms, the chunk identity produced at
`flb_input_chunk.c:2525` is released exactly once, by the `cio_chunk_close` on
that arm, and no other tracked identity is released or left live.

**Explicitly out of scope for O04:** the success exit C6 and its four
publications, the unenumerated exit C8, the descriptor and mapping identities
ID-D/ID-M, `cio_scan_stream_files`, and any unit promotion.

**Prerequisite premises** (carried explicitly, as assumptions): R01, R02, R04,
R05, R06, R07, R09, R12, R29 — i.e. the two loaded chunkio contracts, the
unloaded live/exclusive/no-other-custody premises of `chunk_destructor.qcore`,
and the libc/Fluent Bit allocator contracts.

**Prerequisite features**, in dependency order:

| # | Feature | Why | Depends on |
|---|---|---|---|
| P1 | **C02-P** — admit a direct pointer-field read *only* as a `Borrowed` actual of a contracted call | without it the constructor call is rejected and `chunk` never binds (B01) | **C02, extended** |
| P2 | array-declarator admission for `char name[64]` | `flb_input_chunk.c:2514` is rejected by the same token gate | C02 |
| P3 | borrowed declarations or W4 summaries for `generate_chunk_name`, `cio_chunk_is_up`, `cio_chunk_up_force`, `input_chunk_write_header`, `flb_log_check`, `flb_log_print`, `flb_errno_print` | measured: O-F only reaches Safe once the intervening callees carry borrowed contracts | new `.qcore` declarations |
| P4 | per-exit disposition, so an unmodeled success exit does not poison the four cleanup arms | the published status is per function today; C6's publications (B08–B10) would otherwise force Unknown | analyzer change |

**Card dependencies.**

- **C02 is necessary but not sufficient.** Its whitelist admits only non-pointer
  arithmetic/enum fields in statements containing no call — which excludes both
  the constructor call's pointer-field actuals (P1) and `ic->chunk = chunk`
  (B08). O04 needs C02 *plus* the P1 extension.
- **E-series evidence hardening.** R19–R26 — the whole descriptor/mapping story
  — are bounded native observations taken at a harness caller
  (`cycle19-reopen-failures`, `cycle19-file-descriptor-failures`,
  `cycle19-file-unmap-failures`). O04 must either import them as explicitly
  named assumptions and scope its property to ID-P only, or wait for the
  E-series to bind them to a production caller. The honest choice is the former,
  with the assumption list published on the result.
  Note in particular that `cio_chunk_up_force` is `Borrowed` only with respect
  to ID-P; it mutates ID-D and ID-M.
- **W4 (`legacy_summary.rs`) cannot contribute.** No function on this path can
  pass its zero-diagnostic publication gate.

**Precision warning.** Lifting the indirect-access gate on the constructor call
also unmasks `cio_scan_stream_files`, which would then report a *discarded owned
result* (probe O-C) and stay **Unknown**. Resolving that properly requires B02
— a publication/transfer model for `mk_list_add(&ch->_head, &st->chunks)`, or a
revised `cio_chunk_open` result mode. Either way all four archived 185-unit
artifacts change and must be regenerated, diffed and re-pinned deliberately
under hard invariants 3 and 5.

## 9. Scope and non-claims

- **No raw unit is promoted.** All four units on this path keep their archived
  statuses; the corpus aggregate is unchanged.
- A provisional Unsafe is **not** an upstream defect claim. Nothing in §2 is
  presented as a bug in Fluent Bit or ChunkIO; C8, B04 and R10 are enumerated
  source facts an analysis must account for, and the pinned C releases through
  paths the analysis simply cannot follow.
- Every contract named "assumed" is a **trusted declaration**, not a validated
  claim about the C body. The `declared_contract_v2_unloaded` rows are not even
  in force at analysis time.
- The Cycle 19 native observations cited are **fixed injected runs at a harness
  caller**. They establish neither production caller reachability, nor arbitrary
  OS failure behaviour, nor durability.
- The recommended features are **candidates**. Nothing here establishes that any
  of them is implementable at its stated boundary.
- No gate was weakened, no pinned expectation edited, no diagnostic downgraded.
- No CT mapping changes; coverage remains 11/50 mapped.

## 10. Files written

Only these paths. Nothing was committed, pushed, reset or cleaned, and no
tracked file was modified.

- `docs/reports/remaining-o00/README.md` (this file)
- `docs/reports/remaining-o00/premises.json`
- `docs/reports/remaining-o00/logs/gateway_probe.log`
- `docs/reports/remaining-o00/logs/probe_cases.rs.txt`
- `docs/reports/remaining-o00/logs/path_diagnostics.txt`
- `docs/reports/remaining-o00/logs/call_sites.txt`
- `docs/reports/remaining-o00/logs/line_anchors.txt`
- Session scratchpad (outside the repository): probe crate, hypothetical
  contract module, locally reproduced preprocessed units.
