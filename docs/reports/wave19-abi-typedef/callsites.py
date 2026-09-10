#!/usr/bin/env python3
"""Classify the actual expressions at the three newly declared callees.

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF measurement helper: the declared/C ABI
census says the declarations are in force; this says what shape of actual each
call site passes, which is what decides whether the projection can model it.
"""
import collections
import pathlib
import re
import sys

NAMES = ['pthread_mutex_lock', 'pthread_mutex_unlock', 'msgpack_packer_init']
CALL = re.compile(r'\b(' + '|'.join(NAMES) + r')\s*\(([^();]*)\)')

def shape(text):
    text = text.strip()
    if re.fullmatch(r'[A-Za-z_]\w*', text):
        return 'plain name'
    if re.fullmatch(r'&[A-Za-z_]\w*', text):
        return 'address of a plain name'
    if text.startswith('&') and ('->' in text or '.' in text):
        return 'address of a member'
    if '->' in text or '.' in text:
        return 'member read'
    return 'other'

counts = collections.Counter()
per_name = collections.defaultdict(collections.Counter)
for directory in sys.argv[1:]:
    for unit in sorted(pathlib.Path(directory).glob('*.i')):
        for name, arguments in CALL.findall(unit.read_text(errors='replace')):
            first = arguments.split(',')[0]
            counts[(name, shape(first))] += 1
            per_name[name][unit.name] += 1
for key, value in sorted(counts.items()):
    print(f'{key[0]:22} {key[1]:28} {value}')
print()
for name in NAMES:
    print(f'{name:22} {sum(per_name[name].values())} sites in {len(per_name[name])} units')
