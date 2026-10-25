#!/usr/bin/env python3
"""Show that one lifetime-source summary key changes from run to run.

Usage: time_key.py SIDECAR [SIDECAR ...]   (each a corpus resource-diagnostics.contracts.json)

The member pre-pass keys each unit's lifetime-source summary by a digest over
its preprocessing configuration and preprocessed text. Fluent Bit's compile
database compiles lib/monkey/mk_bin/monkey.c, whose MONKEY_BUILT is
`__DATE__ " " __TIME__`, so that unit's text, and its key, depend on the
clock. Over runs that spell their inputs the same way, every key but one must
be shared, and the one per run must be unique to it with an empty summary.
Read-only; prints JSON.
"""
import json
import sys


def keys(path):
    data = json.load(open(path))
    section = data['callback_members']['lifetime_protocols']['evidence']['summaries']
    return {key: value for key, value in section.items()}


def main():
    runs = {path: keys(path) for path in sys.argv[1:]}
    shared = set.intersection(*(set(value) for value in runs.values()))
    out = {'runs': len(runs), 'shared_keys': len(shared), 'per_run': {}}
    for path, summaries in runs.items():
        own = sorted(set(summaries) - shared)
        out['per_run'][path] = {'keys': len(summaries), 'unshared': own,
                                'unshared_values': [summaries[key] for key in own]}
    others = [set(summaries) - shared for summaries in runs.values()]
    out['unshared_pairwise_disjoint'] = all(
        not (a & b) for i, a in enumerate(others) for b in others[i + 1:])
    out['shared_values_equal'] = all(
        len({json.dumps(summaries[key], sort_keys=True) for summaries in runs.values()}) == 1
        for key in shared)
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    main()
