# GATEWAY-C-LOCATION — a gateway diagnostic names the C a person wrote — 2026-09-12

## Headline

Every line number the existing-C gateway carries is a line of the unit's
`clang -E -P` **preprocessed** translation unit. Diagnostics presented it
directly — `live obligation acquired at preprocessed line 19357` — and that
coordinate names a text nobody reads: a 20,471-line expansion of one 3,000-line
source file, which no editor opens and no reader can check. The user's ruling
for this card is that the presented coordinate shall be the **original**,
pre-preprocessing `file:line`, and that the preprocessed number stays
**internal** — pin identity, leaf identity, certificate comparability — rather
than user-facing.

This card derives that coordinate and presents it. `L19357` is now
`flb_input_chunk.c:2267`, which is
`result = flb_calloc((size_t) stored_count, sizeof(struct flb_chunk_direct_route));`
in the pinned checkout.

**The re-pin is presentation-only, and measured to be.** The BEFORE run
reproduces the current baseline's `diagnostics.json` and `chunk-contracts.json`
**byte for byte**; the AFTER run differs from it in attribution presentation
alone, checked leaf by leaf by this card's own oracle
([`leaf_identity.py`](leaf_identity.py)):

| measure | BEFORE | AFTER |
|---|---:|---:|
| units | 185 — **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** | **identical** |
| projected functions | 4,834 — **150 Safe / 84 provisional Unsafe / 4,600 Unknown** | **identical** |
| `leak` | **266** | **266** |
| `unknown_alias` | **4,344** | **4,344** |
| `unsupported` | **49,773** | **49,773** |
| diagnostic rows | **54,383** | **54,383**, each with the same `(function, line, category)` |
| unit rows, function rows, enumeration blocks, reasons, reason codes | — | **byte-identical** |
| `source.json`, `source-effects.json`, `contracts.json`, `chunk-contracts.json` | — | **byte-identical between the two runs** |

Zero verdict moves, zero leaf-set moves, zero STOP findings. What moved:
54,371 diagnostic rows gained an `origin`, 266 descriptions had their site
rewritten (exactly the 266 leak leaves — they are the only category whose text
names a site), 178 unit rows gained an `origin_coordinates` account, and the
report envelope gained one key and rewrote another.

No new diagnostic code (the icebox rule holds — this is an *attribution* fix,
which §L21 constraint 1 says is not a code change), no new keyword, no verdict,
status, certificate, obligation or contract change.

---

## 1. The mechanism

Two preprocessings of the same unit are paired.

* The **authoritative** one is the unit's own `-E -P` text — the bytes the
  analyzer read, whose lines its diagnostics index. It is handed in by the
  caller and never re-derived here, so a map can only ever describe the text it
  was built against.
* The **presentation-only** one is a second `-E` run *with* Clang's line
  markers. This is the same command and the same marker reader that
  `crates/qcore-c-gateway/src/legacy_enumeration.rs` has used since unit
  E01-TAIL for include-depth attribution; the marker parser is now shared
  rather than copied (`line_marker` and `preprocess_with_line_markers` became
  `pub(crate)`, and `LineMarker` gained the line number it already parsed and
  discarded).

Walking the marked output gives every *content* line an
`(include depth, file, line)` reading. The two outputs' non-blank content lines
are then paired **in order**. The pairing is **checked, not assumed**: the two
sequences must have the same length and the same text line for line, or the
whole map is refused. The only measured difference between the two outputs'
content lines is leading whitespace — `-E` keeps a line's indentation, `-P`
collapses it — so the comparison is on trimmed text. Everything else that
differs between them is blank-line filler and the markers themselves, neither
of which carries a diagnostic.

Implementation: `crates/qcore-c-gateway/src/legacy_location.rs`
(`map_preprocessed_lines`, `map_marked_preprocessing`, `walk_marked`).
Consumption: `crates/qcore-audit/src/projection.rs` builds the map for a unit
that has at least one diagnostic, and `crates/qcore-audit/src/report.rs`
renders every site through it.

### 1.1 Macro expansions

