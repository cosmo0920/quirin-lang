# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

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
    def bundle(self, root, extension=".qcore", source="resource type Example;\n"):
        package = Path(root) / "qcore.example"
        asset = package / f"contracts/example/policy{extension}"
        asset.parent.mkdir(parents=True)
        asset.write_text(source)
        inventory = {
            "schema_version": 1,
            "plugin_id": "qcore.example",
            "contracts": {
                f"contracts/example/policy{extension}": hashlib.sha256(
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
        self.assertEqual(provider_package.contract_path("contracts/example/a.qv").as_posix(),
                         "contracts/example/a.qv")
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

    def test_qv_include_is_scanned_and_dual_stems_are_refused(self):
        self.assertEqual(provider_package.contract_include_targets(
            '#include "quirin-provider-libc:libc/memory.qv";\n', "child.qv"),
            ["quirin-provider-libc:libc/memory.qv"])
        with self.assertRaisesRegex(ValueError, "ambiguous contract asset stem"):
            provider_package.reject_dual_stems(
                ["contracts/example/policy.qcore", "contracts/example/policy.qv"])
        with self.assertRaisesRegex(ValueError, "ambiguous contract asset stem"):
            provider_package.reject_dual_stems(
                ["includes/libc/example/policy.qcore", "includes/libc/example/policy.qv"])

    def test_declared_dual_stem_bundle_is_refused_even_with_valid_hashes(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, asset, _ = self.bundle(temporary)
            sibling = asset.with_suffix(".qv")
            sibling.write_bytes(asset.read_bytes())
            inventory = json.loads((package / "contracts.json").read_text())
            inventory["contracts"]["contracts/example/policy.qv"] = hashlib.sha256(
                sibling.read_bytes()).hexdigest()
            payload = (json.dumps(inventory, indent=2) + "\n").encode()
            (package / "contracts.json").write_bytes(payload)
            executable = (provider_package.CONTRACTS_BEGIN + payload
                          + provider_package.CONTRACTS_END)
            with self.assertRaisesRegex(ValueError, "ambiguous contract asset stem"):
                provider_package.verify_contracts(package, executable, "qcore.example")

    def test_qv_requires_parser_detected_contract_mode(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(
                temporary, ".qv", "module contract;\nresource type Example;\n")
            self.assertEqual(provider_package.verify_contracts(
                package, executable, "qcore.example")["contracts"], 1)
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(
                temporary, ".qv", "module executable;\nfn main() -> unit;\n")
            with self.assertRaisesRegex(ValueError, ".qv requires contract mode"):
                provider_package.verify_contracts(package, executable, "qcore.example")

    def test_legacy_warning_reports_parser_mode_and_recommendation(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(
                temporary, ".qcore", "module contract;\nresource type Example;\n")
            from contextlib import redirect_stderr
            from io import StringIO
            warning = StringIO()
            with redirect_stderr(warning):
                provider_package.verify_contracts(
                    package, executable, "qcore.example", warn_legacy=True)
            self.assertIn("contracts/example/policy.qcore", warning.getvalue())
            self.assertIn("(contract mode); use .qv", warning.getvalue())

    def test_parser_probe_round_trips_special_path_characters(self):
        with tempfile.TemporaryDirectory() as temporary:
            asset = Path(temporary) / 'é "quoted"\\name\npolicy.qv'
            asset.write_text("module contract;\nresource type Example;\n")
            provider_package.validate_source_modes({"contracts/example/policy.qv": asset})

    def test_malformed_header_refuses_qv_but_keeps_legacy_visible(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(
                temporary, ".qv", "module other;\nresource type Example;\n")
            with self.assertRaisesRegex(ValueError, ".qv requires contract mode"):
                provider_package.verify_contracts(package, executable, "qcore.example")
        with tempfile.TemporaryDirectory() as temporary:
            package, _, executable = self.bundle(
                temporary, ".qcore", "module other;\nresource type Example;\n")
            from contextlib import redirect_stderr
            from io import StringIO
            warning = StringIO()
            with redirect_stderr(warning):
                self.assertEqual(provider_package.verify_contracts(
                    package, executable, "qcore.example", warn_legacy=True)["contracts"], 1)
            self.assertIn("(unknown mode); use .qv", warning.getvalue())

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
