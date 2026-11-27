#!/usr/bin/env python3
"""arm_sites.py DATABASE OUT.json UNIT... -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION:
size the arm-by-discriminant route (brief option 1). In each unit (a path
suffix of a compile-database entry), Clang's JSON AST of its own recorded
command is walked for every expression reaching into `rd_kafka_op_s.rko_u`
that can put a function-pointer value into the union's bytes:

* an assignment whose target is a member of an arm that is a function
  pointer, or a record by value that holds one (`rd_list_s`,
  `rd_kafka_timer_s`), or a whole arm;
* an address taken (`&`) of such a member (a callee may store through it);
* a `memcpy`-family destination inside the union;

and every read of `rko_u.leaders.cb`. For each it records the arm, the
function and the local evidence of the op's type a discriminant check could
use: a `switch` on `->rko_type` enclosing it (its case labels), an
`rd_kafka_op_new*` call with a constant type assigned to the same variable
earlier in the function, or an `RD_KAFKA_OP_TYPE_ASSERT` (`__assert_fail`
on `rko_type`) earlier in the function. Headers' inline functions are
deduplicated by file and line. A sizing aid only; no verdict rests on it."""
import json
import shlex
import subprocess
import sys

COPIERS = {"memcpy", "memmove", "bcopy", "__builtin_memcpy", "__builtin_memmove",
           "__builtin___memcpy_chk"}
FP_RECORDS = ("struct rd_list_s", "struct rd_kafka_timer_s")
CONSTRUCTORS = ("rd_kafka_op_new0", "rd_kafka_op_new_cb", "rd_kafka_admin_request_op_new",
                "rd_kafka_admin_fanout_op_new", "rd_kafka_op_new_reply", "rd_kafka_op_new_fetch_msg",
                "rd_kafka_op_new_ctrl_msg", "rd_kafka_admin_result_new")


def command_for(entry):
    words = shlex.split(entry["command"])
    out, skip = [], False
    for word in words:
        if skip:
            skip = False
            continue
        if word == "-o":
            skip = True
            continue
        if word == "-c":
            continue
        out.append(word)
    return out + ["-fsyntax-only", "-Xclang", "-ast-dump=json"]


def qual(node):
    t = node.get("type", {})
    return t.get("desugaredQualType") or t.get("qualType") or ""


def strip(node):
    while node.get("kind") in ("ParenExpr", "ImplicitCastExpr") and node.get("inner"):
        node = node["inner"][0]
    return node


def chain(node):
    """(base spelling, [segments]) of a member chain, or None."""
    node = strip(node)
    segments = []
    while node.get("kind") == "MemberExpr":
        segments.append(node.get("name"))
        node = strip(node["inner"][0])
    segments.reverse()
    if node.get("kind") == "DeclRefExpr":
        base = node.get("referencedDecl", {}).get("name")
    else:
        base = f"<{node.get('kind')}>"
    return base, segments


def arm_of(segments):
    if "rko_u" in segments:
        i = segments.index("rko_u")
        return segments[i + 1:] if i + 1 < len(segments) else ["*"]
    return None


def fp_relevant(node):
    text = qual(strip(node))
    return "(*)" in text or text.endswith("_cb_t *") or any(text.startswith(r) for r in FP_RECORDS) \
        or text.startswith("struct rd_kafka_op_s::")


def first_int(node):
    node = strip(node)
    if node.get("kind") == "IntegerLiteral":
        return node.get("value")
    if node.get("kind") == "DeclRefExpr" and node.get("referencedDecl", {}).get("kind") == "EnumConstantDecl":
        return node["referencedDecl"].get("name")
    if node.get("kind") == "BinaryOperator":
        return "|".join(str(first_int(child)) for child in node.get("inner", []))
    if node.get("kind") == "ConstantExpr" and node.get("inner"):
        return first_int(node["inner"][0])
    return None


