from pathlib import Path
import subprocess,json
p=Path('private/w48-cmt-map')
commands=[]
# This queued slot has not started. Retain its original script and repair the
# failed corpus first using a new complete snapshot, then take both configured
# baselines as originally scheduled. All subprocesses retain the same flock.
code=subprocess.run(['python3',str(p/'retry_base_corpus_v3.py')]).returncode
if code:
    (p/'base-configured-v3.exit').write_text(str(code)+'\n')
    raise SystemExit(code)
for route in ('system','slice'):
    cmd=['python3','docs/reports/w48-bu-cmt-map-metric-get/scripts/measure_configured.py',route,str(p/('base-'+route+'-v3')), '--binary',str(p/'base-inputs-v2/target/release/quirin-audit'), '--snapshot',str(p/'base-inputs-v2'),'--semantic-source','8991b7b58fcbd33da082bbb44e819d47a0fb2561','--link-map','docs/reports/detach-result-wrapper-composition/link-map.corpus.json','--primary-root','/home/cosmo/Documents/GitHub/qcore-c']
    with (p/('base-'+route+'-v3-driver.log')).open('wb') as log:
        code=subprocess.run(cmd,stdout=log,stderr=log).returncode
    (p/('base-'+route+'-v3.exit')).write_text(str(code)+'\n')
    commands.append({'route':route,'command':cmd,'exit':code})
(p/'base-configured-v3-jobs.json').write_text(json.dumps(commands,indent=2)+'\n')
code=0 if all(x['exit']==0 for x in commands) else 101
(p/'base-configured-v3.exit').write_text(str(code)+'\n')
raise SystemExit(code)
