#!/usr/bin/env python3
"""Attribute each edited line-block of AGENTS.md / CLAUDE.md / README.md to
compaction or correction, by measuring the block at the base and at the tip.
Blocks are located by a unique anchor string on each side."""
import subprocess

BASE = "d6a57f66"


def at_base(path):
    return subprocess.run(
        ["git", "show", f"{BASE}:{path}"], capture_output=True, text=True, check=True,
        cwd="/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9936abd188804098",
    ).stdout


def now(path):
    return open(
        "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/"
        "agent-a9936abd188804098/" + path, encoding="utf-8").read()


def line_with(text, needle):
    hits = [l for l in text.split("\n") if needle in l]
    assert len(hits) == 1, (needle, len(hits))
    return hits[0]


def block(text, start_needle, end_needle):
    lines = text.split("\n")
    s = [i for i, l in enumerate(lines) if start_needle in l]
    e = [i for i, l in enumerate(lines) if end_needle in l]
    assert len(s) == 1 and len(e) == 1, (start_needle, end_needle, len(s), len(e))
    return "\n".join(lines[s[0]:e[0]])


A0, A1 = at_base("AGENTS.md"), now("AGENTS.md")
C0, C1 = at_base("CLAUDE.md"), now("CLAUDE.md")
R0, R1 = at_base("README.md"), now("README.md")

rows = []


def add(label, kind, before, after):
    rows.append((label, kind, len(before), len(after), len(after) - len(before)))


# AGENTS.md
add("repo map: contract corpus module count", "correction",
    line_with(A0, "Declarative `.qcore` contract corpus"),
    line_with(A1, "Declarative `.qcore` contract corpus"))
add("repo map: gateway load-bearing modules", "correction",
    line_with(A0, "L2 existing-C analyzer, custody summary"),
    line_with(A1, "L2 existing-C analyzer, custody summary"))
add("constraint 15o: the mutex-crossing TSan sentence", "correction",
    block(A0, "the admitted shape without re-deriving it. The **creator performs no",
          "load-bearing evidence, not formatting"),
    block(A1, "the admitted shape without re-deriving it. The **creator performs no",
          "load-bearing evidence, not formatting"))
add("constraint 15h: the ThreadSanitizer boundary", "correction",
    block(A0, "must not be able to choose. Three boundaries",
          "`struct qcore_buffer`'s own tail padding"),
    block(A1, "must not be able to choose. Three boundaries",
          "`struct qcore_buffer`'s own tail padding"))
add("constraint 14: per-card keyword-census ledger", "compaction",
    block(A0, "14. New keywords are a recorded compatibility hazard",
          "15. Lock ranks and protection growth"),
    block(A1, "14. New keywords are a recorded compatibility hazard",
          "15. Lock ranks and protection growth"))
add("Q-space note", "compaction",
    block(A0, "**Q-space note — `Q-NAMESPACE-DECISION`", "| Codes | Owning pass | Meaning |"),
    block(A1, "**Q-space note — `Q-NAMESPACE-DECISION`", "| Codes | Owning pass | Meaning |"))
add("ledger: headline and arbiter chain", "compaction",
    block(A0, "Headline numbers after `W37-CORPUS-CONTRACTS-4`", "| Measure | Value |"),
    block(A1, "Headline numbers after `W37-CORPUS-CONTRACTS-4`", "| Measure | Value |"))
add("ledger: Cycle 19 raw corpus row", "compaction",
    line_with(A0, "| Cycle 19 raw corpus (185 units) |"),
    line_with(A1, "| Cycle 19 raw corpus (185 units) |"))
add("ledger: CT01-CT50 row", "compaction",
    line_with(A0, "| CT01–CT50 conformance |"), line_with(A1, "| CT01–CT50 conformance |"))
add("ledger: merged gate row", "correction + re-measurement",
    line_with(A0, "| Merged gate |"), line_with(A1, "| Merged gate |"))
add("ledger: follow-on count paragraph", "compaction",
    block(A0, "Where the detail lives, and the only places",
          "## Hard invariants"),
    block(A1, "Where the detail lives, and the only places", "## Hard invariants"))

# CLAUDE.md
add("cadence roster and the percentage sentence", "compaction",
    block(C0, "- **Compact these three files every 2–3 waves.**",
          "- **Re-evaluate stability every 2 waves**"),
    block(C1, "- **Compact these three files every 2–3 waves.**",
          "- **Re-evaluate stability every 2 waves**"))
add("platform records: the Windows open-residual family", "compaction",
    block(C0, "- Open, each with an exact test-name discharge condition",
          "- Four tests keep `runs_sanitized`"),
    block(C1, "- **Open, each with an exact test-name discharge condition",
          "- Four tests keep `runs_sanitized`"))

# README.md
add("the ThreadSanitizer claim", "correction",
    block(R0, "reaches no external peer. The mutex crossing",
          "syntax and separately checked semantic models"),
    block(R1, "reaches no external peer. The mutex crossing",
          "syntax and separately checked semantic models"))
add("the conformance-corpus figures", "correction",
    block(R0, "The conformance-case corpus stands at",
          "Verdict vocabulary and the hard invariants"),
    block(R1, "The conformance-case corpus stands at",
          "Verdict vocabulary and the hard invariants"))

print(f"{'block':52s} {'kind':26s} {'before':>7s} {'after':>7s} {'delta':>7s}")
for label, kind, b, a, d in rows:
    print(f"{label:52s} {kind:26s} {b:7d} {a:7d} {d:+7d}")
for kind in ("compaction", "correction", "correction + re-measurement"):
    t = sum(d for _, k, _, _, d in rows if k == kind)
    print(f"{kind:>40s} total: {t:+d}")
print(f"{'accounted':>40s} total: {sum(d for *_, d in rows):+d}")