A construct produced by expanding a macro presents the **expansion site's**
line, because that is where Clang's markers put the expanded text: the
expansion is written out at the line of the invocation and no marker
intervenes. `a_macro_expansion_presents_its_expansion_site` pins it against a
real preprocessing of a `DEFN(name)` macro that expands to a whole definition —
the E01-B1 shape — and the presented line is the invocation's, not the macro
body's.

### 1.2 What is *not* re-derived

`preprocessed_source` is untouched, and so is every identity bound to it
(standing constraint 11). The map reads no contract, publishes no verdict,
changes no status, and is never consulted by the analysis — in
`project_unit` it is built *after* `result` is already settled.
`deriving_the_map_changes_neither_the_text_nor_the_scope` pins that the
preprocessed text and the enumerated denominator are equal before and after the
derivation.

---

## 2. The anti-forgery argument (standing constraint 12)

Constraint 12 says a `#line` directive in analysed source must not be able to
move the audited scope, and that attribution rides the include-depth flags no
directive can move. That rule is untouched: the scope is decided by
`legacy_enumeration`, which this module does not read, does not write and
cannot influence. The card's obligation is that the *presentation* not open the
door the scope rule closed.

It does not, by construction, and the property is stronger than "hard to
forge":

**No presented file spelling is ever read from a flagless marker.**

Three rules together:

1. **A file name is adopted only from an entering marker.** A frame's spelling
   is written when — and only when — Clang emits flag `1` (entering a file).
   Clang re-emits a `#line` directive as a **flagless** marker, so a `#line` can
   never write a frame's spelling.
2. **Include depth 0 is not named by a marker at all.** The primary input
   presents `OriginFile::Primary`, which the *caller* resolves against the
   audit's own configured file. The opening marker's spelling — which under the
   pinned Cycle 19 transport is the fixed, guessable `<stdin>`
   (`CSourceInput::HistoricalStdin`) — reaches no report. This also removes the
   residual E01-F1-STDIN shape from the presentation side: the transport's
   spelling is not a name anything can claim.
3. **A disagreeing flagless marker withdraws rather than renames.** When a
   flagless marker names a file other than the one its frame was entered as,
   the frame is *relabelled* and every content line it governs carries **no**
   coordinate. The diagnostic falls back to `preprocessed line N`, which names
   itself as internal. Fail-closed: an unattributed diagnostic asserts nothing,
   where a relabelled one would assert something false.

Both measured `#line` directions of `remaining-e01-tail` are pinned against
real preprocessings in
[`crates/qcore-c-gateway/tests/gateway_c_location.rs`](../../../crates/qcore-c-gateway/tests/gateway_c_location.rs):

| written | scope (constraint 12) | presentation (this card) |
|---|---|---|
| `#line 100 "<stdin>"` inside a **header** | header stays out of the denominator — `definitions() == ["subject"]` | the header's line presents **nothing**; the main source's own line is unaffected |
| `#line 5000 "forged.c"` in the **main source** | the tail stays in the denominator — `definitions() == ["after", "before"]` | the tail presents **nothing**; `forged.c` appears in no output |

### 2.1 What a `#line` can still do, stated exactly

A `#line 500` with **no file spelling** renumbers the file it is written in.
Clang re-emits it as a marker carrying that frame's own name, which is
indistinguishable from one of Clang's own resynchronisations after a skipped
region — and must be, because treating every flagless marker as a relabel would
withdraw the coordinate of every file with an `#if 0` block in it. So the
presented *line* inside a file can be moved by that file's own author.

That is exactly the directive's C11 §6.10.4 meaning: it is the author's
statement about the author's own file, it is what `__LINE__` and every C
compiler's own diagnostics report, and it moves **no file identity, no include
depth and no audited scope**. Recorded as
`GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE`, and pinned positively by
`a_line_directive_without_a_file_renumbers_its_own_file_only` so the
discrimination is not vacuous.

### 2.2 The measured corpus is silent on this

Across the pinned 185-unit corpus the AFTER run reports **0** withdrawn
(relabelled) content lines and **0** refused derivations. No unit of the
audited corpus writes a `#line` that this reader has to refuse — the rules
above are load-bearing for the property, not for the numbers.

---

## 3. Fail-closed

Nothing is guessed. Four named grounds, all of them facts about *this analysis
machinery*:

