//! Card **W47-TYPED-ROOT-DECLARATION-READER** (gateway rule 10l): the premises
//! the typed positions (rule 10k) read, served from the lexical declaration
//! reader where it read the function, and otherwise from the Clang AST
//! ([`crate::typed_root_declarations`]) joined with the lexical closure.
//!
//! * **never bound**: where the declaration reader read the function, the
//!   lexical answer joined with the AST closure (rule 10o: a spelling the AST
//!   closure binds is bound wherever the AST reader answers the function);
//!   otherwise a formal or automatic local declared once that neither the AST
//!   closure nor the lexical closure binds.
//! * **pointer root** (rule 5b's premise for a returned pointer): never bound,
//!   and neither the lexical `may_bind` nor the AST's names it (rule 10o: in
//!   every function the AST reader answers).
//! * **declared local** (an assignment operand's destination): every lexical
//!   `yes` stands; a name the lexical reader does not type is a declared local
//!   when the AST finds one automatic, non-formal declaration of it whose type
//!   the typed reader reads.
//!
//! A parity run records both readers' answers side by side for every analysed
//! function (`position: "root-declarations"`).

use super::Parser;
use crate::typed_root_declarations::RootDeclarations;

impl Parser<'_> {
    fn typed_roots(&self) -> &RootDeclarations {
        self.typed_root_declarations.get_or_init(|| {
            self.typed_positions_index()
                .and_then(|index| {
                    RootDeclarations::read(
                        &index.source,
                        &index.function,
                        &index.enums,
                        self.contracts,
                        &self.variadic_tails,
                        &self.protected_formals,
                    )
                    .ok()
                })
                .unwrap_or_default()
        })
    }

    /// Whether the lexical declaration reader read this function; where it
    /// did, every premise below is its answer.
    fn lexical_answers(&self) -> bool {
        self.typed_types.is_readable()
    }

    /// Card **W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT** (rule 10o): the join
    /// of the two closures where the lexical reader answers — the lexical
    /// never-bound answer (its closure now reading every assignment), unless
    /// the AST closure binds the spelling in a function it answers. Where the
    /// AST reader refuses the function, the lexical answer stands.
    pub(super) fn joined_never_bound(&self, name: &str) -> bool {
        self.typed_bases.never_bound(name) && !self.typed_roots().binds(name)
    }

    /// Rules 10l and 10o: the never-bound premise of a typed position.
    pub(super) fn position_never_bound(&self, name: &str) -> bool {
        if self.lexical_answers() {
            return self.joined_never_bound(name);
        }
        self.typed_roots()
            .served_never_bound(name, self.typed_bases.lexical_risky())
    }

    /// Rules 10l and 10o: rule 5b's premise for a pointer handed out of the
    /// body.
    pub(super) fn position_pointer_root(&self, name: &str) -> bool {
        if self.lexical_answers() {
            return self.joined_never_bound(name)
                && !self.typed_types.may_bind(name)
                && !self.typed_roots().names_may_bind(name);
        }
        self.position_never_bound(name)
            && !self.typed_types.may_bind(name)
            && !self.typed_roots().may_bind(name)
    }

    /// Rule 10l: an assignment operand's destination is a declared local.
    pub(super) fn position_declared_local(&self, name: &str) -> bool {
        self.typed_types.declared_is_scalar(name)
            || self.typed_types.pointer_kind(name).is_some()
            || self.typed_roots().declared_local(name)
    }

    /// Parity only: both readers' answers for this function, side by side.
    /// Changes nothing (the AST index is a cache of the unit's own AST).
    pub(super) fn record_typed_root_parity(&self) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let roots = self.typed_roots();
        let lexical = self.lexical_answers();
        let lexical_risky = self.typed_bases.lexical_risky();
        let mut names = Vec::new();
        let mut disagreements = Vec::new();
        for name in roots.declared_names() {
            let lexical_never_bound = self.typed_bases.never_bound(&name);
            let lexical_ungated = !lexical_risky.contains(&name);
            let ast = roots.ast_never_bound(&name);
            let served = self.position_never_bound(&name);
            let nested = self.typed_bases.nested_bindings().contains(&name);
            let lexical_may_bind = self.typed_types.may_bind(&name);
            let ast_may_bind = roots.may_bind(&name);
            // As the assignment-operand hook reads them: never a formal.
            let formal = self.formal_parameters.contains(&name);
            let lexical_declared = !formal
                && (self.typed_types.declared_is_scalar(&name)
                    || self.typed_types.pointer_kind(&name).is_some());
            let ast_declared = !formal && roots.declared_local(&name);
            let mut kinds = Vec::new();
            if lexical && lexical_never_bound && !ast {
                kinds.push("ast-binds-lexical-never-bound");
            }
            // Rule 10o: the served (joined) answer never calls a name the AST
            // closure binds never bound; recorded if it ever does.
            if served && roots.binds(&name) {
                kinds.push("served-never-bound-ast-binds");
            }
            if nested {
                kinds.push("nested-reading-binds");
            }
            if !lexical_ungated && ast {
                kinds.push("lexical-binds-ast-never-bound");
            }
            if lexical_may_bind && !ast_may_bind {
                kinds.push("lexical-may-bind-only");
            }
            if !lexical_may_bind && ast_may_bind {
                kinds.push("ast-may-bind-only");
            }
            if lexical_declared != ast_declared {
                kinds.push(if lexical_declared {
                    "lexical-declared-only"
                } else {
                    "ast-declared-only"
                });
            }
            if !kinds.is_empty() {
                disagreements.push(serde_json::json!({"name": name, "kinds": kinds}));
            }
            names.push(serde_json::json!({
                "name": name,
                "formal": formal,
                "ast_answered": roots.answered(&name),
                "ast_binding": roots.binding_reason(&name),
                "lexical_never_bound": lexical_never_bound,
                "lexical_closure_never_bound": lexical_ungated,
                "ast_never_bound": ast,
                "served_never_bound": served,
                "nested_reading_binds": nested,
                "ast_binds": roots.binds(&name),
                "lexical_may_bind": lexical_may_bind,
                "ast_may_bind": ast_may_bind,
                "lexical_declared_local": lexical_declared,
                "ast_declared_local": ast_declared,
            }));
        }
        crate::typed_access_path::record_row(serde_json::json!({
            "function": self.typed_path_function,
            "position": "root-declarations",
            "lexical_readable": lexical,
            "ast_answers": roots.answers(),
            "served": if lexical { "lexical" } else if roots.answers() { "ast" } else { "none" },
            "names": names,
            "disagreements": disagreements,
            "nested_census": {
                "unread_by_statement": self.typed_bases.nested_census().unread_by_statement,
                "parenthesised": self.typed_bases.nested_census().parenthesised,
                "nested_member_stores": self.typed_bases.nested_census().nested_member_stores,
            },
        }));
    }
}
