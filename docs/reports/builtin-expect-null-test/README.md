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
`QCore.BuiltinExpectCondition`), widens the hint to any integer literal, pins
the reading with fixtures, and re-homes :1712.

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

What changes under this card's rule: no row. The widened hint (`0L`, `0x0`,
…) occurs at no site of any route.

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
