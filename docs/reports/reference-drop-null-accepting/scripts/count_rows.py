"""Count the six OpenSSL frees' call diagnostics and their rows in a current
resource report (card REFERENCE-DROP-NULL-ACCEPTING). Reads files only."""
import json
import re
import sys

d = json.load(open(sys.argv[1]))
six = {'X509_free', 'EVP_PKEY_free', 'SSL_free', 'SSL_CTX_free', 'BIO_free', 'ENGINE_free'}
rows = set()
n = 0
for u in d['units']:
    for r in u.get('functions', []):
        for dg in r.get('diagnostics', []):
            m = re.search(r'call to `(\w+)` with', json.dumps(dg))
            if m and m.group(1) in six:
                rows.add(r['function'])
                n += 1
print(n, len(rows), sorted(rows))
