#!/usr/bin/env python3
"""Print the scheduler rows of one resource-diagnostics report.

Usage: rows.py REPORT [FUNCTION...]

Each named row's status, diagnostics and contract dependencies (premises
included), and each unit's body evidence for `mk_http_thread_destroy`.
"""
import json
import sys

DEFAULT = ['sched_thread_cleanup', 'mk_sched_threads_purge', 'mk_sched_threads_destroy_all',
           'mk_sched_threads_destroy_conn', 'mk_http_thread_destroy']


def main():
    report = json.load(open(sys.argv[1]))
    names = sys.argv[2:] or DEFAULT
    for unit in report['units']:
        for row in unit['functions']:
            if row['function'] in names:
                print(json.dumps({'file': unit['file'], 'function': row['function'], 'status': row['status'],
                                  'diagnostics': row.get('diagnostics'),
                                  'contract_dependencies': row.get('contract_dependencies')}, indent=1))
        evidence = unit.get('contract_body_evidence') or {}
        text = json.dumps(evidence)
        if 'mk_http_thread_destroy' in text:
            print(unit['file'], 'body evidence:', json.dumps(evidence, indent=1)[:4000])


if __name__ == '__main__':
    main()
