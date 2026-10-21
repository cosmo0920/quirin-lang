#!/usr/bin/env python3
"""Replay separately held166 reference bindings and reject missing/foreign inputs.

INPUT_DIRECTORY enumerates every source/manifest/artifact path explicitly.
No runtime or source correspondence evidence is created by these checks.
"""
from pathlib import Path
import argparse, copy, datetime, hashlib, json, subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('inputs', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    records = []
    def run(name, data, good=False):
        directory = out / name; directory.mkdir()
        spec = directory / 'input.json'; spec.write_text(json.dumps(data, indent=2) + '\n')
        result_path = directory / 'result.json'
        command = [str(args.binary.resolve()), str(spec), str(result_path)]
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with (directory / 'console.log').open('w') as stream:
            code = subprocess.run(command, cwd=root, stdout=stream, stderr=subprocess.STDOUT).returncode
        records.append({'name': name, 'command': command, 'exit': code,
                        'expected': 'zero' if good else 'nonzero', 'started': start,
                        'ended': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                        'log': str(directory / 'console.log')})
        (out / 'commands.json').write_text(json.dumps(records, indent=2) + '\n')
        assert (code == 0) == good, name
        assert result_path.exists() == good
        if good:
            result = json.loads(result_path.read_text())
            assert result['source_binding'] == 'open'
            assert result['reference_integrity'] is not None
    counts = {}
    for mode in ['resource', 'null', 'model']:
        data = json.loads((args.inputs / f'{mode}-check-input.json').read_text())
        run(f'{mode}-actual', data, True)
        held = json.loads(Path(data['held_graphs']).read_text())
        counts[mode] = sum('rk_plugin_callback_closure' in graph.get('refinements', {}) for graph in held.values())
        for label, key in [('missing-manifests', 'presented_manifest_index'),
                           ('missing-artifacts', 'artifact_index')]:
            empty = out / f'{mode}-{label}.json'; empty.write_text('{}')
            bad = copy.deepcopy(data); bad[key] = str(empty)
            run(f'{mode}-{label}', bad)
        missing = copy.deepcopy(held)
        selected = next(name for name, graph in held.items() if graph.get('refinements', {}))
        del missing[selected]
        wrong = out / f'{mode}-missing-graph.json'; wrong.write_text(json.dumps(missing))
        bad = copy.deepcopy(data); bad['held_graphs'] = str(wrong)
        run(f'{mode}-missing-held-graph', bad)
        manifests = json.loads(Path(data['presented_manifest_index']).read_text())
        digest = next(iter(manifests)); altered = out / f'{mode}-altered-manifest.json'
        altered.write_bytes(Path(manifests[digest]).read_bytes() + b'\n')
        manifests[digest] = str(altered)
        index = out / f'{mode}-altered-index.json'; index.write_text(json.dumps(manifests))
        bad = copy.deepcopy(data); bad['presented_manifest_index'] = str(index)
        run(f'{mode}-noncanonical-presented-bytes', bad)
        foreign = copy.deepcopy(data)
        other = 'null' if mode == 'resource' else 'resource'
        foreign['held_graphs'] = json.loads((args.inputs / f'{other}-check-input.json').read_text())['held_graphs']
        run(f'{mode}-foreign-property-graph', foreign)
        run(f'{mode}-actual-after-negatives', data, True)
    assert counts == {'resource': 94, 'null': 71, 'model': 1}
    summary = {'status': 'reference integrity only; no source binding or proposition truth',
               'source_binding': 'open', 'counts': counts,
               'positive': sum(record['exit'] == 0 for record in records),
               'negative': sum(record['exit'] != 0 for record in records),
               'all_expected_outcomes': True,
               'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest()}
    (out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
