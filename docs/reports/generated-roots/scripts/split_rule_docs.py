#!/usr/bin/env python3
"""One-time migration (card GENERATED-ROOTS): split docs/agents/*.md constraints
into docs/agents/rules/<id>.md and insert the generator markers.

Usage: split_rule_docs.py <checkout>   (run once on base fb5412930)

A rule segment starts at its `<a id="constraint-<id>"></a>` line, for every id
the router docs/agents/README.md indexes on that page plus gateway.md's
`11-series` lead-in, and ends at the next rule anchor, at any other `<a id=`
anchor, or at a heading (when the segment itself opens with a heading, at the
next heading of the same or a higher level). Each page's segments were checked
to be contiguous. The rule text is the segment after its anchor line with
surrounding blank lines removed; every other byte stays in the page.
"""
import re
import sys
from pathlib import Path


def main():
    root = Path(sys.argv[1]).resolve()
    sys.path.insert(0, str(root / "scripts"))
    import gen_rule_docs as gen

    agents = root / "docs/agents"
    rules_dir = agents / "rules"
    router_text = (agents / "README.md").read_text(encoding="utf-8")
    rows = {}
    for line in router_text.splitlines():
        m = re.match(r"^\| ([^|]+?) \| \[([^\]]+)\]\(([a-z-]+)\.md#constraint-([^)]+)\) \| (.*) \|$", line)
        if m:
            rule_id, label, topic, anchor, summary = m.groups()
            assert rule_id == anchor and label == topic, line
            assert rule_id not in rows, rule_id
            rows[rule_id] = (topic, summary)
    anchor_line = re.compile(r'^<a id="constraint-([^"]+)"></a>\n$')
    rules_dir.mkdir(exist_ok=True)
    written = 0
    for page in sorted(agents.glob("*.md")):
        if page.name == "README.md":
            continue
        lines = page.read_text(encoding="utf-8").splitlines(keepends=True)
        starts = [(i, anchor_line.match(l).group(1)) for i, l in enumerate(lines)
                  if anchor_line.match(l)]
        starts = [(i, x) for i, x in starts
                  if rows.get(x, (None,))[0] == page.stem or (page.stem == "gateway" and x == "11-series")]
        if not starts:
            continue
        indices = {i for i, _ in starts}
        segments = []
        for i, rule_id in starts:
            j = i + 1
            while j < len(lines) and not lines[j].strip():
                j += 1
            level = None
            heading = re.match(r"^(#+) ", lines[j]) if j < len(lines) else None
            if heading:
                level = len(heading.group(1))
                j += 1
            while j < len(lines):
                if j in indices or lines[j].startswith("<a id="):
                    break
                heading = re.match(r"^(#+) ", lines[j])
                if heading and (level is None or len(heading.group(1)) <= level):
                    break
                j += 1
            segments.append((rule_id, i, j))
        for (a, _, end), (b, start, _) in zip(segments, segments[1:]):
            assert end == start, f"{page}: {a} and {b} are not contiguous"
        rules = []
        for rule_id, i, j in segments:
            body = "".join(lines[i + 1:j]).strip("\n")
            summary = rows[rule_id][1] if rule_id in rows else None
            rule = {"id": rule_id, "topic": page.stem, "summary": summary, "body": body}
            (rules_dir / f"{rule_id}.md").write_text(gen.render_rule_file(rule), encoding="utf-8")
            rules.append(rule)
            written += 1
        order = [r["id"] for r in rules]
        assert order == sorted(order, key=gen.natural_key), f"{page}: not in natural order"
        prefix = "".join(lines[:segments[0][1]])
        suffix = "".join(lines[segments[-1][2]:]).lstrip("\n")
        text = (prefix + gen.PAGE_START + "\n\n" + gen.render_section(rules) + gen.PAGE_END + "\n"
                + ("\n" + suffix if suffix else ""))
        page.write_text(text, encoding="utf-8")
    # Router: the stable index table runs from its header to the end of the page.
    head, table = router_text.split(gen.ROUTER_HEADER, 1)
    assert all(line.startswith("| ") for line in table.splitlines()), "router table is not last"
    (agents / "README.md").write_text(head + gen.ROUTER_START + "\n\n" + gen.ROUTER_HEADER
                                      + "<rows>\n" + gen.ROUTER_END + "\n", encoding="utf-8")
    print(f"wrote {written} rule files")


if __name__ == "__main__":
    main()
