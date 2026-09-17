#!/usr/bin/env python3
"""Falsify each arm of the main-ref guard, and record which tests notice.

Card CARD-SCOPE-GUARD (wave 36). A green suite is evidence about the paths it
took; this script asks, of every arm of `.githooks/qcore-main-guard.sh` and of
the two hooks, whether any test actually drives it. It copies the hooks, breaks
one arm per mutant, re-runs `scripts/tests/test_main_guard_hooks.py` against the
broken copy through QCORE_MAIN_GUARD_HOOKS_DIR, and prints the tests that failed.

A mutant that kills no test is a finding, not a pass.

    python3 scripts/tests/mutate_main_guard.py [--out DIR]
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HOOKS = REPO_ROOT / ".githooks"

# name -> (file, pattern, replacement, what the arm is for)
MUTANTS: dict[str, tuple[str, str, str, str]] = {
    "guard-off": (
        "reference-transaction",
        r"\[ \"\$touched\" = 1 \] \|\| exit 0",
        "exit 0",
        "the whole reference-transaction guard",
    ),
    "no-ancestry-arm": (
        "qcore-main-guard.sh",
        r'_anc=\$\(qcore_ancestor_card_cwd "\$_root"\)',
        '_anc=""',
        "process ancestry (the incident shape)",
    ),
    "no-cwd-arm": (
        "qcore-main-guard.sh",
        r'if qcore_is_card_path "\$_cwd" "\$_root"; then',
        "if false; then",
        "the hook's own working directory",
    ),
    "no-git-dir-arm": (
        "qcore-main-guard.sh",
        r'"\$_common"/worktrees/agent-\*\)',
        '"$_common"/worktrees/never-matches-*)',
        "GIT_DIR naming a card worktree",
    ),
    "no-card-marker-arm": (
        "qcore-main-guard.sh",
        r'if \[ -n "\$\{QCORE_CARD_WORKTREE:-\}" \]; then',
        "if false; then",
        "the explicit card marker",
    ),
    "no-session-marker-arm": (
        "qcore-main-guard.sh",
        r'if \[ "\$_veto_child" = "true" \] && \[ -n "\$\{CLAUDE_CODE_CHILD_SESSION:-\}" \]; then',
        "if false; then",
        "the delegated-session marker",
    ),
    "authorisation-always-true": (
        "qcore-main-guard.sh",
        r'    \[ -n "\$_expected" \] \|\| return 1',
        "    return 0",
        "the token requirement",
    ),
    "agent-prefix-dropped": (
        "qcore-main-guard.sh",
        r'        "\$_root"/agent-\*\) return 0 ;;',
        '        "$_root"/*) return 0 ;;',
        "the agent- prefix that spares a named worktree",
    ),
    "deletion-reads-as-noop": (
        "reference-transaction",
        r'    if \[ "\$\{old:-\}" = "\$\{new:-\}" \] && \[ "\$\{new:-\}" != "\$QCORE_ZERO_OID" \]; then',
        '    if [ "${old:-}" = "${new:-}" ]; then',
        "the all-zeroes exclusion that catches a deletion",
    ),
    "pre-push-off": (
        "pre-push",
        r"\[ \"\$touched\" = 1 \] \|\| exit 0",
        "exit 0",
        "the pre-push guard",
    ),
    "pre-push-no-branch-arm": (
        "pre-push",
        r"    worktree-agent-\*\|claude/\*\|card/\*\)",
        "    never-a-branch-name-*)",
        "the card-branch check in pre-push",
    ),
}


def build_mutant(name: str, workdir: Path) -> Path:
    filename, pattern, replacement, _ = MUTANTS[name]
    target = workdir / name
    shutil.copytree(HOOKS, target)
    path = target / filename
    text = path.read_text()
    mutated, count = re.subn(pattern, replacement, text, count=1)
    if count != 1:
        raise SystemExit(
            f"mutant {name!r}: pattern matched {count} times in {filename}; "
            "the hook changed and this mutation no longer describes it."
        )
    path.write_text(mutated)
    path.chmod(0o755)
    return target


def run_suite(hooks_dir: Path) -> tuple[int, list[str]]:
    result = subprocess.run(
        [sys.executable, "-m", "unittest", "scripts.tests.test_main_guard_hooks", "-v"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        env={**__import__("os").environ, "QCORE_MAIN_GUARD_HOOKS_DIR": str(hooks_dir)},
    )
    killed = sorted(
        set(re.findall(r"^(?:FAIL|ERROR): (\S+)", result.stderr, flags=re.MULTILINE))
    )
    return result.returncode, killed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=None, help="write the table here")
    args = ap.parse_args()

    baseline_rc, baseline_killed = run_suite(HOOKS)
    lines = [f"baseline (unmutated hooks): exit {baseline_rc}, {len(baseline_killed)} failing"]
    if baseline_rc != 0:
        lines.append("BASELINE IS RED - mutation evidence means nothing until it is green.")
        print("\n".join(lines))
        return 1

    survivors = []
    with tempfile.TemporaryDirectory(prefix="main-guard-mutants-") as tmp:
        workdir = Path(tmp)
        for name in MUTANTS:
            target = build_mutant(name, workdir)
            rc, killed = run_suite(target)
            lines.append("")
            lines.append(f"mutant {name} - breaks {MUTANTS[name][3]}")
            lines.append(f"  exit {rc}, tests killed: {len(killed)}")
            for test in killed:
                lines.append(f"    {test}")
            if not killed:
                survivors.append(name)
                lines.append("    SURVIVED - no test drives this arm")

    lines.append("")
    lines.append(
        f"{len(MUTANTS) - len(survivors)} of {len(MUTANTS)} mutants killed; "
        f"survivors: {survivors or 'none'}"
    )
    text = "\n".join(lines)
    print(text)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text + "\n")
    return 1 if survivors else 0


if __name__ == "__main__":
    sys.exit(main())
