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

//! Bounded direct calls over extent-carrying `std.buffer` values.
//!
//! This module composes the existing byte-buffer value checker with the
//! cycle-3 ownership IR.  Calls are acyclic and closed: every resource helper
//! is reached from the selected entry, and every indexed parameter access
//! becomes a minimum-extent obligation discharged at each call site.

use std::collections::{BTreeMap, BTreeSet};

use crate::ownership::{self, Footprint, OwnershipOp, ResourceId};
use crate::std_buffer::{
    self, BUFFER_RESOURCE_TYPE, BYTE_AXIS, BufferCertificate, BufferEvent, BufferOperand,
    BufferProgram,
};
use crate::typed_errors::{Effect, EffectRow};

pub const MAX_FUNCTIONS: usize = 256;
pub const MAX_PARAMETERS: usize = 256;
pub const MAX_CALL_DEPTH: usize = 64;
pub const MAX_STATEMENTS: usize = 16_384;
pub const MAX_EXPANDED_ITEMS: usize = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Unit,
    I32,
    OwnedBytes,
    ReadBytes,
    WriteBytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Argument {
    Scalar(BufferOperand),
    Move(String),
    Borrow(String),
    Lend(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReturnValue {
    Unit,
    Scalar(BufferOperand),
    Move(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Buffer(BufferEvent),
    Call {
        binding: Option<String>,
        callee: usize,
        arguments: Vec<Argument>,
    },
    Return(ReturnValue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    /// `None` infers the row. `Some(empty)` is an explicit empty row.
    pub declared_effects: Option<EffectRow>,
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
    pub entry: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusalCategory {
    Ordinary,
    Extent,
    Ownership,
    Certificate,
    Effects,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub function: usize,
    pub statement: usize,
    pub category: RefusalCategory,
    pub message: String,
}

fn bad(function: usize, statement: usize, message: impl Into<String>) -> Refusal {
    categorized_bad(function, statement, RefusalCategory::Ordinary, message)
}

fn categorized_bad(
    function: usize,
    statement: usize,
    category: RefusalCategory,
    message: impl Into<String>,
) -> Refusal {
    Refusal {
        function,
        statement,
        category,
        message: message.into(),
    }
}

fn extent_bad(function: usize, statement: usize, message: impl Into<String>) -> Refusal {
    categorized_bad(function, statement, RefusalCategory::Extent, message)
}

fn ownership_bad(function: usize, statement: usize, message: impl Into<String>) -> Refusal {
    categorized_bad(function, statement, RefusalCategory::Ownership, message)
}

fn certificate_bad(function: usize, statement: usize, message: impl Into<String>) -> Refusal {
    categorized_bad(function, statement, RefusalCategory::Certificate, message)
}

fn effects_bad(function: usize, statement: usize, message: impl Into<String>) -> Refusal {
    categorized_bad(function, statement, RefusalCategory::Effects, message)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Extent {
    Exact(u32),
    Parameter(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Binding {
    Owned(Extent),
    Read(usize),
    Write(usize),
    Scalar,
    Moved,
    Dropped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReturnExtent {
    Parameter(usize),
    Exact(u32),
}

#[derive(Clone, Debug)]
struct Summary {
    effects: EffectRow,
    minimum_extents: Vec<u32>,
    returned_extent: Option<ReturnExtent>,
}

#[derive(Clone, Debug)]
pub struct Checked {
    program: Program,
    summaries: Vec<Summary>,
    flattened: BufferProgram,
    certificate: BufferCertificate,
    ownership: Vec<OwnershipOp>,
}

impl Checked {
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn flattened(&self) -> &BufferProgram {
        &self.flattened
    }
    pub fn certificate(&self) -> &BufferCertificate {
        &self.certificate
    }
    pub fn ownership_operations(&self) -> &[OwnershipOp] {
        &self.ownership
    }
    pub fn function_count(&self) -> usize {
        self.program.functions.len()
    }
    pub fn minimum_extents(&self, function: usize) -> Option<&[u32]> {
        self.summaries
            .get(function)
            .map(|summary| summary.minimum_extents.as_slice())
    }
}

fn operand_minimum(operand: &BufferOperand) -> Option<u32> {
    match operand {
        BufferOperand::Literal(value) => u32::try_from(*value).ok()?.checked_add(1),
        BufferOperand::Scalar(_) => None,
    }
}

fn note_access(
    function: usize,
    statement: usize,
    binding: &Binding,
    index: &BufferOperand,
    write: bool,
    minimum_extents: &mut [u32],
) -> Result<(), Refusal> {
    match (binding, write) {
        (Binding::Owned(_), _) | (Binding::Write(_), _) | (Binding::Read(_), false) => {}
        (Binding::Read(_), true) => {
            return Err(ownership_bad(
                function,
                statement,
                "a borrow<bytes> parameter grants read authority only",
            ));
        }
        (Binding::Moved, _) => {
            return Err(ownership_bad(function, statement, "use after owned move"));
        }
        (Binding::Dropped, _) => {
            return Err(ownership_bad(function, statement, "use after drop"));
        }
        (Binding::Scalar, _) => {
            return Err(bad(
                function,
                statement,
                "a byte access requires a bytes value",
            ));
        }
    }
    let required = operand_minimum(index).ok_or_else(|| {
        bad(
            function,
            statement,
            "helper byte indices are integer literals in resource-calls.v1",
        )
    })?;
    match binding {
        Binding::Owned(Extent::Exact(extent)) if required > *extent => Err(extent_bad(
            function,
            statement,
            format!("byte index requires extent {required}, but this buffer has extent {extent}"),
        )),
        Binding::Owned(Extent::Parameter(parameter))
        | Binding::Read(parameter)
        | Binding::Write(parameter) => {
            minimum_extents[*parameter] = minimum_extents[*parameter].max(required);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn binding_extent(binding: &Binding) -> Option<Extent> {
    match binding {
        Binding::Owned(extent) => Some(extent.clone()),
        _ => None,
    }
}

fn owned_names(bindings: &BTreeMap<String, Binding>) -> Vec<String> {
    bindings
        .iter()
        .filter(|(_, binding)| matches!(binding, Binding::Owned(_)))
        .map(|(name, _)| name.clone())
        .collect()
}

fn analyze_function(
    program: &Program,
    function: usize,
    summaries: &[Option<Summary>],
) -> Result<Summary, Refusal> {
    let f = &program.functions[function];
    let mut bindings = BTreeMap::new();
    let mut minimum_extents = vec![0; f.parameters.len()];
    for (index, parameter) in f.parameters.iter().enumerate() {
        let binding = match parameter.ty {
            Type::OwnedBytes => Binding::Owned(Extent::Parameter(index)),
            Type::ReadBytes => Binding::Read(index),
            Type::WriteBytes => Binding::Write(index),
            Type::I32 => Binding::Scalar,
            Type::Unit => {
                return Err(bad(function, 0, "unit is not a parameter type"));
            }
        };
        if bindings.insert(parameter.name.clone(), binding).is_some() {
            return Err(bad(function, 0, "duplicate parameter"));
        }
    }
    let mut derived = BTreeSet::new();
    let mut returned_extent = None;
    let mut returned = false;
    for (statement, item) in f.statements.iter().enumerate() {
        if returned {
            return Err(bad(function, statement, "statement after the final return"));
        }
        match item {
            Statement::Buffer(event) => match event {
                BufferEvent::Bind { binding, .. } => {
                    if bindings.insert(binding.clone(), Binding::Scalar).is_some() {
                        return Err(bad(function, statement, "binding already exists"));
                    }
                }
                BufferEvent::Acquire { name, extent } => {
                    if !(1..=std_buffer::MAX_EXTENT).contains(extent) {
                        return Err(extent_bad(
                            function,
                            statement,
                            "bytes acquisition extent is outside 1..=MAX_EXTENT",
                        ));
                    }
                    if !owned_names(&bindings).is_empty() {
                        return Err(ownership_bad(
                            function,
                            statement,
                            "allocation failure would leave an owned bytes value without custody; release it before allocating",
                        ));
                    }
                    if bindings
                        .insert(name.clone(), Binding::Owned(Extent::Exact(*extent)))
                        .is_some()
                    {
                        return Err(bad(function, statement, "binding already exists"));
                    }
                    derived.insert(Effect::Alloc);
                }
                BufferEvent::Store { name, index, .. } => {
                    let binding = bindings
                        .get(name)
                        .ok_or_else(|| bad(function, statement, "unknown bytes binding"))?;
                    note_access(
                        function,
                        statement,
                        binding,
                        index,
                        true,
                        &mut minimum_extents,
                    )?;
                    derived.insert(Effect::Mut);
                }
                BufferEvent::Load {
                    binding,
                    name,
                    index,
                } => {
                    let source = bindings
                        .get(name)
                        .ok_or_else(|| bad(function, statement, "unknown bytes binding"))?;
                    note_access(
                        function,
                        statement,
                        source,
                        index,
                        false,
                        &mut minimum_extents,
                    )?;
                    if bindings.insert(binding.clone(), Binding::Scalar).is_some() {
                        return Err(bad(function, statement, "binding already exists"));
                    }
                }
                BufferEvent::Extent { binding, name } => {
                    let source = bindings
                        .get(name)
                        .ok_or_else(|| bad(function, statement, "unknown bytes binding"))?;
                    if !matches!(
                        source,
                        Binding::Owned(_) | Binding::Read(_) | Binding::Write(_)
                    ) {
                        return Err(ownership_bad(
                            function,
                            statement,
                            "extent requires a live bytes value",
                        ));
                    }
                    if bindings.insert(binding.clone(), Binding::Scalar).is_some() {
                        return Err(bad(function, statement, "binding already exists"));
                    }
                }
                BufferEvent::Release { name } => {
                    let slot = bindings
                        .get_mut(name)
                        .ok_or_else(|| bad(function, statement, "unknown bytes binding"))?;
                    match slot {
                        Binding::Owned(_) => *slot = Binding::Dropped,
                        Binding::Read(_) | Binding::Write(_) => {
                            return Err(ownership_bad(
                                function,
                                statement,
                                "a borrowed bytes parameter cannot be dropped",
                            ));
                        }
                        Binding::Moved => {
                            return Err(ownership_bad(
                                function,
                                statement,
                                "drop after owned move",
                            ));
                        }
                        _ => {
                            return Err(ownership_bad(
                                function,
                                statement,
                                "drop requires owned bytes",
                            ));
                        }
                    }
                }
                BufferEvent::Return => {
                    return Err(bad(
                        function,
                        statement,
                        "resource call returns are represented explicitly",
                    ));
                }
                _ => {
                    return Err(bad(
                        function,
                        statement,
                        "loops, records, local lends and line output are outside resource-calls.v1",
                    ));
                }
            },
            Statement::Call {
                binding,
                callee,
                arguments,
            } => {
                let target = program
                    .functions
                    .get(*callee)
                    .ok_or_else(|| bad(function, statement, "unknown helper"))?;
                let summary = summaries
                    .get(*callee)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| {
                        bad(function, statement, "recursive or unordered helper call")
                    })?;
                if arguments.len() != target.parameters.len() {
                    return Err(bad(function, statement, "helper argument count mismatch"));
                }
                let mut resource_actuals: BTreeMap<&str, Vec<Type>> = BTreeMap::new();
                for (argument, parameter) in arguments.iter().zip(&target.parameters) {
                    let (name, actual_type) = match argument {
                        Argument::Move(name) => (Some(name.as_str()), Type::OwnedBytes),
                        Argument::Borrow(name) => (Some(name.as_str()), Type::ReadBytes),
                        Argument::Lend(name) => (Some(name.as_str()), Type::WriteBytes),
                        Argument::Scalar(_) => (None, Type::I32),
                    };
                    if actual_type != parameter.ty {
                        return Err(bad(function, statement, "helper argument type mismatch"));
                    }
                    if let Some(name) = name {
                        resource_actuals.entry(name).or_default().push(actual_type);
                    }
                }
                for (name, modes) in &resource_actuals {
                    let owns = modes.contains(&Type::OwnedBytes);
                    let writes = modes.contains(&Type::WriteBytes);
                    if (owns && modes.len() > 1) || (writes && modes.len() > 1) {
                        return Err(ownership_bad(
                            function,
                            statement,
                            format!(
                                "`{name}` is passed through incompatible owned/borrow modes in one call"
                            ),
                        ));
                    }
                }
                if summary.effects.contains(&Effect::Alloc) {
                    let moved = arguments
                        .iter()
                        .filter_map(|argument| match argument {
                            Argument::Move(name) => Some(name.as_str()),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>();
                    let retained = owned_names(&bindings)
                        .into_iter()
                        .filter(|name| !moved.contains(name.as_str()))
                        .collect::<Vec<_>>();
                    if !retained.is_empty() {
                        return Err(ownership_bad(
                            function,
                            statement,
                            format!(
                                "helper allocation failure would strand caller-owned bytes: {}",
                                retained.join(", ")
                            ),
                        ));
                    }
                }
                let mut actual_extents = Vec::with_capacity(arguments.len());
                for (index, (argument, parameter)) in
                    arguments.iter().zip(&target.parameters).enumerate()
                {
                    match argument {
                        Argument::Scalar(BufferOperand::Literal(_)) => actual_extents.push(None),
                        Argument::Scalar(BufferOperand::Scalar(name)) => {
                            if !matches!(bindings.get(name), Some(Binding::Scalar)) {
                                return Err(bad(function, statement, "unbound scalar argument"));
                            }
                            actual_extents.push(None);
                        }
                        Argument::Move(name) => {
                            let slot = bindings.get_mut(name).ok_or_else(|| {
                                bad(function, statement, "unknown owned argument")
                            })?;
                            let extent = binding_extent(slot).ok_or_else(|| {
                                ownership_bad(function, statement, "move requires live owned bytes")
                            })?;
                            if let Extent::Exact(value) = extent
                                && value < summary.minimum_extents[index]
                            {
                                return Err(extent_bad(
                                    function,
                                    statement,
                                    format!(
                                        "helper `{}` requires extent {}, but `{name}` has extent {value}",
                                        target.name, summary.minimum_extents[index]
                                    ),
                                ));
                            }
                            if let Extent::Parameter(parameter) = extent {
                                minimum_extents[parameter] =
                                    minimum_extents[parameter].max(summary.minimum_extents[index]);
                            }
                            *slot = Binding::Moved;
                            actual_extents.push(Some(extent));
                        }
                        Argument::Borrow(name) | Argument::Lend(name) => {
                            let slot = bindings.get(name).ok_or_else(|| {
                                bad(function, statement, "unknown borrowed argument")
                            })?;
                            let extent = binding_extent(slot).ok_or_else(|| {
                                ownership_bad(
                                    function,
                                    statement,
                                    "borrow requires live owned bytes",
                                )
                            })?;
                            if let Extent::Exact(value) = extent
                                && value < summary.minimum_extents[index]
                            {
                                return Err(extent_bad(
                                    function,
                                    statement,
                                    format!(
                                        "helper `{}` requires extent {}, but `{name}` has extent {value}",
                                        target.name, summary.minimum_extents[index]
                                    ),
                                ));
                            }
                            if let Extent::Parameter(parameter) = extent {
                                minimum_extents[parameter] =
                                    minimum_extents[parameter].max(summary.minimum_extents[index]);
                            }
                            actual_extents.push(Some(extent));
                        }
                    }
                    if parameter.ty == Type::I32 && summary.minimum_extents[index] != 0 {
                        return Err(bad(
                            function,
                            statement,
                            "internal scalar extent obligation",
                        ));
                    }
                }
                derived.extend(summary.effects.iter().cloned());
                match target.result {
                    Type::Unit => {
                        if binding.is_some() {
                            return Err(bad(
                                function,
                                statement,
                                "unit helper result cannot be bound",
                            ));
                        }
                    }
                    Type::I32 => {
                        let name = binding.as_ref().ok_or_else(|| {
                            bad(function, statement, "i32 helper result must be bound")
                        })?;
                        if bindings.insert(name.clone(), Binding::Scalar).is_some() {
                            return Err(bad(function, statement, "binding already exists"));
                        }
                    }
                    Type::OwnedBytes => {
                        let name = binding.as_ref().ok_or_else(|| {
                            bad(function, statement, "owned helper result must be bound")
                        })?;
                        let extent = match summary.returned_extent.as_ref().ok_or_else(|| {
                            certificate_bad(
                                function,
                                statement,
                                "owned helper has no owned return summary",
                            )
                        })? {
                            ReturnExtent::Parameter(parameter) => {
                                actual_extents[*parameter].clone().ok_or_else(|| {
                                    ownership_bad(
                                        function,
                                        statement,
                                        "owned return is not an owned argument",
                                    )
                                })?
                            }
                            ReturnExtent::Exact(value) => Extent::Exact(*value),
                        };
                        if bindings
                            .insert(name.clone(), Binding::Owned(extent))
                            .is_some()
                        {
                            return Err(bad(function, statement, "binding already exists"));
                        }
                    }
                    Type::ReadBytes | Type::WriteBytes => {
                        return Err(bad(function, statement, "borrowed results are unsupported"));
                    }
                }
            }
            Statement::Return(value) => {
                let result_ok = matches!(
                    (f.result, value),
                    (Type::Unit, ReturnValue::Unit)
                        | (Type::I32, ReturnValue::Scalar(_))
                        | (Type::OwnedBytes, ReturnValue::Move(_))
                );
                if !result_ok {
                    return Err(bad(
                        function,
                        statement,
                        "return value does not match the function result",
                    ));
                }
                match value {
                    ReturnValue::Unit => {
                        let live = owned_names(&bindings);
                        if !live.is_empty() {
                            return Err(ownership_bad(
                                function,
                                statement,
                                format!("normal exit leaves owned bytes live: {}", live.join(", ")),
                            ));
                        }
                    }
                    ReturnValue::Scalar(BufferOperand::Literal(_)) => {
                        let live = owned_names(&bindings);
                        if !live.is_empty() {
                            return Err(ownership_bad(
                                function,
                                statement,
                                format!("normal exit leaves owned bytes live: {}", live.join(", ")),
                            ));
                        }
                    }
                    ReturnValue::Scalar(BufferOperand::Scalar(name)) => {
                        if !matches!(bindings.get(name), Some(Binding::Scalar)) {
                            return Err(bad(
                                function,
                                statement,
                                "scalar return requires a live i32 binding",
                            ));
                        }
                        let live = owned_names(&bindings);
                        if !live.is_empty() {
                            return Err(ownership_bad(
                                function,
                                statement,
                                format!("normal exit leaves owned bytes live: {}", live.join(", ")),
                            ));
                        }
                    }
                    ReturnValue::Move(name) => {
                        let slot = bindings.get_mut(name).ok_or_else(|| {
                            ownership_bad(function, statement, "unknown owned return binding")
                        })?;
                        let extent = binding_extent(slot).ok_or_else(|| {
                            ownership_bad(
                                function,
                                statement,
                                "owned return requires live owned bytes",
                            )
                        })?;
                        returned_extent = Some(match extent {
                            Extent::Exact(value) => ReturnExtent::Exact(value),
                            Extent::Parameter(parameter) => ReturnExtent::Parameter(parameter),
                        });
                        *slot = Binding::Moved;
                        let live = owned_names(&bindings);
                        if !live.is_empty() {
                            return Err(ownership_bad(
                                function,
                                statement,
                                format!(
                                    "owned return leaves another value live: {}",
                                    live.join(", ")
                                ),
                            ));
                        }
                    }
                }
                returned = true;
            }
        }
    }
    if !returned {
        return Err(bad(
            function,
            f.statements.len(),
            "function has no final return",
        ));
    }
    let effects = EffectRow::new(derived);
    if let Some(declared) = &f.declared_effects
        && !effects.is_subset_of(declared)
    {
        return Err(effects_bad(
            function,
            f.statements.len(),
            "derived helper effects are not contained in the explicit effect row",
        ));
    }
    Ok(Summary {
        effects,
        minimum_extents,
        returned_extent,
    })
}

fn graph(program: &Program) -> Result<(Vec<BTreeSet<usize>>, Vec<usize>), Refusal> {
    let mut graph = vec![BTreeSet::new(); program.functions.len()];
    for (id, function) in program.functions.iter().enumerate() {
        for (statement, item) in function.statements.iter().enumerate() {
            if let Statement::Call { callee, .. } = item {
                if *callee >= program.functions.len() {
                    return Err(bad(id, statement, "unknown helper"));
                }
                graph[id].insert(*callee);
            }
        }
    }
    fn visit(
        id: usize,
        graph: &[BTreeSet<usize>],
        active: &mut BTreeSet<usize>,
        done: &mut BTreeSet<usize>,
        order: &mut Vec<usize>,
    ) -> Result<(), Refusal> {
        if done.contains(&id) {
            return Ok(());
        }
        if !active.insert(id) {
            return Err(bad(
                id,
                0,
                "recursive resource helper calls are unsupported",
            ));
        }
        if active.len() > MAX_CALL_DEPTH {
            return Err(bad(id, 0, "resource helper call depth bound exceeded"));
        }
        for callee in &graph[id] {
            visit(*callee, graph, active, done, order)?;
        }
        active.remove(&id);
        done.insert(id);
        order.push(id);
        Ok(())
    }
    let mut active = BTreeSet::new();
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    for id in 0..program.functions.len() {
        visit(id, &graph, &mut active, &mut done, &mut order)?;
    }
    // `done` makes cycle detection linear, but it must not make the depth bound
    // depend on declaration order. `order` is postorder, so every callee depth
    // is available before its caller's longest-path depth is computed.
    let mut depths = vec![1usize; graph.len()];
    for id in &order {
        depths[*id] = 1 + graph[*id]
            .iter()
            .map(|callee| depths[*callee])
            .max()
            .unwrap_or(0);
        if depths[*id] > MAX_CALL_DEPTH {
            return Err(bad(*id, 0, "resource helper call depth bound exceeded"));
        }
    }
    Ok((graph, order))
}

#[derive(Default)]
struct Expansion {
    events: Vec<BufferEvent>,
    ownership: Vec<OwnershipOp>,
    next_resource: ResourceId,
    next_call: usize,
    expanded_items: usize,
}

enum ExpandedReturn {
    Unit,
    Scalar(String),
    Owned { place: String, physical: String },
}

fn rename_operand(operand: &BufferOperand, names: &BTreeMap<String, String>) -> BufferOperand {
    match operand {
        BufferOperand::Literal(value) => BufferOperand::Literal(*value),
        BufferOperand::Scalar(name) => {
            BufferOperand::Scalar(names.get(name).cloned().unwrap_or_else(|| name.clone()))
        }
    }
}

impl Expansion {
    fn charge(&mut self, function: usize, statement: usize, items: usize) -> Result<(), Refusal> {
        self.expanded_items = self.expanded_items.checked_add(items).ok_or_else(|| {
            bad(
                function,
                statement,
                "resource helper expanded-item bound exceeded",
            )
        })?;
        if self.expanded_items > MAX_EXPANDED_ITEMS {
            return Err(bad(
                function,
                statement,
                "resource helper expanded-item bound exceeded",
            ));
        }
        Ok(())
    }

    fn expand(
        &mut self,
        program: &Program,
        function: usize,
        mut names: BTreeMap<String, String>,
        mut owned_places: BTreeMap<String, String>,
    ) -> Result<ExpandedReturn, Refusal> {
        let f = &program.functions[function];
        let mut returned = ExpandedReturn::Unit;
        for (statement, item) in f.statements.iter().enumerate() {
            self.charge(function, statement, 1)?;
            match item {
                Statement::Buffer(event) => match event {
                    BufferEvent::Bind { binding, value } => {
                        self.charge(function, statement, 1)?;
                        let renamed =
                            format!("$f{function}_c{}_s{statement}_{binding}", self.next_call);
                        names.insert(binding.clone(), renamed.clone());
                        self.events.push(BufferEvent::Bind {
                            binding: renamed,
                            value: *value,
                        });
                    }
                    BufferEvent::Acquire { name, extent } => {
                        self.charge(function, statement, 2)?;
                        let renamed =
                            format!("$f{function}_c{}_s{statement}_{name}", self.next_call);
                        names.insert(name.clone(), renamed.clone());
                        owned_places.insert(name.clone(), renamed.clone());
                        self.events.push(BufferEvent::Acquire {
                            name: renamed.clone(),
                            extent: *extent,
                        });
                        self.ownership.push(OwnershipOp::Acquire {
                            id: self.next_resource,
                            owner: renamed,
                            resource_type: BUFFER_RESOURCE_TYPE.to_owned(),
                            footprint: Footprint::single(BYTE_AXIS, u64::from(*extent)),
                        });
                        self.next_resource += 1;
                    }
                    BufferEvent::Store { name, index, value } => {
                        self.charge(function, statement, 2)?;
                        let physical = names.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded bytes binding")
                        })?;
                        let place = owned_places.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded custody binding")
                        })?;
                        let index = rename_operand(index, &names);
                        let value = rename_operand(value, &names);
                        self.events.push(BufferEvent::Store {
                            name: physical,
                            index: index.clone(),
                            value,
                        });
                        let low = operand_minimum(&index).unwrap_or(1) - 1;
                        self.ownership.push(OwnershipOp::WriteRange {
                            place,
                            axis: BYTE_AXIS.to_owned(),
                            low: u64::from(low),
                            high: u64::from(low + 1),
                        });
                    }
                    BufferEvent::Load {
                        binding,
                        name,
                        index,
                    } => {
                        self.charge(function, statement, 2)?;
                        let physical = names.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded bytes binding")
                        })?;
                        let place = owned_places.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded custody binding")
                        })?;
                        let renamed =
                            format!("$f{function}_c{}_s{statement}_{binding}", self.next_call);
                        names.insert(binding.clone(), renamed.clone());
                        let index = rename_operand(index, &names);
                        self.events.push(BufferEvent::Load {
                            binding: renamed,
                            name: physical,
                            index: index.clone(),
                        });
                        let low = operand_minimum(&index).unwrap_or(1) - 1;
                        self.ownership.push(OwnershipOp::ReadRange {
                            place,
                            axis: BYTE_AXIS.to_owned(),
                            low: u64::from(low),
                            high: u64::from(low + 1),
                        });
                    }
                    BufferEvent::Extent { binding, name } => {
                        self.charge(function, statement, 2)?;
                        let physical = names.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded bytes binding")
                        })?;
                        let place = owned_places.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded custody binding")
                        })?;
                        let renamed =
                            format!("$f{function}_c{}_s{statement}_{binding}", self.next_call);
                        names.insert(binding.clone(), renamed.clone());
                        self.events.push(BufferEvent::Extent {
                            binding: renamed,
                            name: physical,
                        });
                        self.ownership.push(OwnershipOp::ReadRange {
                            place,
                            axis: BYTE_AXIS.to_owned(),
                            low: 0,
                            high: 0,
                        });
                    }
                    BufferEvent::Release { name } => {
                        self.charge(function, statement, 2)?;
                        let place = owned_places.get(name).cloned().ok_or_else(|| {
                            certificate_bad(function, statement, "missing expanded owned binding")
                        })?;
                        self.events.push(BufferEvent::Release {
                            name: names.get(name).cloned().unwrap_or_else(|| place.clone()),
                        });
                        self.ownership.push(OwnershipOp::Drop(place));
                    }
                    _ => {
                        return Err(certificate_bad(
                            function,
                            statement,
                            "unsupported expanded buffer event",
                        ));
                    }
                },
                Statement::Call {
                    binding,
                    callee,
                    arguments,
                } => {
                    let generated_arguments = arguments
                        .iter()
                        .filter(|argument| {
                            matches!(
                                argument,
                                Argument::Scalar(BufferOperand::Literal(_))
                                    | Argument::Move(_)
                                    | Argument::Borrow(_)
                                    | Argument::Lend(_)
                            )
                        })
                        .count();
                    let generated_result = usize::from(
                        binding.is_some() && program.functions[*callee].result == Type::OwnedBytes,
                    );
                    self.charge(function, statement, generated_arguments + generated_result)?;
                    let call = self.next_call;
                    self.next_call += 1;
                    let target = &program.functions[*callee];
                    let mut child_names = BTreeMap::new();
                    let mut child_owned = BTreeMap::new();
                    let before = self.ownership.len();
                    let mut wrappers = Vec::new();
                    for (index, (argument, parameter)) in
                        arguments.iter().zip(&target.parameters).enumerate()
                    {
                        match argument {
                            Argument::Scalar(BufferOperand::Literal(value)) => {
                                let scalar = format!("$call{call}_arg{index}");
                                self.events.push(BufferEvent::Bind {
                                    binding: scalar.clone(),
                                    value: *value,
                                });
                                child_names.insert(parameter.name.clone(), scalar);
                            }
                            Argument::Scalar(BufferOperand::Scalar(name)) => {
                                child_names.insert(
                                    parameter.name.clone(),
                                    names.get(name).cloned().ok_or_else(|| {
                                        certificate_bad(
                                            function,
                                            statement,
                                            "missing scalar argument",
                                        )
                                    })?,
                                );
                            }
                            Argument::Move(name) => {
                                let from = owned_places.get(name).cloned().ok_or_else(|| {
                                    certificate_bad(function, statement, "missing owned argument")
                                })?;
                                let to = format!("$call{call}_owned{index}");
                                self.ownership.push(OwnershipOp::Move {
                                    from,
                                    to: to.clone(),
                                });
                                child_names.insert(
                                    parameter.name.clone(),
                                    names.get(name).cloned().unwrap_or_else(|| to.clone()),
                                );
                                child_owned.insert(parameter.name.clone(), to);
                            }
                            Argument::Borrow(name) | Argument::Lend(name) => {
                                let owner = owned_places.get(name).cloned().ok_or_else(|| {
                                    certificate_bad(function, statement, "missing borrowed owner")
                                })?;
                                let borrower = format!("$call{call}_loan{index}");
                                child_names.insert(
                                    parameter.name.clone(),
                                    names.get(name).cloned().unwrap_or_else(|| owner.clone()),
                                );
                                wrappers.push((
                                    matches!(argument, Argument::Lend(_)),
                                    owner,
                                    borrower.clone(),
                                ));
                                child_owned.insert(parameter.name.clone(), borrower);
                            }
                        }
                    }
                    let child_return = self.expand(program, *callee, child_names, child_owned)?;
                    let mut body = self.ownership.split_off(before);
                    for (mutable, owner, borrower) in wrappers.into_iter().rev() {
                        body = vec![if mutable {
                            OwnershipOp::BorrowMutable {
                                owner,
                                borrower,
                                body,
                            }
                        } else {
                            OwnershipOp::BorrowShared {
                                owner,
                                borrower,
                                body,
                            }
                        }];
                    }
                    self.ownership.extend(body);
                    if let Some(binding) = binding {
                        if target.result == Type::OwnedBytes {
                            let ExpandedReturn::Owned {
                                place: returned_place,
                                physical,
                            } = child_return
                            else {
                                return Err(certificate_bad(
                                    function,
                                    statement,
                                    "missing expanded owned return",
                                ));
                            };
                            let result_place = format!("$call{call}_result_{binding}");
                            self.ownership.push(OwnershipOp::Move {
                                from: returned_place,
                                to: result_place.clone(),
                            });
                            names.insert(binding.clone(), physical);
                            owned_places.insert(binding.clone(), result_place);
                        } else if target.result == Type::I32 {
                            let ExpandedReturn::Scalar(physical) = child_return else {
                                return Err(certificate_bad(
                                    function,
                                    statement,
                                    "missing expanded scalar return",
                                ));
                            };
                            names.insert(binding.clone(), physical);
                        }
                    }
                }
                Statement::Return(ReturnValue::Move(name)) => {
                    returned = ExpandedReturn::Owned {
                        place: owned_places.get(name).cloned().ok_or_else(|| {
                            certificate_bad(
                                function,
                                statement,
                                "missing expanded owned return place",
                            )
                        })?,
                        physical: names.get(name).cloned().ok_or_else(|| {
                            certificate_bad(
                                function,
                                statement,
                                "missing expanded owned return value",
                            )
                        })?,
                    };
                }
                Statement::Return(ReturnValue::Scalar(value)) => {
                    let physical = match value {
                        BufferOperand::Scalar(name) => {
                            names.get(name).cloned().ok_or_else(|| {
                                certificate_bad(
                                    function,
                                    statement,
                                    "missing expanded scalar return value",
                                )
                            })?
                        }
                        BufferOperand::Literal(value) => {
                            self.charge(function, statement, 1)?;
                            let name = format!("$f{function}_c{}_ret{statement}", self.next_call);
                            self.events.push(BufferEvent::Bind {
                                binding: name.clone(),
                                value: *value,
                            });
                            name
                        }
                    };
                    returned = ExpandedReturn::Scalar(physical);
                }
                Statement::Return(ReturnValue::Unit) => returned = ExpandedReturn::Unit,
            }
        }
        Ok(returned)
    }
}

pub fn check(program: Program) -> Result<Checked, Refusal> {
    if program.functions.is_empty() || program.functions.len() > MAX_FUNCTIONS {
        return Err(bad(0, 0, "resource helper function count bound exceeded"));
    }
    if program.entry >= program.functions.len() {
        return Err(bad(0, 0, "entry is not a function"));
    }
    if program
        .functions
        .iter()
        .any(|function| function.parameters.len() > MAX_PARAMETERS)
    {
        return Err(bad(0, 0, "resource helper parameter bound exceeded"));
    }
    if program
        .functions
        .iter()
        .map(|f| f.statements.len())
        .sum::<usize>()
        > MAX_STATEMENTS
    {
        return Err(bad(0, 0, "resource helper statement bound exceeded"));
    }
    let entry = &program.functions[program.entry];
    if !entry.parameters.is_empty() || entry.result != Type::Unit {
        return Err(bad(
            program.entry,
            0,
            "resource-calls.v1 entry is `() -> unit`",
        ));
    }
    let (edges, order) = graph(&program)?;
    let mut reachable = BTreeSet::new();
    let mut stack = vec![program.entry];
    while let Some(id) = stack.pop() {
        if reachable.insert(id) {
            stack.extend(edges[id].iter().copied());
        }
    }
    if let Some(id) = (0..program.functions.len()).find(|id| !reachable.contains(id)) {
        return Err(bad(
            id,
            0,
            "every resource helper must be instantiated from the closed entry so its extent obligations are discharged",
        ));
    }
    let mut summaries = vec![None; program.functions.len()];
    for id in order {
        summaries[id] = Some(analyze_function(&program, id, &summaries)?);
    }
    let summaries = summaries
        .into_iter()
        .map(Option::unwrap)
        .collect::<Vec<_>>();
    let mut expansion = Expansion::default();
    expansion.expand(&program, program.entry, BTreeMap::new(), BTreeMap::new())?;
    expansion.charge(program.entry, entry.statements.len(), 1)?;
    expansion.events.push(BufferEvent::Return);
    ownership::check_ownership(&expansion.ownership).map_err(|error| {
        ownership_bad(
            program.entry,
            entry.statements.len(),
            format!("ownership checker refused the call expansion: {error}"),
        )
    })?;
    let entry_summary = &summaries[program.entry];
    let declared_effects = entry
        .declared_effects
        .clone()
        .unwrap_or_else(|| entry_summary.effects.clone());
    let flattened = BufferProgram {
        entry: entry.name.clone(),
        declared_effects,
        // This route has no float value, so no comparison effect.
        float_effects: Default::default(),
        records: Vec::new(),
        events: expansion.events,
    };
    let certificate = std_buffer::check_buffer_program(&flattened).map_err(|errors| {
        certificate_bad(
            program.entry,
            entry.statements.len(),
            format!(
                "byte-buffer checker refused the call expansion: {}",
                errors.first().map(ToString::to_string).unwrap_or_default()
            ),
        )
    })?;
    std_buffer::recheck_buffer_certificate(&flattened, &certificate).map_err(|error| {
        certificate_bad(
            program.entry,
            entry.statements.len(),
            format!("buffer certificate recheck failed: {error}"),
        )
    })?;
    Ok(Checked {
        program,
        summaries,
        flattened,
        certificate,
        ownership: expansion.ownership,
    })
}

#[cfg(test)]
#[path = "../tests/unit/resource_calls.rs"]
mod tests;
