#!/usr/bin/env python3
"""Reading-site and environment review for BASE/TIP snapshots.

Usage: site_review.py BASE_ROUTE_DIR TIP_ROUTE_DIR OUT_JSON

BASE and TIP are separately exported source snapshots, so provider module
paths (`.../base-src/...` versus `.../tip-src/...`) enter the exported
environment texts and every digest over them. This review (1) rewrites only
that snapshot directory name in BASE's reading-site environment texts and
checks each recomputed SHA-256 against a TIP environment, and (2) compares
every row's reading-site evidence as a multiset after removing only the three
digest fields (`id`, `observation.environment_sha256`,
`context.assumptions`). Environment texts nest further digests over the same
paths, so an unmatched rewritten text is compared again with every 64-hex
digest masked (`masked_equal`). Read-only; nothing else is normalized.
"""
import collections
import hashlib
import json
import pathlib
import re
import sys

HEX = re.compile(r'[0-9a-f]{64}')

BASE_DIR = 'detach-wrapper/base-src/'
TIP_DIR = 'detach-wrapper/tip-src/'


def load(root):
    return json.loads((pathlib.Path(root) / 'corpus/resource-diagnostics.json').read_text())


def strip(site):
    site = json.loads(json.dumps(site))
    site.pop('id', None)
    site.get('observation', {}).pop('environment_sha256', None)
    site.get('context', {}).pop('assumptions', None)
    return json.dumps(site, sort_keys=True)


def main():
    base, tip = load(sys.argv[1]), load(sys.argv[2])
    environments = dict(units=0, base=0, tip=0, matched=0, masked_equal=0, unmatched=[])
    sites = dict(rows=0, equal=0, different=[])
    for b, t in zip(base['units'], tip['units']):
        name = f"{b['library']}/{b['file']}"
        b_env = b.get('reading_site_environments', {}).get('environments', {})
        t_env = t.get('reading_site_environments', {}).get('environments', {})
        if b_env or t_env:
            environments['units'] += 1
            environments['base'] += len(b_env)
            environments['tip'] += len(t_env)
            masked_tip = collections.Counter(HEX.sub('<sha>', text) for text in t_env.values())
            for text in b_env.values():
                rewritten = text.replace(BASE_DIR, TIP_DIR)
                digest = hashlib.sha256(rewritten.encode()).hexdigest()
                if t_env.get(digest) == rewritten:
                    environments['matched'] += 1
                elif masked_tip[HEX.sub('<sha>', rewritten)] > 0:
                    masked_tip[HEX.sub('<sha>', rewritten)] -= 1
                    environments['masked_equal'] += 1
                else:
                    environments['unmatched'].append(name)
        b_rows = {r['function']: r for r in b.get('functions', [])}
        t_rows = {r['function']: r for r in t.get('functions', [])}
        for function in sorted(set(b_rows) | set(t_rows)):
            bs = (b_rows.get(function) or {}).get('reading_site_evidence', [])
            ts = (t_rows.get(function) or {}).get('reading_site_evidence', [])
            if not bs and not ts:
                continue
            sites['rows'] += 1
            if collections.Counter(map(strip, bs)) == collections.Counter(map(strip, ts)):
                sites['equal'] += 1
            else:
                sites['different'].append(f'{name}::{function}')
    out = dict(environments=environments, sites=sites)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    raise SystemExit(main())
