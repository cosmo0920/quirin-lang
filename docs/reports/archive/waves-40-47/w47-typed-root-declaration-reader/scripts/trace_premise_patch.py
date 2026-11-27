"""Temporary census trace: every typed position row whose real plan refuses but whose
plan under all-true premises admits (a premise-only refusal). Never committed."""
W = '/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ab69b07fd0abe4b3d/'
p = W + 'crates/qcore-c-gateway/src/legacy_typed_positions.rs'
s = open(p).read()
helper = '''
impl Parser<'_> {
    fn w47tr_premise(&self, hook: &str, texts: &[&str], line: usize, real: bool, permissive: bool, declared: Option<bool>) {
        if std::env::var_os("W47TR_PREMISE").is_none() || real || !permissive {
            return;
        }
        eprintln!("W47TRP {}", serde_json::json!({"hook": hook, "function": self.typed_path_function, "line": line, "texts": texts, "readable": self.typed_types.is_readable(), "declared": declared}));
    }
}
'''
s = s.replace("fn is_assignment(texts", helper + "\nfn is_assignment(texts", 1)

# condition
a = '''        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "condition",
            plan.as_ref(),
        );'''
assert a in s
s = s.replace(a, a + '''
        if let Some(index) = self.typed_positions_index() {
            let permissive = index.condition(&texts, line, |_| true).is_some();
            self.w47tr_premise("condition", &texts, line, plan.is_some(), permissive, None);
        }''')

# assignment: before `if !declared` evaluate permissive
b = '''        if !declared
            || self.descriptor_names.contains(name)'''
assert b in s
s = s.replace(b, '''        if !declared && !self.descriptor_names.contains(name) && !self.formal_parameters.contains(name) {
            let statement = &tokens[name_index..statement_end];
            let statement_texts = Self::texts(statement);
            if let (Some(first), Some(index)) = (statement.first(), self.typed_positions_index()) {
                let permissive = index.assignment(&statement_texts, first.line, |_| true).is_some_and(|plan| plan.scalar.destination.as_deref() == Some(name) && !plan.scalar.writes_through_pointer);
                self.w47tr_premise("condition-assignment", &statement_texts, first.line, false, permissive, Some(false));
            }
        }
''' + b)
c = '''        record_position(
            &self.typed_path_function,
            line,
            &statement_texts,
            "condition-assignment",
            plan.as_ref(),
        );'''
assert c in s
s = s.replace(c, c + '''
        if let Some(index) = self.typed_positions_index() {
            let permissive = index.assignment(&statement_texts, line, |_| true).is_some();
            self.w47tr_premise("condition-assignment", &statement_texts, line, plan.is_some(), permissive, Some(true));
        }''')
d = '''        record_position(
            &self.typed_path_function,
            line,
            &texts,
            "return",
            plan.as_ref(),
        );'''
assert d in s
s = s.replace(d, d + '''
        if let Some(index) = self.typed_positions_index() {
            let permissive = index.returned(&texts, line, |_| true, |_| true).is_some();
            self.w47tr_premise("return", &texts, line, plan.is_some(), permissive, None);
        }''')
e = '''        record_position(
            &self.typed_path_function,
            line,
            argument,
            "call-argument",
            plan.as_ref().map(|plan| &plan.plan),
        );'''
assert e in s
s = s.replace(e, e + '''
        if let Some(index) = self.typed_positions_index() {
            let permissive = index.argument(callee, index_arg, arity, argument, line, |_| true).is_some();
            self.w47tr_premise("call-argument", argument, line, plan.is_some(), permissive, None);
        }''')
s = s.replace('''    pub(super) fn typed_call_argument(
        &self,
        callee: &str,
        index: usize,''', '''    pub(super) fn typed_call_argument(
        &self,
        callee: &str,
        index_arg: usize,''')
s = s.replace('''            self.typed_positions_index()?
                .argument(callee, index, arity, argument, line, |name| {''', '''            self.typed_positions_index()?
                .argument(callee, index_arg, arity, argument, line, |name| {''')
open(p, 'w').write(s)
print('ok')
