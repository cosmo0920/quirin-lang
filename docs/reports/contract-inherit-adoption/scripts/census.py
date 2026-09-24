#!/usr/bin/env python3
"""Census: provider contracts whose clauses equal a libc contract under
positional renaming (card CONTRACT-INHERIT-ADOPTION).

Usage: census.py REPOSITORY OUT.json [--before]

Every `extern fn NAME(PARAMS) -> RESULT contract { CLAUSES };` declaration in
every `.qcore` file under `providers/` is read (comments removed). A
declaration outside `quirin-provider-libc` *matches* a libc declaration when

* the parameter count is equal and every parameter type and the result type
  are equal as whitespace-free token strings, and
* the clause lists are equal after each parameter word is replaced by its
  position (`$0`, `$1`, ...) on both sides; clause order, introducers and every
  other word are compared exactly.

That is exactly the positional renaming `contract inherit` applies (contract
spec C15), so a match is a declaration that *could* be written as an inheriting
one. Whether the rules *admit* the conversion is a separate column:

* the declaring crate must be on the load path (`quirin-provider-*`, QI017),
* it must be allowed to include libc (libc declared as a Cargo dependency),
* the module must be a resource-v1 module (only the v1 loader has the
  reading-equality gate; CONTRACT-INHERIT-NON-V1-CONSUMERS),
* and the child's name must differ from the parent's (QI020).

A declaration already written `contract inherit` is reported as converted.
The script reads text only; it runs nothing.
"""
import json
import re
import sys
from pathlib import Path

DECL = re.compile(
    r"(?P<unsafe>unsafe\s+)?extern\s+fn\s+(?P<name>[A-Za-z_]\w*)\s*\((?P<params>[^)]*)\)\s*->\s*"
    r"(?P<result>[^{;]*?)\s*contract\s*(?:\{(?P<clauses>.*?)\}\s*;|inherit\s+(?P<parent>[A-Za-z_:\w]+)\s*;)",
    re.S,
)
WORD = re.compile(r"[A-Za-z_]\w*")
LIBC = "quirin-provider-libc"


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in text.split("\n"))


def squash(text):
    return re.sub(r"\s+", "", text)


def split_params(params):
    result = []
    depth = 0
    current = ""
    for char in params:
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
        if char == "," and depth == 0:
            result.append(current)
            current = ""
        else:
            current += char
    if current.strip():
        result.append(current)
    out = []
    for item in result:
        name, _, ty = item.partition(":")
        out.append((name.strip(), squash(ty)))
    return out


def clauses(body, names):
    items = [c.strip() for c in body.split(";") if c.strip()]
    positional = []
    for clause in items:
        def replace(match):
            word = match.group(0)
            return f"${names.index(word)}" if word in names else word
        positional.append(re.sub(r"\s+", " ", WORD.sub(replace, clause)))
    return positional


def declarations(path):
    text = strip_comments(path.read_text())
    for match in DECL.finditer(text):
        params = split_params(match.group("params"))
        names = [n for n, _ in params]
        yield {
            "name": match.group("name"),
            "unsafe": bool(match.group("unsafe")),
            "types": [t for _, t in params],
            "result": squash(match.group("result")),
            "clauses": None if match.group("clauses") is None else clauses(match.group("clauses"), names),
            "inherits": match.group("parent"),
        }


def crate_of(repository, path):
    return path.relative_to(repository / "providers").parts[0]


def libc_dependency(repository, crate):
    manifest = repository / "providers" / crate / "Cargo.toml"
    if not manifest.is_file():
        return False
    return re.search(r"^quirin-provider-libc(\.workspace)?\s*=", manifest.read_text(), re.M) is not None


def main():
    repository = Path(sys.argv[1]).resolve()
    out = Path(sys.argv[2])
    selected = json.loads((repository / "cycle19-resource-contracts.json").read_text())["modules"]
    selected_logical = set()
    for entry in selected:
        if ":" in entry:
            selected_logical.add(entry.split(":", 1)[1])
        else:
            selected_logical.add(entry.split("/contracts/", 1)[1])
    files = sorted((repository / "providers").glob("*/contracts/**/*.qcore"))
    parents = []
    candidates = []
    for path in files:
        crate = crate_of(repository, path)
        logical = path.relative_to(repository / "providers" / crate / "contracts").as_posix()
        for decl in declarations(path):
            record = dict(decl, crate=crate, module=logical)
            (parents if crate == LIBC else candidates).append(record)
    parent_keys = {}
    for parent in parents:
        if parent["clauses"] is None:
            continue
        key = json.dumps([parent["types"], parent["result"], parent["clauses"]])
        parent_keys.setdefault(key, []).append(parent)
    rows = []
    converted = []
    for decl in candidates:
        if decl["inherits"] is not None:
            converted.append({k: decl[k] for k in ("crate", "module", "name", "inherits")})
            continue
        if decl["clauses"] is None or decl["unsafe"]:
            continue
        key = json.dumps([decl["types"], decl["result"], decl["clauses"]])
        for parent in parent_keys.get(key, []):
            v1 = "resource_contract_v1" in decl["clauses"]
            on_load_path = decl["crate"].startswith("quirin-provider-")
            may_include_libc = libc_dependency(repository, decl["crate"])
            same_name = decl["name"] == parent["name"]
            refusals = []
            if not on_load_path:
                refusals.append("QI017: crate is not quirin-provider-*")
            elif not may_include_libc:
                refusals.append("QI004: crate does not declare quirin-provider-libc")
            if not v1:
                refusals.append("non-v1 module: no reading-equality gate (CONTRACT-INHERIT-NON-V1-CONSUMERS)")
            if same_name:
                refusals.append("QI020: child named like its parent")
            rows.append({
                "crate": decl["crate"],
                "module": decl["module"],
                "name": decl["name"],
                "parent": f"libc::{parent['name']}",
                "parent_module": parent["module"],
                "resource_v1": v1,
                "cycle19_selected": decl["module"] in selected_logical,
                "admitted": not refusals,
                "refusals": refusals,
            })
    result = {
        "method": __doc__.strip().split("\n\n")[1].replace("\n", " "),
        "libc_declarations": len(parents),
        "provider_declarations_outside_libc": len(candidates),
        "matches": len(rows),
        "admitted": sum(row["admitted"] for row in rows),
        "refused": sum(not row["admitted"] for row in rows),
        "already_inheriting": converted,
        "rows": rows,
    }
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: result[k] for k in ("libc_declarations", "provider_declarations_outside_libc",
                                              "matches", "admitted", "refused")}))
    for row in rows:
        print(f"  {row['crate']}:{row['module']}#{row['name']} = {row['parent']}  "
              f"{'ADMITTED' if row['admitted'] else 'refused: ' + '; '.join(row['refusals'])}")


if __name__ == "__main__":
    main()
