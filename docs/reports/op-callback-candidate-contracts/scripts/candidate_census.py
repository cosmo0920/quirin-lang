#!/usr/bin/env python3
"""candidate_census.py OUT.json -- OP-CALLBACK-CANDIDATE-CONTRACTS: a reading
aid for the census of the op-callback candidates (never evidence for a
clause: every contract is written from the body read in full, and the
candidate sets are Clang's closure, rule 12n). For each candidate of
`rd_kafka_op_s.rko_op_cb` (27) and the three non-poll candidates of
`rd_kafka_op_s.rko_serve`, over librdkafka's sources in the two audited
Fluent Bit trees:

* the definition's file and line, and whether the two texts are
  byte-identical;
* the `RD_KAFKA_OP_RES_*` values its body returns literally, and the calls
  it returns the value of (`return f(...)`), followed one level;
* whether the first statement that reads `rko_err` tests
  `RD_KAFKA_RESP_ERR__DESTROY` before any other effect (`destroy_test_early`);
* direct reads of an application callback (`rk_conf.<...>_cb`) and calls
  through a member function pointer (`->X_cb(`, `.cb(`);
* every call the body hands the op formal to (`f(..., rko, ...)`);
* a textual over-approximate call graph over librdkafka's definitions, and
  which application-callback call sites it can reach (`app_reach`): the
  sites are the functions whose body calls `rk_conf.<...>_cb(` or
  `conf->log_cb(`; the log sink `rd_kafka_log_buf` is reported apart
  (`log_reach`).

The textual call graph matches identifiers followed by `(`; macros, function
pointers and name collisions make it an over-approximation in both
directions, so every reported reach is confirmed or refuted by reading."""
import json
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
}
SRC = "lib/librdkafka-2.15.0/src"

OP_CB = [
    "rd_kafka_DeleteRecords_leaders_queried_cb",
    "rd_kafka_ListOffsets_leaders_queried_cb",
    "rd_kafka_admin_fanout_worker",
    "rd_kafka_admin_worker",
    "rd_kafka_broker_monitor_op_cb",
    "rd_kafka_cgrp_assignor_handle_Metadata_op",
    "rd_kafka_cgrp_consumer_handle_Metadata_op",
    "rd_kafka_cgrp_handle_Metadata_op",
    "rd_kafka_oauthbearer_refresh_op",
    "rd_kafka_offset_reset_op_cb",
    "rd_kafka_offset_validate_op_cb",
    "rd_kafka_share_ack_commit_cb_register_op",
    "rd_kafka_share_commit_async_fanout_op",
    "rd_kafka_share_commit_sync_fanout_op",
    "rd_kafka_share_fetch_fanout_op",
    "rd_kafka_share_fetch_reply_op_cb",
    "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb",
    "rd_kafka_txn_op_abort_transaction",
    "rd_kafka_txn_op_abort_transaction_ack",
    "rd_kafka_txn_op_ack_init_transactions",
    "rd_kafka_txn_op_begin_abort",
    "rd_kafka_txn_op_begin_commit",
    "rd_kafka_txn_op_begin_transaction",
    "rd_kafka_txn_op_commit_transaction",
    "rd_kafka_txn_op_commit_transaction_ack",
    "rd_kafka_txn_op_init_transactions",
    "rd_kafka_txn_op_send_offsets_to_transaction",
]
SERVE = ["rd_kafka_cgrp_op_serve", "rd_kafka_mock_cluster_op_serve", "rd_kafka_toppar_op_serve"]

KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "do", "else", "case",
            "defined", "__attribute__", "__builtin_expect", "likely", "unlikely"}
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
DEF = re.compile(r"^(?:static\s+|RD_INLINE\s+|inline\s+|RD_UNUSED\s+)*[A-Za-z_][\w \t\*]*?\b([A-Za-z_]\w*)\s*\(([^;{]*?)\)\s*(?:RD_[A-Z_]+\s*)*\{", re.M | re.S)
APP_CB = re.compile(r"(?:rk_conf|conf)(?:\.|->)(?:[\w]+\.)*(\w*_cb)\s*\(")
MEMBER_CALL = re.compile(r"(?:->|\.)(\w*cb)\s*\(")
RETURN_RES = re.compile(r"return\s+(RD_KAFKA_OP_RES_\w+)")
RETURN_CALL = re.compile(r"return\s+([A-Za-z_]\w*)\s*\(")


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def strip_strings(text):
    text = re.sub(r'"(?:\\.|[^"\\\n])*"', '""', text)
    return re.sub(r"'(?:\\.|[^'\\\n])'", "' '", text)


MACROS = {}
DEFINE = re.compile(r"^[ \t]*#[ \t]*define[ \t]+([A-Za-z_]\w*)\(([^)]*)\)(.*)$")


