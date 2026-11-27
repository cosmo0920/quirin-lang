#!/usr/bin/env python3
"""Check the relocated comparison's exact production tokens.

Only API names and visibility change; comments are excluded, string literals
(including diagnostics) remain exact tokens. The reviewed wrapper is separately
checked by compile-fail tests and serialized Cycle19 artifact parity.
"""
from pathlib import Path
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = '1ff50b7'
OLD = 'compiler/src/typed_source_match.rs'
NEW = 'crates/qcore-c-gateway/src/typed_source_match.rs'
TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_]\w*|\d[\w.]*|[^\s]', re.S)

def tokens(source):
    return [t for t in TOKEN.findall(source) if not t.startswith(('//', '/*'))]

def main():
    before = subprocess.check_output(['git', 'show', f'{BASE}:{OLD}'], cwd=ROOT)
    after = (ROOT / NEW).read_bytes()
    normalized = after.decode().replace('TypedBodyComparison', 'SourceMatch')
    normalized = normalized.replace('pub type SourceFieldProfile', 'pub(crate) type SourceFieldProfile')
    normalized = normalized.replace('pub fn compare_typed_body', 'pub(crate) fn check_source_match')
    old_tokens, new_tokens = tokens(before.decode()), tokens(normalized)
    assert old_tokens == new_tokens, 'production body changed beyond documented API renames'
    result = {
        'baseline_commit': BASE, 'old_path': OLD, 'new_path': NEW,
        'old_sha256': hashlib.sha256(before).hexdigest(),
        'new_sha256': hashlib.sha256(after).hexdigest(),
        'production_tokens_equal_after_api_renames': True,
        'normalized_token_count': len(new_tokens),
        'normalized_tokens_sha256': hashlib.sha256(json.dumps(new_tokens).encode()).hexdigest(),
        'rewrites': ['SourceMatch → TypedBodyComparison', 'check_source_match → compare_typed_body',
                     'comparison function and field-profile type visibility: crate-private → public'],
    }
    (Path(__file__).parent / 'relocation-parity.json').write_text(json.dumps(result, indent=2)+'\n')
    print(f'Validated {len(new_tokens)} comparison tokens after API renames')

if __name__ == '__main__':
    main()
