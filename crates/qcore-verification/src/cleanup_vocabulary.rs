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

//! Card `P04-B1`: the cleanup-outcome contracts' declared referent vocabulary.
//!
//! Before this module the `CleanupProfile` consumed by
//! [`crate::cleanup_outcome`] was Rust-only declared data: neither
//! `filesystem_outcomes.qcore` nor `filesystem_delete.qcore` named a single
//! field, syscall, helper or C type the checker binds, so a transported profile
//! would have had no contract-side referent at all (measured and pinned by
//! `docs/reports/remaining-p04-next-ab/README.md` §6.3(a)).
//!
//! The two contracts now declare that vocabulary, in the corpus's existing
//! identifier-premise idiom — the same shape that made the destructor family's
//! owned-field inventory contract-derived
//! ([`crate::field_destructor::declared_owned_fields`]). This module reads the
//! declaration back and **binds** it: a profile that is not exactly the
//! declared one is refused, and a declaration naming anything the closed
//! grammar cannot express is refused whole, never partially interpreted.
//!
//! What this establishes, exactly: the profile is contract-derived rather than
//! caller-invented. What it does **not** establish: anything about the OS. A
//! declared `close`/`munmap`/`unlink` name is still just a name; the replay in
//! [`crate::cleanup_outcome`] then checks it against the pinned typed source,
//! and the contracts' own `requires` premises stay declared premises.
//!
//! ## The declared grammar
//!
//! Every clause is one identifier, so it parses as an ordinary contract
//! premise and — decisively — is *evaluated* like one: an identifier the
//! checker does not supply a value for is `missing observation`, so a
//! vocabulary clause cannot be added to a contract without this module
//! recognising it. That is the fail-closed tie between declaration and checker.
//!
//! ```text
//! cleanup_vocabulary_v1                       the interpreter selector
//! cleanup_function__NAME                      the pinned C function
//! cleanup_root_type__TYPE                     the record pointer type
//! cleanup_syscall__NAME                       the external cleanup callee
//! cleanup_syscall_signature__RET[__ARG]*      its exact declared signature
//! cleanup_presence_field__NAME                the eligibility field
//! cleanup_field__INDEX__NAME__TYPE__RESET     one inventory entry, in order
//! cleanup_argument_field__INDEX__NAME         one syscall argument, in order
//! cleanup_mapping_field__NAME                 guarded-delete eligibility only
//! cleanup_errno_helper__NAME                  the errno accessor
//! cleanup_report_helper__NAME                 the diagnostic helper
//! ```
//!
//! A payload after a fixed prefix is the C identifier **verbatim**, so
//! `__errno_location` really does read `cleanup_errno_helper____errno_location`.
//! That is exact rather than pretty, and it is preferred to an escape the
//! reader would have to decode. C types travel through the closed
//! [`SCALAR_C_TYPES`] table plus a single-pointer `struct` rule, and reset
//! values through the closed [`ResetValue`] spellings; anything outside them is
//! refused, so widening the family is a deliberate edit, not an inference.
use std::collections::{BTreeMap, BTreeSet};

use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};
use qcore_frontend::lexer::{TokenKind, lex};

use crate::cleanup_outcome::{CleanupField, CleanupProfile, ResetValue};

/// The clause that selects this interpretation, mirroring W1's
/// `resource_contract_v1` marker discipline. Absent, the contract declares no
/// vocabulary and the binding refuses rather than defaulting to the caller's
/// profile.
pub const MARKER: &str = "cleanup_vocabulary_v1";

/// Every prefix of the closed grammar, longest first so a shorter prefix can
/// never claim a longer clause.
const PREFIXES: &[&str] = &[
    "cleanup_syscall_signature__",
    "cleanup_argument_field__",
    "cleanup_presence_field__",
    "cleanup_mapping_field__",
    "cleanup_report_helper__",
    "cleanup_errno_helper__",
    "cleanup_root_type__",
    "cleanup_function__",
    "cleanup_syscall__",
    "cleanup_field__",
];

