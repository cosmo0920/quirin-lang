# Opt-in `current` runs (Fluent Bit 046accf6)

[../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit` and runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh).
Both scripts are copied from FRAME-HOLDER-SHADOWED-NAME with only the paths
changed. The run uses `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the
side tree's user load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. The logs and
reports are in this worktree's `private/nkh/run-*` and are not committed.
The two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree | sha256 (binary) | started / finished (UTC) | build / checkers / exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---|---|---:|---:|---:|
| base | `43410b230` | `18de15cc9e8511f1881f63cccb7e543f5ff56d6579bc44ded9a758a47319394b` | 00:30:59 / 01:36:06 | 0 / 0 / 0 | 1,187 / 2 / 3,607 / 8 | 2,105 | 215 | 8,917 |
| **tip** | `182767251` | `e5ec3a8c03a0ac7ea496eea64aab093745f772ab1b36df7ad906720bc9c55d24` | 00:42:36 / 01:42:36 | 0 / 0 / 0 | **1,187 / 2 / 3,607 / 8** | 2,105 | 215 | 8,917 |

The console's contract-dependency line is identical on both sides: `1187
Safe row(s): 448 on no declaration, 125 only on validated declarations, 45
on a partly validated declaration, 552 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`. It records no move (`{"moves": {}}`).
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty: no row changed status or leaf
  multiset.
