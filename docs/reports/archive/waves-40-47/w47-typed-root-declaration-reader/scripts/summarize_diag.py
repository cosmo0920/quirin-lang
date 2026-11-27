"""Summarise the W47TR trace lines: one row per (function, hook, texts)."""
import re, sys, json
seen = {}
for line in open(sys.argv[1], errors='replace'):
    if not line.startswith('W47TR '):
        continue
    m = re.match(r'W47TR hook=(\S+) fn=(\S+) line=(\d+) readable=(\S+) index=(\S+) texts=(\[.*?\]) names=(\[.*?\]) permissive_return=(.*?) node=(.*)$', line.rstrip('\n'))
    if not m:
        print('UNPARSED', line[:200])
        continue
    hook, fn, ln, readable, index, texts, names, permissive, node = m.groups()
    key = (fn, hook, texts)
    if key in seen:
        continue
    seen[key] = True
    types = re.findall(r'\\"kind\\":\\"(\w+)\\"[^{}]*?\\"type\\":\{\\"(?:desugaredQualType|qualType)\\":\\"([^"\\]*)', node)
    print(f'{fn} [{hook}] line={ln} readable={readable} index={index} texts={texts}')
    print(f'    names={names}')
    print(f'    permissive_return={permissive}')
    print(f'    node types={types[:8]}')
