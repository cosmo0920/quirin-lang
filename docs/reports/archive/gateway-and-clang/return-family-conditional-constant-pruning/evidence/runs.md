# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), copied from
RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS. Only the worktree path, the
private directory and the report directory change in `run.sh`.
`opt-in-current.sh`, `totals.py`, `moved.py`, `changed_rows.py` and
`census.py` are verbatim copies.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` runs
it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's
user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rfcp/run-*` and are not committed.
The two runs overlapped in time, and the scoped gate was queued alongside
them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `1187c3f6d` | `4d99c23d478c76959b6de522fdff9c00ad3bec368e4ba95ee81ff7c40e1ac489` | 09:16:51 / 10:22:25 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `a197c18ca` | `7d7b1afbc2debca74879272ee71821b6b74fad31a1b16e2949ce1df56ecd4061` | 09:38:32 / 10:39:41 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides, the build, `providers/package.py checkers` and route steps
exited 0. Both print the same console dependency line (`1171 Safe row(s):
440 on no declaration, 124 only on validated declarations, 44 on a partly
validated declaration, …`).

The evidence files are:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty, because no row changes its status
  or its leaf multiset.
