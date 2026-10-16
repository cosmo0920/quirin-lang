#!/usr/bin/env python3
"""write_contracts.py [--check] -- NONNULL-MEMBER-UNIFIED-QUALIFIER (USER
DECISION 2026-10-16, decision 3: "As shown above", "Migrate, one spelling",
"Yes, all kinds").

* Migrates every retired spelling in the provider contract modules, clauses
  and comments alike:
  `nonnull_member_stable(TAG, FIELD)` -> `nonnull_member(TAG, FIELD) where quant #scope [[stable]]`,
  `nonnull_member_scope(TAG, FIELD, P)` -> `nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]`.
* Adds `requires resource_premise(rk_no_plugin_library, "...")` (the statement
  byte for byte as client.qv states it) after the last plain member clause of
  librdkafka's four plain declarations that meet
  LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's reachability rule (its census,
  reproduced by scripts/census.sh), with one comment block. No Fluent Bit
  declaration gains a premise (out-of-tree plugins are OUTER-CONTRACT-DESIGN's).

Idempotent. --check verifies: no retired word left in any tracked provider
contract module; the migrated clause counts per crate; every librdkafka
declaration with a scoped or stable kind, and each of the four plain
declarations, names the premise exactly once; no Fluent Bit declaration
names one."""
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
RK = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka"
FB = REPO / "providers/quirin-provider-fluent-bit/contracts/fluent_bit"
NAME = "rk_no_plugin_library"
PLAIN = ["rd_kafka_toppar_new0", "rd_kafka_topic_new0", "rd_kafka_cgrp_new", "rd_kafka_transport_new"]
EXPECTED = {
    # crate: (stable clauses, scope clauses) migrated
    "qcore-provider-librdkafka": (1, 66),
    "quirin-provider-fluent-bit": (4, 10),
}


def statement():
    text = (RK / "client.qv").read_text()
    found = set(re.findall(r'requires resource_premise\(rk_no_plugin_library, "([^"]*)"\);', text))
    assert len(found) == 1, found
    return found.pop()


CLAUSE = f'requires resource_premise({NAME}, "{statement()}");'

STABLE = re.compile(r"nonnull_member_stable\(([^)]*)\)")
SCOPE = re.compile(r"nonnull_member_scope\(([^)]*)\)")

PLAIN_NOTE = """// Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, "Yes, all
// kinds"; contract specification C09.3, gateway rules 14t, 14x): the plugin
// premise applies to every kind of `nonnull_member` by the same reachability
// rule. A plain member's fact is checked against the analysed units' stores
// exactly as a stable member's is, so a store made by a plugin's callback
// could falsify it too. Each of the four declarations below meets
// LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's rule at 66910c10 and 046accf6 (rule
// (a): a store to the member is reachable from an external function; rule
// (b): the host body reaches an rk_conf or interceptor call;
// docs/reports/nonnull-member-unified-qualifier/artifacts/member-census-reproduced.json),
// so each names P-RK-NO-PLUGIN-LIBRARY, and every null row resting on
// `member:TAG.FIELD` lists it.
"""

FB_NOTE = """// Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, decision
// 3, "Migrate, one spelling"): the stable and scope kinds are spelled as one
// clause, `nonnull_member(TAG, FIELD) where quant #scope [[stable]]` and
// `nonnull_member(TAG, FIELD, P) where quant #scope [[scope]]`; each loads to
// the fact its retired spelling did. No declaration here names a premise.
"""

FB_ANCHOR = "// Card FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (contract specification C09.3\n"
RK_PLAIN_ANCHOR = "// rdkafka_partition.c: rd_kafka_toppar_new0 stores `rktp->rktp_rkt =\n"
RK_STALE = ("...)`, admitted only beside these two forms), and every null row resting on\n",
            "...)`, admitted beside every kind of `nonnull_member` since card\n// NONNULL-MEMBER-UNIFIED-QUALIFIER), and every null row resting on\n")


