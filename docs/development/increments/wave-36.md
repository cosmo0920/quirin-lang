# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`CARD-SCOPE-GUARD` (2026-09-18)** answers `CARD-SCOPE-SELF-MERGE` with a
mechanism rather than another instruction, and its interest is how much of the
obvious design was **falsified by measurement**. The wave-33 defect was a *local*
ref update — `git -C <main checkout> merge` from an agent worktree — so a
`pre-push` hook never sees it and the git process's own directory is the main
checkout. The guard is therefore a **`reference-transaction`** hook (git ≥ 2.28;
host 2.53.0), which runs on every ref update from every worktree and aborts by
exiting non-zero in the `prepared` state, with `pre-push` beside it for the
remote door. The first design read the caller's exported `PWD`, reasoning that
`git -C` moves the process but not the variable; measured, **git rewrites `PWD`
for its own hooks**, and measured again, **a POSIX shell re-derives `PWD` from
`getcwd()` at startup**, so that arm could not fire on any input at all — it was
removed as vacuous rather than shipped green, with a test now pinning the
measurement so it is not re-added. What does survive `git -C` is **process
ancestry**: the card's own shell and session remain ancestors of that git
process, so the arm that catches the incident shape reads `/proc` upward for an
ancestor whose directory is a delegated worktree (Linux only, stated). A third
measurement: `update-ref -d refs/heads/main` reaches the hook as
`0000…→0000…`, and the first draft's `old == new` no-op filter therefore
**deleted `main` and reported success** — red before green. A fourth decided the
veto's scope: all 15 Agent-tool worktrees on this host are `agent-<hex>` while
the delegating session sits in a *named* one, so vetoing every
`.claude/worktrees/` path would have refused the integrator and left nobody able
to advance `main`. Two layers stand: a provenance veto no authorisation
overrides, and a token making refusal the default for everyone, with the
`CLAUDE_CODE_CHILD_SESSION` signal **calibrated at activation** against the
integrator's own environment instead of assumed. 27 tests drive it in throwaway
repositories, and **11 of 11 mutants die, no survivors**. Two things are not
claimed: the hooks are **inert** until the integrator runs
`python3 scripts/install_main_guard.py` once in the main checkout — a step a card
cannot take, which is why the residual is amended and kept open rather than
retired — and a hook is **bypassable by intent**, so this guards a card's mistake
and nothing more. `CLAUDE.md`'s worktree hygiene also gains the harness hazard
that surfaced mid-card: an isolation worktree with an unchanged tree is removed
even while its agent's background jobs run (`WORKTREE-AUTOREMOVAL-PRISTINE-TREE`).
No crate source, no `proof/`, no pinned artifact: the workspace is byte-identical
to `main`'s at `60958bf0`. `docs/reports/card-scope-guard/README.md`.