/// The closed scalar C type table. Left is the exact C spelling the typed
/// source import produces; right is its identifier token. No token contains
/// `__`, so `__` stays an unambiguous separator inside a signature or a field
/// clause, and no token starts with `struct_`, so the record-pointer rule below
/// cannot collide with one.
pub const SCALAR_C_TYPES: &[(&str, &str)] = &[
    ("int", "int"),
    ("char *", "char_ptr"),
    ("unsigned long", "unsigned_long"),
    ("unsigned long long", "unsigned_long_long"),
    ("size_t", "size_t"),
    ("void *", "void_ptr"),
    ("const char *", "const_char_ptr"),
];

/// A record tag usable in `struct_TAG_ptr`: an ordinary C identifier with no
/// `__`, so the token stays separator-free and the spelling round-trips.
fn record_tag(tag: &str) -> bool {
    !tag.is_empty()
        && !tag.contains("__")
        && !tag.starts_with(|c: char| c.is_ascii_digit())
        && tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A C type spelling as one identifier token: a closed scalar row, or a single
/// pointer to a `struct` tag. Everything else — arrays, function pointers,
/// double indirection, qualified records, typedef spellings — has no
/// declaration at all and is refused.
fn encode_c_type(spelling: &str) -> Result<String, String> {
    if let Some((_, token)) = SCALAR_C_TYPES.iter().find(|(c, _)| *c == spelling) {
        return Ok((*token).to_owned());
    }
    if let Some(tag) = spelling
        .strip_prefix("struct ")
        .and_then(|rest| rest.strip_suffix(" *"))
        && record_tag(tag)
    {
        return Ok(format!("struct_{tag}_ptr"));
    }
    Err(format!(
        "cleanup C type outside the declared vocabulary: {spelling}"
    ))
}

fn decode_c_type(token: &str) -> Result<String, String> {
    if let Some((spelling, _)) = SCALAR_C_TYPES.iter().find(|(_, t)| *t == token) {
        return Ok((*spelling).to_owned());
    }
    if let Some(tag) = token
        .strip_prefix("struct_")
        .and_then(|rest| rest.strip_suffix("_ptr"))
        && record_tag(tag)
    {
        return Ok(format!("struct {tag} *"));
    }
    Err(format!(
        "declared cleanup C type token is not in the vocabulary: {token}"
    ))
}

fn encode_reset(reset: ResetValue) -> String {
    match reset {
        ResetValue::Null => "null".into(),
        ResetValue::Integer(n) if n < 0 => format!("integer_m{}", n.unsigned_abs()),
        ResetValue::Integer(n) => format!("integer_{n}"),
    }
}

fn decode_reset(token: &str) -> Result<ResetValue, String> {
    if token == "null" {
        return Ok(ResetValue::Null);
    }
    let digits = token
        .strip_prefix("integer_")
        .ok_or_else(|| format!("declared cleanup reset value is unrecognised: {token}"))?;
    let (negative, digits) = match digits.strip_prefix('m') {
        Some(rest) => (true, rest),
        None => (false, digits),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "declared cleanup reset value is unrecognised: {token}"
        ));
    }
    let magnitude: i64 = digits
        .parse()
        .map_err(|_| format!("declared cleanup reset value is out of range: {token}"))?;
    let value = if negative { -magnitude } else { magnitude };
    i32::try_from(value)
        .map(ResetValue::Integer)
        .map_err(|_| format!("declared cleanup reset value is out of range: {token}"))
}

/// A C signature `"int (void *, size_t)"` as `RET__ARG__ARG`. Only the exact
/// spellings the typed importer emits are accepted.
fn encode_signature(signature: &str) -> Result<String, String> {
    let (result, rest) = signature
        .split_once(" (")
        .ok_or_else(|| format!("cleanup signature is not a declared form: {signature}"))?;
    let arguments = rest
        .strip_suffix(')')
        .ok_or_else(|| format!("cleanup signature is not a declared form: {signature}"))?;
    let mut tokens = vec![encode_c_type(result)?.to_owned()];
    if !arguments.is_empty() {
        for argument in arguments.split(", ") {
            tokens.push(encode_c_type(argument)?.to_owned());
        }
    }
    Ok(tokens.join("__"))
}

