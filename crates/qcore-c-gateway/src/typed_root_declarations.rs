// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Card **W47-TYPED-ROOT-DECLARATION-READER** (gateway rule 10l): the
//! never-bound, `may_bind` and declared-local premises of the typed positions
//! (rule 10k), read from the same Clang AST the typed reader matches.
//!
//! The lexical declaration reader ([`crate::typed_field_access::FunctionTypes`])
//! answers nothing for a function any of whose declarations it cannot read,
//! so there every root was "possibly bound" and every typed position premise
//! refused. This reader answers from the function's `FunctionDecl`, where
//! declarations are Clang's (typedef-resolved, scoped), and only where it can
//! answer exactly; everything else refuses.
//!
//! * **Declarations.** A name is answered only when the function declares it
//!   exactly once, as a formal (`ParmVarDecl`) or an automatic local (a
//!   `VarDecl` of the body with no storage class), and no reference in the body
//!   names another object of that spelling. A shadowed, `static`, `extern` or
//!   file-scope name is not answered.
//! * **Bindings** ([`RootDeclarations::read`]). Seeds: the object root of a
//!   `&` operand, of an increment or decrement, of a compound assignment and
//!   of a sub-object store (`s.f = …`); every variable an actual names at a
//!   position the callee's contract does not declare `Borrowed` (all
//!   positions of an uncontracted or indirect callee), except the base of an
//!   `->` read. Closure: the destination of a plain assignment or an
//!   initialiser is bound when its value contains a call, a `&`, a statement
//!   expression, a bound name or a name this function does not declare. The
//!   closure is iterated to its fixed point; a bound that is not reached is a
//!   refusal of the whole function.
//! * **`may_bind`**: every name written, address-taken or named anywhere in a
//!   call's actuals.
//! * **Refusals of the whole function**: no body, an unprototyped (K&R)
//!   definition, inline assembly, a block, a block-scope `typedef`, an
//!   implicitly declared callee.
//!
//! The served answer ([`RootDeclarations::served_never_bound`]) joins the
//! lexical closure's own risky set, so it is never less conservative than the
//! lexical closure where both name a binding; it is consulted only where the
//! lexical declaration reader did not read the function (hard invariant 3:
//! every lexical answer stands). `QCore.TypedRootDeclarations`.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{ResourceContracts, ResourceParameterMode};
use serde_json::Value;

use crate::typed_access_path::{EnumFacts, is_pointer, is_scalar, offset};
use crate::typed_c_ast::{children, kind};

/// How a function declares one object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeclarationKind {
    Formal,
    Automatic,
    /// `static`, `extern`, `register` or any other storage class.
    Other,
}

#[derive(Clone, Debug)]
struct Declaration {
    kind: DeclarationKind,
    /// A proven scalar or a single pointer value (the typed reader's classes).
    typed: bool,
}

/// The AST reader's answers for one function; `facts` is `None` when it
/// cannot answer at all.
#[derive(Clone, Debug, Default)]
pub(crate) struct RootDeclarations {
    facts: Option<Facts>,
}

#[derive(Clone, Debug, Default)]
struct Facts {
    declarations: BTreeMap<String, Vec<Declaration>>,
    /// Spellings of referenced objects this function does not declare.
    foreign: BTreeSet<String>,
    risky: BTreeSet<String>,
    /// The first reason each risky name was bound (parity only).
    reasons: BTreeMap<String, String>,
    bound: BTreeSet<String>,
}

/// Why the reader refused a whole function (parity and tests only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    NoBody,
    Unprototyped,
    InlineAssembly,
    Block,
    BlockScopeTypedef,
    ImplicitCallee,
    ClosureBound,
}

impl RootDeclarations {
    /// Read one `FunctionDecl`.
    pub(crate) fn read(
        source: &str,
        function: &Value,
        enums: &EnumFacts,
        contracts: &ResourceContracts,
        variadic_tails: &BTreeSet<String>,
        protected_formals: &BTreeSet<String>,
    ) -> Result<Self, Refusal> {
        Self::read_with_frame(
            source,
            function,
            enums,
            contracts,
            variadic_tails,
            protected_formals,
            false,
        )
    }

