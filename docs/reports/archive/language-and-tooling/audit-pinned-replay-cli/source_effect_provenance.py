"""Verify the three cwd-bound boolean-import identity cascades in pinned parity."""
import copy
import hashlib
import json
import pathlib
import shlex


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode()


def ordered_identity(value):
    fields = ('raw_source_digest', 'preprocessed_source_digest',
              'configuration_digest', 'dialect', 'source_contract_digest',
              'profile_digest', 'model_library', 'source_revision', 'compiler',
              'helpers')
    result = {key: value[key] for key in fields}
    result['compiler'] = {key: value['compiler'][key]
                          for key in ('program', 'version', 'target')}
    result['helpers'] = {key: ordered_identity(child)
                         for key, child in sorted(value['helpers'].items())}
    return result


def ordered_bounds(value):
    return {key: value[key] for key in ('path_exploration', 'schedules_explored',
                                        'observed_runs_only', 'max_operations')}


def ordered_task(task):
    unit = {key: task['unit'][key] for key in
            ('source_digest', 'build_digest', 'translation_unit')}
    target = {key: task['target'][key] for key in
              ('library', 'revision', 'profile', 'dialect', 'platform', 'backend')}
    obligations = []
    for obligation in task['obligations']:
        subject = {key: obligation['subject'][key] for key in
                   ('subject', 'symbol', 'translation_unit')}
        scope = {key: obligation['scope'][key] for key in ('scope', 'symbols')}
        obligations.append({
            'obligation_id': obligation['obligation_id'],
            'capability': obligation['capability'],
            'property': obligation['property'],
            'subject': subject,
            'scope': scope,
            'bounds': ordered_bounds(obligation['bounds']),
            'assumptions': obligation['assumptions'],
            'evidence_version': obligation['evidence_version'],
        })
    return {
        'task_version': task['task_version'], 'unit': unit, 'target': target,
        'semantic_profile': task['semantic_profile'],
        'contract_sources': task['contract_sources'],
        'bounds': ordered_bounds(task['bounds']),
        'assumptions': task['assumptions'],
        'evidence_version': task['evidence_version'],
        'obligations': obligations,
        'derivation_incomplete': task['derivation_incomplete'],
    }


def build_configuration(database, compiler, cwd, source_file):
    entries = json.loads(pathlib.Path(database).read_bytes())
    matches = [item for item in entries if item['file'].endswith('/' + source_file)]
    if not matches:
        raise ValueError(f'missing pinned compile entry for {source_file}')
    # The pinned route keeps the first matching CMake entry (static before shared).
    entry = matches[0]
    words = shlex.split(entry['command'])
    include_paths = [str(pathlib.Path(entry['file']).parent)]
    definitions = []
    dialect = '-std=gnu17'
    cursor = 1
    while cursor < len(words):
        word = words[cursor]
        if word in ('-I', '-isystem'):
            cursor += 1
            path = words[cursor]
            include_paths.append(str(pathlib.Path(path) if pathlib.Path(path).is_absolute()
                                     else pathlib.Path(entry['directory']) / path))
        elif word.startswith('-I'):
            path = word[2:]
            include_paths.append(str(pathlib.Path(path) if pathlib.Path(path).is_absolute()
                                     else pathlib.Path(entry['directory']) / path))
        elif word == '-D':
            cursor += 1
            if '$' not in words[cursor]:
                definitions.append(words[cursor])
        elif word.startswith('-D'):
            if '$' not in word[2:]:
                definitions.append(word[2:])
        elif word.startswith('-std='):
            dialect = word
        cursor += 1
    clang = {key: compiler[key] for key in ('program', 'version', 'target')}
    value = [dialect, include_paths, definitions, 'main_source_definitions',
             cwd, clang]
    return sha(encoded(value)), len(include_paths), len(definitions)


