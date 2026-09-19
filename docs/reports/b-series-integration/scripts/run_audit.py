"""Run the pinned corpus from the worktree root into a fresh private directory.

The release example must already be built. Pass a fresh run-directory basename.
The pinned external inputs remain those used by the measured baseline.
"""
import pathlib, subprocess, os, json, datetime, hashlib, sys
root=pathlib.Path.cwd(); out=root/'private/b-series'/sys.argv[1]; out.mkdir(parents=True,exist_ok=False); pin=root/'build/cycle19/c-gateway-wave-pin'; binary=root/'target/release/examples/cycle19_fluent_bit_libraries_audit'
env=dict(os.environ,QCORE_CLANG='/usr/bin/clang',QCORE_CHUNKIO_PACKAGE=str(root/'build/cycle19/provider-packages-wave20/qcore.chunkio')); env.pop('QCORE_SOURCE_CONTRACTS',None)
argv=[str(binary),'--pinned-evidence',str(pin/'fluent-bit-clang-source'),str(pin/'monkey/monkey-build/compile_commands.json'),str(out/'monkey-source.json'),str(out/'resource-diagnostics.json')]
facts={'command':argv,'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'start':datetime.datetime.now(datetime.timezone.utc).isoformat()}
with (out/'audit.log').open('w') as f: facts['exit']=subprocess.call(argv,stdout=f,stderr=subprocess.STDOUT,env=env)
facts['end']=datetime.datetime.now(datetime.timezone.utc).isoformat(); (out/'run.json').write_text(json.dumps(facts,indent=2)+'\n'); print(json.dumps(facts,indent=2)); raise SystemExit(facts['exit'])
