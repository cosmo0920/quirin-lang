# Opt-in `current` runs (Fluent Bit 046accf6)

[../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit` and runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh).
Both scripts are copied from CAST-BEFORE-AMPERSAND-READ-AS-BINARY, with only
the paths changed. The run uses `SOURCE_DATE_EPOCH=1767225600`,
`--monkey-linked`, the side tree's user load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/cgr/run-*` (not committed). The two
runs overlapped in time, and the scoped gate ran alongside them.

| run | tree | sha256 (binary) | started / finished (UTC) | build / checkers / exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---|---|---:|---:|---:|
| base | `2c60986b0` | `ad87c3c0c4a76567538862ed9236c15e2cbd0a557a24472c3e811019a99113ad` | 11:56:54 / 13:40:19 | 0 / 0 / 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `a7e8bea5b` | `a37504a2ee1d48aafb8519e42d8c840ebd2849a48e2889b090288a2dde0e0a15` | 12:10:17 / 13:47:54 | 0 / 0 / 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

The console's dependency line at base reads `1172 Safe row(s): … 548 on a
declared assumption …`. At tip it reads `1171 Safe row(s): … 547 on a
declared assumption …`. Every other figure on the line is unchanged.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: one move, `rd_regex_match` Safe -> Unknown.
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`: that row alone, with one leaf added at line
  3048 of the preprocessed unit (the `regerror` call), "use of in-place
  object `re` not certified by its initialiser status (rule 74i)".
