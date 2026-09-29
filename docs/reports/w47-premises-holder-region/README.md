# W47-PREMISES-HOLDER-REGION: the region's macro calls and the holder's opacity derived

Card W47-PREMISES-HOLDER-REGION (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `fe603d49` (the worktree's HEAD). The frozen
prediction is [PREDICTION.md](PREDICTION.md), commit `e8d73983`, made after
BEFORE was measured and before any AFTER census, facts derivation, fixture
run or Lean change (a first draft of the two modules existed, compiled and
never run; disclosed there).

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement has the form
*discharged / not discharged / unresolved at this path under these derived
premises*. The Fluent Bit implementation and test are unchanged; no contract,
provider or pin changes.

## 1. Result

Two premises the fixed test's discharge rested on are now derived from the
pinned source, fail-closed:

- **The region-macro premise** (`W47-LOOP-OWNER-REGION-MACRO-CALLS`): every
  macro invocation on the list-head region is read from the expansion its own
  unit's preprocessor produced (206 invocations of 43 macros at the export),
  and the census graph the region reaches over gains every function's
  macro-made calls (16,741). None names, carries or reaches a hazard.
- **P-HOLDER-OPAQUE**: every read of `struct flb_input_instance::data` (7) is
  followed as the holder's pointer; it reaches 5 carriers, none writing
  through it outside the test's file.

At the `90978ac6` export's configured reading the fixed test's three sites
(`cb_tail_pause_test`, `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) stay
**discharged at this path under the derived premises of §6** on
`(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`. At
`90978ac6~1` the old test (the negative control) stays **not discharged**:
all 8 of its sites keep their resolved test-thread context `T Either`. No
site moves at either revision in any reading; the corpus, system slice and
tail-pause slice are unchanged (§5).

## 2. Mechanism

**Region-macro facts** (`loop_owner_region_macros.rs`, child of
`thread_entry_census.rs`; the `region_macro_facts` example). Each selected
compile command (every command of the region's library files, and the test
as an extra unit borrowing `src/flb_lib.c`'s command with the configured test
header) runs with its own recorded compiler and flags under
`-fsyntax-only -Xclang -dump-tokens` (with `-w`). Every dumped token carries
its expansion location and, when a macro produced it, its spelling location.
Tokens whose expansion location is in a census file the unit read are
grouped by that location (the outermost invocation): each token is an
argument's (spelled in the same file after the invocation's name) or a
replacement list's. File-scope function definitions are read from the same
stream; a definition outside the census's files is kept with its body's
names (the test's `acutest_check_` and `acutest_message_` of
`tests/lib/acutest/acutest.h`, closed over their callees). An entry spelled
across a line splice (`[UnClean='…']`) spans dump lines and is joined; any
line the reader cannot read fails the unit. The compiler identity is
recorded as the predefined facts record it, and the census view decides
nothing unless the database's SHA-256, the recorded `--version` and every
command of a derived file match.

**Step B's region.** For every region part (the test up to the list-head
read, the maker, the constructors read in place with their null-returning
blocks, a keyed setter's two bodies without the arms no key selects), every
invocation on its lines (a line wholly inside a skipped range excepted) is
read from each unit that read the part's file (its own `.c`, or a header the
unit included; the text's SHA-256 must be the facts'):

- a replacement list's `return` or `goto`, a `break`/`continue` without its
  own loop, a field dispatch, or a listed member (`inputs`, `in_plugins`, a
  written `config`) refuses;
- a replacement-list name is resolved **in the unit**: its definition in a
  read file is the census's definition there (none: refused); one outside the
  census is read through its body's names (typed-read and without an indirect
  call under the source root, else an unmodelled call; library code outside
  the root is rule 10p's premise); any other name is an external definition
  of the read files (a name only an unstructured file may define refuses);
- an argument name is resolved as the lexical region resolves it;
- a resolved function that reaches a hazard refuses (a call a replacement
  list makes; one it carries may be dropped, repeated or looped);
- the region's own functions' typed indirect calls (spelled in them or in a
  replacement list) are unmodelled calls.

**The census graph's macro-made calls** (`closure_macro_calls`). Every
function's invocations become edges: from the units' expansions for the 145
census files they read; elsewhere every read file's `#define` of the invoked
macro in every arm (a target no compiled unit defines dropped: a `.c` file
the typed reading read without failure whose AST lacks it) and every
registrant of a field its replacement list calls as a member. A root of the
region that reaches a hazard only over these edges refuses, with a witness
path; the widened closure and reach feed the unmodelled-call blocker. The
first draft over-approximated the member calls by every word of a
replacement list and refused the export spuriously
(`set_log_level_from_env > flb_env_get > (macro) flb_log_print > … >
flb_log_enqueue_signal > (macro) mk_tls_write`: the word `write` of the
`write(2)` call in `flb_pipe_w` taken for a field dispatch); the member
calls are now read from the replacement list's tokens.

**P-HOLDER-OPAQUE** (`loop_owner_holder_opaque.rs`). Every read of the
instance's `data` the typed reading records (and, in a function without a
typed reading, every `->data`/`.data` but a member of an object the function
declares with another record type) starts the holder's pointer:

- compared or tested, discarded (`(void) data;`), copied into a local (a
  carrier), or passed whole to a read function's parameter or through a field
  dispatch to its registrants' parameters (typed-keyed, else field-keyed);
- the dispatch of the instance's own callback (`X->p->F(…, X->data)`, or
  `Q->F` after `Q = X->p`, `X` not reassigned between) reaches only the
  copy's `F`: the installing function's store into the copy's `F`, or the
  creating descriptors' designated initializer (every element designated).
  Whenever `X->data` is the holder, `X->p` is the copy: both are written only
  by the constructor (never with the holder, which escapes only through the
  installation) and the installation, which writes both;
- a carrier may be compared, read through (`v->m`, a dispatch through it) or
  passed on; in the holder's file a member other than the copy may also be
  written or have its address taken;
- anything else leaves the flag underived: a write, an address or a
  dereference through a carrier outside the holder's file, the copy written,
  a carrier returned, stored in a field or passed to a function this reading
  does not read (`memcpy`, `memset`).

It runs after the instance's creation is derived (the literal names the
creating descriptors) and before the flag is joined.

## 3. The derived facts at the export

[`region_facts.py`](scripts/region_facts.py): 18 units per reading, none
failed: the test and the 17 compile commands of 9 library files
(`src/flb_lib.c`, `src/flb_input.c`, `src/flb_config.c`,
`src/config_format/flb_config_format.c`, `src/flb_env.c`,
`src/flb_hash_table.c`, `src/flb_router.c`, `src/flb_sds.c`, each compiled
twice, and `lib/monkey/mk_core/mk_event.c`); the units read 145 census files
(8,240 invocations recorded at the fixed reading, 8,236 at the previous). The file list is the census's own: a region
part in a file no derived unit read refuses and names every such file; the
first runs named `include/fluent-bit/flb_mem.h` (a header part: now read from
the units that include it) and the six library files the context
constructor's null-returning callees live in.

**The region.** 206 invocations of 43 macros; the test's `TEST_CHECK` and
`TEST_MSG` resolve to `tests/lib/acutest/acutest.h`'s `acutest_check_` and
`acutest_message_` (typed-read, no indirect call; their callees
`acutest_colored_printf_`, `acutest_finish_test_line_`,
`acutest_line_indent_`, `acutest_timer_*` likewise), never `lib/monkey`'s;
`flb_errno` → `flb_errno_print`, `flb_error`/`flb_info` → `flb_log_check`, `flb_log_print`;
no invocation names or carries a hazard-reaching function, none jumps or
dispatches, and no region function makes an indirect call.

**The closure.** 16,741 macro-made calls added; no root of the region
reaches a hazard through one.

**The holder.** 7 reads of `data`: `flb_input_instance_init`
(`src/flb_input.c:2019`) and `input_thread` (`src/flb_input_thread.c:369`)
pass it to `p->cb_init` after `p = ins->p`, so the holder reaches only the
copy's `cb_init`, `in_tail_init` (`plugins/in_tail/tail.c:870`), which
leaves its `data` parameter alone; `cb_emitter_init` and `cb_emitter_pause`
(`plugins/in_emitter/emitter.c:430`, `:511–512`) compare it and read
through it; the test's two callbacks write only the holder's non-copy
members. 5 carriers in 5 functions; 5 untyped `data` members belong to
objects declared with another record type (`dirent.h`'s `_WDIR`). Without
the narrowing the pointer reached every input plugin's init, and
`in_storage_backlog`'s `cb_sb_init` stores its `data` in its context
(`plugins/in_storage_backlog/sb.c:756`): a flow of the storage backlog's own
instance, not the holder's.

## 4. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_holder_region.rs` (20 tests,
W47-LOOP-OWNER-INSTANCE-FLOW's Fluent Bit shapes compiled by the real Clang):

| Control | Outcome |
|---|---|
| a region macro expanding to `flb_input(…, "coro", …)` | **refused** with the facts; *discharged* without them (the premise this card derives) |
| a creation passed through `TWICE(x) (x); (x)` | **refused** ("carries"); discharged without the facts |
| `if (!(c)) return NULL` in a macro; a field dispatch in a macro | **refused** |
| a macro calling a unit-local function outside the census that creates | **refused** |
| two headers defining `CHECK`: the unit includes the harmless one | **discharged**; the unit including the creating one: **refused** |
| a region file without facts; facts of another text | **refused** |
| a closure function (no facts for its file) creating through a macro; the same with its unit's facts | **refused** both; discharged without the facts |
| a closure function dispatching `->cb_init` in a macro to a creating registrant | **refused**, witness `(macro) …in_make_init` |
| a `->data` reader that writes `runs_in_coroutine` | **route kept** |
| a `->data` reader writing a holder byte through another record type | **route kept** |
| the holder instance's `cb_init` (a second descriptor named `TAIL`/`tail`) passing `data` to `memset`, or storing it | **route kept** |
| another plugin's `cb_init` misusing its `data` | **discharged** (not the holder instance's callback) |
| an init and a reader that only compare, discard and read through | **discharged**, P-HOLDER-OPAQUE derived |
| the old test's shape with the facts | **not discharged** (`T`) |
| the token dump: nested, pasted and multi-line invocations; spliced spellings | read as replacement list or argument |

The 22 instance-flow, 12 property-key and 16 predefined-macro tests pass
unchanged (their fixed-shape reasons gain the P-HOLDER-OPAQUE note).

## 5. Census, corpus, system and slice

**BEFORE** (base binaries `loop_owner_census` `62e74e04…`,
`typed_dispatch_facts` `4b7655a4…`, `predefined_macro_facts` `9923a0cc…`,
facts regenerated) reproduced the W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS
archive byte for byte: `719a7d0d…`, `983c74b9…`, `4955a12d…` (fixed),
`418a9251…`, `d05ebc0e…`, `a5edf1ed…` (previous); predefined facts
`9dd8bb73…`.

**AFTER** ([`census.py`](scripts/census.py), eight readings; binaries and
hashes in [artifacts/census/](artifacts/census/)); attribution by
[`compare.py`](scripts/compare.py) ([artifacts/census/compare.txt](artifacts/census/compare.txt)):

| Reading | sites discharged / not / unresolved | edges / functions / seeds | against BEFORE |
|---|---|---|---|
| `90978ac6` configured, both facts | **11 / 130 / 7** | 33,529 / 11,932 / 2,852 | equal but the flag's reason text |
| `90978ac6` configured, predefined facts only | 11 / 130 / 7 | same | equal but the reason text |
| `90978ac6` configured control, raw | 8 / 130 / 10, 8 / 126 / 10 | unchanged | equal but the reason text |
| `~1` configured, both facts | **8 / 136 / 7** | 33,503 / 11,930 / 2,852 | equal but the `region_macros` report key |
| `~1` configured predefined, control, raw | unchanged | unchanged | **byte-identical** |

No site's verdict or first reason moves; the census graph's edges,
functions and seeds are unchanged (the macro-made calls widen only Step B's
reach, not the published graph). The fixed readings' only difference is the
instance flag's reason: the holder note now carries P-HOLDER-OPAQUE's
derivation, and with the facts the two region notes. No Safe → non-Safe
move (the census publishes none), no correction.

**Arbiters** ([`arbiters.py`](../w47-loop-owner-typed-dispatch/scripts/arbiters.py),
`--jobs 8`, compared with the round-7 archive by
[`identity.py`](../w47-loop-owner-predefined-macro-arms/scripts/identity.py)):
run once, AFTER only, at `cd21bfbf` (binary `quirin-audit` in
[artifacts/arbiters/binary.sha256](artifacts/arbiters/binary.sha256)), every
exit 0. The corpus `resource-diagnostics.json` is **byte-identical to the
archive** (`ae17152b…`); the system and slice `resource-diagnostics.json`
differ only in the lines that carry the checkout root (45 and 48 lines after
the root alias, as W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS found for its own
BEFORE); every contracts sidecar is identical with the root alias. Composed
with the archive: corpus 526 / 10 / 4,298, system 31 / 2 / 201, slice 24 / 1
/ 244 / 1 Pending, 0 mismatches each ([artifacts/arbiters/](artifacts/arbiters/)).
No BEFORE arbiter run was made: the archive is the baseline the predecessor
reproduced at this base, and nothing on an audit route changed (the census
modules are read by the census examples alone). No new baseline is
archived, since nothing moved.

**Prediction against outcome** ([PREDICTION.md](PREDICTION.md)):

| Measure | Predicted | Measured |
|---|---|---|
| region facts | derive; acutest resolved (80%) | **as predicted**, after a reader fix for spliced spellings (the first run failed closed on every unit) and 6 more library files and the headers the census named |
| region at `90978ac6` | nothing reaches a hazard; discharged (65%) | **as predicted** (206 invocations, 43 macros) |
| P-HOLDER-OPAQUE | derived, `cb_init` registrants unwritten (60%) | **derived, but not as predicted**: `in_storage_backlog`'s init stores its `data`; derived only once the instance's own callback is narrowed to the copy's |
| old test | not discharged (97%) | **as predicted** |
| other sites | unchanged (85%) | **as predicted** |
| raw readings | equal but the reason text (80%) | **as predicted** (previous raw byte-identical) |
| edges / functions / seeds | unchanged (90%) | **as predicted** |
| corpus / system / slice | byte-identical sidecars (95%) | **as predicted** (corpus byte-identical; system and slice but their root lines; 0 mismatches) |
| follow-up | amended, not retired (60%) | **retired**: the closure's macro-made calls were derived too (the name-keyed part registered as a residual) |

## 6. Premises of the three discharged sites

Stated as: **discharged at `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`
(the `90978ac6` export, configured reading with both facts) under these
derived premises**, never a verified fix. Derived at this path: the owners,
the coroutine hop and its resumers, the typed dispatch, the list-head
instance, the holder escape, P-CALLBACK-INSTANCE, the property keys, the
predefined-macro decisions, and now **P-HOLDER-OPAQUE** and **the region's
macro invocations and the census graph's macro-made calls**.

Stated, not proved (trust boundary 28): P-CORO-HANDLE, P-PLUGIN-DIR,
P-HEAD-INSTANCE, P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS,
P-INSTANCE-FIELDS-BY-MEMBER (now also: an instance's `data` is read only
through member accesses), P-KEY-FOLD, P-INSTANCE, P-RECORDED-COMPILER,
P-DD-COMPLETE, the immutable threaded flag, acutest's `TEST_CHECK`, the
lexical and typed readings' faithfulness, C's effective-type rule (also for
an untyped member of an object declared with another record type), and
rule 10p's premise that nothing outside the read files names a read
function (now also: a macro defined only outside the read files, outside the
units' 145 files). Added by this card: **P-DUMP-TOKENS-COMPLETE** (Clang's
token dump prints every token the unit's preprocessor produces, with its
expansion and spelling locations).

## 7. Lean

`proof/QCore/LoopOwner.lean`, audited in `proof/AuditLoopOwner.lean`: 81 →
101 theorems, no new audit module, no placeholders, no project axioms. The
region is a list of lexical calls and invocations; the program runs each
invocation's expansion under the unit's table:

- **`the_region_reading_derives_what_the_expansion_runs`** and
  **`a_derived_region_head_is_the_programs_one_creation`**: a read region
  creates and mutates exactly what the expansion runs, so a derived head is
  the one creation of the program's own path;
- **`an_unexpandable_invocation_refuses`**, **`a_loud_expansion_refuses`**,
  **`a_region_without_invocations_reads_lexically`**,
  **`the_included_definition_is_the_one_read`**;
- **`a_passing_root_reaches_a_hazard_only_as_counted`**,
  **`a_root_reaching_only_through_a_macro_call_refuses`**
  (`reachesWithin_more_edges`);
- **`the_holder_instance_runs_its_copy`**: whenever the user pointer is the
  holder, the plugin is the copy, under any sequence of constructions and
  the installation;
- **`an_opaque_holder_writes_nothing_the_route_reads`**,
  **`an_opaque_holder_keeps_the_derivation`**,
  **`a_carrier_that_writes_keeps_the_route`**,
  **`a_data_reader_that_writes_the_flag_keeps_the_route`**;
- **`the_route_is_dropped_only_through_read_macros_and_an_opaque_holder`**
  and **`the_old_test_path_is_never_discharged_whatever_the_region`**.

## 8. Follow-ups

[`followups.json`](followups.json), applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 568 → 569 (a second run changes nothing).

- **Retired:** `W47-LOOP-OWNER-REGION-MACRO-CALLS`: its DISCHARGE clause is
  met: the region counts the functions an invoked macro names, resolved by
  the unit's own expansion, and the reach counts every function's macro-made
  calls, measured on the census at both revisions with the old test still not
  discharged.
- **Registered:** `W47-CLOSURE-MACRO-CALLS-NAME-KEYED` (outside the units'
  145 files the macro-made calls are the name-keyed over-approximation, and a
  macro defined only outside the read files is not expanded; 0 sites move)
  and `W47-REGION-MACRO-FACTS-FILE-LIST` (the facts' 10 region files are a
  script list the census's refusals maintain).
- **Amended:** `W47-LOOP-OWNER-INSTANCE-FLOW` and
  `W47-INSTANCE-FLOW-CONTRACT-REVIEW` (P-HOLDER-OPAQUE derived).

## 9. Gate

[`scripts/gate.py`](scripts/gate.py) runs `rust`, then `records`. Exits are
in [gate/](gate/) and figures in [gate.json](gate.json). Every step exited
0, on the first run:

- submodule (`git submodule update --init third_party/libevent`: the only
  submodule; `--recursive` was refused by this session's permission layer);
  fmt clean; the Rust suite: 356 units / 369 summaries, **6,375 passed, 0
  failed**, 11 ignored (`private/test-binaries/20260929T060258Z-3518047/`);
  clippy clean; both probes 0;
- **224 Lean audits** (`AuditLoopOwner.lean` 81 → 101 theorems, no new audit
  module);
- `providers/` is unchanged, so the provider gate is not due and was not run
  (the matrix's provider figure, 129, is carried from the predecessor's
  record);
- the `records` phase, after the ledger, matrix and index regeneration: 365
  Python tests OK, docs index 0, Clang 21.1.8 recorded.

**Files likely to conflict:**

- `docs/agents/gateway.md` (the 10p amendment), `docs/agents/README.md` (the
  10p row), `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md`, `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs` (two child modules;
  `BuildConfiguration::region_macros`; `StepB::region_macros`,
  `copy_literal`; the P-HOLDER-OPAQUE call in `copy_flag`; the literal in
  `instance_flag`), `loop_owner_instance.rs` (`list_head_instance`'s region
  hook, the widened closure and reach, the literal; `holder_escape`'s note;
  `in_build` made `pub(super)`), `loop_owner_property_keys.rs`
  (`KeyedCall::parts`);
- `crates/quirin-source-analysis/examples/loop_owner_census.rs`
  (`--region-macros=`), `tests/loop_owner_{instance_flow,predefined_macros,coroutine}.rs`
  (one field each);
- `proof/QCore/LoopOwner.lean`, `proof/AuditLoopOwner.lean` (a new section
  at the end).

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/predefined_facts.py OUT PREDEFINED_BINARY GENERATED_DIR
python3 docs/reports/w47-premises-holder-region/scripts/region_facts.py OUT REGION_BINARY GENERATED_DIR
python3 docs/reports/w47-premises-holder-region/scripts/census.py OUT CENSUS_BINARY TYPED PREDEFINED REGION
python3 docs/reports/w47-premises-holder-region/scripts/compare.py BEFORE_CENSUS AFTER_CENSUS
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/identity.py RUN OUT
python3 docs/reports/w47-premises-holder-region/scripts/gate.py LOGS rust|records
```
