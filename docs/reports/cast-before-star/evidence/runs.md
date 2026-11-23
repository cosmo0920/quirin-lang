# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), the same as
CAST-BEFORE-AMPERSAND-READ-AS-BINARY's with only this worktree's paths changed.
`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` then
runs it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side
tree's user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/cbs/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `7450d69af` | `b6934c7809ec458a37df6b5b8e7cc2d025fc9e20b09907feffa9d3f135ef2356` | 07:42:29 / 08:28:11 | 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `404136f21` | `e47f3a8aaf9e19029a9303510d82f8dc272dde003b5657c75226d2898a38f822` | 07:57:41 / 08:38:06 | 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

Both sides print the same console dependency line: `1172 Safe row(s): 440 on
no declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 548 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty: no row changes status or leaf
  multiset.
