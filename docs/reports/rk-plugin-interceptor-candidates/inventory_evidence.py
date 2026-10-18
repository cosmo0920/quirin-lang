#!/usr/bin/env python3
"""Check the reviewed candidate inventory against both named Git source trees.

Source reading and neutral-custody review remain premises, not proved by these
structural anchors. The analyzer owns inventory validation and source pinning.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

repo = Path(__file__).resolve().parents[3]
inventory = repo / 'providers/qcore-provider-librdkafka/outer-candidates.json'
data = json.loads(inventory.read_text())
assert len(sys.argv) == 3, 'provide both audited source roots'
results = []
for root, identity in zip(sys.argv[1:], data['reviewed_identities']):
    revision = subprocess.check_output(['git', '-C', root, 'rev-parse', 'HEAD'], text=True).strip()
    assert revision == identity['reviewed_revision']
    files = {}
    for path, expected in identity['sources'].items():
        content = subprocess.check_output(['git', '-C', root, 'show', 'HEAD:' + path])
        assert hashlib.sha256(content).hexdigest() == expected, path
        files[Path(path).name] = content.decode()
    assert 'method->ic_opaque = ic_opaque;' in files['rdkafka_interceptor.c']
    assert 'method->u.generic = func;' in files['rdkafka_interceptor.c']
    method_record = files['rdkafka_interceptor.c'].split('} rd_kafka_interceptor_method_t;', 1)[0]
    arms = set(re.findall(r'rd_kafka_interceptor_f_(on_\w+)_t', method_record))
    assert arms == {candidate['callback_member'].split('.')[-1] for candidate in data['candidates']}
    records = []
    for candidate in data['candidates']:
        arm = candidate['callback_member'].split('.')[-1]
        assert 'rd_list_t ' + arm + ';' in files['rdkafka_conf.h']
        assert '*'+arm+';' in re.sub(r'\s+', '', files['rdkafka_interceptor.c'])
        assert candidate['callback_signature'] in ' '.join(files['rdkafka.h'].split())
        match = re.search(re.escape(candidate['setter']) + r'\s*\(.*?\n\}', files['rdkafka_interceptor.c'], re.S)
        assert match, candidate['setter']
        body = match.group(0)
        assert 'interceptors.' + arm in body and '(void *)' + arm in body
        assert 'return rd_kafka_interceptor_method_add' in ' '.join(body.split())
        if candidate['lifecycle'] != 'configuration':
            assert 'assert(!rk->rk_initialized)' in body
            assert 'RD_KAFKA_INTERCEPTOR_REJECT_IF_SHARE(rk)' in body
        records.append({'list': candidate['list_member'], 'union_arm': candidate['callback_member'], 'setter': candidate['setter'], 'signature': candidate['callback_signature'], 'lifecycle': candidate['lifecycle'], 'registration_lifetime': candidate['registration_lifetime'], 'source_pins': 'match'})
    results.append({'revision': revision, 'sources': len(files), 'candidates': records, 'excluded_non_callback_list': 'rd_kafka_conf_s.interceptors.config holds rd_strtup_t configuration values, not public interceptor callback registrations'})
print(json.dumps({'inventory_sha256': hashlib.sha256(inventory.read_bytes()).hexdigest(), 'audited': results}, indent=2))
