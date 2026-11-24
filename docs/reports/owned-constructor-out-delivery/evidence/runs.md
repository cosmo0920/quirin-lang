# Opt-in `current` runs (Fluent Bit 046accf6)

Scripts: [../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit`, then runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh),
a verbatim copy of CAST-BEFORE-AMPERSAND-READ-AS-BINARY's:
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path, `--jobs 12`, over the prepared inputs
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/ocod/run-*` (not committed). The two
runs overlapped in time, the scoped gate ran alongside them, and four other
cards' runs shared the host (load average up to 63), hence the long wall
times.

| run | tree / binary | sha256 (binary) | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `2c60986b0` | `ff7b6aefdfc8e8275897efe4c58427a7dbb4a1fc18d9fab3d30ed3e88f772dfa` | 11:51:36 / 13:35:46 | 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `fcf76ce01` | `f00920c25e9c10a35460ff7055abe09ee0e009ec02f79bb9d7e786615cfa56e5` | 12:11:04 / 13:46:50 | 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

`fcf76ce01` is the implementation commit; later commits change Lean, the
rule, test formatting and this report, not `crates/*/src`.

Console dependency lines, identical on both sides: `1172 Safe row(s): 440 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 548 on a declared assumption, …; 651 declaration(s) consulted
(76 validated, 53 partly validated)`; `refuted contracts: 0` on both.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
(`scripts/totals.py`), [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl)
(`scripts/moved.py`: `{"moves": {}}`), [changed-rows-base-tip.txt](changed-rows-base-tip.txt)
(`scripts/changed_rows.py`: 15 rows, every one `Unknown -> Unknown`, 21 leaves
re-worded), [census-base.txt](census-base.txt) (`scripts/census.py` over the
base report with prefix `discarded owned result`: 21 rows, 30 leaves, 21 of
them at a line spelling `*NAME = …`). The same census over the tip report:
5 rows, 9 leaves, none at a `*NAME` line. `scripts/row_leaves.py` prints a
row's leaves (used for the census table in the README).
