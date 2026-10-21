#!/usr/bin/env python3
"""Archive exact explicit runtime inputs and copy a provenance-pinned analyzer.

The caller supplies the exact reviewed binary source receipt; this script checks
its byte hash and records it without inventing build/source equivalence. Final
BASE source and receipt are supplied by the integrator's merged-base gate.
"""
import argparse, datetime, hashlib, json, shutil, subprocess, tarfile
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--source', required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--binary-sha256', required=True)
    parser.add_argument('--binary-receipt', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    source = subprocess.check_output(['git', 'rev-parse', args.source + '^{commit}'], cwd=root, text=True).strip()
    assert sha(args.binary) == args.binary_sha256
    receipt = json.loads(args.binary_receipt.read_bytes())
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    archive = out / 'runtime-inputs.tar'
    command = ['git', 'archive', source, 'contract', 'providers', 'scripts/run_pinned_audit.py', 'cycle19-resource-contracts.json']
    with archive.open('wb') as stream:
        result = subprocess.run(command, cwd=root, stdout=stream, check=False)
    assert result.returncode == 0
    snapshot = out / 'inputs'; snapshot.mkdir()
    with tarfile.open(archive) as stream: stream.extractall(snapshot, filter='data')
    (snapshot / 'target/release').mkdir(parents=True)
    binary = snapshot / 'target/release/quirin-audit'; shutil.copy2(args.binary, binary)
    assert sha(binary) == args.binary_sha256
    files = {str(path.relative_to(snapshot)): sha(path)
             for path in sorted(snapshot.rglob('*')) if path.is_file()}
    for path in snapshot.rglob('*'):
        if path.is_file(): path.chmod(0o555 if path == binary else 0o444)
    record = {'source': source, 'archive_command': command, 'archive_exit': result.returncode,
              'archive_sha256': sha(archive), 'binary_sha256': sha(binary),
              'supplied_binary': str(args.binary.resolve()),
              'supplied_binary_receipt_sha256': sha(args.binary_receipt),
              'supplied_binary_receipt': receipt, 'inputs': files,
              'frozen_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'meaning': 'Exact explicit runtime-input archive and supplied binary identity only; source/build provenance is the independently supplied receipt.'}
    (out / 'receipt.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'source': source, 'snapshot': str(snapshot), 'files': len(files), 'binary_sha256': sha(binary)}))


if __name__ == '__main__': main()
