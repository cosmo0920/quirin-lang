#!/usr/bin/env python3
"""Inventory every repository link to a docs/agents/*.md#constraint-* anchor.

Usage: constraint_links.py <checkout> <output.json>

Reads tracked files only (`git ls-files`), finds markdown links and bare
repository paths that name a `docs/agents/<page>.md#constraint-<id>` target
(same-page `#constraint-<id>` links inside docs/agents/ included), resolves
each against the target page's `<a id="constraint-<id>">` anchors and writes
the sorted inventory with the unresolved subset. Rule files under
docs/agents/rules/ are rendered into their topic page, so their relative links
resolve from docs/agents/. Exit 1 when any link is unresolved.
"""
import json
import posixpath
import re
import subprocess
import sys
from pathlib import Path

LINK = re.compile(r"\]\(([^)\s#]*)#(constraint-[A-Za-z0-9_-]+)\)")
BARE = re.compile(r"(?<![\w/.-])((?:\.\./)*(?:docs/)?agents/[a-z-]+\.md)#(constraint-[A-Za-z0-9_-]+)")
ANCHOR = re.compile(r'<a id="(constraint-[^"]+)"></a>')


def tracked(root):
    out = subprocess.check_output(["git", "-C", str(root), "ls-files", "-z"])
    return [p.decode() for p in out.split(b"\0") if p]


def main():
    root = Path(sys.argv[1]).resolve()
    files = tracked(root)
    anchors = {}
    for path in files:
        if re.fullmatch(r"docs/agents/[a-z-]+\.md", path):
            anchors[path] = set(ANCHOR.findall((root / path).read_text(encoding="utf-8")))
    links = []
    for path in files:
        if Path(path).suffix not in {".md", ".rs", ".py", ".lean", ".json", ".toml", ".txt", ".qcore", ".qv", ".sh"}:
            continue
        try:
            text = (root / path).read_text(encoding="utf-8")
        except (UnicodeDecodeError, FileNotFoundError, IsADirectoryError):
            continue
        if "constraint-" not in text:
            continue
        base = posixpath.dirname(path)
        if path.startswith("docs/agents/rules/"):
            base = "docs/agents"
        for number, line in enumerate(text.splitlines(), 1):
            found = []
            for m in LINK.finditer(line):
                target, anchor = m.groups()
                if target == "":
                    if not path.startswith("docs/agents/"):
                        continue
                    resolved = path if not path.startswith("docs/agents/rules/") else None
                    if resolved is None:
                        # A same-page link in a rule file targets its topic page.
                        topic = re.search(r"^topic: ([a-z-]+)$", text, re.M)
                        resolved = f"docs/agents/{topic.group(1)}.md" if topic else path
                else:
                    resolved = posixpath.normpath(posixpath.join(base, target))
                found.append((m.group(0), resolved, anchor))
            for m in BARE.finditer(line):
                target, anchor = m.groups()
                if target.startswith("docs/"):
                    resolved = target
                else:
                    resolved = posixpath.normpath(posixpath.join(base, target))
                if not resolved.startswith("docs/agents/"):
                    continue
                found.append((m.group(0), resolved, anchor))
            for spelled, resolved, anchor in found:
                if not re.fullmatch(r"docs/agents/[a-z-]+\.md", resolved):
                    continue
                ok = anchor in anchors.get(resolved, set())
                links.append({"file": path, "line": number, "target": resolved,
                              "anchor": anchor, "resolved": ok})
    links.sort(key=lambda l: (l["file"], l["line"], l["target"], l["anchor"]))
    bad = [l for l in links if not l["resolved"]]
    record = {"links": len(links), "unresolved": len(bad),
              "distinct_targets": len({(l["target"], l["anchor"]) for l in links}),
              "unresolved_links": bad, "inventory": links}
    Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    print(f"constraint links: {len(links)}; distinct targets {record['distinct_targets']}; unresolved {len(bad)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
