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

//! C11 backend for checked resource helper calls.
//!
//! The backend retains the checked call graph as real C functions. Owned byte
//! values pass by value and the caller's source binding is poisoned after the
//! call; read and write loans pass `const qcore_bytes *` and `qcore_bytes *`.

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::resource_calls::{Argument, Checked, ReturnValue, Statement, Type};
use crate::std_buffer::{BufferEvent, BufferOperand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaceKind {
    Owned,
    Read,
    Write,
    Scalar,
}

fn c_type(ty: Type) -> &'static str {
    match ty {
        Type::Unit => "void",
        Type::I32 => "int32_t",
        Type::OwnedBytes => "qcore_bytes",
        Type::ReadBytes => "const qcore_bytes *",
        Type::WriteBytes => "qcore_bytes *",
    }
}

fn scalar(operand: &BufferOperand, names: &BTreeMap<String, (String, PlaceKind)>) -> String {
    match operand {
        BufferOperand::Literal(value) => format!("INT32_C({value})"),
        BufferOperand::Scalar(name) => names[name].0.clone(),
    }
}

fn data(name: &str, names: &BTreeMap<String, (String, PlaceKind)>) -> String {
    let (name, kind) = &names[name];
    match kind {
        PlaceKind::Owned => format!("{name}.data"),
        PlaceKind::Read | PlaceKind::Write => format!("{name}->data"),
        PlaceKind::Scalar => unreachable!("checked byte place cannot be scalar"),
    }
}

fn length(name: &str, names: &BTreeMap<String, (String, PlaceKind)>) -> String {
    let (name, kind) = &names[name];
    match kind {
        PlaceKind::Owned => format!("{name}.len"),
        PlaceKind::Read | PlaceKind::Write => format!("{name}->len"),
        PlaceKind::Scalar => unreachable!("checked byte place cannot be scalar"),
    }
}

