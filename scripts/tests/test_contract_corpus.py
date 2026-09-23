"""Canonical discovery must not hide missing roots or duplicate logical modules."""
import json
from pathlib import Path
import tempfile
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import contract_corpus


class CanonicalCorpus(unittest.TestCase):
    def test_repository_has_one_canonical_copy_per_pinned_module(self):
        root = Path(__file__).resolve().parents[2]
        self.assertFalse((root / "contract/qcore").exists())
        modules = contract_corpus.sources(root)
        self.assertEqual(len(modules), 96)
        self.assertEqual(sum(any(part.startswith("qcore-provider-") for part in path.parts) for path in modules.values()), 72)
        self.assertEqual(sum(any(part.startswith("quirin-provider-") for part in path.parts) for path in modules.values()), 24)
        libc = [path for name, path in modules.items() if name.startswith("libc/")]
        self.assertTrue(libc and all("quirin-provider-libc" in path.parts for path in libc))

    def test_missing_root_duplicate_and_symlink_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "contract"
            directory.mkdir()
            index = directory / "corpus.json"
            index.write_text(json.dumps({"schema_version": 1, "roots": ["missing"]}))
            with self.assertRaisesRegex(ValueError, "invalid corpus root"):
                contract_corpus.sources(root)
            for name in ["one", "two"]:
                (directory / name).mkdir()
                (directory / name / "api.qcore").write_text("module contract;")
            index.write_text(json.dumps({"schema_version": 1, "roots": ["one", "two"]}))
            with self.assertRaisesRegex(ValueError, "duplicate logical contract"):
                contract_corpus.sources(root)
            (directory / "two/api.qcore").unlink()
            (directory / "two/api.qcore").symlink_to(directory / "one/api.qcore")
            with self.assertRaisesRegex(ValueError, "symbolic link"):
                contract_corpus.sources(root)
