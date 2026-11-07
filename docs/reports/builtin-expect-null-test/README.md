# BUILTIN-EXPECT-NULL-TEST

Card registered by CURRENT-UNSAFE-TRIAGE (USER DECISION 2026-11-07, verbatim:
"Register four as implementation cards"). Base `82ec0e80b`. The registered
claim: snappy.c:1712's leak on the `if (unlikely(!outbuf))` arm rests on a
null test inside `__builtin_expect` that the projection does not read
(W44-LEAK-PIN-AUDIT row 226, written before W44-BRANCH-CONDITION-CALLS
landed).

**Finding.** That premise no longer holds. Rule 5h's Boolean reading
(`legacy_condition_desugar.rs`, W44-BRANCH-CONDITION-CALLS) already reads
`__builtin_expect(E, K)` with a digit-literal `K` as `E`, and every site on
the runs has such a hint. The :1712 leaf is reproduced, line for line, with
the wrapper removed from the whole unit (§2): it rests on the two-pass loop's
generation join, LOOP-REACQUISITION-LEAK-ACCUSATION's mechanism. This card
therefore states and proves the existing reading (rule 5h amended, new
`QCore.BuiltinExpectCondition`), pins the reading with fixtures, and
re-homes :1712. It changes no analyser code: `legacy_condition_desugar.rs`
stays byte-identical to the base (§3).

## 1. Census

[`scripts/census.py`](scripts/census.py) preprocesses every unit of each run
with the gateway's own arguments (`-x c <dialect> -E -P` plus the recorded
options; the `-P` bytes equal the report's `preprocessed_source_sha256` for
all 217 configured units) and lists every `__builtin_expect` /
`__builtin_expect_with_probability` token inside a main-file function
definition, whatever macro spelled it. Inputs: the user's current run
(fluent-bit `046accf6`, `build/cycle19/clang/046accf6…/reports/`) and the
`baseline:7b726587b` corpus, system and slice reports. Corpus units come from
the pinned compile databases (identity unrecorded in that report). Rows:
[`census.json.gz`](census.json.gz).

| route | units | units with a site | sites |
| --- | ---: | ---: | ---: |
| current (046accf6) | 186 | 74 (63 librdkafka, 11 Monkey) | 2,905 |
| corpus (66910c10) | 185 | 74 | 2,905, identical rows |
| system | 15 | 0 | 0 |
| slice | 16 | 0 | 0 |

Every site is `__builtin_expect` (none `_with_probability`), and every hint is
`0` (5,582 of both routes), `1` (162), `(0)` (52) or `(1)` (14): all admitted
by the base reading. Positions (current; the corpus is the same): `if`
2,893, `while` 10, `do … while` 1, one initialiser
(`rdkafka_cgrp.c:5577`); every controlling-expression site is in a Boolean
leaf position.

Operand shape against what the run reports on the site's line, current route
(`today` is keyed by the wrapper's own line, so a refusal of another statement
on that line, as in the `rd_kafka_buf_read_*` macros, also counts; §2 is the
exact answer):

| operand | read | a condition refusal on the line |
| --- | ---: | ---: |
| place test (`!p`, `p`, `p == NULL`, `_r == 0`) | 185 | 517 (508 are `_r == 0` in `rd_kafka_buf_read_*` lines) |
| comparison | 160 | 10 |
| bit test or other call-free scalar (`(rk->rk_conf).debug & 0x100`) | 913 | 1 |
| call-free connective | 39 | 3 |
| call-bearing (one or more calls) | 26 | 1,008 |
| call-bearing connective | 6 | 19 |
| assignment inside (`(r = f(…))`) | 6 | 12 |

The snappy site: `snappy.c:1706 if (unlikely(!outbuf))`, operand `( ! outbuf )`,
hint `0`, **read**, no diagnostic on its line.

What changes under this card: no row (no analyser code changes). A hint
spelled `0L`, `0x0` or with any other suffix or prefix, which the reading
refuses, occurs at no site of any route.

## 2. Wrapped against bare

The census harness (`wrapper_census_harness`, ignored in
`crates/qcore-c-gateway/tests/builtin_expect_null_test.rs`) analyses each unit
with a site twice from its recorded compile configuration, main-source
definitions in scope and the repository's contract manifest: as configured,
and with `-D__builtin_expect(e,c)=(e)`, which replaces every wrapper by its
operand on the same line of the unit's own preprocessed text. It ran from a
release build at the base's hint reading.

