#!/usr/bin/env python3
"""Keyed comparison of the new `chunkio-core` route against existing routes.

Compares report shape, per-unit verdicts, per-unit diagnostic categories,
per-function status and assumption keys across three configured audits:
the new second source set, the pinned CMetrics route and the archived
synthetic-repository demo. Counts alone are not the comparison; every
difference is keyed by unit or function.
"""
import collections
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
BUILD = ROOT / 'build/cycle19/fluent-bit-clang-production'

ROUTES = {
    'chunkio-core': (BUILD / 'chunkio-core-audit.json',
                     BUILD / 'chunkio-core-audit.assumptions.json'),
    'cmetrics-counter': (BUILD / 'cmetrics-audit.json',
                         BUILD / 'cmetrics-audit.assumptions.json'),
    'w5-synthetic-demo': (ROOT / 'docs/reports/w5-configured-audit/demo/output/report.json',
                          ROOT / 'docs/reports/w5-configured-audit/demo/output/report.assumptions.json'),
}


def summarize(report_path, assumptions_path):
    report = json.loads(report_path.read_text())
    assumptions = json.loads(assumptions_path.read_text())
    units = {}
    functions = {}
    for unit in report['units']:
        key = f"{unit['library']}/{unit['file']}"
        units[key] = {
            'status': unit['status'],
            'reason': unit.get('reason'),
            'source_functions': len(unit['source_functions']),
            'projected_functions': len(unit['functions']),
            'not_projected': sorted(unit['not_projected']),
            'diagnostic_counts': unit['diagnostic_counts'],
        }
        for function in unit['functions']:
            functions[f"{key}::{function['function']}"] = {
                'status': function['status'],
                'diagnostics': len(function['diagnostics']),
            }
    verdicts = collections.Counter(unit['status'] for unit in units.values())
    return {
        'report_keys': sorted(report.keys()),
        'source_revision': report.get('source_revision'),
        'has_source_contract_evidence': 'source_contracts' in report,
        'verdicts': dict(sorted(verdicts.items())),
        'units': units,
        'function_status': dict(sorted(collections.Counter(
            entry['status'] for entry in functions.values()).items())),
        'functions': functions,
        'assumption_keys': sorted(assumptions.keys()),
        'profile_assumption_keys': sorted(assumptions['profile'].keys()),
        'source_set': assumptions['profile']['source_set'],
        'pins': {key: value for key, value in assumptions['profile'].items()
                 if key.startswith('pin.')},
        'scope': assumptions['scope'],
        'contract_source_assumptions': assumptions['contracts'],
        'audited_source_files': sorted(assumptions['source_sha256'].keys()),
    }


def main():
    missing = [name for name, (report, _) in ROUTES.items() if not report.exists()]
    if missing:
        print(f'missing prepared reports: {missing}', file=sys.stderr)
        return 1
    summaries = {name: summarize(*paths) for name, paths in ROUTES.items()}
    shared_keys = set.intersection(*(set(s['report_keys']) for s in summaries.values()))
    comparison = {
        'schema_version': 1,
        'routes': summaries,
        'shared_report_keys': sorted(shared_keys),
        'report_key_differences': {
            name: sorted(set(summary['report_keys']) - shared_keys)
            for name, summary in summaries.items()
        },
        'shared_assumption_keys': sorted(set.intersection(
            *(set(s['assumption_keys']) for s in summaries.values()))),
        'scope': ('Bounded resource projection comparison across configured routes. '
                  'Unknown stays in the denominator; a provisional Unsafe is not an '
                  'upstream defect and no raw unit is promoted.'),
    }
    (HERE / 'comparison.json').write_text(json.dumps(comparison, indent=2) + '\n')
    for name, summary in summaries.items():
        print(f"{name}: {summary['verdicts']} functions={summary['function_status']}")
        print(f"  report keys: {summary['report_keys']}")
        print(f"  assumption keys: {summary['assumption_keys']}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
