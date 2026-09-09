# E01-TAIL — the enumeration leftovers

2026-09-09. Branch base `19fbaa5` (wave 14 merged). Workstream W3/W2, follow-on
unit of [`docs/reports/remaining-e01-f1/README.md`](../remaining-e01-f1/README.md)
§6, closing its three named residuals: **E01-F1-STDIN**, **E01-F1-DENOM** and
**E01-F1-COST**.

Two of the three are real repairs, both fail-closed; the third is a measurement
whose honest outcome is *do not build it*, with the reason recorded.

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe
/ 137 Unknown plus four data-only units, the contract corpus stays 64 modules,
CT coverage stays 32/50 mapped and full language conformance stays false.

| Residual | Disposition | Corpus effect |
|---|---|---|
| **E01-F1-STDIN** | **Repaired.** Attribution moves from the marker's file *name* to the include *depth* those markers describe, so a `#line` can move no definition into or out of the audited scope and the rule is identical for a `<stdin>` unit and a declared-file unit. No transport, identity or published byte changed. | none (§4) |
| **E01-F1-DENOM** | **Repaired, deliberately strict.** A whole-unit `Safe` now requires every selected definition to carry an analysed row. Two pinned expectations updated, each keeping its load-bearing half. | none — measured reach **0** (§3.1) |
| **E01-F1-COST** | **Measured, not built.** The extra `clang -E` costs a median 53.4 ms against the published `-E -P`'s 53.2 ms, but the audit route already computes it exactly **once per unit**, so a sound input-keyed skip would eliminate **zero** preprocessings. Two *larger* pre-existing redundancies were found and are named, not repaired. | none |

---

## 1. E01-F1-STDIN — attribution by include depth

### 1.1 The residual, and what was actually wrong with it

E01-F1 recorded:

> the primary input is read from the first line marker. For the pinned Cycle 19
> route that marker is `<stdin>`, so a unit transported through standard input
> attributes *everything Clang read from stdin* to the main source. That is
> correct here (the stdin text *is* the main source), but it means the
> header/main split for such a unit rests on the transport, not on a path.

Measured, the dependence is sharper than "rests on the transport". The rule was:
a token is main-source when the marker in force names the primary input. A
`#line` directive writes that name, so the rule is decided by a *directive in
the analysed source*, in both directions. Clang 21, run both ways over the same
fixture (§2 for the transcript):

| Direction | Witness | Under name attribution | Consequence |
|---|---|---|---|
| **into** the scope | `#line 100 "<stdin>"` inside a header | the header's definitions are counted as main-source | a scope misstatement — boundary **E01-B2** exists to keep header definitions out — reachable for every `HistoricalStdin` unit without knowing any path, because `<stdin>` is one fixed word |
| **out of** the scope | `#line 500 "elsewhere.h"` in the main source | the rest of the main source leaves the enumerated set | **fail-open**: a definition spelled literally is rescued by the raw-scan half of the union, but a *macro-produced* one after that directive has no raw-scan row and leaves the denominator with nothing said |

The asymmetry E01-F1-STDIN named is real and is measured here: a declared-file
unit's primary spelling is whatever the compilation database's input word is, so
the first direction needs an author who knows the build; a `HistoricalStdin`
unit's is the literal `<stdin>`.

### 1.2 The repair

Clang's markers carry the provenance directly. Flag `1` enters a file, flag `2`
returns from one; flags `3` (system header) and `4` (`extern "C"`) say nothing
about the stack, and **a `#line` directive carries no flag at all** — Clang
re-emits it as a bare `# <line> "<file>"`. So the include *depth* is a quantity
no directive in the source can move, and

> a token is main-source exactly when its include depth is 0

is the same rule for both transports. `MarkedStream` now carries `depths`
instead of file indices; `stream.primary` survives only as the environment check
that Clang emitted markers at all, and is no longer consulted for attribution.

This is a sound extension *within the recorded transport*, which is the outcome
the card preferred over publishing a scope marker: `CSourceInput` is untouched,
`HistoricalStdin` keeps its identity bytes and both G02 exemptions, the
enumeration-only preprocessing is still never published, digested or analysed,
and no `CompileTaskIdentity` field moves.

