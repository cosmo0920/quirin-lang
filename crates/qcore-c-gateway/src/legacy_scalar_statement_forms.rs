//! Card **SCALAR-STATEMENT-FORMS** (gateway rule 41): the parser hooks of
//! [`crate::scalar_statement_forms`].
//!
//! Two hooks, each a **fallback** asked only for one of the five refusals the
//! rule reads ([`FAMILIES`]) and only after every older route of its position
//! declined (hard invariant 3): the last step of `expression_statement`
//! (after the statement fallbacks and rules 14a/14b), and a refused `for`
//! clause (which has no fallback table), read from the clause's original
//! spelling. A declined hook leaves the refusal byte for byte. The statement
//! is matched to one Clang node by its line and exact tokens; the typed
//! statement reader decides the value and every place, with its per-hop
//! obligations, and rule 41 adds only the unevaluated `sizeof` operand.
//! Each consultation is recorded on the `QCORE_TYPED_PATH_PARITY` stream with
//! `position: "scalar-statement-form"`.

use serde_json::{Value, json};

use super::{LegacyInstruction, Parser, RefusedWrites, Statement, Token, contains_braces};
use crate::scalar_statement_forms::{
    self as forms, Destination, Form, Refusal, Shape, ValueClass, decide,
};
use crate::typed_access_path::{StatementIndex, StatementPlan, record_row, strip};
use crate::typed_c_ast::kind;

/// The rule's registry id.
pub(crate) const RULE: &str = "41";

/// The refusals the rule reads, byte for byte.
pub(crate) const FAMILIES: [&str; 5] = [
    "unsupported assignment expression",
    "unrecognized assignment destination",
    "braced initializer, expression, or declaration is not modeled",
    "unmodeled increment or decrement expression",
    "C operator `sizeof` requires type/operand analysis",
];

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(|token| token.text.as_str()).collect()
}

/// One declarator's or statement's reading: its instruction.
type Lowered = Result<LegacyInstruction, Refusal>;

