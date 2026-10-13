import re, subprocess, sys

files = subprocess.run(['git', 'ls-files', '*.rs', '*.sh', '*.py'], capture_output=True, text=True).stdout.split()
pat = re.compile(r'(\)\s*&|[A-Za-z0-9"\'/.]\s&)(\\n|\s|$|"|;)')
hits = 0
for f in files:
    if f.startswith('docs/'):
        continue
    try:
        text = open(f, encoding='utf-8').read()
    except Exception:
        continue
    if 'bin/sh' not in text:
        continue
    for i, line in enumerate(text.splitlines(), 1):
        l = line.replace('&&', '')
        if pat.search(l) and '&mut' not in l and '&self' not in l:
            print(f"{f}:{i}: {line.strip()[:200]}")
            hits += 1
print('hits', hits)
