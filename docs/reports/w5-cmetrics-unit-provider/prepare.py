#!/usr/bin/env python3
"""Create a persistent, separately declared analysis build for the pinned counter.

This does not claim equivalence to the CMake command: its diagnostic/debug/PIC
and assembler options are omitted and recorded. The gateway's strict option
policy remains in force. Unknown CMake options stop preparation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--checkout', type=Path, default=ROOT.parent / 'fluent-bit')
    args = parser.parse_args()
    checkout = args.checkout.resolve()
    database = checkout / 'build/compile_commands.json'
    source = checkout / 'lib/cmetrics/src/cmt_counter.c'
    entries = json.loads(database.read_text())
    matches = [entry for entry in entries if Path(entry['file']).resolve() == source]
    if len(matches) != 1:
        raise SystemExit('expected one cmt_counter.c compile entry')
    entry = matches[0]
    words = entry.get('arguments') or shlex.split(entry['command'])
    kept, omitted = [], []
    cursor = 1
    while cursor < len(words):
        word = words[cursor]
        if word in ['-D', '-I', '-U', '-isystem', '-iquote']:
            kept.extend(words[cursor:cursor+2])
            cursor += 2
            continue
        if word.startswith(('-D', '-I', '-U', '-std=')):
            kept.append(word)
        elif word in ['-Wall', '-g', '-fPIC', '-Wa,--noexecstack']:
            omitted.append(word)
        elif word == '-o':
            cursor += 2
            continue
        elif word == '-c' or Path(word).resolve() == source:
            pass
        else:
            raise SystemExit(f'unreviewed source-build option: {word}')
        cursor += 1
    if not any(word.startswith('-std=') for word in kept):
        kept.append('-std=gnu11')
    output = ROOT / 'build/cycle19/cmetrics-counter'
    output.mkdir(parents=True, exist_ok=True)
    selected = {'directory': entry['directory'], 'file': str(source),
                'arguments': ['clang', *kept, '-c', str(source)]}
    (output / 'compile_commands.json').write_text(json.dumps([selected], indent=2) + '\n')
    record = {'source_compile_database': str(database),
              'database_sha256': hashlib.sha256(database.read_bytes()).hexdigest(),
              'original_entry': entry, 'analysis_entry': selected,
              'omitted_options': omitted,
              'claim': 'Separate declared Clang analysis build; no CMake build equivalence asserted.'}
    (output / 'build-provenance.json').write_text(json.dumps(record, indent=2) + '\n')
    config = json.loads((ROOT / 'contract/profiles/cmetrics-counter/audit.json').read_text())
    config.update(profile_directory=str(ROOT / 'contract/profiles/cmetrics-counter'),
                  checkout=str(checkout), compile_database=str(output / 'compile_commands.json'),
                  report=str(output / 'audit.json'))
    config['source_contracts']['configuration'] = str(ROOT / 'contract/profiles/cmetrics-counter/source-contracts.json')
    config['source_contracts']['source_rules'] = str(ROOT / 'contract/qcore/library/source_projection.qcore')
    (output / 'audit.json.config').write_text(json.dumps(config, indent=2) + '\n')
    print(output / 'audit.json.config')


if __name__ == '__main__':
    main()