### 1.3 The new refusal ground

Depth is only meaningful if the stack is readable, so a stream that underflows
(a return with nothing to return to) or ends with a file still open refuses the
completeness claim on the new ground `unbalanced-include-markers`. Its
`ReasonCategory` is `environment-unavailable`: the failure is that the analysis
machine's own output could not be read, so under §L21.1 rule 1 it says nothing
about the analysed source and can never contribute to a provisional `Unsafe`.
`breakage_is_absorbing` proves a lost stack cannot be repaired by later markers.

A real Clang stream always balances, so this ground has **measured corpus reach
0** and is exercised at the reader level, not through a preprocessing (§2.2).

### 1.4 What the repair does not close

* Which *headers* a `HistoricalStdin` unit finds is still decided by the
  gateway process's working directory rather than the includer's, because the
  text is piped. That is G02's boundary, unchanged here, and it is about
  *which text is read*, not about which of the read text is main-source.
* Clang's flag semantics remain a **tool boundary**: that `1` enters, `2`
  returns and `#line` sets neither is an observation, pinned behaviourally by
  `crates/qcore-c-gateway/tests/e01_stdin_attribution.rs` (real preprocessings,
  both transports) and by the reader-level matrix in
  `crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs`. It is stated as a
  premise in `QCore.EnumerationAttribution` and not proved there.
* Boundary **E01-B2** itself is unchanged: a definition genuinely written in a
  header is still out of scope.

---

## 2. E01-F1-STDIN — measurement and adversarial evidence

### 2.1 The measured Clang transcripts

`/usr/bin/clang` 21.1.8. Fixture: `main.c` including `hdr.h`, where `hdr.h`
carries `#line 100 "<stdin>"` and `main.c` carries `#line 500 "elsewhere.h"`.

Piped (`clang -x c -std=gnu11 -E -I. - < main.c`), abridged to the markers:

```
# 1 "<stdin>"
# 1 "<built-in>" 1
# 1 "<built-in>" 3
# 410 "<built-in>" 3
# 1 "<command line>" 1
# 1 "<built-in>" 2
# 1 "<stdin>" 2
# 1 "./hdr.h" 1
# 100 "<stdin>"                 <-- flagless: still inside hdr.h, depth 1
int hidden_in_header(void) { return 1; }
# 2 "<stdin>" 2
int real_main_fn(void) { return 2; }
# 500 "elsewhere.h"             <-- flagless: still the primary input, depth 0
int relabelled_main_fn(void) { return 3; }
```

The declared-file run is identical in structure with the path in place of
`<stdin>`. A forced `-include` file was measured separately and sits at depth 2,
inside `<built-in>`.

### 2.2 Adversarial tests

`crates/qcore-c-gateway/tests/e01_stdin_attribution.rs` — five tests, each
driving a **real** preprocessing:

| Test | What it pins |
|---|---|
| `a_header_relabelled_to_the_primary_spelling_stays_out_of_scope` | the header's definition stays out under *both* transports, and the two transports agree definition for definition |
| `a_header_relabelled_to_the_declared_path_stays_out_of_scope` | the same attack aimed at a declared-file unit, writing the compile command's own input word (an absolute path would miss by string comparison and pass vacuously) |
| `a_relabelled_main_source_tail_stays_in_scope` | the fail-open direction, with a macro-produced definition that has *no* raw-scan row — asserted absent from `legacy_c_source_functions` first, so the rescue cannot be the union's |
| `both_transports_enumerate_an_ordinary_unit_identically` | the non-vacuity control: no `#line` anywhere, header definitions out, main-source expansions in, both transports identical |
| `a_balanced_stream_is_established_and_the_unbalanced_ground_is_environmental` | a real stream is *not* refused by the new check, and the ground's §L21.1 subject |

`crates/qcore-c-gateway/tests/unit/legacy_enumeration.rs` adds the reader-level
matrix: `include_depth_follows_the_enter_and_return_flags` (system-header flags,
both `#line` directions, nesting), `unbalanced_marker_streams_are_refused`
(underflow, unclosed, and the balanced control) and
`an_unreadable_trailing_word_is_not_a_marker`.

