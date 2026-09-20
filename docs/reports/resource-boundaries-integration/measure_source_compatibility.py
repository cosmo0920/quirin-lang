#!/usr/bin/env python3
"""Compare check outcomes and emitted bytes for a fixed tracked source inventory."""
import argparse
import concurrent.futures
import hashlib
import json
import pathlib
import shutil
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--baseline')
    args = parser.parse_args()
    root = pathlib.Path.cwd()
    output = pathlib.Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = output / 'quirin'
    shutil.copyfile(pathlib.Path(args.binary).resolve(), binary)
    binary.chmod(0o755)
    before = json.loads(pathlib.Path(args.baseline).read_text()) if args.baseline else None
    files = list(before['files']) if before else [p for p in subprocess.check_output(
        ['git', 'ls-files', '-z'], text=True).split('\0') if p.endswith(('.qcore', '.qc'))]

    def invoke(mode, name):
        command = [str(binary), mode, name]
        try:
            result = subprocess.run(command, cwd=root, capture_output=True, timeout=30)
            return {'exit': result.returncode, 'stdout_sha256': digest(result.stdout),
                    'stderr_sha256': digest(result.stderr),
                    'stdout': result.stdout.decode(errors='replace') if mode == 'check' else None,
                    'stderr': result.stderr.decode(errors='replace')}
        except subprocess.TimeoutExpired:
            return {'exit': None, 'timeout_seconds': 30}

    def measure(name):
        path = root / name
        if not path.exists():
            return name, {'missing': True}
        row = {'source_sha256': digest(path.read_bytes()), 'check': invoke('check', name)}
        if row['check']['exit'] == 0:
            row['emit'] = invoke('emit', name)
        return name, row

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        rows = dict(pool.map(measure, files))
    data = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
            'binary_sha256': digest(binary.read_bytes()), 'command_template': ['quirin', 'check|emit', 'FILE'],
            'cwd': str(root), 'files': rows}
    (output / 'measurement.json').write_text(json.dumps(data, indent=2) + '\n')
    summary = {'total': len(rows), 'check_passed': sum(r.get('check', {}).get('exit') == 0 for r in rows.values()),
               'emit_passed': sum(r.get('emit', {}).get('exit') == 0 for r in rows.values()),
               'timeouts': sum(r.get('check', {}).get('exit') is None for r in rows.values())}
    if before:
        summary['changed'] = [{'file': name, 'before': before['files'][name], 'after': rows[name]}
                              for name in files if before['files'][name] != rows[name]]
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({k: v for k, v in summary.items() if k != 'changed'}, indent=2))
    if before:
        print('changed', len(summary['changed']))


if __name__ == '__main__':
    main()
