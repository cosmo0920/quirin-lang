from pathlib import Path
import subprocess,re
root=Path.cwd();proof=root/'proof';pin=(proof/'lean-toolchain').read_text().strip().rsplit(':v',1)[-1]
banner=subprocess.check_output(['lake','env','lean','--version'],cwd=proof,text=True);print(banner,flush=True)
actual=re.search(r'version ([0-9]+\.[0-9]+\.[0-9]+)',banner)
assert actual and actual.group(1)==pin,(pin,banner)
code=subprocess.run(['lake','build','QCore.CmetricsApiContracts'],cwd=proof,timeout=600).returncode
if code:raise SystemExit(code)
raise SystemExit(subprocess.run(['lake','env','lean','AuditPointerStore.lean'],cwd=proof,timeout=120).returncode)