class Walker:
    def __init__(self):
        self.file = None
        self.line = None
        self.function = None
        self.constructed = {}
        self.asserted = False
        self.switches = []
        self.sites = []

    def loc(self, node):
        for key in ("loc", "range"):
            loc = node.get(key, {})
            if key == "range":
                loc = loc.get("begin", {})
            loc = loc.get("expansionLoc", loc)
            if "file" in loc:
                self.file = loc["file"]
            if "line" in loc:
                self.line = loc["line"]

    def site(self, what, target):
        base, segments = chain(target)
        self.sites.append({
            "what": what,
            "file": (self.file or "").split("/")[-1],
            "line": self.line,
            "function": self.function,
            "base": base,
            "arm": ".".join(arm_of(segments) or []),
            "type": qual(strip(target)),
            "evidence": {
                "constructed": self.constructed.get(base),
                "switch_cases": [list(cases) for cases in self.switches] or None,
                "asserted": self.asserted,
            },
        })

    def walk(self, node, in_switch_cases=None):
        if not isinstance(node, dict):
            return
        self.loc(node)
        kind = node.get("kind")
        if kind == "FunctionDecl" and any(c.get("kind") == "CompoundStmt" for c in node.get("inner", [])):
            self.function = node.get("name")
            self.constructed = {}
            self.asserted = False
        if kind == "BinaryOperator" and node.get("opcode") == "=":
            left, right = node["inner"]
            base, segments = chain(left)
            arm = arm_of(segments)
            if arm is not None and fp_relevant(left):
                self.site("assignment", left)
            rs = strip(right)
            if rs.get("kind") == "CallExpr" and strip(left).get("kind") == "DeclRefExpr":
                callee = strip(rs["inner"][0]).get("referencedDecl", {}).get("name")
                if callee in CONSTRUCTORS:
                    args = rs["inner"][1:]
                    pick = {"rd_kafka_op_new0": 1, "rd_kafka_op_new_cb": 1,
                            "rd_kafka_admin_request_op_new": 1}.get(callee)
                    value = first_int(args[pick]) if pick is not None and len(args) > pick else None
                    self.constructed[chain(left)[0]] = f"{callee}({value})"
        if kind == "UnaryOperator" and node.get("opcode") == "&":
            base, segments = chain(node["inner"][0])
            if arm_of(segments) is not None and fp_relevant(node["inner"][0]):
                self.site("address", node["inner"][0])
        if kind == "MemberExpr" and node.get("name") == "cb":
            base, segments = chain(node)
            if arm_of(segments) == ["leaders", "cb"]:
                self.site("leaders.cb reference", node)
        if kind == "CallExpr":
            inner = node.get("inner", [])
            callee = strip(inner[0]).get("referencedDecl", {}).get("name") if inner else None
            if callee in COPIERS and len(inner) > 1:
                operand = strip(inner[1])
                if operand.get("kind") == "UnaryOperator":
                    base, segments = chain(operand["inner"][0])
                    if arm_of(segments) is not None:
                        self.site(f"{callee} destination", operand["inner"][0])
            if callee == "__assert_fail":
                self.asserted = True
        if kind == "SwitchStmt":
            condition = node["inner"][0]
            text = json.dumps(condition)
            if '"rko_type"' in text:
                cases = set()
                body = node["inner"][-1]

                def labels(n):
                    if not isinstance(n, dict):
                        return
                    if n.get("kind") == "CaseStmt":
                        cases.add(str(first_int(n["inner"][0])))
                    for c in n.get("inner", []):
                        labels(c)
                labels(body)
                self.switches.append(tuple(sorted(cases)))
                for child in node.get("inner", []):
                    self.walk(child)
                self.switches.pop()
                return
        for child in node.get("inner", []):
            self.walk(child)


def main():
    database, out, *units = sys.argv[1:]
    entries = json.load(open(database))
    seen = set()
    result = []
    for unit in units:
        entry = next(e for e in entries if e["file"].endswith(unit))
        proc = subprocess.run(command_for(entry), cwd=entry["directory"], capture_output=True)
        if proc.returncode != 0:
            result.append({"unit": unit, "error": proc.stderr.decode(errors="replace")[:500]})
            continue
        walker = Walker()
        walker.walk(json.loads(proc.stdout))
        for site in walker.sites:
            key = (site["file"], site["line"], site["what"], site["arm"])
            if key in seen:
                continue
            seen.add(key)
            site["unit"] = unit
            result.append(site)
    json.dump(result, open(out, "w"), indent=1)


if __name__ == "__main__":
    main()
