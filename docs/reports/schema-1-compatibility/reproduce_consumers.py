#!/usr/bin/env python3
"""Verify a packaged schema 1.0 consumer and reject its obsolete version range."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.output is None:
        output = Path(tempfile.mkdtemp(prefix='qcore-schema1-consumer-replay-'))
    else:
        output = args.output.resolve()
        output.mkdir(parents=True, exist_ok=False)
    package = REPO / 'target/package/qcore-schemas-1.0.0'
    command = ['cargo', 'package', '-p', 'qcore-schemas', '--allow-dirty', '--offline']
    packaged = subprocess.run(command, cwd=REPO, text=True, capture_output=True)
    (output / 'package.log').write_text(packaged.stdout + packaged.stderr)
    if packaged.returncode:
        raise RuntimeError('schema packaging failed; see package.log')
    results = {}
    for name in ('current', 'obsolete'):
        fixture = HERE / 'consumers' / name
        destination = output / name
        (destination / 'src').mkdir(parents=True)
        original = (fixture / 'Cargo.toml').read_text()
        old_path = tomllib.loads(original)['dependencies']['qcore-schemas']['path']
        (destination / 'Cargo.toml').write_text(original.replace(old_path, str(package)))
        (destination / 'src/main.rs').write_bytes((fixture / 'src/main.rs').read_bytes())
        checked = subprocess.run(
            ['cargo', 'run', '--manifest-path', str(destination / 'Cargo.toml'), '--offline'],
            cwd=output, text=True, capture_output=True,
        )
        log = checked.stdout + checked.stderr
        (output / f'{name}.log').write_text(log)
        if name == 'current':
            assert checked.returncode == 0, log
            assert 'statement input roundtrip and old-shape rejection pass' in log
        else:
            assert checked.returncode != 0, log
            assert 'failed to select a version' in log and '^0.2.0' in log and '1.0.0' in log
        results[name] = {'exit_code': checked.returncode, 'expected_result_observed': True}
    (output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    print(output)
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
