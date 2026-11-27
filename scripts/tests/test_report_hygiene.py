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

"""Tests for `scripts/check_report_hygiene.py` and two-tier `scripts/build_docs_index.py`."""
import contextlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import build_docs_index  # noqa: E402
import check_report_hygiene as hygiene  # noqa: E402
import land  # noqa: E402


def write_text(root: Path, rel: str, content: str) -> Path:
    target = root / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    return target


class ReportHygieneTests(unittest.TestCase):
    def test_real_repository_passes_hygiene_and_index_checks(self):
        self.assertEqual(hygiene.check_reports(ROOT), [])
        expected = build_docs_index.render_all(ROOT)
        for target, content in expected.items():
            self.assertTrue(target.is_file(), f"missing generated index file: {target}")
            self.assertEqual(target.read_text(encoding="utf-8"), content, f"stale {target}")

    def test_clean_two_tier_tree_and_valid_pruned_manifest_pass(self):
        with tempfile.TemporaryDirectory(prefix="qcore-report-hygiene-") as scratch:
            root = Path(scratch)
            write_text(root, "docs/reports/active-card/README.md", "# Active card\n")
            write_text(
                root,
                "docs/reports/archive/wave-48/archived-card/README.md",
                "# Archived card\n",
            )
            write_text(
                root,
                "docs/reports/archive/wave-48/archived-card/pruned-artifacts.json",
                json.dumps(
                    {
                        "schema": hygiene.PRUNED_SCHEMA,
                        "total_pruned_files": 2,
                        "total_pruned_bytes": 1500,
                        "artifacts": [
                            {"path": "gate-scope.json", "bytes": 500, "sha256": "a" * 64},
                            {"path": "raw/before/diagnostics.json.gz", "bytes": 1000, "sha256": "b" * 64},
                        ],
                    },
                    indent=1,
                )
                + "\n",
            )
            self.assertEqual(hygiene.check_reports(root), [])
            rendered = build_docs_index.render_all(root)
            self.assertIn(root / "docs/reports/INDEX.md", rendered)
            self.assertIn(root / "docs/reports/README.md", rendered)
            self.assertIn(root / "docs/reports/archive/wave-48/README.md", rendered)
            index_body = rendered[root / "docs/reports/INDEX.md"]
            self.assertIn("[active-card](active-card/README.md)", index_body)
            self.assertIn("[archived-card](archive/wave-48/archived-card/README.md)", index_body)

    def test_oversized_files_bulk_dumps_and_failed_attempts_are_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-report-hygiene-") as scratch:
            root = Path(scratch)
            write_text(root, "docs/reports/card-a/README.md", "# Card A\n")
            write_text(root, "docs/reports/card-a/big.json", "x" * (hygiene.MAX_REPORT_FILE_BYTES + 1))
            write_text(root, "docs/reports/card-a/gate-scope.json", "{}\n")
            write_text(root, "docs/reports/card-a/raw/before/summary.json", "{}\n")
            write_text(root, "docs/reports/card-a/attempt-1/notes.txt", "aborted\n")
            write_text(root, "docs/reports/card-a/wrong-clang-attempt.json", "{}\n")
            problems = hygiene.check_reports(root)
            joined = "\n".join(problems)
            self.assertEqual(len(problems), 5, joined)
            self.assertIn("big.json", joined)
            self.assertIn("gate-scope.json", joined)
            self.assertIn("raw/before/summary.json", joined)
            self.assertIn("attempt-1/notes.txt", joined)
            self.assertIn("wrong-clang-attempt.json", joined)

    def test_invalid_pruned_manifest_and_duplicate_card_names_are_refused(self):
        with tempfile.TemporaryDirectory(prefix="qcore-report-hygiene-") as scratch:
            root = Path(scratch)
            write_text(root, "docs/reports/dup-card/README.md", "# Top\n")
            write_text(root, "docs/reports/archive/wave-48/dup-card/README.md", "# Archived\n")
            write_text(
                root,
                "docs/reports/dup-card/pruned-artifacts.json",
                json.dumps(
                    {
                        "schema": hygiene.PRUNED_SCHEMA,
                        "total_pruned_files": 2,
                        "total_pruned_bytes": 999,
                        "artifacts": [
                            {"path": "z.log", "bytes": 10, "sha256": "0" * 64},
                            {"path": "a.log", "bytes": 20, "sha256": "not-a-sha"},
                        ],
                    }
                ),
            )
            problems = hygiene.check_reports(root)
            joined = "\n".join(problems)
            self.assertIn("duplicate report card directory 'dup-card'", joined)
            self.assertIn("strictly sorted", joined)
            self.assertIn("64 lowercase hex digits", joined)
            self.assertIn("total_pruned_bytes", joined)

    def test_land_check_enforces_report_hygiene_when_script_present(self):
        with tempfile.TemporaryDirectory(prefix="qcore-report-land-") as scratch:
            root = Path(scratch)
            (root / "scripts").mkdir()
            shutil.copy(ROOT / "scripts/build_docs_index.py", root / "scripts")
            shutil.copy(ROOT / "scripts/check_report_hygiene.py", root / "scripts")
            write_text(root, "docs/reports/card/README.md", "# Card\n")
            subprocess.run(
                [sys.executable, "-I", str(root / "scripts/build_docs_index.py"), "--root", str(root)],
                check=True,
                capture_output=True,
            )
            write_text(
                root,
                str(land.MATRIX),
                json.dumps(
                    {
                        "dimensions": {"requirements": 1, "named_followups": 1},
                        "requirements": [{}],
                        "named_followups": [{"id": "X"}],
                    }
                ),
            )
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            write_text(
                root,
                str(land.STATUS),
                "# Status\n\n<!-- unpublished-landings: x -->\nold\n<!-- unpublished-landings:end -->\n",
            )
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                self.assertEqual(land.main(["unpublished", "--write", "--root", str(root)]), 0)
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                self.assertEqual(land.main(["check", "--root", str(root)]), 0, err.getvalue())

            write_text(root, "docs/reports/card/gate-scope.json", "{}\n")
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = land.main(["check", "--root", str(root)])
            self.assertEqual(code, 1)
            self.assertIn(land.HYGIENE_COMMAND, err.getvalue())


if __name__ == "__main__":
    unittest.main()
