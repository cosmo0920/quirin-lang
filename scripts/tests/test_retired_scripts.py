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

"""Retired scripts stay retired (cards W44-QUIRIN-AUDIT-PROVIDERS,
W44-QUIRIN-AUDIT-PREPARE and W45-AUDIT-SUBCOMMAND-PLUGINS).

`scripts/prepare_cycle19_clang_audit.py` was removed when `quirin-audit
current` took over the configured current-revision route.
`scripts/prepare_cmetrics_clang_audit.py` and `scripts/prepare_cycle19_monkey.py`
were removed when `quirin-audit prepare` (steps `clang-build` and `monkey`)
took over preparation with the same commands and output files.
`scripts/prepare_libevent_core_audit.py` was removed when the external
subcommand `quirin-audit libevent prepare` (crate
`quirin-audit-plugin-libevent`) reproduced its outputs byte for byte. No live file
may point a reader at a removed script. Frozen evidence keeps its text:
reports under `docs/reports/` and the increment history under
`docs/development/increments/` record what was true when they were written.
"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tracked_files import tracked_relative  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]

RETIRED = [
    'prepare_cycle19_clang_audit.py',
    'prepare_cmetrics_clang_audit.py',
    'prepare_cycle19_monkey.py',
    'prepare_libevent_core_audit.py',
]

# Frozen history and the external submodule. Generated and private trees
# (`target/`, `private/`, `measure/`, `build/`, `proof/.lake/`, other
# worktrees) are untracked or ignored, so the tracked listing never reads them
# (card W45-TEST-HYGIENE).
EXCLUDED_PREFIXES = (
    'docs/reports/',
    'docs/development/increments/',
    'third_party/',
)

TEXT_SUFFIXES = {
    '.rs', '.py', '.md', '.json', '.toml', '.lean', '.sh', '.qcore', '.txt', '.yml', '.yaml',
}


def live_files():
    for relative in tracked_relative(ROOT):
        if relative.startswith(EXCLUDED_PREFIXES):
            continue
        if Path(relative).suffix in TEXT_SUFFIXES:
            yield relative


class RetiredScriptsTest(unittest.TestCase):
    def test_retired_scripts_are_absent(self):
        for name in RETIRED:
            self.assertFalse((ROOT / 'scripts' / name).exists(), name)

    def test_cycle19_pinned_example_stays_retired(self):
        for relative in (
            'compiler/examples/cycle19_fluent_bit_libraries_audit.rs',
            'compiler/examples/support/cycle19_compatibility.rs',
        ):
            self.assertFalse((ROOT / relative).exists(), relative)

    def test_no_live_file_references_a_retired_script(self):
        offenders = []
        this_file = Path(__file__).resolve().relative_to(ROOT).as_posix()
        for relative in live_files():
            if relative == this_file:
                continue
            try:
                text = (ROOT / relative).read_text(encoding='utf-8')
            except (UnicodeDecodeError, OSError):
                continue
            for name in RETIRED:
                if name in text:
                    offenders.append(f'{relative}: {name}')
        self.assertEqual(offenders, [])


if __name__ == '__main__':
    unittest.main()
