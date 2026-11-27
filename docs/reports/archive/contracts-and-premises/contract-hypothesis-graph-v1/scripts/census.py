#!/usr/bin/env python3
"""Tracked-input census for the selected contextual .qv graph heads."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[4]
BASE = '198f6fc4349d3a47591a2378835ae9286859d918'
HEADS = ['resource_hypothesis', 'resource_premise_all']


def function(text):
    start = text.index('fn is_reserved_source_word(')
    end = text.index('\n}', start) + 2
    return text[start:end]


def main():
    path = 'crates/qcore-frontend/src/frontend.rs'
    old = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT, text=True)
    current = (ROOT / path).read_text()
    old_fn, current_fn = function(old), function(current)
    uncommented = re.sub(r'//[^\n]*', '', current_fn)
    words = sorted(set(re.findall(r'"([A-Za-z_][A-Za-z_0-9]*)"', uncommented)))
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=ROOT).decode().split('\0')
    selected = [name for name in tracked if name and
                (name.endswith(('.qr', '.qv', '.qcore', '.qc')) or
                 ('test' in name and name.endswith('.rs')))]
    matches = {head: [] for head in HEADS}
    for name in selected:
        text = (ROOT / name).read_text()
        for head in HEADS:
            lines = [index for index, line in enumerate(text.splitlines(), 1)
                     if re.search(r'\b' + re.escape(head) + r'\b', line)]
            if lines:
                matches[head].append({'file': name, 'lines': lines})
    result = {'base': BASE, 'tracked_inputs_examined': len(selected),
              'reserved_function_sha256': hashlib.sha256(current_fn.encode()).hexdigest(),
              'reserved_function_byte_identical_to_base': old_fn == current_fn,
              'reserved_words': words, 'reserved_word_count_case_sensitive': len(words),
              'graph_heads_reserved': {head: head in words for head in HEADS},
              'reserved_word_delta': 0 if old_fn == current_fn else None,
              'lexer_byte_identical_to_base': subprocess.check_output(['git', 'show', f'{BASE}:crates/qcore-frontend/src/lexer.rs'], cwd=ROOT) == (ROOT / 'crates/qcore-frontend/src/lexer.rs').read_bytes(),
              'matches': matches,
              'grammar': 'two contextual resource-v1 requires heads, StringLiteral opaque payload and nonempty ordered atom-ID conjunction; no new token or .qr reservation'}
    print(json.dumps(result, indent=2))
    return 0 if result['reserved_function_byte_identical_to_base'] and result['lexer_byte_identical_to_base'] and not any(result['graph_heads_reserved'].values()) else 1


if __name__ == '__main__':
    raise SystemExit(main())
