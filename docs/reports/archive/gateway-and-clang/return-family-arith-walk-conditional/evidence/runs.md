# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), copied from
RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS. Only `run.sh`'s worktree, private
directory and script paths are changed. `opt-in-current.sh`, `totals.py`,
`moved.py`, `changed_rows.py` and `census.py` are verbatim copies.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` runs
it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's
user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rawc/run-*` and are not committed.
The two runs overlapped in time, and the scoped gate ran alongside the tip
run.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `1187c3f6d` | `247acb786acecd542e5d0e1ddd2d3da70514acf58b1e0a75879ede2f5bb96083` | 09:16:38 / 10:23:36 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `d09d2d5d9` | `3bfe15d0b32168caa0f68e05afc44d5f36e857884e88781f4b7348a850ca66fc` | 09:32:07 / 10:38:41 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides, the build, `providers/package.py checkers` and route steps
exited 0, and both print the same console dependency line (`1171 Safe
row(s): 440 on no declaration, 124 only on validated declarations, 44 on a
partly validated declaration, …`).

The evidence files are:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty, because no row changes its status
  or its leaf multiset.
