# E01-F1 — Complete the source function enumeration

> **Superseded as the current baseline, 2026-09-09 (wave-9 consolidation):**
> the wave-9 integrator measurement on the fully merged tree —
> `docs/reports/wave9-corpus/artifacts/` — is the current baseline, absorbing
> E01-F1-RAW's enumeration repair, O04-B08's fail-closed diagnostic movement
> and REASON-CODES-B3's sidecar field at unchanged headline counts. This
> directory remains the E01-F1 measurement record.

2026-09-09. Branch base `8d0790a`. Workstream W3/W2, follow-on unit of
[`docs/reports/remaining-e01/README.md`](../remaining-e01/README.md).

`docs/reports/remaining-e01/README.md` recorded boundary **E01-B1**: the audited
function inventory of a translation unit was a raw token scan of the
*unpreprocessed* main source (`legacy_c_source_functions`), so two real
definition shapes escaped it **entirely** — no obligation *and no residual
`Unknown` row*:

* a definition produced by macro expansion, whose text exists only after
  preprocessing;
* a K&R definition, whose parameters are an identifier list followed by
  separate declarations before the body.

The reported "N checked / M Unknown" split was therefore not a complete function
denominator for such a unit. This unit repairs that, and — see §2.2 — found and
repaired a third, previously unrecorded, cause of the same loss.

**What this unit is.** It completes the *denominator*, not the analyzer. A newly
enumerated definition is analysed only where the **unmodified** analyzer already
reads it — a definition that is ordinary once expanded is; a K&R definition is
not, and receives an explicit `Unknown` row with a classified `reason_code`
instead. No analysis rule, no diagnostic code and no `qcore-schemas` type
changed.

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe /
137 Unknown plus four data-only units, the contract corpus stays 60 modules, CT
coverage stays 24/50 mapped, and full language conformance stays false. The
re-pin of the archived artifacts is the integrator's decision, not this unit's;
the before/after set and full diffs are archived under `artifacts/`.

**Headline.** On the pinned 185-unit corpus the function denominator grows
**4,775 → 4,921 (+146)**, all 146 of them projected; **no unit changed verdict
in either direction**, `leak` diagnostics are unchanged at 271, and no unit
refuses its completeness claim. All 146 are in a single file,
`librdkafka/src/rdkafka.c`, which reported 19 of its 165 definitions — a third
cause of the same loss, found while implementing this unit and described in
§2.2. The configured `libevent-core` audit gains 45 definitions across three
units with no verdict change; `cmetrics-counter` and `chunkio-core` are
byte-identical on stdout. Zero K&R definitions exist in the pinned corpus, so
that half is proved on held-out fixtures and Lean with **measured corpus reach
0**.

---

## 1. Enumeration semantics

### 1.1 Why a second preprocessing

Whether a macro invocation produces a function definition is decided by the
preprocessor, not by a token scan. The gateway already preprocesses each unit,
but with `-E -P`: **line markers are suppressed**, and that output's bytes are
what every certificate, compile-task identity, diagnostic line number and task
digest in this repository is bound to. It cannot be changed.

So the enumeration reads a *second, enumeration-only* preprocessing of the same
unit, built from the same command minus `-P` and minus the syntax phase
(`crates/qcore-c-gateway/src/legacy_enumeration.rs`,
`preprocess_with_line_markers`). Its output is never published, never digested
and never analysed. Every definition in it is attributed to the file its line
marker names, and the primary input is read from the first marker Clang itself
emits — not synthesised from the declared `CSourceInput`, so a `<stdin>` unit and
a declared-file unit are each attributed by the tool's own account of what it
read.

Consequences, all measured in
`crates/qcore-c-gateway/tests/e01_source_enumeration.rs`:

| Definition site | In scope? | Why |
|---|---|---|
| written in the main `.c` | yes | unchanged |
| macro invoked in the main `.c`, macro defined anywhere | **yes (new)** | the expansion is attributed to the invoking main-source line |
| written in a header | no | boundary **E01-B2**, unchanged |
| macro invoked in a header | no | the expansion is attributed to the header |
| removed by `#if 0` | yes | kept by the union with the raw scan (§1.4) |