    /// Recompute the same closure with the certified list-count actual
    /// omitted from `may_bind`. The caller must check the callee body first;
    /// a false certificate leaves the ordinary reader in force.
    pub(crate) fn read_with_frame(
        source: &str,
        function: &Value,
        enums: &EnumFacts,
        contracts: &ResourceContracts,
        variadic_tails: &BTreeSet<String>,
        protected_formals: &BTreeSet<String>,
        read_only_list_count: bool,
    ) -> Result<Self, Refusal> {
        let body = children(function)
            .iter()
            .find(|node| kind(node) == "CompoundStmt")
            .ok_or(Refusal::NoBody)?;
        let formals = children(function)
            .iter()
            .filter(|node| kind(node) == "ParmVarDecl")
            .collect::<Vec<_>>();
        // A K&R definition declares its formals in a list of `;`-terminated
        // declarations between its declarator and its body; a prototype's
        // parameter list cannot hold a `;`. Clang types both alike.
        let declarator = offset(&function["loc"]).ok_or(Refusal::Unprototyped)?;
        let opening = offset(&body["range"]["begin"]).ok_or(Refusal::Unprototyped)?;
        if source
            .get(declarator..opening)
            .is_none_or(|between| between.contains(';'))
        {
            return Err(Refusal::Unprototyped);
        }
        let mut facts = Facts::default();
        let mut ids = BTreeSet::new();
        for formal in &formals {
            facts.declare(formal, DeclarationKind::Formal, enums, &mut ids);
        }
        // A formal the function's own contract (or a custody use) protects
        // holds a tracked identity at entry.
        for name in protected_formals {
            facts.mark(name, "protected-formal");
            facts.bound.insert(name.clone());
        }
        let mut walk = Walk {
            enums,
            contracts,
            variadic_tails,
            read_only_list_count,
            facts: &mut facts,
            ids: &mut ids,
            assignments: Vec::new(),
            references: Vec::new(),
        };
        walk.node(body)?;
        let Walk {
            assignments,
            references,
            ..
        } = walk;
        for (name, id) in references {
            if !ids.contains(&id) {
                facts.foreign.insert(name);
            }
        }
        // The closure: each round either adds a destination or stops, so it
        // settles within `assignments.len() + 1` rounds.
        let mut settled = false;
        for _ in 0..=assignments.len() {
            let mut added = false;
            for (destination, values, place) in &assignments {
                if facts.risky.contains(destination) {
                    continue;
                }
                if let Some(value) = values.iter().find_map(|value| opaque(value, &facts)) {
                    facts.mark(destination, &format!("assignment:{value}:{place}"));
                    added = true;
                }
            }
            if !added {
                settled = true;
                break;
            }
        }
        if !settled {
            return Err(Refusal::ClosureBound);
        }
        Ok(Self { facts: Some(facts) })
    }

    /// Whether the reader answers for this function at all.
    pub(crate) fn answers(&self) -> bool {
        self.facts.is_some()
    }

    fn unique(&self, name: &str) -> Option<&Declaration> {
        let facts = self.facts.as_ref()?;
        if facts.foreign.contains(name) {
            return None;
        }
        match facts.declarations.get(name)?.as_slice() {
            [declaration] if declaration.kind != DeclarationKind::Other => Some(declaration),
            _ => None,
        }
    }

    /// The AST closure's own answer: `name` is one formal or automatic local
    /// of this function that no binding reaches.
    pub(crate) fn ast_never_bound(&self, name: &str) -> bool {
        self.unique(name).is_some()
            && self
                .facts
                .as_ref()
                .is_some_and(|facts| !facts.risky.contains(name))
    }

    /// The served answer: the AST's, joined with every binding the lexical
    /// closure names (`lexical_risky`), so never less conservative than it.
    pub(crate) fn served_never_bound(&self, name: &str, lexical_risky: &BTreeSet<String>) -> bool {
        self.ast_never_bound(name) && !lexical_risky.contains(name)
    }

