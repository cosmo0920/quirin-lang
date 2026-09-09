# E01-F1-RAW — Repair the raw tokenizer's macro-continuation directive defect

2026-09-09. Branch base `05c51a5`. Workstream W3/W2, follow-on unit of
[`docs/reports/remaining-e01-f1/README.md`](../remaining-e01-f1/README.md) §6.

E01-F1 completed the audited function *denominator* from a second,
enumeration-only preprocessing, and while doing so **diagnosed** a defect in the
mechanism it was measured against — the raw token scan
`legacy_c_source_functions`, via `qcore_c_gateway::legacy_c::tokenize`:

> `tokenize` treats every `#` as a directive to end of line. […] The `#define`
> line ends at its backslash-newline, so the *continuation* line is tokenized as
> ordinary source. Its `#` — the stringification operator in `&(#ENUM)[18]` — is
> read as a directive, and everything after it on that line, **including the
> closing `}`**, is discarded. […] every definition after line 429 is invisible
> to it.

E01-F1 deliberately did not repair it, because the raw scan is the base its
enumeration unions over and a change to it needs its own parity run
(E01-F1 §6, boundary **E01-F1-RAW**). This unit is that repair and that parity
run.

**Headline.** The tokenizer now implements C11's real rule. On the pinned
185-unit corpus the whole measured effect is **one unit, one field**:
`librdkafka/rdkafka.c`'s `enumeration.absent_from_raw_scan` shrinks **146 → 15**.
Four of the five artifacts are byte-identical before and after; the fifth changes
in exactly 1 of 185 units and in no other field. **No verdict moved in either
direction**, at unit or function level; the denominator is unchanged at 4,921;
`leak` diagnostics are unchanged at 271; **no unit newly became provisional
`Unsafe`**, and none left `Unsafe` either. All three configured audits
(`cmetrics-counter`, `chunkio-core`, `libevent-core`) are byte-identical on
stdout *and* report JSON.

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe /
137 Unknown plus four data-only units, the contract corpus stays 60 modules, CT
coverage stays 28/50 mapped, and full language conformance stays false. The
re-pin of the archived artifacts is the integrator's decision; the before/after
sets and both exhaustive diffs are archived under [`artifacts/`](artifacts).

---

## 1. The lexical rule

### 1.1 What was wrong

Two separate errors, both in the `#` branch of `tokenize`:

1. **Every** `#` was read as a directive introducer, wherever it stood. C11 6.10
   reads a directive only when `#` is the *first preprocessing token of a
   logical line*. A `#` in a macro replacement list is the stringification
   operator, and `##` is the pasting operator; neither introduces anything.
2. A directive ran to the end of the **physical** line. C11 5.1.1.2 phase 2
   deletes each backslash-newline pair *before* any preprocessing token is
   formed, so a `#define` continued with `\` is one logical line and the whole
   replacement list is directive text.

Together these produced the measured failure: the continuation line of
`_ERR_DESC` was tokenized as ordinary file-scope source (error 2), and its
stringification `#` then discarded the rest of that line including the closing
brace (error 1). The scan's file-scope brace depth rose and never returned to
zero.

### 1.2 What is implemented

`crates/qcore-c-gateway/src/legacy_c.rs`:

| Rule | Implementation |
|---|---|
| **Phase 2 splicing** | `line_splice_width` admits exactly `\`+`\n` and `\`+`\r\n`. A splice is consumed and the physical line number advances, but line-initiality is **not** restored: the logical line continues. A backslash followed by anything else stays an ordinary punctuator token, so a source relying on a compiler extension keeps its existing tokenization rather than silently acquiring a different one. |
| **Directive start** | A new `at_logical_line_start` flag is true exactly while nothing but white space and comments has been seen since the last *undeleted* new-line. A `#` is a directive introducer iff that flag is set. |
| **Directive extent** | A directive is consumed to the end of its **logical** line, splices included. It emits no token. |
| **A non-line-initial `#`** | Falls through to the ordinary single-byte punctuator branch: `#` becomes a `#` token and `##` becomes two of them. Nothing after it on the line is discarded. |
| **`//` comments** | White space that runs to the end of its *logical* line, so a spliced continuation stays commented (phase 2 precedes phase 3). |
| **Block comments** | White space. A new-line inside one is a real new-line, so it *sets* line-initiality — `/* … \n … */ #define X` is a directive, and `int x; /* … */ # y` is not. |

