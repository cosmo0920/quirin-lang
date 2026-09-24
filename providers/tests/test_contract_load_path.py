"""Card CONTRACT-INHERIT-ADOPTION: the load-path manifest and include pins.

`providers/contracts.py` generates `providers/contract-load-path.json` from
Cargo metadata and each `quirin-provider-*` crate's `NAMESPACE`/`PUBLISHED`
constants, and computes a provider inventory's `includes` pins from it;
`providers/package.py build` installs the pinned texts, which `verify`
accepts. Synthetic trees exercise every refusal; one test reads the real
workspace.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


contracts = load("provider_contracts_load_path", HERE / "contracts.py")
package = load("provider_package_load_path", HERE / "package.py")

PARENT = ("module contract;\nextern fn free(ptr: ptr<mut, u8>) -> unit\ncontract {\n"
          "    requires resource_contract_v1;\n    ensures normal resource_released(ptr, nullable);\n"
          "    ensures normal resource_return_none();\n};\n")
CHILD = ('module contract;\n#include "quirin-provider-libc:libc/memory.qcore";\n'
         "extern fn wrap_free(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n")


def lib_rs(namespace, published):
    items = ", ".join(f'"{m}"' for m in published)
    return (f'pub const NAMESPACE: &str = "{namespace}";\n'
            f"pub const PUBLISHED: &[&str] = &[{items}];\n")


class Tree:
    def __init__(self, root):
        self.root = Path(root) / "providers"
        self.crate("quirin-provider-libc", "libc", ["libc/memory.qcore"], {"libc/memory.qcore": PARENT})
        self.crate("quirin-provider-app", "app", [], {"app/wrap.qcore": CHILD})

    def crate(self, name, namespace, published, modules):
        directory = self.root / name
        (directory / "src").mkdir(parents=True, exist_ok=True)
        (directory / "src/lib.rs").write_text(lib_rs(namespace, published))
        for module, text in modules.items():
            path = directory / "contracts" / module
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)

    def metadata(self, app_dependencies=({"name": "quirin-provider-libc", "kind": None},)):
        def package(name, dependencies):
            return {"name": name, "version": "0.1.0",
                    "manifest_path": str(self.root / name / "Cargo.toml"),
                    "dependencies": list(dependencies)}
        return {"packages": [
            package("quirin-provider-libc", []),
            package("quirin-provider-app", app_dependencies),
            package("qcore-provider-other", [{"name": "quirin-provider-libc", "kind": None}]),
        ]}


class LoadPathManifest(unittest.TestCase):
    def test_generated_from_metadata_and_constants(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            manifest = contracts.load_path_manifest(tree.root, tree.metadata())
            self.assertEqual([c["name"] for c in manifest["crates"]],
                             ["quirin-provider-app", "quirin-provider-libc"])
            app, libc = manifest["crates"]
            self.assertEqual(app["dependencies"], ["quirin-provider-libc"])
            self.assertEqual(app["namespace"], "app")
            self.assertEqual(libc["published"],
                             {"libc/memory.qcore": hashlib.sha256(PARENT.encode()).hexdigest()})
            self.assertEqual(libc["contracts"], "quirin-provider-libc/contracts")

    def test_dev_and_build_dependencies_are_not_contract_edges(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            metadata = tree.metadata(({"name": "quirin-provider-libc", "kind": "dev"},))
            app = contracts.load_path_manifest(tree.root, metadata)["crates"][0]
            self.assertEqual(app["dependencies"], [])

    def test_recorded_constants_are_checked(self):
        cases = [
            ("other", ["libc/memory.qcore"], "not the derived"),
            ("libc", ["libc/absent.qcore"], "does not own"),
            ("libc", ["libc/memory.qcore", "libc/memory.qcore"], "sorted and unique"),
        ]
        for namespace, published, message in cases:
            with self.subTest(message=message), tempfile.TemporaryDirectory() as temporary:
                tree = Tree(temporary)
                (tree.root / "quirin-provider-libc/src/lib.rs").write_text(lib_rs(namespace, published))
                with self.assertRaisesRegex(ValueError, message):
                    contracts.load_path_manifest(tree.root, tree.metadata())
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            (tree.root / "quirin-provider-libc/src/lib.rs").write_text("// nothing recorded\n")
            with self.assertRaisesRegex(ValueError, "one NAMESPACE"):
                contracts.load_path_manifest(tree.root, tree.metadata())

    def test_stale_manifest_is_refused_and_sync_rewrites_it(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            metadata = tree.metadata()
            contracts.maintain_load_path(tree.root, sync=True, metadata=metadata)
            contracts.maintain_load_path(tree.root, metadata=metadata)
            (tree.root / "quirin-provider-libc/contracts/libc/memory.qcore").write_text(
                PARENT.replace("nullable", "nonnull"))
            with self.assertRaisesRegex(ValueError, "stale"):
                contracts.maintain_load_path(tree.root, metadata=metadata)

    def test_include_pins_install_and_verify(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            load_path = contracts.maintain_load_path(tree.root, sync=True, metadata=tree.metadata())
            provider = {"app/wrap.qcore": tree.root / "quirin-provider-app/contracts/app/wrap.qcore"}
            pins = contracts.include_pins(provider, load_path, tree.root)
            key = "quirin-provider-libc:libc/memory.qcore"
            self.assertEqual(list(pins), [key])
            self.assertEqual(pins[key]["sha256"], hashlib.sha256(PARENT.encode()).hexdigest())
            built = Path(temporary) / "qcore.example"
            (built / "contracts/app").mkdir(parents=True)
            (built / "contracts/app/wrap.qcore").write_text(CHILD)
            inventory = {"schema_version": 1, "plugin_id": "qcore.example",
                         "contracts": {"contracts/app/wrap.qcore": hashlib.sha256(CHILD.encode()).hexdigest()},
                         "includes": pins}
            self.assertEqual(package.install_includes(built, inventory, tree.root), 1)
            found = package.installed_contract_files(built)
            self.assertEqual(package.verify_contract_includes(built, inventory, found), 1)
            # A parent that changed after pinning is refused at build time.
            (tree.root / "quirin-provider-libc/contracts/libc/memory.qcore").write_text("changed")
            with self.assertRaisesRegex(ValueError, "changed since it was pinned"):
                package.install_includes(Path(temporary) / "again", inventory, tree.root)

    def test_an_unpublished_or_unknown_include_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            (tree.root / "quirin-provider-libc/src/lib.rs").write_text(lib_rs("libc", []))
            load_path = contracts.load_path_manifest(tree.root, tree.metadata())
            provider = {"app/wrap.qcore": tree.root / "quirin-provider-app/contracts/app/wrap.qcore"}
            with self.assertRaisesRegex(ValueError, "not a published load-path module"):
                contracts.include_pins(provider, load_path, tree.root)
            with self.assertRaisesRegex(ValueError, "not a published load-path module"):
                contracts.include_pins(provider, {"schema_version": 1, "crates": []}, tree.root)

    def test_the_real_workspace_manifest_is_current(self):
        manifest = contracts.maintain_load_path()
        names = [c["name"] for c in manifest["crates"]]
        self.assertTrue(all(name.startswith("quirin-provider-") for name in names))
        self.assertIn("quirin-provider-fluent-bit", names)
        flb = next(c for c in manifest["crates"] if c["name"] == "quirin-provider-fluent-bit")
        self.assertEqual(flb["dependencies"], ["quirin-provider-libc"])


if __name__ == "__main__":
    unittest.main()
