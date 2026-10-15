#!/usr/bin/env python3
"""edit_pins.py -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: raise
`compiler/tests/qcore_contract_corpus.rs`'s EXPECTED_CONTRACTS clause pin of
each declaration write_contracts.py gave the premise by exactly one (the
`requires resource_premise(rk_no_plugin_library, ...)` clause), with one
comment line above the first. Every row must exist exactly once and nothing
else changes; the test itself then checks the counts against the parsed
corpus."""
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
TEST = REPO / "compiler/tests/qcore_contract_corpus.rs"
sys.argv = [sys.argv[0], "--check"]
exec_globals = {"__file__": str(HERE / "write_contracts.py"), "__name__": "write_contracts"}
source_text = (HERE / "write_contracts.py").read_text().replace("\nmain()\n", "\n")
exec(compile(source_text, "write_contracts.py", "exec"), exec_globals)
FROZEN = exec_globals["FROZEN"]

source = TEST.read_text()
start = source.index("EXPECTED_CONTRACTS")
body_start = source.index("[", start)
body_end = source.index("\n];", body_start)
table = source[body_start:body_end]
first = None
for name in FROZEN:
    pattern = re.compile(r'(\(\s*"librdkafka/client\.qv",\s*"%s",\s*)(\d+)(,?\s*\))' % re.escape(name), re.S)
    matches = list(pattern.finditer(table))
    if len(matches) != 1:
        raise SystemExit(f"{name}: {len(matches)} pin rows")
    m = matches[0]
    table = table[:m.start(2)] + str(int(m.group(2)) + 1) + table[m.end(2):]
    first = m.start() if first is None else min(first, m.start())
line_start = table.rindex("\n", 0, first) + 1
table = (table[:line_start]
         + "    // Card LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: +1 on each of the 34 client.qv declarations\n"
         + "    // whose body may reach rk_conf.log_cb (`rk_no_plugin_library`, user decision U2).\n"
         + table[line_start:])
TEST.write_text(source[:body_start] + table + source[body_end:])
print(len(FROZEN), "rows raised")
