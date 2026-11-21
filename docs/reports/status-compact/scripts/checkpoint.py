#!/usr/bin/env python3
"""STATUS-COMPACT: write, or check, the verbatim status checkpoint.

Usage: checkpoint.py write REV
       checkpoint.py check REV

`write` splits `git show REV:docs/STATUS.md` at record boundaries (a blank
line, or before a table row or heading) into parts under LIMIT bytes where a
record allows it, and writes each verbatim inside a fence into
docs/development/increments/wave-48-compaction-status-partK.md. `check`
re-extracts every part's fenced text and asserts that the parts, concatenated
in order, equal the page at REV byte for byte. Exit 0 only when they do.
"""
import re
import subprocess
import sys
from pathlib import Path

LIMIT = 30_000
DIR = Path('docs/development/increments')
STEM = 'wave-48-compaction-status-part'
FENCE = '~~~~~'
OPEN = FENCE + 'text\n'


def source(rev):
    return subprocess.check_output(['git', 'show', f'{rev}:docs/STATUS.md'])


def split(data):
    lines = data.decode('utf-8').splitlines(keepends=True)
    assert not any(line.startswith(FENCE) for line in lines), 'a line would close the fence'
    cuts = []  # index i: a part may end after line i
    for i, line in enumerate(lines[:-1]):
        nxt = lines[i + 1]
        if line.strip() == '' or nxt.startswith('| ') or nxt.startswith('#'):
            cuts.append(i)
    parts, start, size = [], 0, 0
    last_cut = None
    i = 0
    while i < len(lines):
        size += len(lines[i].encode())
        if i in cuts or i == len(lines) - 1:
            if size > LIMIT and last_cut is not None and last_cut >= start:
                parts.append((start, last_cut))
                start = last_cut + 1
                size = sum(len(x.encode()) for x in lines[start:i + 1])
            last_cut = i
        i += 1
    parts.append((start, len(lines) - 1))
    return lines, parts


def page(k, n, rev, a, b, body):
    nav = ['[Development table of contents](../../../DEVELOPMENT.md)', '[History index](README.md)',
           '[Current status](../../STATUS.md)']
    if k > 1:
        nav.append(f'[Part {k - 1}]({STEM}{k - 1}.md)')
    if k < n:
        nav.append(f'[Part {k + 1}]({STEM}{k + 1}.md)')
    size = len(body.encode())
    return (
        f'# Wave 48 compaction checkpoint: status page, part {k} of {n} (`STATUS-COMPACT`)\n\n'
        + ' · '.join(nav) + '\n\n'
        '> **Preserved checkpoint text, not a current authority.** The fenced block is\n'
        f'> `docs/STATUS.md` lines {a}–{b} at `{rev}`, verbatim ({size:,} bytes), part {k} of the\n'
        '> whole page that `STATUS-COMPACT` replaced with a current-state page.\n'
        f'> `git show {rev}:docs/STATUS.md` reproduces it byte for byte, and\n'
        '> [`checkpoint.py check`](../../reports/status-compact/scripts/checkpoint.py) asserts\n'
        '> that the parts concatenated equal that page. Where this text and the current\n'
        '> page disagree, **the current page wins**; figures here describe their recorded\n'
        '> checkpoints. Relative links inside keep their original base, `docs/`.\n\n'
        + OPEN + body + FENCE + '\n'
    )


def write(rev):
    data = source(rev)
    lines, parts = split(data)
    for old in DIR.glob(f'{STEM}*.md'):
        old.unlink()
    for k, (s, e) in enumerate(parts, 1):
        body = ''.join(lines[s:e + 1])
        (DIR / f'{STEM}{k}.md').write_text(page(k, len(parts), rev, s + 1, e + 1, body), encoding='utf-8')
        print(f'part {k}: lines {s + 1}-{e + 1}, {len(body.encode()):,} bytes')
    check(rev)


def check(rev):
    data = source(rev)
    files = sorted(DIR.glob(f'{STEM}*.md'), key=lambda p: int(re.search(r'(\d+)\.md$', p.name)[1]))
    joined = b''
    for f in files:
        text = f.read_text(encoding='utf-8')
        body = text.split(OPEN, 1)[1]
        assert body.endswith(FENCE + '\n'), f
        joined += body[:-len(FENCE + '\n')].encode('utf-8')
    if joined != data:
        sys.exit(f'checkpoint differs from {rev}:docs/STATUS.md ({len(joined)} vs {len(data)} bytes)')
    print(f'OK: {len(files)} parts reproduce {rev}:docs/STATUS.md, {len(data):,} bytes')


if __name__ == '__main__':
    {'write': write, 'check': check}[sys.argv[1]](sys.argv[2])
