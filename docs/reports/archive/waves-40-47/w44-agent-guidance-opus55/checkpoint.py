#!/usr/bin/env python3
"""Write or check the agent-cycle history checkpoint.

  checkpoint.py --write        regenerate the checkpoint from BASE and the working tree
  checkpoint.py --check [TIP]  verify every block against BASE byte for byte and,
                               when TIP is given, that every BASE line TIP removed or
                               reworded in the covered files lies inside some block

Each block is a whole paragraph (blank-line delimited) of a BASE file that this
card removed or reworded. Identical blocks from several files are stored once.
The check reads BASE blobs with `git show`, so it stays valid after later edits.
"""
import difflib
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
BASE = '94502987472c25567d87fc873615ea5de1667603'
OUT = ROOT/'docs/development/increments/structure-checkpoints/agent-cycle-guidance.md'
FILES = ['AGENTS.md', 'CLAUDE.md', 'docs/agents/operations.md', 'docs/agents/validation.md',
         'docs/agents/README.md', 'docs/agents/buffers-records.md', 'docs/agents/concurrency.md',
         'docs/agents/diagnostics.md', 'docs/agents/effects.md', 'docs/agents/evidence.md',
         'docs/agents/gateway.md', 'docs/agents/io.md', 'docs/agents/providers.md',
         'docs/agents/source-identity.md', 'docs/agents/source-routes.md']
HEADER = """# Agent-cycle guidance checkpoint (W44-AGENT-GUIDANCE-OPUS55)

> Historical checkpoint, not current instructions. Current rules start at
> [AGENTS.md](../../../../AGENTS.md) and the [rule router](../../../agents/README.md).

Every paragraph of the base guidance that card W44-AGENT-GUIDANCE-OPUS55 removed
or reworded, verbatim, at base `{base}`. Each block names its source file and
base line range. The card's [report](../../../reports/w44-agent-guidance-opus55/README.md)
maps each block to its current location; `checkpoint.py --check` there verifies
the blocks against the base byte for byte. Relative links are as in the source
file and may not resolve from here.
"""
BLOCK = re.compile(r'^### (?P<src>[^\n]+)\n\n<!-- preserved:(?P<id>agent-cycle-\d+):start -->\n'
                   r'(?P<body>.*?)<!-- preserved:(?P=id):end -->\n', re.M | re.S)


def show(rev, path):
    return subprocess.check_output(['git', '-C', str(ROOT), 'show', f'{rev}:{path}']).decode('utf-8')


def changed_ranges(old, new):
    """0-based half-open BASE line ranges touched by the edit, widened to paragraphs."""
    ranges = []
    for tag, i1, i2, _, _ in difflib.SequenceMatcher(None, old, new, autojunk=False).get_opcodes():
        if tag in ('equal', 'insert'):
            continue
        a, b = i1, i2
        while a > 0 and old[a-1].strip():
            a -= 1
        while b < len(old) and old[b].strip():
            b += 1
        if ranges and a <= ranges[-1][1]:
            ranges[-1] = (ranges[-1][0], max(b, ranges[-1][1]))
        else:
            ranges.append((a, b))
    return ranges


def blocks(tip):
    found = {}
    for path in FILES:
        old = show(BASE, path).splitlines(keepends=True)
        new = (show(tip, path) if tip else (ROOT/path).read_text(encoding='utf-8')).splitlines(keepends=True)
        for a, b in changed_ranges(old, new):
            found.setdefault(''.join(old[a:b]), []).append(f'`{path}` lines {a+1}–{b}')
    return found


def write():
    parts = [HEADER.format(base=BASE)]
    for n, (body, sources) in enumerate(blocks(None).items(), 1):
        parts.append(f'### {"; ".join(sources)}\n\n<!-- preserved:agent-cycle-{n:02d}:start -->\n'
                     f'{body}<!-- preserved:agent-cycle-{n:02d}:end -->\n')
    OUT.write_text('\n'.join(parts), encoding='utf-8')
    print(f'{OUT.relative_to(ROOT)}: {len(parts)-1} blocks, {OUT.stat().st_size} bytes')


def check(tip):
    text = OUT.read_text(encoding='utf-8')
    stored = {}
    for match in BLOCK.finditer(text):
        for source in match['src'].split('; '):
            path, start, end = re.fullmatch(r'`(.+)` lines (\d+)–(\d+)', source).groups()
            lines = show(BASE, path).splitlines(keepends=True)[int(start)-1:int(end)]
            if ''.join(lines) != match['body']:
                sys.exit(f'block {match["id"]} differs from {path}:{start}-{end} at base')
            stored.setdefault(path, []).append((int(start)-1, int(end)))
    count = sum(len(v) for v in stored.values())
    if len(BLOCK.findall(text)) != text.count(':start -->'):
        sys.exit('a preserved block is malformed')
    if tip:
        for body, sources in blocks(tip).items():
            for source in sources:
                path, start, end = re.fullmatch(r'`(.+)` lines (\d+)–(\d+)', source).groups()
                if not any(a <= int(start)-1 and int(end) <= b for a, b in stored.get(path, [])):
                    sys.exit(f'{path}:{start}-{end} changed at {tip} but is not preserved')
    print(f'{count} source ranges verbatim at base{"; coverage complete at " + tip if tip else ""}')


if __name__ == '__main__':
    if sys.argv[1:2] == ['--write']:
        write()
    elif sys.argv[1:2] == ['--check']:
        check(sys.argv[2] if len(sys.argv) > 2 else None)
    else:
        sys.exit(__doc__)
