# Opt-in `current` runs (Fluent Bit 046accf6)

Scripts: [../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit`, then runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh),
a verbatim copy of CAST-BEFORE-AMPERSAND-READ-AS-BINARY's:
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path, `--jobs 12`, over the prepared inputs
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/ldi/run-*` (not committed). The host
was shared with other cards' runs (load average above 40), so wall times are
not comparable.

| run | tree / binary | sha256 (binary) | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `2c60986b0` | `38c8cff5745135045bf262422260e5574cb3e9a205a8fdcaef5fd2da10d9cfb7` | 11:56:03 / 13:38:05 | 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `f4f5a11a1` | `725f97123255cc5e103460257e7d067fb7ae56a5ee51335858d01c31d4f055bc` | 13:47:53 / 14:23:43 | 0 | **1,173 / 2 / 3,621 / 8** | 2,115 | 214 | 8,980 |

Console dependency lines: base `1172 Safe row(s): 440 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 548 on a
declared assumption, 0 consulting a held or withdrawn declaration, 16 only on
a null premise (42 carrying one)`; tip the same with `1173 Safe row(s)` and
`17 only on a null premise (43 carrying one)`.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
(`scripts/totals.py`), [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl)
(`scripts/moved.py`), [changed-rows-base-tip.txt](changed-rows-base-tip.txt)
(`scripts/changed_rows.py`), [candidates-base.jsonl](candidates-base.jsonl)
(`scripts/candidates.py`, the prediction's census over the base report), and
[pins-at-base.txt](pins-at-base.txt) (`scripts/base-pins.sh`: the tip's new and
changed test files run against the base sources).
