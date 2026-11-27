"""parity_units.py BEFORE.jsonl AFTER.jsonl ANCHOR-FUNCTION -- RDHTTP-HEADER-SUMMARY-GAP:
the multiset difference of two QCORE_TYPED_PATH_PARITY streams (Clang node
addresses erased, as parity.py does), by analysed source (`source_sha256`), with
the functions of each source's differing rows. ANCHOR-FUNCTION names a function
whose rows identify the source expected to hold every difference (rdhttp.c's
`rd_http_get`); exit 0 only when every differing row has that source."""
import collections
import json
import re
import sys

ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def read(path):
    rows = []
    with open(path) as stream:
        for line in stream:
            rows.append(ADDRESS.sub('"0x"', line.rstrip('\n')).replace(', retention: None', ''))
    return collections.Counter(rows)


before, after = read(sys.argv[1]), read(sys.argv[2])
anchor = sys.argv[3]
sources = {json.loads(row).get('source_sha256') for row in after if json.loads(row).get('function') == anchor}
print('anchor sources', sorted(sources))
by_source = collections.defaultdict(collections.Counter)
for side, diff in (('only_before', before - after), ('only_after', after - before)):
    for row, count in diff.items():
        data = json.loads(row)
        by_source[data.get('source_sha256')][(side, data.get('function'))] += count
outside = 0
for source, functions in sorted(by_source.items(), key=lambda item: str(item[0])):
    inside = source in sources
    if not inside:
        outside += sum(functions.values())
    print(source, 'anchor-source' if inside else 'OTHER SOURCE', sum(functions.values()))
    for (side, function), count in sorted(functions.items(), key=lambda item: -item[1]):
        print('   ', side, function, count)
print('rows outside the anchor source', outside)
sys.exit(0 if outside == 0 else 1)
