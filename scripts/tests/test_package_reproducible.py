"""Card PACKAGE-BYTES-DEPEND-ON-CHECKOUT: the staged provider build.

`providers/package.py build` compiles a staged copy of the provider workspace
whose root holds every path package, so cargo hashes each of them by a
repository-relative path, and remaps every absolute path left to a constant.
These tests hold the pieces that make that so without running cargo; the
two-checkout comparison itself is `package.py reproduce`, which CI runs.
"""
import importlib.util
import os
from pathlib import Path
import tempfile
import tomllib
import unittest

PACKAGE_PATH = Path(__file__).resolve().parents[2] / "providers/package.py"
SPEC = importlib.util.spec_from_file_location("provider_package_reproducible", PACKAGE_PATH)
package = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package)


class StagedManifest(unittest.TestCase):
    def test_real_manifest_rebases_every_member_and_path_dependency(self):
        manifest = tomllib.loads((package.ROOT / "Cargo.toml").read_text())
        staged = package.staged_manifest(manifest, ["crates/qcore-schemas"])
        workspace = staged["workspace"]
        self.assertEqual(workspace["members"],
                         [f"providers/{member}" for member in manifest["workspace"]["members"]])
        self.assertEqual(workspace["dependencies"]["qcore-schemas"]["path"], "crates/qcore-schemas")
        self.assertEqual(workspace["dependencies"]["quirin-provider-shared"]["path"],
                         "providers/quirin-provider-shared")
        self.assertEqual(workspace["exclude"], ["crates/qcore-schemas"])
        self.assertEqual(staged["profile"], manifest["profile"])
        self.assertEqual(tomllib.loads(package.toml_document(staged)), staged)

    def test_unknown_keys_globs_and_escapes_are_refused(self):
        base = {"workspace": {"members": ["a"]}}
        for manifest in (
            {**base, "patch": {"crates-io": {}}},
            {"workspace": {"members": ["a"], "default-members": ["a"]}},
            {"workspace": {"members": ["crates/*"]}},
            {"workspace": {"members": ["a"], "dependencies": {"x": {"path": "../../x"}}}},
            {"workspace": {"members": ["a"], "dependencies": {"x": {"path": "/abs/x"}}}},
        ):
            with self.subTest(manifest=manifest), self.assertRaises(ValueError):
                package.staged_manifest(manifest, [])

    def test_toml_document_round_trips_quoted_keys_and_empty_tables(self):
        data = {"workspace": {"members": ["providers/a b"], "dependencies": {
            "odd.name": {"path": "crates/\"q\"", "features": ["x"], "optional": True}}},
            "profile": {"release": {"opt-level": 3}, "bench": {}}}
        self.assertEqual(tomllib.loads(package.toml_document(data)), data)
        with self.assertRaises(ValueError):
            package.toml_document({"workspace": {"members": [1.5]}})


class StageTree(unittest.TestCase):
    def test_sync_keeps_unchanged_files_and_removes_stale_ones(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "src.rs"
            source.write_text("fn main() {}\n")
            stage = root / "stage"
            package.sync_tree({"a/src.rs": source, "Cargo.toml": b"[workspace]\n"}, stage)
            kept = stage / "a/src.rs"
            os.utime(kept, (1, 1))
            (stage / "stale/old.rs").parent.mkdir(parents=True)
            (stage / "stale/old.rs").write_text("old")
            package.sync_tree({"a/src.rs": source, "Cargo.toml": b"[workspace]\nx = 1\n"}, stage)
            self.assertEqual(kept.stat().st_mtime, 1)
            self.assertEqual((stage / "Cargo.toml").read_bytes(), b"[workspace]\nx = 1\n")
            self.assertFalse((stage / "stale").exists())
            source.write_text("fn main() { }\n")
            package.sync_tree({"a/src.rs": source}, stage)
            self.assertNotEqual(kept.stat().st_mtime, 1)
            self.assertEqual(sorted(p.name for p in stage.rglob("*")), ["a", "src.rs"])

    def test_package_files_skip_target_directories(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            for name in ("crates/x/src/lib.rs", "crates/x/target/junk", "crates/x/Cargo.toml"):
                (repo / name).parent.mkdir(parents=True, exist_ok=True)
                (repo / name).write_text(name)
            self.assertEqual(sorted(package.package_files(repo, ["crates/x"], set())),
                             ["crates/x/Cargo.toml", "crates/x/src/lib.rs"])


class Remapping(unittest.TestCase):
    def test_stage_is_the_last_and_most_specific_remap(self):
        flags = package.remap_flags(Path("/t/stage"), Path("/t"), Path("/h/.cargo"),
                                    Path("/h/.rustup/x"), "a" * 40)
        self.assertEqual(flags, [
            "--remap-path-prefix=/h/.rustup/x/lib/rustlib/src/rust=/rustc/" + "a" * 40,
            "--remap-path-prefix=/h/.cargo=/cargo",
            "--remap-path-prefix=/t=/target",
            "--remap-path-prefix=/t/stage=/qcore",
        ])

    def test_embedded_paths_report_host_paths_and_ignore_the_root(self):
        data = b"\0/qcore/providers/x.rs\0/home/me/.cargo/registry/src/x.rs\0"
        self.assertEqual(package.embedded_paths(data, [Path("/home/me"), Path("/"),
                                                      Path("/srv/repo")]), ["/home/me"])


class SecondCargoHome(unittest.TestCase):
    def test_index_layout(self):
        self.assertEqual([package.index_entry(name) for name in ("a", "ab", "abc", "Serde")],
                         ["1/a", "2/ab", "3/a/abc", "se/rd/serde"])

    def test_copies_only_the_locked_registry_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / "home"
            files = ["registry/cache/idx/serde-1.0.0.crate", "registry/cache/idx/other-2.0.0.crate",
                     "registry/index/idx/config.json", "registry/index/idx/.cache/se/rd/serde",
                     "config.toml"]
            for name in files:
                (home / name).parent.mkdir(parents=True, exist_ok=True)
                (home / name).write_text(name)
            lock = root / "Cargo.lock"
            lock.write_text('version = 4\n[[package]]\nname = "local"\nversion = "0.1.0"\n'
                            f'[[package]]\nname = "serde"\nversion = "1.0.0"\n'
                            f'source = "{package.CRATES_IO}"\n')
            package.populate_cargo_home(home, root / "second", lock)
            copied = sorted(path.relative_to(root / "second").as_posix()
                            for path in (root / "second").rglob("*") if path.is_file())
            self.assertEqual(copied, sorted(set(files) - {"registry/cache/idx/other-2.0.0.crate"}))
            lock.write_text('version = 4\n[[package]]\nname = "g"\nversion = "1.0.0"\n'
                            'source = "git+https://example.invalid/g#abc"\n')
            with self.assertRaises(ValueError):
                package.populate_cargo_home(home, root / "third", lock)


if __name__ == "__main__":
    unittest.main()
