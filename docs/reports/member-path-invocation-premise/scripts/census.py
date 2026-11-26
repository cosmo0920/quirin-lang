#!/usr/bin/env python3
"""census.py -- MEMBER-PATH-INVOCATION-PREMISE's shipped census.

Run from the repository root: `python3 docs/reports/member-path-invocation-premise/scripts/census.py`.
Reads every shipped `.qv` under `providers/` and `contract/` and the gateway
fixture module `crates/qcore-c-gateway/tests/fixtures/contracts/member_paths.qv`,
and prints:

1. every declaration stating a `#released` or `#moved` member-path state
   (atomic or in a `{V, #kept}` set), with whether it is a hand-off candidate
   (beside `#owner |-> #callback`), a direct destroyer (`resource_released`
   on the subject formal) or another declaration;
2. every invocation clause (`resource_callback_invoked`,
   `resource_callback_invoked_or_default`, `resource_callback_table`,
   `resource_callback_member_invoked`) that hands a caller parameter at some
   position: the only clauses where rule 30 composes a candidate's state
   (`supplied_owned` and `_` hand objects that were never the caller's), and
   for a member invocation whether its member is one a stating declaration
   is a candidate of (rule 13w's `#candidates`, read here from the shipped
   `resource_callback_member_registered` clauses and stated defaults).
"""
import re
import sys
from pathlib import Path

ROOT = Path.cwd()
SOURCES = sorted(
    [p for base in ("providers", "contract") for p in (ROOT / base).rglob("*.qv")]
    + [ROOT / "crates/qcore-c-gateway/tests/fixtures/contracts/member_paths.qv"]
)

DECL = re.compile(r"^extern fn (\w+)\((.*?)\)\s*->", re.M)
STATE = re.compile(r"quant live #(\w+): (\w+)\.([\w.]+) \|-> (\{[^}]*\}|#\w+)")
INVOKE = re.compile(
    r"ensures normal resource_callback_(invoked|invoked_or_default|table|member_invoked)\(([^;]*)\);"
)
REGISTER = re.compile(r"resource_callback_member_registered\((\w+), ([\w.]+), (\w+)\)")


def declarations(text):
    starts = [m for m in DECL.finditer(text)]
    for i, m in enumerate(starts):
        end = starts[i + 1].start() if i + 1 < len(starts) else len(text)
        body = text[m.start():end]
        body = "\n".join(line for line in body.splitlines() if not line.strip().startswith("//"))
        params = [p.split(":")[0].strip() for p in m.group(2).split(",") if p.strip()]
        yield m.group(1), params, body


def main():
    stating = []
    invoking = []
    registered = {}
    for path in SOURCES:
        rel = path.relative_to(ROOT)
        text = path.read_text()
        for name, params, body in declarations(text):
            for reg in REGISTER.finditer(body):
                registered.setdefault(reg.group(2), set()).add((str(rel), name, reg.group(3)))
            states = [s for s in STATE.finditer(body) if re.search(r"#released|#moved", s.group(4))]
            if states:
                handoff = "#owner |-> #callback" in body
                formal = states[0].group(1)
                released = re.search(rf"resource_released\({formal},", body) is not None
                kind = "hand-off candidate" if handoff else ("direct destroyer" if released else "plain")
                stating.append((str(rel), name, kind, [f"{s.group(2)}.{s.group(3)} |-> {s.group(4)}" for s in states]))
            for inv in INVOKE.finditer(body):
                form, args = inv.group(1), [a.strip() for a in inv.group(2).split(",")]
                actuals = args[2:] if form == "member_invoked" else args[1:]
                handed = [(i, a) for i, a in enumerate(actuals) if a in params]
                if form == "member_invoked" and args[0] == "supplied_owned":
                    obj = "supplied_owned"
                else:
                    obj = args[0]
                if handed:
                    member = args[1] if form == "member_invoked" else None
                    invoking.append((str(rel), name, form, obj, member, handed))
    print("# 1. declarations stating #released or #moved")
    for rel, name, kind, states in stating:
        print(f"{rel}\t{name}\t{kind}\t{'; '.join(states)}")
    print(f"# {len(stating)} declarations")
    print("# 2. invocation clauses handing a caller parameter")
    for rel, name, form, obj, member, handed in invoking:
        regs = sorted(r[2] for r in registered.get(member, ())) if member else []
        print(f"{rel}\t{name}\t{form}\tobject={obj}\tmember={member}\thanded={handed}\tregistered={regs}")
    print(f"# {len(invoking)} clauses")
    return 0


if __name__ == "__main__":
    sys.exit(main())