    /// Card **W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT** (rule 10o): whether the
    /// AST closure binds some object spelled `name` in a function it answers.
    /// `false` where it refuses the function, so the lexical answer stands
    /// there; spelling-level, so a shadowed or foreign name the closure binds
    /// is bound.
    pub(crate) fn binds(&self, name: &str) -> bool {
        self.facts
            .as_ref()
            .is_some_and(|facts| facts.risky.contains(name))
    }

    /// Rule 10o (parity only): every spelling the AST closure binds, and every
    /// spelling it names `may_bind`; empty where it refuses the function.
    pub(crate) fn spellings(&self) -> (Vec<String>, Vec<String>) {
        self.facts.as_ref().map_or_else(Default::default, |facts| {
            (
                facts.risky.iter().cloned().collect(),
                facts.bound.iter().cloned().collect(),
            )
        })
    }

    /// Rule 10o: whether the AST names `name` written, address-taken or in a
    /// call's actuals, in a function it answers (spelling-level); `false`
    /// where it refuses the function.
    pub(crate) fn names_may_bind(&self, name: &str) -> bool {
        self.facts
            .as_ref()
            .is_some_and(|facts| facts.bound.contains(name))
    }

    /// Whether this function may bind `name`: written, address-taken or
    /// named in a call's actuals. `true` for every name not answered.
    pub(crate) fn may_bind(&self, name: &str) -> bool {
        self.unique(name).is_none()
            || self
                .facts
                .as_ref()
                .is_none_or(|facts| facts.bound.contains(name))
    }

    /// Whether `name` is one automatic, non-formal local whose type the typed
    /// reader reads (a proven scalar or a pointer value).
    pub(crate) fn declared_local(&self, name: &str) -> bool {
        self.unique(name).is_some_and(|declaration| {
            declaration.kind == DeclarationKind::Automatic && declaration.typed
        })
    }

    /// Whether the reader answers for `name` at all (one formal or automatic
    /// local, no other object of that spelling).
    pub(crate) fn answered(&self, name: &str) -> bool {
        self.unique(name).is_some()
    }

    /// The first reason the AST closure binds `name` (parity only).
    pub(crate) fn binding_reason(&self, name: &str) -> Option<&str> {
        self.facts.as_ref()?.reasons.get(name).map(String::as_str)
    }