def migrate(text):
    stable = len(STABLE.findall(text))
    scope = len(SCOPE.findall(text))
    text = STABLE.sub(r"nonnull_member(\1) where quant #scope [[stable]]", text)
    text = SCOPE.sub(r"nonnull_member(\1) where quant #scope [[scope]]", text)
    return text, stable, scope


def declarations(text):
    return re.findall(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", text, re.S)


def add_plain_premises(text):
    for function in PLAIN:
        pattern = re.compile(
            r"(extern fn " + function + r"\(.*?\ncontract \{.*?)(\n\};)", re.S)
        match = pattern.search(text)
        assert match, function
        body = match.group(1)
        if NAME in body:
            continue
        last = body.rfind("    requires nonnull_member(")
        end = body.index(";", last) + 1
        body = body[:end] + "\n    " + CLAUSE + body[end:]
        text = text[: match.start()] + body + match.group(2) + text[match.end():]
    if PLAIN_NOTE not in text:
        assert RK_PLAIN_ANCHOR in text
        text = text.replace(RK_PLAIN_ANCHOR, PLAIN_NOTE + "//\n" + RK_PLAIN_ANCHOR, 1)
    return text


def write():
    rk = RK / "null_deref.qv"
    text, stable, scope = migrate(rk.read_text())
    clauses_stable = len(re.findall(r"^    requires nonnull_member\([^;]*\[\[stable\]\];", text, re.M))
    clauses_scope = len(re.findall(r"^    requires nonnull_member\([^;]*\[\[scope\]\];", text, re.M))
    text = text.replace(*RK_STALE)
    text = add_plain_premises(text)
    rk.write_text(text)
    print(f"librdkafka: {stable} stable and {scope} scope spellings rewritten; clauses {clauses_stable} / {clauses_scope}")
    fb = FB / "null_deref.qv"
    text, stable, scope = migrate(fb.read_text())
    if FB_NOTE not in text:
        assert FB_ANCHOR in text
        text = text.replace(FB_ANCHOR, FB_NOTE + "//\n" + FB_ANCHOR, 1)
    fb.write_text(text)
    print(f"fluent-bit: {stable} stable and {scope} scope spellings rewritten")


def check():
    failures = []
    tracked = subprocess.run(["git", "ls-files", "providers"], cwd=REPO, check=True,
                             capture_output=True, text=True).stdout.split()
    for path in tracked:
        if path.endswith(".qv"):
            body = (REPO / path).read_text(encoding="utf-8")
            if re.search(r"nonnull_member_(stable|scope)\(", body):
                failures.append(f"{path}: a retired spelling remains")
    crates = {
        "qcore-provider-librdkafka": RK / "null_deref.qv",
        "quirin-provider-fluent-bit": FB / "null_deref.qv",
    }
    for crate, path in crates.items():
        text = path.read_text()
        stable = len(re.findall(r"^    requires nonnull_member\([^,;]*,[^,;]*\) where quant #scope \[\[stable\]\];", text, re.M))
        scope = len(re.findall(r"^    requires nonnull_member\([^,;]*,[^,;]*,[^,;]*\) where quant #scope \[\[scope\]\];", text, re.M))
        if (stable, scope) != EXPECTED[crate]:
            failures.append(f"{crate}: {stable} stable / {scope} scope clauses, expected {EXPECTED[crate]}")
        for function, body in declarations(text):
            premises = body.count(CLAUSE)
            member = "requires nonnull_member(" in body
            scoped = "[[stable]]" in body or "[[scope]]" in body
            if crate == "qcore-provider-librdkafka" and member and (scoped or function in PLAIN):
                if premises != 1:
                    failures.append(f"{function}: {premises} premise clauses, expected 1")
            elif NAME in body:
                failures.append(f"{function} ({crate}): names the premise unexpectedly")
    for failure in failures:
        print(failure)
    print("check", "failed" if failures else "ok")
    return 1 if failures else 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        sys.exit(check())
    write()
    sys.exit(check())
