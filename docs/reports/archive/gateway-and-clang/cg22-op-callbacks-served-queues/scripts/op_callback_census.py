#!/usr/bin/env python3
"""op_callback_census.py OUT.json -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: a
reading aid for the census (never evidence for a clause: the clauses' own
closure is Clang's, at implementation). Over librdkafka's sources in the two
audited Fluent Bit trees, list every store to the op-carried callback members
(`rko_op_cb`, `rko_serve`, the queue's `rkq_serve` that `rd_kafka_q_enq1`
copies into `rko_serve`, `leaders.cb`), every call of the registrar
`rd_kafka_op_new_cb` and of the `rd_kafka_txn_op_req` macro with its callback
actual, and, for each designator so found, whether its body opens with an
`rko_err == RD_KAFKA_RESP_ERR__DESTROY` test and whether it reads an
application callback from `rk->rk_conf` (`rk_conf.<x>_cb`) anywhere. Prints a
table and writes OUT.json."""
import json
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
}
SRC = "lib/librdkafka-2.15.0/src"
STORE = re.compile(r"(\w[\w\.\->]*?)(rko_op_cb|rko_serve|rkq_serve|leaders\.cb)\s*=(?!=)\s*([^;]+);")
REGISTRAR = re.compile(r"rd_kafka_op_new_cb\s*\(([^;]*?)\)\s*[;,)]", re.S)
TXN = re.compile(r"rd_kafka_txn_op_req\s*\(\s*rk\s*,\s*(\w+)")
DESIGNATOR = re.compile(r"^[A-Za-z_]\w*$")
APP_CB = re.compile(r"rk_conf\.(\w+\.)*\w*_cb\b|rk_conf\.\w+\.\w+_cb\b")


def files(root):
    return sorted((root / SRC).glob("*.[ch]"))


def body(root, name):
    """The text of NAME's definition in SRC, or None."""
    head = re.compile(r"^(?:static\s+)?(?:\w[\w\s\*]*?)\b%s\s*\(" % re.escape(name), re.M)
    for path in files(root):
        text = path.read_text(errors="replace")
        for match in head.finditer(text):
            start = match.start()
            brace = text.find("{", match.end())
            semi = text.find(";", match.end())
            if brace < 0 or (0 <= semi < brace):
                continue
            depth, i = 0, brace
            while i < len(text):
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        return path.name, text.count("\n", 0, start) + 1, text[start:i + 1]
                i += 1
    return None


def census(root):
    stores, registrations = [], []
    for path in files(root):
        text = path.read_text(errors="replace")
        for match in STORE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            stores.append({"file": path.name, "line": line, "member": match.group(2),
                           "value": " ".join(match.group(3).split())})
        for match in REGISTRAR.finditer(text):
            args = " ".join(match.group(1).split())
            if path.name == "rdkafka_op.c" and "rd_kafka_op_cb_t" in args:
                continue  # the definition
            if "rd_kafka_op_cb_t" in args:
                continue  # a prototype
            line = text.count("\n", 0, match.start()) + 1
            actual = args.rsplit(",", 1)[-1].strip()
            registrations.append({"file": path.name, "line": line, "actual": actual})
        for match in TXN.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            registrations.append({"file": path.name, "line": line, "actual": match.group(1),
                                  "via": "rd_kafka_txn_op_req"})
    return stores, registrations


def main():
    out = {}
    for rev, root in TREES.items():
        stores, registrations = census(root)
        designators = sorted({s["value"] for s in stores if DESIGNATOR.match(s["value"])
                              and s["value"] != "NULL"}
                             | {r["actual"] for r in registrations if DESIGNATOR.match(r["actual"])
                                and r["actual"] != "op_cb"})
        candidates = {}
        for name in designators:
            found = body(root, name)
            if not found:
                candidates[name] = {"defined": None}
                continue
            file, line, text = found
            first = "\n".join(text.splitlines()[:40])
            candidates[name] = {
                "defined": "%s:%d" % (file, line),
                "lines": text.count("\n") + 1,
                "destroy_test_early": bool(re.search(r"RD_KAFKA_RESP_ERR__DESTROY", first)),
                "destroy_test_anywhere": "RD_KAFKA_RESP_ERR__DESTROY" in text,
                "app_callbacks": sorted(set(m.group(0) for m in APP_CB.finditer(text))),
                "calls_op_destroy": "rd_kafka_op_destroy" in text,
            }
        out[rev] = {"stores": stores, "registrations": registrations, "candidates": candidates}
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    base = out["66910c10"]
    print("stores:")
    for s in base["stores"]:
        print("  %s:%d %s = %s" % (s["file"], s["line"], s["member"], s["value"]))
    print("registrar calls:")
    for r in base["registrations"]:
        print("  %s:%d %s%s" % (r["file"], r["line"], r["actual"], "  (" + r["via"] + ")" if "via" in r else ""))
    print("candidates:")
    for name, c in base["candidates"].items():
        print("  %-55s %s" % (name, json.dumps(c)))
    same = json.dumps(out["66910c10"], sort_keys=True) == json.dumps(out["046accf6"], sort_keys=True)
    print("identical at 046accf6:", same)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
