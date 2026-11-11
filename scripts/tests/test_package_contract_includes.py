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

"""Card CONTRACT-INHERIT: a provider package pins every included parent.

`providers/package.py verify` must fail when a packaged contract includes a
module whose identity is not pinned, when a pinned parent's text changes or
is missing, and when a pin is unused; a package that includes nothing is
verified exactly as before. Fixture packages only: no real package includes
anything yet (adoption is `CONTRACT-INHERIT-ADOPTION`).
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

PACKAGE_PATH = Path(__file__).resolve().parents[2] / "providers/package.py"
SPEC = importlib.util.spec_from_file_location("provider_package_includes", PACKAGE_PATH)
package_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package_module)

LIBC = "quirin-provider-libc"
PARENT = (
    "module contract;\n"
    "extern fn free(ptr: ptr<mut, u8>) -> unit\n"
    "contract {\n"
    "    requires resource_contract_v1;\n"
    "    ensures normal resource_released(ptr, nullable);\n"
    "    ensures normal resource_return_none();\n"
    "};\n"
)
CHILD = (
    "module contract;\n"
    f'#include "{LIBC}:libc/heap.qcore";\n'
    "// A comment naming #include \"x:y.qcore\"; is not a directive.\n"
    "extern fn flb_free(p: ptr<mut, u8>) -> unit\n"
    "contract inherit libc::free;\n"
)


def sha(data):
    return hashlib.sha256(data.encode()).hexdigest()


class PackageContractIncludes(unittest.TestCase):
    def package(self, root, child=CHILD, pins=True, parent=PARENT):
        package = Path(root) / "qcore.example"
        asset = package / "contracts/example/memory.qcore"
        asset.parent.mkdir(parents=True)
        # Install exactly the UTF-8 bytes hashed by the inventory on every OS.
        asset.write_bytes(child.encode("utf-8"))
        inventory = {
            "schema_version": 1,
            "plugin_id": "qcore.example",
            "contracts": {"contracts/example/memory.qcore": sha(child)},
        }
        if pins:
            inventory["includes"] = {
                f"{LIBC}:libc/heap.qcore": {
                    "crate": LIBC,
                    "version": "0.1.0",
                    "module": "libc/heap.qcore",
                    "sha256": sha(PARENT),
                }
            }
            included = package / "includes" / LIBC / "libc/heap.qcore"
            included.parent.mkdir(parents=True)
            included.write_bytes(parent.encode("utf-8"))
        payload = (json.dumps(inventory, indent=2) + "\n").encode()
        (package / "contracts.json").write_bytes(payload)
        executable = (b"prefix" + package_module.CONTRACTS_BEGIN + payload
                      + package_module.CONTRACTS_END + b"suffix")
        return package, executable

    def verify(self, package, executable):
        return package_module.verify_contracts(package, executable, "qcore.example")

    def test_pinned_parents_verify(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, executable = self.package(temporary)
            result = self.verify(package, executable)
            self.assertEqual(result["contracts"], 1)
            self.assertEqual(result["includes"], 1)

    def test_a_package_that_includes_nothing_is_unchanged(self):
        with tempfile.TemporaryDirectory() as temporary:
            plain = CHILD.replace(f'#include "{LIBC}:libc/heap.qcore";\n', "")
            package, executable = self.package(temporary, child=plain, pins=False)
            result = self.verify(package, executable)
            self.assertEqual(set(result), {"payload_sha256", "contracts"})

    def test_an_unpinned_include_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, executable = self.package(temporary, pins=False)
            with self.assertRaisesRegex(ValueError, "not pinned"):
                self.verify(package, executable)

    def test_a_changed_parent_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, executable = self.package(
                temporary, parent=PARENT.replace("nullable", "nonnull"))
            with self.assertRaisesRegex(ValueError, "included parent changed"):
                self.verify(package, executable)

    def test_a_missing_parent_is_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            package, executable = self.package(temporary)
            (package / "includes" / LIBC / "libc/heap.qcore").unlink()
            with self.assertRaisesRegex(ValueError, "set mismatch"):
                self.verify(package, executable)

    def test_an_unused_pin_and_a_stray_tree_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            plain = CHILD.replace(f'#include "{LIBC}:libc/heap.qcore";\n', "")
            package, executable = self.package(temporary, child=plain)
            with self.assertRaisesRegex(ValueError, "no contract reaches"):
                self.verify(package, executable)
        with tempfile.TemporaryDirectory() as temporary:
            plain = CHILD.replace(f'#include "{LIBC}:libc/heap.qcore";\n', "")
            package, executable = self.package(temporary, child=plain, pins=False)
            (package / "includes").mkdir()
            with self.assertRaisesRegex(ValueError, "no pinned identities"):
                self.verify(package, executable)

    def test_a_malformed_directive_is_refused_not_skipped(self):
        for directive in ('#include "../escape.qcore";', "#include libc;",
                          f'#include "{LIBC}:/abs.qcore";'):
            with self.subTest(directive=directive), tempfile.TemporaryDirectory() as temporary:
                child = CHILD.replace(f'#include "{LIBC}:libc/heap.qcore";', directive)
                package, executable = self.package(temporary, child=child)
                with self.assertRaisesRegex(ValueError, "malformed #include"):
                    self.verify(package, executable)

    def test_a_transitively_included_parent_must_be_pinned_too(self):
        with tempfile.TemporaryDirectory() as temporary:
            nested = f'module contract;\n#include "{LIBC}:libc/base.qcore";\n' + PARENT[len("module contract;\n"):]
            package, executable = self.package(temporary, parent=nested)
            inventory = json.loads((package / "contracts.json").read_text())
            inventory["includes"][f"{LIBC}:libc/heap.qcore"]["sha256"] = sha(nested)
            payload = (json.dumps(inventory, indent=2) + "\n").encode()
            (package / "contracts.json").write_bytes(payload)
            executable = (b"prefix" + package_module.CONTRACTS_BEGIN + payload
                          + package_module.CONTRACTS_END + b"suffix")
            with self.assertRaisesRegex(ValueError, "libc/base.qcore is not pinned"):
                self.verify(package, executable)


if __name__ == "__main__":
    unittest.main()
