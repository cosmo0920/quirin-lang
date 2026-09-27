#!/usr/bin/env python3
"""One-shot, base-pinned documentation migration; verify.py checks preservation.

Run only to reproduce this card from its base in a disposable checkout.
This is not a generator for future edits to the live documentation.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "463fa6e8d0c44c23e402dd78bdc146963f0244ca"
REPORT = Path("docs/reports/docs-structure-compression")
HISTORY = "docs/development/increments/structure-checkpoints"
sources = {
    name: subprocess.check_output(["git", "show", f"{BASE}:{name}"], cwd=ROOT).decode()
    for name in ("AGENTS.md", "CLAUDE.md", "README.md", "DEVELOPMENT.md")
}
parts = []
outputs = {}


def init(path, title, kind="rule"):
    if path not in outputs:
        note = (
            "> Historical checkpoint at the migration base, not current instructions.\n"
            "> Original wording is preserved; current rules start at [AGENTS.md]({root}).\n\n"
            if kind == "history" else
            "Current reference, routed by [AGENTS.md]({root}). Rule numbers are stable.\n"
            "Paths in backticks are repository-relative. Read every applicable rule,\n"
            "including cross-topic rules, before editing; the index is not a substitute.\n\n"
        )
        root = os.path.relpath("AGENTS.md", str(Path(path).parent))
        outputs[path] = f"# {title}\n\n" + note.format(root=root)


def preserve(source, start, end, path, key):
    payload = sources[source][start:end]
    transforms = []
    # Rebase actual Markdown links; source-code paths remain root-relative.
    def link(match):
        dest = match[1]
        if re.match(r"[a-zA-Z][\w+.-]*:", dest) or dest.startswith("/"):
            return match[0]
        target, sep, frag = dest.partition("#")
        resolved = Path(source).parent / (target or Path(source).name)
        new = os.path.relpath(str(resolved), str(Path(path).parent))
        if sep:
            new += "#" + frag
        replacement = "](" + new + ")"
        if replacement != match[0]:
            transforms.append({"old": match[0], "new": replacement})
        return replacement
    rendered = re.sub(r"\]\(([^)\s]+)\)", link, payload)
    outputs[path] += f"<!-- preserved:{key}:start -->\n" + rendered + f"<!-- preserved:{key}:end -->\n\n"
    parts.append({"source": source, "start": start, "end": end, "path": path,
                  "key": key, "sha256": hashlib.sha256(payload.encode()).hexdigest(),
                  "transforms": transforms})


agent = sources["AGENTS.md"]
def pos(heading):
    return agent.index(heading)

# Replace the bloated startup material, preserving its checkpoint in history.
for name, first, last in [
    ("agent-introduction", 0, pos("## What this project is")),
    ("status", pos("## Status ledger"), pos("## Hard invariants")),
]:
    path = f"{HISTORY}/{name}.md"
    init(path, f"Pre-restructure {name}", "history")
    preserve("AGENTS.md", first, last, path, name)

path = "docs/development/architecture.md"
init(path, "Architecture and code entry points")
preserve("AGENTS.md", pos("## What this project is"), pos("## Status ledger"), path, "architecture")
preserve("AGENTS.md", pos("## Key entry points"), pos("### Diagnostics namespaces"), path, "entry-points")
preserve("AGENTS.md", pos("## Where to read more"), len(agent), path, "further-reading")

outputs["AGENTS.md"] = """# QCore-C agent briefing

This file is the startup router and authority for which rules to read. Detailed
rules live in the linked topic files; their existing numbers and force are
unchanged. This structure deliberately replaces the former monolithic briefing.

## Start here

1. Read [operations](docs/agents/operations.md) for workflow, preservation,
   reconciliation, worktree hygiene and the main-branch guard.
2. Read [current status](docs/STATUS.md) for measured coverage and its sources.
   Mapping, model proof and native observations are separate evidence classes.
3. Use [the rule router](docs/agents/README.md) to read the union of path and
   topic rules for committed changes since the base, staged/unstaged changes,
   renamed/deleted/untracked paths and planned edits. Shared files need that
   union. Unknown relevance requires more reading, never silent omission.
4. Run the [standing gate](docs/agents/validation.md) and report commands,
   exits, counts and private log paths. No optimistic fallback or hidden failure.

Use `rg -n` on an exact symbol/card/heading before reading a long section.
Select JSON keys rather than dumping a matrix. Historical reports retain their
checkpoint claims; each report owns its evidence. The authoritative specifications
are [language](docs/QCORE_SPEC_r0_3.md) and
[contracts](docs/QCORE_CONTRACT_SPEC_r0_3.md); supported source forms are in
[implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).

## Documentation retrieval and compact context

