# Opt-in `current` runs (Fluent Bit 046accf6)

[../scripts/run.sh](../scripts/run.sh) extracts a `git archive` of the
measured revision into its own side tree, builds that tree's release
`quirin-audit` and runs [../scripts/opt-in-current.sh](../scripts/opt-in-current.sh).
Both scripts are copied from CG25-IN-PLACE-REGEX, with only the paths
changed. The run uses `SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the
side tree's user load path and `--jobs 12`, over
`/home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6`. Logs and
reports are in this worktree's `private/cgr/run-*` (not committed). The two
runs overlapped in time with each other, with three other cards' runs and
with the scoped gate.

| run | tree | sha256 (binary) | started / finished (UTC) | build / checkers / exit | Safe / Pending / Unknown / Unsafe | access rows | alone | access leaves |
|---|---|---|---|---|---|---:|---:|---:|
| base | `3ca85c7a6` | `34cba0eeb7331c692778d0f8c656217dccb61fd6903f24ec688fdb4aa42db0f9` | 00:11:10 / 01:48:50 | 0 / 0 / 0 | 1,171 / 2 / 3,623 / 8 | 2,118 | 215 | 8,992 |
| **tip** | `16ad6f982` | `51f1bf7c58c85e33059617327549feb2fda69ee335fcbb2195f7a8490e2c4ed2` | 00:41:13 / 02:06:54 | 0 / 0 / 0 | **1,172 / 2 / 3,622 / 8** | 2,118 | 215 | 8,992 |

The console's dependency line at base reads `1171 Safe row(s): … 547 on a
declared assumption …`. At tip it reads `1172 Safe row(s): … 548 on a
declared assumption …`. Every other figure on the line is unchanged.

The measured tip `16ad6f982` is the implementation commit. The later code
commits change only tests (`ba5746092` drops an unused test constant;
`ad79c44f7` updates `compiler/tests/contract_loader.rs`'s previous-mode
table), so the `quirin-audit` binary's sources are those measured.

Files:

* [totals-base.json](totals-base.json) and [totals-tip.json](totals-tip.json),
  from `scripts/totals.py`.
* [moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl), from
  `scripts/moved.py`: one move, `rd_regex_match` Unknown -> Safe, resting at
  tip on `regcomp`, `regerror`, `regexec` and `regfree`.
* [changed-rows-base-tip.txt](changed-rows-base-tip.txt), from
  `scripts/changed_rows.py`: that row alone, with its one base leaf removed
  (line 3048 of the preprocessed unit, `rdregex.c:127`, the `regerror` call:
  "use of in-place object `re` not certified by its initialiser status (rule
  74i)").
