# DEEP-AST-TRANSPORTS-AND-WALL-TIME: every Clang JSON transport's nesting, and `rdhttp.c`'s own time

Card DEEP-AST-TRANSPORTS-AND-WALL-TIME (wave 48, card-high), on the user's
direction "handle as safe as possible". Base: local `main` at `5214a3ec3`
(unchanged while the card ran). [PREDICTION.md](PREDICTION.md) was committed
at `2b87c5632`, after the BEFORE measurement and profile and before any
analysis, transport or cache code; the only code before it is the
measurement-only `unit-timing` feature (compiled out without it).

**Published property and scope.** No new property: custody (`c.custody`) on
the pinned corpus, the system route and the tail-pause slice. Task A closes
`DEEP-AST-FILTERED-TRANSPORTS` (gateway rules **13q**, **13r**); task B closes
`RDHTTP-DEEP-UNIT-WALL-TIME` (rules **13s**, **13t**). Lean
`QCore.DeepAstTransports`. Rule id `13u` is unused.

## 1. BEFORE

Binary `private/dat/bin/before` (`3dcd3b2e…`, built at the base, clean tree).
Three runs, every route exit 0, each reproducing the brief's figures: corpus
881 / 9 / 3,944, report `ab8f5f14…`, verdict rows **`321a8913…`** (the brief's
digest); system 87 / 4 / 357 (`06954e60…`); slice 45 / 1 / 354 / 4 Pending
(`f2368128…`). Every BEFORE run is identical to every other
([`scripts/identity.py`](scripts/identity.py), section 4). The measurement-only build
(`before-timing2`, `1af0f0a7…`) writes the same report.

## 2. Task A: the transports

### 2.1 Inventory

Every place that parses Clang's JSON (`rg -- '-ast-dump'` and every
`serde_json` reader of a Clang output), with the nesting it reads and where
its readers recurse. The table is `deep_ast_transports::TRANSPORTS`, bound to
`QCore.DeepAstTransports.transports` by a unit test.