**snappy.c** ([`harness-snappy.json`](harness-snappy.json)): all 53 functions
identical, wrapped and bare. `rd_kafka_snappy_java_uncompress` is `Unsafe`
with exactly the current run's thirteen diagnostics on the same lines,
including the leak at -P line 20509 (snappy.c:1712, acquired at 20506 =
:1705) and the `loop re-acquisition` refusal at :1705. The leaf at :1712 does
not depend on the wrapper.

**The other 73 units with a site** were not run: the run was queued in an
audit slot behind other cards' jobs for about 2.5 hours and then cancelled by this
card. It needs no slot to be re-run: the list is written by
`census.py --harness-list`, and the harness reads
`QCORE_BUILTIN_EXPECT_CENSUS` (list), `QCORE_BUILTIN_EXPECT_CENSUS_OUT`,
`QCORE_BUILTIN_EXPECT_CENSUS_CONTRACTS` (`cycle19-resource-contracts.json`)
and `QCORE_BUILTIN_EXPECT_CENSUS_JOBS`. It is evidence about the base reading
only; this card changes no analyser code, so no measured row depends on it.
The fixtures show the two directions it would find: identical decisions for
null and comparison leaves, and two shapes where the wrapped form is decided
and the bare one is not (`BARE-CONNECTIVE-NULL-LEAF-REFINEMENT`).

## 2a. Gate

`python3 scripts/gate_scope.py --base 82ec0e80b --out private/gate-N`:

* attempt 1: stopped by the card before its Rust suite, after a unit-test
  spelling bug was found in review (no result);
* attempt 2 (`6a8f19039`, full mode): exit 1. Rust suite 616 units, 9,280
  passed, 4 failed (`reading_site_domain` 1, `libc_reading_site_evidence` 3:
  the pinned artifact digest of `legacy_condition_desugar.rs`), 17 ignored;
  fmt, clippy, both reevaluation probes exit 0; Lean 415 audits exit 0;
  Python 685 tests, 1 failed: `test_gate_slot_starvation …
  test_holders_a_draining_suite_and_a_waiter` (`0.18 not greater than 0.2`
  CPU seconds, host load);
* attempt 3 (`3eda75769`, scoped: no gateway source differs from the base):
  exit 1. fmt and five clippy targets exit 0; Rust suite 11 units, 55
  passed, 0 failed, 1 ignored (the harness); Lean 415 audits exit 0; Python
  685 tests, the same timing test failed (`0.2 not greater than 0.2`, load
  average 26 on 16 CPUs) and passes in isolation (1 test, 1.97 s, exit 0).

## 3. What changed

* **Rule 5h amended** ([text](../../agents/rules/5h.md)): the reading of
  `__builtin_expect(E, K)` as `E` is stated, with its hint premise (exactly one
  token of decimal digits, which has no evaluation event) and its conversion premise
  (the operand's truthiness survives the implicit conversion to `long`: Clang
  rejects a pointer operand, a null or comparison test is `int` 0/1, an
  integer operand no wider than `long` keeps zero and non-zero on LP64;
  stated, not checked: `BUILTIN-EXPECT-OPERAND-CONVERSION`). No new rule id.
* **Rust: no analyser change.** The first version of this card widened the
  hint to any integer literal in `legacy_condition_desugar.rs` (commits
  `6dd88c8aa`, `6a8f19039`). That file is a pinned artifact of the libc
  reading-site evidence (`docs/reports/libc-reading-site-evidence/bindings.json`,
  `scalar-reader`, SHA-256 `02c929cf…`), so gate attempt 2 failed
  `reading_site_domain` and `libc_reading_site_evidence` on the artifact
  mismatch. Rebinding that digest would change the bindings manifest every
  published receipt binds; as MK-EVENT-CELL-CROSS-FUNCTION did, the card keeps
  the file byte-identical to the base instead and drops the widening, which
  no site needed. Follow-up `BUILTIN-EXPECT-LITERAL-HINT-SPELLINGS`.
