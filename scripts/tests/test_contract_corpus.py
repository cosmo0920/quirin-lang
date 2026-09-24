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
        # 108 -> 109: W43-DESCRIPTOR-NATIVE-CHECK's shared observation policy
        # library/descriptor_native_observation.qcore in quirin-provider-shared.
        self.assertEqual(len(modules), 109)
        self.assertEqual(sum(any(part.startswith("qcore-provider-") for part in path.parts) for path in modules.values()), 72)
        self.assertEqual(sum(any(part.startswith("quirin-provider-") for part in path.parts) for path in modules.values()), 37)
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

    def test_libc_contracts_are_taken_by_reference_outside_the_libc_crate(self):
        # Card CONTRACT-INHERIT-ADOPTION: a Fluent Bit wrapper takes its libc
        # contract by `#include` of a published libc module plus
        # `contract inherit libc::NAME`; the libc text stays in the libc crate.
        root = Path(__file__).resolve().parents[2]
        references, violations = contract_corpus.libc_inheritance(root)
        self.assertEqual(violations, [])
        memory = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qcore"
        string = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_string.qcore"
        # Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the Monkey and
        # librdkafka allocator wrappers, owned by the executable crates.
        monkey = "providers/qcore-provider-libevent/contracts/monkey/memory.qcore"
        rdkafka = "providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qcore"
        self.assertEqual(references, [
            (monkey, "mk_mem_alloc", "libc::malloc"),
            (monkey, "mk_mem_alloc_z", "libc::malloc"),
            (monkey, "mk_mem_free", "libc::free"),
            (rdkafka, "rd_malloc", "libc::malloc"),
            (rdkafka, "rd_calloc", "libc::calloc"),
            (rdkafka, "rd_strdup", "libc::strdup"),
            (rdkafka, "rd_strndup", "libc::strndup"),
            (rdkafka, "rd_free", "libc::free"),
            (memory, "flb_malloc", "libc::malloc"),
            (memory, "flb_calloc", "libc::calloc"),
            (memory, "flb_free", "libc::free"),
            (string, "flb_strdup", "libc::strdup"),
            (string, "flb_strndup", "libc::strndup"),
        ])

    def test_libc_inheritance_refuses_unpublished_and_undeclared_parents(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "contract").mkdir()
            libc = root / contract_corpus.LIBC_CRATE / "contracts/libc"
            libc.mkdir(parents=True)
            (libc / "heap.qcore").write_text("module contract;\nextern fn free(p: ptr<mut, u8>) -> unit\n")
            (libc / "other.qcore").write_text("module contract;\nextern fn strlen(s: ptr<const, u8>) -> usize\n")
            other = root / "providers/quirin-provider-fluent-bit/contracts/fluent_bit"
            other.mkdir(parents=True)
            (root / "providers/contract-load-path.json").write_text(json.dumps({
                "schema_version": 1,
                "crates": [{"name": "quirin-provider-libc", "published": {"libc/heap.qcore": "0" * 64}}],
            }))
            (root / "contract/corpus.json").write_text(json.dumps({
                "schema_version": 2,
                "load_path": "../providers/contract-load-path.json",
                "roots": ["../" + contract_corpus.LIBC_CRATE + "/contracts",
                          "../providers/quirin-provider-fluent-bit/contracts"],
            }))
            (other / "good.qcore").write_text(
                'module contract;\n#include "quirin-provider-libc:libc/heap.qcore";\n'
                "extern fn flb_free(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n")
            self.assertEqual(contract_corpus.libc_inheritance(root), ([
                ("providers/quirin-provider-fluent-bit/contracts/fluent_bit/good.qcore",
                 "flb_free", "libc::free")], []))
            (other / "bad.qcore").write_text(
                'module contract;\n#include "quirin-provider-libc:libc/other.qcore";\n'
                "extern fn flb_len(s: ptr<const, u8>) -> usize contract inherit libc::strlen;\n"
                "extern fn flb_nothing(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n")
            _, violations = contract_corpus.libc_inheritance(root)
            bad = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/bad.qcore"
            self.assertEqual(violations, [
                (bad, "includes unpublished libc module libc/other.qcore"),
                (bad, "flb_len inherits libc::strlen, which no included libc module declares"),
                (bad, "flb_nothing inherits libc::free, which no included libc module declares"),
            ])

    def test_schema_two_index_names_a_load_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "contract"
            (directory / "one").mkdir(parents=True)
            (directory / "one/api.qcore").write_text("module contract;")
            index = directory / "corpus.json"
            index.write_text(json.dumps({"schema_version": 2, "roots": ["one"]}))
            with self.assertRaisesRegex(ValueError, "unsupported corpus index"):
                contract_corpus.sources(root)
            index.write_text(json.dumps({"schema_version": 2, "roots": ["one"], "load_path": "absent.json"}))
            with self.assertRaisesRegex(ValueError, "invalid corpus load path"):
                contract_corpus.sources(root)
            (directory / "load-path.json").write_text("{}")
            index.write_text(json.dumps({"schema_version": 2, "roots": ["one"], "load_path": "load-path.json"}))
            self.assertEqual(list(contract_corpus.sources(root)), ["api.qcore"])
            index.write_text(json.dumps({"schema_version": 1, "roots": ["one"], "load_path": "load-path.json"}))
            with self.assertRaisesRegex(ValueError, "unsupported corpus index"):
                contract_corpus.sources(root)

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
            try:
                (directory / "two/api.qcore").symlink_to(directory / "one/api.qcore")
            except OSError as error:
                self.assertEqual((sys.platform, getattr(error, "winerror", None)), ("win32", 1314))
                return
            with self.assertRaisesRegex(ValueError, "symbolic link"):
                contract_corpus.sources(root)
