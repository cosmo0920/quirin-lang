#!/usr/bin/env python3
"""Verify the shared structural rules are the previous manifest-loader rules.

Only the two original audited-file/witness loop bodies are compared. Rust
comments and formatting are ignored; string contents are preserved. The public
derivation now calls those rules, intentionally changing invalid-profile behavior.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = 'ad92245'
PATH = 'crates/qcore-contract/src/profile_loader.rs'
TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_]\w*|\d[\w.]*|[^\s]', re.S)


def tokens(source):
    return [item for item in TOKEN.findall(source) if not item.startswith(('//', '/*'))]


def main():
    before = subprocess.check_output(['git', 'show', f'{BASE}:{PATH}'], cwd=ROOT)
    after = (ROOT / PATH).read_bytes()
    old = before.decode()
    a = old.index('        let mut audited = BTreeSet::new();', old.index('fn validate_manifest'))
    b = old.index('        for root in &library.source_roots {', a)
    c = old.index('        let mut witnesses = BTreeSet::new();', b)
    d = old.index('        for (kind, summary) in &library.operation_summaries {', c)
    new = after.decode()
    e = new.index('    let mut audited = BTreeSet::new();', new.index('pub fn validate_library_source_profile'))
    f = new.index('    Ok(())', e)
    old_tokens, new_tokens = tokens(old[a:b] + old[c:d]), tokens(new[e:f])
    assert old_tokens == new_tokens, 'shared source-witness rules differ from the original loader'
    result = {
        'base_commit': BASE, 'path': PATH,
        'old_sha256': hashlib.sha256(before).hexdigest(),
        'new_sha256': hashlib.sha256(after).hexdigest(),
        'unchanged_rule_tokens': len(new_tokens),
        'rule_tokens_sha256': hashlib.sha256(json.dumps(new_tokens).encode()).hexdigest(),
        'rules_equal': True,
        'intentional_behavior_change': 'Direct source-summary derivation now invokes these rules before checking source bytes. Invalid JSON manifests remain rejected; if both witness and source-root rules fail, witness validation now reports first.',
    }
    (Path(__file__).parent / 'rule-parity.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f'Validated {len(new_tokens)} unchanged structural rule tokens')


if __name__ == '__main__':
    main()
