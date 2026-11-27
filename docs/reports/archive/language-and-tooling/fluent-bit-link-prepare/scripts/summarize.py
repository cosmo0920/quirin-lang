"""Print the headline facts of one `prepare --link` output directory.

usage: summarize.py LINK_RESOLUTION_DIR
"""
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
artifact = json.loads((root / 'link-resolution.json').read_text())
counts = artifact['counts']
print(json.dumps({
    'executable_sha256': artifact['executable']['sha256'],
    'compile_database_sha256': artifact['compile_database']['sha256'],
    'artifact_sha256': hashlib.sha256((root / 'link-resolution.json').read_bytes()).hexdigest(),
    'candidate_map_sha256': hashlib.sha256((root / 'link-map.candidate.json').read_bytes()).hexdigest(),
    'refusals_sha256': hashlib.sha256((root / 'link-map.candidate.refusals.json').read_bytes()).hexdigest(),
    'map_sha256': artifact['link']['map_sha256'],
    'link_txt_sha256': artifact['link']['link_txt_sha256'],
    'build_seconds': artifact['link']['build_seconds'],
    'relinked_equal': artifact['link']['relinked_equal'],
    'counts': {k: v for k, v in counts.items() if k not in ('shared_objects', 'linker_scripts')},
}, indent=1))
