#!/usr/bin/env python3
"""Compare BASE and TIP route outputs for the CONTRACT-SUPERSESSION identity run.

Usage: compare_identity.py RUN_DIR BASE_SRC TIP_SRC OUT_JSON [ROUTE...]

Adapted from docs/reports/audit-composition/scripts/compare_identity.py. For
each route (corpus, slice, system) under RUN_DIR/{base,tip}/<route>:

* The file sets must be equal (BASE already writes the component sidecars).
* Every report artifact must be byte-identical, or byte-identical after one
  checked one-to-one substitution: every pin lock's BASE file digest by its
  TIP file digest. The re-pin changes every lock's bytes (stamps and the
  revision tie only, docs/reports/contract-supersession/repin-review.json),
  and the audit records each lock it reads with the SHA-256 of its bytes
  (`contract_revision.locks`, and the bounds and outer pin records). The map
  is computed here from the two snapshots' lock files; it must be injective,
  and every substituted occurrence is counted. Any other difference fails.
* With SOURCE_DATE_EPOCH set on both sides, the Monkey `__DATE__`/`__TIME__`
  lifetime-source key must not vary; if it does, the file fails.
* Logs and driver records are compared after the substitution of the BASE run
  directory by the TIP one, and every residual difference is listed.

Exit 0 only when every report artifact passes.
"""
import hashlib
import json
import pathlib
import sys

ROUTES = ('corpus', 'slice', 'system')
RECORDS = {'invocation.json', 'stderr.log', 'corpus/run.json', 'corpus.exit', 'slice.exit',
           'system.exit', 'resource-diagnostics.invocation.json'}
LOGS = {'console.log', 'corpus/audit.log'}
LOCKS = ('contracts.lock', 'bounds-contracts.lock', 'outer-contracts.lock')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files(root):
    return {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}


def diff_keys(left, right, prefix=''):
    if isinstance(left, dict) and isinstance(right, dict):
        out = []
        for key in sorted(set(left) | set(right)):
            out.extend(diff_keys(left.get(key), right.get(key), f'{prefix}/{key}'))
        return out
    if isinstance(left, list) and isinstance(right, list) and len(left) == len(right):
        out = []
        for index, (a, b) in enumerate(zip(left, right)):
            out.extend(diff_keys(a, b, f'{prefix}/{index}'))
        return out
    return [] if left == right else [prefix or '/']


def lock_map(base_src, tip_src):
    mapping = {}
    for path in sorted((base_src / 'providers').glob('*/*.lock')):
        if path.name not in LOCKS:
            continue
        relative = path.relative_to(base_src)
        mapping[sha(path)] = {'tip': sha(tip_src / relative), 'lock': str(relative)}
    tips = [entry['tip'] for entry in mapping.values()]
    assert len(set(tips)) == len(tips) == len(mapping), 'the lock digest map is not injective'
    assert not set(tips) & set(mapping), 'a TIP digest is also a BASE digest'
    return mapping


def substituted(text, substitutions):
    for old, new in substitutions:
        text = text.replace(old, new)
    return text


def main():
    run = pathlib.Path(sys.argv[1]).absolute()
    base_src, tip_src = pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    out = pathlib.Path(sys.argv[4])
    locks = lock_map(base_src, tip_src)
    record = {'run': str(run), 'lock_digest_substitution': locks, 'routes': {}}
    ok = True
    for route in sys.argv[5:] or ROUTES:
        base, tip = (run / 'base' / route).resolve(), (run / 'tip' / route).resolve()
        dirs = [(str(base.parent), str(tip.parent))]
        base_files, tip_files = files(base), files(tip)
        identical, after_lock_substitution, differing, logs, records = [], {}, [], {}, {}
        for name in sorted(base_files & tip_files):
            left, right = (base / name).read_bytes(), (tip / name).read_bytes()
            if name in RECORDS or name in LOGS:
                text = substituted(left.decode(errors='replace'), dirs)
                other = right.decode(errors='replace')
                entry = {'equal_after_substitution': text == other}
                if text != other:
                    try:
                        entry['differing_keys'] = diff_keys(json.loads(text), json.loads(other))
                    except json.JSONDecodeError:
                        entry['differing_lines'] = [
                            [a, b] for a, b in zip(text.splitlines(), other.splitlines()) if a != b
                        ][:20]
                        entry['line_counts'] = [len(text.splitlines()), len(other.splitlines())]
                (logs if name in LOGS else records)[name] = entry
                continue
            if left == right:
                identical.append(name)
                continue
            counts = {}
            swapped = left
            for old, entry in locks.items():
                n = swapped.count(old.encode())
                if n:
                    counts[entry['lock']] = n
                    swapped = swapped.replace(old.encode(), entry['tip'].encode())
            if swapped == right:
                after_lock_substitution[name] = {'substituted': counts, 'tip_sha256': sha(tip / name)}
                continue
            entry = {'file': name}
            try:
                entry['differing_keys'] = diff_keys(json.loads(swapped), json.loads(right))[:50]
            except (json.JSONDecodeError, UnicodeDecodeError):
                entry['differing_lines'] = sum(
                    1 for a, b in zip(swapped.splitlines(), right.splitlines()) if a != b)
            differing.append(entry)
        missing = sorted(base_files - tip_files)
        new = sorted(tip_files - base_files)
        route_ok = not differing and not missing and not new
        ok = ok and route_ok
        record['routes'][route] = {
            'ok': route_ok,
            'base': str(base),
            'tip': str(tip),
            'directory_substitution': dirs,
            'report_artifacts_identical': [{'file': n, 'sha256': sha(base / n)} for n in identical],
            'report_artifacts_identical_after_lock_digest_substitution': after_lock_substitution,
            'report_artifacts_differing': differing,
            'missing_on_tip': missing,
            'new_on_tip': new,
            'logs': logs,
            'driver_records': records,
        }
    record['ok'] = ok
    out.write_text(json.dumps(record, indent=1) + '\n')
    print(json.dumps({route: value['ok'] for route, value in record['routes'].items()}))
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