| ground | condition |
|---|---|
| `expansion-unavailable` | the line-marked preprocessing did not run |
| `primary-input-unidentified` | the marked output does not open with a marker |
| `unbalanced-include-markers` | a return with nothing to return to, or a file still open at end of output |
| `content-disagreement` | the marked output's content lines do not correspond one for one with the analysed `-E -P` text |

Each carries `ReasonCategory::EnvironmentUnavailable`, published through the
producer-supplied path (`reason_code_value`) rather than re-classified from
prose, so nothing was added to `qcore_schemas`'s closed vocabulary or its
`REASON_RULES` table. §L21.1 rule 1 then applies: none of these ever describes
the analysed source, and none can contribute to a provisional `Unsafe`. A
refused unit publishes `"origin_coordinates": {"status": "refused", …}` and
every diagnostic keeps its internal position — never an approximate coordinate.

---

## 4. The spot check, against the pinned sources by hand

[`spot_check.py`](spot_check.py) reads each presented coordinate back out of
the pinned checkout and prints the line it names, so the derivation and the
source can be compared by eye
([`logs/spot-check.txt`](logs/spot-check.txt)).

**A. The corpus record's own leaf.** The one surviving leak of
`flb_input_chunk_get_direct_routes` (constraint 7: wave 23 retired six of the
seven, `L19544` stands because it is on the success path):

| internal | presented | line in `src/flb_input_chunk.c` |
|---|---|---|
| leak row at `L19544` | `flb_input_chunk.c:2480` | `return 0;` — the success-path exit |
| its acquisition, written `L19357` in every prior record | `flb_input_chunk.c:2267` | `result = flb_calloc((size_t) stored_count, sizeof(struct flb_chunk_direct_route));` |

**B. The hand-noted `:2483` / `:2623` oracles** of the wave-22 and
wave-23 records, read straight out of the pinned source and reproduced by this
card's own reader:

```
flb_input_chunk.c:2483  void flb_input_chunk_destroy_direct_routes(struct flb_chunk_direct_route *routes
flb_input_chunk.c:2623  int flb_input_chunk_destroy_corrupted(struct flb_input_chunk *ic,
```

Both land on the declaration line the reports name, which is what those
coordinates were written to mean.

**C. A deterministic cross-library sample** — every 4,001st diagnostic row of
the run, with the source line its coordinate names:

| unit | internal | presented | source line |
|---|---:|---|---|
| `cmetrics/cmetrics.c` | `L5588` | `cmetrics.c:47` | `cmt = calloc(1, sizeof(struct cmt));` |
| `cmetrics/cmetrics.c` | `L5595` | `cmetrics.c:55` | `free(cmt);` |
| `cmetrics/cmt_encode_prometheus_remote_write.c` | `L7103` | `…:867` | `summary = (struct cmt_summary *) map->parent;` |
| `librdkafka/lz4.c` | `L1128` | `lz4.c:1613` | `workingStream->internal_donotuse.dictCtx = dictCtx;` |
| `librdkafka/rdkafka.c` | `L23813` | `rdkafka.c:5194` | `rd_kafka_q_fwd_set(rk->rk_rep, rkq);` |
| `librdkafka/rdkafka_admin.c` | `L26540` | `…:9735` | `rd_kafka_buf_skip_tags(reply);` |
| `librdkafka/rdkafka_cert.c` | `L20403` | `rdkafka_cert.c:405` | `cert->pkey = PEM_read_bio_PrivateKey(` |
| `librdkafka/rdkafka_queue.c` | `L19700` | `rdkafka_queue.c:71` | `assert(!rkq->rkq_fwdq);` |
| `librdkafka/rdkafka_timer.c` | `L19616` | `rdkafka_timer.c:131` | `rd_kafka_timers_unlock(rkts);` |
| `librdkafka/rdunittest_msgset_errors.c` | `L20116` | `…:920` | `RD_UT_ASSERT(rko != NULL, "Expected FETCH op %d", i)` |
| `monkey-libevent/mk_server/mk_http.c` | `L6565` | `mk_server/mk_http.c:756` | `if (memmem(sr->uri_processed.data, sr->uri_processed…` |

Every sampled coordinate lands on a line whose text matches the diagnostic's
own subject — an allocation for a leak, an assert for a branch-condition
refusal, a member store for an indirect-access refusal. The full sample is in
the log.