    /// Every name this function declares (for the parity record).
    pub(crate) fn declared_names(&self) -> Vec<String> {
        self.facts
            .as_ref()
            .map(|facts| facts.declarations.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Facts {
    fn mark(&mut self, name: &str, reason: &str) {
        if self.risky.insert(name.to_owned()) {
            self.reasons.insert(name.to_owned(), reason.to_owned());
        }
    }

    fn declare(
        &mut self,
        node: &Value,
        kind: DeclarationKind,
        enums: &EnumFacts,
        ids: &mut BTreeSet<String>,
    ) {
        let Some(name) = node["name"].as_str() else {
            return;
        };
        if let Some(id) = node["id"].as_str() {
            ids.insert(id.to_owned());
        }
        let typed = is_scalar(node) || enums.is_enum(node) || is_pointer(node);
        self.declarations
            .entry(name.to_owned())
            .or_default()
            .push(Declaration { kind, typed });
    }
}

struct Walk<'a, 'f> {
    enums: &'a EnumFacts,
    contracts: &'a ResourceContracts,
    variadic_tails: &'a BTreeSet<String>,
    read_only_list_count: bool,
    facts: &'f mut Facts,
    ids: &'f mut BTreeSet<String>,
    /// `(destination, values, place)` of every plain assignment and
    /// initialiser; `place` says where it stands (parity only).
    assignments: Vec<(String, Vec<&'a Value>, &'static str)>,
    /// `(spelling, id)` of every variable reference.
    references: Vec<(String, String)>,
}

impl<'a> Walk<'a, '_> {
    fn bind(&mut self, name: &str, reason: &'static str) {
        self.facts.mark(name, reason);
        self.facts.bound.insert(name.to_owned());
    }

    fn node(&mut self, node: &'a Value) -> Result<(), Refusal> {
        self.node_at(node, false)
    }

    /// `statement`: `node` is a whole statement of a block, a `case` or a
    /// label (parity only: where an assignment stands).
    fn node_at(&mut self, node: &'a Value, statement: bool) -> Result<(), Refusal> {
        match kind(node) {
            "GCCAsmStmt" | "MSAsmStmt" => return Err(Refusal::InlineAssembly),
            "BlockExpr" => return Err(Refusal::Block),
            "TypedefDecl" => return Err(Refusal::BlockScopeTypedef),
            "VarDecl" => {
                let storage = node["storageClass"].as_str();
                let declaration = if storage.is_none() {
                    DeclarationKind::Automatic
                } else {
                    DeclarationKind::Other
                };
                self.facts.declare(node, declaration, self.enums, self.ids);
                // Card CG15-STORAGE-DURATION-AND-VOLATILE (rule 11h): a
                // static- or thread-duration initializer is a constant
                // evaluated before the program starts, never a store here.
                let static_duration = crate::storage_duration::storage_of(node, true)
                    .is_some_and(crate::storage_duration::Storage::outlives_frame);
                if let Some(name) = node["name"].as_str()
                    && !static_duration
                {
                    let values = children(node).iter().collect::<Vec<_>>();
                    if !values.is_empty() {
                        self.facts.bound.insert(name.to_owned());
                        self.assignments
                            .push((name.to_owned(), values, "initialiser"));
                    }
                }
            }
            "DeclRefExpr" => {
                if is_object(node)
                    && let (Some(name), Some(id)) = (
                        node["referencedDecl"]["name"].as_str(),
                        node["referencedDecl"]["id"].as_str(),
                    )
                {
                    self.references.push((name.to_owned(), id.to_owned()));
                }
                if node["referencedDecl"]["kind"].as_str() == Some("FunctionDecl")
                    && node["referencedDecl"]["isImplicit"].as_bool() == Some(true)
                {
                    return Err(Refusal::ImplicitCallee);
                }
            }
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let pair = children(node);
                if let [left, right] = pair {
                    match strip_lvalue(left) {
                        Some(target) if kind(target) == "DeclRefExpr" && is_object(target) => {
                            if let Some(name) = target["referencedDecl"]["name"].as_str() {
                                self.facts.bound.insert(name.to_owned());
                                let place = if statement { "statement" } else { "nested" };
                                self.assignments.push((name.to_owned(), vec![right], place));
                            }
                        }
                        _ => {
                            if let Some(root) = object_root(left) {
                                self.bind(root, "sub-object-store");
                            }
                        }
                    }
                }
            }
            "CompoundAssignOperator" => {
                if let Some(root) = children(node).first().and_then(object_root) {
                    self.bind(root, "compound-assignment");
                }
            }
            "UnaryOperator"
                if matches!(node["opcode"].as_str(), Some("++") | Some("--") | Some("&")) =>
            {
                let reason = if node["opcode"].as_str() == Some("&") {
                    "address"
                } else {
                    "increment"
                };
                if let Some(root) = children(node).first().and_then(object_root) {
                    self.bind(root, reason);
                }
            }
            "CallExpr" => self.call(node),
            // Builtins Clang does not spell as a call (`__atomic_*`,
            // `__c11_atomic_*`, `__builtin_va_arg`) take every operand as an
            // uncontracted callee would.
            "AtomicExpr" | "VAArgExpr" => {
                for operand in children(node) {
                    self.uncontracted_operand(operand);
                }
            }
            _ => {}
        }
        let block = matches!(
            kind(node),
            "CompoundStmt" | "CaseStmt" | "DefaultStmt" | "LabelStmt"
        );
        for child in children(node) {
            self.node_at(child, block)?;
        }
        Ok(())
    }

    fn uncontracted_operand(&mut self, operand: &'a Value) {
        let mut names = Vec::new();
        variables(operand, &mut names, false);
        for name in &names {
            self.facts.bound.insert((*name).to_owned());
        }
        let mut roots = Vec::new();
        variables(operand, &mut roots, true);
        for root in roots {
            self.facts.mark(root, "custody-actual");
        }
    }

    fn call(&mut self, node: &'a Value) {
        let inner = children(node);
        let Some((callee, actuals)) = inner.split_first() else {
            return;
        };
        let name = callee_name(callee);
        let contract = name
            .and_then(|name| crate::borrowed_actual::root_custody_contract(self.contracts, name));
        for (index, actual) in actuals.iter().enumerate() {
            if self.read_only_list_count
                && name == Some("rd_list_cnt")
                && actuals.len() == 1
                && index == 0
                && framed_list_actual(actual)
            {
                continue;
            }
            let mut names = Vec::new();
            variables(actual, &mut names, false);
            for name in &names {
                self.facts.bound.insert((*name).to_owned());
            }
            let borrowed = contract.is_some_and(|contract| match contract.parameters.get(index) {
                Some(ResourceParameterMode::Borrowed) => true,
                Some(_) => false,
                None => name.is_some_and(|name| self.variadic_tails.contains(name)),
            });
            if borrowed {
                continue;
            }
            let mut roots = Vec::new();
            variables(actual, &mut roots, true);
            for root in roots {
                self.facts.mark(root, "custody-actual");
            }
        }
    }
}

/// `&ROOT->field[.field]*` and no value computation inside the actual.
fn framed_list_actual(actual: &Value) -> bool {
    let Some(address) = strip_lvalue(actual) else {
        return false;
    };
    if kind(address) != "UnaryOperator" || address["opcode"].as_str() != Some("&") {
        return false;
    }
    let Some(mut place) = children(address).first().and_then(strip_lvalue) else {
        return false;
    };
    let mut fields = 0;
    loop {
        if kind(place) != "MemberExpr" {
            return false;
        }
        fields += 1;
        let arrow = place["isArrow"].as_bool() == Some(true);
        let Some(base) = children(place).first().and_then(strip_lvalue) else {
            return false;
        };
        if arrow {
            return fields > 0 && kind(base) == "DeclRefExpr" && is_object(base);
        }
        place = base;
    }
}

/// A reference to an object (a variable or formal), not a function or an
/// enumerator.
fn is_object(node: &Value) -> bool {
    matches!(
        node["referencedDecl"]["kind"].as_str(),
        Some("VarDecl") | Some("ParmVarDecl")
    )
}

fn strip_lvalue(mut node: &Value) -> Option<&Value> {
    while matches!(kind(node), "ParenExpr" | "ImplicitCastExpr") {
        node = children(node).first()?;
    }
    Some(node)
}

/// The variable whose own storage an lvalue designates: through `.`
/// selections and subscripts of an array object, never through a pointer
/// (`->`, `*`, a subscript of a pointer value).
fn object_root(node: &Value) -> Option<&str> {
    let mut cursor = strip_lvalue(node)?;
    loop {
        match kind(cursor) {
            "DeclRefExpr" if is_object(cursor) => {
                return cursor["referencedDecl"]["name"].as_str();
            }
            "MemberExpr" if cursor["isArrow"].as_bool() != Some(true) => {
                cursor = strip_lvalue(children(cursor).first()?)?;
            }
            "ArraySubscriptExpr" => {
                let base = children(cursor).first()?;
                if kind(base) != "ImplicitCastExpr"
                    || base["castKind"].as_str() != Some("ArrayToPointerDecay")
                {
                    return None;
                }
                cursor = strip_lvalue(children(base).first()?)?;
            }
            _ => return None,
        }
    }
}

/// The callee of a direct call: a function name through its decay.
fn callee_name(node: &Value) -> Option<&str> {
    let bare = strip_lvalue(node)?;
    (kind(bare) == "DeclRefExpr" && bare["referencedDecl"]["kind"].as_str() == Some("FunctionDecl"))
        .then(|| bare["referencedDecl"]["name"].as_str())
        .flatten()
}

/// Every variable an expression names; with `roots`, not the base of an
/// `->` read (its value is read, never rebound).
fn variables<'v>(node: &'v Value, out: &mut Vec<&'v str>, roots: bool) {
    if kind(node) == "DeclRefExpr" && is_object(node) {
        if let Some(name) = node["referencedDecl"]["name"].as_str() {
            out.push(name);
        }
        return;
    }
    if roots
        && kind(node) == "MemberExpr"
        && node["isArrow"].as_bool() == Some(true)
        && children(node)
            .first()
            .and_then(strip_lvalue)
            .is_some_and(|base| kind(base) == "DeclRefExpr")
    {
        return;
    }
    for child in children(node) {
        variables(child, out, roots);
    }
}

