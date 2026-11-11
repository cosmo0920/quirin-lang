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

//! C11 emission for `source.ffi.v1` (card `W48-QR-C-STRUCT-IMPORT`).
//!
//! The emitted program **includes the real header** and uses the real C type,
//! `struct TAG`; no struct is ever re-declared. The layout the checker read is
//! then asserted where the C is compiled:
//!
//! * `_Static_assert(sizeof(struct TAG) == N, …)`;
//! * for every listed field, `_Static_assert(offsetof(struct TAG, f) == O, …)`
//!   and a `_Generic` assertion that the member's type is the field's §G02t
//!   C11 spelling;
//!
//! so a header whose layout drifted after checking does not build. Each callee
//! is declared from its contract signature; a header that declares it
//! differently makes the translation unit ill-formed rather than silently
//! calling through a second type.
//!
//! A construction zeroes the object (`memset`) before storing the listed
//! fields, so an opaque byte of a constructed record is zero. A declared
//! outcome is compared against the real result before anything reads it, and
//! a disagreement exits 70 — the extern route's established status — before
//! any line is written.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::c_program::{self, Fragment, Section};
use crate::ffi::{
    CheckedStruct, FfiCertificate, FfiFunction, FfiOutcome, FfiProgram, FfiStatement, FfiType,
    FfiValue,
};
use crate::scalar_types::{self as st, ScalarType};
use crate::std_io::{Rendering, StdIoEvent, StdIoOperand, Stream};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FfiEmissionRefusal {
    /// The certificate is not this program's.
    CertificateDisagrees,
    /// A name the program does not bind.
    Unbound(String),
    /// A header spelling that cannot appear inside `#include "…"`.
    HeaderSpelling(String),
}

fn record_variable(name: &str) -> String {
    format!("r_{name}")
}

fn value_variable(name: &str) -> String {
    format!("v_{name}")
}

fn c_struct(record: &CheckedStruct) -> String {
    format!("struct {}", record.tag)
}

fn c_type(program: &FfiProgram, ty: &FfiType) -> Option<String> {
    Some(match ty {
        FfiType::Unit => "void".to_owned(),
        FfiType::Scalar(scalar) => st::c_type(*scalar).to_owned(),
        FfiType::Record(name) => c_struct(program.record(name)?),
        FfiType::ConstRecordPointer(name) => format!("const {} *", c_struct(program.record(name)?)),
    })
}

fn literal(value: &FfiValue, ty: ScalarType) -> Option<String> {
    match value {
        FfiValue::Literal(number) => Some(st::c_literal(ty, *number)),
        FfiValue::Bool(flag) => Some(st::c_literal(ty, i128::from(*flag))),
        FfiValue::Float(literal) => Some(st::c_literal(ty, literal.at(ty).ok()?)),
        FfiValue::Name(_) => None,
    }
}

/// The C test that a result disagrees with its declared outcome: `!=` for an
/// integer or `bool`, and — card `W48-QR-FLOATING-POINT` — a bitwise
/// comparison for a float, so `-0.0` is not `0.0` and no NaN is ever equal to
/// a literal (no literal is a NaN).
fn differs(expression: &str, ty: ScalarType, expected: &str) -> String {
    if ty.is_float() {
        format!(
            "memcmp(&{expression}, &({}){{{expected}}}, sizeof({})) != 0",
            st::c_type(ty),
            st::c_type(ty)
        )
    } else {
        format!("{expression} != {expected}")
    }
}