[Documentation hub](docs/README.md) is the common entry point for users and
contributors. [DEVELOPMENT.md](DEVELOPMENT.md) retains legacy heading links.
[Architecture](docs/development/architecture.md) maps crates and entry points.
[Fluent Bit queue](docs/plans/fluent-bit-system-coverage/README.md) holds
prospective production cards. Open work is the acceptance matrix's
`named_followups` plus each report's follow-ons; a rule mention is not tracking.

One current authority per rule. Edit the relevant topic page, not a duplicate
in a root guide. Add exactly one increment paragraph, newest first, to the
current record named by [the increment index](docs/development/increments/README.md).
Update [status](docs/STATUS.md) from primary evidence when figures change; do
not turn it into history. Keep new history pages below 32 KiB where a complete
record boundary permits. Preserve frozen evidence. Add a topic to the router
when necessary; do not grow root chronicles. Compaction cadence and preservation
requirements remain in [operations](docs/agents/operations.md).

Measure startup guidance, cumulative retrieval and unique source bytes separately;
byte counts are not token billing. A smaller index alone proves no task-cost gain.

"""
preserve("AGENTS.md", pos("## Hard invariants"), pos("## Standing constraints"), "AGENTS.md", "hard-invariants")

start, end = pos("## Standing constraints"), pos("## Standing per-change gate")
block = agent[start:end]
boundaries = sorted({0, len(block)} | {m.start() for m in re.finditer(
    r"^(?:\d+[a-z]?\. |\*\*[^\n]+\*\*\n)", block, re.M)})
groups = {
    "diagnostics": ["1", "2", "3"],
    "gateway": ["4", "5", "5a", "5b", "5c", "6", "7", "8", "9", "10", "10a", "10b"],
    "source-identity": ["11", "12", "12a"],
    "source-routes": ["13", "14", "15", "15a", "15b"],
    "io": ["15c", "15d", "15e", "15f", "15g", "15i", "15p"],
    "buffers-records": ["15h", "15k"],
    "effects": ["15j", "15l", "15m"],
    "concurrency": ["15n", "15o", "27"],
    "providers": ["16", "17", "18", "19", "20", "26"],
    "evidence": ["21", "22", "23", "24", "25", "28"],
}
rule_rows = []
archive = f"{HISTORY}/constraint-preambles.md"
init(archive, "Original constraint section headings", "history")
for n, (a, b) in enumerate(zip(boundaries, boundaries[1:])):
    content = block[a:b]
    m = re.match(r"(\d+[a-z]?)\. ", content)
    if m:
        ident = m[1]
        group = next(g for g, ids in groups.items() if ident in ids)
        path = f"docs/agents/{group}.md"
        init(path, group.replace("-", " ").title() + " rules")
        outputs[path] += f'<a id="constraint-{ident}"></a>\n\n'
        preserve("AGENTS.md", start+a, start+b, path, "constraint-"+ident)
        title = re.sub(r"[`*]", "", content.splitlines()[0][len(ident)+2:])
        rule_rows.append({"id": ident, "path": path, "title": title})
    else:
        preserve("AGENTS.md", start+a, start+b, archive, f"constraint-preamble-{n}")

path = "docs/agents/diagnostics.md"
preserve("AGENTS.md", pos("### Diagnostics namespaces"), pos("## Where to read more"), path, "diagnostic-namespaces")
path = "docs/agents/validation.md"
init(path, "Validation gate and Lean audit policy")
preserve("AGENTS.md", pos("## Standing per-change gate"), pos("## Key entry points"), path, "validation")

claude = sources["CLAUDE.md"]
path = "docs/agents/operations.md"
init(path, "Operations and integration policy")
outputs[path] += ("Authority update: references below to rules/status in `AGENTS.md` now\n"
                  "resolve through its topic router and `docs/STATUS.md`. New amendments\n"
                  "land in the canonical topic file. The cadence covers these pages as\n"
                  "well as the concise root guides. Migration changes location, not force.\n\n")
preserve("CLAUDE.md", claude.index("## Working rules"), claude.index("## Platform records"), path, "operations")
path = "docs/development/platform-evidence.md"
init(path, "Platform evidence and boundaries")
preserve("CLAUDE.md", claude.index("## Platform records"), claude.index("## Pointer"), path, "platform-records")
for key, a, b in [("claude-intro", 0, claude.index("## Working rules")),
                  ("claude-pointer", claude.index("## Pointer"), len(claude))]:
    path = f"{HISTORY}/agent-introduction.md"
    preserve("CLAUDE.md", a, b, path, key)
outputs["CLAUDE.md"] = """# Claude Code project notes

@AGENTS.md

