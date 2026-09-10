#!/usr/bin/env python3
"""Preprocess every unit of the pinned corpus with its own compile command.

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF measurement helper. The declared/C ABI
census reads preprocessed text; this produces it the same way card LOADER-ABI's
sweep did — each unit with its OWN full compile command, which is deliberately
not the historical option subset the pinned compatibility route projects.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

database = Path(sys.argv[1])
out = Path(sys.argv[2])
selection = json.loads(Path(sys.argv[3]).read_text()) if len(sys.argv) > 3 else None
out.mkdir(parents=True, exist_ok=True)
entries = json.loads(database.read_text())
written, failed = 0, 0
seen = set()
for entry in entries:
    source = Path(entry['file'])
    if source.suffix not in ('.c',):
        continue
    if selection is not None and not any(str(source).find(part) >= 0 for part in selection):
        continue
    name = source.name
    if name in seen:
        name = f'{source.parent.name}__{name}'
    seen.add(name)
    target = out / (name + '.i')
    command = entry.get('arguments')
    if command is None:
        command = __import__('shlex').split(entry['command'])
    command = [c for c in command if c not in ('-c',)]
    # Drop the output selection and add -E.
    filtered, skip = [], False
    for token in command:
        if skip:
            skip = False
            continue
        if token == '-o':
            skip = True
            continue
        filtered.append(token)
    filtered.append('-E')
    result = subprocess.run(filtered, cwd=entry['directory'], capture_output=True, text=True)
    if result.returncode:
        failed += 1
        continue
    target.write_text(result.stdout)
    written += 1
print(f'preprocessed {written}, failed {failed}', file=sys.stderr)
