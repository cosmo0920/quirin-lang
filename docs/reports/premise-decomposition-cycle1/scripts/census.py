#!/usr/bin/env python3
"""Small tracked provider-resource prose-premise census."""
import json
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[4]
rows = []
paths = subprocess.check_output(['git', 'ls-files', '-z', 'providers'], cwd=ROOT).decode().split('\0')
inputs = [p for p in paths if p.endswith(('.qv', '.qcore'))]
for path in inputs:
    text = (ROOT / path).read_text()
    for match in re.finditer(r'requires resource_premise\((\w+), "([^"\n]*)"\);', text):
        declaration = re.findall(r'extern fn (\w+)', text[:match.start()])[-1]
        rows.append({'path': path, 'declaration': declaration, 'name': match[1], 'statement': match[2]})
print(json.dumps({'tracked_inputs': len(inputs), 'premise_clauses': len(rows), 'cjson_candidates': [r for r in rows if r['path'].endswith('/cjson.qv')], 'selected': {'declaration': 'cJSON_DetachItemViaPointer', 'name': 'detach_list_representation'}}, indent=2))