All 54,371 presented coordinates name the unit's **own** file and none names a
header, which is the expected shape: the pinned route's scope is
`MainSourceDefinitions`, so every analysed body is main-source text at include
depth 0. The header arm is exercised by
`a_header_line_presents_the_header_its_entering_marker_named`, which analyses a
unit whose header carries a `static` definition and checks the presented name
is the header's, at depth 1.

---

## 5. The re-pin, attributed

### 5.1 The route

```sh
git submodule update --init --recursive          # FIRST: the libevent pin gate fires at the END of a run
sh measure/prepare-pin.sh                        # archived as prepare-pin.sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-location
sh measure/build-binaries.sh                     # archived as build-binaries.sh
sh measure/run-audit.sh <before|after> measure/bin/<before|after>   # archived as run-audit.sh
python3 -B measure/byte_identity.py measure/runs/before
python3 -B measure/leaf_identity.py measure/runs/before measure/runs/after
```

A **fresh** Clang pin, per the wave-20 corpus record: neither the long-lived
developer database nor any earlier card's pin is reused. The pin's own
production database is copied into `<source>/build/compile_commands.json`,
which the preparation script does not create, and **both runs read the same
bytes** (SHA-256 `ee471fbad091f7a0…`, **1,415 entries**, `/usr/bin/clang`
spelled, not `cc`). Both runs: **exit 0**, zero gateway failures, all six
scopes summing to **185 units**, with per-scope lines identical between the two
runs.

Both binaries are built **in the worktree itself**, from the same manifest
directory, with the base sources checked out over the tip's and restored
afterwards (`git checkout f122cc03 -- crates compiler proof`, then
`git checkout HEAD -- …`, both verified with `git status`). That is the wave-25
route detail: a `git archive` of the base has no submodule, so the libevent pin
reader resolves `third_party/libevent` against that tree's root and reads the
enclosing repository's HEAD, voiding the run.

| binary | SHA-256 |
|---|---|
| `measure/bin/before` (base `f122cc03`) | `9564846fc7bc3e645c93da69c2ee7ce715d328d0a64341db0d582c6a7e66ea2a` |
| `measure/bin/after` (card tip) | `2ea0d4b875a5c2928d1fb87c398ddb4e854fb0e67da8d8f20bb0c78f97ef1fc0` |

### 5.2 The BEFORE run reproduces the baseline

The authoritative baseline is card AOM-BINDING-POISON's archive — the run the
tip's leak pin of 266 and its 73-module contract corpus describe.
`docs/reports/wave23-c-operations/` remains the frozen corpus baseline
underneath it and is **not touched** (standing constraint 21); neither is
`aom-binding-poison`'s.

[`byte_identity.py`](byte_identity.py),
[`logs/byte-identity-before.txt`](logs/byte-identity-before.txt):

| artifact | archived baseline | BEFORE run |
|---|---|---|
| `diagnostics.json` | `6fd12d24b8381bac…` | **identical** |
| `chunk-contracts.json` | `c4f438e4ed3dab3c…` | **identical** |
| `source.json`, `source-effects.json`, `contracts.json` | — | differ: the checkout-root and identity-digest class |

