# Opt-in `current` runs (Fluent Bit 046accf6)

Scripts: [../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit`, then runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh),
a verbatim copy of AMPERSAND-RUN-PARITY-OTHER-READERS' (itself
NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP's): `SOURCE_DATE_EPOCH=1767225600`,
`--monkey-linked`, the side tree's user load path, `--jobs 12`, over the
prepared inputs `/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`.
Logs and reports are in this worktree's `private/cba/run-*` (not committed).
The two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `d7540d746` | `b58220b1166a1e02fc4ef8229f4f6beb4ee0c60d48094f7cfec3ddb9a5f0c422` | 04:32:44 / 05:15:16 | 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `5f0d4b7b7` | `3a34188ad2799c738fbfcefd1371f950ce3c3a27e362eba30b233ad576925628` | 04:48:36 / 05:31:05 | 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

Console dependency lines, identical on both sides: `1172 Safe row(s): 440 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 548 on a declared assumption, …`.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
(`scripts/totals.py`), [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl)
(`scripts/moved.py`: `{"moves": {}}`), [changed-rows-base-tip.txt](changed-rows-base-tip.txt)
(`scripts/changed_rows.py`: empty, so no row changes status or leaf multiset).
