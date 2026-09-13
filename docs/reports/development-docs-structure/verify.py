"""Replay this migration at its recorded documentation revision.

Run from any directory. Historical Git source must be available locally.
Checks do not execute code from historical documents or write any artifacts.
The JSON on stdout is migration evidence, not a current status ledger.
"""

from pathlib import Path
import hashlib
import json
import re
import subprocess
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]
REPORT = ROOT / "docs/reports/development-docs-structure"
manifest = json.loads((REPORT / "migration.json").read_text())
assert manifest["schema_version"] == 1
assert re.fullmatch(r"[0-9a-f]{40}", manifest["source_commit"])
rebuilt = manifest["original_preamble"]
link_rebases = 0
command_updates = 0
cli_name_updates = 0
for row in manifest["parts"]:
    text = (ROOT / row["path"]).read_text()
    start, end = "<!-- migrated-section:start -->\n", "<!-- migrated-section:end -->\n"
    assert text.count(start) == text.count(end) == 1, row["path"]
    payload = text.split(start, 1)[1].split(end, 1)[0]
    assert hashlib.sha256(payload.encode()).hexdigest() == row["payload_sha256"], row["path"]
    for transform in reversed(row["transforms"]):
        assert payload.count(transform["new"]) == transform["count"], transform
        payload = payload.replace(transform["new"], transform["old"])
        if transform["reason"].startswith("relative link"):
            old_url = urlsplit(transform["old"][2:-1])
            new_url = urlsplit(transform["new"][2:-1])
            assert (ROOT / old_url.path).resolve() == (ROOT / row["path"]).parent.joinpath(new_url.path).resolve()
            assert old_url.fragment == new_url.fragment
            link_rebases += transform["count"]
        elif transform["reason"].startswith("bring two"):
            command_updates += transform["count"]
        else:
            assert transform["reason"].startswith("current CLI binary"), transform
            cli_name_updates += transform["count"]
    assert hashlib.sha256(payload.encode()).hexdigest() == row["source_sha256"], row["path"]
    rebuilt += payload
assert len(rebuilt.encode()) == manifest["source_bytes"]
assert hashlib.sha256(rebuilt.encode()).hexdigest() == manifest["source_sha256"]
original = subprocess.check_output(["git", "show", manifest["source_commit"] + ":DEVELOPMENT.md"], cwd=ROOT).decode()
assert rebuilt == original
old_agent = subprocess.check_output(["git", "show", manifest["source_commit"] + ":AGENTS.md"], cwd=ROOT).decode()
new_agent = (ROOT / "AGENTS.md").read_text()
blocks = [
    ("## Hard invariants", "## Standing constraints"),
    ("## Standing constraints", "## Standing per-change gate"),
    ("## Standing per-change gate", "## Lean CI hygiene"),
    ("## Lean CI hygiene", "## Key entry points"),
]
for first, last in blocks:
    old = old_agent[old_agent.index(first):old_agent.index(last)]
    new = new_agent[new_agent.index(first):new_agent.index(last)]
    assert old == new, first
constraints = new_agent[new_agent.index(blocks[1][0]):new_agent.index(blocks[1][1])]
ids = re.findall(r"^(\d+[a-z]?)\. ", constraints, re.M)
expected = {str(x) for x in range(1, 29)} | {"5a", "10a", "12a"} | {"15" + x for x in "abcdefghijklm"}
assert len(ids) == len(set(ids)) == 44 and set(ids) == expected

def headings(text):
    inside = False
    result = []
    for line in text.splitlines():
        if line.lstrip().startswith(("```", "~~~")):
            inside = not inside
        elif not inside and re.match(r"^#{1,6} ", line):
            result.append(line)
    return result

root_headings = headings((ROOT / "DEVELOPMENT.md").read_text())
assert set(headings(original)) <= set(root_headings)