| transport | readers (recursing over what it returns) | before | after |
| --- | --- | --- | --- |
| `typed_access_path::AstIndex::from_json` (the unit index) | every typed reader of the analysis, on the analysis worker (2 MiB) | band (rule 13o) | band, unchanged |
| `typed_c_ast::ast_declarations` for the analysed unit: `definition_ast`, `appended_declarations` | rule 11y's extension reader (`extension_in_ast`), the retention guard's shadow check, the 12u probe readers (they stop at the probe's first `ConstantExpr`), the cache's clone and the drop | default (128) | **band (13q)**, own launch `unit_declarations` and own cache |
| `typed_c_ast::translation_unit_ast` | `atomic_publication_source` (rule 11i's closed world) | default | **band (13q)** |
| `typed_c_ast::ast_declarations` / `function_ast` for the source importers | `typed_source`, `boolean_source`, `typed_source_match`, `layout_extents`, `cast_representation` | default | default (13r) |
| `c_numeric_obligations::typed_ast` | the numeric property's report (no route caller) | default | default (13r) |
| `ffi_layout::read_struct_layout` | the FFI layout probe of one tag | default | default (13r) |
| `typed_dispatch_census` | its streaming seed, on its 256 MiB parse stack | no limit | unchanged (13r; residual) |
| `index_extent::analyze_bounds_preprocessed` | the bounds side pass, on its 256 MiB analysis stack | no limit | unchanged (13r; residual) |

(`examples/numeric_obligations.rs` also reads a filtered stream with the
default limit; it is an example, not a route.)

### 2.2 Rule 13q: the two transports that now read the band

* **The analysed-unit filtered launch.** `typed_c_ast::unit_declarations` is
  `ast_declarations`'s launch with the stream read by
  `deep_unit_ast::values_from_slice`: the stream is parsed as before (every
  value, default limit); only when that fails is the text's nesting measured
  (`depth` of the whole text, its deepest value's) and a text of at most 256
  levels parsed again without the limit on a dedicated 256 MiB stack.
  Otherwise, and when the reparse fails too, the default error stands. The
  importers keep `ast_declarations` and its cache; `unit_declarations` has
  its own, so a stream only the band reads is never answered to an importer.
* **The closed world.** `translation_unit_ast` reads its document with
  `deep_unit_ast::from_slice`, rule 13o's own reader.

**Readers, measured.** `deep_ast_transports::stack::high_water` runs a
closure on a fresh 64 MiB thread (never a reused stack: beyond the C
library's 40 MiB cache) and reads the deepest touched page from
`/proc/self/pagemap` (a control checks it measures recursion). Debug build,
the controls' budget 1 MiB (half an analysis worker's 2 MiB stack):

| reading | nesting | stack high-water |
| --- | ---: | ---: |
| `definition_ast` + extension reader + shadow check + drop, `else if` chain of 122 arms | 255 (filtered stream) | 236 KiB |
| the same, nested conditional expression of 60 arms (the type-check macro's shape) | 253 | 232 KiB |
| `extract_atomic_publication_source`, unit holding a 121-arm chain | ≤ 256 (whole unit) | 232 KiB |
| `definition_ast` + readers on `rd_http_ssl_configure` (`rdhttp.c` preprocessed with its compile-database flags, `private/dat/rdhttp.i`, [`scripts/deep_source.py`](scripts/deep_source.py)) | 221 (filtered value) | 232 KiB |
| the same on `rd_http_get`, `rd_http_req_init`, `rd_http_post_expect_json` | 211, 209, 209 | 232 KiB each |
| the same on the shallow `rd_http_error_new` | 25 | 56 KiB |

([log](artifacts/readers-rdhttp-debug.log), the ignored test
`measure_the_readers_on_a_preprocessed_unit`.) Release analysis workers over
the whole corpus, `rdhttp.c`'s 223-level index readers included, never touch
more than **144 KiB** of their 2,048 KiB stacks (`unit-timing`'s pagemap
mark, BEFORE and AFTER alike). One arm beyond the bound the stream and the
closed world are refused with the default parse's error, as before (controls
`the_analysed_unit_readers_fit_at_the_bound`,
`the_closed_world_readers_fit_at_the_bound`). The one-arm-deeper beyond-bound
shadow check refuses (`true` for any name), the answer it gave before.

### 2.3 Rule 13r: the transports that keep their limit

* **The source importers** read named chunkio, Fluent Bit and Monkey
  functions of fixed shapes through many recursive interpreters, none
  measured deep; a refusal is their fail-closed answer, so they keep the
  default limit.
* **The numeric whole-unit AST** has no route caller; **the FFI layout probe**
  reads one tag's probe declarations. Neither reader is measured deep.
* **The typed dispatch census and the bounds side pass** already parse with no
  limit on their own 256 MiB stacks and recurse there. They are unchanged and
  registered (`UNBOUNDED-DEPTH-AST-READERS`): their depth has no bound of
  ours, only Clang's.

### 2.4 AFTER for task A

No transport change reaches a corpus, system or slice row: rule 11y's fallback
is not consulted where the unit index exists (rule 13o), and no retention
guard or 12u probe consultation names one of the four deep `rdhttp.c`
definitions. Every route is byte-identical to BEFORE (section 4). The
`mk_vhost_set` tip probe ([`scripts/tip-probe.sh`](scripts/tip-probe.sh)):
verdict rows identical on the tip (`a35c40cf…`) and the master copy
(`7bf0efa2…`); `mk_vhost_set` Unknown (tip 1 leaf, master 6),
`mk_vhost_set_property` Unknown (2).

## 3. Task B: `rdhttp.c`'s own time

### 3.1 Measured where it goes (BEFORE)

The optimization card's sampler ([`scripts/gdb_sampler.py`](scripts/gdb_sampler.py),
every 0.5 s, now with each thread's kernel id) joined to the measurement-only
`unit-timing` intervals (`qcore-audit`'s feature `unit-timing`: each unit's
thread, start and end, and the thread's stack mark;
[`scripts/profile_unit.py`](scripts/profile_unit.py)). Sampled run `bprof1`
([units](artifacts/units-before-bprof1.json),
[profile](artifacts/profile-before-rdhttp.json),
[callers](artifacts/callers-before-rdhttp.txt)):

* the corpus is one phase (185 unit analyses, no phase-two re-analysis);
* `rdhttp.c` is analysed for **132.5 s** (129.7 s in the unsampled `bt1`),
  started at 412 s and ending the run at 544.5 s: heaviest-first dispatch
  weighs units by source size, and its source is small;
* of its 265 thread-samples: Clang wait 1 %, the rule-13o reparse 1 %, the
  depth scan 0 %, the default parse 0 %: the 750 MB document is parsed once
  and its parse is not the cost;
* **33 %** `call_exits::resolve` under the parity-only `call_exit_census`: a
  whole-function walk per uncontracted call statement (the four deep
  definitions call 44 warning functions 908 times);
* **44 %** `Parser::elaboration_node`: per statement hook a walk from the root
  computing each node's begin and end line (`node_line`, 18 % self) and
  tokenizing each candidate's source span (9 %), where the type-check
  expansion puts whole subtrees on one line.

### 3.2 Rules 13s, 13t

`function_reading_reuse::FunctionReuse` lives in each function's entry of the
unit index, beside its line index: one Clang document of one launch identity
(the full text, frontend request, dialect, resolution environment; the key
`clang_reuse::UnitKey` compares byte for byte), so every cached reading keys
on the full document by construction and is dropped with it.

* **13s.** `call_exits::resolve(function, callee, …)` is computed by
  `resolve` itself on the first ask for a callee and answered from a memo
  after. (A first version built one table per function by one walk; it
  cloned every reference of every function asked once and was replaced
  before the final measurement.)
* **13t.** `elaboration_node`'s walk reads each node's begin and end line from
  a line tree built on the walk's first descent into the node (same child
  order, the same `node_line` / `node_end_line`), and each candidate span's
  tokens from a per-function memo of `node_tokens`.

A memo stores only its own computation's answer for the key, and a key is
answered only from its own entry: `QCore.DeepAstTransports`
(`the_memo_answers_as_the_function`, `asking_keeps_the_memo_sound`,
`every_ask_is_answered_by_the_function`, `the_annotated_walk_is_the_walk`).
Unit tests compare every name and every node against the uncached readers.

### 3.3 AFTER

Final binary `private/dat/bin/a3` (`d5ac72e8…` = every AFTER run's
`binary_sha256`), built at `8f55b9ab8` (clean tree); its `unit-timing` twin
`a3-timing` (`84af3b52…`). The host's speed drifted by several percent over
the hours of measurement (the same BEFORE binary's system route took 63.2 s
and 71.6 s), so the headline compares runs taken alternately, BEFORE and AFTER,
lock time excluded, `--jobs 4`, parity on
([table](artifacts/timing-interleaved.json); every run in
[timing-all.json](artifacts/timing-all.json)):

| route | BEFORE `before7`, `before8`, `before9` (s) | median | AFTER `final3`, `final4`, `final5` (s) | median | change |
| --- | --- | ---: | --- | ---: | ---: |
| corpus | 566.2, 547.7, 550.2 | 550.2 | 499.5, 494.5, 497.8 | **497.8** | **−9.5 %** |
| system | 65.8, 65.2, 66.3 | 65.8 | 66.0, 69.8, 69.6 | 69.6 | +5.8 % |
| slice | 29.7, 28.1, 29.2 | 29.2 | 31.5, 30.3, 30.8 | 30.8 | +5.5 % |

The first trio (`before1`–`3` 550.3 / 522.8 / 538.9 s against `final1`–`3`
517.3 / 519.6 / 499.5 s, the second pair taken while the host was slow)
gives −4.0 % on the corpus.

**`rdhttp.c`'s own time** (`unit-timing`, unsampled runs taken alternately):
129.7, 129.1, 122.5, 130.8 s BEFORE → 68.7, 70.2, 64.5 s AFTER (medians
129.4 → 68.7 s, **−47 %**); its own CPU 128.8 → 62.5 s
([cpu](artifacts/cpu-units-c1-c2.json), `cpu_poll.py`: the analysing thread's
ticks). It is still dispatched last (started at 412–453 s) and still ends
the run, so the corpus saves about the unit's own saving. The sampled AFTER
run ([profile](artifacts/profile-after-rdhttp.json),
[callers](artifacts/callers-after-rdhttp.txt)): `call_exits` 3 samples of 140
(was 87 of 265); `elaboration_node` 59, called by three hooks per statement
over the same macro subtree (registered,
`ELABORATION-NODE-PER-HOOK-WALKS`). Corpus-wide the touched readers fell from
9.5 % to 5.3 % of busy samples (`elaboration_node`) and from 2.6 % to 0.1 %
(`call_exits::resolve`) ([frames](artifacts/frames.txt)).

**The other units.** The arbiter's own CPU over the corpus is 1,862.9 s
BEFORE and 1,777.3 s AFTER (`c1`, `c2`; [totals](artifacts/cpu-totals.txt));
without `rdhttp.c`, 1,751.1 → 1,733.3 s. Per unit the gain is in the large
units (`rdkafka_mock_handlers.c` 335.8 → 305.4 CPU s, `rdkafka_admin.c` 300.0
→ 278.8) and the median unit over 5 s is 6.5 % slower (wall: 9–16 % in the
`u1`–`u4` pairs, [units](artifacts/units-wall-u1-u4.txt)), with no frame of
the card's in its profile (`rdkafka_sticky_assignor.c` spends its samples in
the same untouched readers before and after). On the system route the card's
readers are under 1 % of busy samples before and after
([frames](artifacts/frames.txt)); its own CPU is 184.0 and 198.4 s BEFORE,
213.3 and 197.5 s AFTER, an 8 % spread between two runs of one binary. The
system and slice slowdown of about 5 % is therefore measured but not
attributed to a frame; it is registered (`SYSTEM-SLICE-WALL-TIME-AFTER-READING-REUSE`)
rather than claimed as noise.

**Stack.** The release workers' high-water mark stays 144 KiB of 2,048 KiB
(`aprof1`, `aprof3`).

## 4. Identity

[`scripts/identity.py`](scripts/identity.py): every output file of every route
by sha256, raw and, where a file names its own run directory or the
`preparation:` durations, after replacing exactly those; each route's parity
stream and its contexts sidecar as multisets with Clang node addresses
erased. Against `before1`, **every** run is identical on every route it ran:
the five other full BEFORE runs and the system and slice routes of
`before4`–`6`, the five final AFTER runs (`final1`–`5`, binary
`a3`), and the two superseded AFTER binaries (`a1`, the eager line tree and
the one-walk table; `after1`, binary `a2`, the lazy tree and the table)
([proofs](artifacts/identity/)): report `ab8f5f14…`, verdict rows
`321a8913…`, system `06954e60…`, slice `f2368128…`; parity multisets 683,320
/ 139,962 / 50,407 rows, identical. The measurement-only builds' reports are
`ab8f5f14…` and `06954e60…` too. No row moved, so there is no correction and
nothing to attribute.

## 5. Prediction against outcome

| prediction | outcome |
| --- | --- |
| task B: every route byte-identical | held (section 4) |
| `rdhttp.c` 132 s → 35–70 s | 129.4 → 68.7 s (medians of unsampled runs), held |
| corpus 550 s → 450–500 s | 550.2 → 497.8 s (interleaved medians), held |
| system and slice within ±5 % | **+5.8 % and +5.5 %**, just outside; not attributed to a frame (section 3.3) |
| task A: no verdict-row change | held (no row, report or parity difference) |
| workers' stack at most 256 KiB | 144 KiB, held; the readers at the bound 232–236 KiB (debug) |
| no wall-time change from task A | not separable: both tasks ship in one binary; task A's readers do not run on a deep definition on these routes |

## 6. Lean

`proof/QCore/DeepAstTransports.lean`, audited by
`proof/AuditDeepAstTransports.lean` (16 theorems, no placeholder, no project
axiom): `reads_iff_within_the_bound`, `the_band_keeps_every_default_reading`,
`nothing_beyond_the_bound`, `a_default_transport_is_unchanged`,
`the_band_transports`, `streamDepth_le_iff`,
`a_stream_is_read_iff_every_value_is`, `fits_at_the_bound_fits_within_it`
(task A); `lookup_mem`, `the_memo_answers_as_the_function`,
`asking_keeps_the_memo_sound`, `the_empty_memo_is_sound`,
`every_ask_is_answered_by_the_function`, `annotated_walk`,
`annotated_walkList`, `the_annotated_walk_is_the_walk` (task B). The
transport list is bound to the Rust table
(`lean_transports_are_the_rust_transports`). Not proved: that the measured
stack figures hold for every input of a given depth (they are measurements),
that `serde_json` reads the same value with and without its limit (the
library's contract, as in `QCore.DeepUnitAst`), Clang's fidelity (trust
boundary 28).

## 7. Gate

TBD-GATE

## 8. Ledger

* **Retired `DEEP-AST-FILTERED-TRANSPORTS`**: each transport reads within the
  bound with its readers measured on the deepest admitted definition (13q), or
  records why it must not (13r).
* **Retired `RDHTTP-DEEP-UNIT-WALL-TIME`**: the unit's own time measured and
  reduced with no row changed (13s, 13t).
* Registered `UNBOUNDED-DEPTH-AST-READERS` (2 readers, no document beyond 256
  on the three routes), `CORPUS-DISPATCH-BY-SOURCE-SIZE` (`rdhttp.c` started
  last in every measured run), `ELABORATION-NODE-PER-HOOK-WALKS` (59 of
  `rdhttp.c`'s 140 AFTER samples), `SYSTEM-SLICE-WALL-TIME-AFTER-READING-REUSE`
  (+5.8 % / +5.5 %, unattributed), `TOKEN-ROUTE-ELSE-IF-RECURSION-STACK`
  (RDHTTP-HEADER-SUMMARY-GAP's measured debug overflow at 130 arms, until now
  unregistered). Follow-ups 737 → 740.

## Files

* Code: `crates/qcore-c-gateway/src/deep_ast_transports.rs` (new: inventory,
  stack measurement, controls), `function_reading_reuse.rs` (new),
  `deep_unit_ast.rs` (`values_from_slice`, the shared parse stack),
  `typed_c_ast.rs` (`unit_declarations`, the closed world's reader),
  `typed_access_path.rs` (the two consultations, `FunctionEntry.reuse`,
  `StatementIndex::{call_resolution, line_tree, node_tokens_once}`),
  `legacy_call_exits.rs`, `legacy_elaboration.rs` (hooks),
  `legacy_retention_guard.rs` (`shadowed_in` visible to the crate's tests),
  `lib.rs`; `crates/qcore-audit/src/unit_timing.rs` and its call site
  (feature `unit-timing`, measurement only).
* Lean: `proof/QCore/DeepAstTransports.lean`, `proof/AuditDeepAstTransports.lean`,
  `proof/QCore.lean`.
* Rules: gateway 13q–13t, router rows. `providers/` unchanged: no contract, no
  re-pin.
* Scripts: [`scripts/`](scripts/); artifacts: [`artifacts/`](artifacts/).
