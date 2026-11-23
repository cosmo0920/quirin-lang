# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), CAST-BEFORE-STAR-READ-AS-BINARY's
with only this worktree's paths changed (`opt-in-current.sh`, `totals.py`,
`moved.py` and `changed_rows.py` are verbatim copies). `run.sh` extracts a
`git archive` of the measured revision into its own side tree and builds that
tree's release `quirin-audit`. `opt-in-current.sh` then runs it with
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rfs/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `5cfca8170` | `6b9656d830f2296e9acf21bdba8dc66ea464b5e3d2cd8fd3ac2e9374c308df44` | 12:15:05 / 13:48:15 | 0 | 1,172 / 2 / 3,622 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `9de28d1ec` | `f1db90844e48eaa7a14d7af040f7f722a0601ff61fd9573cd863918d00b6d27a` | 12:15:08 / 13:47:17 | 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

Both sides' build, `providers/package.py checkers` and route steps exited 0,
and both print the same console dependency line: `1172 Safe row(s): 440 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 548 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty: no row changes status or leaf
  multiset.
