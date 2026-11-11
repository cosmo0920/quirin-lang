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

"""Supplementary checks on card O01's fixture seeds and archived identities.

The card's decision procedure is `qcore_core::chunk_integrity`; these tests only
confirm that the preparation driver seeds the images it says it seeds, and that
the archived evidence still names the pinned sources it was built from.
"""
import json
from pathlib import Path
import sys
import unittest
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.dont_write_bytecode = True
import cycle19_file_integrity_observations as driver  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
REPORT = REPO / "docs/reports/remaining-o01"

HEADER_MIN = 24
CONTENT_OFFSET = 22
CONTENT_LENGTH_OFFSET = 10


def classify(image):
    """An independent statement of the pinned layout and checksum rules."""
    if len(image) < HEADER_MIN:
        return "truncated_header"
    if image[0] != 0xC1 or image[1] != 0x00:
        return "bad_magic"
    metadata = int.from_bytes(image[CONTENT_OFFSET:CONTENT_OFFSET + 2], "big")
    content = int.from_bytes(
        image[CONTENT_LENGTH_OFFSET:CONTENT_LENGTH_OFFSET + 4], "big")
    if HEADER_MIN + metadata + content > len(image):
        return "truncated_content"
    region = image[CONTENT_OFFSET:CONTENT_OFFSET + 2 + metadata + content]
    stored = int.from_bytes(image[2:6], "big")
    if zlib.crc32(region) & 0xFFFFFFFF != stored:
        return "bad_checksum"
    return "clean"


class FixtureSeedTest(unittest.TestCase):
    def test_each_scenario_seeds_the_class_it_declares(self):
        expected = ["clean", "bad_checksum", "bad_magic", "truncated_header",
                    "truncated_content", "clean", "bad_checksum", "clean"]
        self.assertEqual(driver.SCENARIOS, len(expected))
        for scenario, name in enumerate(expected):
            self.assertEqual(classify(driver.fixture_bytes(scenario)), name, scenario)

    def test_the_family_covers_every_class_and_every_injection(self):
        classes = {classify(driver.fixture_bytes(s)) for s in range(driver.SCENARIOS)}
        self.assertEqual(classes, {"clean", "bad_checksum", "bad_magic",
                                   "truncated_header", "truncated_content"})

    def test_the_clean_seed_is_only_clean_because_its_checksum_agrees(self):
        image = bytearray(driver.fixture_bytes(0))
        self.assertEqual(classify(bytes(image)), "clean")
        image[2] ^= 0xFF
        self.assertEqual(classify(bytes(image)), "bad_checksum")


class ArchivedEvidenceTest(unittest.TestCase):
    def setUp(self):
        path = REPORT / "logs/metadata.json"
        if not path.exists():
            self.skipTest("archived evidence is absent")
        self.metadata = json.loads(path.read_text())

    def test_the_archive_names_the_pinned_sources(self):
        recorded = self.metadata["pinned_source_sha256"]
        for relative, digest in driver.PINNED.items():
            self.assertEqual(recorded[relative], digest, relative)
        self.assertEqual(self.metadata["fluent_bit_revision"], driver.PIN)

    def test_the_archive_claims_no_certificate(self):
        self.assertFalse(self.metadata["static_certificate"])
        self.assertFalse(self.metadata["durability_proved"])

    def test_the_archived_records_match_the_committed_validator_fixture(self):
        archived = (REPORT / "logs/records.txt").read_bytes()
        fixture = (REPO / "crates/qcore-core/tests/data/o01_records.txt").read_bytes()
        self.assertEqual(archived, fixture)


if __name__ == "__main__":
    unittest.main()
