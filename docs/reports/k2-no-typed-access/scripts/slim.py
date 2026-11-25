import json, collections, pickle, sys
R = sys.argv[1]
r = json.load(open(R))
print(list(r.keys()))
u = r['units']
print(len(u))
print(collections.Counter(f['status'] for d in u for f in d['functions']))
print({k: v for k, v in u[0].items() if k != 'functions'})
print(u[0]['functions'][0]['diagnostics'][:2])
slim = [{'file': d['file'], 'library': d.get('library'), 'status': d['status'],
         'functions': [{'function': f['function'], 'status': f['status'],
                        'diagnostics': [(x['category'], x['description'], x['line'], x.get('origin'))
                                        for x in f['diagnostics']]} for f in d['functions']]} for d in u]
pickle.dump(slim, open(sys.argv[2], 'wb'))