/// Whether a program holds a float value anywhere (card
/// `W48-QR-FLOATING-POINT`): a listed field, a callee's or helper's position,
/// or a float literal.
#[must_use]
pub fn uses_floats(program: &FfiProgram) -> bool {
    let float_type = |ty: &FfiType| matches!(ty, FfiType::Scalar(scalar) if scalar.is_float());
    let float_value = |value: &FfiValue| matches!(value, FfiValue::Float(_));
    let statements_use = |statements: &[FfiStatement]| {
        statements.iter().any(|statement| match statement {
            FfiStatement::Construct { fields, .. } => fields.iter().any(|(_, v)| float_value(v)),
            FfiStatement::Call {
                arguments, outcome, ..
            } => {
                arguments.iter().any(float_value)
                    || match outcome {
                        Some(FfiOutcome::Scalar(value)) => float_value(value),
                        Some(FfiOutcome::Record { fields, .. }) => {
                            fields.iter().any(|(_, v)| float_value(v))
                        }
                        None => false,
                    }
            }
            FfiStatement::Write { value, .. } => float_value(value),
            FfiStatement::Return(Some(value)) => float_value(value),
            _ => false,
        })
    };
    program
        .structs
        .iter()
        .any(|record| record.listed.iter().any(|field| field.ty.is_float()))
        || program.callees.values().any(|callee| {
            float_type(&callee.result) || callee.parameters.iter().any(|(_, ty)| float_type(ty))
        })
        || program.functions.iter().any(|function| {
            float_type(&function.result)
                || function.parameters.iter().any(|(_, ty)| float_type(ty))
                || statements_use(&function.statements)
        })
        || statements_use(&program.statements)
}

fn operand(
    value: &FfiValue,
    ty: ScalarType,
    scalars: &BTreeSet<String>,
) -> Result<String, FfiEmissionRefusal> {
    match value {
        FfiValue::Name(name) if scalars.contains(name) => Ok(value_variable(name)),
        FfiValue::Name(name) => Err(FfiEmissionRefusal::Unbound(name.clone())),
        other => literal(other, ty).ok_or(FfiEmissionRefusal::CertificateDisagrees),
    }
}

/// The file-scope layout assertions of one imported struct.
#[must_use]
pub fn layout_assertions(record: &CheckedStruct) -> String {
    let c = c_struct(record);
    let mut text = format!(
        "_Static_assert(sizeof({c}) == {}, \"{} is {} bytes, as the recorded compiler reported\");\n",
        record.facts.size, c, record.facts.size
    );
    for field in &record.listed {
        let _ = writeln!(
            text,
            "_Static_assert(offsetof({c}, {name}) == {offset}, \"{c}.{name} is at offset {offset}, as the recorded compiler reported\");",
            name = field.name,
            offset = field.offset
        );
        let _ = writeln!(
            text,
            "_Static_assert(_Generic((({c} *) 0)->{name}, {ty}: 1, default: 0), \"{c}.{name} is {ty}, as the recorded compiler reported\");",
            name = field.name,
            ty = st::c_type(field.ty)
        );
    }
    text
}

