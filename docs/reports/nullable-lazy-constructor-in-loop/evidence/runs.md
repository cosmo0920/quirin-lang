# Opt-in `current` runs (Fluent Bit 046accf6)

Script: [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), a
verbatim copy of NULLABLE-GUARDED-RELEASE's (itself the constructors card's
and OPENSSL-SSL-CTX-CALLBACK-SLOTS'): `SOURCE_DATE_EPOCH=1767225600`,
`--monkey-linked`, the side tree's user load path, `--jobs 12`, over the
prepared inputs `/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`.
Each run reads its own side tree (`git archive REV`) and its own binary
(`cargo build --release -p qcore-audit --bin quirin-audit` in that tree).
Logs and reports are in this worktree's `private/nlc/run-*` (not committed).

| run | tree / binary | sha256 (binary) | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `1b7028430` | `87af1bf443239df8197fa69ea36af984141bcef6fd8e6bdcd8486f649c75428f` | 19:19:06 / 19:58:33 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| tip-1 (part B only) | `d671a4447` | `fc06f0be7e6f7eb6d9bfee5e8c5a01d3119fd36b6b2c4db22070e022b2aeee92` | 20:05:31 / 20:51:36 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip-2 (parts B and C)** | `9c479777c` | `a9d733a5cc7dc605dec9022eca44526cdd2f46e05283a4f4502f74bb5ed34430` | 21:00:20 / 21:36:52 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

Console dependency lines: base, tip-1 and tip-2 `1171 Safe row(s): 439 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 548 on a declared assumption`.

Files: [totals-base.json](totals-base.json), [totals-tip1.json](totals-tip1.json),
[totals-tip2.json](totals-tip2.json) (`scripts/totals.py`),
[moved-rows-base-tip2.jsonl](moved-rows-base-tip2.jsonl) (`scripts/moved.py`),
[changed-rows-base-tip1.txt](changed-rows-base-tip1.txt),
[changed-rows-base-tip2.txt](changed-rows-base-tip2.txt) (`scripts/changed_rows.py`),
[rows-base-tip1.txt](rows-base-tip1.txt), [rows-base-tip2.txt](rows-base-tip2.txt)
(`scripts/rows_compare.py`: the eight rows' named leaves).

**Diagnosis runs (not evidence for any figure).** A temporary trace build
(an uncommitted `NLC_TRACE` print in the custody work list, reverted before
any commit) printed each transfer and join that made a traced binding unknown
or possibly null. Run (i): a local harness over the four units preprocessed
with their compile commands (`clang -E -P`) and the librdkafka constructor
contracts only; run (ii): the real opt-in route above restricted by
`--selection` to `rdkafka_cgrp.c`, `rdkafka_mock_cgrp.c`,
`rdkafka_mock_handlers.c`, `rdkafka_sticky_assignor.c` (base tree, trace
binary). The base selection run reproduced all 11 nullable leaves. The
findings in README §1 (the `rd_list_new` refusal poisoning
`assignment_with_metadata`, the `&&` frame reading in ShareFetch, the unbound
later declarator in ConsumerGroupHeartbeat, the uncontracted arms, the
flag-correlated copy in `balance`) are from these traces.
