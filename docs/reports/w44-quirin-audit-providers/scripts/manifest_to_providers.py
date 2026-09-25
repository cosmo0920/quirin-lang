#!/usr/bin/env python3
"""Write the provider selection equal to a schema-2 resource manifest.

Usage: manifest_to_providers.py MANIFEST.json [--config OUT.json KEY=VALUE...]

Each manifest entry becomes one `crate:module` provider entry, in manifest
order: a `crate:module` reference is kept; a file path is mapped to the crate
whose contract directory (from the manifest's load path) contains it. Without
--config the entries are printed one per line; with it, a `quirin-audit
current` configuration is written with `providers` and every KEY=VALUE given
(values are JSON when they parse as JSON, strings otherwise).
"""
import json
import sys
from pathlib import Path


def selection(manifest_path):
    manifest_path = Path(manifest_path)
    manifest = json.loads(manifest_path.read_text())
    if manifest.get('schema_version') != 2:
        raise SystemExit('a schema-2 manifest with a load path is required')
    load_path = manifest_path.parent / manifest['load_path']
    crates = json.loads(load_path.read_text())['crates']
    roots = {c['name']: f"{Path(manifest['load_path']).parent.as_posix()}/{c['contracts']}/" for c in crates}
    entries = []
    for entry in manifest['modules']:
        if ':' in entry:
            entries.append(entry)
            continue
        owners = [(name, entry[len(root):]) for name, root in roots.items() if entry.startswith(root)]
        if len(owners) != 1:
            raise SystemExit(f'{entry}: owned by {len(owners)} crates')
        entries.append(f'{owners[0][0]}:{owners[0][1]}')
    return entries


def main():
    arguments = sys.argv[1:]
    if not arguments:
        raise SystemExit(__doc__)
    entries = selection(arguments[0])
    if len(arguments) >= 2 and arguments[1] == '--config':
        config = {'schema_version': 1}
        for pair in arguments[3:]:
            key, value = pair.split('=', 1)
            try:
                config[key] = json.loads(value)
            except json.JSONDecodeError:
                config[key] = value
        config['providers'] = entries
        Path(arguments[2]).write_text(json.dumps(config, indent=2) + '\n')
    else:
        print('\n'.join(entries))


if __name__ == '__main__':
    main()
