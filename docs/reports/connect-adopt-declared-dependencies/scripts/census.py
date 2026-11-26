#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card CONNECT-ADOPT-DECLARED-DEPENDENCIES: assemble census.json from the
readings recorded below (load paths, locks, Cargo dependencies). Usage:
`python3 census.py <repo root> <out.json>`.
"""
import json, os, sys, collections
root = sys.argv[1]
out = sys.argv[2]
P = os.path.join(root, 'providers')
NS = {}
for f in ('contract-load-path.json', 'user-crates-load-path.json'):
    for c in json.load(open(os.path.join(P, f)))['crates']:
        NS[c['name']] = (c['namespace'], c['version'], c.get('dependencies', []), f)

def lock(crate):
    p = os.path.join(P, crate, 'contracts.lock')
    if not os.path.exists(p): return {}
    d = json.load(open(p))
    m = {}
    for k in ('pinned', 'flexible', 'tip'):
        for e in d.get(k, []) or []:
            m.setdefault(e['function'], []).append({'mode': k, 'module': e['module'], 'contract_sha256': e['contract_sha256']})
    return m
LOCKS = {c: lock(c) for c in NS}

FB, RK, EV, CH, LC = ('quirin-provider-fluent-bit', 'qcore-provider-librdkafka',
                      'qcore-provider-libevent', 'qcore-provider-chunkio', 'quirin-provider-libc')
K_INH = 'inherit'
K_BODY = 'body-checked-specification'
K_FRAME = 'call-frame-of-another-crates-function'
edges = []
def e(kind, dc, dm, df, tc, tf, reading, site):
    edges.append({'kind': kind, 'dependent_crate': dc, 'dependent_module': dm,
                  'dependent': df, 'target_crate': tc, 'target': NS[tc][0] + '::' + tf,
                  'reading': reading, 'site': site})
INH = 'C15 inheritance: the child elaborates to the parent\'s clauses (contract_inclusion.rs), so every discharge through the child reads the parent clause'
for f, p in [('flb_malloc', 'malloc'), ('flb_calloc', 'calloc'), ('flb_free', 'free')]:
    e(K_INH, FB, 'fluent_bit/resource_memory.qv', f, LC, p, INH, 'contract inherit libc::' + p)
for f, p in [('flb_strdup', 'strdup'), ('flb_strndup', 'strndup')]:
    e(K_INH, FB, 'fluent_bit/resource_string.qv', f, LC, p, INH, 'contract inherit libc::' + p)
for f, p in [('rd_malloc', 'malloc'), ('rd_strndup', 'strndup'), ('rd_free', 'free')]:
    e(K_INH, RK, 'librdkafka/memory.qv', f, LC, p, INH, 'contract inherit libc::' + p)
for f, p in [('mk_mem_alloc', 'malloc'), ('mk_mem_alloc_z', 'malloc'), ('mk_mem_free', 'free')]:
    e(K_INH, EV, 'monkey/memory.qv', f, LC, p, INH, 'contract inherit libc::' + p)
R37 = 'rule 37 checked specification: resource_return_owned(nonnull) is established against the wrapper body, whose allocation is read through the libc declaration (owned nullable) before the asserted failure arm'
e(K_BODY, RK, 'librdkafka/memory.qv', 'rd_calloc', LC, 'calloc', R37, 'rd.h:139-143 calloc(num, sz)')
e(K_BODY, RK, 'librdkafka/memory.qv', 'rd_strdup', LC, 'strdup', R37, 'rd.h:161-169 strdup(s) (_strdup on Windows: undeclared, Unknown)')
SB = 'resource_body_checked: the specification is withdrawn unless the gateway body reading of plugins/in_storage_backlog/sb.c discharges it, and that reading applies the callee declaration at this call'
sb = {
    'sb_allocate_backlogs': [(EV, 'mk_list_add')],
    'sb_destroy_backlogs': [(EV, 'mk_list_del')],
    'sb_remove_chunk_from_segregated_backlog': [(EV, 'mk_list_del')],
    'sb_append_chunk_to_segregated_backlog': [(EV, 'mk_list_add')],
    'sb_append_chunk_to_segregated_backlogs': [(LC, 'memset')],
    'sb_segregate_chunks': [(CH, x) for x in ('cio_chunk_close', 'cio_chunk_down', 'cio_chunk_get_content_size', 'cio_chunk_is_up', 'cio_chunk_up_force')],
    'sb_release_output_queue_space': [(CH, 'cio_chunk_close')],
    'cb_queue_chunks': [(CH, x) for x in ('cio_chunk_close', 'cio_chunk_down', 'cio_chunk_get_content_size', 'cio_chunk_is_up', 'cio_chunk_up_force')],
}
for f, ts in sb.items():
    for tc, tf in ts:
        e(K_BODY, FB, 'fluent_bit/storage_backlog.qv', f, tc, tf, SB, 'sb.c ' + f + ' calls ' + tf)
FR = 'quant cell FN: TYPE |-> #untouched (G05 frame_clause) is applied by legacy_cell_frames at every call of FN; a Safe across that call rests on the frame and on FN\'s own declaration together'
e(K_FRAME, FB, 'fluent_bit/teardown_destroy.qv', 'flb_config_set_program_name', EV, 'mk_list_del', FR,
  'quant cell mk_list_del: flb_input_instance |-> #untouched; quant cell mk_list_del: flb_output_instance |-> #untouched')

for x in edges:
    tns, tfn = x['target'].split('::')
    x['target_identity'] = LOCKS[x['target_crate']].get(tfn, 'UNPINNED')
    x['target_version'] = NS[x['target_crate']][1]
    x['dependent_lock'] = LOCKS[x['dependent_crate']].get(x['dependent'], 'UNPINNED')
    x['cargo_dependency_present'] = x['target_crate'] in NS[x['dependent_crate']][2]
    x['target_has_library_target'] = os.path.exists(os.path.join(P, x['target_crate'], 'src', 'lib.rs'))
    x['satisfiable_under_c15_2'] = x['cargo_dependency_present']

pairs = collections.Counter((x['dependent_crate'], x['target_crate']) for x in edges)
kinds = collections.Counter(x['kind'] for x in edges)
decls = sorted({(x['dependent_crate'], x['dependent']) for x in edges})
doc = {
    'schema': 'connect-adopt-census/1',
    'base': '1ecaf97e9',
    'load_paths': ['providers/contract-load-path.json', 'providers/user-crates-load-path.json'],
    'edges': edges,
    'counts': {
        'edges': len(edges),
        'dependent_declarations_to_repin': len(decls),
        'per_crate_pair': [{'dependent_crate': a, 'target_crate': b, 'edges': n,
                            'declarations': len({x['dependent'] for x in edges if (x['dependent_crate'], x['target_crate']) == (a, b)})}
                           for (a, b), n in sorted(pairs.items())],
        'per_kind': dict(sorted(kinds.items())),
        'unsatisfiable_edges': sum(not x['satisfiable_under_c15_2'] for x in edges),
        'inherit_edges_blocked_by_qi002': sum(x['kind'] == K_INH for x in edges),
    },
}
json.dump(doc, open(out, 'w'), indent=2)
open(out, 'a').write('\n')
print(json.dumps(doc['counts'], indent=1))
