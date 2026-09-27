# W46 contract and typed-statement expansion

Integration in progress from `207cebe4`. The [prediction](PREDICTION.md)
precedes merging the three independent Sol 6 medium implementation cards.
Astra owns gap selection, source-boundary decisions and combined measurements.

## Worktree sweep

The user requested outdated worktrees be swept while the cards run. Seven
clean, merged and unlocked worktrees were removed using `git worktree remove`:
`fb-system-audit-cards`, `agent-a31872259c7e9850d`, `hungry-thompson-1740da`,
`w46-access-path-census-launch-8ba0f2`, and the previous expansion's
`qcore-w46-fluent-events`, `qcore-w46-libc-events`, `qcore-w46-metric-trace`.
All branch refs remain. Eight locked worktrees, one worktree containing an
untracked fault-injection report, the main checkout and all three active cards
are preserved.

Before removal, ignored private evidence was archived and every preserved file
was checked by SHA-256 (symlink targets checked separately). Only disposable
`target/`, `providers/target/`, `proof/.lake/` and `__pycache__` caches were
excluded. Each removal rechecked a clean status, unchanged head, ancestry in
main, unlocked state and archive digest. All seven removals exit 0. Inventory,
per-file manifests, tar archives and removal results live in
`private/w46-next/worktree-sweep/`. This directory is the relocation index for
old worktree-relative private log paths; frozen reports are unchanged.
Observed free space on the main filesystem rose from 97 GiB to 157 GiB during
the sweep (other builds were running, so this is not exact per-file accounting).
