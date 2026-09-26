"""Card W45-TEST-HYGIENE: the tracked-file helper the tree-walking Python tests
use lists exactly what git tracks, including a file only added to the index,
and fails closed outside a git checkout instead of walking the filesystem."""
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tracked_files import TrackedFilesUnavailable, tracked_relative  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]


def git(root, *args):
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE")}
    subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True,
                   env=environment)


def write(root, relative, text):
    path = Path(root) / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


class TrackedFiles(unittest.TestCase):
    def test_only_tracked_files_are_listed(self):
        with tempfile.TemporaryDirectory() as scratch:
            git(scratch, "init", "-q")
            write(scratch, ".gitignore", "/private/\n")
            write(scratch, "examples/a.qcore", "a\n")
            write(scratch, "examples/nested/b.qcore", "b\n")
            write(scratch, "examples/gone.qcore", "gone\n")
            git(scratch, "add", ".")
            # Added to the index after the others, never committed: tracked.
            write(scratch, "examples/staged.qcore", "staged\n")
            git(scratch, "add", "examples/staged.qcore")
            (Path(scratch) / "examples/gone.qcore").unlink()
            write(scratch, "examples/untracked.qcore", "untracked\n")
            write(scratch, "examples/export/copy.qcore", "untracked copy\n")
            write(scratch, "private/export/examples/a.qcore", "ignored copy\n")
            self.assertEqual(tracked_relative(scratch), [
                ".gitignore", "examples/a.qcore", "examples/nested/b.qcore",
                "examples/staged.qcore"])
            self.assertEqual(tracked_relative(Path(scratch) / "examples"),
                             ["a.qcore", "nested/b.qcore", "staged.qcore"])

    def test_an_export_fails_closed(self):
        # Under the system temporary directory, outside any checkout, like an
        # unpacked tarball. Were it inside one, nothing would be tracked here
        # and the helper would fail closed on that instead.
        with tempfile.TemporaryDirectory() as scratch:
            write(scratch, "examples/a.qcore", "a\n")
            with self.assertRaises(TrackedFilesUnavailable) as caught:
                tracked_relative(scratch)
            self.assertIn("W45-TEST-HYGIENE", str(caught.exception))

    def test_a_directory_with_nothing_tracked_fails_closed(self):
        with tempfile.TemporaryDirectory() as scratch:
            git(scratch, "init", "-q")
            write(scratch, ".gitignore", "/private/\n")
            git(scratch, "add", ".gitignore")
            write(scratch, "private/export/a.qcore", "ignored\n")
            with self.assertRaises(TrackedFilesUnavailable) as caught:
                tracked_relative(Path(scratch) / "private")
            self.assertIn("git tracks no file", str(caught.exception))

    def test_the_repository_listing_holds_this_file_and_no_ignored_tree(self):
        files = tracked_relative(ROOT)
        self.assertIn("scripts/tests/test_tracked_files.py", files)
        for ignored in ("target/", "private/", "measure/", "build/cycle19/", ".claude/"):
            self.assertFalse(any(path.startswith(ignored) for path in files), ignored)


if __name__ == "__main__":
    unittest.main()
