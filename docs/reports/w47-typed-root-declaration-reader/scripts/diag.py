"""Run the temporary-trace diagnostic binary over the pinned corpus (not a measurement)."""
import os, pathlib, subprocess
W = pathlib.Path('/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab69b07fd0abe4b3d')
PIN = pathlib.Path('/home/cosmo/Documents/GitHub/qcore-c/build/cycle19')
import sys
out = W / 'private/w47tr' / sys.argv[1]
out.mkdir(parents=True, exist_ok=False)
env = dict(os.environ)
env['W47TR_PREMISE'] = '1'
env['W47TR_FUNCS_UNUSED'] = ','.join([
    'rd_kafka_messages_get', 'rd_kafka_opaque', 'rd_kafka_ConfigEntry_name', 'rd_kafka_Node_host',
    'rd_kafka_Node_rack', 'rd_kafka_handle_mock_cluster', 'rd_kafka_mock_cluster_bootstraps',
    'rd_kafka_mock_cluster_handle', 'rd_kafka_share_partition_offsets_list_get',
    'rd_kafka_share_partition_offsets_offsets', 'rd_kafka_topic_partition_get_leader_epoch',
    'rd_kafka_topic_partition_get_current_leader_epoch'])
env['QCORE_CLANG'] = '/usr/bin/clang'
env['QCORE_CHUNKIO_PACKAGE'] = str(PIN / 'provider-packages-wave20/qcore.chunkio')
env.pop('QCORE_SOURCE_CONTRACTS', None)
cmd = [str(W / 'private/w47tr/target-diag/release/quirin-audit'), 'pinned', '--jobs', '5',
       str(PIN / 'c-gateway-wave-pin/fluent-bit-clang-source'),
       str(PIN / 'c-gateway-wave-pin/monkey/monkey-build/compile_commands.json'),
       str(out / 'monkey-source.json'), str(out / 'resource-diagnostics.json')]
with open(out / 'audit.log', 'wb') as log:
    code = subprocess.run(cmd, env=env, stdout=log, stderr=log).returncode
(out / 'exit').write_text(f'{code}\n')
print(code)
