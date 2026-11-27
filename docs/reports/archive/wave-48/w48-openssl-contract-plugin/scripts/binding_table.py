"""W48-OPENSSL-CONTRACT-PLUGIN: the per-binding table of the report.

Joins the plugin's check record (proposal, class, first undischarged
obligation) with the host's report rows (the published verdict) and prints a
Markdown table, bindings in the contract set's order (ERR_clear_error first,
then by census pairs).

usage: python3 -I binding_table.py openssl-check.json quirin-audit-openssl.report.json
"""
import json
import sys


def main():
    record = json.load(open(sys.argv[1]))
    report = json.load(open(sys.argv[2]))
    rows = {row['artifact']: row for row in report['rows']}
    print('| Function | Pairs | Class | Proposed | Published | Undischarged obligation |')
    print('|---|---:|---|---|---|---|')
    for binding in record['bindings']:
        row = rows['openssl@3.5.5::' + binding['function']]
        failed = next((o for o in binding['obligations'] if not o['discharged']), None)
        why = ''
        if failed:
            detail = failed['detail']
            if failed['obligation'] == 'class-rule' and 'no rule for contract class' in detail:
                detail = 'no rule for the class'
            why = '`%s`: %s' % (failed['obligation'], detail.split(' (')[0])
        print('| `%s` | %d | %s | %s | **%s** | %s |' % (
            binding['function'], binding['census_pairs'], binding['class'],
            binding['proposed'], row['verdict'], why))
    counts = {}
    for row in report['rows']:
        counts[row['verdict']] = counts.get(row['verdict'], 0) + 1
    print()
    print('published:', counts, '; report verdict:', report['verdict'])


main()
