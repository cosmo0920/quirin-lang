#!/usr/bin/env python3
"""List the call sites of the three newly declared callees whose actual the
resource projection can model, i.e. the only places the corpus could move.

Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF. An `&x->y` actual is refused by the
importer before any contract is consulted (report §3.3), so those sites cannot
move whatever the declaration says. A plain name, or the address of a plain
name, is the shape that can.
"""
import pathlib
import re
import sys

NAMES = ['pthread_mutex_lock', 'pthread_mutex_unlock', 'msgpack_packer_init']
CALL = re.compile(r'\b(' + '|'.join(NAMES) + r')\s*\(([^();]*)\)\s*;')

for directory in sys.argv[1:]:
    for unit in sorted(pathlib.Path(directory).glob('*.i')):
        for line in unit.read_text(errors='replace').splitlines():
            for name, arguments in CALL.findall(line):
                first = arguments.split(',')[0].strip()
                if re.fullmatch(r'&?[A-Za-z_]\w*', first):
                    print(f'{unit.name}\t{name}\t{first}\t{line.strip()[:90]}')
