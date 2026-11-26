# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh). They are
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST's, with only this worktree's paths
changed in `run.sh`; `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies. `run.sh` extracts a `git archive` of
the measured revision into its own side tree and builds that tree's release
`quirin-audit`. `opt-in-current.sh` then runs it with
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/tgq/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside the tip run.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `7bfa0414b` | `78dd9a0a89d67de3fa95f9a0e3452719f287b9d6adda8b246e85e6778c618bbe` | 04:53:23 / 05:40:43 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `8ed7a7a7f` | `3bfb383fd5758da7f68e225e79539f8b749c2cbe5aea6056f1bfdba7da2f1dd0` | 05:06:29 / 06:03:40 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

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
