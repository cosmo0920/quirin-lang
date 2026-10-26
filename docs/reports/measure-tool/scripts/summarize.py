#!/usr/bin/env python3
"""Print a short summary of a `quirin-audit measure` output (read-only).

Usage: summarize.py OUT_DIR [OUT_DIR ...]
"""
import json
import pathlib
import sys


def main():
    for out in map(pathlib.Path, sys.argv[1:]):
        record = json.loads((out / 'native-comparison.json').read_text())
        print(f'== {out}: {record["status"]} failures={record["failures"]} '
              f'spelling={record["inputs_spelling"].get("spelling")}')
        for name, side in record['sides'].items():
            binary = (side.get('binary') or {}).get('sha256') or ''
            print(f'  side {name}: {side["kind"]} rev={side.get("revision")} '
                  f'build_commit={side.get("build_commit")} binary={binary[:16]}')
        for comparison in record['comparisons']:
            for route, entry in comparison['routes'].items():
                print(f'  {comparison["comparison"]} {route}: {entry["status"]} '
                      f'{entry.get("why", "")}')
                if 'function_rows' not in entry:
                    continue
                totals = entry['totals']
                print(f'    rows {entry["function_rows"]}')
                print(f'    before {totals["before"]["function_status"]} units '
                      f'{totals["before"]["unit_verdict"]} ({totals["before"]["row_source"][:14]})')
                print(f'    after  {totals["after"]["function_status"]} units '
                      f'{totals["after"]["unit_verdict"]}')
                print(f'    deltas {entry["status_deltas"]}')
                for move in entry['moves']:
                    print(f'    move {move["row"]} {move["status"]} '
                          f'-{len(move["leaves_removed"])}/+{len(move["leaves_added"])} leaves '
                          f'other={move["other_fields_changed"]}')
                for change in entry['content_changes']:
                    print(f'    content {change["row"]} {change["status"]} '
                          f'other={change["other_fields_changed"]}')
                print(f'    unit moves {entry["unit_moves"]}')
                print(f'    unit fields {entry["unit_field_differences"]} '
                      f'env-only units {entry["unit_rows_environment_only"]}')
                composition = entry.get('composition', {})
                print(f'    composition {composition.get("components", composition)} '
                      f'conflicts {composition.get("conflicts")}')
                artifacts = entry['artifacts']
                print(f'    artifacts identical={len(artifacts["identical"])} '
                      f'ok={artifacts["byte_identical_except_named_substitutions"]} '
                      f'differing={[d["file"] for d in artifacts["differing"]]} '
                      f'only={artifacts["only_before"]}{artifacts["only_after"]}')
                for name, swap in artifacts['identical_after_named_substitution'].items():
                    print(f'    substitution {name}: {swap["before_key"][:16]}.. -> '
                          f'{swap["after_key"][:16]}.. x{swap["occurrences"]}')
        if record.get('declared'):
            declared = record['declared']
            print(f'  declared {declared["result"]}: ' + json.dumps(declared['routes']))


if __name__ == '__main__':
    main()
