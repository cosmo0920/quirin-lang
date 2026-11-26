# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), copied from
RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC. Only this worktree's paths
are changed in `run.sh`. `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` runs
it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's
user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rfc/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside the tip run.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `03aadb221` | `e2238891469c6a0b0c5e77f8406aecddb81d6a1bf3b55fe434e3175e7369713e` | 07:14:55 / 08:27:13 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `c88cf5357` | `967483ec08f36c009093c88ef60e65b187f089f8afad500363fcb87cd576fc2b` | 07:36:10 / 08:39:50 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides, the build, `providers/package.py checkers` and route steps
exited 0, and both print the same console dependency line (`1171 Safe
row(s): 440 on no declaration, 124 only on validated declarations, 44 on a
partly validated declaration, 547 on a declared assumption, …`).

The evidence files are:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty, because no row changes its status
  or its leaf multiset.
