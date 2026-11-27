#!/usr/bin/env python3
"""rk_conf_census.py OUT.json -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: a reading aid
for the census (never evidence for a clause: the gateway reads Clang's AST).
Over the two audited Fluent Bit trees (`66910c10`, `046accf6`), for every
function-pointer member of librdkafka's `struct rd_kafka_conf_s`
(`rdkafka_conf.h`), list:

* the dedicated setter that writes it (a `rd_kafka_conf_set_*` /
  `rd_kafka_conf_set_*_cb` body naming the property through the
  `rd_kafka_anyconf_set_internal` macro), and the property's `pdef` default in
  the `rd_kafka_properties[]` table;
* every direct store `X.MEMBER = V` / `X->MEMBER = V` in librdkafka's `src/`;
* every call of a setter in librdkafka's `src/` and in Fluent Bit's `src/` and
  `plugins/` (the registrations), with its callback actual;
* every invocation `X.MEMBER(` / `X->MEMBER(` in librdkafka's `src/`.

It also records the two facts the census turns on: whether the trees'
librdkafka sources are byte-identical, and the build's `WITH_PLUGINS`.
Prints a table and writes OUT.json."""
import hashlib
import json
import pathlib
import re
import sys

TREES = {
    "66910c10": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"),
    "046accf6": pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
}
CONFIG = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production/lib/librdkafka-2.15.0/generated/config.h")
SRC = "lib/librdkafka-2.15.0/src"
FB = ["src", "plugins"]

FNPTR = re.compile(r"\(\s*\*\s*(\w+)\s*\)\s*\(")
SETTER = re.compile(r"(?:void|rd_kafka_conf_res_t)\s+(rd_kafka_conf_set_\w+)\s*\(([^{;]*?)\)\s*\{(.*?)\n\}", re.S)
INTERNAL = re.compile(r"rd_kafka_anyconf_set_internal\(\s*_RK_GLOBAL\s*,\s*(?:\w+|&\w+->\w+)\s*,\s*\"([\w\.]+)\"\s*,\s*(\w+)\s*\)")
PROPERTY = re.compile(r"\{\s*_RK_GLOBAL[^,]*,\s*\"([\w\.]+)\"\s*,\s*_RK_C_PTR\s*,\s*_RK\(([\w\.]+)\)(.*?)\}\s*,", re.S)
PDEF = re.compile(r"\.pdef\s*=\s*(?:#ifdef[^\n]*\n\s*)?(\w+)")


def members(text):
    """Function-pointer members of struct rd_kafka_conf_s, with their path."""
    start = text.index("struct rd_kafka_conf_s {")
    end = text.index("\n};", start)
    body = text[start:end]
    # Each open anonymous record collects the paths declared inside it; its
    # closing `} name;` prefixes them with `name.`.
    stack = [[]]
    for line in body.splitlines()[1:]:
        stripped = line.strip()
        if re.match(r"(struct|union)\s*\{$", stripped):
            stack.append([])
            continue
        closing = re.match(r"\}\s*(\w+)\s*;", stripped)
        if closing and len(stack) > 1:
            inner = stack.pop()
            stack[-1].extend("%s.%s" % (closing.group(1), path) for path in inner)
            continue
        match = FNPTR.search(stripped)
        if match:
            stack[-1].append(match.group(1))
    return stack[0]


def read(root, rel):
    return (root / rel).read_text(errors="replace")


def census(root):
    src = root / SRC
    conf_h = read(root, SRC + "/rdkafka_conf.h")
    conf_c = read(root, SRC + "/rdkafka_conf.c")
    record = {"members": {}}
    paths = members(conf_h)
    setters = {}
    for match in SETTER.finditer(conf_c):
        for prop, value in INTERNAL.findall(match.group(3)):
            setters.setdefault(prop, []).append(match.group(1))
    properties = {}
    for match in PROPERTY.finditer(conf_c):
        pdef = PDEF.search(match.group(3))
        properties[match.group(2)] = {"property": match.group(1),
                                      "pdef": pdef.group(1) if pdef else None}
    lib_files = sorted(src.glob("*.c")) + sorted(src.glob("*.h"))
    fb_files = [p for d in FB for p in sorted((root / d).rglob("*.c"))]
    for path in paths:
        leaf = path.split(".")[-1]
        prop = properties.get(path, {})
        entry = {
            "property": prop.get("property"),
            "pdef": prop.get("pdef"),
            "setters": sorted(set(setters.get(prop.get("property"), []))) if prop else [],
            "direct_stores": [],
            "registrations": [],
            "invocations": [],
        }
        store = re.compile(r"(?:\.|->)%s\s*=(?!=)\s*([^;]+);" % re.escape(leaf))
        invoke = re.compile(r"(?:\.|->)%s\s*\(" % re.escape(leaf))
        for file in lib_files:
            for number, line in enumerate(file.read_text(errors="replace").splitlines(), 1):
                rel = "%s:%d" % (file.relative_to(root), number)
                for value in store.findall(line):
                    entry["direct_stores"].append({"at": rel, "value": value.strip()})
                if invoke.search(line) and "(*" not in line:
                    entry["invocations"].append(rel)
        for setter in entry["setters"]:
            call = re.compile(r"\b%s\s*\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)\s*;" % re.escape(setter))
            for file in [f for f in lib_files if f.suffix == ".c"] + fb_files:
                text = file.read_text(errors="replace")
                for match in call.finditer(text):
                    line_start = text.rfind("\n", 0, match.start()) + 1
                    if re.match(r"\s*(?:RD_EXPORT\s+)?(?:void|rd_kafka_conf_res_t)\b",
                                text[line_start:match.start()]):
                        continue  # a definition or prototype, not a call
                    number = text.count("\n", 0, match.start()) + 1
                    entry["registrations"].append({
                        "at": "%s:%d" % (file.relative_to(root), number),
                        "setter": setter, "conf": match.group(1), "callback": match.group(2)})
        record["members"][path] = entry
    digest = hashlib.sha256()
    for file in sorted(src.glob("*.[ch]")):
        digest.update(file.name.encode())
        digest.update(file.read_bytes())
    record["librdkafka_src_sha256"] = digest.hexdigest()
    return record


def main():
    out = {rev: census(root) for rev, root in TREES.items()}
    config = CONFIG.read_text()
    out["build"] = {line.split()[1]: line.split()[2] for line in config.splitlines()
                    if line.startswith("#define WITH_PLUGINS") or line.startswith("#define WITH_LIBDL")}
    out["identical_librdkafka"] = (out["66910c10"]["librdkafka_src_sha256"]
                                   == out["046accf6"]["librdkafka_src_sha256"])
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    rev = out["66910c10"]
    print("librdkafka identical at both revisions:", out["identical_librdkafka"], out["build"])
    for path, entry in rev["members"].items():
        fb = [r for r in entry["registrations"] if not r["at"].startswith(SRC)]
        lib = [r for r in entry["registrations"] if r["at"].startswith(SRC)]
        print("%-34s pdef=%-26s setters=%d fb_reg=%d lib_reg=%d stores=%d invocations=%d" % (
            path, entry["pdef"], len(entry["setters"]), len(fb), len(lib),
            len(entry["direct_stores"]), len(entry["invocations"])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
