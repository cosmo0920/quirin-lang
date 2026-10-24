#!/usr/bin/env python3
"""Check the rule split against the base revision.

Usage: verify_split.py <checkout> <base-rev> <output.json>

1. Every rule file's text occurs byte for byte in its base topic page,
   immediately after that rule's anchor line and one blank line.
2. Each generated page equals its base page after deleting the two marker
   lines and dropping blank lines (the only other edits).
3. Every `<a id=...>` anchor and `<!-- preserved:... -->` marker of the base
   pages is present, the same number of times, in the generated pages.
4. The router's rows are the base rows as a set (only their order may differ).
"""
import json
import re
import subprocess
import sys
from pathlib import Path


def show(root, rev, path):
    return subprocess.check_output(["git", "-C", str(root), "show", f"{rev}:{path}"]).decode("utf-8")


def squeeze(text):
    """Drop blank lines: the split only adds, removes or merges blank lines."""
    return "\n".join(line for line in text.split("\n") if line.strip()) + "\n"


def main():
    root, rev, out = Path(sys.argv[1]).resolve(), sys.argv[2], Path(sys.argv[3])
    sys.path.insert(0, str(root / "scripts"))
    import gen_rule_docs as gen
    agents = root / "docs/agents"
    from listed_files import listed_files
    rules = gen.load_rules(listed_files(agents))
    result = {"rule_files": len(rules), "router_rows_from_rule_files": 0,
              "body_bytes": 0, "failures": [], "pages": {}}
    bases = {}
    for page in sorted(agents.glob("*.md")):
        bases[page.name] = show(root, rev, f"docs/agents/{page.name}")
    for rule in rules:
        base = bases[f"{rule['topic']}.md"]
        needle = f'<a id="constraint-{rule["id"]}"></a>\n\n{rule["body"]}\n'
        if base.count(needle) != 1:
            result["failures"].append(f"rule {rule['id']}: text not found once after its anchor")
        result["body_bytes"] += len(rule["body"].encode())
        if rule["summary"] is not None:
            result["router_rows_from_rule_files"] += 1
    for page in sorted(agents.glob("*.md")):
        now = page.read_text(encoding="utf-8")
        base = bases[page.name]
        anchors_base = sorted(re.findall(r'<a id="[^"]+"></a>', base))
        anchors_now = sorted(re.findall(r'<a id="[^"]+"></a>', now))
        preserved_base = sorted(re.findall(r"<!-- preserved:[^ ]+ -->", base))
        preserved_now = sorted(re.findall(r"<!-- preserved:[^ ]+ -->", now))
        stripped = "\n".join(line for line in now.split("\n")
                             if not line.startswith(("<!-- generated:rules:", "<!-- generated:router:")))
        record = {"anchors": len(anchors_base), "preserved_markers": len(preserved_base),
                  "anchors_equal": anchors_base == anchors_now,
                  "preserved_equal": preserved_base == preserved_now}
        if page.name == "README.md":
            rows_base = sorted(l for l in base.splitlines() if re.match(r"^\| [0-9]", l))
            rows_now = sorted(l for l in now.splitlines() if re.match(r"^\| [0-9]", l))
            record["router_rows"] = len(rows_now)
            record["router_rows_equal_as_set"] = rows_base == rows_now
            record["equal_after_marker_removal_ignoring_row_order"] = (
                sorted(squeeze(stripped).splitlines()) == sorted(squeeze(base).splitlines()))
            ok = record["router_rows_equal_as_set"] and record["equal_after_marker_removal_ignoring_row_order"]
        else:
            record["equal_after_marker_removal_and_blank_squeeze"] = squeeze(stripped) == squeeze(base)
            ok = record["equal_after_marker_removal_and_blank_squeeze"]
        if not (ok and record["anchors_equal"] and record["preserved_equal"]):
            result["failures"].append(f"{page.name}: {record}")
        result["pages"][page.name] = record
    out.write_text(json.dumps(result, indent=1) + "\n")
    print(f"rule files {len(rules)}; router rows {result['router_rows_from_rule_files']}; "
          f"body bytes {result['body_bytes']}; failures {len(result['failures'])}")
    for failure in result["failures"]:
        print(" ", failure)
    return 1 if result["failures"] else 0


if __name__ == "__main__":
    sys.exit(main())
