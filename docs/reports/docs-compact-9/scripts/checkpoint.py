"""Write the wave-45 compaction checkpoints: every moved base range, verbatim.

Rule pages come from the apply_ranges specs; STATUS.md from the ranges
new_status.py replaced. Each block is re-read from `git show BASE:file` and
asserted to be a contiguous substring of it, so a checkpoint can only hold
text that was in the base file, byte for byte.
"""
import json
import subprocess
from pathlib import Path

BASE = '8c1f15eb'
D = Path('docs/development/increments')
FENCE = '~~~~~'


def base_lines(path):
    return subprocess.check_output(['git', 'show', f'{BASE}:{path}']).decode().split('\n')


def block(path, a, b):
    lines = base_lines(path)
    text = '\n'.join(lines[a - 1:b])
    whole = '\n'.join(lines)
    assert text in whole and FENCE not in text
    return text


NOTICE = ('> **Preserved checkpoint text. These blocks are not a current authority.**\n'
          '> They are the verbatim text `DOCS-COMPACT-9` compacted out of {what} at\n'
          '> base `8c1f15eb`, kept so that nothing the compaction removed is lost. Each\n'
          '> block names its base lines; `git show 8c1f15eb:<file>` reproduces it byte for\n'
          '> byte. Where a block and the current page disagree, **the current page wins**;\n'
          '> where a block states something the card measured to be false, the correction\n'
          '> is in [the card report](../../reports/docs-compact-9/README.md) and is\n'
          '> deliberately not applied here. Relative links inside a block keep their\n'
          '> original base, the page they were written in.\n')


def render(title, what, sections, siblings):
    out = [f'# {title}\n',
           '[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · '
           + ' · '.join(siblings) + '\n', NOTICE.format(what=what)]
    n = 0
    for anchor, heading, current, items in sections:
        out.append(f'<a id="{anchor}"></a>\n\n## {heading}\n\nCurrent page: [{current}]({current}).\n')
        for path, a, b, t, note in items:
            n += 1
            text = block(path, a, b)
            size = len(text.encode())
            out.append(f'### {n}. {t}\n\n`{path}` lines {a}–{b} at `8c1f15eb`, {size:,} bytes. {note}\n\n'
                       f'{FENCE}text\n{text}\n{FENCE}\n')
    return '\n'.join(out)


def rule_items(spec_path):
    spec = json.loads(Path(spec_path).read_text())
    items = []
    for e in spec['edits']:
        for a, b in e['moved']:
            items.append((spec['file'], a, b, e.get('title', 'Moved lines'), e.get('note', '')))
    return spec, items


sections = []
for s in ['docs/reports/docs-compact-9/artifacts/spec-gateway.json', 'docs/reports/docs-compact-9/artifacts/spec-diagnostics.json', 'docs/reports/docs-compact-9/artifacts/spec-buffers.json']:
    spec, items = rule_items(s)
    rel = '../../agents/' + Path(spec['file']).name
    sections.append((spec['section'], spec['heading'], rel, items))
rules = render('Wave 45 compaction checkpoints: rule pages (`DOCS-COMPACT-9`)',
               'the rule topic pages', sections,
               ['[Status checkpoint](wave-45-compaction-status.md)', '[Wave 45 records](wave-45-continuation.md)'])
(D / 'wave-45-compaction-checkpoints.md').write_text(rules)

status_titles = json.loads(Path('docs/reports/docs-compact-9/artifacts/status-titles.json').read_text())
ranges = json.loads(Path('docs/reports/docs-compact-9/artifacts/status-ranges.json').read_text())
items = []
for a, b in ranges:
    t, note = status_titles[f'{a}-{b}']
    items.append(('docs/STATUS.md', a, b, t, note))
table = [i for i in items if i[1] < 60]
prose = [i for i in items if i[1] >= 60]
status = render('Wave 45 compaction checkpoint: status page (`DOCS-COMPACT-9`)',
                '`docs/STATUS.md`', [('status', '`docs/STATUS.md`: provenance and coverage table', '../../STATUS.md', table)],
                ['[Status prose checkpoint](wave-45-compaction-status-prose.md)', '[Rule-page checkpoints](wave-45-compaction-checkpoints.md)', '[Wave 45 records](wave-45-continuation.md)'])
(D / 'wave-45-compaction-status.md').write_text(status)
status2 = render('Wave 45 compaction checkpoint: status page prose (`DOCS-COMPACT-9`)',
                 '`docs/STATUS.md`', [('status-prose', '`docs/STATUS.md`: prose sections', '../../STATUS.md', prose)],
                 ['[Status table checkpoint](wave-45-compaction-status.md)', '[Rule-page checkpoints](wave-45-compaction-checkpoints.md)', '[Wave 45 records](wave-45-continuation.md)'])
(D / 'wave-45-compaction-status-prose.md').write_text(status2)
for f in ['wave-45-compaction-checkpoints.md', 'wave-45-compaction-status.md', 'wave-45-compaction-status-prose.md']:
    print(f, (D / f).stat().st_size)