Follow the startup router in `AGENTS.md`. Read
[operations](docs/agents/operations.md), the applicable
[numbered rules](docs/agents/README.md), and the
[validation gate](docs/agents/validation.md). These are the canonical policies;
this file deliberately carries no duplicate ledger or working rules.

## Working rules

See [operations](docs/agents/operations.md#working-rules).

## Worktree and measurement hygiene

See [worktree policy](docs/agents/operations.md#worktree-and-measurement-hygiene).

## Platform records

See [platform evidence](docs/development/platform-evidence.md).

## Pointer

Start at [the documentation hub](docs/README.md).
"""

# Preserve the public overview, including every old claim/link, as a dated
# checkpoint; the new overview deliberately avoids duplicating volatile status.
for source, key in [("README.md", "public-overview"), ("DEVELOPMENT.md", "development-index")]:
    path = f"{HISTORY}/{key}.md"
    init(path, "Pre-restructure " + key, "history")
    preserve(source, 0, len(sources[source]), path, key)

outputs["README.md"] = """# Quirin

Quirin is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup and concurrency, lowering to C11. It also
provides a conservative checker for existing C, exercised on a pinned Fluent Bit
corpus. Unsupported or unproven behavior stays `Unknown`.

The commands are `quirin` and `quirin-audit`; sources use `.qcore`. Rust crates,
namespaces and the emitted-C API retain the `qcore` spelling. The `.qc` extension
is retired.

## Quick start

```sh
git submodule update --init --recursive
cargo build --bin quirin
./target/debug/quirin check examples/cycle12.qr
./target/debug/quirin emit -o generated.c examples/cycle12.qr
```

Checking/emitting Quirin does not require Clang. Compiling emitted C needs a
C11 compiler; existing-C analysis and the full test suite need Clang. See
[build and platform setup](docs/development/build-and-test.md) and the
[complete validation gate](docs/agents/validation.md).

## What the type system tracks

Usage grades, static indices with nominal units, spendable quantitative credit,
live resource obligations, cost upper bounds, and effects with per-exit states
are distinct domains. A work bound is not a wall-clock bound.

## What source can say today

Support is defined per admitted profile in
[implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).
[Runnable examples](examples/README.md) cover threads, buffers, records,
strings, rendered output, extern calls, events, sockets and HTTP. Profile
boundaries matter: a checked model or a successful native run does not establish
full source-language support, protocol conformance or general race freedom.

## Status at a glance

**Full language conformance is not claimed.** Read [current measured status](docs/STATUS.md)
for corpus verdicts, CT mapping, proof and host-scoped native evidence. Unknowns
stay in the denominator; provisional Unsafe findings are not upstream defects.

## Documentation map

**Start at [the documentation hub](docs/README.md).** It separates current
specifications, implementation status, future work and historical evidence.

| Need | Entry point |
|---|---|
| Language and contracts | [Specifications](docs/README.md#specifications) |
| Build, architecture or contribute | [Development guide](DEVELOPMENT.md) |
| Work as an agent | [Agent rules](AGENTS.md) |
| Choose the next card | [Planning](docs/README.md#planning) |
| Find a past card or wave | [Evidence](docs/README.md#evidence-and-history) |

Versioned main-branch hooks are activated by `scripts/install_main_guard.py` in
the main checkout. They prevent accidental updates, not intentional bypass;
the [operations policy](docs/agents/operations.md) explains authorized updates.
"""

outputs["DEVELOPMENT.md"] = """# QCore-C development notes

[Documentation hub](docs/README.md) · [Current status](docs/STATUS.md)

This is a compact compatibility entry point. The hub is the common navigation
surface; the pages below own their content.

## Table of contents

| Topic | Authority |
|---|---|
| Specifications and records | [References](docs/development/references.md) |
| Repository layout | [Architecture](docs/development/architecture.md) |
| Current measurements | [Status](docs/STATUS.md) |
| Build and test | [Setup](docs/development/build-and-test.md) · [gate](docs/agents/validation.md) |
| Agent policies | [Rule router](docs/agents/README.md) |
| Past increments | [Wave index](docs/development/increments/README.md) |

## Specifications and records

See [references](docs/development/references.md).

## Repository layout

See [architecture and entry points](docs/development/architecture.md).

## Project status

See [current status](docs/STATUS.md). Historical cycle summaries are in
[cycle evidence](docs/development/cycle-status.md).

## Build and test

Read [setup](docs/development/build-and-test.md) and run the
[standing gate](docs/agents/validation.md).

### Windows

See [Windows setup](docs/development/build-and-test.md#windows) and
[platform evidence](docs/development/platform-evidence.md).

## Increment records

[The increment index](docs/development/increments/README.md) names the current
record file and every wave. Add one paragraph there per increment, newest first.

## Historical section links

Legacy anchors below remain valid. The linked records preserve their checkpoints.

"""
# Keep the old historical headings and pointers unchanged, without a second TOC.
dev = sources["DEVELOPMENT.md"]
outputs["DEVELOPMENT.md"] += dev[dev.index("### Quant delimiter"):]

outputs["AGENTS.md"] += """## Standing constraints when touching specific subsystems

Read [the numbered rule router](docs/agents/README.md). The full text lives in
topic files, not in this startup guide. Evidence/trust rules 21–28 apply across
topics; diagnostic and source-identity rules also cross crate boundaries.

## Status ledger (do not overstate)

See [current measurements and provenance](docs/STATUS.md).

## Repository map

See [architecture](docs/development/architecture.md#repository-map).

## What this project is

See [the public overview](README.md).

## Standing per-change gate

Read and run [the full gate](docs/agents/validation.md#standing-per-change-gate).

## Lean CI hygiene

See [Lean audit policy](docs/agents/validation.md#lean-ci-hygiene).

## Key entry points

See [code entry points](docs/development/architecture.md#key-entry-points).

### Diagnostics namespaces

See [namespace policy](docs/agents/diagnostics.md#diagnostics-namespaces).

## Where to read more

[Documentation hub](docs/README.md) · [Rule router](docs/agents/README.md) ·
[Preservation evidence](docs/reports/docs-structure-compression/README.md).
"""

router = """# Numbered rule router

These pages are the canonical full text of the standing constraints formerly
embedded in `AGENTS.md`; references to “AGENTS constraint N” retain their meaning.
Read all applicable rules, not just this table. The rule text is preserved
verbatim apart from Markdown link rebasing. Root guides are navigation only.

## Choose by task

| Work | Read |
|---|---|
| Every change | [Operations](operations.md), [validation](validation.md), [evidence/trust](evidence.md) |
| C gateway, summaries, contracts or corpus | [Gateway](gateway.md), [source identity](source-identity.md), [diagnostics](diagnostics.md) |
| Parsing, lowering, keywords, source profiles | [Source routes](source-routes.md), [diagnostics](diagnostics.md), then affected feature below |
| Buffers, strings, slices, leases, records | [Buffers/records](buffers-records.md), [concurrency](concurrency.md) when crossing tasks |
| Effects, error regions, epoch credit | [Effects](effects.md), [source routes](source-routes.md) |
| Events, sockets, HTTP, rendering | [I/O](io.md), [source routes](source-routes.md) |
| Threads, mutex sharing, traces, F02 | [Concurrency](concurrency.md), [buffers/records](buffers-records.md), [source routes](source-routes.md) |
| Provider host, protocol, packages, transport | [Providers](providers.md), [source identity](source-identity.md) |
| Documentation, CT, acceptance, archive or tracker | [Evidence](evidence.md), [operations](operations.md); add topic rules if claims change |

Shared-file changes require the union. Relevant paths include the committed
diff from the declared base and all staged, unstaged, renamed, deleted,
untracked and planned files. If relevance is uncertain, inspect the authoritative
rules. This table routes reading; it neither exempts a constraint nor proves it
was read. Backticked code paths throughout these pages are repository-relative.

## Stable rule index

| Rule | Full text | Opening sentence (navigation only) |
|---|---|---|
"""
for row in rule_rows:
    router += f"| {row['id']} | [{Path(row['path']).stem}]({Path(row['path']).name}#constraint-{row['id']}) | {row['title'].replace('|', '&#124;')}… |\n"
outputs["docs/agents/README.md"] = router
outputs[f"{HISTORY}/README.md"] = """# Documentation restructuring checkpoints

Preserved text from the restructuring base, not current authority. Each file is
below 32 KiB. The [migration report](../../../reports/docs-structure-compression/README.md)
explains exact reconstruction; current rules start at [AGENTS.md](../../../../AGENTS.md).

- [Agent introduction](agent-introduction.md)
- [Former status narrative](status.md)
- [Constraint preambles](constraint-preambles.md)
- [Former public overview](public-overview.md)
- [Former development index](development-index.md)

Standing rules and operational policy were moved into live topic references,
not duplicated in this archive. The migration manifest maps every source span.
"""

for path, text in outputs.items():
    p = ROOT / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.rstrip() + "\n")
manifest = {"schema_version": 1, "source_commit": BASE,
            "sources": {k: {"bytes": len(v.encode()), "sha256": hashlib.sha256(v.encode()).hexdigest()}
                        for k, v in sources.items()}, "parts": parts, "rules": rule_rows}
(ROOT / REPORT / "migration.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Preserved {len(parts)} spans, {len(rule_rows)} numbered rules; wrote {len(outputs)} documents")
