# TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE

Card TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE (named follow-up registered by
LERI-CROSS-UNIT-ELEMENTS-RELEASED). Base `f47823dd1` (integrate/r10); code
tip `09794501e`, measured. New rule 49 (gateway); Lean
`QCore.GuardCorrelatedCensus` (new). The [prediction](PREDICTION.md) was
committed at `57041a537`, before any tip measurement (the mechanism, fixtures
and Lean model were drafted and run only on fixtures and raw files; the
prediction says so).

## Headline

* **Root cause.** `plugins/in_tail/tail_file.c` at upstream `90978ac6`,
  `tag_compose`: lines 1255–1310 are `#ifdef FLB_HAVE_REGEX` … `else {`
  `#endif` and lines 1368–1370 are `#ifdef FLB_HAVE_REGEX` `}` `#endif`. Each
  group alone disagrees (the arm +1 against the implicit empty arm, then −1
  against it), so E01-RAW-COND refused both, carried the greater depth and
  ended the file one level deep: the raw scan saw 32 of the file's 48
  definitions (`flb_tail_file_remove_all` among the 16 hidden). It was the
  slice's only unit with an enumeration refusal, and that one refusal refused
  the whole import phase.
* **Rule 49, guard correlation.** When the raw scan's *only* ground is an arm
  disagreement, a second reading decides it without evaluating any
  conditional: one raw scan per *world* (assumptions `NAME is (not)
  defined`); a group opened by exactly `#ifdef NAME`/`#ifndef NAME` with no
  `#elif` is read under the world's assumption (splitting when it has none),
  every other group must still have agreeing arms in every world, every other
  directive and `_Pragma` clears assumptions, worlds with equal states merge
  keeping shared assumptions, more than 64 worlds refuse, and a directive
  spelling `_Pragma`/`__pragma`/`push_macro`/`pop_macro` refuses. The names
  it collects are unioned into the raw half and the refusal dropped; anything
  it cannot decide keeps the refusal exactly as before.
* **Measured (own `prepare --link` of 90978ac6, rule 40):** `tail_file.c`'s
  census is complete; the slice's import phase now runs; home exports 8 → 10
  (`flb_tail_file_remove_all`, `flb_tail_file_dormant_scan_begin`); **no row
  moves** (59 Safe / 1 Unsafe / 340 Unknown / 4 Pending both sides); four
  Unknown rows lose leaves and keep others. All as predicted. Arbiter (stated
  map, slice): see section 4.

## 1. Mechanism

`crates/qcore-c-gateway/src/legacy_guard_correlation.rs`
(`correlated_source_scan`, `guard_of`); one call in `legacy_enumeration.rs`'s
`raw_half`; `tokenize_with_directives` in `legacy_tokens.rs` (the token stream
and conditional marks are byte-identical to `tokenize_with_conditionals`'s, it
also reports where every other directive stood). `legacy_c_source_scan`,
`function_names` and every other reader are unchanged, so E01-RAW-COND's
Lean checksum still reproduces.

Why sound: the worlds always cover every valuation of every guard in each
epoch between directives (a split covers both values, a merge and a clear only
weaken assumptions); each world follows exactly the configurations its
assumptions admit for guard groups, and every arm of any other group from the
entry depth, refusing when they disagree; so every definition any
configuration keeps at file scope — guarded-out arms included — is reached at
file scope by some world. A merged world may follow an infeasible
configuration, which can only add a name to the denominator (fail-closed
direction). A different-guard shape (`#ifdef A` opens, `#ifdef B` closes) is
therefore *decided*, not refused: every configuration is followed and a
definition at file scope in any of them is collected (fixture
`different_guards_are_followed_in_every_configuration`).

The premise is E01-RAW-COND's, unchanged: the raw half reads written text,
not macro expansions (a macro defined elsewhere that expands to a brace, or to
a `push_macro`/`pop_macro` pragma, is outside what it reads), and the
expansion half stays authoritative for the configuration actually compiled.
A macro *defined in the file* with a macro-table operator is refused outright.

Raw-source reach (every `.c` file of Fluent Bit 66910c10, 046accf6 and
90978ac6, and of `third_party/libevent`; `private/census/zz-raw-scan-census.txt`):
19 Fluent Bit files per tree have the arm disagreement as their only ground,
14 are decided (the WAMR interpreters/loaders and mbedtls `ssl_srv.c` stay
refused, uncorrelated groups). Of the decided files the live routes select
only `tail_file.c` (slice): the system selection's 8 files and the corpus
selection (cmetrics, ctraces, librdkafka, chunkio, `src/flb_input_chunk.c`)
contain none, so neither route can move. `libevent-core` (not a live route)
has two: `buffer.c` (`evbuffer_write_atmost`, `USE_SENDFILE`) and
`bufferevent_sock.c` (`_WIN32`), both the same shape and both now decided;
E01-TAIL recorded them as that route's two enumeration refusals. Not
measured here (follow-up).

## 2. Fixtures

`crates/qcore-c-gateway/tests/guard_correlated_census.rs` (16 tests):