def definitions(root):
    """name -> (file, line, body text) over SRC's .c and .h files: a
    top-level `NAME ( ... )` followed, after attributes, by `{` (the
    preprocessor lines blanked, comments and strings stripped)."""
    out = {}
    for path in sorted((root / SRC).glob("*.[ch]")):
        text = strip_comments(path.read_text(errors="replace"))
        # Join continuation lines (keeping the line count), record each
        # function-like macro's callees, then blank every preprocessor line.
        lines = text.split("\n")
        joined = []
        i = 0
        while i < len(lines):
            line, extra = lines[i], 0
            while line.endswith("\\") and i + 1 < len(lines):
                i += 1
                extra += 1
                line = line[:-1] + " " + lines[i]
            joined.append(line)
            joined.extend([""] * extra)
            i += 1
        kept = []
        for line in joined:
            if line.lstrip().startswith("#"):
                m = DEFINE.match(line)
                if m:
                    MACROS.setdefault(m.group(1), callees("{" + strip_strings(m.group(3))))
                kept.append("")
            else:
                kept.append(line)
        clean = strip_strings("\n".join(kept)).split("\n")
        # librdkafka is clang-formatted: a definition's signature starts in
        # column 0 and its body ends at the first line that is exactly `}`.
        i = 0
        while i < len(clean):
            line = clean[i]
            if not line or line[0] in " \t}/*{" or line.startswith(("typedef", "struct", "union", "enum")) and "(" not in line:
                i += 1
                continue
            j, signature = i, ""
            while j < len(clean) and j < i + 15:
                signature += clean[j] + "\n"
                if signature.count("(") and signature.count("(") == signature.count(")") and re.search(r"\)\s*(?:RD_[A-Z_]+\s*)*\{\s*$", signature):
                    break
                if clean[j].rstrip().endswith(";"):
                    j = -1
                    break
                j += 1
            if j < 0 or j >= len(clean) or j >= i + 15:
                i += 1
                continue
            head = signature[:signature.index("(")]
            names = re.findall(r"[A-Za-z_]\w*", head)
            if not names or names[-1] in KEYWORDS:
                i = j + 1
                continue
            e = j + 1
            while e < len(clean) and clean[e] != "}":
                e += 1
            out.setdefault(names[-1], (path.name, i + 1, "\n".join(clean[i:e + 1])))
            i = e + 1
    return out


def callees(body):
    head = body.find("{")
    return {name for name in CALL.findall(body[head:]) if name not in KEYWORDS}


def reach(graph, start, sites):
    seen, stack, hits = {start}, [start], set()
    while stack:
        name = stack.pop()
        if name in sites:
            hits.add(name)
        for callee in graph.get(name, ()):
            if callee not in seen:
                seen.add(callee)
                stack.append(callee)
    return hits


def formal_op(body):
    """The op formal's name: the third parameter of an op callback."""
    params = body[body.find("(") + 1: body.find(")")]
    names = [re.findall(r"(\w+)\s*$", p.strip()) for p in params.split(",")]
    names = [n[0] for n in names if n]
    return names[2] if len(names) >= 3 else None


def destroy_test_early(body, op):
    head = body[body.find("{") + 1:]
    first = re.search(r"%s->rko_err" % re.escape(op), head) if op else None
    if not first:
        return False
    before = head[:first.start()]
    test = head[first.start():first.start() + 80]
    calls = [c for c in CALL.findall(before) if c not in KEYWORDS and not c.startswith(("RD_KAFKA_OP_TYPE_ASSERT", "rd_assert", "rd_dassert"))]
    return "RD_KAFKA_RESP_ERR__DESTROY" in test and not calls


def census(root):
    defs = definitions(root)
    graph = {name: callees(body) for name, (_, _, body) in defs.items()}
    for name, called in MACROS.items():
        graph.setdefault(name, called)
    sites = {name for name, (_, _, body) in defs.items() if APP_CB.search(body) and name != "rd_kafka_log_buf"}
    rows = {}
    for name in OP_CB + SERVE:
        if name not in defs:
            rows[name] = {"defined": None}
            continue
        path, line, body = defs[name]
        op = formal_op(body)
        forwarded = sorted(set(RETURN_CALL.findall(body)) - KEYWORDS)
        returns = set(RETURN_RES.findall(body))
        for callee in forwarded:
            if callee in defs:
                returns |= set(RETURN_RES.findall(defs[callee][2]))
        handed = sorted({m.group(1) for m in re.finditer(r"\b(\w+)\s*\([^;]*?\b%s\b[^;]*?\)" % re.escape(op or "rko"), body[body.find("{"):])} - KEYWORDS) if op else []
        app = reach(graph, name, sites)
        rows[name] = {
            "member": "rko_serve" if name in SERVE else "rko_op_cb",
            "defined": "%s:%d" % (path, line),
            "lines": body.count("\n") + 1,
            "op_formal": op,
            "returns": sorted(returns),
            "returns_forwarded_from": forwarded,
            "destroy_test_early": destroy_test_early(body, op),
            "direct_app_callbacks": sorted(set(APP_CB.findall(body))),
            "member_callback_calls": sorted(set(MEMBER_CALL.findall(body))),
            "op_handed_to": handed,
            "app_reach": sorted(app),
            "log_reach": "rd_kafka_log_buf" in reach(graph, name, {"rd_kafka_log_buf"}),
            "body": body,
        }
    return rows


def main():
    out = {}
    for rev, root in TREES.items():
        out[rev] = census(root)
    for name, row in out["66910c10"].items():
        twin = out["046accf6"].get(name, {})
        row["identical_at_046accf6"] = row.get("body") is not None and row.get("body") == twin.get("body")
    for rev in out:
        for row in out[rev].values():
            row.pop("body", None)
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    for name, row in out["66910c10"].items():
        print("%-62s %-9s %-34s early=%-5s app=%s log=%s same=%s" % (
            name, row.get("member"), ",".join(r.replace("RD_KAFKA_OP_RES_", "") for r in row.get("returns", [])),
            row.get("destroy_test_early"), ",".join(row.get("direct_app_callbacks", []) + row.get("member_callback_calls", [])) or "-",
            row.get("log_reach"), row.get("identical_at_046accf6")))
        if row.get("app_reach"):
            print("    app_reach:", ", ".join(row["app_reach"]))


if __name__ == "__main__":
    main()
