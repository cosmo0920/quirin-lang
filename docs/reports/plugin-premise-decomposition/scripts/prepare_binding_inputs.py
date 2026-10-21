#!/usr/bin/env python3
"""Write explicit independent input indexes for the three frozen source views.

This enumerates inputs, not evidence. The Rust replay helper separately checks
frozen graphs/manifests against actual sources and explicit artifact bytes.
"""
import argparse, json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[4]
    report = Path(__file__).resolve().parents[1]
    output = args.output.resolve(); output.mkdir(parents=True, exist_ok=False)
    rows = json.loads((report / 'census.json').read_text())['function_resource_premise_sites']
    bindings = json.loads((report / 'bindings/index.json').read_text())['occurrences']
    artifacts = {name: str(root / name) for name in
                 ['proof/QCore/PluginPremiseDecomposition.lean', 'proof/AuditPluginPremiseDecomposition.lean']}
    (output / 'artifacts.json').write_text(json.dumps(artifacts, indent=2) + '\n')
    def mode(path):
        return 'null' if path.endswith('null_deref.qv') else 'model' if path.endswith('queue_destructor.qv') else 'resource'
    for property_view in ['resource', 'null', 'model']:
        paths = sorted({row['path'] for row in rows if mode(row['path']) == property_view})
        data = {'mode': property_view,
                'sources': {path: str(root / path) for path in paths if not path.endswith('/memory.qv')},
                'design': str(report / 'formal-catalog-design.json'),
                'model': artifacts['proof/QCore/PluginPremiseDecomposition.lean'],
                'audit': artifacts['proof/AuditPluginPremiseDecomposition.lean'],
                'check': True, 'held_graphs': str(report / f'bindings/{property_view}-expected-graphs.json'),
                'artifact_index': str(output / 'artifacts.json')}
        if property_view == 'resource':
            data.update(load_path=str(root / 'providers/contract-load-path.json'),
                        elaborated=[{'crate': 'qcore-provider-librdkafka', 'module': 'librdkafka/memory.qv'}])
        manifest_paths = {item['manifest_sha256']: str(report / item['manifest_path'])
                          for item in bindings if item['property_view'] == property_view}
        for kind in ['held', 'presented']:
            index = output / f'{property_view}-{kind}-index.json'
            index.write_text(json.dumps(manifest_paths, indent=2) + '\n')
            data[f'{kind}_manifest_index'] = str(index)
        (output / f'{property_view}-check-input.json').write_text(json.dumps(data, indent=2) + '\n')
    print(json.dumps({'explicit_source_views': 3, 'selected_occurrences': len(bindings), 'source_binding': 'open'}))


if __name__ == '__main__':
    main()