| Fixture | Result |
|---|---|
| `tag_compose` reduced (two `#ifdef FLB_HAVE_REGEX` groups) | raw scan refused, later definitions hidden; correlation decides all four; census complete, `supports_unit_safe`, nothing expansion-only |
| `#ifndef` closing an `#ifdef` | decided |
| same shape behind `#if defined(FLB_HAVE_REGEX)` | refused (`ArmsDisagree`); census refused as before |
| an `#elif` in the first group | refused |
| an ordinary disagreeing group inside the correlated region | refused |
| different guards whose arms leave different definition sets | decided, every configuration's definitions collected |
| `#undef` between the groups | decided (precision lost, the second group splits again) |
| 65 independent guards opening a brace | refused (`WorldBudget`) |
| unstructured streams | refused |
| `_Pragma`/`__pragma`/`#pragma pop_macro` in a directive | refused; census refused; a plain `#define` is not |
| `extern "C"` behind `#ifdef __cplusplus` | read as `function_names` reads it |
| `guard_of` spellings | exact `#ifdef`/`#ifndef NAME`, one trailing comment; `#if defined`, `#elifdef`, a commented directive word are not guards |
| Lean domain checksum, Lean vectors | reproduce `QCore.GuardCorrelatedCensus` |

On the real `tail_file.c` (90978ac6) the reading decides 48 definitions, the
16 the raw scan hid among them.

## 3. Lean

`proof/QCore/GuardCorrelatedCensus.lean`, audited by
`proof/AuditGuardCorrelatedCensus.lean` (16 `#print axioms`; only `propext`
and `Quot.sound`): the world machine (`cscan`, flat, mirroring the Rust) and a
selection semantics (`run`: a preprocessor under a guard valuation per
`reset` epoch and an arbitrary outcome for every other controlling
expression). `a_split_covers_every_value`, `a_merge_only_weakens`,
`a_clear_assumes_nothing` (general); `e01_raw_cond_refuses_the_measured_shape`
(through `QCore.ConditionalGroupDepth`), `the_measured_shape_is_decided`,
`the_measured_shape_is_sound`; negatives `an_uncorrelated_shape_is_refused`,
`an_elif_group_is_refused`, `a_reset_between_is_still_sound`,
`a_reset_between_is_decided`, `different_guards_collect_every_configuration`;
**`the_domain_is_sound`**: over all 11,111 streams of length at most four over
ten items, whenever the scan decides, every configuration's file-scope
definitions are collected (by `decide +kernel`; a mutation that stops
splitting fails it and `different_guards_collect_every_configuration`);
`the_domain_checksum_is_pinned` (673782), reproduced by the Rust test. No
placeholder, no project axiom. Rust boundaries: `guard_of`'s spellings, the
reporting of every other directive as a reset, `_Pragma` clearing, the
macro-table-operator refusal, `extern "C"` braces, and that a valuation beyond
the enumerated epochs and positions is not read (the bounded theorems
enumerate exactly the ones a stream reads).

## 4. Measurement

### Inputs

This card's own `quirin-audit prepare clang-build --link` of upstream
`90978ac6` (base binary `42b8d622…` built in this worktree at `f47823dd1`;
Ubuntu clang 21.1.8; linked, every linked object reproduced;
[inputs.json](artifacts/inputs.json); log `private/logs/prep-slice.log`;
link-resolution artifact `d1c76d8c…`). Base and tip binaries for the quick
runs are built inside `git archive` exports of `f47823dd1` (`56b7aedb…`) and
`09794501e` (`1a3affea…`) with `third_party/libevent` cloned at its pin. A
first tip run on `a7213f08c` (before the macro-table-operator refusal was
added) was stopped and discarded.

### The link-standing route (quick runs, not the arbiter)

`private/scripts/route.sh` (the measure plugin's route environment, copied
from the cross-unit card), [lines](artifacts/route-lines.txt), row
comparisons by [compare.py](scripts/compare.py):

| Slice | base | tip |
|---|---|---|
| rule 40 (124 derived homes) | 59/1/340/4; `cross-unit import phase refused: incomplete definition census`; 8 home exports, no import | 59/1/340/4; **import phase runs** (16/16 units); 10 home exports |
| stated map | 59/1/340/4; import phase refused (census) | 59/1/340/4; census line gone; 0 home exports (its eight homes are not slice units), nothing imported |

[Rule 40 base/tip](artifacts/quick-slice-resolution-base-tip.json): 0 moves,
4 leaf trades, every one read:

* `in_tail_exit` (`tail.c`) loses `flb_tail_file_remove_all`; keeps an
  indirect access, unresolved `data`/`ctx`, uncontracted `flb_tail_fs_exit`.
* `flb_tail_config_destroy` (`tail_config.c`) loses `flb_tail_file_remove_all`
  and `flb_tail_mult_destroy`; keeps 21 others.
* `flb_tail_scan` (`tail_scan.c`) loses `flb_tail_file_dormant_scan_begin`;
  keeps 9 others.
