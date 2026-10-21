#!/usr/bin/env python3
"""Official CLI repins for the exact existing selected source identities only.

Additional atoms are proposed metadata. No body fact or envelope truth is added.
Retain every command/exit and refuse any source-site or revision-set change.
"""
import argparse, datetime, json, subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    report = Path(__file__).resolve().parents[1]
    frozen = json.loads((report / 'selected-source-identities.json').read_text())
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    primary = Path('/home/cosmo/Documents/GitHub/qcore-c')
    sources = [primary / 'build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source',
               primary / 'private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source']
    records = []
    groups = {}
    for item in frozen['selected']:
        if not item['original_entries']: continue  # Model-only, no C pin exists.
        crate = item['module'].split('/')[1]
        groups.setdefault((crate, item['property_view']), []).append(item)
    for (crate, mode), items in sorted(groups.items()):
        for source in sources:
            command = [str(args.binary.resolve()), 'pin', '--source-root', str(source), '--crate', crate]
            if mode == 'bounds': command.append('--bounds')
            for item in sorted(items, key=lambda item: item['declaration']):
                command += ['--function', item['declaration']]
            log = out / f'{len(records):02}.log'
            start = datetime.datetime.now(datetime.timezone.utc).isoformat()
            with log.open('w') as stream:
                code = subprocess.run(command, cwd=root, stdout=stream, stderr=subprocess.STDOUT).returncode
            records.append({'command': command, 'exit': code, 'started': start,
                            'ended': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'log': str(log)})
            (out / 'commands.json').write_text(json.dumps(records, indent=2) + '\n')
            assert code == 0, log
    # The official generated format has one entry for each declaration. Compare
    # the complete original reviewed source identities, including every site.
    checked = []
    for item in frozen['selected']:
        if not item['original_entries']: continue
        lock = json.loads((root / item['lock']).read_text())
        entries = lock['pinned']
        if isinstance(entries, dict): entry = entries[item['declaration']]
        else: entry = next(e for e in entries if e['function'] == item['declaration'])
        originals = item['original_entries']; assert len(originals) == 1
        before = originals[0]
        assert sorted(entry['identities'], key=lambda x: json.dumps(x, sort_keys=True)) == sorted(before['identities'], key=lambda x: json.dumps(x, sort_keys=True)), item['declaration']
        assert entry['module'] == before['module']
        checked.append({'property': item['property_view'], 'declaration': item['declaration'],
                        'module': item['module'], 'reviewed_source_identities_exact': True,
                        'before_contract_sha256': before['contract_sha256'],
                        'after_contract_sha256': entry['contract_sha256']})
    (out / 'identities.json').write_text(json.dumps({'selected': checked, 'count': len(checked)}, indent=2) + '\n')
    print(json.dumps({'commands': len(records), 'unchanged_source_identities': len(checked)}))


if __name__ == '__main__':
    main()
