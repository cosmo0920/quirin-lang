import gzip,hashlib,json,pathlib
p=pathlib.Path('private/premise-symbolic-integration');report=pathlib.Path('docs/reports/premise-symbolic-integration');manifest=[]
sha=lambda f:hashlib.file_digest(pathlib.Path(f).open('rb'),'sha256').hexdigest()
def copy(src,relative,compress=False):
 src=pathlib.Path(src);target=report/relative;assert src.is_file() and not target.exists();target.parent.mkdir(parents=True,exist_ok=True);b=src.read_bytes();target.write_bytes(gzip.compress(b,mtime=0) if compress else b);manifest.append({'private_source':str(src),'published':str(target),'source_sha256':sha(src),'published_sha256':sha(target),'gzip':compress})
# Explicit diagnostic/public-provenance whitelist only; canonical source-pair and
# receipt payloads stay private, with exact path+SHA/size inventories below.
for gate_name in ['joint-gate-2','joint-gate-3']:
 for f in sorted((p/gate_name).iterdir()):
  if f.is_file() and f.suffix in {'.log','.json','.exit'}:copy(f,pathlib.Path('validation')/gate_name/(f.name+('.gz' if f.suffix=='.log' else '')),f.suffix=='.log')
for directory in ['tip-collector-1','postgate-source-1']:
 for f in sorted((p/directory).iterdir()):
  if f.is_file() and f.suffix in {'.log','.json'}:copy(f,pathlib.Path('receipts')/directory/(f.name+('.gz' if f.suffix=='.log' else '')),f.suffix=='.log')
copy(p/'tip-freeze/receipt.json','receipts/tip-freeze.json')
copy(p/'joint-gate-3/clang.log','clang-version.txt');copy(p/'joint-gate-3/clang.exit','clang-version.exit')
for route in ['system','slice','corpus']:
 for name in ['invocation.json','copy-manifest.json']:copy(p/'prepared-tip'/route/name,pathlib.Path('native/tip')/route/name)
for f in sorted(p.glob('tip-route-summary-*.json')):copy(f,pathlib.Path('receipts')/f.name)
# These final comparator artifacts contain only route/status/leaf counts,
# flags, proof hashes and a short scope qualification, not canonical payloads.
for route,directory in [('system','comparison-configured-1'),('slice','comparison-configured-1'),('corpus','comparison-corpus-1')]:
 f=p/directory/route/'final-comparison.json';v=json.loads(f.read_bytes())
 assert v['all_existing_fields_equal_after_exact_time_identity_attribution'] and v['all_fields_retained'] and not any(v['unexplained_remaining_leaf_counts'].values())
 assert set(v['base']) <= {'Safe','Pending','Unknown','Unsafe'} and set(v['tip']) <= {'Safe','Pending','Unknown','Unsafe'}
 assert len(f.read_bytes()) < 20000 and 'requires resource_' not in f.read_text()
 copy(f,pathlib.Path('comparisons')/route/'final-comparison.json')
for name in ['commands.json','receipt.json']:
 copy(p/'comparator-helpers'/name,pathlib.Path('receipts/comparator-helpers')/name)
private=[]
for directory in ['measure-1','prepared-tip','comparator-helpers','comparison-configured-1','comparison-corpus-1','unknown-pin-negative-1']:
 d=p/directory
 if not d.exists():continue
 for f in sorted(d.rglob('*')):
  if f.is_file():private.append({'path':str(f),'sha256':sha(f),'bytes':f.stat().st_size})
(report/'private-artifact-manifest.json').write_text(json.dumps(private,indent=2)+'\n')
(report/'publication-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n');print('Published whitelist files',len(manifest),'private artifacts inventoried',len(private))