def check_chain(row, configuration_digest):
    identity = row['import_identity']
    if identity['configuration_digest'] != configuration_digest or any(
            helper['configuration_digest'] != configuration_digest
            for helper in identity['helpers'].values()):
        return False
    build = sha(encoded(ordered_identity(identity)))
    task = row['task']
    if task['unit']['build_digest'] != build or \
            task['contract_sources']['qcore:boolean-field-import:v1'] != build or \
            row['evidence']['unit']['build_digest'] != build:
        return False
    if len(task['obligations']) != 1:
        return False
    obligation = task['obligations'][0]
    subject = obligation['subject']
    unit = task['unit']
    parts = [str(task['task_version']), unit['source_digest'], build,
             unit['translation_unit'], task['semantic_profile'],
             obligation['capability'], obligation['property'],
             'function|' + subject['translation_unit'] + '|' + subject['symbol']]
    obligation_id = obligation['capability'] + ':' + sha('\x1f'.join(parts).encode())[:24]
    if obligation['obligation_id'] != obligation_id or \
            row['evidence']['coverage'] != [obligation_id] or \
            list(row['bounds']) != [obligation_id]:
        return False
    return row['witness']['task_digest'] == sha(encoded(ordered_task(task)))


def normalize(row):
    row = copy.deepcopy(row)
    identity = row['import_identity']
    identity['configuration_digest'] = '<cwd-bound>'
    for helper in identity['helpers'].values():
        helper['configuration_digest'] = '<cwd-bound>'
    row['task']['unit']['build_digest'] = '<derived-build>'
    row['task']['contract_sources']['qcore:boolean-field-import:v1'] = '<derived-build>'
    row['evidence']['unit']['build_digest'] = '<derived-build>'
    row['task']['obligations'][0]['obligation_id'] = '<derived-obligation>'
    row['evidence']['coverage'] = ['<derived-obligation>']
    row['bounds'] = {'<derived-obligation>': next(iter(row['bounds'].values()))}
    row['witness']['task_digest'] = '<derived-task>'
    return row


def compare_source_effects(before, after, archive_cwd, measured_cwd, database):
    old = json.loads(before)
    new = json.loads(after)
    expected = {'cio_chunk_is_locked', 'cio_chunk_unlock',
                'flb_input_chunk_release_lock'}
    changed = []
    details = []
    if old.keys() != new.keys() or len(old['results']) != len(new['results']):
        return {'passes': False, 'reason': 'envelope or result count changed'}
    if {k: v for k, v in old.items() if k != 'results'} != \
            {k: v for k, v in new.items() if k != 'results'}:
        return {'passes': False, 'reason': 'source-effect envelope changed'}
    for index, (left, right) in enumerate(zip(old['results'], new['results'])):
        if left == right:
            continue
        name = left['function']
        changed.append(name)
        old_config, include_count, definition_count = build_configuration(
            database, left['import_identity']['compiler'], archive_cwd, left['file'])
        new_config, _, _ = build_configuration(
            database, right['import_identity']['compiler'], measured_cwd, right['file'])
        valid = (name == right['function'] and name in expected and
                 check_chain(left, old_config) and check_chain(right, new_config) and
                 normalize(left) == normalize(right))
        details.append({
            'index': index, 'function': name, 'passes': valid,
            'archived_configuration_digest': old_config,
            'migrated_configuration_digest': new_config,
            'include_paths': include_count, 'definitions': definition_count,
            'status_before': left['status'], 'status_after': right['status'],
            'semantic_profile_before': left['task']['semantic_profile'],
            'semantic_profile_after': right['task']['semantic_profile'],
            'assumptions_equal': left['task']['assumptions'] == right['task']['assumptions'],
            'evidence_version_equal': left['task']['evidence_version'] == right['task']['evidence_version'],
        })
    return {
        'archive_sha256': sha(before), 'migrated_sha256': sha(after),
        'raw_byte_identical': before == after,
        'result_count_before': len(old['results']),
        'result_count_after': len(new['results']),
        'changed_functions': changed, 'cwd_only_identity_cascades': details,
        'passes': set(changed) == expected and len(changed) == 3 and
                  all(item['passes'] for item in details),
    }