**Both repaired directions were confirmed to fail before the repair.** The
attribution line was temporarily reverted to the name rule in a throwaway edit
and the suite re-run:

```
a_header_relabelled_to_the_primary_spelling_stays_out_of_scope   FAILED
  left: ["relabelled_header_fn", "subject"]   right: ["subject"]
a_relabelled_main_source_tail_stays_in_scope                     FAILED
  left: ["before", "literal_after_relabel"]
  right: ["before", "hidden_after_relabel", "literal_after_relabel"]
```

Note the second: `hidden_after_relabel` — the macro-produced definition —
simply was not there.

### 2.3 Two pre-existing unit tests updated

`a_marker_is_read_only_at_a_logical_line_start` and
`a_marker_after_a_continued_directive_is_still_read` asserted over the removed
`names`/`files` fields. Both keep their load-bearing halves — a mid-line
marker-looking run is ordinary punctuation and now additionally must not push
the stack, and a macro's replacement list still contributes no token, now
asserted by token text as well as by position.

---

## 3. E01-F1-DENOM — a `Safe` unit and its unprojected functions

### 3.1 The census, and what the residual really was

E01-F1 recorded:

> a `Safe` unit may still carry a non-empty `not_projected` when a definition
> the raw scan enumerated is absent from the expansion (an `#if 0` body).

**Census of the current pinned baseline**
(`docs/reports/wave14-corpus/artifacts/diagnostics.json.gz`, 185 units):

| | count |
|---|---|
| `Safe` units | 2 (`ctraces/ctr_version.c`, `monkey-libevent/tls/tls_stub.c`) |
| …of those, with non-empty `not_projected` | **0** |
| units with non-empty `not_projected`, by status | 21 `Unknown`, 3 `Unsafe` |

So the residual is **not occurring** on the pinned corpus. That makes the
question a rule question, not a repair-the-corpus question, and reading the code
made the rule question sharper than the residual's own wording.

`analyze_legacy_c_translation_unit_selected_partial` intersects the selected
definition set with the definitions its own body finder reports in the
preprocessed text and **silently drops the rest**: `names.retain(…)`, with no
diagnostic, no projection failure and no row. A dropped definition lands in
`not_projected` with no explanation, and the unit can still be `Safe`. Two
causes reach that point:

1. a definition the enumeration's raw half kept but the preprocessor removed —
   the `#if 0` body the residual names. It is not in this translation unit at
   all;
2. a definition **present in the preprocessed text** that the body finder does
   not read. K&R is the named instance (E01-F1 already withheld `Safe` for it,
   at the projection layer, via `unanalyzable`) — but an *unnamed* one would be
   an analyzer blind spot, silently buying a whole-unit `Safe`.

Cause 2 is what the residual's wording missed, and it is the reason the current
rule is not defensible.

### 3.2 The decision

Hard invariant 2: *"`Safe` only after all obligations discharge … Projection
coverage is not safety coverage; unknowns stay in the denominator."* A
whole-unit `Safe` is a claim over the declared scope. A selected definition with
no analysed row was never looked at, so publishing `Safe` over it asserts safety
for a function the analysis never read.

**The rule changes.** `Safe` is published only when every selected definition
carries an analysed row, with the count and the first unread definition named in
the reason.

Three deliberate choices, each recorded rather than assumed:

* **The gate is in the gateway, not the projection layer.** E01-F1's
  withheld-`Safe` rule lives in `qcore-audit::projection` because it is about
  the *report's* denominator. This one is about the *certificate's* coverage, so
  it belongs where the certificate is issued. Every caller of
  `analyze_unrestricted_c11` is covered, not only the audit driver. E01-F1's
  projection-layer rule is retained unchanged — it is now partly redundant, and
  removing a second independent check to tidy up would be a weakening.
* **Only the `Safe` arm is gated.** An `Unsafe` verdict names a validated
  reachable violation in a function that *was* analysed; an unread definition
  elsewhere does not retract it, and downgrading would discard a real finding
  without gaining safety. An analysis-derived `Unknown` keeps its own reason —
  the gate cannot overwrite an explanation the analysis produced.
