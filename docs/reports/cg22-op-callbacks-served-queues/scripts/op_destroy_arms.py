#!/usr/bin/env python3
"""op_destroy_arms.py OUT.json -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: read
`rd_kafka_op_destroy`'s body from Clang's JSON AST of `rdkafka_op.c` (the
compile command of the audited build, run read-only on the audited tree), and
print, per case value, the payload paths (member paths of `rko` under
`rko_u`, through `&`, casts and subscripts) the case group hands to a call:
the same reading rule 12o's check makes (`arm_release_check.rs`), used to
write the declaration's arms, which the corpus run's 12o check then validates
or withdraws. Writes OUT.json; prints the `resource_arm_*` clauses with each
value's enumerator name (from the AST) for the source comment."""
import json
import pathlib
import shlex
import subprocess
import sys

DATABASE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production/compile_commands.json")
UNIT = "rdkafka_op.c"
FUNCTION = "rd_kafka_op_destroy"
ROOTS = {"rko_u"}


def command():
    for entry in json.loads(DATABASE.read_text()):
        if entry["file"].endswith("/src/" + UNIT) and "librdkafka" in entry["file"]:
            arguments = entry.get("arguments") or shlex.split(entry["command"])
            return entry["directory"], entry["file"], arguments
    raise SystemExit("no compile command for " + UNIT)


def ast():
    directory, file, arguments = command()
    keep = []
    skip = False
    for argument in arguments[1:]:
        if skip:
            skip = False
            continue
        if argument in ("-o", "-c"):
            skip = argument == "-o"
            continue
        if argument == file or argument.endswith("/" + UNIT):
            continue
        keep.append(argument)
    clang = "/usr/lib/llvm-21/bin/clang"
    out = subprocess.run(
        [clang, *keep, "-fsyntax-only", "-Xclang", "-ast-dump=json",
         "-Xclang", "-ast-dump-filter=" + FUNCTION, file],
        cwd=directory, capture_output=True, text=True, check=False)
    decoder = json.JSONDecoder()
    text, at, found = out.stdout, 0, []
    while at < len(text):
        while at < len(text) and text[at].isspace():
            at += 1
        if at >= len(text):
            break
        node, at = decoder.raw_decode(text, at)
        found.append(node)
    for node in found:
        if node.get("kind") == "FunctionDecl" and node.get("name") == FUNCTION and any(
                c.get("kind") == "CompoundStmt" for c in node.get("inner", [])):
            return node
    raise SystemExit("no body: " + out.stderr[-2000:])


def enumerators():
    """RD_KAFKA_OP_* -> value, from the enum in rdkafka_op.h (implicit values
    counted on, explicit ones read)."""
    import re
    root = pathlib.Path(command()[1]).parent
    text = (root / "rdkafka_op.h").read_text(errors="replace")
    start = text.index("RD_KAFKA_OP_NONE")
    body = text[start:text.index("} rd_kafka_op_type_t", start)]
    body = re.sub(r"/\*.*?\*/", "", body, flags=re.S)
    body = re.sub(r"//[^\n]*", "", body)
    values, value = {}, -1
    for item in body.split(","):
        item = item.strip()
        if not item:
            continue
        name, _, init = item.partition("=")
        name = name.strip()
        value = int(eval(init.strip())) if init.strip() else value + 1
        values[name] = value
    return values


def kids(node):
    return node.get("inner", [])


def strip(node):
    while node.get("kind") in ("ParenExpr", "ImplicitCastExpr", "CStyleCastExpr", "ConstantExpr") and kids(node):
        node = kids(node)[0]
    return node


def path_of(node, parameter):
    segments = []
    node = strip(node)
    while node.get("kind") == "MemberExpr":
        segments.append(node["name"])
        base = strip(kids(node)[0])
        if node.get("isArrow"):
            if base.get("kind") == "DeclRefExpr" and base["referencedDecl"]["id"] == parameter:
                return ".".join(reversed(segments))
            return None
        node = base
    return None


def actual_path(node, parameter):
    node = strip(node)
    while True:
        if node.get("kind") == "UnaryOperator" and node.get("opcode") == "&":
            node = strip(kids(node)[0])
        elif node.get("kind") == "ArraySubscriptExpr":
            node = strip(kids(node)[0])
        else:
            return path_of(node, parameter)


def collect(node, parameter, out):
    if node.get("kind") == "CallExpr":
        for argument in kids(node)[1:]:
            path = actual_path(argument, parameter)
            if path and path.split(".")[0] in ROOTS:
                out.add(path)
    for child in kids(node):
        collect(child, parameter, out)


def switches(node, out):
    if node.get("kind") == "SwitchStmt":
        out.append(node)
    for child in kids(node):
        switches(child, out)


def main():
    global ENUM
    ENUM = enumerators()
    function = ast()
    parameter = next(c["id"] for c in kids(function) if c.get("kind") == "ParmVarDecl")
    found = []
    switches(function, found)
    switch = found[0]
    groups, names, active = {}, {}, set()

    def label(value, name):
        active.add(value)
        groups.setdefault(value, set())
        if name:
            names[value] = name

    def statement(node):
        kind = node.get("kind")
        if kind == "CaseStmt":
            constant = kids(node)[0]
            inner = strip(constant)
            name = None
            if inner.get("kind") == "DeclRefExpr":
                name = inner["referencedDecl"].get("name")
                value = ENUM[name]
            else:
                value = int(inner["value"])
            label(value, name)
            if len(kids(node)) > 1:
                statement(kids(node)[1])
        elif kind == "DefaultStmt":
            label("default", None)
            if kids(node):
                statement(kids(node)[0])
        elif kind == "BreakStmt":
            active.clear()
        elif kind == "CompoundStmt":
            for child in kids(node):
                statement(child)
        else:
            found = set()
            collect(node, parameter, found)
            for value in active:
                groups[value] |= found
            if kind == "ReturnStmt":
                active.clear()

    statement(kids(switch)[-1])
    record = {str(k): sorted(v) for k, v in groups.items()}
    pathlib.Path(sys.argv[1]).write_text(json.dumps({"groups": record, "names": {str(k): v for k, v in names.items()}}, indent=1) + "\n")
    for value in sorted(v for v in groups if v != "default"):
        paths = sorted(groups[value])
        name = names.get(value, "?")
        if paths:
            for path in paths:
                print(f"    // {name} = {value}")
                print(f"    ensures normal resource_arm_released(rko, {path}) when rko_type == {value};")
        else:
            print(f"    // {name} = {value}")
            print(f"    ensures normal resource_arm_empty(rko) when rko_type == {value};")
    default = sorted(groups.get("default", set()))
    print("    default:", default)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
