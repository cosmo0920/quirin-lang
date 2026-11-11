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

"""Provider-owned source edits must reach package inventories deliberately."""
import importlib.util
import json
import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[2] / "providers/contracts.py"
spec = importlib.util.spec_from_file_location("provider_contract_ownership", SCRIPT)
contracts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contracts)


class Ownership(unittest.TestCase):
    def symlink_or_assert_denied(self, link, target, *, directory=False):
        try:
            link.symlink_to(target, target_is_directory=directory)
        except OSError as error:
            self.assertEqual((sys.platform, getattr(error, "winerror", None)), ("win32", 1314))
            return False
        return True

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "providers"
        self.provider = self.root / "qcore-provider-example"
        self.source = self.provider / "contracts/example/model.qcore"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("original policy\n")
        (self.provider / "provider-template.json").write_text(json.dumps({"plugin_id": "qcore.example"}))
        owner_patch = patch.object(contracts, "OWNERS", {"qcore-provider-example": ("example",)})
        owner_patch.start()
        self.addCleanup(owner_patch.stop)
        contracts.maintain(self.root, sync=True)

    def test_changed_source_requires_deliberate_inventory_sync(self):
        self.source.write_text("changed policy\n")
        with self.assertRaisesRegex(ValueError, "stale"):
            contracts.maintain(self.root)
        contracts.maintain(self.root, sync=True)
        contracts.maintain(self.root)

    def test_canonical_omission_from_inventory_is_refused(self):
        added = self.source.with_name("added.qcore")
        added.write_text("new policy\n")
        with self.assertRaisesRegex(ValueError, "stale"):
            contracts.maintain(self.root)
        contracts.maintain(self.root, sync=True)
        contracts.maintain(self.root)
        inventory = json.loads((self.provider / "contracts.json").read_text())
        self.assertIn("contracts/example/added.qcore", inventory["contracts"])

    def test_missing_inventory_and_stale_hash_are_refused(self):
        inventory = self.provider / "contracts.json"
        data = json.loads(inventory.read_text())
        data["contracts"]["contracts/example/model.qcore"] = "0" * 64
        inventory.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "stale"):
            contracts.maintain(self.root)
        inventory.unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            contracts.maintain(self.root)

    def test_legacy_contract_corpus_is_refused_by_check_and_sync(self):
        legacy = self.root.parent / "contract/qcore"
        legacy.mkdir(parents=True)
        with self.assertRaisesRegex(ValueError, "must be absent"):
            contracts.maintain(self.root)
        with self.assertRaisesRegex(ValueError, "must be absent"):
            contracts.maintain(self.root, sync=True)

    def test_inventory_symlink_never_overwrites_an_external_file(self):
        inventory = self.provider / "contracts.json"
        inventory.unlink()
        external = self.root.parent / "outside.json"
        external.write_text("keep")
        if not self.symlink_or_assert_denied(inventory, external):
            self.assertEqual(external.read_text(), "keep")
            return
        with self.assertRaisesRegex(ValueError, "symbolic-link"):
            contracts.maintain(self.root, sync=True)
        self.assertEqual(external.read_text(), "keep")

    def test_sync_accepts_an_alias_above_the_repository_boundary(self):
        alias = self.root.parent / "repo-alias"
        if not self.symlink_or_assert_denied(alias, self.root.parent, directory=True):
            return
        contracts.maintain(alias / "providers", sync=True)
        contracts.maintain(alias / "providers")

    def test_unowned_contract_is_refused(self):
        self.source.parent.rename(self.source.parent.with_name("other"))
        with self.assertRaisesRegex(ValueError, "unowned"):
            contracts.maintain(self.root)

    def test_contract_symlink_is_refused(self):
        external = self.root.parent / "outside.qcore"
        external.write_text("outside policy\n")
        linked = self.source.with_name("linked.qcore")
        if not self.symlink_or_assert_denied(linked, external):
            self.assertEqual(external.read_text(), "outside policy\n")
            return
        with self.assertRaisesRegex(ValueError, "symbolic link"):
            contracts.maintain(self.root, sync=True)


if __name__ == "__main__":
    unittest.main()
