#!/usr/bin/env python3
"""Install exact canonical manifests from explicit property replay outputs.

The Rust producer validates actual declarations and serializes typed manifests.
This installer never reads an implicit manifest directory or mints evidence.
"""
from pathlib import Path
import argparse, hashlib, json


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('replay_directory', type=Path)
    parser.add_argument('report_directory', type=Path)
    parser.add_argument('repository', type=Path)
    args = parser.parse_args()
    census = json.loads((args.report_directory / 'census.json').read_text())
    rows = census['function_resource_premise_sites']
    output = args.report_directory / 'bindings'
    output.mkdir(exist_ok=True)
    index = []
    modules = {}
    for mode, expected_count in [('resource', 94), ('null', 71), ('model', 1)]:
        data = json.loads((args.replay_directory / f'{mode}.json').read_text())
        assert data['mode'] == mode and data['source_binding'] == 'open'
        assert len(data['declaration_manifest_sha256']) == expected_count
        (output / f'{mode}-expected-graphs.json').write_text(json.dumps(data['graphs'], indent=2) + '\n')
        for declaration, digest in sorted(data['declaration_manifest_sha256'].items()):
            raw = data['canonical_manifests'][digest].encode()
            assert sha(raw) == digest
            manifest = json.loads(raw)
            assert manifest['declaration'] == declaration
            assert manifest['source_binding'] == 'open'
            assert len(manifest['atoms']) == 7
            assert all(atom['prerequisites'] == [manifest['environment']['id']]
                       for atom in manifest['atoms'].values())
            matches = [row for row in rows if row['declaration'] == declaration and
                       ('null' if row['path'].endswith('null_deref.qv') else
                        'model' if row['path'].endswith('queue_destructor.qv') else 'resource') == mode]
            assert len(matches) == 1
            path = matches[0]['path']
            (output / f'{digest}.json').write_bytes(raw)
            marker = f'// qcore-premise-binding-manifest-v1 {declaration} rk_plugin_callback_closure {digest}\n'
            modules.setdefault(path, []).append(marker)
            index.append({'property_view': mode, 'declaration': declaration,
                          'source_path': path, 'original_source_sha256': matches[0]['sha256'],
                          'manifest_path': f'bindings/{digest}.json', 'manifest_sha256': digest,
                          'graph_sha256': manifest['graph_sha256'],
                          'model_only_no_c_pin': mode == 'model', 'source_binding': 'open'})
    assert len(index) == 166
    receipts = []
    for path, markers in sorted(modules.items()):
        destination = args.repository / path
        before = destination.read_bytes()
        assert b'// qcore-premise-binding-manifest-v1 ' not in before
        after = ''.join(sorted(markers)).encode() + before
        destination.write_bytes(after)
        assert after[len(after)-len(before):] == before
        # Remove only this card's exact atom/attachment/marker lines.
        reconstructed = b''.join(line for line in after.splitlines(keepends=True)
                                if not (line.lstrip().startswith(b'requires resource_hypothesis(plugin_') or
                                        line.lstrip().startswith(b'requires resource_premise_decomposition(rk_plugin_callback_closure,') or
                                        line.startswith(b'// qcore-premise-binding-manifest-v1 ')))
        expected = {row['sha256'] for row in rows if row['path'] == path}
        assert expected == {sha(reconstructed)}, path
        receipts.append({'path': path, 'selected_declarations': len(markers),
                         'raw_sha256': sha(after), 'original_reconstructed_sha256': sha(reconstructed),
                         'original_bytes_reconstructed': True})
    (output / 'index.json').write_text(json.dumps({'source_binding': 'open',
        'property_counts': {'resource': 94, 'null': 71, 'model': 1},
        'occurrences': index, 'modules': receipts}, indent=2) + '\n')
    print(json.dumps({'occurrences': len(index), 'modules': len(modules), 'source_binding': 'open'}))


if __name__ == '__main__':
    main()
