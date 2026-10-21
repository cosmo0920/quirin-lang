#!/usr/bin/env python3
"""Produce fresh, explicitly non-production contract drafts from the frozen census.

No tracked contract or frozen record is mutated. The final merged-API refresh
must pass independently before installing these declaration-local attachments.
"""
from pathlib import Path
from collections import Counter
import argparse
import hashlib
import json
import subprocess

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    census = json.loads((REPORT / 'census.json').read_text())
    design = json.loads((REPORT / 'formal-catalog-design.json').read_text())
    atoms = sorted(design['callback_atoms'], key=lambda atom: atom['id'])
    assert len(atoms) == 7
    assert len({atom['id'] for atom in atoms}) == 7
    assert all('\\' not in atom['statement'] and '"' not in atom['statement'] for atom in atoms)
    lines = [f'    requires resource_hypothesis({atom["id"]}, "{atom["statement"]}");'
             for atom in atoms]
    lines.append('    requires resource_premise_decomposition(rk_plugin_callback_closure, '
                 + ', '.join(atom['id'] for atom in atoms) + ');')
    tracked = set(subprocess.check_output(['git', 'ls-files', 'providers'], cwd=REPO,
                                        text=True).splitlines())
    modules = sorted({row['path'] for row in census['function_resource_premise_sites']})
    receipt = []
    for relative in modules:
        assert relative in tracked
        source = (REPO / relative).read_text()
        rows = [row for row in census['function_resource_premise_sites'] if row['path'] == relative]
        expected_sha = {row['sha256'] for row in rows}
        assert expected_sha == {hashlib.sha256(source.encode()).hexdigest()}, relative
        assert 'resource_premise_decomposition(rk_plugin_callback_closure' not in source
        before = source.splitlines(keepends=True)
        replacements = {row['line']: row for row in rows}
        after = []
        for number, line in enumerate(before, 1):
            after.append(line)
            if number in replacements:
                assert line.rstrip('\n') == replacements[number]['text']
                after.extend(item + '\n' for item in lines)
        result = ''.join(after)
        # Reconstruct original bytes by deleting only the newly inserted exact block.
        assert result.replace('\n'.join(lines) + '\n', '') == source
        assert result.count('requires resource_premise_decomposition(rk_plugin_callback_closure,') == len(rows)
        destination = output / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(result)
        receipt.append({'path': relative, 'declarations': [row['declaration'] for row in rows],
                        'before_sha256': hashlib.sha256(source.encode()).hexdigest(),
                        'draft_sha256': hashlib.sha256(result.encode()).hexdigest(),
                        'occurrences': len(rows), 'added_clauses': len(rows) * 8})
    assert sum(row['occurrences'] for row in receipt) == 166
    assert sum(row['added_clauses'] for row in receipt) == 1328
    (output / 'receipt.json').write_text(json.dumps({'production_modified': False,
        'meaning': 'Draft only, before explicit merged-base refresh and property-specific admission.',
        'module_drafts': receipt}, indent=2) + '\n')
    print(json.dumps({'draft_modules': len(receipt), 'occurrences': 166, 'added_clauses': 1328}))


if __name__ == '__main__':
    main()
