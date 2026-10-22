# Obsolete worktree sweep — 2026-10-22

Removed the completed, clean, fully merged and inactive Clang-transport and
scheduler checkouts. The scheduler checkout is recoverably archived in Codex;
the Clang source remains reachable through main. The [receipt](sweep.json)
records exact heads, decisions, exits, archive identity and post-sweep inventory.

Before removal, both complete ignored `private/` directories were atomically
moved on the same filesystem, preserving their directory device/inode, into:

- `/home/cosmo/Documents/GitHub/qcore-c/private/worktree-sweep-20261022-1/audit-ir-clang-transport/private/`
- `/home/cosmo/Documents/GitHub/qcore-c/private/worktree-sweep-20261022-1/scheduler-lifetime-contracts/private/`

These are custody relocations, not new measurements. Original receipts retain
their historical absolute paths; the receipt maps each old prefix to its new
preserved prefix. Recreate the recorded source checkout and restore that directory
to the old prefix before reproducing a path-bound run. Raw evidence was retained;
only obsolete checkout files and rebuildable caches were discarded. The exact
preservation and removal logs are beside the preserved directories.

Git first refused ordinary Clang worktree removal (exit128) because the checkout
contains a submodule. After checking clean merged source, preserved evidence and
clean deinitialization, scoped `git worktree remove --force` passed0. Codex then
confirmed the scheduler archived attachment. Shared submodule registration was
restored, and the retained guarded/logger Libevent checkouts still report the
exact pinned revision.

Retained the two unmerged audit worktrees, active Claude/locked agent worktrees,
and guarded/logger paths required by the explicitly deferred
[guarded supplemental card](../guarded-contract-adoption/SUPPLEMENTAL_CARD.md).
No guarded implementation, compiler/native validation or remote publication
resumed. The frozen matrix remains unchanged; this maintenance does not establish
acceptance for the later merged source.
