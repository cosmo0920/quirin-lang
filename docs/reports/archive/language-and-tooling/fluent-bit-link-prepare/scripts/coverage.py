"""Coverage of the candidate link map against BODY-SUMMARY-REUSE's census.

usage: coverage.py ARTIFACT CANDIDATE_MAP REFUSALS CENSUS_JSON EXPERIMENT_MAP OUT_JSON

ARTIFACT, CANDIDATE_MAP and REFUSALS are `quirin-audit prepare --link`'s
`link-resolution.json`, `link-map.candidate.json` and
`link-map.candidate.refusals.json`. CENSUS_JSON and EXPERIMENT_MAP are
docs/reports/body-summary-reuse/census.json and link-map-experiment.json.

For the census's cross-unit population (156 callees, 438 sites) and for the
165-home experiment map, it reports which names get a derived home (and
whether that home is the experiment's), which are refused and why, and which
the artifact does not list (no linked object references them across inputs).
A site is also classified by its caller: a caller whose route configuration
is not linked into the executable is outside the candidate premise.
Auxiliary reading of committed and prepared records; it decides nothing.
"""
import collections
import json
import sys

artifact_path, candidate_path, refusals_path, census_path, experiment_path, out = sys.argv[1:7]
artifact = json.load(open(artifact_path))
candidate = json.load(open(candidate_path))
refusals = json.load(open(refusals_path))
census = json.load(open(census_path))
experiment = json.load(open(experiment_path))

derived = {s['function']: s['home_unit'] for s in candidate['symbols']}
refused = {r['function']: (r['reason'], r['detail']) for r in refusals['refused']}
listed = {s['name']: s for s in artifact['symbols']}
experiment_home = {s['function']: s['home_unit'] for s in experiment['symbols']}

# Route configurations by checkout-relative file (the census names caller units
# by file); a file can have several configurations.
linked_route_files = collections.defaultdict(set)
for configuration in artifact['configurations']:
    unit = configuration.get('route_unit')
    if unit:
        file = unit.split('/', 1)[1]
        linked_route_files[file].add(bool(configuration['linked']))


def classify(name):
    if name in derived:
        home = derived[name]
        if name in experiment_home and experiment_home[name] != home:
            return 'derived, home differs from the experiment map', home
        return 'derived', home
    if name in refused:
        return 'refused: ' + refused[name][0], refused[name][1]
    return 'not listed: no linked object references it across inputs as a function', None


def caller(site):
    states = linked_route_files.get(site['unit'])
    if not states:
        return 'caller not a unit of this compile database (Monkey database or unselected)'
    if states == {True}:
        return 'caller linked'
    if states == {False}:
        return 'caller not linked'
    return 'caller has linked and unlinked configurations'


sites = [s for s in census['sites'] if s['reason'] == 'cross-unit']
callees = sorted({s['callee'] for s in sites})
by_callee = {name: classify(name) for name in callees}
callee_table = collections.Counter(by_callee[name][0] for name in callees)
site_table = collections.Counter(by_callee[s['callee']][0] for s in sites)
site_caller_table = collections.Counter(
    (by_callee[s['callee']][0], caller(s)) for s in sites)
experiment_table = collections.Counter(classify(name)[0] for name in experiment_home)
# The census's seven cross-unit sole-blocked functions (its README) and the
# callees blocking them.
SOLE_BLOCKED = {
    'ctxTypeID_to_size': ['LZ4_sizeofState', 'LZ4_sizeofStateHC'],
    'cio_chunk_hash': ['cio_file_hash'],
    'cio_chunk_is_up': ['cio_file_is_up'],
    'mk_mq_send': ['mk_fifo_send'],
    'rd_kafka_share_consumer_closed_err': ['rd_kafka_error_code', 'rd_kafka_error_destroy'],
    'net_plain_writev': ['mk_iov_send'],
    'rd_kafka_transport_ssl_threadid_cb': ['thrd_current_id'],
}
sole = {function: {callee: classify(callee)[0] for callee in callees_of}
        for function, callees_of in SOLE_BLOCKED.items()}

result = {
    'population': {'callees': len(callees), 'sites': len(sites)},
    'callees_by_outcome': dict(sorted(callee_table.items())),
    'sites_by_outcome': dict(sorted(site_table.items())),
    'sites_by_outcome_and_caller': {f'{k[0]} | {k[1]}': v for k, v in sorted(site_caller_table.items())},
    'experiment_map': {'names': len(experiment_home), 'by_outcome': dict(sorted(experiment_table.items()))},
    'sole_blocked': sole,
    'candidate_map': {'derived': len(derived), 'refused': len(refused),
                      'refused_by_reason': dict(sorted(collections.Counter(r[0] for r in refused.values()).items()))},
    'callees': {name: {'outcome': by_callee[name][0], 'detail': by_callee[name][1],
                       'experiment_home': experiment_home.get(name)} for name in callees},
}
json.dump(result, open(out, 'w'), indent=1, sort_keys=False)
print(json.dumps({k: v for k, v in result.items() if k != 'callees'}, indent=1))
