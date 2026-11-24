# Opt-in `current` runs (Fluent Bit 046accf6)

[../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit` and runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh).
Both scripts are copied from CG25-IN-PLACE-REGEX with only the paths changed.
The run uses `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side
tree's user load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. The logs and
reports are in this worktree's `private/cgr/run-*` and are not committed.
The two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree | sha256 (binary) | started / finished (UTC) | build / checkers / exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---|---|---:|---:|---:|
| base | `3ca85c7a6` | `b7b86d1d3bf691a18424c85004c25f28b6e52be063a96bee8246d65458e14505` | 00:15:04 / 01:53:26 | 0 / 0 / 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `10770a4db` | `052fb6e534db27f4951199bd1bcbf49a0f3f784e62aab18d2aae0d941942532b` | 00:22:41 / 01:56:13 | 0 / 0 / 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

The console's contract-dependency line is identical on both sides: `1171
Safe row(s): 440 on no declaration, 124 only on validated declarations, 44
on a partly validated declaration, 547 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`. It records no move (`{"moves": {}}`).
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty: no row changed status or leaf
  multiset.
