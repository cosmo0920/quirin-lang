#!/usr/bin/env python3
"""Reverse only this card's checked graph metadata, preserving every old field.

This is attribution, not generic graph erasure. Unknown atoms/refinements,
changed legacy nodes, and all Context coordinates remain visible to comparison.
Rust separately validates actual retained-source replay and published digests.
"""
import copy, hashlib, json
from pathlib import Path
REPORT = Path(__file__).resolve().parents[1]
PREMISE = 'rk_plugin_callback_closure'
DESIGN = json.loads((REPORT / 'formal-catalog-design.json').read_text())
ATOMS = {item['id']: item['statement'] for item in DESIGN['callback_atoms']}
INDEX = json.loads((REPORT / 'bindings/index.json').read_text())['occurrences']
MANIFESTS = {item['manifest_sha256']: item for item in INDEX}
SELECTED = {item['declaration'] for item in INDEX if item['property_view'] == 'resource'}


def canonical_graph(graph):
    assert set(graph) <= {'version', 'atoms', 'premises', 'refinements'}
    result = {'version': graph['version'], 'atoms': {}, 'premises': {}}
    for key, atom in sorted(graph['atoms'].items()):
        assert set(atom) == {'id', 'proposition'} and atom['id'] == key
        proposition = atom['proposition']
        assert set(proposition) == {'kind', 'statement'} and proposition['kind'] == 'opaque'
        result['atoms'][key] = {'id': key, 'proposition': {'kind': 'opaque', 'statement': proposition['statement']}}
    for key, premise in sorted(graph['premises'].items()):
        assert set(premise) == {'id', 'children', 'erasure'} and premise['id'] == key
        erasure = premise['erasure']; assert set(erasure) == {'name', 'statement', 'origin'}
        result['premises'][key] = {'id': key, 'children': premise['children'],
            'erasure': {name: erasure[name] for name in ['name', 'statement', 'origin']}}
    if graph.get('refinements'):
        result['refinements'] = {}
        for key, refinement in sorted(graph['refinements'].items()):
            assert set(refinement) == {'premise', 'children'}
            result['refinements'][key] = {name: refinement[name] for name in ['premise', 'children']}
    return json.dumps(result, separators=(',', ':'), ensure_ascii=False).encode()


def digest(graph):
    return hashlib.sha256(canonical_graph(graph)).hexdigest()


def reverse_graph(graph):
    if PREMISE not in graph.get('refinements', {}):
        return copy.deepcopy(graph)
    assert graph['version'] == 2
    attachment = graph['refinements'][PREMISE]
    assert attachment['children'] == sorted(ATOMS)
    target = graph['premises'][attachment['premise']]
    assert target['erasure']['name'] == PREMISE and target['erasure']['origin'] == 'declared'
    assert len(target['children']) == 1
    legacy_id = target['children'][0]
    assert legacy_id.startswith('legacy_atom_')
    assert graph['atoms'][legacy_id] == {'id': legacy_id, 'proposition': {'kind': 'opaque', 'statement': target['erasure']['statement']}}
    for key, statement in ATOMS.items():
        assert graph['atoms'][key] == {'id': key, 'proposition': {'kind': 'opaque', 'statement': statement}}
        assert all(key not in premise['children'] for premise in graph['premises'].values())
        assert all(key not in refinement['children'] for name, refinement in graph['refinements'].items() if name != PREMISE)
    result = copy.deepcopy(graph)
    del result['refinements'][PREMISE]
    for key in ATOMS: del result['atoms'][key]
    if not result['refinements']:
        del result['refinements']; result['version'] = 1
    return result


def attribute(document):
    """Return reversed metadata + exact graph digest map + changed path receipt."""
    hashes, changes = {}, []
    def graphs(value, path):
        if isinstance(value, list): return [graphs(item, path + [str(i)]) for i, item in enumerate(value)]
        if not isinstance(value, dict): return value
        if {'version', 'atoms', 'premises'} <= set(value):
            result = reverse_graph(value)
            if result != value:
                before, after = digest(value), digest(result)
                assert hashes.get(before, after) == after
                hashes[before] = after; changes.append({'path': path, 'operation': 'exact-seven-atom-neutral-attachment', 'tip_graph_sha256': before, 'base_graph_sha256': after})
            return result
        return {key: graphs(item, path + [key]) for key, item in value.items()}
    result = graphs(document, [])
    def metadata(value, path):
        if isinstance(value, list): return [metadata(item, path + [str(i)]) for i, item in enumerate(value)]
        if not isinstance(value, dict): return value
        output = {}
        for key, item in value.items():
            if key in ['hypothesis_binding_requirements', '$hypothesis_binding_requirements']:
                kept = []
                for requirement in item:
                    if requirement['premise'] != PREMISE:
                        kept.append(requirement); continue
                    held = MANIFESTS[requirement['manifest_sha256']]
                    assert requirement['declaration'] == held['declaration']
                    assert requirement['graph_sha256'] == held['graph_sha256']
                    changes.append({'path': path + [key], 'operation': 'exact-added-binding-requirement', 'requirement': requirement})
                if kept: output[key] = kept
                continue
            if key == 'hypothesis_dependencies':
                filtered = {}
                for declaration, ids in item.items():
                    selected = set(ids) & set(ATOMS)
                    if selected:
                        assert declaration in SELECTED and selected == set(ATOMS)
                        changes.append({'path': path + [key, declaration], 'operation': 'exact-seven-atom-consultation-closure'})
                    filtered[declaration] = [atom for atom in ids if atom not in selected]
                output[key] = filtered; continue
            if key in ['hypothesis_graph_sha256', 'graph_sha256'] and isinstance(item, str) and item in hashes:
                output[key] = hashes[item]; continue
            if key == '$hypothesis_graph_digests':
                output[key] = {name: hashes.get(digest_value, digest_value) for name, digest_value in item.items()}; continue
            output[key] = metadata(item, path + [key])
        return output
    return metadata(result, []), hashes, changes


def self_check():
    count = 0
    for item in INDEX:
        graph_file = REPORT / f'bindings/{item["property_view"]}-expected-graphs.json'
        graph = json.loads(graph_file.read_text())[item['declaration']]
        assert digest(graph) == item['graph_sha256']
        stripped = reverse_graph(graph)
        assert stripped['premises'] == graph['premises']
        assert not set(ATOMS) & set(stripped['atoms'])
        assert len(stripped['atoms']) + 7 == len(graph['atoms'])
        wrong = copy.deepcopy(graph); wrong['atoms'][next(iter(ATOMS))]['proposition']['statement'] = 'fabricated effect'
        try: reverse_graph(wrong)
        except AssertionError: pass
        else: raise AssertionError('changed atom hidden')
        wrong = copy.deepcopy(graph); wrong['refinements'][PREMISE]['children'].pop()
        try: reverse_graph(wrong)
        except AssertionError: pass
        else: raise AssertionError('missing atom hidden')
        count += 1
    assert count == 166
    print(json.dumps({'held_canonical_graph_sha_checks': count, 'changed_atom_negatives': count,
                      'missing_atom_negatives': count, 'legacy_nodes_removed': 0,
                      'Context_fields_removed': 0, 'source_binding': 'open'}))


if __name__ == '__main__': self_check()