fn emit_function(checked: &Checked, id: usize, out: &mut String) {
    let function = &checked.program().functions[id];
    write!(out, "static {} qcore_f{id}(", c_type(function.result)).unwrap();
    if function.parameters.is_empty() {
        out.push_str("void");
    }
    for (index, parameter) in function.parameters.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "{} q_p{index}", c_type(parameter.ty)).unwrap();
    }
    out.push_str(") {\n");
    let mut names = BTreeMap::new();
    for (index, parameter) in function.parameters.iter().enumerate() {
        let kind = match parameter.ty {
            Type::OwnedBytes => PlaceKind::Owned,
            Type::ReadBytes => PlaceKind::Read,
            Type::WriteBytes => PlaceKind::Write,
            Type::I32 => PlaceKind::Scalar,
            Type::Unit => unreachable!("checked unit parameter"),
        };
        names.insert(parameter.name.clone(), (format!("q_p{index}"), kind));
        writeln!(out, "    (void) q_p{index};").unwrap();
    }
    for (statement, item) in function.statements.iter().enumerate() {
        match item {
            Statement::Buffer(event) => match event {
                BufferEvent::Bind { binding, value } => {
                    let emitted = format!("q_s{statement}");
                    writeln!(out, "    int32_t {emitted} = INT32_C({value});").unwrap();
                    writeln!(out, "    (void) {emitted};").unwrap();
                    names.insert(binding.clone(), (emitted, PlaceKind::Scalar));
                }
                BufferEvent::Acquire { name, extent } => {
                    let emitted = format!("q_b{statement}");
                    writeln!(
                        out,
                        "    qcore_bytes {emitted} = qcore_bytes_new((size_t) {extent}u);"
                    )
                    .unwrap();
                    names.insert(name.clone(), (emitted, PlaceKind::Owned));
                }
                BufferEvent::Store { name, index, value } => {
                    writeln!(
                        out,
                        "    qcore_bytes_store({}, {}, (size_t) ({}), {});",
                        data(name, &names),
                        length(name, &names),
                        scalar(index, &names),
                        scalar(value, &names)
                    )
                    .unwrap();
                }
                BufferEvent::Load {
                    binding,
                    name,
                    index,
                } => {
                    let emitted = format!("q_s{statement}");
                    writeln!(
                        out,
                        "    int32_t {emitted} = qcore_bytes_load({}, {}, (size_t) ({}));",
                        data(name, &names),
                        length(name, &names),
                        scalar(index, &names)
                    )
                    .unwrap();
                    writeln!(out, "    (void) {emitted};").unwrap();
                    names.insert(binding.clone(), (emitted, PlaceKind::Scalar));
                }
                BufferEvent::Extent { binding, name } => {
                    let emitted = format!("q_s{statement}");
                    writeln!(
                        out,
                        "    int32_t {emitted} = (int32_t) {};",
                        length(name, &names)
                    )
                    .unwrap();
                    writeln!(out, "    (void) {emitted};").unwrap();
                    names.insert(binding.clone(), (emitted, PlaceKind::Scalar));
                }
                BufferEvent::Release { name } => {
                    let emitted = &names[name].0;
                    writeln!(out, "    qcore_bytes_drop(&{emitted});").unwrap();
                }
                _ => unreachable!("checked resource call buffer event"),
            },
            Statement::Call {
                binding,
                callee,
                arguments,
            } => {
                let target = &checked.program().functions[*callee];
                let mut rendered = Vec::new();
                let mut moved = Vec::new();
                for argument in arguments {
                    match argument {
                        Argument::Scalar(value) => rendered.push(scalar(value, &names)),
                        Argument::Move(name) => {
                            rendered.push(names[name].0.clone());
                            moved.push(names[name].0.clone());
                        }
                        Argument::Borrow(name) | Argument::Lend(name) => {
                            rendered.push(format!("&{}", names[name].0));
                        }
                    }
                }
                let call = format!("qcore_f{callee}({})", rendered.join(", "));
                match target.result {
                    Type::Unit => writeln!(out, "    {call};").unwrap(),
                    Type::I32 => {
                        let emitted = format!("q_s{statement}");
                        writeln!(out, "    int32_t {emitted} = {call};").unwrap();
                        writeln!(out, "    (void) {emitted};").unwrap();
                        names.insert(binding.clone().unwrap(), (emitted, PlaceKind::Scalar));
                    }
                    Type::OwnedBytes => {
                        let emitted = format!("q_b{statement}");
                        writeln!(out, "    qcore_bytes {emitted} = {call};").unwrap();
                        names.insert(binding.clone().unwrap(), (emitted, PlaceKind::Owned));
                    }
                    Type::ReadBytes | Type::WriteBytes => unreachable!("checked borrowed result"),
                }
                for moved in moved {
                    writeln!(out, "    {moved}.data = NULL; {moved}.len = 0u;").unwrap();
                }
            }
            Statement::Return(ReturnValue::Unit) => out.push_str("    return;\n"),
            Statement::Return(ReturnValue::Scalar(value)) => {
                writeln!(out, "    return {};", scalar(value, &names)).unwrap();
            }
            Statement::Return(ReturnValue::Move(name)) => {
                writeln!(out, "    return {};", names[name].0).unwrap();
            }
        }
    }
    out.push_str("}\n\n");
}

#[must_use]
pub fn emit(checked: &Checked) -> String {
    let mut out = String::from(
        "#include <stddef.h>\n#include <stdint.h>\n#include <stdlib.h>\n\n\
typedef struct { unsigned char *data; size_t len; } qcore_bytes;\n\
static qcore_bytes qcore_bytes_new(size_t len) {\n\
    unsigned char *data = (unsigned char *) calloc(len, 1u);\n\
    if (data == NULL) { exit(86); }\n\
    qcore_bytes value = { data, len }; return value;\n\
}\n\
static void qcore_bytes_drop(qcore_bytes *value) { free(value->data); value->data = NULL; value->len = 0u; }\n\
static int32_t qcore_bytes_load(const unsigned char *data, size_t len, size_t at) { if (at >= len) { exit(86); } return (int32_t) data[at]; }\n\
static void qcore_bytes_store(unsigned char *data, size_t len, size_t at, int32_t value) { if (at >= len || value < 0 || value > 255) { exit(86); } data[at] = (unsigned char) value; }\n\n",
    );
    for (id, function) in checked.program().functions.iter().enumerate() {
        write!(out, "static {} qcore_f{id}(", c_type(function.result)).unwrap();
        if function.parameters.is_empty() {
            out.push_str("void");
        }
        for (index, parameter) in function.parameters.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            write!(out, "{} q_p{index}", c_type(parameter.ty)).unwrap();
        }
        out.push_str(");\n");
    }
    out.push('\n');
    for id in 0..checked.program().functions.len() {
        emit_function(checked, id, &mut out);
    }
    writeln!(
        out,
        "int main(void) {{ qcore_f{}(); return 0; }}",
        checked.program().entry
    )
    .unwrap();
    out
}