def anchors(text):
    found = set(re.findall(r'<a\s+(?:id|name)="([^"]+)"', text))
    for heading in headings(text):
        base = re.sub(r"[^\w\s-]", "", re.sub(r"^#+ ", "", heading).lower()).replace(" ", "-")
        value = base
        suffix = 0
        while value in found:
            suffix += 1
            value = base + "-" + str(suffix)
        found.add(value)
    return found

pages = [ROOT / p for p in ["DEVELOPMENT.md", "AGENTS.md", "CLAUDE.md", "README.md", "docs/GENERALIZATION_PLAN.md"]]
pages += sorted((ROOT / "docs/development").rglob("*.md"))
links_checked = 0
targets = set()
for page in pages:
    for target in re.findall(r"\[[^\]\n]+\]\(([^)]+)\)", page.read_text()):
        parsed = urlsplit(target)
        if parsed.scheme or target.startswith("//"):
            continue
        destination = (page.parent / unquote(parsed.path)).resolve() if parsed.path else page
        assert destination.exists(), (page.relative_to(ROOT), target)
        if parsed.fragment and destination.is_file() and destination.suffix == ".md":
            assert unquote(parsed.fragment) in anchors(destination.read_text()), (page.relative_to(ROOT), target)
        links_checked += 1
        targets.add(destination)
assert all((ROOT / row["path"]).resolve() in targets for row in manifest["parts"])

# A global list of link targets does not prove reachability from the front door.
development_pages = {p.resolve() for p in (ROOT / "docs/development").rglob("*.md")}
front_door = ROOT / "DEVELOPMENT.md"
pending = [front_door]
visited = set()
while pending:
    page = pending.pop()
    if page in visited:
        continue
    visited.add(page)
    for target in re.findall(r"\[[^\]\n]+\]\(([^)]+)\)", page.read_text()):
        parsed = urlsplit(target)
        if parsed.scheme or target.startswith("//"):
            continue
        child = (page.parent / unquote(parsed.path)).resolve() if parsed.path else page
        if child in development_pages or child == front_door:
            pending.append(child)
assert development_pages <= visited, sorted(str(p.relative_to(ROOT)) for p in development_pages - visited)

# Original relative targets, including the two preamble links retained at root.
original_relative_links = [urlsplit(target) for target in re.findall(r"\[[^\]\n]+\]\(([^)]+)\)", original) if not urlsplit(target).scheme]
assert all((ROOT / url.path).resolve() in targets for url in original_relative_links)

new_root_size = (ROOT / "DEVELOPMENT.md").stat().st_size
measurement = {
    "source_commit": manifest["source_commit"],
    "complete_original_reconstructed": True,
    "source_bytes": manifest["source_bytes"],
    "new_root_index_bytes": new_root_size,
    "root_read_bytes_reduction_percent": round(100 * (1 - new_root_size / manifest["source_bytes"]), 2),
    "migrated_payload_pages": len(manifest["parts"]),
    "additional_directory_index_pages": 1,
    "largest_migrated_payload_bytes": max(row["payload_bytes"] for row in manifest["parts"]),
    "largest_child_page_bytes": max(p.stat().st_size for p in (ROOT / "docs/development").rglob("*.md")),
    "rebased_original_relative_link_occurrences": link_rebases,
    "original_relative_link_occurrences_still_resolved": len(original_relative_links),
    "current_build_command_updates": command_updates,
    "current_cli_binary_name_updates": cli_name_updates,
    "original_heading_lines_retained_in_root": len(headings(original)),
    "standing_constraints_byte_identical": len(ids),
    "hard_invariants_gate_and_lean_hygiene_byte_identical": True,
    "local_links_checked": links_checked,
    "all_payload_pages_reachable": True,
    "all_development_pages_reachable_from_root": len(development_pages),
    "measurement_scope": "UTF-8 document bytes only. This reduces the root guide read and provides smaller on-demand pages. It does not claim lower total repository storage, automatic AGENTS context, runtime token counts or billing savings.",
}
print(json.dumps(measurement, indent=2))
