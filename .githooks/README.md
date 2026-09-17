# `.githooks` — the QCore-C main-ref guard

Versioned git hooks. They are **inert until activated**, because `core.hooksPath`
is repository-local configuration and cannot be shipped in a commit:

```sh
python3 scripts/install_main_guard.py     # once, in the MAIN checkout
```

| file | what it is |
|---|---|
| `reference-transaction` | the guard. Runs on every ref update in the repository, from every worktree and every command, and refuses an unauthorised update to `refs/heads/main` |
| `pre-push` | the second door: a push to a remote's `main` updates no local ref, so the transaction hook never sees it |
| `qcore-main-guard.sh` | the shared decision logic. Not a hook — git runs only files whose names match a hook name |

Two layers decide, in this order:

1. **Provenance veto.** A process rooted in a delegated card worktree
   (`<main>/.claude/worktrees/agent-*`) is refused, with or without
   authorisation — so an inherited or copied token does not help a card.
2. **Positive authorisation.** Everyone else must carry the repository-local
   token on that one command:
   `QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)" git merge …`.
   The default is refusal.

Refusals and authorised updates are appended to `.git/qcore-main-guard.log`.

A hook is bypassable by intent (`--no-verify`, overriding `core.hooksPath`,
writing `.git/refs` by hand). This guard is against a delegated agent's
**mistake**. Design, measurements, tests and the full boundary:
`docs/reports/card-scope-guard/README.md`.