`crates/qcore-c-gateway/src/legacy_enumeration.rs`'s `tokenize_marked` — the
copy of this lexer that additionally reads Clang's `# <line> "<file>"`
markers — gets the identical rule, so the two cannot drift. Its marker branch is
now gated on line-initiality as a whole (it previously discarded *any* `#` line
and only gated the marker *read*), and
`the_two_tokenizers_agree_on_marker_free_input` pins token-for-token agreement
over an eleven-source corpus that exercises every shared branch. E01-F1's §1.1
claimed such a test existed; it did not, and is supplied here.

### 1.3 The deliberate behavioural decision: macro bodies

The task asked which way macro bodies are treated, preserved or fixed, and that
it be recorded. **Fixed, deliberately, toward uniformity:**

| Form | Before | After |
|---|---|---|
| single-line `#define` body | invisible (the whole physical line was swallowed) | invisible |
| multi-line `#define` body | **first line invisible, every continuation line tokenized as file-scope source** | invisible |

So a macro replacement list now contributes no token and no brace in either
form, and a macro's braces perturb no file-scope depth. This is the direction
the single-line case already had; the multi-line case was the outlier. It is
what makes the `OPEN_SCOPE(name)`-style unbalanced macro body — a common C
idiom — stop raising the depth permanently
(`macro_bodies_never_perturb_file_scope_brace_depth`).

**The measured cost of that decision, stated rather than hidden.** A
function-like macro that expands to a whole definition used to contribute a
*phantom* name — its own formal parameter — from its continuation line:

```c
#define DEFN(n)                    \
        int n(void) { return 0; }
DEFN(produced)
```

The old scan returned `["n"]`. `n` never denoted a definition anywhere: the
analyzer's body finder cannot find it in the preprocessed unit, so it was a row
nothing could discharge. It is gone now, and E01-F1's expansion reader
enumerates the real `produced` in its place
(`a_macro_parameter_is_no_longer_a_phantom_definition`).

**On the pinned corpus the phantom count is zero.** The exhaustive before→after
diff records **0 rows removed** from any unit's denominator and
`source_functions` unchanged at 4,921 — so on this corpus the repair is purely
additive to the raw scan and removes nothing at all. That was the load-bearing
risk in this card (a shrinking denominator is exactly how a unit could gain a
`Safe` verdict by losing a row); it did not materialise, and the general
direction is guarded by `the_enumeration_is_still_a_superset_of_the_repaired_raw_scan`.

---

## 2. Adversarial matrix

`crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs`, 14 tests. Every case is
stated against the *public* `legacy_c_source_functions`, so a regression is
visible at the API the rest of the repository consumes.

