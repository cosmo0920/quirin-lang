# E01-RAW-COND — Decide conditional-group brace balance in the raw scan

**Card:** E01-RAW-COND (wave 11), closing the residual
`docs/reports/remaining-e01-f1-raw/README.md` §3.2 named **E01-F1-RAW-COND**.
**Worktree base:** main `695f0e1` (wave-10 consolidation).
**Corpus baseline:** `docs/reports/wave10-corpus/artifacts/`.

Nothing here promotes any raw unit, and nothing here establishes that any
audited source is safe.

---

## 1. What was wrong

`librdkafka/src/rdkafka.c`'s `rd_kafka_dir_is_empty` (librdkafka 2.15.0, lines
7080–7118) contains:

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

Two arms each open a brace and a single `}` closes, because exactly one arm
survives preprocessing. The raw scan behind `legacy_c_source_functions`
**summed** the braces of every arm, so it counted two opens against one close
and stayed one level deep for the rest of the file, hiding fifteen ordinary
definitions written literally in the source.

Keeping both arms is not an oversight: E01-F1 §1.4 makes it load-bearing, so a
definition guarded out by `#if 0` never leaves the denominator. Deciding which
arm survives is conditional evaluation, not lexing — so the previous card
recorded the residual rather than repairing it.

## 2. The rule

> **A `#if … #endif` group's resulting file-scope brace depth is the greatest
> depth its arms reach**, each arm scanned from the depth the group began at,
> and a group with no `#else` carrying an implicit empty arm that reaches the
> entry depth.

Four properties, each proved in Lean (§5) and pinned in Rust (§4):

* **No conditional is ever evaluated and no arm is ever selected.** The
  controlling expressions are not read at all — `conditional_directive` reads
  only the directive *word*, and recognition is spelling-exact because a
  directive introducer is never macro-expanded (C11 6.10.3p10).
* **Agreement is exactness.** When every arm reaches the same depth — the
  `rd_kafka_dir_is_empty` case, and the overwhelmingly common one — the greatest
  depth *is* that depth, whichever arm survives. The group is decided exactly
  without selecting an arm.
* **Disagreement is refused, never guessed.** The greatest depth dominates every
  arm selection, so an undecided group can only make the scan *hide* definitions
  and never invent one; and
  `ConditionalScanRefusal::ArmDepthDisagreement` is recorded, which flows into
  the wave-8 enumeration as `EnumerationRefusal::ConditionalArmDepthDisagreement`
  and refuses that unit's completeness claim — the existing E01-F1 refusal
  mechanism, reused, not a new one.
* **Monotonicity is preserved (E01-F1 §1.4).** Every arm is still scanned, from
  the depth the group began at, so a definition written in *any* arm — including
  one under `#if 0` — keeps its row.

A second, structural ground, `ConditionalScanRefusal::UnstructuredGroup`, covers
an `#elif`/`#else`/`#endif` with no open group, an `#elif`/`#else` after an
`#else`, and a group never closed. None can occur in a unit Clang preprocessed
successfully, so each is a refusal rather than behaviour anyone relies on.

### 2.1 Where it lives, and what does not change

`tokenize`'s token stream is **byte-identical**: a directive still contributes
no token. `tokenize_with_conditionals` records the group marks as a side channel
that only `function_names` reads, so every consumer reading tokens alone is
untouched. `legacy_c_source_functions` keeps its exact signature and meaning;
the new `legacy_c_source_scan` is the same computation with the grounds kept, so
a caller that publishes a completeness claim can refuse it.

### 2.2 A latent soundness hazard closed on the way

The rule this replaces did not merely hide definitions. Summing the arms could
lower the depth *below* every arm selection — an arm that closes more braces
than it opens drove the running depth to file scope **inside** a function body,
where a block-macro statement was then read as a **phantom definition**. The
`max`-from-entry-depth rule cannot do this: the recorded depth is always a depth
some arm actually reaches, and it dominates every selection.

This is pinned in both directions —
`a_closing_arm_is_refused_and_never_lowers_the_depth` (Rust) and
`the_summing_rule_reaches_file_scope_inside_a_body` /
`the_rule_dominates_the_phantom_shape` /
`every_selection_of_the_phantom_shape_is_dominated` (Lean). It was not the
reported defect and it had no measured corpus occurrence; it is recorded because
the replacement removes it.

## 3. Recovered-work accounting

