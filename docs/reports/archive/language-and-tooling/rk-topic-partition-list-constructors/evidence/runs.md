# Opt-in `current` runs (Fluent Bit 046accf6)

Script: [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), a
verbatim copy of `docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`
(`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path, `--jobs 12`), over the prepared inputs
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6` used directly
(their compile-database prefix was fixed before this card). One binary:
`cargo build --release -p qcore-audit --bin quirin-audit` at `8e7e8d509`
(this card changes no Rust), sha256
`ff817b5cbdf81673c01073b77d88d3e7a0f89b78cd6ff91489b7ec1781b8528d`. Each run
reads its own revision's side tree (`git archive`): the declarations live in
the tree the load path reads. Logs and reports in this worktree's
`private/rk/run-*` (not committed).

| run | tree | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---:|---|---:|---:|---:|
| base | `8e7e8d509` | 00:48:24 / 01:48:09 | 0 | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| **tip** | `e03c4835f` | 02:02:02 / 02:51:14 | 0 | **1,161 / 2 / 3,632 / 9** | **2,129** | **211** | **8,991** |

An earlier base attempt (`private/rk/run-base` first try, 00:47) exited 127
at once (the binary path was relative to the side tree); discarded, not
evidence.

Console dependency lines: base `1159 Safe row(s): 439 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 536 on
a declared assumption`, 641 declarations consulted (75 validated, 52 partly);
tip `1161 …, 538 on a declared assumption`, 646 consulted (76 validated, 53
partly). `refuted contracts: 0` on both.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
(`scripts/totals.py`, round 4's), [moved-rows.jsonl](moved-rows.jsonl)
(`scripts/moved.py`, round 4's), [changed-rows-base-tip.txt](changed-rows-base-tip.txt)
(`scripts/changed_rows.py`, CG10-MEMBER-STORE-RETAINED-BY's: every row whose
status or leaves differ, 83 rows), [callers-base.json](callers-base.json)
(`scripts/callers.py`: the 68 base rows naming a declared function
uncontracted; 0 on the tip). Its card set is the four functions the ledger entry and the first draft named; `rd_kafka_topic_partition_new`, declared afterwards, appears there as a co-blocking callee (11 rows, 7 beside the destructor).
