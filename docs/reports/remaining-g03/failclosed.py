#!/usr/bin/env python3
"""Execute the G03 fail-closed mutations against the real `chunkio-core` inputs.

Each mutation copies the checked-in profile directory into a scratch tree,
changes exactly one declared input, and runs `qcore-audit` on it. A mutation
is accepted only if the audit rejects and writes no report. Run from the
repository root after `scripts/prepare_cmetrics_clang_audit.py`.
"""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
PROFILE = ROOT / 'contract/profiles/chunkio-core'
BUILD = ROOT / 'build/cycle19/fluent-bit-clang-production'


def prepared(work, mutate_profile=None, mutate_audit=None, mutate_database=None):
    # Reproduce the checked-in layout so the profile's relative contract
    # manifest resolves exactly as it does in the repository.
    contract = work / 'contract'
    (contract / 'profiles').mkdir(parents=True)
    shutil.copy(ROOT / 'contract/cycle19-resource-contracts.json', contract)
    (contract / 'qcore').symlink_to(ROOT / 'contract/qcore', target_is_directory=True)
    directory = contract / 'profiles/chunkio-core'
    shutil.copytree(PROFILE, directory)
    profile = json.loads((directory / 'profile.json').read_text())
    audit = json.loads((directory / 'audit.json').read_text())
    audit['checkout'] = str(ROOT / 'build/cycle19/fluent-bit-clang-source')
    audit['compile_database'] = str(BUILD / 'compile_commands.json')
    audit['report'] = str(work / 'report.json')
    if mutate_database:
        entries = json.loads((BUILD / 'compile_commands.json').read_text())
        mutate_database(entries)
        database = work / 'compile_commands.json'
        database.write_text(json.dumps(entries))
        audit['compile_database'] = str(database)
    if mutate_profile:
        mutate_profile(profile)
    if mutate_audit:
        mutate_audit(directory, audit)
    (directory / 'profile.json').write_text(json.dumps(profile, indent=2) + '\n')
    (directory / 'audit.json').write_text(json.dumps(audit, indent=2) + '\n')
    return directory / 'audit.json'


def changed_pin(profile):
    profile['pins']['fluent-bit']['git_commit'] = '0' * 39 + '1'


def changed_bytes(profile):
    audited = profile['libraries']['chunkio']['audited_files']
    audited[0]['sha256'] = 'f' * 64


def unsupported_flag(entries):
    for entry in entries:
        if entry['file'].endswith('lib/chunkio/src/cio_error.c'):
            words = entry['command'].split(' ')
            entry['command'] = ' '.join(words[:1] + ['-Wextra'] + words[1:])


def absent_package(directory, audit):
    (directory / 'source-contracts.json').write_text(json.dumps({
        'schema_version': 1,
        'packages': [str(ROOT / 'build/cycle19/provider-packages-unit/qcore.absent')],
        'bindings': [{'library': 'chunkio', 'provider': 'qcore.absent',
                      'contract': str(ROOT / 'contract/qcore/chunkio/lock_state.qcore')}],
    }, indent=2) + '\n')
    audit['source_contracts'] = {
        'configuration': 'source-contracts.json',
        'source_rules': str(ROOT / 'contract/qcore/library/source_projection.qcore'),
        'libraries': [{'library': 'chunkio', 'source_root': 'lib/chunkio/src'}],
    }


CASES = {
    'control_unmutated': {},
    'changed_source_pin': dict(mutate_profile=changed_pin),
    'changed_audited_bytes': dict(mutate_profile=changed_bytes),
    'unsupported_compiler_flag': dict(mutate_database=unsupported_flag),
    'absent_provider_package': dict(mutate_audit=absent_package),
}


def main():
    failures = 0
    for name, arguments in CASES.items():
        with tempfile.TemporaryDirectory(prefix=f'g03-{name}-') as scratch:
            work = Path(scratch)
            config = prepared(work, **arguments)
            result = subprocess.run(
                ['cargo', 'run', '--quiet', '-p', 'qcore-audit', '--', str(config)],
                cwd=ROOT, text=True, capture_output=True)
            rejected = result.returncode != 0 and not (work / 'report.json').exists()
            message = (result.stderr.strip() or result.stdout.strip()).splitlines()
            control = name == 'control_unmutated'
            correct = (not rejected) if control else rejected
            verdict = 'ACCEPTED' if not rejected else 'REJECTED'
            print(f'{name}: {verdict}{"" if correct else "  <-- DEFECT"}')
            print(f'  {message[-1] if message else "<no output>"}')
            failures += 0 if correct else 1
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
