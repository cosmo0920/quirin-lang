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

import os
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

RETIRED = [
    'prepare_cycle19_clang_audit.py',
    'prepare_cmetrics_clang_audit.py',
    'prepare_cycle19_monkey.py',
    'prepare_libevent_core_audit.py',
]

# Frozen history and generated or external trees.
EXCLUDED_PREFIXES = (
    'docs/reports/',
    'docs/development/increments/',
    'target/',
    'third_party/',
    'private/',
    'measure/',
    'build/',
    'proof/.lake/',
    'providers/target/',
    '.git/',
    '.claude/',
)

TEXT_SUFFIXES = {
    '.rs', '.py', '.md', '.json', '.toml', '.lean', '.sh', '.qcore', '.txt', '.yml', '.yaml',
}


def live_files():
    for directory, subdirectories, files in os.walk(ROOT):
        relative_directory = Path(directory).relative_to(ROOT).as_posix()
        prefix = '' if relative_directory == '.' else relative_directory + '/'
        subdirectories[:] = [
            name for name in subdirectories
            if not (prefix + name + '/').startswith(EXCLUDED_PREFIXES)
        ]
        for name in files:
            relative = prefix + name
            if relative.startswith(EXCLUDED_PREFIXES):
                continue
            if Path(name).suffix in TEXT_SUFFIXES:
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
