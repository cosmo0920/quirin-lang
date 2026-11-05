"""Card CONTRACT-INHERIT-ADOPTION: the load-path manifest and include pins.

`providers/contracts.py` generates `providers/contract-load-path.json` from
Cargo metadata and each `quirin-provider-*` and (card
CONTRACT-INHERIT-QCORE-PROVIDER-NAMING) `qcore-provider-*` crate's
`NAMESPACE`/`PUBLISHED` constants, and computes a provider inventory's `includes` pins from it;
`providers/package.py build` installs the pinned texts, which `verify`
accepts. Synthetic trees exercise every refusal; one test reads the real
workspace.

Card W44-PROVIDER-RESOURCE-VIEW: each crate record carries its
`resource_view`, the owned modules the resource-contract loader accepts.
Synthetic trees use a stand-in for the loader (`loader_like`); the real
workspace test runs the loader itself, through the probe example.
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
CHILD = ('module contract;\n#include "quirin-provider-libc:libc/memory.qv";\n'
         "extern fn wrap_free(p: ptr<mut, u8>) -> unit contract inherit libc::free;\n")


# A module written for another checker: the stand-in loader refuses it, as the
# resource loader refuses lifecycle and observation vocabularies.
POLICY = ("module contract;\nextern fn policy_step(p: ptr<mut, u8>) -> unit\ncontract {\n"
          "    requires lifecycle_policy_fact;\n};\n")


def loader_like(root, load_path_file):
    """A stand-in for `contracts.loader_resource_views`: every owned module is
    accepted unless it requires `lifecycle_policy_fact`."""
    load_path_file = Path(load_path_file)
    manifest = json.loads(load_path_file.read_text())
    return {record["name"]: [
        module for module in record["modules"]
        if "lifecycle_policy_fact" not in
        (load_path_file.parent / record["contracts"] / module).read_text()]
        for record in manifest["crates"]}


# Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: a bounds contract v1 module, which
# the resource loader refuses and the bounds loader accepts.
BOUNDS = ("module contract;\nextern fn get_meta_len(map: ptr<mut, u8>) -> u16\ncontract {\n"
          "    requires bounds_contract_v1;\n    quant checks map, Idx #i: 22 <= #i && #i < 24;\n};\n")


def loader_with_bounds(root, load_path_file):
    """A stand-in for both loaders: a module selecting `bounds_contract_v1`
    is in the bounds view only; `loader_like` decides the resource view."""
    load_path_file = Path(load_path_file)
    manifest = json.loads(load_path_file.read_text())
    resource = loader_like(root, load_path_file)
    result = {}
    for record in manifest["crates"]:
        texts = {module: (load_path_file.parent / record["contracts"] / module).read_text()
                 for module in record["modules"]}
        bounds = [m for m in record["modules"] if "bounds_contract_v1" in texts[m]]
        result[record["name"]] = {"resource_view": [m for m in resource[record["name"]]
                                                    if m not in bounds],
                                  "bounds_view": bounds}
    return result


def inventory_like(root, load_path_file):
    """A stand-in for `contracts.probe_inventories` (card CONTRACT-LOCKS):
    one declaration per `extern fn`, its clauses the `requires`/`ensures`
    lines up to the next declaration (an inheriting child gets its parent's
    count, as elaboration would give it), `contract inherit` edges as written,
    no premises and no binding pins."""
    load_path_file = Path(load_path_file)
    manifest = json.loads(load_path_file.read_text())
    parent_clauses = PARENT.count("requires ") + PARENT.count("ensures ")
    result = {}
    for record in manifest["crates"]:
        modules = {}
        for module in record["modules"]:
            text = (load_path_file.parent / record["contracts"] / module).read_text()
            declarations, inherits = [], []
            for chunk in text.split("extern fn ")[1:]:
                name = chunk.split("(", 1)[0]
                if "contract inherit " in chunk:
                    parent = chunk.split("contract inherit ", 1)[1].split(";", 1)[0]
                    inherits.append({"child": name, "parent": parent})
                    clauses = parent_clauses
                else:
                    clauses = chunk.count("requires ") + chunk.count("ensures ")
                declarations.append({"name": name, "clauses": clauses})
            modules[module] = {"declarations": declarations, "inherits": inherits,
                               "premises": {}, "premise_bindings": []}
        result[record["name"]] = modules
    return result


def lib_rs(namespace, published):
    items = ", ".join(f'"{m}"' for m in published)
    return (f'pub const NAMESPACE: &str = "{namespace}";\n'
            f"pub const PUBLISHED: &[&str] = &[{items}];\n")


class Tree:
    """Two data crates and (card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING) one
    executable crate, `qcore-provider-exe`, whose constants live in
    `src/main.rs` and which inherits from libc like the real librdkafka and
    libevent crates; plus a package named `other-provider-x`, whose prefix is
    not admitted, and the conformance harness, which owns no contracts."""

    def __init__(self, root):
        self.root = Path(root) / "providers"
        self.crate("quirin-provider-libc", "libc", ["libc/memory.qv"], {"libc/memory.qv": PARENT})
        self.crate("quirin-provider-app", "app", [], {"app/wrap.qcore": CHILD})
        self.crate("qcore-provider-exe", "exe", [], {"exe/wrap.qcore": CHILD}, binary=True)
        (self.root / "conformance/src").mkdir(parents=True)
        (self.root / "quirin-provider-plugin/src").mkdir(parents=True)

    def crate(self, name, namespace, published, modules, binary=False):
        directory = self.root / name
        (directory / "src").mkdir(parents=True, exist_ok=True)
        (directory / ("src/main.rs" if binary else "src/lib.rs")).write_text(lib_rs(namespace, published))
        for module, text in modules.items():
            path = directory / "contracts" / module
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)

    def metadata(self, app_dependencies=({"name": "quirin-provider-libc", "kind": None},)):
        def package(name, dependencies, binary=False, directory=None):
            directory = self.root / (directory or name)
            source = directory / ("src/main.rs" if binary else "src/lib.rs")
            return {"name": name, "version": "0.1.0",
                    "manifest_path": str(directory / "Cargo.toml"),
                    "targets": [{"kind": ["bin" if binary else "lib"], "src_path": str(source)}],
                    "dependencies": list(dependencies)}
        libc = [{"name": "quirin-provider-libc", "kind": None}]
        return {"packages": [
            package("quirin-provider-libc", []),
            package("quirin-provider-app", app_dependencies),
            package("qcore-provider-exe", libc, binary=True),
            package("other-provider-x", libc),
            package("qcore-provider-conformance", libc, directory="conformance"),
            # Card W45-USER-CONTRACT-PLUGINS: the SDK owns no contracts.
            package("quirin-provider-plugin", [], directory="quirin-provider-plugin"),
        ]}


class LoadPathManifest(unittest.TestCase):
    def test_exact_owner_selector_admits_new_spelling_without_broadening_owner(self):
        # A legacy `.qcore` selector admits its own `.qv` spelling, and no
        # other stem (the pre-f4018a92 intent; the migration collapsed both
        # assertions onto the `.qv` spelling, leaving the legacy branch unexercised).
        selector = ("fluent_bit/busy_state.qcore",)
        self.assertTrue(contracts.owned("fluent_bit/busy_state.qcore", selector))
        self.assertTrue(contracts.owned("fluent_bit/busy_state.qv", selector))
        self.assertFalse(contracts.owned("fluent_bit/busy_state_extra.qv", selector))
        selector = ("fluent_bit/busy_state.qv",)
        self.assertTrue(contracts.owned("fluent_bit/busy_state.qv", selector))
        self.assertFalse(contracts.owned("fluent_bit/busy_state.qcore", selector))
        self.assertFalse(contracts.owned("fluent_bit/busy_state_extra.qv", selector))

    def test_generated_from_metadata_and_constants(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            manifest = contracts.load_path_manifest(tree.root, tree.metadata())
            self.assertEqual([c["name"] for c in manifest["crates"]],
                             ["qcore-provider-exe", "quirin-provider-app", "quirin-provider-libc"])
            exe, app, libc = manifest["crates"]
            self.assertEqual(exe["namespace"], "exe")
            self.assertEqual(exe["dependencies"], ["quirin-provider-libc"])
            self.assertEqual(exe["modules"], ["exe/wrap.qcore"])
            self.assertEqual(app["dependencies"], ["quirin-provider-libc"])
            self.assertEqual(app["namespace"], "app")
            self.assertEqual(libc["published"],
                             {"libc/memory.qv": hashlib.sha256(PARENT.encode()).hexdigest()})
            self.assertEqual(libc["contracts"], "quirin-provider-libc/contracts")

    def test_qv_published_module_and_include_pin_keep_exact_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            (tree.root / "quirin-provider-libc/contracts/libc/memory.qv").unlink()
            (tree.root / "quirin-provider-app/contracts/app/wrap.qcore").unlink()
            tree.crate("quirin-provider-libc", "libc", ["libc/memory.qv"],
                       {"libc/memory.qv": PARENT})
            child = CHILD.replace("memory.qcore", "memory.qv")
            tree.crate("quirin-provider-app", "app", [], {"app/wrap.qv": child})
            manifest = contracts.load_path_manifest(tree.root, tree.metadata())
            libc = manifest["crates"][2]
            self.assertEqual(libc["modules"], ["libc/memory.qv"])
            self.assertEqual(libc["published"],
                             {"libc/memory.qv": hashlib.sha256(PARENT.encode()).hexdigest()})
            pins = contracts.include_pins(
                contracts.files(tree.root / "quirin-provider-app/contracts"), manifest, tree.root)
            self.assertEqual(pins["quirin-provider-libc:libc/memory.qv"]["module"],
                             "libc/memory.qv")

    def test_dual_extension_same_stem_is_refused_before_manifest_selection(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            # The fixture already owns `libc/memory.qv`; the legacy spelling
            # of the same stem beside it is the ambiguous pair (EXT-01).
            owned_qv = tree.root / "quirin-provider-libc/contracts/libc/memory.qv"
            self.assertTrue(owned_qv.is_file())
            sibling = owned_qv.with_suffix(".qcore")
            self.assertFalse(sibling.exists())
            sibling.write_text(PARENT)
            with self.assertRaisesRegex(ValueError, "ambiguous contract asset stem"):
                contracts.load_path_manifest(tree.root, tree.metadata())

    def test_dev_and_build_dependencies_are_not_contract_edges(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            metadata = tree.metadata(({"name": "quirin-provider-libc", "kind": "dev"},))
            app = contracts.load_path_manifest(tree.root, metadata)["crates"][1]
            self.assertEqual(app["name"], "quirin-provider-app")
            self.assertEqual(app["dependencies"], [])

    def test_both_prefixes_derive_and_a_shared_namespace_is_refused(self):
        """Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING (option B): exactly two
        prefixes derive a namespace; a crate under each prefix with the same
        remainder derives one namespace twice and is refused; an executable
        crate records its constants in `src/main.rs`; a recorded namespace
        must still be the derived one; the conformance harness is not a
        contract crate, and may not grow a contract tree."""
        self.assertEqual(contracts.derive_namespace("quirin-provider-fluent-bit"), "fluent_bit")
        self.assertEqual(contracts.derive_namespace("qcore-provider-librdkafka"), "librdkafka")
        self.assertEqual(contracts.derive_namespace("qcore-provider-libevent"), "libevent")
        for refused in ("other-provider-x", "qcore-provider-", "qcore-provider-1x", "libc"):
            self.assertIsNone(contracts.derive_namespace(refused), refused)
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            tree.crate("qcore-provider-libc", "libc", [], {"libc/other.qcore": PARENT}, binary=True)
            metadata = tree.metadata()
            metadata["packages"].append({
                "name": "qcore-provider-libc", "version": "0.1.0",
                "manifest_path": str(tree.root / "qcore-provider-libc/Cargo.toml"),
                "targets": [{"kind": ["bin"],
                             "src_path": str(tree.root / "qcore-provider-libc/src/main.rs")}],
                "dependencies": []})
            with self.assertRaisesRegex(ValueError, "namespace 'libc' is also derived by"):
                contracts.load_path_manifest(tree.root, metadata)
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            (tree.root / "qcore-provider-exe/src/main.rs").write_text(lib_rs("other", []))
            with self.assertRaisesRegex(ValueError, "not the derived"):
                contracts.load_path_manifest(tree.root, tree.metadata())
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            (tree.root / "conformance/contracts").mkdir()
            with self.assertRaisesRegex(ValueError, "owning no contracts"):
                contracts.load_path_manifest(tree.root, tree.metadata())

    def test_recorded_constants_are_checked(self):
        cases = [
            ("other", ["libc/memory.qv"], "not the derived"),
            ("libc", ["libc/absent.qcore"], "does not own"),
            ("libc", ["libc/memory.qv", "libc/memory.qv"], "sorted and unique"),
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
            contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                         views=loader_like)
            contracts.maintain_load_path(tree.root, metadata=metadata, views=loader_like)
            (tree.root / "quirin-provider-libc/contracts/libc/memory.qv").write_text(
                PARENT.replace("nullable", "nonnull"))
            with self.assertRaisesRegex(ValueError, "stale"):
                contracts.maintain_load_path(tree.root, metadata=metadata, views=loader_like)

    def test_resource_views_are_generated_by_sync_and_checked(self):
        """A view is the loader's, never a hand list: `sync` writes it, and
        `check` refuses a view that declares a module the loader refuses,
        omits one it accepts, or is missing."""
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            tree.crate("quirin-provider-app", "app", [], {"app/policy.qcore": POLICY})
            metadata = tree.metadata()
            written = contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                                   views=loader_like)
            views = {c["name"]: c["resource_view"] for c in written["crates"]}
            self.assertEqual(views, {"qcore-provider-exe": ["exe/wrap.qcore"],
                                     "quirin-provider-app": ["app/wrap.qcore"],
                                     "quirin-provider-libc": ["libc/memory.qv"]})
            load_path_file = tree.root / "contract-load-path.json"
            self.assertEqual(json.loads(load_path_file.read_text()), written)
            contracts.maintain_load_path(tree.root, metadata=metadata, views=loader_like)
            good = load_path_file.read_bytes()

            def declare(view):
                manifest = json.loads(good)
                for record in manifest["crates"]:
                    if record["name"] == "quirin-provider-app":
                        if view is None:
                            del record["resource_view"]
                        else:
                            record["resource_view"] = view
                load_path_file.write_text(json.dumps(manifest, indent=2) + "\n")

            cases = [
                (["app/policy.qcore", "app/wrap.qcore"],
                 r"quirin-provider-app declares modules the resource loader refuses: "
                 r"\['app/policy.qcore'\]"),
                ([], r"quirin-provider-app omits modules the resource loader accepts: "
                     r"\['app/wrap.qcore'\]"),
                (None, "quirin-provider-app declares no resource view"),
            ]
            for view, message in cases:
                with self.subTest(view=view):
                    declare(view)
                    with self.assertRaisesRegex(ValueError, message):
                        contracts.maintain_load_path(tree.root, metadata=metadata,
                                                     views=loader_like)
            # Restored by rewriting the bytes; the round trip is exact.
            load_path_file.write_bytes(good)
            self.assertEqual(load_path_file.read_bytes(), good)
            contracts.maintain_load_path(tree.root, metadata=metadata, views=loader_like)

    def test_bounds_views_are_generated_by_sync_and_checked(self):
        """Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds view is the
        bounds loader's, written by `sync` beside the resource view, checked by
        `check`, and disjoint from the resource view."""
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            tree.crate("quirin-provider-app", "app", [], {"app/meta.qv": BOUNDS})
            metadata = tree.metadata()
            written = contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                                   views=loader_with_bounds)
            bounds = {c["name"]: c["bounds_view"] for c in written["crates"]}
            resource = {c["name"]: c["resource_view"] for c in written["crates"]}
            self.assertEqual(bounds, {"qcore-provider-exe": [], "quirin-provider-app": ["app/meta.qv"],
                                      "quirin-provider-libc": []})
            self.assertEqual(resource["quirin-provider-app"], ["app/wrap.qcore"])
            contracts.maintain_load_path(tree.root, metadata=metadata, views=loader_with_bounds)
            load_path_file = tree.root / "contract-load-path.json"
            good = load_path_file.read_bytes()

            def declare(key, view):
                manifest = json.loads(good)
                for record in manifest["crates"]:
                    if record["name"] == "quirin-provider-app":
                        if view is None:
                            del record[key]
                        else:
                            record[key] = view
                load_path_file.write_text(json.dumps(manifest, indent=2) + "\n")

            cases = [
                ("bounds_view", [], r"quirin-provider-app omits modules the bounds loader accepts: "
                                    r"\['app/meta.qv'\]"),
                ("bounds_view", None, "quirin-provider-app declares no bounds view"),
                ("resource_view", ["app/meta.qv", "app/wrap.qcore"],
                 r"quirin-provider-app declares modules the resource loader refuses: "
                 r"\['app/meta.qv'\]"),
            ]
            for key, view, message in cases:
                with self.subTest(key=key, view=view):
                    declare(key, view)
                    with self.assertRaisesRegex(ValueError, message):
                        contracts.maintain_load_path(tree.root, metadata=metadata,
                                                     views=loader_with_bounds)
            load_path_file.write_bytes(good)
            self.assertEqual(load_path_file.read_bytes(), good)

            def both(root, load_path_file):
                views = loader_with_bounds(root, load_path_file)
                views["quirin-provider-app"]["resource_view"] = ["app/meta.qv", "app/wrap.qcore"]
                return views

            with self.assertRaisesRegex(ValueError, "in both the resource and the bounds view"):
                contracts.maintain_load_path(tree.root, sync=True, metadata=metadata, views=both)

    def test_a_probe_failure_or_an_unowned_view_is_an_error_not_an_empty_view(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            metadata = tree.metadata()
            # No repository workspace above this tree: the probe cannot run.
            with self.assertRaisesRegex(ValueError, "resource view probe failed"):
                contracts.maintain_load_path(tree.root, sync=True, metadata=metadata)

            def unowned(root, load_path_file):
                views = loader_like(root, load_path_file)
                views["quirin-provider-libc"] = ["libc/absent.qcore"]
                return views

            with self.assertRaisesRegex(ValueError, "not sorted, unique and owned"):
                contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                             views=unowned)

            def partial(root, load_path_file):
                views = loader_like(root, load_path_file)
                del views["quirin-provider-libc"]
                return views

            with self.assertRaisesRegex(ValueError, "not the load path's"):
                contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                             views=partial)

    def test_include_pins_install_and_verify(self):
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            load_path = contracts.maintain_load_path(tree.root, sync=True, metadata=tree.metadata(),
                                                     views=loader_like)
            provider = {"app/wrap.qcore": tree.root / "quirin-provider-app/contracts/app/wrap.qcore"}
            pins = contracts.include_pins(provider, load_path, tree.root)
            key = "quirin-provider-libc:libc/memory.qv"
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
            (tree.root / "quirin-provider-libc/contracts/libc/memory.qv").write_text("changed")
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

    def test_inventory_locks_are_generated_by_sync_and_checked(self):
        """Card CONTRACT-LOCKS: `sync` writes one `contract-inventory.lock` per
        load-path crate; `check` refuses a contract changed without its lock, a
        lock changed without its contract, a missing lock, a lock outside every
        load-path crate and a probe that does not describe exactly the tree."""
        with tempfile.TemporaryDirectory() as temporary:
            tree = Tree(temporary)
            metadata = tree.metadata()
            load_path = contracts.maintain_load_path(tree.root, sync=True, metadata=metadata,
                                                     views=loader_like)
            rows = contracts.maintain_inventory_locks(tree.root, load_path, sync=True,
                                                      probe=inventory_like)
            self.assertEqual([(r["crate"], r["files"], r["declarations"], r["clauses"])
                              for r in rows],
                             [("qcore-provider-exe", 1, 1, 3), ("quirin-provider-app", 1, 1, 3),
                              ("quirin-provider-libc", 1, 1, 3)])
            lock_file = tree.root / "quirin-provider-app" / contracts.INVENTORY_LOCK
            lock = json.loads(lock_file.read_text())
            source = tree.root / "quirin-provider-app/contracts/app/wrap.qcore"
            self.assertEqual(lock["crate"], "quirin-provider-app")
            self.assertEqual(lock["resource_view"], ["app/wrap.qcore"])
            self.assertEqual((lock["bounds_view"], lock["outer_view"]), ([], []))
            self.assertEqual(lock["files"]["app/wrap.qcore"], {
                "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                "declaration_count": 1, "clause_count": 3,
                "declarations": [{"name": "wrap_free", "clauses": 3}],
                "inherits": [{"child": "wrap_free", "parent": "libc::free"}],
                "premises": {}, "premise_bindings": []})
            self.assertEqual(lock_file.read_text(), contracts.canonical(lock))
            contracts.maintain_inventory_locks(tree.root, load_path, probe=inventory_like)

            good_lock, good_source = lock_file.read_bytes(), source.read_bytes()
            mutations = [
                (source, good_source + b"// a comment changes the digest\n",
                 r"stale contract inventory lock: .*app/wrap.qcore differs in \['sha256'\]"),
                (lock_file, good_lock.replace(b'"clauses": 3', b'"clauses": 4'),
                 r"app/wrap.qcore differs in \['declarations'\]"),
                (lock_file, good_lock.replace(b'"libc::free"', b'"libc::malloc"'),
                 r"app/wrap.qcore differs in \['inherits'\]"),
                (lock_file, good_lock.replace(b'"app/wrap.qcore"\n  ]', b'\n  ]'),
                 "resource_view differs"),
                (lock_file, good_lock.replace(b"\n", b"\r\n"), "byte form differs"),
            ]
            for path, mutated, message in mutations:
                with self.subTest(message=message):
                    original = path.read_bytes()
                    self.assertNotEqual(mutated, original)
                    path.write_bytes(mutated)
                    try:
                        with self.assertRaisesRegex(ValueError, message):
                            contracts.maintain_inventory_locks(tree.root, load_path,
                                                               probe=inventory_like)
                    finally:
                        # Restored by rewriting the bytes; the round trip is exact.
                        path.write_bytes(original)
                    self.assertEqual(path.read_bytes(), original)

            lock_file.unlink()
            with self.assertRaisesRegex(ValueError, "missing contract inventory lock"):
                contracts.maintain_inventory_locks(tree.root, load_path, probe=inventory_like)
            lock_file.write_bytes(good_lock)
            stray = tree.root / "conformance" / contracts.INVENTORY_LOCK
            stray.write_text("{}\n")
            with self.assertRaisesRegex(ValueError, "outside a load-path crate"):
                contracts.maintain_inventory_locks(tree.root, load_path, probe=inventory_like)
            stray.unlink()

            def missing_module(root, load_path_file):
                facts = inventory_like(root, load_path_file)
                facts["quirin-provider-app"].pop("app/wrap.qcore")
                return facts

            def missing_crate(root, load_path_file):
                facts = inventory_like(root, load_path_file)
                facts.pop("qcore-provider-exe")
                return facts

            with self.assertRaisesRegex(ValueError, "not the contract files"):
                contracts.maintain_inventory_locks(tree.root, load_path, probe=missing_module)
            with self.assertRaisesRegex(ValueError, "not the load path's"):
                contracts.maintain_inventory_locks(tree.root, load_path, probe=missing_crate)
            contracts.maintain_inventory_locks(tree.root, load_path, probe=inventory_like)

    def test_the_real_workspace_manifest_is_current(self):
        manifest = contracts.maintain_load_path()
        names = [c["name"] for c in manifest["crates"]]
        self.assertTrue(all(name.startswith(("quirin-provider-", "qcore-provider-")) for name in names))
        self.assertIn("quirin-provider-fluent-bit", names)
        self.assertNotIn("qcore-provider-conformance", names)
        # Card W45-USER-CONTRACT-PLUGINS: the SDK is a workspace member that
        # owns no contracts, so it is not on the load path; 11 crates remain.
        self.assertNotIn("quirin-provider-plugin", names)
        self.assertEqual(len(names), 11)
        self.assertEqual(sum(name.startswith("qcore-provider-") for name in names), 5)
        edges = {c["name"]: c["dependencies"] for c in manifest["crates"]}
        self.assertEqual(edges["quirin-provider-fluent-bit"], ["quirin-provider-libc"])
        for name in ("qcore-provider-librdkafka", "qcore-provider-libevent"):
            self.assertIn("quirin-provider-libc", edges[name])
        namespaces = [c["namespace"] for c in manifest["crates"]]
        self.assertEqual(len(namespaces), len(set(namespaces)))
        # Combined W46 cards: 54 selected modules, 219 resource declarations, 118 corpus modules.
        # 54 -> 55 by card W47-LOOP-OWNER-CONTRACTS: `fluent_bit/loop_owner.qv`.
        # 55 -> 56 (libc 16 -> 17) by card W47-CONDVAR-JOIN-PUBLICATION:
        # `libc/pthread_sync.qv`.
        # 56 -> 57 (fluent-bit 19 -> 20) by card W47-CONDVAR-JOIN-STOP-PATHS:
        # `fluent_bit/lib_engine_thread.qv`, five declarations awaiting review.
        # 57 -> 58 by card W47-LOOP-OWNER-INSTANCE-FLOW: `fluent_bit/input_instance.qv`.
        # 58 -> 60 (libevent) by cards CONTRACT-PIN-TIP-MODE (`monkey/vhost_set.qv`)
        # and MONKEY-VHOST-PROPERTY-CUSTODY (`monkey/string.qv`); this pin was
        # left stale by both (PROVIDER-LOAD-PATH-VIEW-PIN-STALE).
        # 60 -> 61 by card W43-ABI-ENUM-AND-CONST-PP: `librdkafka/client.qv`.
        # 61 -> 63 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION:
        # `monkey/event_loop.qv` and `fluent_bit/input_collector_registration.qv`.
        # 63 -> 64 by card FB-TEARDOWN-DESTROY-DECLARATIONS:
        # `fluent_bit/teardown_destroy.qv`.
        views = {c["name"]: c["resource_view"] for c in manifest["crates"]}
        # b5f74d79a added librdkafka/logging.qv: the committed view became 65.
        # e76b048f6 added librdkafka/log_caller_frame.qv before this card: 66.
        # Card CONTRACT-LOCKS: the loader's views (recomputed above) are read
        # against each crate's `contract-inventory.lock`, not a literal count
        # (66 modules in all and 17 in libc at base 6c4855745), and every lock
        # is checked against its tree with the loaders' own parser.
        locks = {name: json.loads((HERE / name / contracts.INVENTORY_LOCK).read_text())
                 for name in names}
        for name in names:
            for key in (contracts.RESOURCE_VIEW, contracts.BOUNDS_VIEW, contracts.OUTER_VIEW):
                self.assertEqual(locks[name][key], next(
                    c[key] for c in manifest["crates"] if c["name"] == name), (name, key))
        self.assertEqual(sum(len(view) for view in views.values()),
                         sum(len(lock["resource_view"]) for lock in locks.values()))
        inventory = contracts.maintain_inventory_locks(load_path=manifest)
        self.assertEqual([row["crate"] for row in inventory], names)
        self.assertIn("librdkafka/logging.qv", views["qcore-provider-librdkafka"])
        self.assertIn("librdkafka/log_caller_frame.qv", views["qcore-provider-librdkafka"])
        self.assertEqual(views["quirin-provider-shared"], [])
        self.assertEqual(views["quirin-provider-libc"], locks["quirin-provider-libc"]["resource_view"])
        self.assertTrue(views["quirin-provider-libc"])
        # Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the one bounds module, in its
        # crate's bounds view and in no resource view. Card
        # CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS adds the second,
        # `chunkio/file_st_bounds.qv`.
        # Card NULL-SAFETY-PROPERTY adds the three `quant null-deref` modules
        # (bounds contract v1, read only by the null-dereference property).
        # Card NULL-TRIAGE-UNRESOLVED-PATHS adds cfl's first bounds view
        # (`cfl_list_size`'s `nonnull_result_range`). Card
        # CELL-RESULT-SET-WRITTEN-SET-DERIVATION adds Fluent Bit's second,
        # `fluent_bit/result_range.qv` (`flb_pipe_create`'s written sets).
        # Card CELL-GUARD-RESULT-RANGE-CHECKED adds libevent's first,
        # `monkey/result_range.qv`, and libc's first, `libc/result_sets.qv`;
        # this pin was left stale (providers/tests
        # is outside the scoped card gate).
        bounds = {c["name"]: c["bounds_view"] for c in manifest["crates"]}
        self.assertEqual({name: view for name, view in bounds.items() if view},
                         {"qcore-provider-chunkio": ["chunkio/file_st_bounds.qv",
                                                     "chunkio/meta_len_bounds.qv"],
                          "qcore-provider-cmetrics": ["cmetrics/null_deref.qv"],
                          "qcore-provider-libevent": ["monkey/result_range.qv"],
                          "qcore-provider-librdkafka": ["librdkafka/null_deref.qv"],
                          "quirin-provider-cfl": ["cfl/null_deref.qv"],
                          "quirin-provider-fluent-bit": ["fluent_bit/null_deref.qv",
                                                         "fluent_bit/result_range.qv"],
                          "quirin-provider-libc": ["libc/result_sets.qv"]})
        self.assertNotIn("chunkio/meta_len_bounds.qv", views["qcore-provider-chunkio"])
        self.assertNotIn("chunkio/file_st_bounds.qv", views["qcore-provider-chunkio"])
        # The real inventories: exactly the two crates that inherit carry pins.
        rows = {row["crate"]: row["includes"] for row in contracts.maintain(load_path=manifest)}
        self.assertEqual(rows, {"qcore-provider-chunkio": 0, "qcore-provider-cmetrics": 0,
                                "qcore-provider-ctraces": 0, "qcore-provider-libevent": 1,
                                "qcore-provider-librdkafka": 2})


if __name__ == "__main__":
    unittest.main()