This card was relaunched after a wave-10 fleet trim. The prior agent's work
survived as dangling commit **`830e387`** ("Decide conditional-group brace
balance in the raw scan (E01-RAW-COND)"), whose parent was wave 9's `0f6d66c`.

`git cherry-pick -n 830e387` onto `695f0e1` applied **cleanly** — two
auto-merges (`legacy_c.rs`, `proof/QCore.lean`), no conflict, nothing salvaged
by hand and nothing re-derived. Everything in §2, §4 and §5 is that agent's
work, reviewed line by line and verified here rather than taken on trust:

| Recovered | State on arrival |
|---|---|
| `crates/qcore-c-gateway/src/legacy_c.rs` (+290) | complete |
| `crates/qcore-c-gateway/src/legacy_enumeration.rs` (+73) | complete |
| `crates/qcore-c-gateway/tests/e01_raw_cond.rs` (18 tests) | complete, all passing |
| `crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs` (pin update) | complete |
| `proof/QCore/ConditionalGroupDepth.lean` + audit + `QCore.lean` | complete, 24 audited theorems |

**Added by this relaunch:** the whole of §6 (the corpus re-measurement, both
routes, with archived artifacts and diff documents), §7 (the standing gate),
this report, and the review that confirmed §2.2 and the §6.2 no-movement
mechanism. The prior agent's commit message claimed its last step was the audit
file and the Rust checksum tie; both are present and both pass.

## 4. Adversarial matrix

`crates/qcore-c-gateway/tests/e01_raw_cond.rs`, **18 tests**, every case stated
against the public `legacy_c_source_functions` / `legacy_c_source_scan` /
`enumerate_source_definitions`.

| # | Case | Test | Asserted |
|---|---|---|---|
| 1 | the exact `rd_kafka_dir_is_empty` shape | `the_exact_rdkafka_shape_no_longer_desynchronises_the_file` | depth returns to zero; before, it did not |
| 2 | the measured 15-row residual shape | `every_definition_after_an_agreeing_group_is_visible_again` | all come back; **no new name appears** |
| 3 | arms that open, close, balance or leave depth alone | `arms_that_agree_are_decided_exactly` | decided exactly, no refusal, in all four |
| 4 | `#elif` chains (three or more arms) | `elif_chains_are_one_group_with_one_contribution` | one group, one contribution; implicit empty arm absent only under `#else` |
| 5 | nested groups | `nested_groups_compose` | inner contribution computed before the containing arm is measured |
| 6 | one arm opening a brace, no `#else` | `an_unbalanced_single_arm_is_refused` | **refused**; greater depth taken; later definition hidden, not invented |
| 7 | two arms disagreeing | `arms_that_disagree_are_refused_and_dominated` | **refused** both directions; recorded depth is the greatest |
| 8 | an arm that closes more than it opens | `a_closing_arm_is_refused_and_never_lowers_the_depth` | **refused**; depth is the *entry* depth — §2.2's phantom direction |
| 9 | the same shape, arms carrying definitions | `a_closing_arm_still_contributes_its_own_definitions` | every arm scanned from entry depth, so its definitions count |
| 10 | `#elif`/`#else`/`#endif` with no open group; `#elif` after `#else`; unclosed group | `unstructured_groups_are_refused` | each **refused** under `UnstructuredGroup` |
| 11 | a conditional directive on a macro continuation line | `a_conditional_inside_a_macro_continuation_is_not_a_group` | **not a directive** — wave-9's logical-line rule preserved, not resurrected |
| 12 | an `#endif`-looking run after code on the same line | `a_hash_after_code_opens_and_closes_no_group` | closes no group; a real group around it still decided |
| 13 | indent, block comment, phase-2 splice between `#` and word; every other directive word | `directive_words_are_recognised_exactly` | recognition total and spelling-exact; unknown words open no group |
| 14 | exhaustive length ≤ 4 domain against Lean | `the_lean_conditional_domain_checksum_reproduces` | both sides fold the same checksum independently |
| 15 | an ordinary token | `an_ordinary_token_is_the_identity` | identity on the scan state, so the Lean domain may omit it |
| 16 | a definition under `#if 0` | `every_arm_keeps_its_definitions` | keeps its row — E01-F1 §1.4 monotonicity |
| 17 | subset direction | `the_enumeration_is_still_a_superset_of_the_group_aware_raw_scan` | the raw scan stays a subset of the audited denominator |
| 18 | a refused group | `a_refused_group_refuses_the_completeness_claim` | the enumeration's completeness claim is withheld |

The 14 tests of `e01_f1_raw_tokenizer.rs` and the 19 of `e01_source_enumeration.rs`
run unchanged apart from the single deliberate pin update below.

### 4.1 Deliberate pin update

One, carried from the recovered commit and verified here:

**`e01_f1_raw_tokenizer.rs`** —
`a_conditional_brace_imbalance_is_a_separate_recorded_residual` becomes
`a_conditional_brace_imbalance_is_decided_when_the_arms_agree`. That test
asserted the raw scan *loses* the definition after the group and that the
expansion reader recovers it. Its arms agree, so this unit decides it.

**The load-bearing half is kept and strengthened**: the enumerated denominator
is still exactly those three definitions, the raw scan is still a subset of it,
and the two mechanisms now *agree* instead of disagreeing by one row — the test
additionally asserts `absent_from_raw_scan` is empty and `is_complete()` holds.
The direction it used to guard, a scan that cannot decide, is pinned harder than
before by matrix rows 6–10, which did not exist. No fail-closed gate is weakened:
the new grounds only ever *add* refusals.

No diagnostic code, contract file, schema type or wire byte changed.
`qcore-schemas` stays 8.0.0; `qcore-compiler` stays 0.4.0. The contract corpus
stays 60 modules and CT coverage stays 30/50.

## 5. Lean

`proof/QCore/ConditionalGroupDepth.lean`, audited by
`proof/AuditConditionalGroupDepth.lean`: **24 theorems**, no new axiom and no
placeholder (at most `propext` and `Quot.sound`). `check_lean_audits.py` moves
**51 → 52** audits.

The model abstracts the scan to six items (`{`, `}`, `#if`, `#elif`, `#else`,
`#endif`) — an ordinary token is proved to be the identity, so omitting it from
the domain costs nothing — and defines `step`/`scan` beside `scanSummed`, the
rule this unit replaces.

| Group | Theorems |
|---|---|
| the per-group rule | `groupDepth_dominates`, `agreeing_arms_are_exact`, `groupDepth_is_reached` |
| refusal is exactly the undecided case | `disagreeing_arms_refuse`, `agreeing_arms_do_not_refuse`, `refusal_is_monotone` |
| which arms are measured | `the_last_arm_is_measured`, `a_group_without_else_carries_the_entry_depth` |
| monotonicity (E01-F1 §1.4) | `a_separator_restores_the_entry_depth`, `a_group_opens_at_the_entry_depth` |
| identity | `other_is_the_identity` |
| non-vacuity, the measured shape | `the_rule_balances_the_measured_shape`, `the_measured_shape_is_not_refused`, `the_summing_rule_desynchronises_the_measured_shape`, `the_unit_changes_the_measured_shape`, `every_selection_of_the_measured_shape_is_matched` |
| §2.2's phantom direction | `the_summing_rule_reaches_file_scope_inside_a_body`, `the_rule_dominates_the_phantom_shape`, `every_selection_of_the_phantom_shape_is_dominated` |
| chains, nesting, anomalies | `an_elif_chain_is_decided_exactly`, `nested_groups_compose`, `unstructured_streams_are_refused` |
| exhaustive domain and the two-sided tie | `the_domain_is_exhaustive` (1,555 strings), `the_domain_is_decided`, `the_domain_checksum_is_pinned` (`360626`) |

`every_selection_of_*_is_matched`/`is_dominated` are the soundness statements:
they quantify over *every* arm selection — the expansion semantics this scan
refuses to compute — and show the rule's depth matches or dominates each one.

**Boundaries recorded, not proved.** The Rust byte lexer's recognition of the
directive spellings, and `function_names`' reading of a depth-zero name as a
definition, are Rust-side premises of the model; the checksum tie
(`the_lean_conditional_domain_checksum_reproduces` /
`the_domain_checksum_is_pinned`) binds the two sides over the length ≤ 4 domain,
not over all inputs. The wave-9 `QCore.RawDirectiveLexing` audit and its own
checksum tie are unchanged and still pass.

## 6. Corpus re-measurement

Canonical pinned route, prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`), run
twice — once with the two changed production files at base `695f0e1`
(**before**), once from this branch (**after**), with one provider-package build
shared by both runs so package identity cannot enter the diff:

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-e01rawcond
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<worktree>/build/cycle19/provider-packages-e01rawcond/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone>/fluent-bit-clang-source \
  <clone>/monkey/monkey-build/compile_commands.json \
  <clone>/reports/monkey-source.json <clone>/reports/resource-diagnostics.json
```

Both exit 0 with zero gateway failures. Artifacts and transcripts are archived
under [`artifacts/`](artifacts) with a `manifest.json` of uncompressed sizes and
SHA-256 digests.

### 6.1 Harness validation — the before run reproduces the wave-10 baseline

Against [`docs/reports/wave10-corpus/artifacts/`](../wave10-corpus/artifacts):

| Artifact | before SHA-256 | vs wave-10 |
|---|---|---|
| `diagnostics.json` | `cc6d8dc0…` | **byte-identical** |
| `chunk-contracts.json` | `c4f438e4…` | **byte-identical** |
| `source.json` | `53d55b7e…` | **byte-identical** |
| `contracts.json` | `38fa3dd6…` | differs — known residue |
| `source-effects.json` | `99fe9537…` | differs — known residue |

Both differences are exactly the two residue classes wave 9 and wave 10 each
recorded, confirmed leaf by leaf with [`leafdiff.py`](leafdiff.py):

* `contracts.json` — 14 leaves each side, **0 changed**; five `sources.*` keys
  differ only by the worktree path prefix
  (`…/qcore-c/compiler/…` → `…/qcore-c/.claude/worktrees/agent-…/compiler/…`).
  Path-literal provenance.
* `source-effects.json` — 998 leaves each side; 26 changed and 15 added/removed,
  every one a derived digest or obligation identifier
  (`package_digest`, `build_digest`, `configuration_digest`, `task_digest`,
  `obligation_id`, `coverage[0]`) moving because the provider packages were
  rebuilt at this worktree's path. Package identity. No verdict, status or
  reason leaf differs.

That is what makes the before→after diff an isolation of this unit.

### 6.2 Headline — before and after

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only, before
and after. No unit and no function changed verdict in either direction.**

| | before | after |
|---|---|---|
| units | 185 | 185 |
| unit verdicts | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** |
| function statuses | 119 Safe / 84 Unsafe / 4,631 Unknown | **identical** |
| enumerated denominator (`source_functions`) | 4,921 | **4,921** |
| enumeration completeness | 185/185 `established` | **185/185 `established`** |
| enumeration refusal grounds | none | **none** |
| diagnostic counts | leak 271, unknown_alias 4,386, unsupported 49,936 | **identical** |
| **`absent_from_raw_scan`** | **15** | **0** |

The console transcripts are **byte-identical** (`diff` reports 0 lines). Four of
the five artifacts are byte-identical. The whole measured effect of this unit on
the pinned corpus is one artifact, `diagnostics.json`, and within it
[`leafdiff.py`](leafdiff.py) reports **15 removed, 0 added, 0 changed** leaves,
all of them `units[69].enumeration.absent_from_raw_scan[*]` — that is,
`librdkafka/rdkafka.c` (`status: Unknown`, before and after). The exhaustive
document is [`diff-before-to-after.txt`](diff-before-to-after.txt); per-unit
residual censuses are [`residual-before.txt`](residual-before.txt) and
[`residual-after.txt`](residual-after.txt).

### 6.3 The 15 definitions, and why no verdict moved

The fifteen rows that leave `absent_from_raw_scan` are exactly the fifteen
E01-F1-RAW §3.2 named:

`rd_kafka_mem_malloc`, `rd_kafka_mem_calloc`, `rd_kafka_mem_free`,
`rd_kafka_errno`, `rd_kafka_unittest`, and `rd_kafka_Uuid_base64str`,
`rd_kafka_Uuid_copy`, `rd_kafka_Uuid_destroy`, `rd_kafka_Uuid_hash`,
`rd_kafka_Uuid_least_significant_bits`, `rd_kafka_Uuid_map_hash`,
`rd_kafka_Uuid_most_significant_bits`, `rd_kafka_Uuid_new`,
`rd_kafka_Uuid_random`, `rd_kafka_Uuid_str`.

The card's brief asked that newly visible definitions enter the analyzer so
verdicts *can* move, and that each mover be named with its rule. **None moved,
and the measured reason is structural rather than incidental** — two independent
mechanisms, both verified in the code:

1. **The analyzer selects from the enumeration *union*, not the raw scan.**
   Under `LegacyFunctionScope::MainSourceDefinitions`,
   `analyze_unrestricted_c11_with_inventory` takes `inventory.definitions()` —
   raw scan ∪ expansion reader. E01-F1's expansion reader already supplied all
   fifteen, so the union, the denominator (4,921) and the analyzed selection are
   unchanged. The raw half simply stopped disagreeing.
2. **The analysis pass never sees a conditional.**
   `analyze_legacy_c_translation_unit_selected_partial` runs `function_names` on
   the **preprocessed** text, where Clang has already resolved every `#if`; no
   conditional mark is ever produced there, so `apply_conditional` never runs
   and that path is byte-identical by construction.

So on this corpus the rule reaches exactly one thing: the raw half of the union.
A verdict could move under this rule only where the raw half is the *sole*
contributor of a definition — a definition guarded out of the expansion, after a
group the old rule desynchronised — which does not occur in these 185 units.
The cost of the fifteen-row residual was, as E01-F1-RAW predicted, zero; closing
it buys agreement between the two mechanisms, not coverage.

### 6.4 Second route, same answer

The current-revision route (`--compile-database`, where `qcore-audit`'s
`current.rs` calls `legacy_c_source_functions` directly) was measured before and
after as an independent check: **185 files / 186 configurations, 2 Safe /
43 provisional Unsafe / 137 Unknown / 4 data-only** both times, transcripts
byte-identical, and the same **15 removed / 0 added / 0 changed** leaves. The
42-vs-43 reading is CORPUS-REPIN's recorded files-versus-configurations
distinction (`src/flb_input_chunk.c` counted twice under G01's compile-configuration
ambiguity), not a movement.

### 6.5 Re-pin

`artifacts/after/` is this unit's measurement. **Whether it becomes the corpus
baseline is the integrator's decision**; `docs/reports/wave10-corpus/artifacts/`
is untouched. Note for that decision: the two artifacts that differ from wave 10
(`contracts.json`, `source-effects.json`) do so only in §6.1's residue classes,
which are provenance of *this worktree*, not of the change — an integrator
re-measurement on the merged tree would carry its own.

## 7. Validation

Standing gate, this worktree, Linux x86_64:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **2,052 passed, 0 failed, 4 ignored** (127 targets) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **52 audits pass** (was 51) |
| providers workspace `cargo test` | **76 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | 125 ran, **1 failed by design** — §7.1 |
| pinned 185-unit audit, before and after | exit 0, 0 gateway failures |

The wave-10 merged gate was 2,034 workspace tests / 76 provider / 125 Python /
51 Lean audits. This card adds 18 workspace tests and one Lean audit; the
remaining workspace delta is other wave-11 cards' if any land alongside.

### 7.1 Note for the integrator

**A new Lean audit makes the acceptance-matrix derived-facts test fail by
design.** The one Python failure is
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`:

```
AssertionError: Lists differ: ['8.0.0', 51] != ['8.0.0', 52]
```

`AuditConditionalGroupDepth.lean` moves the audit count 51 → 52, and
`docs/reviews/r0_3/acceptance_matrix.json` is off limits to this card — that is
precisely the tripwire the test exists to be. It clears when the matrix is
regenerated at the merged gate numbers, as in previous waves. The other 124
Python tests pass.

**File overlap.** This card touches `legacy_c.rs` (`tokenize`,
`tokenize_with_conditionals`, `function_names`, `legacy_c_source_scan`),
`legacy_enumeration.rs` (`EnumerationRefusal`, `enumerate_source_definitions`),
its two test files, and three proof files. It is the direct successor to wave 9's
E01-F1-RAW in the same functions, so it must merge after it — which it does, the
recovered commit having been rebased onto `695f0e1` cleanly.

**Load-flaky tests** (P02 `RLIMIT_NPROC`, P03 confined-write, vp04) are unrelated
to this card; any failure there should be rerun in isolation.

## 8. Open boundaries

* **Disagreeing arms are dominated, not decided.** A group whose arms leave
  different depths is refused, and the scan carries the greatest depth. That is
  sound (it can only hide definitions) and it withholds the unit's completeness
  claim, but it is not a decision. Deciding one would require a conditional
  evaluator with its own parity run, and with it a defensible answer for
  target-dependent macros such as `__sun` — outside this card, and outside the
  raw scan's stated purpose. **No corpus instance exists** (§6.2: zero refusal
  grounds across 185 units), so this boundary is currently unexercised in
  production.
* **`UnstructuredGroup` is unreachable in practice.** Every ground it names is
  impossible in a unit Clang preprocessed successfully. It is a fail-closed
  backstop, exercised only by fixtures.
* **The Rust/Lean tie is bounded.** The checksum binds the two sides over item
  strings of length ≤ 4 (1,555 strings). It is not an inductive correspondence,
  and the byte lexer's directive recognition and `function_names`' definition
  reading remain Rust-side premises of the model.
* **The rule reaches only the raw half of the union on this corpus** (§6.3), so
  it buys agreement rather than coverage. Whether any *other* source set has a
  definition contributed solely by the raw half after a desynchronising group is
  unmeasured.
* **Carried unchanged from E01-F1-RAW §7** — splices inside tokens, line numbers
  inside string literals, E01-F1-STDIN, E01-F1-DENOM, E01-F1-COST, E01-B2/B3/B4.

With this card **E01-F1-RAW-COND is closed**: the pinned corpus's
`absent_from_raw_scan` is zero in every one of its 185 units, and the raw scan
and the wave-8 enumeration agree exactly.