fn decode_signature(payload: &str) -> Result<String, String> {
    let mut parts = payload.split("__");
    let result = decode_c_type(
        parts
            .next()
            .filter(|part| !part.is_empty())
            .ok_or("declared cleanup signature has no result type")?,
    )?;
    let arguments = parts
        .map(decode_c_type)
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("{result} ({arguments})"))
}

/// Indexed clauses must be duplicate-free and contiguous from zero, so a
/// declaration cannot leave a hole the reader would have to guess at.
fn ordered<T>(map: BTreeMap<usize, T>, what: &str) -> Result<Vec<T>, String> {
    if map.keys().copied().ne(0..map.len()) {
        return Err(format!(
            "declared cleanup {what} indices are not contiguous"
        ));
    }
    Ok(map.into_values().collect())
}

/// A field or argument name usable in an indexed clause: nonempty and free of
/// the `__` separator, so the clause splits back exactly.
fn plain_name(name: &str, what: &str) -> Result<(), String> {
    if name.is_empty() || name.contains("__") {
        return Err(format!("cleanup {what} name is not declarable: {name}"));
    }
    Ok(())
}

fn index_of(payload: &str, what: &str) -> Result<(usize, String), String> {
    let (index, rest) = payload
        .split_once("__")
        .ok_or_else(|| format!("declared cleanup {what} clause has no index"))?;
    if index.is_empty()
        || !index.bytes().all(|b| b.is_ascii_digit())
        || (index.len() > 1 && index.starts_with('0'))
    {
        return Err(format!(
            "declared cleanup {what} index is not canonical: {index}"
        ));
    }
    Ok((
        index
            .parse()
            .map_err(|_| format!("declared cleanup {what} index is out of range"))?,
        rest.to_owned(),
    ))
}

/// The profile shape a contract declares. Compared field-for-field against the
/// `CleanupProfile` the checker was handed; a mismatch anywhere is a refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupVocabularyShape {
    pub function: String,
    pub root_type: String,
    pub syscall: String,
    pub syscall_signature: String,
    pub presence_field: String,
    pub fields: Vec<CleanupField>,
    pub argument_fields: Vec<String>,
    pub mapping_field: Option<String>,
    pub errno_function: String,
    pub report_function: String,
}

impl CleanupVocabularyShape {
    /// The exact clause identifiers this shape is spelled by. Total or reject:
    /// a profile carrying a C type, reset value or name the grammar cannot
    /// express has no spelling at all, and is therefore refused rather than
    /// silently approximated.
    pub fn spelling(&self) -> Result<BTreeSet<String>, String> {
        let mut names = BTreeSet::from([MARKER.to_owned()]);
        let mut push = |name: String| -> Result<(), String> {
            if !names.insert(name.clone()) {
                return Err(format!("cleanup vocabulary spells {name} twice"));
            }
            Ok(())
        };
        push(format!("cleanup_function__{}", self.function))?;
        push(format!(
            "cleanup_root_type__{}",
            encode_c_type(&self.root_type)?
        ))?;
        push(format!("cleanup_syscall__{}", self.syscall))?;
        push(format!(
            "cleanup_syscall_signature__{}",
            encode_signature(&self.syscall_signature)?
        ))?;
        plain_name(&self.presence_field, "presence field")?;
        push(format!("cleanup_presence_field__{}", self.presence_field))?;
        for (index, field) in self.fields.iter().enumerate() {
            plain_name(&field.name, "field")?;
            push(format!(
                "cleanup_field__{index}__{}__{}__{}",
                field.name,
                encode_c_type(&field.c_type)?,
                encode_reset(field.reset)
            ))?;
        }
        for (index, argument) in self.argument_fields.iter().enumerate() {
            plain_name(argument, "argument field")?;
            push(format!("cleanup_argument_field__{index}__{argument}"))?;
        }
        if let Some(mapping) = &self.mapping_field {
            plain_name(mapping, "mapping field")?;
            push(format!("cleanup_mapping_field__{mapping}"))?;
        }
        push(format!("cleanup_errno_helper__{}", self.errno_function))?;
        push(format!("cleanup_report_helper__{}", self.report_function))?;
        Ok(names)
    }
}

