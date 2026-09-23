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
        self.assertEqual(len(modules), 107)
        self.assertEqual(sum(any(part.startswith("qcore-provider-") for part in path.parts) for path in modules.values()), 72)
        self.assertEqual(sum(any(part.startswith("quirin-provider-") for part in path.parts) for path in modules.values()), 35)
        libc = [path for name, path in modules.items() if name.startswith("libc/")]
        self.assertTrue(libc and all("quirin-provider-libc" in path.parts for path in libc))

    def test_libc_functions_are_declared_only_in_the_libc_crate(self):
        # Card W43-LIBC-CONTRACTS, user-directed placement rule: a libc/POSIX
        # function is declared only in providers/quirin-provider-libc.
        root = Path(__file__).resolve().parents[2]
        self.assertEqual(contract_corpus.libc_placement_violations(root), [])
        names = contract_corpus.libc_names(root)
        self.assertGreater(len(names), 1000)
        owned = set()
        for path in (root / contract_corpus.LIBC_CRATE / "contracts").rglob("*.qcore"):
            owned |= set(contract_corpus.declared_functions(path))
        # Every name the libc crate declares is a listed libc/POSIX name.
        self.assertTrue(owned and owned <= names, sorted(owned - names))

    def test_libc_placement_refuses_a_misplaced_or_unlisted_declaration(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "contract").mkdir()
            libc = root / contract_corpus.LIBC_CRATE
            (libc / "contracts/libc").mkdir(parents=True)
            other = root / "providers/quirin-provider-fluent-bit/contracts/fluent_bit"
            other.mkdir(parents=True)
            (root / "contract/corpus.json").write_text(json.dumps({
                "schema_version": 1,
                "roots": ["../" + contract_corpus.LIBC_CRATE + "/contracts",
                          "../providers/quirin-provider-fluent-bit/contracts"],
            }))
            (libc / "contracts/libc/own.qcore").write_text(
                "module contract;\nextern fn only_in_libc_crate() -> i32\ncontract { };\n")
            (other / "ok.qcore").write_text("module contract;\nextern fn flb_ok() -> i32\n")
            with self.assertRaises(OSError):
                contract_corpus.libc_placement_violations(root)
            (libc / "libc-names.txt").write_text("# header only\n")
            with self.assertRaisesRegex(ValueError, "empty libc name list"):
                contract_corpus.libc_placement_violations(root)
            (libc / "libc-names.txt").write_text("# list\nmemcpy\n")
            self.assertEqual(contract_corpus.libc_placement_violations(root), [])
            # A listed name outside the crate, in the corpus or loose under providers/.
            (other / "bad.qcore").write_text("module contract;\n  extern fn memcpy(d: ptr<mut, u8>) -> ptr<mut, u8>\n")
            loose = root / "providers/qcore-provider-x/fixtures"
            loose.mkdir(parents=True)
            (loose / "loose.qcore").write_text("module contract;\nunsafe extern fn memcpy() -> unit\n// extern fn strlen() -> usize\n")
            # A name the libc crate declares but the list does not carry.
            (other / "copy.qcore").write_text("module contract;\nextern fn only_in_libc_crate() -> i32\n")
            self.assertEqual(contract_corpus.libc_placement_violations(root), [
                ("providers/qcore-provider-x/fixtures/loose.qcore", "memcpy"),
                ("providers/quirin-provider-fluent-bit/contracts/fluent_bit/bad.qcore", "memcpy"),
                ("providers/quirin-provider-fluent-bit/contracts/fluent_bit/copy.qcore", "only_in_libc_crate"),
            ])

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