impl Parser<'_> {
    /// Hook (rule 41), the last step of `expression_statement`: a statement
    /// refused with one of [`FAMILIES`].
    pub(super) fn scalar_statement_form(
        &self,
        tokens: &[Token],
        line: usize,
        instruction: &LegacyInstruction,
    ) -> Option<Statement> {
        let lowered = self.scalar_form_lowering(tokens, instruction, true)?;
        Some(match lowered.as_slice() {
            [one] => Statement::Instruction(one.clone(), line),
            _ => Statement::Block(
                lowered
                    .into_iter()
                    .map(|instruction| Statement::Instruction(instruction, line))
                    .collect(),
            ),
        })
    }

    /// Hook (rule 41): a `for` clause refused with one of [`FAMILIES`], read
    /// from its original spelling. A clause is one node of the loop, so only a
    /// reading of one instruction answers.
    pub(super) fn scalar_form_clause(
        &self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) -> Option<LegacyInstruction> {
        match self
            .scalar_form_lowering(tokens, instruction, false)?
            .as_slice()
        {
            [one] => Some(one.clone()),
            _ => None,
        }
    }

    fn scalar_form_lowering(
        &self,
        tokens: &[Token],
        instruction: &LegacyInstruction,
        statement: bool,
    ) -> Option<Vec<LegacyInstruction>> {
        let LegacyInstruction::Unknown { description, .. } = instruction else {
            return None;
        };
        if !FAMILIES.contains(&description.as_str()) || !crate::gateway_rules::enabled(RULE) {
            return None;
        }
        let wanted = texts(tokens);
        let first = tokens.first()?;
        if self.opaque_callee(tokens)
            || wanted
                .iter()
                .any(|text| self.descriptor_names.contains(text))
            || (!self.list_frames.is_empty() && self.list_iteration_refusal(&wanted).is_some())
        {
            return None;
        }
        let index = self.typed_positions_index()?;
        let node = Self::elaboration_node(index, first.line, &wanted, &forms::candidate);
        let outcome = match node.and_then(forms::shape) {
            None => Err(Refusal::UnreadDestination),
            // Braces stand only in a type declaration, an object of an inline
            // type (refused by name below) or a scalar's braced initializer
            // (C11 6.7.9p11), which the declarator reads; every other braced
            // statement keeps its refusal.
            Some(shape @ (Shape::TypeDeclaration(_) | Shape::Declarators(_))) => {
                self.scalar_form_shape(index, &shape, statement)
            }
            Some(_) if contains_braces(tokens) => Err(Refusal::UnreadValue),
            Some(shape) => self.scalar_form_shape(index, &shape, statement),
        };
        record_row(json!({
            "function": self.typed_path_function,
            "line": first.line,
            "texts": wanted,
            "position": "scalar-statement-form",
            "old_route": "old_refusal",
            "old_refusal": description,
            "old_admitted": false,
            "new_admitted": outcome.is_ok(),
            "new_refusal": outcome.as_ref().err().map(|refusal| refusal.reason()),
            "disagreement": outcome.is_ok(),
        }));
        outcome.ok()
    }

    fn scalar_form_shape(
        &self,
        index: &StatementIndex,
        shape: &Shape<'_>,
        statement: bool,
    ) -> Result<Vec<LegacyInstruction>, Refusal> {
        match shape {
            Shape::TypeDeclaration(node) => {
                decide(
                    Form::TypeDeclaration,
                    forms::type_declaration_class(node, index.function.as_ref()),
                    ValueClass::None,
                )?;
                Ok(vec![LegacyInstruction::NoOp])
            }
            Shape::Update { form, place, value } => {
                Ok(vec![self.scalar_form_store(index, *form, place, *value)?])
            }
            Shape::Assignment { place, value } => Ok(vec![self.scalar_form_store(
                index,
                Form::Assignment,
                place,
                Some(value),
            )?]),
            Shape::Declarators(declarations) => {
                let form = if declarations.len() == 1 {
                    Form::Declaration
                } else {
                    Form::Declarators
                };
                if form == Form::Declarators && !statement {
                    return Err(Refusal::Mismatched);
                }
                // An object of a type the statement defines inline is rule
                // 11f's, whatever its class.
                if declarations
                    .iter()
                    .any(|declaration| matches!(kind(declaration), "RecordDecl" | "EnumDecl"))
                {
                    decide(form, Destination::InlineRecordObject, ValueClass::None)?;
                }
                declarations
                    .iter()
                    .map(|declaration| self.scalar_form_declarator(index, form, declaration))
                    .collect()
            }
        }
    }

    /// One `VarDecl` of a declaration.
    fn scalar_form_declarator(
        &self,
        index: &StatementIndex,
        form: Form,
        declaration: &Value,
    ) -> Lowered {
        if kind(declaration) != "VarDecl" {
            return Err(Refusal::UnreadDestination);
        }
        let name = declaration["name"]
            .as_str()
            .ok_or(Refusal::UnreadDestination)?;
        let never_bound = |name: &str| self.joined_never_bound(name);
        let mut reader = index.reader(&never_bound);
        reader.unevaluated_queries = true;
        let destination = self.scalar_form_object(index, declaration, name, &reader);
        let initializers = crate::typed_c_ast::children(declaration);
        // C11 6.7.9p11: a scalar's initializer may be enclosed in braces;
        // the braced list of one expression is that expression.
        let initializers = match initializers {
            [list] if kind(list) == "InitListExpr" && destination == Destination::ScalarObject => {
                match crate::typed_c_ast::children(list) {
                    [one] => std::slice::from_ref(one),
                    _ => initializers,
                }
            }
            _ => initializers,
        };
        let value = match initializers {
            [] => ValueClass::None,
            [value] => {
                if reader.expression(value).is_some() {
                    ValueClass::Scalar
                } else {
                    forms::refused_value_class(value)
                }
            }
            _ => ValueClass::Unread,
        };
        decide(form, destination, value)?;
        reader.plan.scalar.destination = Some(name.to_owned());
        Ok(self.scalar_form_instruction(reader.plan, Some(name)))
    }

    /// A step, compound assignment or plain assignment of `place`.
    fn scalar_form_store(
        &self,
        index: &StatementIndex,
        form: Form,
        place: &Value,
        value: Option<&Value>,
    ) -> Lowered {
        let never_bound = |name: &str| self.joined_never_bound(name);
        let mut reader = index.reader(&never_bound);
        reader.unevaluated_queries = true;
        let bare = strip(place).ok_or(Refusal::UnreadDestination)?;
        let named = (kind(bare) == "DeclRefExpr")
            .then(|| {
                Some((
                    bare["referencedDecl"]["id"].as_str()?,
                    bare["referencedDecl"]["name"].as_str()?,
                ))
            })
            .flatten();
        let destination = match named {
            Some((id, name)) => {
                if !matches!(
                    bare["referencedDecl"]["kind"].as_str(),
                    Some("VarDecl" | "ParmVarDecl")
                ) {
                    Destination::Unread
                } else {
                    self.scalar_form_name(index.function.as_ref(), id, name, &reader)
                }
            }
            None => match forms::scalar_class(bare, &|node| reader.scalar(node)) {
                Some(class) => class,
                None => Destination::ScalarPath,
            },
        };
        // The place first: a named scalar sets the destination, a path its
        // bases and per-hop obligations (read and written alike).
        let place_read = matches!(
            destination,
            Destination::ScalarObject | Destination::ScalarPath
        ) && reader.store(place).is_some();
        let destination = if place_read {
            destination
        } else if matches!(
            destination,
            Destination::ScalarObject | Destination::ScalarPath
        ) {
            Destination::Unread
        } else {
            destination
        };
        let value_class = match value {
            None => ValueClass::None,
            Some(value) => {
                if reader.expression(value).is_some() {
                    ValueClass::Scalar
                } else {
                    forms::refused_value_class(value)
                }
            }
        };
        decide(form, destination, value_class)?;
        let name = named
            .map(|(_, name)| name)
            .filter(|_| destination == Destination::ScalarObject);
        Ok(self.scalar_form_instruction(reader.plan, name))
    }

    /// The class of a named object, with the parser's own premises: a
    /// protected formal and a tracked descriptor are never this rule's.
    fn scalar_form_name(
        &self,
        function: &Value,
        id: &str,
        name: &str,
        reader: &crate::typed_access_path::Reader<'_>,
    ) -> Destination {
        if self.protected_formals.contains(name) || self.descriptor_names.contains(name) {
            return Destination::ProtectedFormal;
        }
        forms::object_class(function, id, name, &|node| reader.scalar(node))
    }

    fn scalar_form_object(
        &self,
        index: &StatementIndex,
        declaration: &Value,
        name: &str,
        reader: &crate::typed_access_path::Reader<'_>,
    ) -> Destination {
        let Some(id) = declaration["id"].as_str() else {
            return Destination::Unread;
        };
        self.scalar_form_name(index.function.as_ref(), id, name, reader)
    }

    /// The lowering of an admitted plan: a named object's write with no
    /// access is `ForgetScalar`; every other plan is the typed reader's
    /// `IndirectScalarAccess`, its bases and chains carrying their per-hop
    /// obligations and a write through a pointer the refusal's frame.
    fn scalar_form_instruction(
        &self,
        plan: StatementPlan,
        name: Option<&str>,
    ) -> LegacyInstruction {
        let destination = name.map(str::to_owned);
        if plan.accesses == 0 {
            return match destination {
                Some(destination) => LegacyInstruction::ForgetScalar { destination },
                None => LegacyInstruction::NoOp,
            };
        }
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: None,
            alias_reads: plan
                .scalar
                .record_bases
                .iter()
                .filter_map(|base| self.read_alias(Some(base)))
                .collect(),
            bases: plan.scalar.bases,
            destination,
            frame: plan
                .scalar
                .writes_through_pointer
                .then(|| RefusedWrites::Sparing(self.frame_protected.clone())),
            chains: plan.chains,
            borrowed_destination: false,
            readback: None,
        }
    }
}