### 1.2 The file-scope grammar

At brace depth 0, for a name followed by `(` whose `)` is found:

| Shape | Decision |
|---|---|
| `f(…) { … }` | **prototype definition** — counted, and read by the unmodified analyzer |
| `f(a, b) int a; int b; { … }` | **identifier-list (K&R) definition** — counted, not read by the resource analyzer |
| `f(…) ;` | declaration — not counted |
| `f(…)` followed by anything else, with a typed parameter list | not a definition (function-pointer declarator, attribute, initialiser) |
| `f(a, b)` in an *expression* context (`sizeof(a), sizeof(b)`) | not a definition — see the guards below |
| `T (name)(args)` — a `(` or `[` right after `)`, or a declaration specifier in the name position | not a definition — see the guards below |
| `f(a, b)` in a declaration context, followed by a declarator run this scanner cannot read | **refusal** (§1.3) |
| `(` with no matching `)` | **refusal** (§1.3) |

A K&R declarator run is a nonempty token run free of `(`, `)`, `{` and `}`
terminated by `;`, repeated until `{`. That is what separates
`int f(a, b) int a; int b; { … }` (a definition) from `int f(a, b);` (an
old-style *declaration*, whose parameter list is also an identifier list): the
terminator is read first, and an empty declaration run refuses rather than
guesses.

**Three guards on the K&R arm, each added from a measurement.** An identifier
list is not a distinctive shape: `sizeof(a), sizeof(b)` inside a file-scope
initialiser table, and `typedef T (name)(args);`, both have exactly the token
shape of `f(a, b) int a; { … }`. Entering the K&R arm from those contexts
refused the completeness claim across ordinary C. Two probe runs over the pinned
corpus found every case and each guard is the narrowest rule that answers it:

| Probe | units refusing | construct | guard added |
|---|---|---|---|
| unguarded | 6 | `librdkafka/opentelemetry/{common,metrics,resource}.pb.c` (nanopb field tables), `monkey-libevent/mk_server/mk_header.c` line 127 `(sizeof(status_response)/(sizeof(status_response[0])))` | **declaration context**: the K&R arm is entered only when the token before the name can end a definition's declaration specifiers — nothing, an identifier, `*`, or the `;`/`}` that ended the previous declaration; never after `,`, `{`, `=`, `(`, `[` or an operator |
| + declaration-context guard | 2 | `librdkafka/rdkafka_admin.c` — three function typedefs `typedef rd_kafka_resp_err_t(rd_kafka_admin_Request_cb_t)(…);`; `librdkafka/rdkafka_partition.c` — function-pointer parameters `unsigned int (hash)(const void *_a)` | **declarator suffix**: `(` or `[` immediately after `)` is a declarator continuation, and a K&R declaration list can never begin with either; and **specifier in the name position**: `int` in `unsigned int (hash)(…)` is never a function name |
| + both declarator guards | **0** | — | — |

None of the three guards touches the prototype arm, so `function_names`'
historical behaviour is byte-identical. And none is a blanket exemption: a real
declarator run this scanner cannot read, in a declaration context, still refuses
(`the_expression_guard_does_not_hide_a_real_unreadable_declarator`), and all
three directions are pinned in Rust and in Lean.

### 1.3 Fail-closed: refusing the completeness claim

The inventory carries an explicit completeness claim. Four mechanical grounds
refuse it, each with a free-text reason that classifies through the closed
§L21.1 vocabulary:

| Ground | `reason_code` | Subject |
|---|---|---|
| `expansion-unavailable` — the enumeration-only preprocessing did not run | `environment-unavailable` | environment |
| `primary-input-unidentified` — the marked output does not open with a line marker | `environment-unavailable` | environment |
| `unbalanced-delimiters` — a file-scope `(` has no match | `unsupported-syntax` | analysed source |
| `unclassified-file-scope-construct` — a declaration-context `name(identifier-list)` followed by neither a body, a `;`, nor a readable declarator run | `unsupported-syntax` | analysed source |

