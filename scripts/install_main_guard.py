#!/usr/bin/env python3
"""Activate the QCore-C main-ref guard in this checkout.

Card CARD-SCOPE-GUARD (wave 36), discharging CARD-SCOPE-SELF-MERGE. The guard
itself is `.githooks/reference-transaction` (plus `.githooks/pre-push`); this
script is the one step a delegated card cannot take for the integrator, because
activation is repository-local configuration rather than versioned content.

What it does, all of it local to `.git/config`:

  * `core.hooksPath`  -> an ABSOLUTE path to this checkout's `.githooks`, so a
    worktree whose checked-out commit predates the guard is still covered.
  * `qcore.mainguard.token` -> a fresh random token. An update to
    `refs/heads/main` is authorised only by `QCORE_ALLOW_MAIN_UPDATE` carrying
    exactly this value, one command at a time.
  * `qcore.mainguard.vetochildsession` -> measured, not assumed: if the
    integrator's own environment sets CLAUDE_CODE_CHILD_SESSION, that signal
    would refuse the integrator too, so it is recorded disabled and said so.

It then SELF-TESTS the installed hook by running it directly on a synthetic
transaction in each of three environments, touching no reference.

Usage:
    python3 scripts/install_main_guard.py            # activate (idempotent)
    python3 scripts/install_main_guard.py --rotate   # new token
    python3 scripts/install_main_guard.py --verify   # report, change nothing
"""

from __future__ import annotations

import argparse
import os
import secrets
import stat
import subprocess
import sys
from pathlib import Path

HOOK_NAMES = ("reference-transaction", "pre-push")
SHARED = "qcore-main-guard.sh"
TOKEN_KEY = "qcore.mainguard.token"
VETO_KEY = "qcore.mainguard.vetochildsession"
ZERO = "0" * 40


def git(*args: str, cwd: Path | None = None) -> str:
    out = subprocess.run(
        ["git", *args],
        cwd=str(cwd) if cwd else None,
        capture_output=True,
        text=True,
        check=False,
    )
    if out.returncode != 0:
        raise SystemExit(
            f"git {' '.join(args)} failed ({out.returncode}): {out.stderr.strip()}"
        )
    return out.stdout.strip()


def git_config_get(key: str, cwd: Path) -> str | None:
    out = subprocess.run(
        ["git", "config", "--get", key],
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    return out.stdout.strip() if out.returncode == 0 else None


def refuse_if_linked_worktree(top: Path) -> None:
    """Activation belongs in the main checkout; a card must not perform it."""
    git_dir = Path(git("rev-parse", "--absolute-git-dir", cwd=top))
    common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir", cwd=top))
    if git_dir != common:
        raise SystemExit(
            "refusing: this is a linked worktree "
            f"({git_dir}). Activate the guard in the main checkout ({common.parent})."
        )
    if "/.claude/worktrees/" in str(top):
        raise SystemExit(f"refusing: {top} is an agent worktree.")


def run_hook(
    hook: Path, state: str, stdin: str, env: dict, cwd: Path
) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(hook), state],
        cwd=str(cwd),
        input=stdin,
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )


def self_test(top: Path, hooks: Path, token: str) -> list[tuple[str, bool, str]]:
    """Drive the installed hook on a synthetic transaction. Touches no ref.

    Provenance is supplied as a real working directory and as the explicit card
    marker. It is deliberately NOT supplied as an inherited PWD: a POSIX shell
    re-derives PWD at startup, so a hook cannot read a caller's PWD, and a
    self-test built on one would prove nothing.
    """
    head = git("rev-parse", "HEAD", cwd=top)
    line = f"{ZERO} {head} refs/heads/main\n"
    hook = hooks / "reference-transaction"

    base = {k: v for k, v in os.environ.items()}
    for key in ("QCORE_ALLOW_MAIN_UPDATE", "QCORE_CARD_WORKTREE", "GIT_DIR"):
        base.pop(key, None)
    base["PWD"] = str(top)

    with_token = dict(base)
    with_token["QCORE_ALLOW_MAIN_UPDATE"] = token
    marked = dict(with_token)
    marked["QCORE_CARD_WORKTREE"] = "1"

    results = []
    r = run_hook(hook, "prepared", line, base, top)
    results.append(
        (
            "an unauthorised update to refs/heads/main is refused",
            r.returncode != 0,
            f"exit {r.returncode}",
        )
    )
    r = run_hook(hook, "prepared", line, marked, top)
    results.append(
        (
            "a card-marked update is refused even carrying the token",
            r.returncode != 0 and "qcore-card-worktree-marker" in r.stderr,
            f"exit {r.returncode}",
        )
    )

    card_root = top / ".claude" / "worktrees"
    if card_root.is_dir():
        probe = card_root / "agent-mainguard-selftest"
        probe.mkdir(exist_ok=True)
        try:
            r = run_hook(hook, "prepared", line, with_token, probe)
            results.append(
                (
                    "an update from a card worktree directory is refused with the token",
                    r.returncode != 0 and "cwd-in-card-worktree" in r.stderr,
                    f"exit {r.returncode}",
                )
            )
        finally:
            probe.rmdir()
    else:
        results.append(
            (
                f"(skipped: {card_root} does not exist, so the card-path arm was not driven)",
                True,
                "skipped",
            )
        )

    r = run_hook(hook, "prepared", line, with_token, top)
    results.append(
        (
            "the integrator's authorised update is allowed",
            r.returncode == 0,
            f"exit {r.returncode}" + (f": {r.stderr.strip()}" if r.returncode else ""),
        )
    )
    return results


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--rotate", action="store_true", help="generate a fresh token")
    ap.add_argument("--verify", action="store_true", help="report only; change nothing")
    args = ap.parse_args()

    top = Path(git("rev-parse", "--show-toplevel", cwd=Path.cwd()))
    hooks = top / ".githooks"
    if not (hooks / "reference-transaction").is_file():
        raise SystemExit(f"refusing: {hooks}/reference-transaction is missing.")

    if args.verify:
        configured = git_config_get("core.hooksPath", top)
        token = git_config_get(TOKEN_KEY, top)
        veto = git_config_get(VETO_KEY, top)
        print(f"core.hooksPath        : {configured or '(unset - guard INACTIVE)'}")
        print(f"{TOKEN_KEY} : {'set' if token else '(unset - main is frozen)'}")
        print(f"{VETO_KEY}: {veto or '(unset - defaults to true)'}")
        active = bool(configured) and Path(configured) == hooks and bool(token)
        print(f"\nguard active          : {'yes' if active else 'NO'}")
        return 0 if active else 1

    refuse_if_linked_worktree(top)

    for name in (*HOOK_NAMES, SHARED):
        path = hooks / name
        mode = path.stat().st_mode
        path.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    git("config", "core.hooksPath", str(hooks), cwd=top)

    token = git_config_get(TOKEN_KEY, top)
    if token is None or args.rotate:
        token = secrets.token_hex(16)
        git("config", TOKEN_KEY, token, cwd=top)
        print(f"{TOKEN_KEY}: {'rotated' if args.rotate else 'generated'}")
    else:
        print(f"{TOKEN_KEY}: kept (use --rotate to replace)")

    child_marker = bool(os.environ.get("CLAUDE_CODE_CHILD_SESSION"))
    git("config", VETO_KEY, "false" if child_marker else "true", cwd=top)
    if child_marker:
        print(
            f"{VETO_KEY}: DISABLED - this environment sets CLAUDE_CODE_CHILD_SESSION,\n"
            "  so vetoing on it would refuse the integrator too. The guard rests on\n"
            "  worktree provenance and the token. Measured, not assumed."
        )
    else:
        print(f"{VETO_KEY}: enabled")

    print(f"core.hooksPath: {hooks}")
    print("\nself-test (no reference is touched):")
    failures = 0
    for label, ok, detail in self_test(top, hooks, token):
        print(f"  [{'ok' if ok else 'FAIL'}] {label} ({detail})")
        failures += 0 if ok else 1
    if failures:
        print("\nGUARD NOT PROVEN. Do not rely on it.", file=sys.stderr)
        return 1

    print(
        "\nThe guard is active. From now on, EVERY advance of refs/heads/main in\n"
        "this checkout needs the token on that one command:\n\n"
        '  QCORE_ALLOW_MAIN_UPDATE="$(git config --get qcore.mainguard.token)" \\\n'
        "      git merge --no-ff <card-branch>\n\n"
        "Do NOT `export` it: every process launched from that shell, agents\n"
        "included, would inherit it and the guard would be off without saying so.\n"
        "Refusals and authorised updates are logged to\n"
        f"  {top}/.git/qcore-main-guard.log\n"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
