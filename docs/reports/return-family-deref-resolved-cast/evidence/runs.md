# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh). They are
RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST's, with only this worktree's paths
changed in `run.sh`; `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies. `run.sh` extracts a `git archive` of the
measured revision into its own side tree and builds that tree's release
`quirin-audit`. `opt-in-current.sh` then runs it with
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rfd/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `3ca85c7a6` | `48250e55d7f492db46de338c38bdd72d61343cb295334ec2a3b30d1161c887de` | 00:08:26 / 01:48:04 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `1eae6f5e6` | `a35d86a8aa3be96b24bec42c5e3546032454d372f6ec4d3052b7268eab9be2a1` | 00:26:25 / 01:56:38 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

Both sides' build, `providers/package.py checkers` and route steps exited 0.
Both print the same console dependency line: `1171 Safe row(s): 440 on no
declaration, 124 only on validated declarations, 44 on a partly validated
declaration, 547 on a declared assumption, …`.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty, because no row changes its status or
  its leaf multiset.