The two grounds that name a *tool* failure are typed as environment, so no
failure of the analysis machine can be read as a statement about the analysed
source (§L21.1 rule 1). No ground ever yields `Unsafe`.

A refused claim withholds the whole-unit `Safe` verdict
(`SourceDefinitionInventory::supports_unit_safe`), and so does any enumerated
definition the resource analyzer does not read. This is applied in the
*projection* layer (`crates/qcore-audit/src/projection.rs`), not in the
analyzer: the per-function rows and their diagnostics are untouched, and only
the unit-level claim is withheld, with its ground in the unit's `reason`.

### 1.4 Monotonicity

`SourceDefinitionInventory::definitions` **unions in** the historical raw scan
rather than replacing it. This is load-bearing: a definition guarded out by
`#if 0` is in the raw scan and absent from the expansion, and dropping it would
shrink a denominator — which is exactly how a unit could gain a `Safe` verdict
by losing a row. The union makes that impossible by construction, and the Lean
module proves it (§3).

`legacy_c_source_functions` is deliberately unchanged. It is the monotonicity
base, not the audited denominator.

### 1.5 Where the inventory is consumed

One enumeration per unit, computed in the driver and handed to every consumer,
so two computations of the denominator cannot disagree:

| Consumer | Change |
|---|---|
| `qcore_audit::run_config` | enumerates once; passes the inventory to the projection *and* the source-contract route |
| `qcore_audit::projection::project_unit` | denominator, analyzer selection, `enumeration` report block, withheld-`Safe` rule |
| `qcore_audit::source_audit::analyze_unit` | backstop denominator; an unanalyzable definition reports *its own* reason, not "absent from the typed import" |
| `qcore_audit::current` (checkout-head route) | same, with the raw scan retained as the fallback for a configuration that never became a unit |
| `qcore_c_gateway::boolean_source` | the typed importer iterates the enumerated set |
| `analyze_unrestricted_c11` | new `_with_inventory` entry point; the no-inventory form enumerates for itself, so every existing caller keeps working |
| `compiler/examples/support/cycle19_compatibility.rs` | the pinned Cycle 19 route reports the same denominator |

The report gains one per-unit `enumeration` block and one header key,
`enumeration_meaning`. `status`, `reason`, `reason_code`, `source_functions`,
`functions`, `not_projected` and `diagnostic_counts` keep their exact meaning.

---

## 2. Corpus re-measurement

### 2.1 The pinned 185-unit corpus

