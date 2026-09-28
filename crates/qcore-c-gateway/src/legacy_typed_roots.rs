//! Card **W47-TYPED-ROOT-DECLARATION-READER** (gateway rule 10l): the premises
//! the typed positions (rule 10k) read, served from the lexical declaration
//! reader where it read the function, and otherwise from the Clang AST
//! ([`crate::typed_root_declarations`]) joined with the lexical closure.
//!
//! * **never bound**: the lexical answer where the declaration reader read the
//!   function (unchanged); otherwise a formal or automatic local declared
//!   once that neither the AST closure nor the lexical closure binds.
//! * **pointer root** (rule 5b's premise for a returned pointer): never bound,
//!   and neither the lexical `may_bind` nor the AST's names it.
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

    /// Rule 10l: the never-bound premise of a typed position.
    pub(super) fn position_never_bound(&self, name: &str) -> bool {
        if self.lexical_answers() {
            return self.typed_bases.never_bound(name);
        }
        self.typed_roots()
            .served_never_bound(name, self.typed_bases.lexical_risky())
    }

    /// Rule 10l: rule 5b's premise for a pointer handed out of the body.
    pub(super) fn position_pointer_root(&self, name: &str) -> bool {
        if self.lexical_answers() {
            return self.typed_bases.never_bound(name) && !self.typed_types.may_bind(name);
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
            let lexical_may_bind = self.typed_types.may_bind(&name);
            let ast_may_bind = roots.may_bind(&name);
            let lexical_declared = self.typed_types.declared_is_scalar(&name)
                || self.typed_types.pointer_kind(&name).is_some();
            let ast_declared = roots.declared_local(&name);
            let mut kinds = Vec::new();
            if lexical && lexical_never_bound && !ast {
                kinds.push("ast-binds-lexical-never-bound");
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
                "lexical_never_bound": lexical_never_bound,
                "lexical_closure_never_bound": lexical_ungated,
                "ast_never_bound": ast,
                "served_never_bound": served,
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
        }));
    }
}
