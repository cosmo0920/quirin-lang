"""Canonical discovery must not hide missing roots or duplicate logical modules."""
import json
from pathlib import Path
import re
import tempfile
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import contract_corpus
from tracked_files import tracked_relative


def tracked(root):
    """The repository's tracked files (card W45-TEST-HYGIENE): the checks on
    the real tree read these only, so an untracked or ignored `.qcore` under a
    corpus root or `providers/` changes no result."""
    return tracked_relative(root)


class CanonicalCorpus(unittest.TestCase):
    def test_repository_has_one_canonical_copy_per_pinned_module(self):
        root = Path(__file__).resolve().parents[2]
        self.assertFalse((root / "contract/qcore").exists())
        modules = contract_corpus.sources(root, tracked(root))
        # Card CONTRACT-COUNT-PINS-FROM-LOCKS: the module counts are the
        # providers' inventory locks' (contract_corpus.lock_totals, the reader
        # tests/support/contract_locks.rs mirrors); the former literals and
        # their per-card history are in this file at base 25508ccaf.
        totals = contract_corpus.lock_totals(root)
        self.assertEqual(len(modules), totals["corpus_modules"])
        self.assertEqual(sum(any(part.startswith("qcore-provider-") for part in path.parts) for path in modules.values()),
                         totals["qcore_provider_modules"])
        self.assertEqual(sum(any(part.startswith("quirin-provider-") for part in path.parts) for path in modules.values()),
                         totals["quirin_provider_modules"])
        libc = [path for name, path in modules.items() if name.startswith("libc/")]
        self.assertTrue(libc and all("quirin-provider-libc" in path.parts for path in libc))

    def test_lock_totals_agree_with_the_shared_cases(self):
        # Card CONTRACT-COUNT-PINS-FROM-LOCKS: tests/support/contract_locks_cases.json
        # holds the cases both aggregate readers must agree on (the Rust reader
        # runs the same file in compiler/tests/contract_loader.rs): the totals
        # are sums of lock fields, and a lock inconsistent with the load path
        # or with itself is refused, never summed.
        repository = Path(__file__).resolve().parents[2]
        cases = json.loads((repository / "tests/support/contract_locks_cases.json").read_text())
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def write(locks):
                (root / "providers").mkdir(exist_ok=True)
                (root / "providers/contract-load-path.json").write_text(
                    json.dumps(cases["load_path"]))
                for name, lock in locks.items():
                    (root / "providers" / name).mkdir(exist_ok=True)
                    (root / "providers" / name / "contract-inventory.lock").write_text(
                        json.dumps(lock))

            write(cases["locks"])
            self.assertEqual(contract_corpus.lock_totals(root), cases["totals"])
            self.assertEqual(list(cases["totals"]), list(contract_corpus.LOCK_TOTAL_FIELDS))
            for refusal in cases["refusals"]:
                with self.subTest(why=refusal["why"]):
                    locks = dict(cases["locks"])
                    locks[refusal["crate"]] = {**locks[refusal["crate"]], **refusal["replace"]}
                    write(locks)
                    with self.assertRaisesRegex(ValueError, re.escape(refusal["contains"])):
                        contract_corpus.lock_totals(root)

    def test_libc_functions_are_declared_only_in_the_libc_crate(self):
        # Card W43-LIBC-CONTRACTS, user-directed placement rule: a libc/POSIX
        # function is declared only in providers/quirin-provider-libc.
        root = Path(__file__).resolve().parents[2]
        files = tracked(root)
        self.assertEqual(contract_corpus.libc_placement_violations(root, files), [])
        names = contract_corpus.libc_names(root)
        self.assertGreater(len(names), 1000)
        owned = set()
        prefix = contract_corpus.LIBC_CRATE + "/contracts/"
        for relative in files:
            if relative.startswith(prefix) and relative.endswith((".qcore", ".qv")):
                owned |= set(contract_corpus.declared_functions(root / relative))
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
        references, violations = contract_corpus.libc_inheritance(root, tracked(root))
        self.assertEqual(violations, [])
        memory = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv"
        string = "providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_string.qv"
        # Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: the Monkey and
        # librdkafka allocator wrappers, owned by the executable crates.
        monkey = "providers/qcore-provider-libevent/contracts/monkey/memory.qv"
        rdkafka = "providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv"
        # Rule37: rd_calloc/rd_strdup are checked specifications, not inheritance.
        self.assertEqual(references, [
            (monkey, "mk_mem_alloc", "libc::malloc"),
            (monkey, "mk_mem_alloc_z", "libc::malloc"),
            (monkey, "mk_mem_free", "libc::free"),
            (rdkafka, "rd_malloc", "libc::malloc"),
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

    def test_a_tracked_listing_replaces_the_walk(self):
        # Card W45-TEST-HYGIENE: given the tracked list, discovery reads only
        # it; an untracked module under a root is not part of the corpus.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            directory = root / "contract"
            (directory / "one/export").mkdir(parents=True)
            (directory / "one/api.qcore").write_text("module contract;")
            (directory / "one/export/stale.qcore").write_text("module contract;")
            (directory / "corpus.json").write_text(json.dumps({"schema_version": 1, "roots": ["one"]}))
            self.assertEqual(list(contract_corpus.sources(root)), ["api.qcore", "export/stale.qcore"])
            tracked_list = ["contract/corpus.json", "contract/one/api.qcore"]
            self.assertEqual(list(contract_corpus.sources(root, tracked_list)), ["api.qcore"])
            with self.assertRaisesRegex(ValueError, "empty corpus"):
                contract_corpus.sources(root, ["contract/corpus.json"])

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
