import shutil
W = '/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab69b07fd0abe4b3d/'
shutil.copyfile(W + 'private/w47tr/ltp.orig', W + 'crates/qcore-c-gateway/src/legacy_typed_positions.rs')
p = W + 'crates/qcore-c-gateway/src/typed_access_positions.rs'
s = open(p).read()
s = s.replace("    fn expression_node(&self", "    pub(crate) fn expression_node(&self", 1)
open(p, 'w').write(s)
p = W + 'crates/qcore-c-gateway/src/legacy_typed_positions.rs'
s = open(p).read()
helper = '''
fn w47tr_on(function: &str) -> bool {
    std::env::var("W47TR_FUNCS").is_ok_and(|list| list.split(',').any(|f| f == function))
}

impl Parser<'_> {
    fn w47tr(&self, hook: &str, texts: &[&str], line: usize) {
        if !w47tr_on(&self.typed_path_function) {
            return;
        }
        let names = texts.iter().filter(|t| crate::typed_field_access::is_name(t)).map(|t| format!("{t}:nb={} mb={} sc={} pk={:?} dt={:?} formal={}", self.typed_bases.never_bound(t), self.typed_types.may_bind(t), self.typed_types.declared_is_scalar(t), self.typed_types.pointer_kind(t), self.typed_types.declared_type(t), self.formal_parameters.contains(*t))).collect::<Vec<_>>();
        let index = self.typed_positions_index();
        let permissive = index.and_then(|i| i.returned(texts, line, |_| true, |_| true)).map(|p| format!("{:?}", p.scalar));
        let node = index.and_then(|i| i.expression_node(texts, line)).map(|n| {
            let mut s = n.to_string();
            s.truncate(6000);
            s
        });
        eprintln!("W47TR hook={hook} fn={} line={line} readable={} index={} texts={texts:?} names={names:?} permissive_return={permissive:?} node={node:?}", self.typed_path_function, self.typed_types.is_readable(), index.is_some());
    }
}
'''
s = s.replace("fn is_assignment(texts", helper + "\nfn is_assignment(texts", 1)
a = '''        let line = expression.first()?.line;
        let plan = self.typed_positions_index()?.returned('''
assert a in s
s = s.replace(a, '''        let line = expression.first()?.line;
        self.w47tr("return", &texts, line);
        let plan = self.typed_positions_index()?.returned(''')
b = '''        let line = tokens.first()?.line;
        let plan = self
            .typed_positions_index()?
            .condition('''
assert b in s
s = s.replace(b, '''        let line = tokens.first()?.line;
        self.w47tr("condition", &texts, line);
        let plan = self
            .typed_positions_index()?
            .condition(''')
c = '''        let declared = self.typed_types.declared_is_scalar(name)'''
assert c in s
s = s.replace(c, '''        if let Some(first) = tokens.first() {
            self.w47tr("condition-assignment", &texts, first.line);
        }
        let declared = self.typed_types.declared_is_scalar(name)''')
open(p, 'w').write(s)
print("ok")
