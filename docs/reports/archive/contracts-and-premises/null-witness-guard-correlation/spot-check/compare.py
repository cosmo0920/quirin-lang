import collections
import json
import sys

FS = {'rd_kafka_metadata_cache_entry_new', 'rd_kafka_ProduceRequest', 'rd_kafka_handle_Produce_metadata_update',
      'rd_kafka_handle_idempotent_Produce_success', 'rd_kafka_topic_info_new_with_rack', 'rd_kafka_msgq_allow_wakeup_at'}
FILES = {'rdkafka_msg.c', 'rdkafka_request.c', 'rdkafka_metadata_cache.c', 'rdkafka_topic.c'}
spot, base = {}, {}
for line in open(sys.argv[1]):
    r = json.loads(line)
    spot[(r['file'].split('/')[-1], r['function'])] = r
for line in open(sys.argv[2]):
    r = json.loads(line)
    if r['library'] == 'librdkafka' and r['file'] in FILES:
        base[(r['file'], r['function'])] = r
print('spot rows', len(spot), 'base rows', len(base), 'common', len(set(spot) & set(base)))
for k in sorted(spot):
    if k[1] in FS:
        r = spot[k]
        print(k[1], 'base', base.get(k, {}).get('null', {}).get('verdict'), '-> spot', r['null']['verdict'], r['combined'])
        for t in r['null']['triage'] or []:
            print('   ', t['check'], t['status'], t['reason'][:300])
moved = collections.Counter()
for k in spot:
    if k in base and spot[k]['null']['verdict'] != base[k]['null']['verdict']:
        moved[(base[k]['null']['verdict'], spot[k]['null']['verdict'])] += 1
        if k[1] not in FS:
            print('OTHER MOVE', k, base[k]['null']['verdict'], '->', spot[k]['null']['verdict'])
print(moved)
statuses = collections.Counter(t['status'] for r in spot.values() for t in (r['null']['triage'] or []))
bstatuses = collections.Counter(t['status'] for r in base.values() for t in (r['null']['triage'] or []))
print('triage spot', statuses, 'base', bstatuses)
