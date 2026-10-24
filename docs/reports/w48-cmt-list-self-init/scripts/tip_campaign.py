from pathlib import Path
import json,subprocess,hashlib,shutil,os
root=Path.cwd();p=root/'private/w48-cmt-member-storage';freeze=json.loads((p/'semantic-freeze.json').read_text())['head'];gate=json.loads((p/'latest-gate.json').read_text());assert gate['exit']==0 and gate['head']==freeze
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()==freeze
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
slot=json.loads((p/'development-lock.json').read_text());ancestor=os.getppid()
while ancestor>1 and ancestor!=slot['pid']:ancestor=int(Path(f'/proc/{ancestor}/stat').read_text().rsplit(')',1)[1].split()[1])
assert ancestor==slot['pid']
build=json.loads((p/'steps/build-tip-1.json').read_text());assert build['exit']==0 and build['head']==freeze and 'QCORE_AUDIT_BUILD_COMMIT='+freeze in build['command']
runtime=p/'tip-runtime';(runtime/'target/release').mkdir(parents=True,exist_ok=False);(runtime/'scripts').mkdir();binary=runtime/'target/release/quirin-audit';shutil.copyfile(root/'target/release/quirin-audit',binary);binary.chmod(0o755);runner=runtime/'scripts/run_pinned_audit.py';runner.write_bytes((p/'inputs/scripts/run_pinned_audit.py').read_bytes())
(p/'tip-binary.json').write_text(json.dumps({'source':freeze,'build':build,'sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'binary':str(binary),'campaign_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},indent=2)+'\n')
code=subprocess.run(['sh','docs/reports/w48-bu-cmt-map-metric-get/scripts/original_paths.sh','python3',str(p/'measure.py'),'tip']).returncode
(p/'tip-complete.exit').write_text(str(code)+'\n');raise SystemExit(code)