and `diff_artifacts.py --archived docs/reports/aom-binding-poison/artifacts`
([`logs/diff-summary-before.txt`](logs/diff-summary-before.txt),
[`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz))
itemises **zero** differences in every section that carries a verdict:

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 18/18, manifest 18/18, audit_selection 5/5
--- diagnostics:     units 185/185, +0 -0 ~0
--- source:          envelope key `compile_entry`
--- source-effects:  3 result rows changed
```

The residues are the checkout-root and identity-digest classes every prior
re-pin has recorded — this worktree's absolute path differs from wave 25's —
and this card can say so exactly: `source.json`, `source-effects.json` and
`contracts.json` are **byte-identical between this card's own two runs**
(`artifact-manifest.json`).

### 5.3 BEFORE → AFTER: presentation only, itemised

[`logs/diff-summary-before-to-after.txt`](logs/diff-summary-before-to-after.txt),
[`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz):

```
--- chunk-contracts: results 96/96 +0 -0 ~0;  backend_teardown 21/21 +0 -0 ~0
--- contracts:       sources 18/18, manifest 18/18, audit_selection 5/5
--- diagnostics:     units 185/185, +0 -0 ~178;  envelope `line_coordinates`
--- source:          (no differences)
--- source-effects:  results 96/96 +0 -0 ~0
```

Four of the five artifacts are **byte-identical**. The standard itemizer says
"178 unit rows changed" without saying what inside them moved, so this card
carries its own oracle, [`leaf_identity.py`](leaf_identity.py), which compares
every claim directly and permits exactly four kinds of difference. It reports
**zero STOP findings** ([`logs/leaf-identity.txt`](logs/leaf-identity.txt)):

```json
{ "units": 185, "functions": 4834, "diagnostics": 54383,
  "origin_present": 54371, "origin_null": 12,
  "descriptions_changed": 266, "descriptions_unchanged": 54117,
  "origin_coordinates_established": 178, "origin_coordinates_refused": 0,
  "origin_coordinates_null": 7, "withdrawn_lines": 0,
  "sites_presented": 266, "sites_without_a_sibling_row": 266 }
```

Read row by row:

* **185 units, 4,834 functions, 54,383 diagnostics**, every one with the same
  `(function, preprocessed line, category)`. The leaf set did not move; the
  leaves have new coordinates beside their unchanged internal positions.
* Every unit key except `functions` and `origin_coordinates` is byte-identical
  — `status`, `reason`, `reason_code`, `source_functions`, `not_projected`,
  `diagnostic_counts`, `enumeration`, `compile_configuration`. Every function
  row's `status` is unchanged.
* **266 descriptions changed**, which is exactly the `leak` count. Leaks are
  the only category present in this corpus whose text names a site (the other
  three site-bearing kinds — `double_release`, `release_after_escape`,
  `custody_conflict` — have no leaf here). Each rewrite is verified to be the
  *substitution of the site alone*: the text before and after the
  `preprocessed line N` phrase is byte-identical and what replaced it is a
  `file:line`.
* **12 rows carry `"origin": null`.** All twelve are at preprocessed line
  **0**, the synthetic position the envelope has always described as synthetic;
  there is no line to attribute. Their descriptions still name the
  *acquisition's* original coordinate, e.g.
  `ctraces/ctr_encode_opentelemetry.c:otel_span_set_links` moves from
  `…acquired at preprocessed line 9079` to
  `…acquired at ctr_encode_opentelemetry.c:778`.
* **178 units carry an established coordinate block, 0 refused.** The
  remaining **7** carry `null` because they publish no diagnostic to attribute
  and the derivation is not run for them: four data-only units
  (`ctr_utils.c`, and librdkafka's three `opentelemetry/*.pb.c`), two `Safe`
  units with no diagnostics (`ctr_version.c`, `tls/tls_stub.c`) and one
  `Unknown` unit with no projected function (`tinycthread.c`).
* **0 withdrawn lines** across the corpus (§2.2).

### 5.4 The envelope

`line_coordinates` is rewritten to say that `line` is the internal identity and
`origin` the presented coordinate, and one key, `origin_meaning`, is added
carrying the derivation and the anti-forgery rule. Those are the only two
envelope keys that move; `schema_version`, `scope`, `fluent_bit_revision`,
`not_projected_meaning`, `enumeration_meaning`, `reason_code_meaning` and
`diagnostic_counts` are byte-identical.

---

## 6. Gate

| step | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,236 passed, 0 failed, 8 ignored** across 212 test binaries, exit 0 ([`logs/workspace-tests.txt.gz`](logs/workspace-tests.txt.gz)). The wave-26 close measured 4,209; this card adds exactly its 27 new rows. |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **111 Lean audits passed**, exit 0 ([`logs/lean-audits-summary.txt`](logs/lean-audits-summary.txt)). 110 + this card's `AuditSourceCoordinate`. |
| `python3 -m unittest discover -s scripts/tests` | **192 passed, 1 failed** — the acceptance-matrix tripwire, §6.1 ([`logs/python-tests.txt`](logs/python-tests.txt)) |

No constraint-16 row was tripped: all five load-sensitive provider/host rows
passed inside the single `--no-fail-fast` workspace run, so none needed
isolation. `--no-fail-fast` is itself part of the gate as of the wave-26 close
(`GATE-NO-FAIL-FAST`), and the 212-binary count is the evidence that the run
reached every crate rather than stopping at the first failing target.

### 6.1 The one Python failure is a tripwire firing by design, and it is the integrator's to clear

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
asserts that the **committed** acceptance matrix agrees with the regenerated
one on two derived facts, one of which is the Lean audit count. `proof/` now
holds **111** `Audit*.lean` files; `docs/reviews/r0_3/acceptance_matrix.json`
records **110**:

```
AssertionError: Lists differ: ['14.0.0', 110] != ['14.0.0', 111]
```

That is the check doing its job — it exists so a card that adds an audit cannot
leave the matrix silently stale. Clearing it means regenerating the matrix,
which is **integrator work this card may not do** (constraint 25:
whole-file regeneration must not clobber later reconciliations, and the section
ownership block is the integrator's). Wave 25 recorded the identical shape at
103 → 104. The other **192** Python tests pass.

Provider code and packages are unchanged; the ledger's 122 provider tests are
carried from the preceding checkpoint, not rerun for this card. The five
packages were **built** for the audit route (the pinned route selects the
ChunkIO package through `QCORE_CHUNKIO_PACKAGE`), but no template and no
`qcore-schemas` version moved, so constraint 18's rebuild-and-verify obligation
is not engaged by a change of this card's.

New tests: 15 reader-level rows in
`crates/qcore-c-gateway/tests/unit/legacy_location.rs`, 9 Clang-driven rows in
`crates/qcore-c-gateway/tests/gateway_c_location.rs`, and 3 report-level rows
in `crates/qcore-audit/src/report.rs`.

---

## 7. Lean

`QCore.SourceCoordinate` (audit `AuditSourceCoordinate`) models the
presentation reader beside `QCore.EnumerationAttribution`'s scope reader, over
the same marker stream. Its centre is the **anti-forgery theorem**, proved in
general rather than on witnesses:

> `presented_files_are_entered_files` — every file spelling a run presents was
> written by a marker that **entered** a file, or was already presentable on the
> reader's stack when the run began.

with the corollary `a_flagless_marker_names_no_presented_file` at the reader a
real run starts from. It rests on three lemmas that are each exactly one
clause of §2: `step_adds_only_an_entering_spelling` (a spelling becomes
presentable only at an entering marker — including the *opening* marker, whose
frame presents `primary` and therefore contributes no spelling at all),
`content_keeps_the_stack_names`, and `content_presents_a_stack_name`.

Beside it: the two measured `#line` directions as decidable witnesses
(`a_relabelled_header_presents_nothing`,
`a_relabelled_tail_stays_in_scope_and_presents_nothing` — which states both
halves in one sentence, the scope unmoved and the coordinate withdrawn), the
agreeing-resync non-vacuity (`an_agreeing_resync_sets_the_line`,
`the_name_comparison_is_load_bearing`), the per-content-line advance that makes
a macro expansion present its site (`content_lines_advance_by_one`), the
fail-closed arms, and §L21.1 rule 1 over all four refusal grounds.

**A note on the ground.** This card's task instruction says Lean is required
only if a semantic surface moves, and an attribution presentation moves none:
no verdict, status, certificate, obligation, contract or emitted byte changes,
and `QCore.EnumerationAttribution` — which owns the scope rule — is untouched.
The module was written anyway, because the card's *claim* is a
non-forgeability property and that is exactly the kind of statement a
decidable witness cannot establish: the witnesses say three streams behave, the
theorem says every stream does.

What is **not** proved, and stays a tool boundary exactly as it does for
`EnumerationAttribution`: that Clang emits flag `1` on entering a file, flag
`2` on returning, a flagless marker for `#line`, and an opening marker naming
the primary input. Those are observations of the tool, pinned behaviourally by
`gateway_c_location.rs`, which drives real preprocessings with real `#line`
directives in both positions.

---

## 8. Boundaries and named follow-ons

| name | statement |
|---|---|
| `GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE` | a `#line` with no file spelling renumbers the file it is written in, and this reader cannot tell it from one of Clang's own resynchronisations. It moves no file identity, no include depth and no audited scope; it is the directive's C11 §6.10.4 meaning and what every C compiler reports. §2.1. |
| `GATEWAY-LOCATION-CONTRACT-TEXT` | contract-text locations are **out of scope by the user's ruling for this card**. A `.qcore` module's own line numbers are not touched and no contract diagnostic changed. |
| `GATEWAY-LOCATION-INTERNAL-CHANNELS` | the preprocessed position remains the sole coordinate of the certificate, `source-effects.json`, `chunk-contracts.json`, the census APIs and the probe examples (`alias_aggregate_probe` still prints `L19544`). That is the user's ruling — the L-numbers stay internal for pin identity, leaf identity and certificate comparability — not an omission. |
| `GATEWAY-LOCATION-INPUT-IDENTITY-REFUSAL` | two unit-level refusal texts in `legacy_c.rs` (`reject_filename_sensitive_constructs`) and `LegacyCError::MalformedFunction` still name a preprocessed/input line. They fire *before* any analysis, where no map exists and no unit row is published; `MalformedFunction`'s text is additionally pinned by `qcore-schemas`'s `reason_codes` test. A follow-on could attribute them. |
| `GATEWAY-LOCATION-SYNTHETIC-EXIT` | 12 leak rows sit at the synthetic preprocessed line 0 and therefore carry `"origin": null`. Their text still names the acquisition's original coordinate. Giving a synthetic exit a coordinate would mean choosing one, which this card will not do. |
| `GATEWAY-LOCATION-UNSAFE-REPREPROCESS` | `UnrestrictedLegacyResult::Unsafe` carries no preprocessed text, so that arm re-runs `validate_unrestricted_c11` to obtain the authoritative bytes rather than accept a text derived another way. Adding the field to the variant would touch 37 match sites across the workspace; a follow-on could do it deliberately. |
| `GATEWAY-LOCATION-SECOND-PREPROCESS` | the map costs one extra marked `-E` run per unit with diagnostics, on top of the enumeration's own. The enumeration already holds that output and could hand it over; it does not today, because changing `enumerate_source_definitions`' shape touches the pinned denominator route. |
| `GATEWAY-LOCATION-HEADER-REACH` | the header arm (a coordinate at include depth ≥ 1) is exercised only by a fixture: the pinned corpus's `MainSourceDefinitions` scope puts all 54,371 presented coordinates at depth 0. A route analysing header bodies would exercise it in production. |

---

## 9. What was touched

* `crates/qcore-c-gateway/src/legacy_location.rs` — new.
* `crates/qcore-c-gateway/src/legacy_c.rs` — module declaration and
  re-exports.
* `crates/qcore-c-gateway/src/legacy_enumeration.rs` — `line_marker`,
  `LineMarker` and `preprocess_with_line_markers` made `pub(crate)`;
  `LineMarker` gained the `number` field it already parsed. An unparseable
  digit run reads `0` rather than rejecting the marker, so this module's
  acceptance is exactly what it was.
* `crates/qcore-audit/src/{projection,report}.rs` — build the map, present the
  coordinate, publish `origin` and `origin_coordinates`.
* `crates/qcore-c-gateway/tests/{gateway_c_location.rs,unit/legacy_location.rs}` — new.
* `proof/QCore/SourceCoordinate.lean`, `proof/AuditSourceCoordinate.lean` — new;
  `proof/QCore.lean` — one import.
* `docs/reports/gateway-c-location/` — this report and its evidence.

Nothing outside those: no `AGENTS.md`, `CLAUDE.md`, `README.md`,
`DEVELOPMENT.md` or acceptance-matrix edit, no `examples/`, no source-language
pass, no provider.

---

## 10. For the integrator

The corpus baseline moves to `docs/reports/gateway-c-location/artifacts/`, and
the move is **presentation-only**: the ledger's corpus row — 185 units
2/42/137/4, 4,834 functions 150/84/4,600, leak **266**, `unknown_alias`
**4,344**, `unsupported` **49,773** — is unchanged in every figure, and the
preceding archives stay frozen. The one thing a reader of the new artifact must
know is that a diagnostic row now carries two coordinates: `line`, the internal
preprocessed position that every pinned comparison is keyed on and that has not
moved, and `origin`, the original `file:line` the text presents.
