import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest


PACKAGE_PATH = Path(__file__).parents[1] / "package.py"
SPEC = importlib.util.spec_from_file_location("provider_package", PACKAGE_PATH)
provider_package = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(provider_package)


class ContractBundleVerification(unittest.TestCase):
    def bundle(self, root):
        package = Path(root) / "qcore.example"
        asset = package / "contracts/example/policy.qcore"
        asset.parent.mkdir(parents=True)
        asset.write_text("resource type Example;\n")
        inventory = {
            "schema_version": 1,
            "plugin_id": "qcore.example",
            "contracts": {
                "contracts/example/policy.qcore": hashlib.sha256(
                    asset.read_bytes()
                ).hexdigest()
            },
        }
        payload = (json.dumps(inventory, indent=2) + "\n").encode()
        (package / "contracts.json").write_bytes(payload)
        executable = (
            b"prefix"
            + provider_package.CONTRACTS_BEGIN
            + payload
            + provider_package.CONTRACTS_END
            + b"suffix"
        )
        return package, asset, executable

    def test_exact_recursive_bundle_and_legacy_absence_are_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(temporary)
            result = provider_package.verify_contracts(
                package, executable, "qcore.example"
            )
            self.assertEqual(result["contracts"], 1)
            legacy = Path(temporary) / "legacy"
            legacy.mkdir()
            self.assertIsNone(
                provider_package.verify_contracts(legacy, b"old", "qcore.legacy")
            )

    def test_missing_tampered_extra_and_mismatched_inventory_are_refused(self):
        mutations = ("missing", "tampered", "tampered_rehashed", "extra", "inventory")
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                package, asset, executable = self.bundle(temporary)
                if mutation == "missing":
                    asset.unlink()
                elif mutation == "tampered":
                    asset.write_text("changed\n")
                elif mutation == "tampered_rehashed":
                    asset.write_text("changed and rehashed\n")
                    inventory = json.loads((package / "contracts.json").read_text())
                    inventory["contracts"]["contracts/example/policy.qcore"] = (
                        hashlib.sha256(asset.read_bytes()).hexdigest()
                    )
                    (package / "contracts.json").write_text(json.dumps(inventory))
                elif mutation == "extra":
                    (asset.parent / "extra.qcore").write_text("extra\n")
                else:
                    inventory = json.loads((package / "contracts.json").read_text())
                    inventory["plugin_id"] = "qcore.other"
                    (package / "contracts.json").write_text(json.dumps(inventory))
                with self.assertRaises(ValueError):
                    provider_package.verify_contracts(
                        package, executable, "qcore.example"
                    )

    def test_symbolic_link_asset_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, asset, executable = self.bundle(temporary)
            outside = Path(temporary) / "outside.qcore"
            outside.write_bytes(asset.read_bytes())
            asset.unlink()
            try:
                os.symlink(outside, asset)
            except OSError as error:
                self.skipTest(f"symbolic links unavailable: {error}")
            with self.assertRaisesRegex(ValueError, "symbolic-link"):
                provider_package.verify_contracts(
                    package, executable, "qcore.example"
                )

    def test_contract_paths_are_canonical_and_contained(self):
        for path in (
            "/contracts/example/a.qcore",
            "contracts/../a.qcore",
            "contracts/example/../../a.qcore",
            "contracts/example\\a.qcore",
            "other/example/a.qcore",
            "contracts/example/a.txt",
        ):
            with self.subTest(path=path), self.assertRaises(ValueError):
                provider_package.contract_path(path)

    def test_archive_is_recursive_and_deterministic(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, _, _ = self.bundle(temporary)
            (package / "provider").write_bytes(b"executable")
            (package / "verifier-plugin.json").write_text("{}\n")
            first = Path(temporary) / "first.tar.gz"
            second = Path(temporary) / "second.tar.gz"
            provider_package.archive(package, first)
            provider_package.archive(package, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(first, "r:gz") as archive:
                self.assertEqual(
                    archive.getnames(),
                    [
                        "qcore.example/contracts.json",
                        "qcore.example/contracts/example/policy.qcore",
                        "qcore.example/provider",
                        "qcore.example/verifier-plugin.json",
                    ],
                )


if __name__ == "__main__":
    unittest.main()