/// The vocabulary one pinned policy declares, with the raw clause identifiers
/// it was read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredCleanupVocabulary {
    names: BTreeSet<String>,
    shape: CleanupVocabularyShape,
}

impl DeclaredCleanupVocabulary {
    pub fn shape(&self) -> &CleanupVocabularyShape {
        &self.shape
    }

    /// Every declared clause identifier, marker included. These are exactly the
    /// observations the contract evaluation needs supplied.
    pub fn names(&self) -> &BTreeSet<String> {
        &self.names
    }

    /// The facts a checker supplies for its own declaration. Every value is
    /// `1` *because* [`Self::bind`] has already established that the declared
    /// shape is the profile the replay then checks against the pinned source —
    /// these are checked facts, not assumed premises.
    pub fn facts(&self) -> Vec<(String, i128)> {
        self.names.iter().map(|name| (name.clone(), 1)).collect()
    }

    /// Refuse unless the supplied profile *is* the declared vocabulary.
    ///
    /// `mapping_field` is the guarded-delete eligibility field, which is not a
    /// `CleanupProfile` member; `None` requires the declaration to carry no
    /// `cleanup_mapping_field__` clause, so the close/unmap policies cannot
    /// declare one and the delete policy cannot omit one.
    pub fn bind(
        &self,
        profile: &CleanupProfile,
        mapping_field: Option<&str>,
    ) -> Result<(), String> {
        let supplied = CleanupVocabularyShape {
            function: profile.function.clone(),
            root_type: profile.root_type.clone(),
            syscall: profile.syscall.clone(),
            syscall_signature: profile.syscall_signature.clone(),
            presence_field: profile.presence_field.clone(),
            fields: profile.fields.clone(),
            argument_fields: profile.argument_fields.clone(),
            mapping_field: mapping_field.map(str::to_owned),
            errno_function: profile.errno_function.clone(),
            report_function: profile.report_function.clone(),
        };
        if supplied != self.shape {
            return Err("cleanup profile is not the vocabulary its contract declares".into());
        }
        // Belt and braces: the profile must also *spell* to the identifiers
        // that were read, so a future grammar change cannot make two distinct
        // shapes share one declaration.
        if supplied.spelling()? != self.names {
            return Err("cleanup profile does not spell its declared vocabulary".into());
        }
        Ok(())
    }
}

fn clause_identifier(expression: &str) -> Result<Option<String>, String> {
    let tokens: Vec<_> = lex(expression)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.kind)
        .filter(|t| *t != TokenKind::Eof)
        .collect();
    match tokens.as_slice() {
        [TokenKind::Word(name)] => Ok(Some(name.clone())),
        _ => Ok(None),
    }
}

