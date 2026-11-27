import gzip, json, subprocess, collections, re
files = subprocess.check_output(['git', 'ls-files', 'providers/*/contracts/*.qcore']).decode().split()
by = collections.Counter(f.split('/')[1] for f in files)
print(dict(by), 'total', len(files))
decl = sum(open(f).read().count('requires resource_contract_v1;') for f in files)
print('resource-v1 declarations', decl)
d = json.load(gzip.open('docs/reports/w45-container-root-reach/integration/artifacts/diagnostics.json.gz'))
for u in d['units']:
    for f in u['functions']:
        if f['function'] == 'flb_input_chunk_create':
            print(u['file'], f['status'])
executable = sum(v for k, v in by.items() if k.startswith('qcore-provider-'))
shared = sum(v for k, v in by.items() if k.startswith('quirin-provider-'))
print('qcore-provider-* (executable)', executable, 'quirin-provider-*', shared)
