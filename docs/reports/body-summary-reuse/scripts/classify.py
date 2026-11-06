"""BODY-SUMMARY-REUSE census.

usage: classify.py BASE_REPORT [EXPERIMENT_REPORT] OUT_JSON

Population: `call to uncontracted function F` leaves on non-Safe rows where
F has at least one analysed body in the run and every row of F (every
configuration) is Safe. Each site is classified by where F's body is and,
when an experiment report (same binary, QCORE_CROSS_UNIT_LINK_MAP naming the
population's homes) is given, by what rule 4c's existing route did with it.
"""
import json, re, sys, collections

PAT = re.compile(r"^call to uncontracted function `([^`]+)`")


def load(path):
    d = json.load(open(path))
    rows = collections.defaultdict(list)
    idx = {}
    for u in d['units']:
        for f in u.get('functions', []):
            rows[f['function']].append((u['file'], f['status']))
            idx[(u['file'], f['function'])] = f
    return d, rows, idx


def uncontracted(f):
    return [PAT.match(x.get('description', '')).group(1)
            for x in f.get('diagnostics', []) if PAT.match(x.get('description', ''))]


def static_linkage(src_root, unit_file, name):
    try:
        text = open(f"{src_root}/{unit_file}", errors='replace').read()
    except OSError:
        return None
    return re.search(r"(^|\n)\s*static\b[^;{]*\b" + re.escape(name) + r"\s*\(", text) is not None


def main():
    base_path = sys.argv[1]
    exp_path = sys.argv[2] if len(sys.argv) > 3 else None
    out_path = sys.argv[-1]
    src_root = ("/media/Data/GitHub/quirin-lang/build/cycle19/clang/"
                "046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/"
                "fluent-bit-clang-source")
    d, rows, idx = load(base_path)
    sites = collections.defaultdict(list)
    for u in d['units']:
        for f in u.get('functions', []):
            if f['status'] == 'Safe':
                continue
            for c in uncontracted(f):
                sites[c].append((u['file'], f['function']))
    pop = {c for c in sites if rows.get(c) and all(s == 'Safe' for _, s in rows[c])}
    exp = exp_idx = None
    imported = set()
    if exp_path:
        exp, _, exp_idx = load(exp_path)
        for u in exp['units']:
            for h in (u.get('cross_unit_link_resolution') or {}).get('imported', []):
                imported.add(h['function'])
    per_site = []
    for c in sorted(pop):
        homes = {u for u, _ in rows[c]}
        for (unit, fn) in sites[c]:
            if unit in homes:
                st = static_linkage(src_root, unit, c)
                reason = 'same-unit-internal' if st else 'same-unit-external'
            elif len(homes) > 1:
                reason = 'cross-unit-ambiguous-home'
            elif exp is None:
                reason = 'cross-unit'
            elif c not in imported:
                reason = 'cross-unit-not-exported'
            else:
                after = exp_idx.get((unit, fn))
                still = after is not None and c in uncontracted(after)
                reason = 'cross-unit-imported-still-refused' if still else 'cross-unit-imported-cleared'
            per_site.append({'callee': c, 'unit': unit, 'function': fn, 'reason': reason})
    # sole blocker: every leaf of the function is a population uncontracted call
    sole_fns = set()
    for (unit, fn) in {(s['unit'], s['function']) for s in per_site}:
        f = idx[(unit, fn)]
        if all(PAT.match(x.get('description', '')) and PAT.match(x['description']).group(1) in pop
               for x in f.get('diagnostics', [])):
            sole_fns.add((unit, fn))
    table = collections.OrderedDict()
    for r in sorted({s['reason'] for s in per_site}):
        ss = [s for s in per_site if s['reason'] == r]
        fns = {(s['unit'], s['function']) for s in ss}
        table[r] = {'sites': len(ss), 'callees': len({s['callee'] for s in ss}),
                    'functions': len(fns), 'sole_blocked': len(fns & sole_fns)}
    allf = {(s['unit'], s['function']) for s in per_site}
    total = {'callees': len(pop), 'sites': len(per_site), 'functions': len(allf),
             'sole_blocked': len(sole_fns)}
    moves = None
    if exp_idx is not None:
        moves = collections.Counter()
        moved = []
        for k, f in idx.items():
            a = exp_idx.get(k)
            if a is None:
                moves['missing'] += 1
                continue
            if a['status'] != f['status']:
                moves[f"{f['status']}->{a['status']}"] += 1
                moved.append({'unit': k[0], 'function': k[1], 'before': f['status'], 'after': a['status']})
        moves = {'counts': dict(moves), 'rows': moved}
    json.dump({'total': total, 'table': table, 'imported': sorted(imported),
               'moves': moves, 'sites': per_site}, open(out_path, 'w'), indent=1)
    print(json.dumps(total))
    for r, v in table.items():
        print(f"{v['sites']:5d} {v['callees']:4d} {v['functions']:4d} {v['sole_blocked']:3d}  {r}")
    if moves:
        print(moves['counts'])


main()