* **Cause 1 is refused too, deliberately** (recorded as **E01-TAIL-EXCUSE**).
  A `#if 0` body is genuinely not in the translation unit, so one could argue
  for excusing it. It is not excused, for two reasons: the two causes are
  indistinguishable at the publication site, and the report's own
  `not_projected_meaning` already conflates them ("raw source definitions absent
  from the projection, **including build-excluded definitions**"). Building a
  decided "absent from the expansion" excuse would need its own soundness
  argument for a case with measured reach 0; the strictly-refusing rule needs
  none. If that strictness ever costs a real unit, the excuse is a bounded
  follow-on with an obvious shape.

**Measured movement: zero.** Both `Safe` units on the pinned corpus have empty
`not_projected`, so no unit moves `Safe` → `Unknown`. The corpus re-measurement
(§4) confirms it. The rule has **measured reach 0**, stated plainly in the
spirit of C03-NFR's and C03-REACH's reach findings: it is a fail-closed gate
against a hole that is currently unoccupied, and its evidence is held-out
fixtures and Lean.

### 3.3 Two pinned expectations updated

Both asserted the gateway published `Safe` over a unit containing an unread
definition — the fail-open itself. Each keeps its load-bearing half, now read
out of the retained partial summary, and each gains a control.

1. **`crates/qcore-c-gateway/tests/unit/legacy_c.rs`**,
   `source_scope_uses_a_definition_that_survives_preprocessing`. Its real
   property — a compiled-out *entry selection* does not blind the unit, and the
   effective entry falls back to `subject` — is kept and asserted directly
   (`summary.entry == "subject"`, `summary.status == Safe`,
   `functions == ["subject"]`). The unit verdict is now `Unknown` with the
   ground named. A new control shows the same source without the compiled-out
   definition is still `Safe`.
2. **`crates/qcore-c-gateway/tests/e01_source_enumeration.rs`**,
   `knr_definition_with_declarations_is_enumerated_but_not_analysed`. Its real
   property — the analyzer is unchanged and `knr` acquires no unearned status —
   is kept (`!summary.functions.contains_key("knr")`), and `plain` is still
   asserted analysed and clean.

### 3.4 What the repair does not close

**E01-TAIL-CERT-SCOPE.** The gate is at *issuance*. A `LegacySafetyCertificate`
carries no record of the definition set it was selected over —
`check_legacy_safety_certificate` passes `certificate.checked_functions` as the
selection, so the coverage question is trivially satisfied at recheck. An
independent checker therefore cannot verify a certificate's coverage against the
audited scope. Carrying the denominator in the certificate would change every
certificate's shape and identity, so it is recorded, not repaired.

### 3.5 Held-out tests

`crates/qcore-c-gateway/tests/e01_denominator_coverage.rs`:

| Test | What it pins |
|---|---|
| `a_compiled_out_definition_withholds_the_unit_safe_claim` | cause 1 refused, with `supports_unit_safe()` first asserted **true** — E01-F1's own rule admits this unit, which is why the gate is needed — and a `Safe` control |
| `a_definition_the_body_finder_does_not_read_withholds_safe` | cause 2 refused at the gateway, on its own ground, so a caller that never runs the audit's projection layer is covered too |
| `an_unread_definition_does_not_retract_an_unsafe_finding` | precision: the `Unsafe` arm is untouched |
| `an_unknown_unit_keeps_its_own_reason` | precision: an analysis-derived `Unknown` reason is not overwritten |

---

## 4. Corpus parity

Re-measured through the canonical pinned route on a freshly prepared pinned
clone (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json),
and compared against the current baseline
[`docs/reports/wave14-corpus/artifacts/`](../wave14-corpus/artifacts) with
[`docs/reports/corpus-repin/diff_artifacts.py`](../corpus-repin/diff_artifacts.py).

<!-- CORPUS-RESULTS -->

---

## 5. E01-F1-COST — measured, and deliberately not built

### 5.1 What the extra preprocessing costs

Forty real compile commands from the pinned corpus database, each run three
times and the minimum taken, `/usr/bin/clang` 21.1.8, median source 12.8 KiB:

| | min | median | mean | max |
|---|---|---|---|---|
| published `clang -E -P` | 25.7 ms | **53.2 ms** | 52.2 ms | 80.5 ms |
| enumeration `clang -E` | 33.3 ms | **53.4 ms** | 54.9 ms | 84.0 ms |

Ratio of medians **1.002**. The enumeration therefore roughly *doubles* the
per-unit preprocessing cost. That is the honest headline number the residual
asked for.

### 5.2 …and how much of a run that is

`strace -f -e trace=execve` over the whole `chunkio-core` configured audit
(13 units, `qcore-audit`, exit 0, reproducing its recorded
0 Safe / 2 provisional Unsafe / 11 Unknown):

| Clang invocations | count | per unit |
|---|---|---|
| `-E` (enumeration) | **13** | **1.0** |
| `-E -P` (published) | 26 | 2.0 |
| `-fsyntax-only` | 26 | 2.0 |
| `--version` (discovery probe) | 94 | 7.2 |
| `-dumpmachine` (identity capture) | 1 | — |
| total Clang processes | 160 | 12.3 |

Wall clock for the same audit without `strace`: **20.79 s** (user 15.57 s) under
concurrent load. Thirteen enumeration preprocessings at the measured median are
≈ 0.69 s, so the enumeration is **20 % of the Clang invocations that do real
work** and **≈ 3 % of wall clock**.

### 5.3 Is a sound skip available? Yes. Is it worth building? No.

**The key would be sound.** The enumeration's output is a function of exactly
one subprocess invocation: the Clang program path, the complete argument vector,
the working directory and the piped source bytes. Equal key ⇒ identical process
⇒ identical output, under the premise that the filesystem is unchanged for the
duration of one audit run — which the `-E -P` run already assumes. Every
component of that key is bound by `CompileTaskIdentity` already (clang
program/version/target, the complete ordered argument vector, the dialect, the
ordered transported options, the compile working directory, the raw source
bytes, `source_input`), so this is a cache keyed on inputs the identity binds,
not a content-weaker one.

**It would save nothing.** The measurement above is decisive: `-E` runs
**exactly once per unit**, 13 for 13. E01-F1's `analyze_unrestricted_c11_with_
inventory` entry point — added for precisely this reason — already shares the
one enumeration between the driver, the projection and the source-contract
route, and the pinned Cycle 19 adapter enumerates once at
`compiler/examples/support/cycle19_compatibility.rs:608` and hands the inventory
down. There is no redundant call for a cache to eliminate on the production
route.

**Decision: record the measurement and leave it.** Adding a process-global memo
that eliminates zero invocations would introduce shared mutable state — with its
own staleness question across a long-running process — for no measured benefit.
The soundness argument above is recorded so that a future card which *does*
create redundancy can adopt it without re-deriving it.

### 5.4 Two larger redundancies found while measuring

Both are pre-existing, both are bigger than the cost E01-F1-COST names, and
neither is this card's to repair. Named for the follow-on ledger:

* **E01-TAIL-COST-DOUBLE-PP** — the *published* `-E -P` runs **twice per unit**
  (26 for 13). `qcore_audit::lib.rs:398` computes the effective preprocessing
  input for the compile-task identity, and
  `analyze_unrestricted_c11_with_inventory` computes the same text again for the
  analysis. This is twice the cost the enumeration adds.
* **E01-TAIL-COST-DISCOVERY** — `qcore_c_gateway::clang::command()` re-probes
  the executable with `clang --version` on every call, so nearly every real
  Clang invocation is preceded by a second Clang process: 94 probes for 65
  working invocations, at a measured median 28.1 ms each (≈ 2.6 s of the 20.8 s
  run). `QCORE_CLANG` being set does not skip the probe.

Sizing these two against §5.1: per unit, the enumeration adds ~53 ms; the
duplicate published preprocessing adds ~53 ms; the discovery probes add
~7 × 28 ms ≈ 200 ms.

---

## 6. Lean

Two new modules, audited by `AuditEnumerationAttribution.lean` and
`AuditDenominatorCoverage.lean`. `scripts/check_lean_audits.py` moves
**64 → 66** audits. No new axiom and no placeholder; every theorem depends on at
most `propext` and `Quot.sound`.

### 6.1 `QCore.EnumerationAttribution` (20 audited theorems)

Models the marker stream, the depth rule and E01-F1's name rule side by side.

* **`#line` is inert** — `relabel_preserves_depth`,
  `relabel_preserves_breakage`, `enter_then_leave_restores_depth`.
* **The two measured directions, both ways** —
  `a_relabelled_header_is_not_main_source` (stated for `<stdin>` *and* a
  declared file), `name_attribution_counted_the_relabelled_header`,
  `a_relabelled_tail_is_main_source`,
  `name_attribution_dropped_the_relabelled_tail`, and
  `the_two_rules_are_different` as one non-vacuity sentence.
* **The E01-F1-STDIN sentence** — `rename_preserves_depths` and
  `depth_attribution_is_transport_independent`: renaming *every* file in a
  stream, the primary input included, leaves the main-source token set
  unchanged, so a `<stdin>` unit and a declared-file unit are attributed by one
  rule. `name_attribution_is_not_transport_independent` shows the old rule had
  no such property.
* **Clang's preamble** — `the_preamble_contributes_no_main_source_token`, over
  the measured `<built-in>` / `<command line>` / forced-`-include` structure.
* **Fail-closed** — `an_underflowing_stream_is_unbalanced`,
  `an_unclosed_stream_is_unbalanced`, `a_well_formed_stream_is_balanced` (the
  non-vacuity control) and `breakage_is_absorbing`.
* **§L21.1 rule 1** — `the_unbalanced_ground_says_nothing_about_the_source`,
  `the_two_subjects_are_distinct`.

Explicit boundaries stated in the module header and **not** proved there: that
Clang emits flag `1` on entering, `2` on returning and neither for `#line`, and
that it opens its output with a marker naming the primary input. The streams are
transcriptions of §2.1, not extractions.

### 6.2 `QCore.DenominatorCoverage` (13 audited theorems)

Models the publication site with both rules side by side.

* **The gate is exactly full coverage** —
  `safe_is_published_exactly_when_clean_and_covered` (an `iff`, so the gate adds
  no refusal beyond the one it names), `an_unread_definition_withholds_safe`.
* **It only ever refuses** — `the_gate_only_refuses` (the repaired publication
  is `Safe` nowhere the original was not — hard invariant 1),
  `the_gate_is_not_vacuous`, and `only_the_uncovered_safe_arm_moves` stated once
  over every arm.
* **The other arms are untouched** —
  `an_unread_definition_does_not_retract_unsafe`,
  `an_analysis_unknown_keeps_its_reason`.
* **Nothing is lost** — `a_withheld_claim_retains_every_row`: a withheld claim
  costs the unit verdict and no per-function evidence.
* **The measured causes** — `both_measured_causes_are_refused`,
  `the_covered_control_is_still_safe`.
* **Relation to E01-F1's rule** — `the_rules_agree_on_the_knr_cause`, and
  `the_coverage_gate_is_strictly_stronger`: E01-F1's `supports_unit_safe`
  *admits* the compiled-out unit (established claim, nothing unanalyzable) while
  the coverage gate refuses it. That separation **is** the residual, stated as a
  theorem. `the_gate_refuses_every_unanalyzable_definition` gives the general
  direction over an arbitrary unit.

**E01-F1-COST needed no Lean module**, and the reason is recorded rather than
elided: nothing was implemented for it. Its content is a measurement (§5.1–5.2)
and a soundness argument for a key (§5.3) whose implementation was declined;
proving a property of code that does not exist would be a claim about a design,
not about this repository.

---

## 7. Validation

| Gate | Result |
|---|---|
| `cargo test --workspace` | <!-- WS --> |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | **one pre-existing failure, not this card's** (§7.1); clean everywhere else, and clean for `--workspace --lib --bins` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | <!-- P1 --> |
| `cargo run -p qcore-compiler --example reevaluation_followup` | <!-- P2 --> |
| `python3 -B scripts/check_lean_audits.py` | **66 audits passed** (64 → 66) |
| `cargo test --manifest-path providers/Cargo.toml` | <!-- PROV --> |
| `python3 -B -m unittest discover -s scripts/tests` | **134 passed, 1 failed by design** (§7.2) |
| pinned 185-unit audit | <!-- CORPUS --> |
| `chunkio-core` configured audit | exit 0; 13 units, 0 Safe / 2 provisional Unsafe / 11 Unknown — its recorded result, unchanged |

New tests: 9 integration (`e01_stdin_attribution.rs` 5,
`e01_denominator_coverage.rs` 4) plus 3 reader-level unit tests.

### 7.1 The pre-existing Clippy failure

`crates/qcore-c-gateway/tests/prod_contracts.rs:699` trips
`clippy::manual_contains`. The file is **byte-identical to base `19fbaa5`**
(`git diff 19fbaa5 -- crates/qcore-c-gateway/tests/prod_contracts.rs` is empty)
and this card adds no API it consumes, so the failure is not attributable here.
It is deliberately **not** repaired: the file sits in the gateway, where the
NEIGHBOURS and BORROW-RESIDUALS cards of this wave are working concurrently
(§8), and a drive-by edit there is a merge hazard for no benefit.

### 7.2 The expected Python failure

`scripts/tests/test_build_matrix_sections.py` compares the committed
`acceptance_matrix.json`'s `validation.lean_audits` against the produced count:
`['11.0.0', 64] != ['11.0.0', 66]`. This is the by-design failure a new Lean
audit produces, doubled because this card lands two. `acceptance_matrix.json` is
out of scope for this card; **the integrator must regenerate it.**

### 7.3 Load-flaky

`qcore-verifier-host`'s `a_declared_process_limit_stops_the_fixture_from_forking`
(the recorded P02 `RLIMIT_NPROC` flake, `docs/reports/remaining-p03/README.md`
§9.1) failed in the loaded aggregate run and <!-- FLAKE -->. Its assertion is
untouched.

---

## 8. Concurrency with other wave-15 cards

**NEIGHBOURS** and **BORROW-RESIDUALS** are working in
`crates/qcore-c-gateway` at the same time as this card. Recorded overlap, not
resolved here:

| File | This card | Overlap risk |
|---|---|---|
| `crates/qcore-c-gateway/src/legacy_c.rs` | the `Safe` arm of `analyze_unrestricted_c11_with_inventory` only (~20 added lines, no existing line deleted) | both cards work in the analyzer; the arm this card touches is the publication site, not the analysis |
| `crates/qcore-c-gateway/src/legacy_enumeration.rs` | rewritten attribution | neither card is recorded as touching the enumeration |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | one test body updated | shared test file |
| `crates/qcore-c-gateway/tests/prod_contracts.rs` | **not touched** (§7.1) | left to whichever card owns it |

Their corpus runs were observed running against the *shared* checkout's build
tree; this card's runs use its own worktree's prepared clone, so the inputs do
not interact.

---

## 9. Open boundaries

* **E01-TAIL-CERT-SCOPE** — a `Safe` certificate carries no record of the
  definition set it was selected over, so coverage is checked at issuance only
  (§3.4).
* **E01-TAIL-EXCUSE** — a definition removed by the preprocessor is refused
  along with a definition nothing read, deliberately (§3.2). A decided
  "absent from this translation unit" excuse is the shape of the follow-on if
  the strictness ever costs a real unit.
* **E01-TAIL-COST-DOUBLE-PP**, **E01-TAIL-COST-DISCOVERY** — the two measured
  redundancies, each larger than the enumeration's cost (§5.4).
* **E01-B2**, **E01-B3**, **E01-B4** — unchanged.
* Clang's marker-flag semantics and the analyzer's body finder remain tool and
  Rust boundaries, pinned behaviourally, not proved (§1.4, §6).

Nothing here establishes that any audited source is safe, and no raw unit is
promoted.
