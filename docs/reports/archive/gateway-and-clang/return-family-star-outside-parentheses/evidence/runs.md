# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh). They are
RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST's. Only `run.sh` changes: it uses
this worktree's paths and `private/rso/`. `opt-in-current.sh`, `totals.py`,
`moved.py` and `changed_rows.py` are verbatim copies. `run.sh` extracts a
`git archive` of the measured revision into its own side tree and builds that
tree's release `quirin-audit`. `opt-in-current.sh` then runs it with
`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rso/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `7678fe5f1` | `66842cf3dbecee2b7347e01396a235fd97edd58bde77efb1f06808b2e51be14f` | 02:11:05 / 03:25:41 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `12ca1c33c` | `a54613a66b76f6fa6826846e90ebe60e936563925ebe2b0ce6dca4d248ba4457` | 02:22:43 / 03:33:03 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides the build, `providers/package.py checkers` and route steps exited
0. Both print the same console dependency line: `1171 Safe row(s): 440 on no
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
