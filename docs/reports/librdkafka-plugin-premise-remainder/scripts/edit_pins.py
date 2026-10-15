#!/usr/bin/env python3
"""edit_pins.py -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (adapted from
LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED): raise
`compiler/tests/qcore_contract_corpus.rs`'s EXPECTED_CONTRACTS clause pin of
each declaration write_contracts.py gave the premise by exactly one (the
`requires resource_premise(rk_no_plugin_library, ...)` clause): the 67
premised null_deref.qv declarations, memory.qv's rd_kafka_crash, kafka_mem.qv's
rd_kafka_dump0 and queue_destructor.qv's rd_kafka_q_destroy_final, with one
comment line above the first raised row of each module. Every row must exist
exactly once and nothing else changes; the test itself then checks the counts
against the parsed corpus."""
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
TEST = REPO / "compiler/tests/qcore_contract_corpus.rs"
rows = json.loads((HERE.parent / "artifacts/member-census.json").read_text())["rows"]
NULL = sorted({r["function"] for r in rows if r["premise"]})
assert len(NULL) == 67
TARGETS = [("librdkafka/null_deref.qv", n) for n in NULL] + [
    ("librdkafka/memory.qv", "rd_kafka_crash"),
    ("librdkafka/kafka_mem.qv", "rd_kafka_dump0"),
    ("librdkafka/queue_destructor.qv", "rd_kafka_q_destroy_final"),
]
NOTE = {
    "librdkafka/null_deref.qv": "    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1 on each of the 67 premised\n"
                                "    // nonnull_member_stable / nonnull_member_scope declarations (`rk_no_plugin_library`).\n",
    "librdkafka/memory.qv": "    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1, the crash path names `rk_no_plugin_library`.\n",
    "librdkafka/kafka_mem.qv": "    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1, the dump names `rk_no_plugin_library`.\n",
    "librdkafka/queue_destructor.qv": "    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: 6 -> 7, the model contract names\n"
                                      "    // `rk_no_plugin_library` (user decision 2026-10-15, \"Add premise support\").\n",
}

source = TEST.read_text()
start = source.index("EXPECTED_CONTRACTS")
body_start = source.index("[", start)
body_end = source.index("\n];", body_start)
table = source[body_start:body_end]
first = {}
for module, name in TARGETS:
    pattern = re.compile(r'(\(\s*"%s",\s*"%s",\s*)(\d+)(,?\s*\))' % (re.escape(module), re.escape(name)), re.S)
    matches = list(pattern.finditer(table))
    if len(matches) != 1:
        raise SystemExit(f"{module} {name}: {len(matches)} pin rows")
    m = matches[0]
    table = table[:m.start(2)] + str(int(m.group(2)) + 1) + table[m.end(2):]
    first[module] = m.start() if module not in first else min(first[module], m.start())
for module, at in sorted(first.items(), key=lambda kv: -kv[1]):
    line_start = table.rindex("\n", 0, at) + 1
    table = table[:line_start] + NOTE[module] + table[line_start:]
TEST.write_text(source[:body_start] + table + source[body_end:])
print(len(TARGETS), "rows raised")
