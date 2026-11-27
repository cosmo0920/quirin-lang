#!/usr/bin/env python3
"""edit_pins.py ACTUAL.txt -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: update
`compiler/tests/qcore_contract_corpus.rs`'s EXPECTED_CONTRACTS clause pins for
the card's declarations from the test's own `left:` list (the parsed corpus),
read from ACTUAL.txt (the failing test's output). Only rows of the card's
declarations, and the mock cluster's restated premises, change; every other
row must already match, or the script stops."""
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
TEST = REPO / "compiler/tests/qcore_contract_corpus.rs"

text = pathlib.Path(sys.argv[1]).read_text()
left = text[text.index("left: [") + len("left: "):]
left = left[: left.index("]\n") + 1]
actual = [(m.group(1), m.group(2), int(m.group(3)))
          for m in re.finditer(r'\("([^"]+)", "([^"]+)", (\d+)\)', left)]

source = TEST.read_text()
start = source.index("EXPECTED_CONTRACTS")
body_start = source.index("[", start)
body_end = source.index("\n];", body_start)
table = source[body_start:body_end]
entry = re.compile(r'\(\s*"([^"]+)",\s*"([^"]+)",\s*(\d+),?\s*\)', re.S)
expected = [(m.group(1), m.group(2), int(m.group(3))) for m in entry.finditer(table)]

ours = {"librdkafka/client.qv", "fluent_bit/logging.qv"}
new_rows = [row for row in actual if row[:2] not in {(p, n) for p, n, _ in expected}]
for row in new_rows:
    assert row[0] in ours, row
changed = {(p, n): c for p, n, c in actual}
# Every existing row keeps its count except the restated mock cluster.
for p, n, c in expected:
    if changed.get((p, n)) != c:
        assert (p, n) == ("librdkafka/client.qv", "rd_kafka_mock_cluster_op_serve"), (p, n, c, changed.get((p, n)))


def spell(row):
    path, name, count = row
    one = f'    ("{path}", "{name}", {count}),'
    if len(one) <= 100:
        return one
    return f'    (\n        "{path}",\n        "{name}",\n        {count},\n    ),'


# Rewrite the mock row's count.
p, n = "librdkafka/client.qv", "rd_kafka_mock_cluster_op_serve"
table = re.sub(r'\("%s", "%s", \d+\)' % (re.escape(p), re.escape(n)),
               '("%s", "%s", %d)' % (p, n, changed[(p, n)]), table)
# Insert each new row after the previous row of the actual order.
for row in new_rows:
    index = actual.index(row)
    previous = actual[index - 1]
    anchor = re.compile(r'\(\s*"%s",\s*"%s",\s*\d+,?\s*\),' % (re.escape(previous[0]), re.escape(previous[1])), re.S)
    match = None
    for match in anchor.finditer(table):
        pass
    assert match, previous
    comment = ""
    if row == new_rows[0] or (new_rows.index(row) > 0 and new_rows[new_rows.index(row) - 1][0] != row[0]):
        comment = ("\n    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: the rk_conf setters, "
                   "`rd_kafka_conf_new`, the member\n    // invokers and the run-closed candidates."
                   if row[0] == "librdkafka/client.qv" else
                   "\n    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: Fluent Bit's registered "
                   "rk_conf callbacks.")
    table = table[: match.end()] + comment + "\n" + spell(row) + table[match.end():]
TEST.write_text(source[:body_start] + table + source[body_end:])
print(len(new_rows), "rows added")
