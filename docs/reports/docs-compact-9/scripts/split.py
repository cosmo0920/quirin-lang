"""Split over-cap increment records at a paragraph boundary, moving the oldest tail verbatim.

For each record, the tail from the first paragraph whose opening matches
`start` moves byte for byte into `<stem>-history.md`; the parent keeps its
head and gains one sentence naming the new page and the cards it holds. The
moved bytes are asserted equal to the base file's tail.
"""
import re
import subprocess
from pathlib import Path

BASE = '8c1f15eb'
D = 'docs/development/increments'
SPLITS = [
    ('wave-45.md', '**`W45-PARENTHESISED-CALLEE`', 'Wave 45', 'wave-44.md'),
    ('wave-44.md', '**`W44-CORPUS-CALLEE-SUMMARIES`', 'Wave 44', 'wave-43.md'),
    ('wave-41.md', '**`CT-PROOF-LEG-3`', 'Wave 41', 'wave-40.md'),
    ('wave-41-continuation.md', '**`CYCLE18-BROADER-INTEGRATION`', 'Wave 41 continuation', 'wave-41.md'),
]
CARD = re.compile(r'^\n?\*\*`?([A-Z][A-Z0-9–-]+)`?|^\n?\*\*\d{4}-\d{2}-\d{2} — ([A-Z0-9-]+)', re.M)

for name, start, label, _ in SPLITS:
    text = subprocess.check_output(['git', 'show', f'{BASE}:{D}/{name}']).decode()
    cut = text.index('\n\n' + start) + 2
    head, tail = text[:cut], text[cut:]
    assert head + tail == text
    cards = []
    for block in tail.split('\n\n'):
        m = CARD.match(block)
        if m:
            cards.append(m[1] or m[2])
    stem = name[:-3]
    new = f'{stem}-history.md'
    header = (f'# {label} records, earliest part\n\n'
              '[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · '
              f'[Later records]({name})\n\n'
              f'> The oldest paragraphs of [{name}]({name}), moved here verbatim at a\n'
              '> paragraph boundary by `DOCS-COMPACT-9` (wave 45) so that page stays below\n'
              '> 32 KiB; nothing was reworded. Counts and observations describe their\n'
              '> recorded dates. Current rules and headline status live in\n'
              '> [AGENTS.md](../../../AGENTS.md) and [STATUS.md](../../STATUS.md); each report\n'
              '> owns its evidence.\n\n')
    Path(D, new).write_text(header + tail)
    names = ', '.join(f'`{c}`' for c in cards)
    note = (f'>\n> **Split at wave 45.** The oldest paragraphs of this record ({names})\n'
            f'> are in [{new}]({new}), moved verbatim by `DOCS-COMPACT-9` to keep\n'
            '> this page below 32 KiB.')
    q = head.index('\n>')
    end = head.index('\n\n', q)
    parent = head[:end] + '\n' + note + head[end:]
    parent = parent.rstrip('\n') + '\n'
    Path(D, name).write_text(parent)
    assert (Path(D, new).read_text()[len(header):]) == tail
    print(name, len(parent.encode()), '->', new, len((header + tail).encode()), cards)