* `flb_input_collectors_start` (`src/flb_input.c`, linked configuration)
  loses `flb_input_thread_collectors_signal_start` (a base export, never
  imported at base because the phase was refused); keeps others.

The tip also prints seven `cross-unit import … refused: caller configuration
is not a linked, reproduced configuration of the executable` notes for the
shared (`-fPIC`) `flb_input.c` configuration (rule 40 condition 3, fail-closed
as designed); no leaf or status changes there.
[Stated map base/tip](artifacts/quick-slice-map-base-tip.json): 0 moves, 0
leaf trades.

### Arbiter

`quirin-audit measure --declared PREDICTION.md --routes slice`
(`private/scripts/arbiter.sh`, output `measure/arbiter-1`, `quirin-audit`
`f8548d1e…` and measure plugin `89a755a6…` built from this worktree at
`09794501e`), over [inputs.json](artifacts/inputs.json), the live route as the
plugin runs it (stated eight-home map). Both sides are checkout directories,
`git archive` exports of `f47823dd1` (base) and `09794501e` (tip) with
`third_party/libevent` cloned at its pin, built in place by the tool (side
binaries `89725901…` and `1a3affea…`); a `rev:` side needs user namespaces this
host refuses (as the two preceding cards found; not retried). `--no-slots`:
outside the host's shared gate slots, disclosed. Exit 0, status complete, no
failure. [native-comparison-slice.json](artifacts/native-comparison-slice.json):

| Route | base | tip | moved | content changed | declared |
|---|---|---|---|---|---|
| slice (404 rows) | 59 / 1 / 340 / 4 Pending | 59 / 1 / 340 / 4 | 0 | 8 | **match** |

All eight content-changed rows (`flb_input_instance_exit` ×2,
`flb_create` ×2, `flb_input` ×2, `flb_service_set` ×2) change only
`review_holds`, and only in the hold's `module` path, which spells the side's
export directory (`private/base-src/…` against `private/tip-src/…`): with that
prefix normalised, status, diagnostics and holds are equal on all 404 rows
(`private/scripts/holds.py`). The same artefact of directory sides the
cross-unit card recorded. The arbiter's figures differ from the route's
committed baseline (58/341) on both sides alike: this card's own preparation,
not the published inputs.

The code measured is `09794501e`. The later commit `0a4ad59a8` is the
integrator's test-only fixture fix cherry-picked from integrate/r10
(`562d4594d`: the cross-unit fixtures' `item_destroy` names its link member,
`resource_unpublished(e, _head)`, as rule 46 requires; it repairs five tests
failing on this base, `cross_unit_elements_released` ×4 and
`link_standing_elements_released` ×1, that are not this card's).

## 5. Corpus and system

Not measured: no unit either route selects has the refusal rule 49 reads
(section 1), so `raw_half` returns exactly what it returned at base for every
one of their units.

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) retires
`TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE` and registers
`LIBEVENT-CORE-CENSUS-GUARD-CORRELATION` (re-measure `libevent-core`, whose
two refused units `buffer.c` and `bufferevent_sock.c` are now decided). The
callers' remaining leaves stay with their existing trackers (out of scope).
No evaluation-profile baseline changes (`profiles: []`). The increment
paragraph is the file's `increment_paragraph`; `--check` exits 0 on this tree.

## 7. Gate

`python3 scripts/gate_scope.py --base f47823dd1` (through
`private/toolchain/env.sh`) on `459590a8f` (code equal to the measured
`09794501e`; later commits are the integrator's fixture fix, the merge of
integrate/r10 `98e0cc91e`, the regenerated matrix counts, and this record):
**exit 0**, mode full (the diff reaches `qcore-c-gateway`), 8 steps
([gate.json](gate.json), logs `private/gate-scope/20261113T173453Z-3485182/`,
console `private/logs/gate-3.console.log`; Clang recording
[gate/clang-version.txt](gate/clang-version.txt), taken in this worktree).

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy (`-D warnings`) | 0 / 0 / 0 | 1 submodule |
| Rust suite | 0 | 636 units, 679 summaries; 9,492 passed, 0 failed, 17 ignored |
| both reevaluation probes | 0 / 0 | |
| Lean (`check_lean_audits.py`, all: a proof root changed) | 0 | 433 audits (this card adds `AuditGuardCorrelatedCensus.lean`) |
| Python suite | 0 | 761 tests |

Incomplete attempts, kept: gate 1 (`private/logs/gate-1.console.log`) was
stopped on the integrator's note that the base carries five failing
cross-unit fixture tests (fixed by cherry-picking `562d4594d`); gate 2
(`private/gate-scope/20261113T165840Z-3006310/`, exit 1) passed every step
but Python, whose one failure
(`test_each_derived_dimension_matches_the_section_it_counts`, 949 ≠ 956) is
the base's stale matrix (it fails identically on a `f47823dd1` export).
Merging integrate/r10 brought its regenerated matrix; `build_matrix.py` then
moved the Lean audit count 432 → 433 for this card's audit (it also relabels
the gate block as carried, not measured, until the integrator passes
`--gate`). No provider change, so no provider gate.
