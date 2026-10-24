#!/usr/bin/env python3
"""Compare BASE and TIP route outputs for the AUDIT-COMPOSITION identity run.

Usage: compare_identity.py RUN_DIR OUT_JSON

For each route (corpus, slice, system) under RUN_DIR/{base,tip}/<route>:

* the TIP file set must be the BASE file set plus exactly the component
  sidecars (`*.components.json`), one beside each unit report;
* every report artifact present on both sides must be byte-identical;
* the measurement driver's own records, which name their side's paths, binary
  digest and wall-clock times, are compared after a checked one-to-one
  substitution of the BASE paths by the TIP paths; every residual difference
  is listed by JSON key, never hidden.

Exit 0 only when every report artifact is byte-identical and the only new
files are the sidecars.
"""
import hashlib
import json
import pathlib
import sys

ROUTES = ('corpus', 'slice', 'system')
# Records that name their side's own directories, binary digest or times:
# the driver's (measure.py, run_pinned_audit.py) and the current route's
# invocation provenance (`current_route::write_invocation`, "not an input to
# any verdict"), which embeds the report path. Compared after the checked
# substitution; residual differences are listed by key.
DRIVER_RECORDS = {'invocation.json', 'console.log', 'stderr.log', 'corpus/run.json',
                  'corpus.exit', 'slice.exit', 'system.exit',
                  'resource-diagnostics.invocation.json'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files(root):
    return {str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}


def diff_keys(left, right, prefix=''):
    """Every JSON path at which two values differ."""
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


def main():
    run = pathlib.Path(sys.argv[1]).resolve()
    out = pathlib.Path(sys.argv[2])
    substitutions = [(str(run / 'base'), str(run / 'tip')),
                     ('/private/ac/base-src', '/private/ac/tip-src')]
    record = {'run': str(run), 'substitutions': substitutions, 'routes': {}}
    ok = True
    for route in ROUTES:
        base, tip = run / 'base' / route, run / 'tip' / route
        base_files, tip_files = files(base), files(tip)
        new = sorted(tip_files - base_files)
        missing = sorted(base_files - tip_files)
        sidecars = [name for name in new if name.endswith('.components.json')]
        unexpected_new = [name for name in new if name not in sidecars]
        identical, differing, driver = [], [], {}
        for name in sorted(base_files & tip_files):
            if (base / name).read_bytes() == (tip / name).read_bytes():
                identical.append(name)
                continue
            if name in DRIVER_RECORDS or name.endswith('.exit'):
                text = (base / name).read_text(errors='replace')
                for old, new_text in substitutions:
                    text = text.replace(old, new_text)
                tip_text = (tip / name).read_text(errors='replace')
                if text == tip_text:
                    driver[name] = {'equal_after_substitution': True}
                    continue
                entry = {'equal_after_substitution': False}
                try:
                    entry['differing_keys'] = diff_keys(json.loads(text), json.loads(tip_text))
                except json.JSONDecodeError:
                    entry['differing_lines'] = sum(
                        1 for a, b in zip(text.splitlines(), tip_text.splitlines()) if a != b
                    ) + abs(len(text.splitlines()) - len(tip_text.splitlines()))
                driver[name] = entry
                continue
            differing.append(name)
        expected_sidecars = sorted(
            str(pathlib.Path(name).with_suffix('.components.json'))
            for name in base_files
            if name.endswith('resource-diagnostics.json')
        )
        route_ok = (not differing and not missing and not unexpected_new
                    and sidecars == expected_sidecars)
        ok = ok and route_ok
        record['routes'][route] = {
            'ok': route_ok,
            'report_artifacts_identical': [
                {'file': name, 'sha256': sha(base / name)}
                for name in identical if name not in DRIVER_RECORDS
            ],
            'report_artifacts_differing': differing,
            'missing_on_tip': missing,
            'new_on_tip': [{'file': name, 'bytes': (tip / name).stat().st_size,
                            'sha256': sha(tip / name)} for name in new],
            'unexpected_new_on_tip': unexpected_new,
            'expected_sidecars': expected_sidecars,
            'driver_records': driver,
        }
    record['ok'] = ok
    out.write_text(json.dumps(record, indent=1) + '\n')
    print(json.dumps({route: value['ok'] for route, value in record['routes'].items()}))
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
