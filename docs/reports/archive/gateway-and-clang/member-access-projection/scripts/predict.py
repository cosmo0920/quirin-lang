#!/usr/bin/env python3
"""MEMBER-ACCESS-PROJECTION prediction: rows whose indirect-access leaves sit
on uninitialized declarations (origin line, the decl_count.py shape), and the
rows whose every diagnostic is such a leaf.

usage: predict.py REPORT CHECKOUT_ROOT
"""
import collections, json, pathlib, re, sys

IND = "indirect memory access is outside the resource projection"
DECL = re.compile(r"^(?:const\s+|unsigned\s+|signed\s+|struct\s+|enum\s+|union\s+)*[A-Za-z_]\w*(?:\s+[A-Za-z_]\w*)?[\s\*]+[A-Za-z_]\w*\s*(\[|,|;)")
KEYWORD = re.compile(r"^(?:const\s+)*(?:unsigned|signed|char|short|int|long|float|double|void|_Bool|struct|union|enum)\b")


def main():
    report = json.load(open(sys.argv[1]))
    root = pathlib.Path(sys.argv[2])
    cache = {}
    index = {}
    for path in root.rglob("*.c"):
        rel = str(path.relative_to(root))
        parts = rel.split("/")
        for k in range(len(parts)):
            index.setdefault("/".join(parts[k:]), set()).add(rel)
    def resolve(name):
        found = index.get(name, set())
        return next(iter(found)) if len(found) == 1 else name
    touched, freed = [], []
    for unit in report["units"]:
        for function in unit["functions"]:
            hits, others = [], 0
            for leaf in function["diagnostics"]:
                origin = leaf.get("origin") or ""
                text = ""
                if leaf["category"] == "unsupported" and leaf["description"] == IND and ":" in origin:
                    file, number = origin.rsplit(":", 1)
                    if file not in cache:
                        try:
                            cache[file] = (root / resolve(file)).read_text(errors="replace").splitlines()
                        except OSError:
                            cache[file] = []
                    lines = cache[file]
                    if number.isdigit() and int(number) <= len(lines):
                        text = lines[int(number) - 1].strip()
                if text and DECL.match(text) and "=" not in text and "(" not in text \
                        and (not KEYWORD.match(text) or "," in text or "[" in text):
                    hits.append((origin, text))
                else:
                    others += 1
            if hits:
                touched.append((unit["file"], function["function"], function["status"], hits, others))
                if others == 0 and function["status"] == "Unknown":
                    freed.append((unit["file"], function["function"]))
    print(f"rows with such leaves: {len(touched)}; rows with no other diagnostic: {len(freed)}")
    for file, name in freed:
        print("  freed:", file, name)
    for file, name, status, hits, others in touched:
        print(f"  {status:8} {file}::{name} leaves={len(hits)} others={others} | {hits[0][1][:70]}")


if __name__ == "__main__":
    main()
