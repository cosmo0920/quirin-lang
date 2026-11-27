#!/usr/bin/env python3
"""Summarise a diff_artifacts.py document: which artifacts moved and how much.

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF. Constraint 22 requires the `contracts`
section's normalised AND raw module keys to be read together, so both are
printed here rather than one standing for the other.
"""
import json
import sys

document = json.load(open(sys.argv[1]))
print('schema_version:', document.get('schema_version'))
for stem, entry in sorted((document.get('artifacts') or {}).items()):
    if not entry.get('archived_present') or not entry.get('measured_present'):
        print(f'{stem}: archived={entry.get("archived_present")} measured={entry.get("measured_present")}')
        continue
    parts = []
    envelope = entry.get('envelope') or {}
    if envelope:
        parts.append(f'envelope keys moved: {sorted(envelope)}')
    for field in ('units', 'results', 'backend_teardown', 'callbacks', 'wrappers'):
        section = entry.get(field)
        if not isinstance(section, dict):
            continue
        added = len(section.get('added') or [])
        removed = len(section.get('removed') or [])
        changed = len(section.get('changed') or [])
        if added or removed or changed:
            parts.append(f'{field}: +{added} -{removed} ~{changed}')
    if stem == 'contracts':
        for key in sorted(entry):
            if key in ('archived_present', 'measured_present'):
                continue
            value = entry[key]
            if isinstance(value, (dict, list)) and value:
                parts.append(f'{key}: {json.dumps(value)[:400]}')
            elif not isinstance(value, (dict, list)):
                parts.append(f'{key}: {value}')
    print(f'--- {stem}: ' + ('; '.join(parts) if parts else 'NO DIFFERENCES'))
