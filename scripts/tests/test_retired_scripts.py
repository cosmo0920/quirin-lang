"""Retired scripts stay retired (cards W44-QUIRIN-AUDIT-PROVIDERS and
W44-QUIRIN-AUDIT-PREPARE).

`scripts/prepare_cycle19_clang_audit.py` was removed when `quirin-audit
current` took over the configured current-revision route.
`scripts/prepare_cmetrics_clang_audit.py` and `scripts/prepare_cycle19_monkey.py`
were removed when `quirin-audit prepare` (steps `clang-build` and `monkey`)
took over preparation with the same commands and output files. No live file
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
