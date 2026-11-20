# Opt-in `current` runs (Fluent Bit 046accf6)

Script: `docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`
(`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path, `--jobs 12`). Binaries: `cargo build --release -p qcore-audit --bin
quirin-audit` (`scripts/build.sh`), exit 0 each. Logs and reports in this
worktree's `private/r4/run-*` (not committed).

**Prepared inputs.** The inputs round 3 prepared were moved from round 3's
worktree to `/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`,
but their compile databases, generated headers and Monkey inputs still name
the removed worktree's path, so a run over them fails (`private/r4/run-base-failed-stale-paths`:
`compile database contains no selected cmetrics sources`, exit 1; not
evidence). This card copied them to its own `private/r4/fb-046accf6` and
rewrote that one path prefix in the 2,168 text files naming it (sources
unchanged; `__FILE__` strings in the analysed texts carry the new path).
Base and tip read the same copy, and the base reproduces the statuses of
CG10-MEMBER-STORE-RETAINED-BY's tip (1,150 / 2 / 3,645 / 7).

**Side trees.** The declarations live in the tree the load path reads, so
each run reads its own revision's tree: base `git archive 9b107caf1`, tip
`git archive df45ff563` (the code of `e9aeaa511`, the rule 66 text, Lean and
the regenerated inventory). The tip binary was built from `e9aeaa511`'s code;
`b5758c406` is `cargo fmt` only.

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `9b107caf1` | `6acb901c…` | 18:13:23 / 19:10:53 | 0 | 1,150 / 2 / 3,645 / 7 | 2,154 | 222 | 9,066 |
| **tip1** | `df45ff563` | `40a3b51e…` | 19:40:24 / 20:22:12 | 0 | **1,157 / 2 / 3,638 / 7** | **2,148** | **218** | **9,046** |

Console dependency lines: base `1150 Safe row(s): 439 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 527 on
a declared assumption`; tip `1157 …, 534 on a declared assumption`, 641
declarations consulted (638 on the base), 74 validated on both. `refuted
contracts: 0` on both; 181 configurations; checker plugins 150 bindings, all
`Pending`, on both.

Files: [totals-base.json](totals-base.json), [totals-tip1.json](totals-tip1.json)
(`scripts/totals.py`), [moved-rows.jsonl](moved-rows.jsonl)
(`scripts/moved.py`), [changed-rows-base-tip1.txt](changed-rows-base-tip1.txt)
(CG10-MEMBER-STORE-RETAINED-BY's `changed_rows.py`: every row whose status or
leaves differ).

**Probes before the prediction** (unit-scoped `current --selection`,
`scripts/sel.sh`, not the measurement): `sel-jitter-*` (seven librdkafka
units, base tree vs the tree with `rd_jitter` pinned: one move,
`rd_kafka_broker_update_reconnect_backoff`), `sel-xxh-c2` (`rdxxhash.c` with
the finalizers pinned, base binary: no move).
