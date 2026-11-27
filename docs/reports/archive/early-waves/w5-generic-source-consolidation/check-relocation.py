#!/usr/bin/env python3
"""Compare production Rust tokens after only the documented relocation rewrites.

The default-rule constant/helper changes module owner; its exact policy bytes
are separately hashed. Test modules and Rust comments are outside this comparison.
String literals are retained as single exact tokens, including their whitespace.
Top-level use declarations are compared as a sorted inventory because rustfmt
reorders their module prefixes after relocation.
This is source parity evidence, not a theorem of Rust semantics.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = '128d355'
TARGETS = {
    'typed_source': 'qcore-c-gateway',
    'source_call_use': 'qcore-c-gateway',
    'cleanup_outcome': 'qcore-verification',
    'counter_contract': 'qcore-verification',
    'field_destructor': 'qcore-verification',
    'list_unlink_contract': 'qcore-verification',
}
TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_]\w*|\d[\w.]*|[^\s]', re.S)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def tokens(source):
    source = source.split('\n#[cfg(test)]', 1)[0]
    imports = []
    def collect(found):
        imports.append([t for t in TOKEN.findall(found.group()) if not t.startswith(('//', '/*'))])
        return ''
    source = re.sub(r'^use\s[^;]+;', collect, source, flags=re.M)
    return [token for declaration in sorted(imports) for token in declaration] + [
        t for t in TOKEN.findall(source) if not t.startswith(('//', '/*'))]


def main():
    rows = []
    for name, crate in TARGETS.items():
        old_path = f'compiler/src/{name}.rs'
        new_path = f'crates/{crate}/src/{name}.rs'
        old = subprocess.check_output(['git', 'show', f'{BASE}:{old_path}'], cwd=ROOT)
        new = (ROOT / new_path).read_bytes()
        source = new.decode()
        if name == 'typed_source':
            start = source.index('pub const DEFAULT_SOURCE_RULES:')
            end = source.index('const MAX_PATHS:')
            expected_helper = '''pub const DEFAULT_SOURCE_RULES: &str =
                include_str!("../../../contract/qcore/library/source_projection.qcore");
                fn check_source_rule(name: &str, facts: &[(&str, i128)]) -> Result<(), String> {
                    qcore_contract::contract_observation::check_contract_facts(DEFAULT_SOURCE_RULES, name, facts)
                }'''
            assert tokens(source[start:end]) == tokens(expected_helper)
            source = source[:start] + source[end:]
            source = source.replace('check_source_rule(', 'crate::source_contract::check(')
        for origin, destination in [
            ('qcore_c_gateway::typed_source::', 'crate::typed_source::'),
            ('qcore_frontend::frontend::', 'crate::frontend::'),
            ('qcore_frontend::lexer::', 'crate::lexer::'),
            ('qcore_contract::contract_observation::', 'crate::contract_observation::'),
            ('qcore_core::usage::', 'crate::usage::'),
        ]:
            source = source.replace(origin, destination)
        before, after = tokens(old.decode()), tokens(source)
        assert before == after, f'production token mismatch: {name}'
        rows.append({
            'old_path': old_path, 'new_path': new_path,
            'old_sha256': digest(old), 'new_sha256': digest(new),
            'normalized_token_count': len(after),
            'normalized_tokens_sha256': digest(json.dumps(after, ensure_ascii=False).encode()),
            'production_tokens_equal_after_module_rewrites': True,
        })
    policy = 'contract/qcore/library/source_projection.qcore'
    old_policy = subprocess.check_output(['git', 'show', f'{BASE}:{policy}'], cwd=ROOT)
    current_policy = (ROOT / policy).read_bytes()
    assert old_policy == current_policy
    result = {'base_commit': BASE, 'modules': rows, 'unchanged_default_policy': {
        'path': policy, 'sha256': digest(current_policy)}}
    (Path(__file__).parent / 'relocation-parity.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f'Validated production token parity for {len(rows)} modules and unchanged policy bytes')


if __name__ == '__main__':
    main()
