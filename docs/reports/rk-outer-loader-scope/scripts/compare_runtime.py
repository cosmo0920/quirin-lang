#!/usr/bin/env python3
"""Compare the removed RK promises with the completed composed checkpoint."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
REPORT = ROOT / 'docs/reports/rk-outer-loader-scope'
CENSUS = json.loads((REPORT / 'artifacts/old-envelope-dependencies.json').read_text())
COMPOSED = ROOT / 'docs/reports/fb-event-loop-lifetime-s3/artifacts'


def row_key(row):
    return (row['part'], row['library'], row['file'], row['function'], row['occurrence'])


def main():
    receipt = {
        'basis': CENSUS['basis'],
        'comparison': 'frozen lexical913 before versus composed S3+corrected RK after; not isolated S3 experiment',
        'routes': {},
    }
    for route, scenarios in CENSUS['routes'].items():
        actual = json.loads((COMPOSED / route / 'runtime-attribution.json').read_text())
        receipt['routes'][route] = {}
        for scenario, expected in scenarios.items():
            rows = actual['scenarios'][scenario]['other_changed_moves']
            expected_keys = {
                row_key(row) for row in expected['rows_with_removed_RK_promise']
                + expected['matching_combined_rows']
            }
            actual_keys = {tuple(row['key']) for row in rows}
            assert actual_keys == expected_keys, (route, scenario, actual_keys ^ expected_keys)
            assert all(row['before']['runtime'] == 'Pending'
                       and row['after']['runtime'] == 'Unknown' for row in rows)
            null_rows = [row for row in rows if row['key'][0] == 'null']
            assert all(any('outer-uncovered:' in reason and 'rk_plugin_library' in reason
                           for reason in row['after']['reasons']) for row in null_rows)
            receipt['routes'][route][scenario] = {
                'exact_dependency_census_match': True,
                'null_pending_to_unknown': len(null_rows),
                'combined_pending_to_unknown': len(rows) - len(null_rows),
                'rows': rows,
            }
    (REPORT / 'artifacts/composed-runtime-receipt.json').write_text(
        json.dumps(receipt, indent=2) + '\n')
    print('9 route/scenario sets exactly match; null 1/2/16, combined 1/2/0 under all/RK-only')


if __name__ == '__main__':
    main()