* **Lean**: `QCore.BuiltinExpectCondition` (audited by
  `proof/AuditBuiltinExpectCondition.lean`, 11 theorems, no project axiom):
  `the_wrapper_has_the_operand_value`, `the_reading_is_c_evaluation`,
  `the_desugared_reading_is_c_evaluation` (composed with
  `QCore.ConditionCallDesugar.the_desugaring_is_c_evaluation`),
  `a_leaf_decides_as_its_bare_test`, `the_argument_order_is_unobservable`,
  `an_effectful_hint_refuses`, `a_refused_operand_refuses`,
  `a_nested_effectful_hint_refuses`, `an_effectful_hint_is_observable`,
  `admitted_spellings_are_literals`, `vectors_match`.
* **Fixtures** (`crates/qcore-c-gateway/tests/builtin_expect_null_test.rs`):
  * `likely_and_unlikely_around_null_tests_refine_as_the_bare_test`: six
    wrapped/bare pairs (`unlikely(!p)`, `unlikely(p == 0)`, `likely(p != 0)`,
    `likely(!!(p))`, an assignment-then-test, a `do … while (unlikely(…))`)
    with identical status and diagnostics, all Safe; two pairs whose non-null
    arm returns with the object live, both a leak wrapped and bare; and two
    positions where the wrapped form is decided and the bare one is not
    (below);
  * `the_hint_spellings_bind_the_admission_vector`: through the reader,
    class by class (`0`/`(0)`/`1`/`(1)` read; `0L`/`0UL`/`0x0`/`1ll`,
    `k`/`s->k`, `opaque()`/`k++`/`k = 1`, `-1`/`(long)0`/`0.5` refused),
    equal to `QCore.BuiltinExpectCondition.vectors`;
  * `an_operand_that_is_not_a_bare_test_stays_refused`: `unlikely(!p |
    opaque())`, `unlikely(opaque())` keep the bare operand's refusal;
    `__builtin_expect_with_probability` refuses;
  * `a_hint_with_a_side_effect_stays_refused`: `opaque()`, `k++`, `k = 1`,
    `k`, `-1`, `(long)0`;
  * `the_snappy_failure_arm_refines_the_allocation_away`: snappy's shape
    without the loop, with the pass test and with a one-generation loop is
    Safe; in the two-pass loop the wrapped and bare forms are identical and
    the leaf at the failure arm's return is accompanied by one at the
    pre-allocation return, the generation join's signature.

## 4. Measurement

[PREDICTION.md](PREDICTION.md): no move on any route; the integrator
measures. No Safe row is added, so no per-function justification is owed.

## 5. Follow-ups

* `BUILTIN-EXPECT-OPERAND-CONVERSION` (registered): bind rule 5h's
  conversion premise with the Clang AST (the operand's implicit cast to
  `long` is an integral cast from a type no wider than `long`, or the operand
  is Boolean-valued), refusing a floating or wider operand; today a premise.
* `BUILTIN-EXPECT-LITERAL-HINT-SPELLINGS` (registered): admit a suffixed or
  prefixed integer-literal hint (`0L`, `0x0`, `1UL`), effect-free but refused
  today; it needs an edit of the pinned `legacy_condition_desugar.rs` with the
  reading-site artifact digest rebound (and every receipt that binds the
  manifest re-measured). No site of any run has such a hint.
* `BARE-CONNECTIVE-NULL-LEAF-REFINEMENT` (registered): the bare forms
  `if (!p && n) return; if (!p) return; k_free(p);` and `return (p != 0) ?
  p : 0;` / `return p ? p : 0;` report a leak (provisional Unsafe) although
  no path leaks; the same tests under the wrapper are desugared by rule 5h
  and decided Safe. An older whole-condition route accepts the call-free
  connective or `?:` without refining its null leaf. Not sized on the runs.
* `LOOP-NULL-STORE-AFTER-RELEASE` (registered): `while (p != 0) { k_free(p);
  p = 0; }` and the `for` form with `if (p != 0) { k_free(p); p = 0; }` report
  a double release (bare and wrapped alike); the straight-line form is Safe.
  The null store after the release does not reach the loop's back edge. Not
  sized on the runs.
* `LOOP-REACQUISITION-LEAK-ACCUSATION` (amended): snappy.c:1712 is that
  card's seventh leaf, not this card's.
* `LEAK-UNDECIDED-NULL-TEST` (retired): its remaining snappy half rested on
  the same misattribution.
