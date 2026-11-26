# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh), copied from
RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC. Only `run.sh`'s worktree
and output paths changed; `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` runs
it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's
user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rfrs/run-*` and are not committed.
The two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `03aadb221` | `9467011f1fcb935703b8c7cee771d2ad2aa55da9398fa63fcfbf096f23792487` | 07:15:07 / 08:26:15 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `0efdfd0c4` | `b93651bedfc152bbe477af2455a2ac5597a113522ac23718a28bfae74e11b607` | 07:35:15 / 08:37:50 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides the build, `providers/package.py checkers` and route steps
exited 0, and both print the same console dependency line (`1171 Safe
row(s): 440 on no declaration, 124 only on validated declarations, …`).

* [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json):
  `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl): `scripts/moved.py`,
  `{"moves": {}}`.
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt):
  `scripts/changed_rows.py`; empty, because no row changes its status or its
  leaf multiset.