/// Emit the program.
///
/// # Errors
///
/// A refusal when the certificate is not this program's, or a name or header
/// the program cannot spell.
pub fn emit_ffi_c11(
    program: &FfiProgram,
    certificate: &FfiCertificate,
) -> Result<String, FfiEmissionRefusal> {
    if certificate.entry != program.entry || certificate.structs != program.structs {
        return Err(FfiEmissionRefusal::CertificateDisagrees);
    }
    let mut fragments = vec![Fragment::new(
        Section::Banner,
        "ffi.banner",
        "/* Generated by qcore `source.ffi.v1` (card W48-QR-C-STRUCT-IMPORT). This file\n   imports C: every result it supports holds under the unsafe-FFI assumption. */\n",
    )];
    let mut includes = String::from(
        "#include <inttypes.h>\n#include <stdbool.h>\n#include <stddef.h>\n#include <stdint.h>\n#include <stdio.h>\n#include <string.h>\n",
    );
    // Card `W48-QR-USABLE-RECORDS`: a helper stops the program with
    // `exit(70)` on a disagreeing outcome, so a file with one includes
    // `<stdlib.h>`; every file without one keeps its bytes.
    if !program.functions.is_empty() {
        includes.push_str("#include <stdlib.h>\n");
    }
    // Card `W48-QR-FLOATING-POINT`: a file holding a float value asserts the
    // IEEE environment when compiled; every file without one keeps its bytes.
    let floats = uses_floats(program);
    if floats {
        includes.push_str("#include <float.h>\n");
    }
    let mut headers = BTreeSet::new();
    for record in &program.structs {
        if record.header.contains(['"', '\\', '\n', '\r']) {
            return Err(FfiEmissionRefusal::HeaderSpelling(record.header.clone()));
        }
        if headers.insert(record.header.clone()) {
            let _ = writeln!(includes, "#include \"{}\"", record.header);
        }
    }
    fragments.push(Fragment::new(Section::Includes, "ffi.includes", includes));
    let mut assertions = String::from("\n");
    let uses_target_width = program
        .structs
        .iter()
        .any(|record| record.listed.iter().any(|field| field.ty.is_target_width()))
        || program.callees.values().any(|callee| {
            callee
                .parameters
                .iter()
                .map(|(_, ty)| ty)
                .chain([&callee.result])
                .any(|ty| matches!(ty, FfiType::Scalar(scalar) if scalar.is_target_width()))
        })
        // Card `W48-QR-USABLE-RECORDS`: a helper's own signature too.
        || program.functions.iter().any(|function| {
            function
                .parameters
                .iter()
                .map(|(_, ty)| ty)
                .chain([&function.result])
                .any(|ty| matches!(ty, FfiType::Scalar(scalar) if scalar.is_target_width()))
        });
    if uses_target_width {
        assertions.push_str(&st::c_target_assertions(&program.target));
    }
    if floats {
        assertions.push_str(&crate::scalar_floats::c_environment());
    }
    for record in &program.structs {
        assertions.push_str(&layout_assertions(record));
    }
    fragments.push(Fragment::new(
        Section::Declarations,
        "ffi.layout",
        assertions,
    ));
    fragments.push(crate::io_emit::io_prelude_fragment());
    let mut prototypes = String::from("\n");
    for callee in program.callees.values() {
        let result =
            c_type(program, &callee.result).ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
        let parameters = if callee.parameters.is_empty() {
            "void".to_owned()
        } else {
            callee
                .parameters
                .iter()
                .map(|(name, ty)| {
                    c_type(program, ty)
                        .map(|ty| {
                            if ty.ends_with('*') {
                                format!("{ty}{name}")
                            } else {
                                format!("{ty} {name}")
                            }
                        })
                        .ok_or(FfiEmissionRefusal::CertificateDisagrees)
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        };
        let _ = writeln!(prototypes, "{result} {}({parameters});", callee.name);
    }
    fragments.push(Fragment::new(
        Section::FileScope,
        "ffi.prototypes",
        prototypes,
    ));

    let mut helpers_text = String::new();
    let mut renderers = BTreeSet::new();
    if !program.functions.is_empty() {
        let mut prototypes = String::from("\n");
        for function in &program.functions {
            let _ = writeln!(prototypes, "{};", helper_signature(program, function)?);
        }
        helpers_text.push_str(&prototypes);
        for function in &program.functions {
            let mut env = Emission::default();
            let mut discards = String::new();
            for (name, ty) in &function.parameters {
                match ty {
                    FfiType::Scalar(scalar) => {
                        env.scalars.insert(name.clone());
                        env.scalar_types.insert(name.clone(), *scalar);
                        let _ = writeln!(discards, "    (void) {};", value_variable(name));
                    }
                    FfiType::Record(record) => {
                        env.records.insert(name.clone(), record.clone());
                        let _ = writeln!(discards, "    (void) {};", record_variable(name));
                    }
                    FfiType::Unit | FfiType::ConstRecordPointer(_) => {
                        return Err(FfiEmissionRefusal::CertificateDisagrees);
                    }
                }
            }
            let mut body = discards;
            emit_statements(
                program,
                &function.statements,
                &mut env,
                Some(function),
                &mut body,
            )?;
            renderers.extend(env.renderers);
            let _ = write!(
                helpers_text,
                "\n{}\n{{\n{body}}}\n",
                helper_signature(program, function)?
            );
        }
        fragments.push(Fragment::new(
            Section::FileScope,
            "ffi.helpers",
            helpers_text,
        ));
    }

    let mut body = String::new();
    let mut env = Emission::default();
    emit_statements(program, &program.statements, &mut env, None, &mut body)?;
    renderers.extend(env.renderers);
    // Card `W48-QR-FLOATING-POINT`: the renderers the lines use, before the
    // helpers and the entry that call them.
    if !renderers.is_empty() {
        fragments.push(Fragment::new(
            Section::Prelude,
            "scalar-floats.renderers",
            format!("\n{}", crate::scalar_floats::c_renderers(&renderers)),
        ));
    }
    Ok(c_program::assemble(
        &fragments,
        &c_program::Entry::source_route(body),
    ))
}

/// The C name of a `.qr` helper function (card `W48-QR-USABLE-RECORDS`),
/// prefixed so a helper and a C callee of one name cannot collide.
fn helper_name(name: &str) -> String {
    format!("qr_fn_{name}")
}

/// `static RESULT qr_fn_NAME(PARAMETERS)`: a helper takes and returns
/// scalars and imported records **by value**, which is what C's own struct
/// parameters and results are.
fn helper_signature(
    program: &FfiProgram,
    function: &FfiFunction,
) -> Result<String, FfiEmissionRefusal> {
    let result =
        c_type(program, &function.result).ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
    let parameters = if function.parameters.is_empty() {
        "void".to_owned()
    } else {
        function
            .parameters
            .iter()
            .map(|(name, ty)| {
                let c = c_type(program, ty).ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                Ok(match ty {
                    FfiType::Record(_) => format!("{c} {}", record_variable(name)),
                    _ => format!("{c} {}", value_variable(name)),
                })
            })
            .collect::<Result<Vec<_>, FfiEmissionRefusal>>()?
            .join(", ")
    };
    Ok(format!(
        "static {result} {}({parameters})",
        helper_name(&function.name)
    ))
}

/// What one function's emission knows about its names.
#[derive(Default)]
struct Emission {
    scalars: BTreeSet<String>,
    scalar_types: std::collections::BTreeMap<String, ScalarType>,
    records: std::collections::BTreeMap<String, String>,
    /// The float renderings used, `(hex, type)` (card `W48-QR-FLOATING-POINT`).
    renderers: BTreeSet<(bool, ScalarType)>,
}

/// Emit `statements` of the entry (`function` is `None`) or of a helper.
#[allow(clippy::too_many_lines)]
fn emit_statements(
    program: &FfiProgram,
    statements: &[FfiStatement],
    env: &mut Emission,
    function: Option<&FfiFunction>,
    body: &mut String,
) -> Result<(), FfiEmissionRefusal> {
    // A declared outcome that disagrees stops the program with status 70
    // before anything reads the result: `return 70;` from the entry, and —
    // card `W48-QR-USABLE-RECORDS` — `exit(70)` from a helper, the same
    // status from inside a call.
    let stop = if function.is_some() {
        "exit(70);"
    } else {
        "return 70;"
    };
    for statement in statements {
        match statement {
            FfiStatement::Construct {
                binding,
                record,
                fields,
            } => {
                let checked = program
                    .record(record)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let variable = record_variable(binding);
                let _ = writeln!(body, "    {} {variable};", c_struct(checked));
                let _ = writeln!(body, "    memset(&{variable}, 0, sizeof {variable});");
                for field in &checked.listed {
                    let (_, value) = fields
                        .iter()
                        .find(|(name, _)| *name == field.name)
                        .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                    let _ = writeln!(
                        body,
                        "    {variable}.{} = {};",
                        field.name,
                        operand(value, field.ty, &env.scalars)?
                    );
                }
                let _ = writeln!(body, "    (void) {variable};");
                env.records.insert(binding.clone(), record.clone());
            }
            FfiStatement::Read {
                binding,
                source,
                field,
            } => {
                let record = env
                    .records
                    .get(source)
                    .ok_or_else(|| FfiEmissionRefusal::Unbound(source.clone()))?;
                let checked = program
                    .record(record)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let listed = checked
                    .field(field)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let variable = value_variable(binding);
                let _ = writeln!(
                    body,
                    "    {} {variable} = {}.{field};\n    (void) {variable};",
                    st::c_type(listed.ty),
                    record_variable(source)
                );
                env.scalars.insert(binding.clone());
                env.scalar_types.insert(binding.clone(), listed.ty);
            }
            // Card `W48-QR-USABLE-RECORDS`: one member assignment of a listed
            // field, of a value the checker proved a value of its type.
            FfiStatement::Write {
                record: source,
                field,
                value,
            } => {
                let record = env
                    .records
                    .get(source)
                    .ok_or_else(|| FfiEmissionRefusal::Unbound(source.clone()))?;
                let checked = program
                    .record(record)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let listed = checked
                    .field(field)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let _ = writeln!(
                    body,
                    "    {}.{field} = {};",
                    record_variable(source),
                    operand(value, listed.ty, &env.scalars)?
                );
            }
            FfiStatement::Copy { binding, source } => {
                if let Some(record) = env.records.get(source).cloned() {
                    let checked = program
                        .record(&record)
                        .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                    let variable = record_variable(binding);
                    let _ = writeln!(
                        body,
                        "    {} {variable} = {};\n    (void) {variable};",
                        c_struct(checked),
                        record_variable(source)
                    );
                    env.records.insert(binding.clone(), record);
                } else if let Some(ty) = env.scalar_types.get(source).copied() {
                    let variable = value_variable(binding);
                    let _ = writeln!(
                        body,
                        "    {} {variable} = {};\n    (void) {variable};",
                        st::c_type(ty),
                        value_variable(source)
                    );
                    env.scalars.insert(binding.clone());
                    env.scalar_types.insert(binding.clone(), ty);
                } else {
                    return Err(FfiEmissionRefusal::Unbound(source.clone()));
                }
            }
            FfiStatement::Call {
                binding,
                callee,
                arguments,
                outcome,
            } => {
                // Card `W48-QR-USABLE-RECORDS`: a `.qr` helper, by value.
                if let Some(helper) = program.function(callee) {
                    let mut actuals = Vec::new();
                    for ((_, ty), argument) in helper.parameters.iter().zip(arguments) {
                        actuals.push(match (ty, argument) {
                            (FfiType::Scalar(scalar), value) => {
                                operand(value, *scalar, &env.scalars)?
                            }
                            (FfiType::Record(_), FfiValue::Name(name))
                                if env.records.contains_key(name) =>
                            {
                                record_variable(name)
                            }
                            _ => return Err(FfiEmissionRefusal::CertificateDisagrees),
                        });
                    }
                    let call = format!("{}({})", helper_name(callee), actuals.join(", "));
                    match (&helper.result, binding) {
                        (FfiType::Unit, None) => {
                            let _ = writeln!(body, "    {call};");
                        }
                        (FfiType::Scalar(ty), Some(name)) => {
                            let variable = value_variable(name);
                            let _ = writeln!(
                                body,
                                "    {} {variable} = {call};\n    (void) {variable};",
                                st::c_type(*ty)
                            );
                            env.scalars.insert(name.clone());
                            env.scalar_types.insert(name.clone(), *ty);
                        }
                        (FfiType::Record(record), Some(name)) => {
                            let checked = program
                                .record(record)
                                .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                            let variable = record_variable(name);
                            let _ = writeln!(
                                body,
                                "    {} {variable} = {call};\n    (void) {variable};",
                                c_struct(checked)
                            );
                            env.records.insert(name.clone(), record.clone());
                        }
                        _ => return Err(FfiEmissionRefusal::CertificateDisagrees),
                    }
                    continue;
                }
                let admitted = program
                    .callees
                    .get(callee)
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                let mut actuals = Vec::new();
                for ((_, ty), argument) in admitted.parameters.iter().zip(arguments) {
                    actuals.push(match (ty, argument) {
                        (FfiType::Scalar(scalar), value) => operand(value, *scalar, &env.scalars)?,
                        (FfiType::Record(_), FfiValue::Name(name))
                            if env.records.contains_key(name) =>
                        {
                            record_variable(name)
                        }
                        (FfiType::ConstRecordPointer(_), FfiValue::Name(name))
                            if env.records.contains_key(name) =>
                        {
                            format!("&{}", record_variable(name))
                        }
                        _ => return Err(FfiEmissionRefusal::CertificateDisagrees),
                    });
                }
                let call = format!("{}({})", callee, actuals.join(", "));
                match (&admitted.result, binding) {
                    (FfiType::Unit, None) => {
                        let _ = writeln!(body, "    {call};");
                    }
                    (FfiType::Scalar(ty), Some(name)) => {
                        let variable = value_variable(name);
                        let _ = writeln!(
                            body,
                            "    {} {variable} = {call};\n    (void) {variable};",
                            st::c_type(*ty)
                        );
                        if let Some(FfiOutcome::Scalar(declared)) = outcome {
                            let expected = literal(declared, *ty)
                                .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                            let _ = writeln!(
                                body,
                                "    if ({}) {{\n        {stop}\n    }}",
                                differs(&variable, *ty, &expected)
                            );
                        }
                        env.scalars.insert(name.clone());
                        env.scalar_types.insert(name.clone(), *ty);
                    }
                    (FfiType::Record(record), Some(name)) => {
                        let checked = program
                            .record(record)
                            .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                        let variable = record_variable(name);
                        let _ = writeln!(
                            body,
                            "    {} {variable} = {call};\n    (void) {variable};",
                            c_struct(checked)
                        );
                        if let Some(FfiOutcome::Record { fields, .. }) = outcome {
                            let mut tests = Vec::new();
                            for field in &checked.listed {
                                let (_, value) = fields
                                    .iter()
                                    .find(|(name, _)| *name == field.name)
                                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                                let expected = literal(value, field.ty)
                                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                                tests.push(differs(
                                    &format!("{variable}.{}", field.name),
                                    field.ty,
                                    &expected,
                                ));
                            }
                            let _ = writeln!(
                                body,
                                "    if ({}) {{\n        {stop}\n    }}",
                                tests.join(" || ")
                            );
                        }
                        env.records.insert(name.clone(), record.clone());
                    }
                    _ => return Err(FfiEmissionRefusal::CertificateDisagrees),
                }
            }
            FfiStatement::Line(event) => match event {
                StdIoEvent::Print {
                    rendering,
                    operand: value,
                }
                | StdIoEvent::Log {
                    rendering,
                    operand: value,
                    ..
                } => {
                    let level = match event {
                        StdIoEvent::Log { level, .. } => Some(*level),
                        _ => None,
                    };
                    let (ty, expression) = match value {
                        StdIoOperand::Literal(number) => (
                            ScalarType::I32,
                            st::c_literal(ScalarType::I32, i128::from(*number)),
                        ),
                        StdIoOperand::Scalar(name) => (
                            *env.scalar_types
                                .get(name)
                                .ok_or_else(|| FfiEmissionRefusal::Unbound(name.clone()))?,
                            value_variable(name),
                        ),
                        StdIoOperand::Borrowed(name) => {
                            return Err(FfiEmissionRefusal::Unbound(name.clone()));
                        }
                    };
                    let (stream, prefix) = crate::std_io::line_framing(level);
                    if ty.is_float() {
                        env.renderers.insert((*rendering == Rendering::Hex, ty));
                    }
                    let (conversion, argument) = st::c_rendering(
                        *rendering == Rendering::Hex,
                        ty,
                        &expression,
                        Some(&program.target),
                    )
                    .ok_or(FfiEmissionRefusal::CertificateDisagrees)?;
                    let _ = writeln!(
                        body,
                        "    {}",
                        crate::io_emit::write_line(
                            stream,
                            &format!("{prefix}{conversion}"),
                            &[&argument]
                        )
                    );
                }
                StdIoEvent::Flush => {
                    let _ = writeln!(body, "    {}", crate::io_emit::flush(Stream::Err));
                }
                StdIoEvent::LogLevel { .. } | StdIoEvent::Return => {
                    return Err(FfiEmissionRefusal::CertificateDisagrees);
                }
            },
            // The entry's `return;` is the assembler's; a helper returns its
            // value — a scalar operand or a record binding — or nothing.
            FfiStatement::Return(value) => match (function, value) {
                (None, None) => {}
                (Some(_), None) => {
                    let _ = writeln!(body, "    return;");
                }
                (Some(helper), Some(value)) => {
                    let expression = match (&helper.result, value) {
                        (FfiType::Scalar(ty), value) => operand(value, *ty, &env.scalars)?,
                        (FfiType::Record(_), FfiValue::Name(name))
                            if env.records.contains_key(name) =>
                        {
                            record_variable(name)
                        }
                        _ => return Err(FfiEmissionRefusal::CertificateDisagrees),
                    };
                    let _ = writeln!(body, "    return {expression};");
                }
                (None, Some(_)) => return Err(FfiEmissionRefusal::CertificateDisagrees),
            },
        }
    }
    Ok(())
}
