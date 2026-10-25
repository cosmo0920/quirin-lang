#!/usr/bin/env python3
"""Compare BASE and TIP route outputs for the AUDIT-COMPOSITION identity run.

Usage: compare_identity.py RUN_DIR OUT_JSON

For each route (corpus, slice, system) under RUN_DIR/{base,tip}/<route>
(a route directory may be a symbolic link into the run that measured it):

* The TIP file set must be the BASE file set plus exactly one component
  sidecar (`*.components.json`) beside each unit report.
* Every report artifact present on both sides must be byte-identical, with
  one checked exception, `resource-diagnostics.contracts.json`'s
  lifetime-source unit identities. `loop_lifetime_source::read_unit` hashes
  the preprocessed text, and Monkey's `mk_bin/monkey.c` expands `__DATE__` and
  `__TIME__`, so that unit's identity depends on the minute it was
  preprocessed, whatever the binary. The file is accepted only when exactly one
  key of the summaries map differs, and replacing every occurrence of BASE's key
  text by TIP's turns the BASE bytes into the TIP bytes exactly. The pair is
  recorded.
* Logs (`audit.log`, `console.log`) and the driver's records name their side's
  output directory, binary digest and times. They are compared after the
  checked substitution of the BASE run directory by the TIP run directory;
  every residual difference is listed (by JSON key or by line), never hidden.

Exit 0 only when every report artifact passes and the only new files are the
sidecars.
"""
import hashlib
import json
import pathlib
import sys

ROUTES = ('corpus', 'slice', 'system')
# The driver's records (measure.py, run_pinned_audit.py) and the current
# route's invocation provenance (`current_route::write_invocation`, "not an
# input to any verdict"), which embeds the report path.
RECORDS = {'invocation.json', 'stderr.log', 'corpus/run.json', 'corpus.exit', 'slice.exit',
           'system.exit', 'resource-diagnostics.invocation.json'}
LOGS = {'console.log', 'corpus/audit.log'}
CONTRACTS = 'resource-diagnostics.contracts.json'


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


def lifetime_keys(document):
    section = document.get('callback_members') or {}
    keys = set((section.get('lifetime_protocols') or {}).get('evidence', {}).get('summaries', {}))
    return keys


def contracts_time_key(base_bytes, tip_bytes):
    """The single time-dependent key swap, or why the file is not that."""
    base_doc, tip_doc = json.loads(base_bytes), json.loads(tip_bytes)
    only_base = sorted(lifetime_keys(base_doc) - lifetime_keys(tip_doc))
    only_tip = sorted(lifetime_keys(tip_doc) - lifetime_keys(base_doc))
    if len(only_base) != 1 or len(only_tip) != 1:
        return None, {'lifetime_keys_only_base': only_base, 'lifetime_keys_only_tip': only_tip}
    old, new = only_base[0], only_tip[0]
    if tip_bytes.count(old.encode()) or base_bytes.count(new.encode()):
        return None, {'collision': [old, new]}
    swapped = base_bytes.replace(old.encode(), new.encode())
    # The key's place in its sorted map moves with its text, so the swapped
    # document is compared as JSON and as a multiset of lines; both must agree.
    json_equal = json.loads(swapped) == tip_doc
    lines_equal = sorted(swapped.splitlines()) == sorted(tip_bytes.splitlines())
    if not (json_equal and lines_equal):
        return None, {'pair': [old, new], 'json_equal_after_swap': json_equal,
                      'lines_equal_after_swap': lines_equal}
    return {'base_key': old, 'tip_key': new, 'occurrences': base_bytes.count(old.encode()),
            'json_equal_after_swap': True, 'lines_equal_after_swap': True}, None


def substituted(text, substitutions):
    for old, new in substitutions:
        text = text.replace(old, new)
    return text


def main():
    run = pathlib.Path(sys.argv[1]).absolute()
    out = pathlib.Path(sys.argv[2])
    record = {'run': str(run), 'routes': {}}
    ok = True
    for route in ROUTES:
        base, tip = (run / 'base' / route).resolve(), (run / 'tip' / route).resolve()
        substitutions = [(str(base.parent), str(tip.parent))]
        base_files, tip_files = files(base), files(tip)
        new = sorted(tip_files - base_files)
        missing = sorted(base_files - tip_files)
        sidecars = [name for name in new if name.endswith('.components.json')]
        unexpected_new = [name for name in new if name not in sidecars]
        identical, differing, time_keys, logs, records = [], [], {}, {}, {}
        for name in sorted(base_files & tip_files):
            left, right = (base / name).read_bytes(), (tip / name).read_bytes()
            if name in RECORDS or name in LOGS:
                text = substituted(left.decode(errors='replace'), substitutions)
                other = right.decode(errors='replace')
                entry = {'equal_after_substitution': text == other}
                if text != other:
                    try:
                        entry['differing_keys'] = diff_keys(json.loads(text), json.loads(other))
                    except json.JSONDecodeError:
                        entry['differing_lines'] = [
                            [a, b] for a, b in zip(text.splitlines(), other.splitlines()) if a != b
                        ]
                        entry['line_counts'] = [len(text.splitlines()), len(other.splitlines())]
                (logs if name in LOGS else records)[name] = entry
                continue
            if left == right:
                identical.append(name)
                continue
            if pathlib.Path(name).name == CONTRACTS:
                swap, why = contracts_time_key(left, right)
                if swap:
                    time_keys[name] = swap
                    continue
                differing.append({'file': name, 'why': why})
                continue
            differing.append({'file': name})
        expected_sidecars = sorted(
            str(pathlib.Path(name).with_suffix('.components.json'))
            for name in base_files if pathlib.Path(name).name == 'resource-diagnostics.json'
        )
        route_ok = (not differing and not missing and not unexpected_new
                    and sidecars == expected_sidecars)
        ok = ok and route_ok
        record['routes'][route] = {
            'ok': route_ok,
            'base': str(base),
            'tip': str(tip),
            'substitutions': substitutions,
            'report_artifacts_identical': [{'file': name, 'sha256': sha(base / name)}
                                           for name in identical],
            'report_artifacts_identical_after_time_key_swap': time_keys,
            'report_artifacts_differing': differing,
            'missing_on_tip': missing,
            'new_on_tip': [{'file': name, 'bytes': (tip / name).stat().st_size,
                            'sha256': sha(tip / name)} for name in new],
            'unexpected_new_on_tip': unexpected_new,
            'expected_sidecars': expected_sidecars,
            'logs': logs,
            'driver_records': records,
        }
    record['ok'] = ok
    out.write_text(json.dumps(record, indent=1) + '\n')
    print(json.dumps({route: value['ok'] for route, value in record['routes'].items()}))
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())
