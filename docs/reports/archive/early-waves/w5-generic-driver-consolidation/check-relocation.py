#!/usr/bin/env python3
"""Check production token parity with the pre-relocation compiler.

Comments and test modules are excluded; strings retain their exact contents.
Imports are sorted because rustfmt reorders them after module-owner changes.
Only the explicitly listed module paths and default-registry split normalize.
This is reproducible source comparison, not a Rust semantic equivalence proof.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = '7cb45a0'
TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|[A-Za-z_]\w*|\d[\w.]*|[^\s]', re.S)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def production(source):
    return source.split('\n#[cfg(test)]', 1)[0]


def tokens(source):
    imports = []

    def collect(found):
        imports.append([t for t in TOKEN.findall(found.group())
                        if not t.startswith(('//', '/*'))])
        return ''

    source = re.sub(r'^use\s[^;]+;', collect, source, flags=re.M)
    return [t for item in sorted(imports) for t in item] + [
        t for t in TOKEN.findall(source) if not t.startswith(('//', '/*'))]


def between(source, start, end=None):
    source = source[source.index(start):]
    return source if end is None else source[:source.index(end)]


def main():
    rows = []

    def check(name, old_path, new_path, old_cut=lambda s: s,
              new_cut=lambda s: s, rewrites=()):
        before = subprocess.check_output(['git', 'show', f'{BASE}:{old_path}'], cwd=ROOT)
        after = (ROOT / new_path).read_bytes()
        old = old_cut(production(before.decode()))
        new = new_cut(production(after.decode()))
        for current, previous in rewrites:
            new = new.replace(current, previous)
        old_tokens, new_tokens = tokens(old), tokens(new)
        if old_tokens != new_tokens:
            for index, (left, right) in enumerate(zip(old_tokens, new_tokens)):
                if left != right:
                    raise AssertionError(f'{name}: token {index}: '
                                         f'{old_tokens[max(0,index-8):index+12]} != '
                                         f'{new_tokens[max(0,index-8):index+12]}')
            raise AssertionError(f'{name}: lengths {len(old_tokens)} != {len(new_tokens)}')
        rows.append({
            'component': name, 'old_path': old_path, 'new_path': new_path,
            'old_sha256': digest(before), 'new_sha256': digest(after),
            'normalized_token_count': len(new_tokens),
            'normalized_tokens_sha256': digest(json.dumps(new_tokens).encode()),
            'equal_after_documented_rewrites': True,
            'reverse_module_rewrites': list(rewrites),
        })

    external = 'compiler/src/external_integration.rs'
    mechanism = 'pub use qcore_schemas::mechanism::LibraryMechanismSummary;'
    monkey = '#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum MonkeyCallback'
    custody = 'pub type ExternalResourceId'
    pin = 'pub use crate::verifier_plugins::chunkio::PINNED_FLUENT_BIT_REVISION;'
    check('pure custody model', external, 'crates/qcore-core/src/library_custody.rs',
          lambda s: between(s, custody, pin), lambda s: between(s, custody))
    check('declared library and lifecycle imports', external,
          'crates/qcore-verification/src/library_import.rs',
          lambda s: between(s, mechanism, monkey), lambda s: between(s, mechanism),
          [('qcore_core::chunk_lifecycle::ChunkResidency', 'crate::chunk_lifecycle::ChunkResidency')])
    check('retained Monkey specialization', external, external,
          lambda s: between(s, monkey), lambda s: between(s, monkey))
    check('source bundle gates', 'compiler/src/external_source.rs',
          'crates/qcore-verification/src/external_source.rs', rewrites=[
              ('crate::library_import::', 'crate::external_integration::')])
    check('research aggregate', 'compiler/src/research_extensions.rs',
          'crates/qcore-verification/src/research_extensions.rs', rewrites=[
              ('qcore_c_gateway::legacy_c::', 'crate::legacy_c::'),
              ('qcore_core::inference::', 'crate::inference::'),
              ('qcore_core::native_concurrency::', 'crate::native_concurrency::'),
              ('qcore_core::weak_memory::', 'crate::weak_memory::')])

    default = '/// Parses one or more source files'
    explicit = '/// Compiles with an explicit verifier registry.'

    def remove_default(source):
        return source[:source.index(default)] + source[source.index(explicit):]

    driver_rewrites = [
        (f'qcore_core::{name}::', f'crate::{name}::')
        for name in ['cleanup_lowering', 'ownership', 'resource_flow', 'typed_errors', 'usage']
    ] + [
        (f'qcore_frontend::{name}::', f'crate::{name}::')
        for name in ['frontend', 'lexer', 'whole_program']
    ] + [
        ('qcore_core::interpret_cfg', 'crate::interpret_cfg'),
        ('qcore_core::ExecutionOutcome', 'crate::ExecutionOutcome'),
        ('crate::frontend::SourceTypeKind', 'crate::SourceTypeKind'),
        ('crate::frontend::SymbolId', 'crate::SourceSymbolId'),
        ('crate::frontend::source_diagnostic', 'crate::source_diagnostic'),
        ('use qcore_core::{ExecutionObservation, emit_c11};',
         'use crate::{ExecutionObservation, VerifierRegistry, emit_c11, first_party_registry};'),
        ('use qcore_verifier_host::VerifierRegistry;', ''),
    ]
    check('explicit-registry source driver', 'compiler/src/driver.rs',
          'crates/qcore-verification/src/driver.rs', old_cut=remove_default,
          rewrites=driver_rewrites)
    check('retained default registry wrapper', 'compiler/src/driver.rs', 'compiler/src/driver.rs',
          lambda s: between(s, default, explicit), lambda s: between(s, default))
    result = {'base_commit': BASE, 'components': rows}
    (Path(__file__).parent / 'relocation-parity.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f'Validated production token parity for {len(rows)} components')


if __name__ == '__main__':
    main()