/// Whether a value may carry a tracked identity, and why: a call, an address,
/// a statement expression, a bound name, or a name this function does not
/// declare.
fn opaque(node: &Value, facts: &Facts) -> Option<&'static str> {
    match kind(node) {
        "CallExpr" => return Some("call"),
        "StmtExpr" => return Some("statement-expression"),
        "UnaryOperator" if node["opcode"].as_str() == Some("&") => return Some("address"),
        "DeclRefExpr" if is_object(node) => {
            let Some(name) = node["referencedDecl"]["name"].as_str() else {
                return Some("unnamed");
            };
            if facts.risky.contains(name) {
                return Some("bound-name");
            }
            if facts.foreign.contains(name) || !facts.declarations.contains_key(name) {
                return Some("undeclared-name");
            }
            return None;
        }
        _ => {}
    }
    children(node).iter().find_map(|child| opaque(child, facts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_c::CCompilerDialect;
    use crate::typed_access_path::{StatementIndex, enter_source};
    use qcore_contract::contract_loader::load_resource_contract_sources;

    const CONTRACTS: &str = "\
module contract;\n\
extern fn c_hold(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(b);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_drop(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(b, nonnull);\n\
           ensures normal resource_return_none(); };";

    const PREFIX: &str = "struct c_box { int flag; struct c_box *next; }; \
        struct c_box *c_make(int n); void c_hold(struct c_box *b); void c_drop(struct c_box *b); \
        void c_other(struct c_box *b); \
        struct c_box *c_global;\n";

    fn read(body: &str) -> Result<RootDeclarations, Refusal> {
        let source = format!("{PREFIX}{body}\n");
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "roots.qv".to_owned(),
            CONTRACTS.to_owned(),
        )]))
        .expect("contracts load");
        let _scope = enter_source(&source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("w").expect("typed index");
        RootDeclarations::read(
            &index.source,
            &index.function,
            &index.enums,
            &contracts,
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
    }

    fn clang() -> bool {
        crate::clang_command().is_ok()
    }

    fn framed(body: &str) -> RootDeclarations {
        let source = format!(
            "struct inner {{ int rl_cnt; }}; \
            struct outer {{ struct inner list; }}; \
            int rd_list_cnt(const struct inner *rl) {{ return rl->rl_cnt; }} \
            {body}"
        );
        let _scope = enter_source(&source, CCompilerDialect::C11, None);
        let index = StatementIndex::read("w").expect("typed index");
        RootDeclarations::read_with_frame(
            &index.source,
            &index.function,
            &index.enums,
            &ResourceContracts::default(),
            &BTreeSet::new(),
            &BTreeSet::new(),
            true,
        )
        .expect("answers")
    }

    #[test]
    fn only_a_direct_exact_framed_call_loses_its_actual_binding() {
        if !clang() {
            return;
        }
        let direct = framed("int w(struct outer *b) { return rd_list_cnt(&b->list); }");
        assert!(direct.ast_never_bound("b"));
        assert!(!direct.may_bind("b"));

        let written = framed("int w(struct outer *b) { rd_list_cnt(&b->list); b = 0; return 0; }");
        assert!(written.may_bind("b"));

        let indirect = framed(
            "int w(struct outer *b) { \
            int (*rd_list_cnt)(const struct inner *) = 0; \
            return rd_list_cnt(&b->list); }",
        );
        assert!(indirect.may_bind("b"));
    }

    #[test]
    fn an_untouched_formal_is_never_bound_and_not_may_bind() {
        if !clang() {
            return;
        }
        let roots = read("int w(struct c_box *b) { return b->flag; }").expect("answers");
        assert!(roots.ast_never_bound("b"));
        assert!(!roots.may_bind("b"));
        assert!(!roots.declared_local("b"));
        assert!(!roots.served_never_bound("b", &BTreeSet::from(["b".to_owned()])));
    }

    #[test]
    fn every_binding_form_binds() {
        if !clang() {
            return;
        }
        for body in [
            "int w(struct c_box *b) { struct c_box **p = &b; return (*p)->flag; }",
            "int w(struct c_box *b) { b = c_make(1); return b->flag; }",
            "int w(struct c_box *b) { struct c_box *a = c_make(1); b = a; return b->flag; }",
            "int w(struct c_box *b) { b++; return b->flag; }",
            "int w(struct c_box *b) { b += 1; return b->flag; }",
            "int w(struct c_box *b) { c_drop(b); return 0; }",
            "int w(struct c_box *b) { c_other(b); return 0; }",
            "int w(struct c_box *b) { b = c_global; return b->flag; }",
            "int w(struct c_box *b) { b = ({ c_make(1); }); return b->flag; }",
            "int w(struct c_box *b) { struct c_box *o = __atomic_exchange_n(&c_global, b, 5); \
             return o->flag; }",
        ] {
            let roots = read(body).expect("answers");
            assert!(!roots.ast_never_bound("b"), "{body}");
        }
    }

    #[test]
    fn a_borrowed_actual_is_never_bound_but_may_bind() {
        if !clang() {
            return;
        }
        let roots = read("int w(struct c_box *b) { c_hold(b); return b->flag; }").expect("answers");
        assert!(roots.ast_never_bound("b"));
        assert!(roots.may_bind("b"));
    }

    #[test]
    fn an_arrow_read_in_an_actual_does_not_bind_its_base() {
        if !clang() {
            return;
        }
        let roots =
            read("int w(struct c_box *b) { c_drop(b->next); return b->flag; }").expect("answers");
        assert!(roots.ast_never_bound("b"));
        assert!(roots.may_bind("b"));
    }

    #[test]
    fn shadowed_static_and_foreign_names_are_not_answered() {
        if !clang() {
            return;
        }
        let shadowed =
            read("int w(struct c_box *b) { { struct c_box *b = 0; (void)b; } return b->flag; }")
                .expect("answers");
        assert!(!shadowed.ast_never_bound("b"));
        assert!(shadowed.may_bind("b"));
        let local = read(
            "int w(struct c_box *b) { static struct c_box *s; struct c_box *a; a = b; s = b; \
             return a->flag + s->flag + c_global->flag; }",
        )
        .expect("answers");
        assert!(local.declared_local("a"));
        assert!(local.ast_never_bound("a"));
        assert!(!local.declared_local("s"));
        assert!(!local.ast_never_bound("s"));
        assert!(!local.ast_never_bound("c_global"));
        assert!(local.may_bind("c_global"));
    }

    #[test]
    fn whole_function_refusals() {
        if !clang() {
            return;
        }
        assert_eq!(
            read("int w(b) struct c_box *b; { return b->flag; }").err(),
            Some(Refusal::Unprototyped)
        );
        assert_eq!(
            read("int w(struct c_box *b) { typedef int t; t x = 0; return x; }").err(),
            Some(Refusal::BlockScopeTypedef)
        );
        assert_eq!(
            read("int w(struct c_box *b) { int x = 0; __asm__(\"\" : \"=r\"(x)); return x; }")
                .err(),
            Some(Refusal::InlineAssembly)
        );
        assert!(!RootDeclarations::default().answers());
        assert!(!RootDeclarations::default().ast_never_bound("b"));
        assert!(RootDeclarations::default().may_bind("b"));
        assert!(!RootDeclarations::default().declared_local("b"));
    }
}
