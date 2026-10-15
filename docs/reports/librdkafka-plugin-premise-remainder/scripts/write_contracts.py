#!/usr/bin/env python3
"""write_contracts.py [--check] -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: add
`requires resource_premise(rk_no_plugin_library, "...")` (the statement byte
for byte as client.qv's 89 declarations state it) to

* every `nonnull_member_stable` / `nonnull_member_scope` declaration of
  librdkafka's null_deref.qv the member census premises
  (artifacts/member-census.json, `premise`), after its member clause;
* memory.qv's rd_kafka_crash and kafka_mem.qv's rd_kafka_dump0 (user decision
  2, "Yes, list it"), after the selector;
* queue_destructor.qv's model contract rd_kafka_q_destroy_final (user decision
  3, "Add premise support"), as its first clause;

with one comment block per module. Idempotent; --check verifies each named
declaration states the premise exactly once and no other declaration of
null_deref.qv states it."""
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
RK = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka"
CENSUS = HERE.parent / "artifacts/member-census.json"
NAME = "rk_no_plugin_library"


def statement():
    text = (RK / "client.qv").read_text()
    found = set(re.findall(r'requires resource_premise\(rk_no_plugin_library, "([^"]*)"\);', text))
    assert len(found) == 1, found
    return found.pop()


CLAUSE = f'requires resource_premise({NAME}, "{statement()}");'

NULL_NOTE = """// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Only
// nonnull_member* need it"; contract specification C09.3, gateway rules 14x,
// 14z): a stable member's and a function-scope entry's fact is checked
// against every store the analysed units make, so a store made by code
// outside the run -- a callback or interceptor a plugin library registers
// (`plugin.library.paths`) -- could falsify it unchecked. Each declaration
// below whose member a callback outside the run could reach names
// P-RK-NO-PLUGIN-LIBRARY (`requires resource_premise(rk_no_plugin_library,
// ...)`, admitted only beside these two forms), and every null row resting on
// the fact lists it. The rule and its census are in
// docs/reports/librdkafka-plugin-premise-remainder/ (`member_reach.py`):
// rd_kafka_broker_s.rkb_rk is stored by rd_kafka_broker_add, which the public
// rd_kafka_new reaches, so a plugin's callback may run a store to it; all 67
// declarations of the member carry the premise. A `quant null-deref` fact does
// not depend on which callback runs and carries none.
"""

CRASH_NOTE = """// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Yes,
// list it"): with a non-null rk the assertion-failure dump (rd_kafka_dump0)
// drops queue references that may run an interceptor or rk_conf.log_cb, so the
// declaration names P-RK-NO-PLUGIN-LIBRARY (re-pinned at 66910c10 and
// 046accf6).
"""

DUMP_NOTE = """// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Yes,
// list it"): the dump's queue-length reads (rd_kafka_q_len) take and drop a
// forwarded queue's reference, whose final drop may run an interceptor or
// rk_conf.log_cb, so the declaration names P-RK-NO-PLUGIN-LIBRARY (re-pinned
// at 66910c10 and 046accf6).
"""

MODEL_NOTE = """// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Add
// premise support"): a Quirin model contract states a named premise in the
// contract format every module parses through (`requires
// resource_premise(NAME, "STATEMENT");`, total-or-reject at parse; read by
// `qcore_contract::contract_loader::module_premises`). The cascade
// rd_kafka_q_purge0 -> rd_kafka_op_destroy -> ... -> rd_kafka_log_buf may run
// rk_conf.log_cb, so the destructor names P-RK-NO-PLUGIN-LIBRARY.
"""


def premised_functions():
    census = json.loads(CENSUS.read_text())
    return sorted({r["function"] for r in census["rows"] if r["premise"]})


def add_after(text, function, anchor_re, note=None):
    """Insert CLAUSE after the first line matching anchor_re inside
    function's contract block; insert note (once) before its `extern fn`."""
    m = re.search(r"(?m)^extern fn " + re.escape(function) + r"\(.*?\n\};\n", text, re.S)
    assert m, function
    block = m.group(0)
    if CLAUSE in block:
        return text
    lines = block.split("\n")
    for i, line in enumerate(lines):
        if re.search(anchor_re, line):
            indent = re.match(r"\s*", line).group(0)
            if line.startswith("contract {"):
                indent = "    "
            lines.insert(i + 1, indent + CLAUSE)
            break
    else:
        raise SystemExit(f"{function}: no anchor {anchor_re}")
    new = "\n".join(lines)
    if note:
        new = note + new
    return text[:m.start()] + new + text[m.end():]


def main():
    check = "--check" in sys.argv
    functions = premised_functions()
    assert len(functions) == 67, len(functions)
    edits = {
        "null_deref.qv": [(f, r"requires nonnull_member_(stable|scope)\(", None) for f in functions],
        "memory.qv": [("rd_kafka_crash", r"requires resource_contract_v1;", CRASH_NOTE)],
        "kafka_mem.qv": [("rd_kafka_dump0", r"requires resource_contract_v1;", DUMP_NOTE)],
        "queue_destructor.qv": [("rd_kafka_q_destroy_final", r"^contract \{", MODEL_NOTE)],
    }
    if check:
        bad = 0
        for module, items in edits.items():
            text = (RK / module).read_text()
            for function, _, _ in items:
                m = re.search(r"(?m)^extern fn " + re.escape(function) + r"\(.*?\n\};\n", text, re.S)
                if not m or m.group(0).count(CLAUSE) != 1:
                    print("missing or repeated:", module, function)
                    bad += 1
        text = (RK / "null_deref.qv").read_text()
        total = text.count(CLAUSE)
        if total != len(functions):
            print("null_deref.qv states the premise", total, "times, expected", len(functions))
            bad += 1
        print("check:", "ok" if not bad else f"{bad} problem(s)")
        sys.exit(1 if bad else 0)
    for module, items in edits.items():
        path = RK / module
        text = path.read_text()
        before = text
        for function, anchor, note in items:
            text = add_after(text, function, anchor, note)
        if module == "null_deref.qv" and NULL_NOTE not in text:
            marker = "// Card NULL-MEMBER-INVARIANT-SCOPE (contract specification C09.3, grammar\n"
            assert text.count(marker) == 1
            text = text.replace(marker, NULL_NOTE + "//\n" + marker)
        if text != before:
            path.write_text(text)
            print("wrote", module)


main()