| # | Case | Test | Asserted |
|---|---|---|---|
| 1 | the exact rdkafka shape (`&(#ENUM)[18]` on a continuation line, carrying the initializer's closing brace) | `a_stringification_on_a_continuation_line_is_not_a_directive` | both definitions after the table are found; before, none were |
| 2 | brace desynchronisation over three later definitions | `a_desynchronised_brace_depth_no_longer_hides_the_rest_of_the_file` | all three found |
| 3 | `##` pasting, on a macro's first line and on a continuation line | `token_pasting_is_not_a_directive` | neither introduces a directive |
| 4 | single-line, multi-line and **unbalanced** macro bodies containing braces | `macro_bodies_never_perturb_file_scope_brace_depth` | depth unmoved in all three |
| 5 | the phantom-name cost of case 4 | `a_macro_parameter_is_no_longer_a_phantom_definition` | the formal is not a definition; the expansion supplies the real one |
| 6 | `#` in a string literal, a character constant, a `//` comment and a multi-line block comment; a `#` after a literal | `hashes_in_literals_and_comments_are_inert` | five following definitions all found |
| 7 | a `//` comment ending in `\` | `a_spliced_line_comment_swallows_its_continuation` | the spliced line stays comment text and its braces do not count |
| 8 | a real directive after a continued macro (`#define` … `\` … then `#define`, `#undef`) | `a_directive_after_a_continued_macro_is_still_a_directive` | the following directives are still directives |
| 9 | nested `#if`/`#if`/`#else`/`#endif`/`#elif` groups | `nested_conditionals_contribute_no_token` | directive lines contribute nothing; both arms' definitions are kept (E01-F1 §1.4 monotonicity) |
| 10 | an indented `#   define`, and a directive after a multi-line block comment | `indented_and_comment_preceded_directives_are_directives` | both are directives; their braces do not count |
| 11 | a `#` after real code on the same line | `a_hash_after_code_on_the_same_line_is_not_a_directive` | not a directive; the closing brace after it survives |
| 12 | established behaviour: `extern "C"` linkage blocks, `#if 0` bodies, prototypes, block-macro statements | `the_established_raw_scan_behaviour_is_unchanged` | all four unchanged |
| 13 | **the residual this repair does not close** (§3.2) | `a_conditional_brace_imbalance_is_a_separate_recorded_residual` | the raw scan under-reports; the enumeration covers it |
| 14 | monotonicity over seven shapes | `the_enumeration_is_still_a_superset_of_the_repaired_raw_scan` | the raw scan stays a subset of the audited denominator |

Plus four in-crate unit tests in
`crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs`: the two-tokenizer
parity corpus, a marker read only at a logical line start (a mid-line
marker-looking run is ordinary punctuation, so no attribution can be forged), a
marker after a continued directive, and the Lean checksum tie (§4).

---

## 3. Corpus re-measurement

### 3.1 The pinned 185-unit corpus

Re-measured through the canonical pinned route on the prepared pinned clone
(Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang`), run twice — once with the two changed production files at the
branch base `05c51a5` (**before**), once from this branch (**after**):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-e01f1raw
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-e01f1raw/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <reports>/monkey-source.json <reports>/resource-diagnostics.json
```

Both exit 0 with zero gateway failures. Transcripts under [`logs/`](logs).

**Harness validation — the before run reproduces the current pinned baseline
([`docs/reports/remaining-e01-f1/artifacts/after/`](../remaining-e01-f1/artifacts/after)).**
`resource-diagnostics.json` and `resource-diagnostics.chunk-contracts.json` are
**byte-identical** (SHA-256 `f4431c2c…` and `29122661…`, matching that
manifest exactly), and
[`docs/reports/corpus-repin/diff_artifacts.py`](../corpus-repin/diff_artifacts.py)
reports **0 added / 0 removed / 0 changed** rows in `diagnostics.units` (185),
`chunk-contracts.results` (96), `chunk-contracts.backend_teardown` (21) and
`source.callbacks` (3). Two artifacts differ, in fields that moved before any
change of mine:

* `monkey-source.json` — 17 scalar sites, every one an absolute path or a
  `char[N]` length derived from one. E01-F1 prepared its clones inside its own
  worktree; this run uses the shared prepared clone. Same
  location-provenance class E01-F1 §2.1, O04-ARMS and CORPUS-REPIN each
  recorded.
* `resource-diagnostics.source-effects.json` and
  `resource-diagnostics.contracts.json` — the same path class **plus** ChunkIO
  provider version `0.5.0 → 0.6.0` and its derived `package_digest`,
  `build_digest`, `configuration_digest`, obligation identifiers and
  `task_digest`. P04-NEXT bumped that package after E01-F1's measurement, so
  this is a wave-8 delta, not this unit's.

That is what makes the before→after diff an isolation of this unit.

**Headline, 185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only, before and after. No unit changed verdict in either direction, and
no function row was added, removed or changed status.**

| | before | after |
|---|---|---|
| units | 185 | 185 |
| enumerated source functions | 4,921 | 4,921 |
| `Safe` / provisional `Unsafe` / `Unknown` / data-only units | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 |
| `leak` diagnostics | 271 | **271** |
| `unknown_alias` diagnostics | 4,286 | **4,286** |
| `unsupported` diagnostics | 50,089 | **50,089** |
| `absent_from_raw_scan` rows | **146** | **15** |
| definitions counted but unread (`unanalyzable`) | 0 | 0 |
| units refusing the completeness claim | 0 | 0 |

**No unit moved to provisional `Unsafe`, and none left it.** Every diagnostic
count is identical, so the 131 definitions the raw scan newly sees surfaced no
new evidence in either direction — they were already in the audited denominator
and already analysed, because E01-F1's enumeration had covered them since wave 8.
That is the precise reason the verdict surface does not move: this repair
closes the *disagreement between two mechanisms*, and the audited denominator
was already the union of both.

**Isolation, from the exhaustive before→after diff.** Four of five artifacts are
byte-identical (`chunk-contracts.json`, `contracts.json`, `monkey-source.json`,
`source-effects.json`). In `diagnostics.json`, **exactly one unit changes and
exactly one field within it**: `librdkafka/rdkafka.c`'s
`enumeration.absent_from_raw_scan`. No unit was added or removed; no `status`,
`reason`, `reason_code`, `source_functions`, `functions`, `not_projected` or
`diagnostic_counts` field moved anywhere in the corpus.

| Library | units | `absent_from_raw_scan` before → after |
|---|---|---|
| cmetrics | 31 | 0 → 0 |
| ctraces | 17 | 0 → 0 |
| librdkafka | 94 | **146 → 15** |
| chunkio | 13 | 0 → 0 |
| fluent-bit-chunk | 1 | 0 → 0 |
| monkey-libevent | 29 | 0 → 0 |

### 3.2 The residual delta, per unit, and its cause

**184 of 185 units now have a zero residual: the raw scan and the enumeration
agree exactly.** One unit does not.

`librdkafka/src/rdkafka.c` keeps **15** rows, and they are not macro-produced
either — they are ordinary definitions written literally in the source
(`rd_kafka_mem_malloc`, `rd_kafka_mem_calloc`, `rd_kafka_mem_free`,
`rd_kafka_errno`, `rd_kafka_unittest` and the ten `rd_kafka_Uuid_*` functions),
all of them **after line 7123**. The cause is a *second, distinct*
desynchronisation, found by this re-measurement and named here for the first
time.

`rd_kafka_dir_is_empty` (librdkafka 2.15.0, `src/rdkafka.c` lines 7080–7118)
contains:

```c
#if defined(__sun)
                if (S_ISREG(st.st_mode) || S_ISDIR(st.st_mode) ||
                    S_ISLNK(st.st_mode)) {
#else
                if (d->d_type == DT_REG || d->d_type == DT_LNK ||
                    d->d_type == DT_DIR) {
#endif
                        closedir(dir);
                        return rd_false;
                }
```

Two conditional arms each open a brace and one `}` closes, because exactly one
arm survives preprocessing. The raw scan is deliberately **not** a preprocessor —
E01-F1 §1.4 makes keeping both arms load-bearing, so a definition guarded out by
`#if 0` never leaves the denominator — so it counts both opens and one close and
stays one level deep for the rest of the file.

Deciding which arm survives is conditional evaluation, not lexing. It is
therefore **outside** this card and is recorded as boundary **E01-F1-RAW-COND**,
not repaired; a reduced fixture pins it in both directions
(`a_conditional_brace_imbalance_is_a_separate_recorded_residual`): the raw scan
under-reports, and E01-F1's expansion reader supplies the hidden definition, so
the audited denominator is unaffected. This is the fail-closed direction and it
is why the residual costs nothing: `rdkafka.c` remains `Unknown`, with all 165
definitions in its denominator and `not_projected` empty.

### 3.3 The three configured audits

Run before (two production files reverted to `05c51a5`) and after, from the same
prepared inputs. Transcripts under [`logs/`](logs).

| Audit | result | delta |
|---|---|---|
| `cmetrics-counter` | 1 unit, 0 Safe / 1 provisional Unsafe / 0 Unknown; source contracts 1 checked / 6 Unknown | stdout **byte-identical**; report JSON **byte-identical** (`bdcaac6e…`) |
| `chunkio-core` | 13 units, 0 Safe / 2 provisional Unsafe / 11 Unknown; 126 source functions | stdout **byte-identical**; report JSON **byte-identical** (`08b01b71…`) |
| `libevent-core` | 21 units, 0 Safe / 4 provisional Unsafe / 17 Unknown; 729 source functions | stdout **byte-identical**; report JSON **byte-identical** (`b22e6482…`) |

No source set outside the pinned corpus contained a macro-continuation `#` that
the old rule mis-read, so all three are unchanged down to the byte, report
`enumeration` blocks included.

### 3.4 Archived artifacts

[`artifacts/`](artifacts) holds the gzipped before and after sets with a
`manifest.json` of uncompressed sizes and SHA-256 digests, the exhaustive
`diff_artifacts.py` documents for archived→before (harness validation) and
before→after (this unit's whole effect), `census.json` from E01-F1's own
read-only census tool, and `raw-residual.txt` from
[`raw_residual.py`](raw_residual.py) in this directory.

| Artifact | before SHA-256 | after SHA-256 | |
|---|---|---|---|
| `chunk-contracts.json` | `29122661…` | `29122661…` | identical, and identical to the pinned baseline |
| `contracts.json` | `144fa1ab…` | `144fa1ab…` | identical |
| `monkey-source.json` | `53d55b7e…` | `53d55b7e…` | identical |
| `source-effects.json` | `3227a56f…` | `3227a56f…` | identical |
| `diagnostics.json` | `f4431c2c…` | `79eeb928…` | changed in one unit, one field; `f4431c2c…` is the pinned baseline's digest |

**The re-pin is the integrator's decision.**
`docs/reports/remaining-e01-f1/artifacts/` is untouched.

---

## 4. Lean

`proof/QCore/RawDirectiveLexing.lean`, audited by
`proof/AuditRawDirectiveLexing.lean`: **31 theorems**, five depending on no
axiom at all and the rest on at most `propext` and `Quot.sound`; no new axiom
and no placeholder. `scripts/check_lean_audits.py` moves 44 → 45 audits.

The model abstracts the byte lexer to ten symbol classes and defines two
lexers — `lex` (the repaired rule) and `lexBefore` (the rule as it stood) —
together with the saturating file-scope `depth` that `function_names` computes.
What is proved:

* **the classification is total and single-valued** (`classify_is_total`,
  `classify_is_single_valued`), `#` introduces a directive **iff** it is
  line-initial in both directions (`hash_at_line_start_is_a_directive`,
  `hash_after_a_token_is_a_punctuator`, `directive_iff_line_initial`), nothing
  else ever introduces one (`only_hash_introduces_a_directive`), and a backslash
  is a splice exactly when a new-line follows (`splice_iff_newline_follows`);
* **a directive contributes no token**, single-line and continued alike
  (`a_directive_contributes_no_token`,
  `a_continued_directive_contributes_no_token`), with the corollaries in the
  quantity that matters — a directive's braces move no file-scope depth
  (`a_directive_body_moves_no_brace_depth`,
  `a_continued_directive_body_moves_no_brace_depth`);
* **the state flag is right in both directions** — a new-line restores
  line-initiality and a splice does **not**
  (`a_newline_starts_a_logical_line`, `a_splice_preserves_line_initiality`,
  `a_hash_across_a_splice_after_code_is_a_punctuator`), white space and comments
  preserve it, a line-spanning block comment restores it, a `//` comment
  swallows its continuation, and every token clears it;
* **stringification and pasting are punctuators**
  (`stringification_is_emitted`, `pasting_is_two_punctuators`);
* **non-vacuity against the measured defect** — the reduced `rdkafka.c` shape is
  decided under both rules: the repaired lexer returns to depth 0 and the
  pre-repair one to depth 1, the two differ
  (`the_repair_changes_the_measured_shape`), and the brace counts name exactly
  what happened — the old rule counted the `{` the continuation line opened and
  lost the `}` that closed it
  (`the_pre_repair_lexer_counts_an_unclosed_macro_brace`, 3 opens against 2
  closes; the repaired rule sees 2 and 2);
* **the repair removes nothing outside a directive** — on input free of `#`,
  `\` and `//` the two rules agree token for token
  (`the_rules_agree_on_directive_free_input`), which is what makes §1.3's
  phantom-name disappearance an accounting statement about directive text
  rather than a lost definition;
* **exhaustive agreement with the Rust lexer** — over all 400 symbol strings of
  length at most three on the seven single-byte symbols, both sides run their
  own lexer and fold the same checksum. Lean decides `domainChecksum 3 = 478977`
  in the kernel; `the_lean_domain_checksum_reproduces` recomputes it by running
  the real `tokenize` on the corresponding byte strings. A divergence between
  the model's directive rule and the implementation's moves the number, so
  neither can be changed alone.

Explicit Rust boundaries, stated in the module header and **not** proved there:
that the byte lexer's other branches (identifier and number runs, string and
character literals, the two comment scanners) behave as the symbols abstract
them, and that `function_names` reads the resulting token stream as the
definition finder this repository documents. Those are pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs`. The checksum ties the
directive rule over the seven single-byte symbols, not the whole byte lexer; and
the two general continuation theorems are stated over runs free of new-lines and
backslashes, with the real backslash-carrying shape decided outright instead.

---

## 5. Validation

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1,805 passed, 0 failed, 3 ignored** (1,787 passed, 3 ignored at the base) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **45 Lean audits passed** (44 → 45) |
| `cargo test --manifest-path providers/Cargo.toml` | 67 passed, 0 failed |
| providers `cargo fmt` / `cargo clippy -D warnings` | clean |
| `python3 -B -m unittest discover -s scripts/tests` | 108 passed |
| pinned 185-unit audit, before and after | exit 0, 0 gateway failures |
| `cmetrics-counter` / `chunkio-core` / `libevent-core` | exit 0, byte-identical (§3.3) |

New tests: **14** in `crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs` plus
**4** in `crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs`.

**Known load flake, not weakened.** `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs:736`, `elapsed < 500 ms`) failed
once while the Lean build and three configured audits were running concurrently,
and passes on an idle machine. It is a wall-clock bound on process spawn latency
and is unrelated to a C token scanner; the assertion is byte-for-byte unchanged.

---

## 6. Deliberate pin updates

Two, each stated with what it kept:

1. **`crates/qcore-c-gateway/tests/e01_source_enumeration.rs`** —
   `a_raw_scan_brace_desynchronisation_no_longer_hides_definitions` asserted
   that the raw scan *loses* both definitions after the table and that reading
   the expansion recovers them. The tokenizer defect is now repaired at its
   source, so the raw scan finds them itself and `absent_from_raw_scan` is empty
   there. **The load-bearing half is kept and strengthened**: the enumerated
   denominator is still exactly those two definitions and the claim is still
   established — and the two mechanisms now *agree* on this shape instead of
   disagreeing by two rows. The lexical matrix that keeps the raw half honest is
   the new `e01_f1_raw_tokenizer.rs`.
2. **`crates/qcore-c-gateway/src/legacy_enumeration.rs` documentation** — the
   module header and `SourceDefinitionInventory::absent_from_raw_scan`'s doc
   comment recorded the raw defect as live. They now record it as repaired by
   this unit while **keeping the field's literal name and meaning**: it still
   says only "the raw scan did not see it" and does not assert that a lexical
   cause is impossible, precisely so a future lexical gap lands there rather
   than being mislabeled. `tokenize_marked`'s header no longer claims a parity
   test that did not exist; the test now does.

No diagnostic code, contract file, schema type or wire byte changed.
`qcore-schemas` stays 7.0.0; `qcore-compiler` stays 0.3.0. The archived 185-unit
artifacts are **not** re-pinned here.

---

## 7. Open boundaries

* **E01-F1-RAW-COND** (new, §3.2) — a conditional group whose arms open braces
  unequally desynchronises the raw scan's file-scope depth. It is not a lexical
  defect: deciding which arm survives is conditional evaluation, which the raw
  scan deliberately does not do because keeping both arms is E01-F1's
  monotonicity base. It is the whole of the remaining 15-row residual on the
  pinned corpus, it costs nothing today (the enumeration covers it and the unit
  is `Unknown` either way), and closing it would mean giving the raw scan a
  conditional evaluator with its own parity run.
* **Splices inside tokens** — phase 2 splices *anywhere*, identifiers and string
  literals included. This lexer splices between tokens, in comments and inside
  directives, but a splice in the middle of an identifier still yields two
  tokens rather than one. It cannot hide a definition (the name and its `(` are
  still separate tokens either way) and no corpus instance exists, but it is a
  deviation from the standard and is recorded rather than claimed absent.
* **Line numbers inside string literals** — the literal scanner consumes
  `\`+newline as an escape without advancing the line counter. Pre-existing,
  untouched here, and affects only reported line numbers, never token identity.
* **E01-F1-STDIN, E01-F1-DENOM, E01-F1-COST, E01-B2/B3/B4** — unchanged by this
  unit.

## 8. Note for the integrator

Wave 9's **O04-B08** also touches gateway analysis, in
`crates/qcore-c-gateway/src/borrowed_actual.rs` and the exit-residual path. This
card touches only `legacy_c.rs::tokenize` (plus `line_splice_width`) and
`legacy_enumeration.rs::tokenize_marked`, so there is **no file overlap**. The
behavioural surface both cards share is the token stream: this card's whole
measured effect on it is confined to directive text, and every analysis artifact
of the 185-unit corpus and the three configured audits is byte-identical before
and after — so a merge should produce no verdict interaction. It is recorded
here rather than resolved cross-card. If O04-B08's own re-measurement lands
first, this card's before-run digests (`f4431c2c…` diagnostics, `29122661…`
chunk-contracts) identify which baseline it was measured against.

Nothing here establishes that any audited source is safe, and no raw unit is
promoted.