/// Read the vocabulary one policy declares.
///
/// Total or reject, in the loader's idiom: the marker must be present, every
/// `cleanup_`-prefixed premise must parse under the closed grammar, each
/// single-valued item must appear exactly once, and indexed items must be
/// duplicate-free and contiguous from zero. A premise that is not a
/// `cleanup_` identifier is an ordinary declared premise and is left alone.
pub fn declared_cleanup_vocabulary(
    contract: &str,
    policy: &str,
) -> Result<DeclaredCleanupVocabulary, String> {
    let parsed = parse_source_program(contract).map_err(|e| e.to_string())?;
    let declarations: Vec<_> = parsed
        .declarations
        .iter()
        .filter(|d| d.value.name == policy)
        .collect();
    let [declaration] = declarations.as_slice() else {
        return Err(format!("expected one declaration of {policy}"));
    };
    let contract_clauses = &declaration
        .value
        .function_contract
        .as_ref()
        .ok_or("missing cleanup policy contract")?
        .clauses;

    let mut names = BTreeSet::new();
    let mut marked = false;
    let mut single: BTreeMap<&str, String> = BTreeMap::new();
    let mut fields: BTreeMap<usize, CleanupField> = BTreeMap::new();
    let mut arguments: BTreeMap<usize, String> = BTreeMap::new();

    for clause in contract_clauses {
        if clause.value.kind != SourceContractClauseKind::Requires {
            continue;
        }
        let Some(name) = clause_identifier(&clause.value.expression_source)? else {
            continue;
        };
        if !name.starts_with("cleanup_") {
            continue;
        }
        if !names.insert(name.clone()) {
            return Err(format!("duplicate cleanup vocabulary clause: {name}"));
        }
        if name == MARKER {
            marked = true;
            continue;
        }
        let Some(prefix) = PREFIXES.iter().find(|p| name.starts_with(**p)) else {
            return Err(format!("unknown cleanup vocabulary clause: {name}"));
        };
        let payload = &name[prefix.len()..];
        if payload.is_empty() {
            return Err(format!("empty cleanup vocabulary payload: {name}"));
        }
        match *prefix {
            "cleanup_field__" => {
                let (index, rest) = index_of(payload, "field")?;
                let parts: Vec<_> = rest.split("__").collect();
                let [field_name, c_type, reset] = parts.as_slice() else {
                    return Err(format!("declared cleanup field is malformed: {name}"));
                };
                plain_name(field_name, "field")?;
                let field = CleanupField {
                    name: (*field_name).to_owned(),
                    c_type: decode_c_type(c_type)?.to_owned(),
                    reset: decode_reset(reset)?,
                };
                if fields.insert(index, field).is_some() {
                    return Err(format!("duplicate declared cleanup field index: {index}"));
                }
            }
            "cleanup_argument_field__" => {
                let (index, rest) = index_of(payload, "argument field")?;
                plain_name(&rest, "argument field")?;
                if arguments.insert(index, rest).is_some() {
                    return Err(format!(
                        "duplicate declared cleanup argument index: {index}"
                    ));
                }
            }
            key => {
                if single.insert(key, payload.to_owned()).is_some() {
                    return Err(format!("duplicate declared cleanup item: {key}"));
                }
            }
        }
    }

    if !marked {
        return Err(format!("{policy} declares no {MARKER} cleanup vocabulary"));
    }
    let mut take = |key: &str| -> Result<String, String> {
        single
            .remove(key)
            .ok_or_else(|| format!("{policy} declares no {key} clause"))
    };
    let function = take("cleanup_function__")?;
    let root_type = decode_c_type(&take("cleanup_root_type__")?)?.to_owned();
    let syscall = take("cleanup_syscall__")?;
    let syscall_signature = decode_signature(&take("cleanup_syscall_signature__")?)?;
    let presence_field = take("cleanup_presence_field__")?;
    plain_name(&presence_field, "presence field")?;
    let errno_function = take("cleanup_errno_helper__")?;
    let report_function = take("cleanup_report_helper__")?;
    let mapping_field = single.remove("cleanup_mapping_field__");
    if let Some(mapping) = &mapping_field {
        plain_name(mapping, "mapping field")?;
    }
    if let Some((key, _)) = single.into_iter().next() {
        return Err(format!("unconsumed cleanup vocabulary item: {key}"));
    }

    let fields = ordered(fields, "field")?;
    let argument_fields = ordered(arguments, "argument field")?;
    if fields.is_empty() || argument_fields.is_empty() {
        return Err(format!("{policy} declares an empty cleanup inventory"));
    }

    Ok(DeclaredCleanupVocabulary {
        names,
        shape: CleanupVocabularyShape {
            function,
            root_type,
            syscall,
            syscall_signature,
            presence_field,
            fields,
            argument_fields,
            mapping_field,
            errno_function,
            report_function,
        },
    })
}