Re-measured through the canonical pinned route on freshly prepared pinned clones
(Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 --clang clang --clangxx clang++
cp <build>/compile_commands.json <clone>/build/compile_commands.json
python3 -B providers/package.py build --output build/cycle19/provider-packages-e01f1
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-e01f1/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Run twice: once from a clean worktree at this branch's base `8d0790a`
(**before**), once from this worktree (**after**). Both exit 0 with zero gateway
failures. The *after* run was directed at a private report directory after a
concurrent audit process was found writing into the shared
`build/cycle19/clang/<commit>/reports/`; the report path is an argument and
does not appear in any artifact's bytes (verified by grep), so the before run
— whose `diagnostics.json` is byte-identical to the archived pin — needed no
re-run. Its transcript therefore names a `/tmp` report path.

**Harness validation.** The *before* run reproduces
[`docs/reports/remaining-o04-arms/artifacts/`](../remaining-o04-arms/artifacts)
exactly where analysis is concerned: `resource-diagnostics.json` and
`resource-diagnostics.chunk-contracts.json` are **byte-identical** (SHA-256
`d97626fc…` and `29122661…`, matching that manifest's
`uncompressed_sha256`), and the exhaustive
`docs/reports/corpus-repin/diff_artifacts.py` run reports **0 added, 0 removed,
0 changed** rows in `units`, `results` and `backend_teardown`.
`monkey-source.json`, `resource-diagnostics.source-effects.json` and
`resource-diagnostics.contracts.json` differ only in absolute-path and
`build_digest`-derived identity fields — the same location-provenance class
O04-ARMS and CORPUS-REPIN each recorded, present *before* any change of mine.
That is what makes the before/after diff an isolation of this unit.

**Headline, 185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only, before and after. No unit changed verdict in either direction, and
no unit changed class** (no data-only unit gained a definition and left that
class).

| | before | after |
|---|---|---|
| units | 185 | 185 |
| enumerated source functions | 4,775 | **4,921** (+146) |
| projected functions | 4,688 | **4,834** (+146) |
| `Safe` / provisional `Unsafe` / `Unknown` / data-only units | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 |
| `leak` diagnostics | 271 | **271** |
| `unknown_alias` diagnostics | 4,167 | 4,286 (+119) |
| `unsupported` diagnostics | 48,312 | 50,089 (+1,777) |
| definitions counted but unread (`unanalyzable`) | — | **0** |
| units refusing the completeness claim | — | **0** |

**No unit moved to provisional `Unsafe`.** The `leak` count is unchanged at 271,
so the 146 newly enumerated functions surfaced no residual-leak evidence; every
new diagnostic is in the fail-closed direction (`unsupported`, `unknown_alias`).
Had a newly enumerated definition surfaced a real leak, the unit would have
moved and this section would name it; none did. Movement *to* `Safe` is
impossible by construction — this unit only adds rows and withholds claims — and
none occurred.

**Isolation, from the exhaustive before→after diff.** Of the five artifacts,
**four are byte-identical**: `chunk-contracts.json`, `contracts.json`,
`monkey-source.json` and `source-effects.json`. In the fifth,
`resource-diagnostics.json`, all 185 units carry the new additive `enumeration`
block and **exactly one unit changes anything else** —
`librdkafka/rdkafka.c`, in `source_functions`, `functions`, `reason` and
`diagnostic_counts`. No unit was added or removed.

Per library:

| Library | units | source functions before → after | new | refused |
|---|---|---|---|---|
| cmetrics | 31 | 540 → 540 | 0 | 0 |
| ctraces | 17 | 274 → 274 | 0 | 0 |
| librdkafka | 94 | 3,410 → **3,556** | 146 | 0 |
| chunkio | 13 | 126 → 126 | 0 | 0 |
| fluent-bit-chunk | 1 | 66 → 66 | 0 | 0 |
| monkey-libevent | 29 | 359 → 359 | 0 | 0 |

The `chunk-contracts.json` and `source-effects.json` sidecars keep their
3 checked / 93 Unknown split: no ChunkIO unit gained a definition.

**Zero K&R definitions exist in the pinned corpus.** The K&R half of this unit
is therefore proved on held-out fixtures and Lean, with measured corpus reach 0
— stated plainly rather than implied, in the same spirit as C03-NFR's measured
reach.

### 2.2 The third cause, found by implementing this unit

All 146 new definitions are in **one file**, `librdkafka/src/rdkafka.c`, which
reported **19** of its **165** definitions. They are not macro-produced: they are
written literally in the main source. The cause is a defect in the raw scan
itself.

`tokenize` treats every `#` as a directive to end of line. `rdkafka.c` line 426
is

```c
#define _ERR_DESC(ENUM, DESC)                                                  \
        [ENUM - RD_KAFKA_RESP_ERR__BEGIN] = {ENUM, &(#ENUM)[18] /*pfx*/, DESC}
```

The `#define` line ends at its backslash-newline, so the *continuation* line is
tokenized as ordinary source. Its `#` — the stringification operator in
`&(#ENUM)[18]` — is read as a directive, and everything after it on that line,
**including the closing `}`**, is discarded. The scan's file-scope brace depth
rises to 1 and never returns to 0 (it ends at 2), so every definition after line
429 is invisible to it. `rd_kafka_destroy`, `rd_kafka_consumer_poll`,
`rd_kafka_flush` and 143 others carried **no row at all** in the audited
denominator — 3.1% of the whole corpus's function denominator, in one file.

This is a *third* instance of the same loss E01-B1 named, and it is repaired for
the same reason: the enumeration reads text the preprocessor has already
resolved, where no `#` remains. The reduced reproduction is
`a_raw_scan_brace_desynchronisation_no_longer_hides_definitions`. The inventory
field is named `absent_from_raw_scan`, not "macro-expanded", precisely because
it holds both causes and does not distinguish them.

`legacy_c_source_functions` is **not** repaired: it keeps its exact behaviour as
the monotonicity base. Repairing the raw tokenizer's `#` handling would move
every consumer of it at once, and is recorded as a named follow-on (§6).

### 2.3 The three configured audits

Run before (at `8d0790a`) and after, from the same prepared inputs. Transcripts
under [`logs/`](logs).

| Audit | before | after | delta |
|---|---|---|---|
| `cmetrics-counter` | 1 unit, 0 Safe / 1 provisional Unsafe / 0 Unknown; source contracts 1 checked / 6 Unknown | **identical** | stdout **byte-identical**; report JSON differs only by the added `enumeration_meaning` key and per-unit `enumeration` block; `cmetrics-audit.assumptions.json` **byte-identical** |
| `chunkio-core` | 13 units, 0 Safe / 2 provisional Unsafe / 11 Unknown; 126 source functions | **identical** | stdout **byte-identical**; report JSON differs only by the two new report keys |
| `libevent-core` | 21 units, 0 Safe / 4 provisional Unsafe / 17 Unknown; 684 source functions | 21 units, 0 Safe / 4 provisional Unsafe / 17 Unknown; **729 source functions (+45)** | no verdict change; `unknown_alias` 344 → 362, `unsupported` 4,940 → 5,121 |

`libevent-core`'s +45 are in three units and are genuine main-source
expansions — libevent's `HT_GENERATE`/`HT_PROTOTYPE` hash-table macros and its
`EVUTIL_IS*_` character-class macros:

| Unit | verdict | source functions | examples of the new rows |
|---|---|---|---|
| `event.c` | Unknown → Unknown | 174 → 187 | `event_debug_map_HT_CLEAR`, `event_debug_map_HT_FIND`, `event_debug_map_HT_GROW` |
| `evmap.c` | Unknown → Unknown | 17 → 41 | `delete_all_in_dlist`, `event_changelist_add_`, `event_changelist_assert_ok` |
| `evutil.c` | provisional Unsafe → provisional Unsafe | 92 → 100 | `EVUTIL_ISALNUM_`, `EVUTIL_ISDIGIT_`, `EVUTIL_ISSPACE_` |

`evutil.c` was already provisional `Unsafe`; its eight new rows added no leak
diagnostic and did not change that.

Across all four measured source sets — the 185-unit corpus, `cmetrics-counter`,
`chunkio-core` and `libevent-core`, 220 units and 5,783 enumerated definitions —
there are **zero K&R definitions and zero completeness refusals**. The
denominator grows by 191 definitions in four files.

### 2.4 Archived artifacts

[`artifacts/`](artifacts) holds the gzipped before and after artifact sets with a
`manifest.json` of uncompressed sizes and SHA-256 digests, the exhaustive
`diff_artifacts.py` documents for archived→before (harness validation) and
before→after (this unit's effect), and `census.json` — the per-library
denominator delta, every added row, every refusal ground and every unit verdict
move, produced by the read-only [`census.py`](census.py) in this directory.
Transcripts of all four audit runs are under [`logs/`](logs).

| Artifact | before SHA-256 | after SHA-256 | |
|---|---|---|---|
| `chunk-contracts.json` | `29122661…` | `29122661…` | identical, and identical to the archived pin |
| `contracts.json` | `e6c7266d…` | `e6c7266d…` | identical |
| `monkey-source.json` | `5b5c3c3a…` | `5b5c3c3a…` | identical |
| `source-effects.json` | `b01a4d33…` | `b01a4d33…` | identical |
| `diagnostics.json` | `d97626fc…` | `f4431c2c…` | changed; `d97626fc…` is the archived pin's digest |

**The re-pin is the integrator's decision.** `docs/reports/remaining-o04-arms/`
is untouched.

---

## 3. Lean

`proof/QCore/SourceEnumeration.lean`, audited by
`proof/AuditSourceEnumeration.lean`: **38 theorems**, every one depending on at
most `propext`; no new axiom, no placeholder. `scripts/check_lean_audits.py`
moves 42 → 43 audits.

What is proved:

* **monotonicity** — `raw_is_kept`, `prototype_is_kept`,
  `identifier_list_is_kept`, `unanalyzable_is_counted`: every input class lands
  in the denominator, and the historical raw scan is a subset of it;
* **honest accounting** — `the_denominator_is_exactly_its_two_classes`,
  `unanalyzable_is_not_analysed`, `absent_from_raw_scan_is_new`,
  `absent_from_raw_scan_is_analysed`: the denominator is exactly the analysed
  names plus the unanalyzable ones, the classes are disjoint, and the growth is
  real growth read by the unmodified analyzer;
* **fail-closed unit `Safe`** — `a_refused_inventory_cannot_support_safe`,
  `an_unread_definition_cannot_support_safe`, and the `iff`
  `safe_is_admitted_exactly_when_established_and_read`, so the rule adds no
  refusal beyond the two named ones;
* **refusals never manufacture a verdict** — `a_refusal_never_yields_unsafe`,
  `a_refusal_never_yields_safe`, plus §L21.1 rule 1 over this unit's ground
  vocabulary (`an_environment_ground_says_nothing_about_the_source`,
  `a_token_stream_ground_names_the_analysed_source`,
  `no_ground_is_both_environment_and_source`, `both_subjects_are_reached`);
* **the file-scope grammar over the adversarial corpus** — a prototype
  definition, a prototype declaration, an old-style declaration, a K&R
  definition, a pointer declarator list, a typed parameter list, an unreadable
  declarator, an unbalanced construct, a non-parameter-list run, and all three
  guards of §1.2 in both directions
  (`an_expression_context_is_neither_counted_nor_ambiguous`,
  `a_comma_context_is_neither_counted_nor_ambiguous`,
  `a_function_typedef_declarator_is_not_a_definition`,
  `a_specifier_in_the_name_position_is_not_a_definition`,
  `the_guard_does_not_hide_a_real_unreadable_declarator`,
  `the_declaration_contexts_are_exactly_these`);
* **non-vacuity** — the two E01-B1 witnesses grow the denominator strictly, the
  `#if 0` witness keeps its historical row, and the unavailable-expansion
  witness keeps the historical scan while refusing.

Explicit Rust/tool boundaries, stated in the module header and **not** proved
there: that Clang's line markers attribute expanded tokens to the invoking file;
that the gateway tokenizer produces the token lists transcribed in the corpora;
and that the analyzer's own body finder reads exactly the prototype-shaped
definitions. Those are pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_source_enumeration.rs`.

---

## 4. Validation

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1,771 passed, 0 failed, 3 ignored** (1,747 passed, 3 ignored at the base) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | 43 Lean audits passed (42 → 43) |
| `cargo test --manifest-path providers/Cargo.toml` | 58 passed, 0 failed |
| providers `cargo fmt` / `cargo clippy -D warnings` | clean |
| `python3 -B -m unittest discover -s scripts/tests` | 108 passed |
| pinned 185-unit audit, before and after | exit 0, 0 gateway failures |
| `cmetrics-counter` / `chunkio-core` / `libevent-core` configured audits | exit 0 (§2.3) |

New tests: **19** in `crates/qcore-c-gateway/tests/e01_source_enumeration.rs`,
plus six reason-corpus rows in `crates/qcore-schemas/tests/reason_codes.rs`.

---

## 5. Deliberate pin updates

Three, each stated with what it kept:

1. **`crates/qcore-c-gateway/tests/unit/legacy_c.rs`** —
   `main_source_scope_excludes_macro_expanded_header_bodies` asserted that a
   definition produced by expanding a macro *in the main source* was outside
   `MainSourceDefinitions` scope, and that the rest of the unit was therefore
   `Safe`. That is E01-B1 enshrined as a feature: the expansion happens at a
   main-source line, so the definition is a main-source definition, and
   excluding it bought a `Safe` verdict. The test is renamed
   `main_source_scope_excludes_header_bodies_but_not_main_source_expansions`
   and now asserts the expansion is enumerated, analysed and leaves the unit
   `Unknown`. **The property it was really protecting is kept**: a third leg
   moves the same helper into a real header, where boundary E01-B2 applies and
   `subject` is `Safe` again, certificate recheck included.
2. **`crates/qcore-verification/tests/source_effect_obligations.rs`** — the
   block that pinned E01-B1 (`assert!(inventory(&unit(&macro_defined), &[])
   .is_empty())`) now asserts the repair. Its load-bearing half is kept
   verbatim: `legacy_c_source_functions` is *still* asserted to miss both
   shapes, because that function is deliberately unchanged. `assert_inventory`'s
   cross-check moves from the raw scan to the enumerated denominator, which is a
   **strictly tighter** assertion (the enumeration is a superset).
   One capability-specific expectation is deliberately weakened by one step: a
   K&R definition is asserted to carry *a* row, not specifically an `Unknown`
   one, because that route reads Clang's typed AST rather than this token
   stream and does derive a contract for it. What E01-B1 forbade — no row at
   all — is what is asserted.
3. **`crates/qcore-schemas/src/reason.rs`** — five marker rows added to
   `REASON_RULES`, two in the environment section (matched first, so a tool
   failure can never reach a source-subject category) and three in the
   unsupported-syntax section. Six matching corpus rows added to
   `crates/qcore-schemas/tests/reason_codes.rs`. This is data, not schema:
   `qcore-schemas` stays **6.0.0**, no type changed and no wire byte moved.

Per-unit function counts in the pinned artifacts move as §2.1 records; the
archived artifacts themselves are **not** re-pinned here.

---

## 6. Open boundaries

* **E01-F1-RAW** — `legacy_c_source_functions`'s `#`-to-end-of-line rule is
  still wrong on a macro continuation line (§2.2). It is left wrong on purpose:
  it is this unit's monotonicity base, and repairing it would move every
  consumer of the raw scan at once — `legacy_summary`'s custody derivation, the
  typed field-access classifier and the certificate recheck all tokenize the
  same way. Repairing it is its own card, with its own parity plan.
* **E01-F1-STDIN** — the primary input is read from the first line marker. For
  the pinned Cycle 19 route that marker is `<stdin>`, so a unit transported
  through standard input attributes *everything Clang read from stdin* to the
  main source. That is correct here (the stdin text *is* the main source), but
  it means the header/main split for such a unit rests on the transport, not on
  a path. A declared-file unit does not have this dependence.
* **E01-F1-DENOM** — a `Safe` unit may still carry a non-empty `not_projected`
  when a definition the raw scan enumerated is absent from the expansion (an
  `#if 0` body). That is pre-existing behaviour, untouched here, and not this
  unit's business; the new withheld-`Safe` rule covers only definitions this
  unit newly enumerated and nothing read.
* **E01-F1-COST** — the enumeration costs one extra Clang `-E` per unit. The
  driver computes it once and shares it, but `analyze_unrestricted_c11`'s
  no-inventory form still enumerates for itself.
* **E01-B2, E01-B3, E01-B4** — unchanged, and still pinned by
  `crates/qcore-verification/tests/source_effect_obligations.rs`.
* **E01-F2, E01-F3, E01-F4** — the other capabilities' inventory audits are
  untouched by this unit.

Nothing here establishes that any audited source is safe, and no raw unit is
promoted.
