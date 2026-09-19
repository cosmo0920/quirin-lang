#!/usr/bin/env python3
"""Check this migration at its own checkpoint, not all future rule revisions.

Reconstruct all four base guides from preserved spans, verify rule coverage and
legacy headings, measure explicit retrieval packets, and check new local links.
No Lean/Rust semantic or token-billing claim follows from these checks.
"""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]
REPORT = Path("docs/reports/docs-structure-compression")
manifest = json.loads((ROOT / REPORT / "migration.json").read_text())
source_commit = manifest["source_commit"]
originals = {}
for source, info in manifest["sources"].items():
    original = subprocess.check_output(["git", "show", f"{source_commit}:{source}"], cwd=ROOT).decode()
    originals[source] = original
    rows = sorted((p for p in manifest["parts"] if p["source"] == source), key=lambda p: p["start"])
    cursor = 0
    rebuilt = ""
    for row in rows:
        assert row["start"] == cursor, (source, "gap or overlap", row)
        text = (ROOT / row["path"]).read_text()
        start = f"<!-- preserved:{row['key']}:start -->\n"
        end = f"<!-- preserved:{row['key']}:end -->\n"
        assert text.count(start) == text.count(end) == 1, row
        payload = text.split(start)[1].split(end)[0]
        for transform in reversed(row["transforms"]):
            assert transform["new"] in payload, transform
            old = urlsplit(transform["old"][2:-1])
            new = urlsplit(transform["new"][2:-1])
            old_target = (ROOT / source).parent / (old.path or Path(source).name)
            new_target = (ROOT / row["path"]).parent / new.path
            assert old_target.resolve() == new_target.resolve() and old.fragment == new.fragment
            payload = payload.replace(transform["new"], transform["old"], 1)
        assert hashlib.sha256(payload.encode()).hexdigest() == row["sha256"], row
        assert payload == original[row["start"]:row["end"]], row
        rebuilt += payload
        cursor = row["end"]
    assert cursor == len(original) and rebuilt == original, source
    assert len(rebuilt.encode()) == info["bytes"]
    assert hashlib.sha256(rebuilt.encode()).hexdigest() == info["sha256"]


def anchors(text):
    result = set(re.findall(r'<a\s+(?:id|name)="([^"]+)"', text))
    counts = {}
    fenced = False
    for line in text.splitlines():
        if line.startswith(("```", "~~~")):
            fenced = not fenced
        if fenced or not re.match(r"^#{1,6} ", line):
            continue
        title = re.sub(r"^#+ ", "", line)
        slug = re.sub(r"[^\w\s-]", "", title.lower()).replace(" ", "-")
        n = counts.get(slug, 0)
        result.add(slug + (f"-{n}" if n else ""))
        counts[slug] = n + 1
    return result


for source, text in originals.items():
    assert anchors(text) <= anchors((ROOT / source).read_text()), (source, "lost legacy heading")
rules = manifest["rules"]
assert len(rules) == len({r["id"] for r in rules}) == 50
for row in rules:
    assert "constraint-" + row["id"] in anchors((ROOT / row["path"]).read_text())
for p in (ROOT / "docs/development/increments/structure-checkpoints").glob("*.md"):
    assert p.stat().st_size < 32768, p
policy_sizes = {str(p.relative_to(ROOT)): p.stat().st_size for p in (ROOT / "docs/agents").glob("*.md")}
assert max(policy_sizes.values()) < 32768, policy_sizes

changed = subprocess.check_output(["git", "diff", source_commit, "--name-only"], cwd=ROOT, text=True).splitlines()
untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard"], cwd=ROOT, text=True).splitlines()
md_paths = sorted({p for p in changed + untracked if p.endswith(".md") and (ROOT / p).is_file()})
link_count = 0
failures = []
for name in md_paths:
    text = (ROOT / name).read_text()
    # Archived links and preservation payload links retain their original
    # destinations; inherited broken links are disclosed, not silently repaired.
    historical = "/structure-checkpoints/" in name
    for dest in re.findall(r"\]\(([^)\s]+)\)", text):
        url = urlsplit(dest)
        if url.scheme or url.netloc:
            continue
        path = (ROOT / name).parent / unquote(url.path) if url.path else ROOT / name
        if not path.exists():
            failures.append({"source": name, "target": dest, "reason": "missing path", "historical": historical})
        elif url.fragment and path.suffix == ".md" and unquote(url.fragment) not in anchors(path.read_text()):
            failures.append({"source": name, "target": dest, "reason": "missing anchor", "historical": historical})
        link_count += 1

roots = list(originals)
before = sum(len(t.encode()) for t in originals.values())
after = sum((ROOT / p).stat().st_size for p in roots)
common = ["AGENTS.md", "CLAUDE.md", "docs/agents/operations.md", "docs/agents/validation.md",
          "docs/agents/README.md", "docs/agents/evidence.md", "docs/STATUS.md"]
packets = {
    "documentation": common,
    "gateway": common + ["docs/agents/gateway.md", "docs/agents/source-identity.md", "docs/agents/diagnostics.md"],
    "buffer-task": common + ["docs/agents/source-routes.md", "docs/agents/buffers-records.md", "docs/agents/concurrency.md", "docs/agents/diagnostics.md"],
}
result = {"source_commit": source_commit, "source_reconstruction": "exact after reversible link rebasing",
          "source_bytes_preserved": before, "standing_rules": len(rules), "hard_invariants": 6,
          "root_guides_before_bytes": before, "root_guides_after_bytes": after,
          "root_reduction_percent": round(100*(1-after/before), 2),
          "largest_policy_page_bytes": max(policy_sizes.values()),
          "startup_guidance_assumed_injected": {"before_bytes": sum(len(originals[p].encode()) for p in ["AGENTS.md","CLAUDE.md"]),
                                                "after_bytes": sum((ROOT/p).stat().st_size for p in ["AGENTS.md","CLAUDE.md"])},
          "packets": {}, "local_links_checked": link_count, "link_findings": failures,
          "limits": "Explicit reading packets, not runtime telemetry or token billing; evidence archives retained and total repository bytes may grow."}
for key, packet in packets.items():
    result["packets"][key] = {"reads": packet, "cumulative_source_bytes": sum((ROOT/p).stat().st_size for p in packet),
                              "unique_source_bytes": sum((ROOT/p).stat().st_size for p in set(packet))}
if "--write" in sys.argv:
    (ROOT / REPORT / "preservation.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({k:v for k,v in result.items() if k not in ["packets","link_findings"]}, indent=2))
if failures:
    print(json.dumps(failures, indent=2))
    raise SystemExit(1)
