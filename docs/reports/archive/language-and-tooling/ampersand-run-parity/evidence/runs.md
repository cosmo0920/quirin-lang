# Opt-in `current` runs (Fluent Bit 046accf6)

Scripts: [../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit`, then runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh),
a verbatim copy of NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP's (itself
NULLABLE-GUARDED-RELEASE's): `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`,
the side tree's user load path, `--jobs 12`, over the prepared inputs
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and reports
are in this worktree's `private/arp/run-*` (not committed). The two runs
overlapped in time.

| run | tree / binary | sha256 (binary) | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---:|---|---:|---:|---:|
| base | `9193b196a` | `f04c27aa7c8ac51fc61c738df73f0cc59e1b2719d7ad918c4a6eb26929f24e1f` | 01:02:45 / 02:03:26 | 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `96218cc75` | `3e7eea510fb0072a5d4b5d3e1265a8461b64e0a8ad98a6fcbf170017463a3553` | 01:28:27 / 02:26:35 | 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

Console dependency lines: base `1171 Safe row(s): 439 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 548 on a
declared assumption`; tip `1172 Safe row(s): 440 on no declaration, …` (the
rest equal).

An earlier tip run of `73cfa12e4` was stopped before it finished (its tree
lacked `legacy_call_instantiation`'s change) and is discarded.

Files: [totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)
(`scripts/totals.py`), [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl)
(`scripts/moved.py`), [changed-rows-base-tip.txt](changed-rows-base-tip.txt)
(`scripts/changed_rows.py`).
