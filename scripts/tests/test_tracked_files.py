"""Card W45-TEST-HYGIENE: the tracked-file helper the tree-walking Python tests
use lists exactly what git tracks, including a file only added to the index,
and fails closed outside a git checkout instead of walking the filesystem."""
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tracked_files import TrackedFilesUnavailable, tracked_relative  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]

TEST_FILE = re.compile(
    r"^(compiler/tests/|crates/[^/]+/tests/|providers/[^/]+/tests/|providers/tests/"
    r"|tests/support/|scripts/tests/).*\.(rs|py)$")
HELPERS = {"tests/support/tracked_files.rs", "scripts/tests/tracked_files.py",
           "scripts/tests/test_tracked_files.py"}
WALK = re.compile(
    r"read_dir\(|WalkDir|walkdir::|os\.walk\(|\.rglob\(|\.glob\(|glob\.glob\(|os\.listdir\("
    r"|os\.scandir\(|\.iterdir\(|Command::new\(\"find\"\)")
# Every walk primitive left in a test file, by file, with its reviewed reason
# (docs/reports/w45-test-hygiene/README.md, census rows U1-U14).
REVIEWED_WALKS = {
    "compiler/tests/composed_five_providers.rs": 1,  # mirrors production profile discovery
    "compiler/tests/ct_corpus.rs": 2,  # one CT case's fixture directory (two sites)
    "compiler/tests/std_event.rs": 1,  # the system temporary directory (Windows)
    "compiler/tests/support/ct_atomic_publication.rs": 1,  # CT41's fixture directory
    "compiler/tests/unit/pinned_lifecycle.rs": 1,  # the vendored fixture tree, copied
    "crates/qcore-audit/tests/configured_cli.rs": 6,  # .qc artifacts in its own temporary fixture
    "crates/qcore-c-gateway/tests/unit/audit_ir_fragments.rs": 1,  # its own temporary object store
    "crates/qcore-audit/tests/checkpoint_cli.rs": 1,  # checkpoint files in its own temporary fixture
    "crates/qcore-audit/tests/user_contract_plugins.rs": 2,  # fixture copy; its scratch copy
    "crates/qcore-contract/tests/contract_inherit.rs": 1,  # its fixture crates
    "crates/qcore-core/tests/unit/capacity_boundary.rs": 1,  # a scratch directory
    "providers/quirin-provider-plugin/tests/sdk.rs": 1,  # the example crate fixture, copied
    "scripts/tests/test_build_matrix_sections.py": 4,  # mirrors build_matrix.py's own reads
    "scripts/tests/test_check_lean_audits.py": 1,  # a scratch proof directory
    "scripts/tests/test_land.py": 1,  # a scratch measure directory (a TemporaryDirectory)
    "scripts/tests/test_package_reproducible.py": 2,  # a scratch stage and CARGO_HOME (TemporaryDirectory)
    "scripts/tests/test_run_test_binaries.py": 1,  # a scratch capture directory
    "scripts/tests/test_w43_chunk_source_profile.py": 1,  # the vendored fixture tree
}


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

    def test_every_remaining_tree_walk_in_the_tests_is_a_reviewed_one(self):
        # Rule 29's tripwire. The W45-TEST-HYGIENE census converted every
        # repository walk in the test directories; what remains is a walk of
        # the test's own fixture directory, of a temporary directory it made,
        # or the expected side of a production walk, each reviewed in
        # docs/reports/w45-test-hygiene/README.md. A new walk primitive in a
        # test file fails here: list inputs with tracked_files instead, or
        # review the walk and add it with its reason.
        found = {}
        for relative in tracked_relative(ROOT):
            if not TEST_FILE.match(relative) or relative in HELPERS:
                continue
            count = len(WALK.findall((ROOT / relative).read_text(encoding="utf-8")))
            if count:
                found[relative] = count
        self.assertEqual(found, REVIEWED_WALKS)

    def test_the_repository_listing_holds_this_file_and_no_ignored_tree(self):
        files = tracked_relative(ROOT)
        self.assertIn("scripts/tests/test_tracked_files.py", files)
        for ignored in ("target/", "private/", "measure/", "build/cycle19/", ".claude/"):
            self.assertFalse(any(path.startswith(ignored) for path in files), ignored)


if __name__ == "__main__":
    unittest.main()
