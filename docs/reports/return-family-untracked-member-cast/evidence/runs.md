# Opt-in `current` runs (Fluent Bit 046accf6)

The scripts are [../scripts/run.sh](../scripts/run.sh) and
[../scripts/opt-in-current.sh](../scripts/opt-in-current.sh). They are
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST's. Only this worktree's paths are
changed in `run.sh`. `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are verbatim copies.

`run.sh` extracts a `git archive` of the measured revision into its own side
tree and builds that tree's release `quirin-audit`. `opt-in-current.sh` runs
it with `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's
user load path and `--jobs 12`, over the prepared inputs in
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/rmc/run-*` and are not committed. The
two runs overlapped in time, and the scoped gate ran alongside them.

| run | tree / binary | sha256 (binary) | route started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `7bfa0414b` | `837e09281e2ede5c86a2f0e0f778ee23f9fb25e1f35f36cd0d8aa5eeacf6ae14` | 03:58:24 / 04:47:04 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `c3228a355` | `c201a6b356f40daca5a1815f061f4d23f41b2ff1dfbfbef8f833e82f90bd9259` | 04:13:22 / 05:02:54 | 0 | **1,171 / 2 / 3,623 / 8** | 2,118 | 215 | 8,992 |

On both sides, the build, `providers/package.py checkers` and route steps
exited 0. Both sides print the same console dependency line: `1171 Safe
row(s): 440 on no declaration, 124 only on validated declarations, 44 on a
partly validated declaration, 547 on a declared assumption, …`. The route
reports 181 units, from librdkafka (88), cmetrics (31), Monkey (30),
ctraces (17), chunkio (13) and two Fluent Bit `src/` files.

The evidence files are:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`;
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: `{"moves": {}}`;
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`. It is empty, because no row changes its status
  or its leaf multiset.
