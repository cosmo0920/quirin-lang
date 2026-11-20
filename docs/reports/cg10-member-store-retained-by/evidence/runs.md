# Opt-in `current` runs (Fluent Bit 046accf6)

Script: `docs/reports/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`
(`SOURCE_DATE_EPOCH=1767225600`, `--monkey-linked`, the side tree's user load
path, `--jobs 12`). Side tree: `git archive 0111ddf38` for every run. Prepared
inputs: round 3's worktree `private/fb-046accf6` (read-only). Binaries:
`cargo build --release -p qcore-audit --bin quirin-audit --features
qcore-c-gateway/indirect-census`, exit 0 each. Logs and reports in this
worktree's `private/m/run-*` (not committed).

| run | revision | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---:|---|
| base | `0111ddf38` | `99ed0224…` | 11:50:54 / 12:31:20 | 0 | 1,140 / 2 / 3,655 / 7 |
| tip1 | `1e746f9ba` | `55198bdd…` | 12:19:42 / 13:02:21 | 0 | 1,150 / 2 / 3,645 / 7 |
| **tip2** | `a07d6e8b5` | `93b45b67…` | 13:06:26 / 13:39:19 | 0 | **1,150 / 2 / 3,645 / 7** |

An earlier base attempt (`private/m/run-base-failed-relpath`, exit 127) passed
a relative binary path and ran nothing; not evidence.

Console dependency lines: base `1140 Safe row(s): 432 on no declaration, 124
only on validated declarations, 44 on a partly validated declaration, 524 on a
declared assumption`; tip1 and tip2 `1150 Safe row(s): 439 on no declaration,
124 …, 44 …, 527 on a declared assumption`. `refuted contracts: 0` on every run.

Comparisons (`../../cg10-indirect-access-sole-residuals-r3/scripts/compare.py`,
whose row key is unit file, function and occurrence; its own totals count the
rows that key reaches): [compare-base-tip1.json](compare-base-tip1.json),
[compare-base-tip2.json](compare-base-tip2.json),
[compare-tip1-tip2.json](compare-tip1-tip2.json); every changed row, base vs
tip2: [changed-rows-base-tip2.txt](changed-rows-base-tip2.txt)
(`../scripts/changed_rows.py`).
