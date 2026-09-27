import csv, json, os, pathlib, re, subprocess, sys
root=pathlib.Path.cwd()
rows=list(csv.DictReader((root/'docs/reports/extension-compatibility/mapping.tsv').open(),delimiter='\t'))
mapping={r['old_path']:r['proposed_new_path'] for r in rows}
paths=subprocess.check_output(['git','ls-files','-z','*.md'],text=True).split('\0')
changes=[]
for name in filter(None,paths):
    if name.startswith('docs/reports/extension-'):
        continue # These reports explicitly record before/after mappings.
    path=root/name
    text=path.read_text()
    updates=[]
    def replace(match):
        target=match.group(1)
        wrapped=target.startswith('<') and target.endswith('>')
        actual=target[1:-1] if wrapped else target
        if '://' in actual or actual.startswith(('/', '#')):
            return match.group()
        original,mark,anchor=actual.partition('#')
        resolved=os.path.normpath(os.path.join(os.path.dirname(name),original))
        if resolved not in mapping:
            return match.group()
        new=os.path.relpath(mapping[resolved],os.path.dirname(name) or '.')
        if original.startswith('./') and not new.startswith('.'):
            new='./'+new
        if mark: new+='#'+anchor
        updates.append({'old':actual,'new':new,'source':resolved})
        if wrapped: new='<'+new+'>'
        return ']('+new+')'
    updated=re.sub(r'\]\(([^)\n]+)\)',replace,text)
    if updates:
        changes.append({'document':name,'links':updates})
        if '--apply' in sys.argv:
            path.write_text(updated)
print(json.dumps(changes,indent=2))
