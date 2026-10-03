//! Card **CAST-FORMALIZATION** (gateway rule 12h): the route-G adapter of
//! [`crate::cast_semantics`]. The hooks in `legacy_c.rs` naming the card are
//! thin; each consults the one conversion table and records its decision as
//! a `QCORE_TYPED_PATH_PARITY` row (`position: "cast-semantics-…"`).
//!
//! * **Census** (parity runs only, no verdict effect): every explicit cast of
//!   a function body, with its Clang kind, conversion class, position and the
//!   line of its enclosing statement (`position: "cast-semantics-census"`),
//!   which the card's census joins to the route's leaves.

use serde_json::json;

use super::Parser;
use crate::cast_semantics::explicit_casts;
use crate::typed_access_path::{qual, record_row};

impl Parser<'_> {
    /// Parity runs only: the function's explicit-cast census.
    pub(super) fn record_cast_census(&self) {
        if !crate::typed_access_path::parity_enabled() {
            return;
        }
        let Some(index) = self.typed_positions_index().cloned() else {
            return;
        };
        for site in explicit_casts(&index.function) {
            let operand = crate::typed_c_ast::children(site.node).first();
            record_row(json!({
                "function": self.typed_path_function,
                "line": index.node_line(site.node),
                "statement_line": index.node_line(site.statement),
                "position": "cast-semantics-census",
                "cast_kind": site.node["castKind"],
                "class": site.class.name(),
                "cast_position": site.position.name(),
                "from": operand.and_then(qual),
                "to": qual(site.node),
                "texts": index.node_tokens(site.node).map(|mut tokens| {
                    tokens.truncate(24);
                    tokens
                }),
            }));
        }
    }
}
