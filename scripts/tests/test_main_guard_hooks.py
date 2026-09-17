"""The main-ref guard, driven in a throwaway repository.

Card CARD-SCOPE-GUARD (wave 36), discharging CARD-SCOPE-SELF-MERGE.

Every test here builds a real git repository in a temporary directory, with a
real linked worktree under `.claude/worktrees/`, and attempts to advance
`refs/heads/main` by the same command shapes a delegated card has available —
including the one the wave-33 incident actually used, `git -C <main checkout>
merge`. Nothing in this module touches the QCore-C repository's own refs.

The hooks under test are this repository's versioned `.githooks`. Set
QCORE_MAIN_GUARD_HOOKS_DIR to point the suite at a mutated copy instead; that is
how each arm of the guard was shown load-bearing (see the card report §4).
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
HOOKS_DIR = Path(os.environ.get("QCORE_MAIN_GUARD_HOOKS_DIR", REPO_ROOT / ".githooks"))
ZERO = "0" * 40

# The guard reads provenance out of the environment, so the suite must not leak
# its own. These are cleared for every git invocation and re-supplied per case.
SCRUBBED = (
    "QCORE_ALLOW_MAIN_UPDATE",
    "QCORE_CARD_WORKTREE",
    "CLAUDE_CODE_CHILD_SESSION",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
)


class GuardFixture(unittest.TestCase):
    """A repository with main, one agent worktree, and the guard installed."""

    def setUp(self) -> None:
        root = Path(tempfile.mkdtemp(prefix="qcore-main-guard-"))
        self.addCleanup(self._remove, root)
        # A temporary directory whose own path matched the agent-worktree rule
        # would make every case vacuously refused.
        self.assertNotIn("/.claude/worktrees/", str(root))
        self.root = root
        self.repo = root / "repo"
        self.repo.mkdir()

        self.git("init", "-b", "main", cwd=self.repo)
        self.git("config", "user.email", "card@example.invalid", cwd=self.repo)
        self.git("config", "user.name", "Card", cwd=self.repo)
        self.git("config", "commit.gpgsign", "false", cwd=self.repo)
        (self.repo / "base.txt").write_text("base\n")
        self.git("add", "base.txt", cwd=self.repo)
        self.git("commit", "-m", "base", cwd=self.repo)
        self.base_sha = self.rev_parse("refs/heads/main")

        # The card's worktree, where a delegated card works.
        self.worktree = self.repo / ".claude" / "worktrees" / "agent-x"
        self.git(
            "worktree", "add", "-b", "worktree-agent-x", str(self.worktree), "main",
            cwd=self.repo,
        )

        # Activate the guard the way install_main_guard.py does.
        self.token = "t0kenf0rtest"
        self.git("config", "core.hooksPath", str(HOOKS_DIR), cwd=self.repo)
        self.git("config", "qcore.mainguard.token", self.token, cwd=self.repo)
        self.git("config", "qcore.mainguard.vetochildsession", "true", cwd=self.repo)

        # A commit on the card's own branch: the work a card legitimately does.
        (self.worktree / "card.txt").write_text("card work\n")
        self.git("add", "card.txt", cwd=self.worktree, pwd=self.worktree)
        self.git("commit", "-m", "card work", cwd=self.worktree, pwd=self.worktree)
        self.card_sha = self.rev_parse("refs/heads/worktree-agent-x")
        self.assertNotEqual(self.card_sha, self.base_sha)

    @staticmethod
    def _remove(root: Path) -> None:
        subprocess.run(["chmod", "-R", "u+w", str(root)], check=False)
        subprocess.run(["rm", "-rf", str(root)], check=False)

    def env(self, *, pwd: Path, token: bool = False, extra: dict | None = None) -> dict:
        env = {k: v for k, v in os.environ.items() if k not in SCRUBBED}
        env["PWD"] = str(pwd)
        if token:
            env["QCORE_ALLOW_MAIN_UPDATE"] = self.token
        env.update(extra or {})
        return env

    def git(
        self,
        *args: str,
        cwd: Path,
        pwd: Path | None = None,
        token: bool = False,
        extra: dict | None = None,
        check: bool = True,
    ) -> subprocess.CompletedProcess:
        result = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            capture_output=True,
            text=True,
            check=False,
            env=self.env(pwd=pwd or cwd, token=token, extra=extra),
        )
        if check and result.returncode != 0:
            self.fail(f"git {' '.join(args)} failed: {result.stderr}")
        return result

    def git_from_card_shell(
        self, *args: str, token: bool = False, extra: dict | None = None
    ) -> subprocess.CompletedProcess:
        """Run git from a shell whose own directory is the card's worktree.

        This is the wave-33 incident's process shape: the card's shell sits in
        its worktree and points git at the main checkout. The trailing `exit`
        keeps the shell from exec'ing itself away, so it stays an ancestor —
        as a card's real shell and session processes do.
        """
        quoted = " ".join(f"'{a}'" for a in args)
        return subprocess.run(
            ["sh", "-c", f"git {quoted}; status=$?; exit $status"],
            cwd=str(self.worktree),
            capture_output=True,
            text=True,
            check=False,
            env=self.env(pwd=self.worktree, token=token, extra=extra),
        )

    def run_hook_directly(
        self,
        name: str,
        state: str,
        stdin: str,
        *,
        cwd: Path | None = None,
        token: bool = False,
        extra: dict | None = None,
    ) -> subprocess.CompletedProcess:
        """Drive one hook on a synthetic transaction, as the installer does.

        The directory is passed as a real working directory, not as an inherited
        PWD: a POSIX shell re-derives PWD from getcwd() at startup, so a hook
        written in sh cannot see a caller's PWD at all (card report §3.2).
        """
        where = cwd or self.repo
        return subprocess.run(
            [str(HOOKS_DIR / name), state],
            cwd=str(where),
            input=stdin,
            capture_output=True,
            text=True,
            check=False,
            env=self.env(pwd=where, token=token, extra=extra),
        )

    def rev_parse(self, ref: str) -> str:
        return self.git("rev-parse", ref, cwd=self.repo).stdout.strip()

    def guard_log(self) -> str:
        path = self.repo / ".git" / "qcore-main-guard.log"
        return path.read_text() if path.exists() else ""

    def assertMainUnmoved(self) -> None:
        self.assertEqual(self.rev_parse("refs/heads/main"), self.base_sha)


class ReferenceTransactionGuard(GuardFixture):
    def test_the_wave33_incident_shape_is_refused(self) -> None:
        """A card in its own worktree running git against the main checkout.

        The process directory of that git is the MAIN checkout and git rewrites
        PWD for its own hooks, so neither directory names the card's worktree.
        Process ancestry does: the card's shell is still the parent.
        """
        result = self.git_from_card_shell(
            "-C", str(self.repo), "merge", "--ff-only", "worktree-agent-x"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("main-ref guard: REFUSED", result.stderr)
        self.assertIn("ancestor-in-card-worktree", result.stderr)
        self.assertMainUnmoved()
        self.assertIn("refused-provenance", self.guard_log())

    def test_the_incident_shape_is_refused_even_carrying_the_token(self) -> None:
        """Provenance beats authorisation, so an inherited token does not help."""
        result = self.git_from_card_shell(
            "-C", str(self.repo), "merge", "--ff-only", "worktree-agent-x",
            token=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("delegated card-worktree provenance", result.stderr)
        self.assertMainUnmoved()

    def test_without_a_card_ancestor_that_shape_rests_on_the_token_alone(self) -> None:
        """The reach of the provenance layer, pinned rather than overstated.

        A process pointed at the main checkout with NO card worktree anywhere in
        its directory or its ancestry is indistinguishable from the integrator's
        own shell. It is refused — but by the authorisation layer, and a valid
        token admits it. This is MAIN-GUARD-ANCESTRY-REACH in the card report.
        """
        refused = self.git(
            "-C", str(self.repo), "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, check=False,
        )
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("no valid QCORE_ALLOW_MAIN_UPDATE", refused.stderr)
        allowed = self.git(
            "-C", str(self.repo), "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, token=True, check=False,
        )
        self.assertEqual(allowed.returncode, 0, allowed.stderr)

    def test_updating_main_from_inside_the_worktree_is_refused(self) -> None:
        """The other door: a ref update whose process directory IS the worktree."""
        result = self.git(
            "push", ".", "HEAD:refs/heads/main",
            cwd=self.worktree, pwd=self.worktree, token=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cwd-in-card-worktree", result.stderr)
        self.assertMainUnmoved()

    def test_an_unauthorised_update_from_the_main_checkout_is_refused(self) -> None:
        """The default is refusal for everyone, not only for an agent."""
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no valid QCORE_ALLOW_MAIN_UPDATE authorisation", result.stderr)
        self.assertMainUnmoved()

    def test_a_wrong_token_is_refused(self) -> None:
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, extra={"QCORE_ALLOW_MAIN_UPDATE": "guess"}, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertMainUnmoved()

    def test_the_integrators_authorised_merge_advances_main(self) -> None:
        """The guard must not be a wall: the authorised path has to work."""
        result = self.git(
            "merge", "--no-ff", "-m", "Merge card", "worktree-agent-x",
            cwd=self.repo, token=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(self.rev_parse("refs/heads/main"), self.base_sha)
        self.assertIn("authorised-main-update", self.guard_log())

    def test_deleting_main_is_refused(self) -> None:
        result = self.git(
            "update-ref", "-d", "refs/heads/main",
            cwd=self.worktree, pwd=self.worktree, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertMainUnmoved()

    def test_a_card_may_still_commit_on_its_own_branch(self) -> None:
        """The guard is narrow: it refuses one ref, not a card's own work."""
        (self.worktree / "more.txt").write_text("more\n")
        self.git("add", "more.txt", cwd=self.worktree, pwd=self.worktree)
        result = self.git(
            "commit", "-m", "more card work",
            cwd=self.worktree, pwd=self.worktree, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(self.rev_parse("refs/heads/worktree-agent-x"), self.card_sha)

    def test_a_no_op_update_of_main_is_not_refused(self) -> None:
        """Writing main's own value back advances nothing, so it is not a merge."""
        result = self.git(
            "update-ref", "refs/heads/main", self.base_sha, self.base_sha,
            cwd=self.worktree, pwd=self.worktree, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertMainUnmoved()

    def test_an_unconfigured_token_freezes_main_rather_than_opening_it(self) -> None:
        self.git("config", "--unset", "qcore.mainguard.token", cwd=self.repo)
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, token=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("install_main_guard.py", result.stderr)
        self.assertMainUnmoved()


class SignalArms(GuardFixture):
    """Each provenance arm, driven on its own through the hook directly.

    Going through git exercises the arms git's own environment can reach; these
    cases pin the arms that answer for a hook invoked by anything else, which is
    also how install_main_guard.py self-tests at activation.
    """

    def line(self) -> str:
        return f"{ZERO} {self.card_sha} refs/heads/main\n"

    def test_the_cwd_arm_refuses_even_carrying_the_token(self) -> None:
        result = self.run_hook_directly(
            "reference-transaction", "prepared", self.line(),
            cwd=self.worktree, token=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cwd-in-card-worktree", result.stderr)

    def test_the_git_dir_arm_refuses(self) -> None:
        result = self.run_hook_directly(
            "reference-transaction", "prepared", self.line(),
            token=True,
            extra={"GIT_DIR": str(self.repo / ".git" / "worktrees" / "agent-x")},
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("git-dir-is-card-worktree", result.stderr)

    def test_an_inherited_pwd_is_not_a_signal(self) -> None:
        """Measured, and pinned so no one re-adds a vacuous arm.

        An exported PWD naming a card worktree decides nothing, because a POSIX
        shell re-derives PWD from getcwd() when it starts. A hook arm reading it
        could never fire. This test asserts the measurement, not a wish.
        """
        result = self.run_hook_directly(
            "reference-transaction", "prepared", self.line(),
            cwd=self.repo, token=True,
            extra={"PWD": str(self.worktree)},
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_named_worktree_is_not_vetoed_by_provenance(self) -> None:
        """MAIN-GUARD-NAMED-WORKTREE, pinned as a boundary rather than implied.

        A worktree under .claude/worktrees whose name does not begin `agent-` is
        where an integrator session may itself be working, so it is gated by the
        token alone. Only the Agent tool's `agent-<hex>` worktrees are vetoed.
        """
        named = self.repo / ".claude" / "worktrees" / "some-named-branch"
        named.mkdir(parents=True)
        allowed = self.run_hook_directly(
            "reference-transaction", "prepared", self.line(),
            cwd=named, token=True,
        )
        self.assertEqual(allowed.returncode, 0, allowed.stderr)
        refused = self.run_hook_directly(
            "reference-transaction", "prepared", self.line(), cwd=named,
        )
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn("no valid QCORE_ALLOW_MAIN_UPDATE", refused.stderr)

    def test_a_transaction_that_does_not_touch_main_is_ignored(self) -> None:
        result = self.run_hook_directly(
            "reference-transaction", "prepared",
            f"{ZERO} {self.card_sha} refs/heads/worktree-agent-x\n",
            cwd=self.worktree,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_only_the_prepared_state_can_refuse(self) -> None:
        """git ignores the exit status elsewhere; the hook must not pretend."""
        for state in ("committed", "aborted"):
            with self.subTest(state=state):
                result = self.run_hook_directly(
                    "reference-transaction", state, self.line(), cwd=self.worktree,
                )
                self.assertEqual(result.returncode, 0, result.stderr)


class DelegatedSessionCalibration(GuardFixture):
    def test_the_session_marker_refuses_when_the_signal_is_enabled(self) -> None:
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, token=True,
            extra={"CLAUDE_CODE_CHILD_SESSION": "1"}, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("delegated-session-marker", result.stderr)
        self.assertMainUnmoved()

    def test_calibration_can_disable_the_session_marker(self) -> None:
        """If the integrator's own environment sets it, the signal is recorded
        off rather than left to refuse the integrator. Measured at activation."""
        self.git("config", "qcore.mainguard.vetochildsession", "false", cwd=self.repo)
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, token=True,
            extra={"CLAUDE_CODE_CHILD_SESSION": "1"}, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(self.rev_parse("refs/heads/main"), self.base_sha)

    def test_the_explicit_card_marker_is_refused(self) -> None:
        result = self.git(
            "merge", "--ff-only", "worktree-agent-x",
            cwd=self.repo, token=True,
            extra={"QCORE_CARD_WORKTREE": "1"}, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("qcore-card-worktree-marker", result.stderr)
        self.assertMainUnmoved()


class PrePushGuard(GuardFixture):
    def setUp(self) -> None:
        super().setUp()
        self.remote = self.root / "remote.git"
        self.git("init", "--bare", "-b", "main", str(self.remote), cwd=self.root)
        self.git("remote", "add", "origin", str(self.remote), cwd=self.repo)

    def test_a_push_of_main_from_an_agent_worktree_is_refused(self) -> None:
        result = self.git(
            "push", "origin", f"{self.card_sha}:refs/heads/main",
            cwd=self.worktree, pwd=self.worktree, token=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("REFUSED a push to refs/heads/main", result.stderr)
        self.assertIn("card-branch:worktree-agent-x", result.stderr)

    def test_an_unauthorised_push_of_main_from_the_main_checkout_is_refused(self) -> None:
        result = self.git(
            "push", "origin", "main", cwd=self.repo, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("REFUSED a push to refs/heads/main", result.stderr)

    def test_an_authorised_push_of_main_is_allowed(self) -> None:
        result = self.git(
            "push", "origin", "main", cwd=self.repo, token=True, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_card_may_push_its_own_branch(self) -> None:
        result = self.git(
            "push", "origin", "worktree-agent-x",
            cwd=self.worktree, pwd=self.worktree, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


class InstallerActivation(unittest.TestCase):
    """The one command the integrator runs, exercised rather than described.

    A delegated card cannot activate the guard in the real checkout, so the
    activation path is proved here, in a throwaway repository built out of this
    repository's own `.githooks` and `scripts/install_main_guard.py`.
    """

    def setUp(self) -> None:
        root = Path(tempfile.mkdtemp(prefix="qcore-main-guard-install-"))
        self.addCleanup(GuardFixture._remove, root)
        self.assertNotIn("/.claude/worktrees/", str(root))
        self.repo = root / "repo"
        (self.repo / "scripts").mkdir(parents=True)
        subprocess.run(
            ["cp", "-r", str(HOOKS_DIR), str(self.repo / ".githooks")], check=True
        )
        subprocess.run(
            ["cp", str(REPO_ROOT / "scripts" / "install_main_guard.py"),
             str(self.repo / "scripts" / "install_main_guard.py")],
            check=True,
        )
        self.env = {k: v for k, v in os.environ.items() if k not in SCRUBBED}
        self.env["PWD"] = str(self.repo)
        for args in (
            ["init", "-b", "main", "."],
            ["config", "user.email", "integrator@example.invalid"],
            ["config", "user.name", "Integrator"],
            ["add", "-A"],
            ["commit", "-m", "base"],
        ):
            got = subprocess.run(
                ["git", *args], cwd=str(self.repo), env=self.env,
                capture_output=True, text=True,
            )
            self.assertEqual(got.returncode, 0, got.stderr)

    def install(self, *extra: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["python3", "scripts/install_main_guard.py", *extra],
            cwd=str(self.repo), env=self.env, capture_output=True, text=True,
        )

    def test_activation_configures_the_guard_and_proves_it(self) -> None:
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("The guard is active", result.stdout)
        self.assertNotIn("[FAIL]", result.stdout)
        hooks_path = subprocess.run(
            ["git", "config", "--get", "core.hooksPath"], cwd=str(self.repo),
            env=self.env, capture_output=True, text=True,
        ).stdout.strip()
        self.assertEqual(Path(hooks_path), self.repo / ".githooks")
        token = subprocess.run(
            ["git", "config", "--get", "qcore.mainguard.token"], cwd=str(self.repo),
            env=self.env, capture_output=True, text=True,
        ).stdout.strip()
        self.assertTrue(token)

        # And the guard is really on afterwards: a commit onto main is refused.
        (self.repo / "next.txt").write_text("next\n")
        subprocess.run(["git", "add", "next.txt"], cwd=str(self.repo), env=self.env,
                       capture_output=True, text=True)
        commit = subprocess.run(
            ["git", "commit", "-m", "unauthorised"], cwd=str(self.repo), env=self.env,
            capture_output=True, text=True,
        )
        self.assertNotEqual(commit.returncode, 0)
        self.assertIn("main-ref guard: REFUSED", commit.stderr)

    def test_verify_reports_inactive_before_activation(self) -> None:
        before = self.install("--verify")
        self.assertNotEqual(before.returncode, 0)
        self.assertIn("guard active          : NO", before.stdout)
        self.install()
        after = self.install("--verify")
        self.assertEqual(after.returncode, 0, after.stdout)
        self.assertIn("guard active          : yes", after.stdout)

    def test_activation_is_refused_in_a_linked_worktree(self) -> None:
        """A card must not be able to activate, rotate or calibrate the guard."""
        wt = self.repo / ".claude" / "worktrees" / "agent-y"
        subprocess.run(
            ["git", "worktree", "add", "-b", "worktree-agent-y", str(wt), "main"],
            cwd=str(self.repo), env=self.env, capture_output=True, text=True,
        )
        env = dict(self.env)
        env["PWD"] = str(wt)
        result = subprocess.run(
            ["python3", "scripts/install_main_guard.py"],
            cwd=str(wt), env=env, capture_output=True, text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("linked worktree", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
