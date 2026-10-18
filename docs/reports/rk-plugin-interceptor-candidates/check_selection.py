#!/usr/bin/env python3
"""Check fresh route metadata loaded this exact runtime-only inventory in force."""
import hashlib
import json
from pathlib import Path
import sys

repo = Path(__file__).resolve().parents[3]
inventory = repo / 'providers/qcore-provider-librdkafka/outer-candidates.json'
expected = hashlib.sha256(inventory.read_bytes()).hexdigest()
measure = Path(sys.argv[1])
output = {}
for route in ('corpus', 'system', 'slice'):
    projection = json.loads((measure / f'{route}-outer.json').read_text())
    pins = projection['selection']['pins']
    candidates = pins['list_candidates']
    assert pins['in_force'] == 3 and pins['held'] == 0, route
    assert sorted(item['loader'] for item in pins['outcomes']) == [
        'flb_plugin_load', 'flb_plugin_proxy_create', 'rd_kafka_plugin_new'
    ], route
    assert candidates['held_loaders'] == [], route
    records = candidates['inventories']
    assert len(records) == 1, route
    record = records[0]
    assert record['sha256'] == expected and record['in_force'], route
    assert record['static_invocation_support'] is False, route
    entries = record['contract']['candidates']
    assert len(entries) == 14 and sum(len(c['aliases']) for c in entries) == 14, route
    output[route] = {
        'candidate_sha256': record['sha256'], 'candidate_source_pins_in_force': True,
        'outer_loaders_in_force': pins['in_force'], 'held_loaders': candidates['held_loaders'],
        'callback_lists': len(entries), 'aliases': sum(len(c['aliases']) for c in entries),
        'static_invocation_support': False,
        'meaning': 'runtime review restrictions; no deployment membership certified',
    }
Path(sys.argv[2]).write_text(json.dumps(output, indent=2, sort_keys=True) + '\n')
print(json.dumps(output, indent=2, sort_keys=True))
