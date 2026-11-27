# Opt-in runs

`sh docs/reports/out-slot-conditional-delivery/scripts/run.sh NAME TAR`: each
revision from its own `git archive` tree and its own release `quirin-audit`,
`current --monkey-linked` over the prepared Fluent Bit `046accf6` inputs
(`private/fb-046accf6`, read-only), `--jobs 12`, outputs in the worktree's
`private/oscd/run-NAME/`.

| run | tree | binary sha256 | started / finished (UTC) | exit |
|---|---|---|---|---:|
| base `43410b230` | `private/oscd/side-base` | `f0ba65d6…49cc40b2` | 00:23:38 / 01:28:11 | 0 |
| tip `80f747e4b` | `private/oscd/side-tip` | `2927c194…13bfcbd` | 00:53:19 / 01:55:17 | 0 |

Totals ([totals-base.json](totals-base.json), [totals-tip.json](totals-tip.json)):
both 1,187 Safe / 2 Pending / 3,607 Unknown / 8 Unsafe; access rows 2,105,
alone 215, leaves 8,917.

[moved-rows-base-tip.jsonl](moved-rows-base-tip.jsonl): `{"moves": {}}`.
[changed-rows-base-tip.txt](changed-rows-base-tip.txt): empty, no row changes
status or any leaf. [named-rows-tip.txt](named-rows-tip.txt): the eight named
rows' leaves at the tip (identical at base).
