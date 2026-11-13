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

//! Card **W45-FB-ENGINE-PROGRESS-MEASURE** (gateway rule 44): a checked
//! measure for a counted `for` loop, recorded as a bound fact beside the
//! resource verdict.
//!
//! This is a **fact, not a verdict**. It reads Clang's typed AST of the unit's
//! preprocessed text and never reads or writes custody state; a custody
//! verdict reads none of it, and it produces no progress verdict. A loop either
//! carries a [`Measure`] (the bound `B - c0` and the number of iterations it
//! allows) or a named [`Refusal`]; any doubt is a refusal.
//!
//! **The admitted shape.** A `for` loop
//!
//! * whose step is exactly `c++` or `++c` on a counter `c` that is an
//!   automatic, non-volatile local or formal of a standard integer type other
//!   than `_Bool`, whose address is taken nowhere in the function and which is
//!   not otherwise written in the loop (condition, step or body, nested
//!   statements included);
//! * whose init clause is exactly `c = c0` or the declaration `T c = c0`,
//!   `c0` an integer constant representable in `c`'s type;
//! * whose condition is `c < B`, or a top-level `&&` chain with at least one
//!   such conjunct, where `B` is
//!   * a constant Clang evaluated: a literal (a macro constant is a literal
//!     after Clang's preprocessing), a `ConstantExpr`, an enumerator, or the
//!     array-count idiom `sizeof(a) / sizeof(a[k])` (or `sizeof(*a)`) over an
//!     array whose extent Clang's type of `a` states;
//!   * a *stable* local: an automatic, non-volatile local or formal whose
//!     address is taken nowhere in the function and which is not written in
//!     the loop; or
//!   * any value of an unsigned type at most 16 bits wide (`uint16_t`,
//!     `uint8_t`), whose type alone bounds it even if it changes;
//! * every conversion to the comparison's type (Clang's recorded implicit
//!   casts) preserves values, so `c < B` is the mathematical comparison; and
//! * `B`'s value (a constant) or type range (otherwise) lies within `c`'s
//!   type, so `c++` after a true test can neither wrap nor overflow.
//!
//! The function must call no returns-twice function (`setjmp`, `vfork`, …)
//! and contain no inline assembly or block literal, and the loop must contain no label and no
//! `case` of a `switch` outside it, so nothing enters the loop except through
//! its init clause and nothing writes the counter except its step. `break`,
//! `return` and a `goto` (necessarily to a label outside the loop) only end
//! the loop early; `continue` reaches the step. They are admitted.
//!
//! **What is recorded.** For each admitted conjunct, the measure `B - c` is
//! non-negative while the test holds and strictly decreases by one per
//! iteration (`QCore.CountedLoopMeasure`); the loop runs at most
//! `max(Bmax - c0, 0)` iterations, `Bmax` the constant, or the maximum of
//! `B`'s type. The smallest admitted bound is recorded.
//!
//! Premises stated, not proved (trust boundary 28): Clang's JSON AST is read
//! faithfully (types, recorded implicit conversions, referenced declarations,
//! literal values), and the target integer model is the analysing Clang's for
//! the unit's flags (rule 11q).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::c_numeric::{CType, IntKind, TargetIntegerModel};
use crate::legacy_c::C11TranslationUnit;
use crate::list_length_bound::{self, FrameSummary, ListBoundFact, ListShape, PendingList};
use qcore_contract::list_finiteness::ListFinitenessPremises;

/// Card W45-FB-LIST-FINITENESS-PREMISE (gateway rule 48): the list-finiteness
/// premises a run declared, when it selected the list-length bound. A
/// rejected premise file declares nothing; its loader error is carried into
/// each refusal's detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListSelection {
    Declared(ListFinitenessPremises),
    Rejected(String),
}

/// The property name recorded with every fact.
pub const LOOP_MEASURE_PROPERTY: &str = "loop-measure.v1";

/// The fact model's version.
pub const MODEL_VERSION: &str = "qcore-loop-measure/1";

/// Why a loop carries no checked measure. The codes are shared with
/// `QCore.CountedLoopMeasure` (`Refusal.code`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Refusal {
    /// A `while` or `do` loop, or a `for` loop with no counted shape.
    NoCounter,
    /// The step is not exactly `c++` or `++c` on a named variable.
    StepNotIncrement,
    /// The counter is not an automatic, non-volatile local or formal of a
    /// standard integer type other than `_Bool`.
    CounterNotLocalInteger,
    /// The counter's address is taken somewhere in the function.
    CounterAddressTaken,
    /// The function calls a returns-twice function (`setjmp`, `vfork`, …),
    /// contains inline assembly or a block literal: control or a write the
    /// AST does not show.
    OpaqueControl,
    /// The loop contains a label, or a `case` of a `switch` outside it.
    LabelInLoop,
    /// The counter is written in the loop other than by its step.
    CounterWrittenInLoop,
    /// The init clause is not `c = c0` / `T c = c0` with a representable
    /// integer constant `c0`.
    InitNotConstant,
    /// No top-level conjunct of the condition is `c < B`.
    TestNotLessThan,
    /// A conversion to the comparison's type may change a value.
    ComparisonConverts,
    /// `B` may exceed the counter type's maximum, so `c++` may wrap.
    CounterMayWrap,
    /// `B` is a local written in the loop (and its type does not bound it).
    BoundWrittenInLoop,
    /// `B` is a local whose address is taken (and its type does not bound it).
    BoundAddressTaken,
    /// `B` is neither a constant, a stable local nor of a narrow unsigned type.
    BoundNotStable,
}

impl Refusal {
    /// The stable code the report and the Lean table spell.
    pub fn code(self) -> &'static str {
        match self {
            Self::NoCounter => "no-counter",
            Self::StepNotIncrement => "step-not-increment",
            Self::CounterNotLocalInteger => "counter-not-local-integer",
            Self::CounterAddressTaken => "counter-address-taken",
            Self::OpaqueControl => "opaque-control",
            Self::LabelInLoop => "label-in-loop",
            Self::CounterWrittenInLoop => "counter-written-in-loop",
            Self::InitNotConstant => "init-not-constant",
            Self::TestNotLessThan => "test-not-less-than",
            Self::ComparisonConverts => "comparison-converts",
            Self::CounterMayWrap => "counter-may-wrap",
            Self::BoundWrittenInLoop => "bound-written-in-loop",
            Self::BoundAddressTaken => "bound-address-taken",
            Self::BoundNotStable => "bound-not-stable",
        }
    }
}

/// How an admitted bound is established.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum BoundKind {
    /// A constant Clang evaluated (literal, macro constant, enumerator,
    /// `ConstantExpr`, array count).
    Constant,
    /// A local or formal neither written in the loop nor address-taken.
    StableLocal,
    /// A value of an unsigned type at most 16 bits wide.
    TypeRange,
}

impl BoundKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Constant => "constant",
            Self::StableLocal => "stable-local",
            Self::TypeRange => "type-range",
        }
    }
}

/// One conjunct `c < B`, abstracted to what the decision reads. Shared with
/// `QCore.CountedLoopMeasure.Conjunct`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Conjunct {
    /// Every conversion to the comparison's type preserves values.
    pub preserving: bool,
    /// `B` is a constant Clang evaluated.
    pub constant: bool,
    /// `B` is an automatic, non-volatile local or formal.
    pub local: bool,
    /// `B` is written in the loop.
    pub written: bool,
    /// `B`'s address is taken in the function.
    pub address_taken: bool,
    /// `B`'s type is unsigned and at most 16 bits wide.
    pub narrow: bool,
    /// `B`'s value (a constant) or type range lies within the counter's type.
    pub fits_counter: bool,
}

/// The loop, abstracted to what the decision reads. Shared with
/// `QCore.CountedLoopMeasure.Shape`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Shape {
    pub is_for: bool,
    pub step_increment: bool,
    pub counter_local_integer: bool,
    pub counter_address_taken: bool,
    pub opaque_control: bool,
    pub label_in_loop: bool,
    pub counter_written: bool,
    pub init_constant: bool,
    pub conjuncts: Vec<Conjunct>,
}

/// The decision for one conjunct (`QCore.CountedLoopMeasure.conjunctKind`).
pub fn conjunct_kind(conjunct: &Conjunct) -> Result<BoundKind, Refusal> {
    if !conjunct.preserving {
        Err(Refusal::ComparisonConverts)
    } else if conjunct.constant {
        if conjunct.fits_counter {
            Ok(BoundKind::Constant)
        } else {
            Err(Refusal::CounterMayWrap)
        }
    } else if conjunct.local && !conjunct.written && !conjunct.address_taken {
        if conjunct.fits_counter {
            Ok(BoundKind::StableLocal)
        } else {
            Err(Refusal::CounterMayWrap)
        }
    } else if conjunct.narrow {
        if conjunct.fits_counter {
            Ok(BoundKind::TypeRange)
        } else {
            Err(Refusal::CounterMayWrap)
        }
    } else if conjunct.local && conjunct.written {
        Err(Refusal::BoundWrittenInLoop)
    } else if conjunct.local && conjunct.address_taken {
        Err(Refusal::BoundAddressTaken)
    } else {
        Err(Refusal::BoundNotStable)
    }
}

/// The decision for one loop (`QCore.CountedLoopMeasure.decide`): `None`
/// when the shape is admitted (some conjunct admits a bound), otherwise the
/// first refusal in this precedence.
pub fn decide(shape: &Shape) -> Option<Refusal> {
    let gates = [
        (!shape.is_for, Refusal::NoCounter),
        (!shape.step_increment, Refusal::StepNotIncrement),
        (
            !shape.counter_local_integer,
            Refusal::CounterNotLocalInteger,
        ),
        (shape.counter_address_taken, Refusal::CounterAddressTaken),
        (shape.opaque_control, Refusal::OpaqueControl),
        (shape.label_in_loop, Refusal::LabelInLoop),
        (shape.counter_written, Refusal::CounterWrittenInLoop),
        (!shape.init_constant, Refusal::InitNotConstant),
    ];
    if let Some((_, refusal)) = gates.iter().find(|(fails, _)| *fails) {
        return Some(*refusal);
    }
    let Some(first) = shape.conjuncts.first() else {
        return Some(Refusal::TestNotLessThan);
    };
    if shape.conjuncts.iter().any(|c| conjunct_kind(c).is_ok()) {
        None
    } else {
        conjunct_kind(first).err()
    }
}

/// One shared vector: the shape's gates, its conjuncts and the expected code.
pub type DecideVector = ([bool; 8], &'static [[bool; 7]], &'static str);

/// Shared decision vectors: `(shape gates, conjuncts, expected code)`, the
/// gates in [`Shape`]'s field order and each conjunct in [`Conjunct`]'s.
/// `"admitted"` is the code of an admitted shape. Identical to
/// `QCore.CountedLoopMeasure.decideVectors`.
pub const DECIDE_VECTORS: &[DecideVector] = &[
    // An admitted constant bound (the output-event batch).
    (
        [true, true, true, false, false, false, false, true],
        &[[true, true, false, false, false, false, true]],
        "admitted",
    ),
    // An admitted stable local (`stored_count`).
    (
        [true, true, true, false, false, false, false, true],
        &[[true, false, true, false, false, true, true]],
        "admitted",
    ),
    // An address-taken narrow local is still bounded by its type.
    (
        [true, true, true, false, false, false, false, true],
        &[[true, false, true, false, true, true, true]],
        "admitted",
    ),
    // `flb_task_create` :652: an address-taken `int` bound.
    (
        [true, true, true, false, false, false, false, true],
        &[[true, false, true, false, true, false, true]],
        "bound-address-taken",
    ),
    // A while loop (`cb_queue_chunks`, `ipow`, `random_uniform`).
    (
        [false, false, false, false, false, false, false, false],
        &[],
        "no-counter",
    ),
    // `i += 2`.
    (
        [true, false, true, false, false, false, false, true],
        &[[true, true, false, false, false, false, true]],
        "step-not-increment",
    ),
    // A pointer cursor.
    (
        [true, true, false, false, false, false, false, true],
        &[],
        "counter-not-local-integer",
    ),
    (
        [true, true, true, true, false, false, false, true],
        &[[true, true, false, false, false, false, true]],
        "counter-address-taken",
    ),
    (
        [true, true, true, false, true, false, false, true],
        &[[true, true, false, false, false, false, true]],
        "opaque-control",
    ),
    (
        [true, true, true, false, false, true, false, true],
        &[[true, true, false, false, false, false, true]],
        "label-in-loop",
    ),
    // A double increment.
    (
        [true, true, true, false, false, false, true, true],
        &[[true, true, false, false, false, false, true]],
        "counter-written-in-loop",
    ),
    (
        [true, true, true, false, false, false, false, false],
        &[[true, true, false, false, false, false, true]],
        "init-not-constant",
    ),
    // `i <= n`, `i != n`.
    (
        [true, true, true, false, false, false, false, true],
        &[],
        "test-not-less-than",
    ),
    // `int i < size_t n`.
    (
        [true, true, true, false, false, false, false, true],
        &[[false, false, true, false, false, false, true]],
        "comparison-converts",
    ),
    // `unsigned char i < 300`.
    (
        [true, true, true, false, false, false, false, true],
        &[[true, true, false, false, false, false, false]],
        "counter-may-wrap",
    ),
    (
        [true, true, true, false, false, false, false, true],
        &[[true, false, true, true, false, false, true]],
        "bound-written-in-loop",
    ),
    // A field bound (`config->task_map_size`).
    (
        [true, true, true, false, false, false, false, true],
        &[[true, false, false, false, false, false, true]],
        "bound-not-stable",
    ),
    // The first conjunct refuses, the second admits.
    (
        [true, true, true, false, false, false, false, true],
        &[
            [true, false, false, false, false, false, true],
            [true, true, false, false, false, false, true],
        ],
        "admitted",
    ),
];

/// Build the abstract shape of a vector row.
pub fn vector_shape(gates: [bool; 8], conjuncts: &[[bool; 7]]) -> Shape {
    let [
        is_for,
        step_increment,
        counter_local_integer,
        counter_address_taken,
        opaque_control,
        label_in_loop,
        counter_written,
        init_constant,
    ] = gates;
    Shape {
        is_for,
        step_increment,
        counter_local_integer,
        counter_address_taken,
        opaque_control,
        label_in_loop,
        counter_written,
        init_constant,
        conjuncts: conjuncts
            .iter()
            .map(
                |&[
                    preserving,
                    constant,
                    local,
                    written,
                    address_taken,
                    narrow,
                    fits_counter,
                ]| Conjunct {
                    preserving,
                    constant,
                    local,
                    written,
                    address_taken,
                    narrow,
                    fits_counter,
                },
            )
            .collect(),
    }
}

/// A checked measure for one loop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Measure {
    /// The counter's name and its type as Clang spells it.
    pub counter: String,
    pub counter_type: String,
    /// The constant the init clause sets.
    pub init: i128,
    pub bound_kind: BoundKind,
    /// The bound: the constant's value, the stable local's name, or the
    /// narrow type's spelling.
    pub bound: String,
    /// The measure, `B - c` (with `c = c0` on entry).
    pub measure: String,
    /// The most iterations the loop can run: `max(Bmax - c0, 0)`.
    pub max_iterations: u128,
    /// How many top-level conjuncts were admitted (the smallest bound is
    /// recorded).
    pub admitted_conjuncts: usize,
}

/// One loop of a function and its fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoopFact {
    /// The loop keyword's line in the analysed (preprocessed) text; a loop
    /// written by a macro is at the macro's use.
    pub line: Option<u64>,
    /// `for`, `while` or `do`.
    pub kind: &'static str,
    pub measure: Option<Measure>,
    /// The refusal's code, when there is no measure.
    pub refusal: Option<&'static str>,
    /// A human-readable account of the refusal.
    pub detail: Option<String>,
    /// Card W45-FB-LIST-FINITENESS-PREMISE (rule 48): the list-length bound
    /// fact or its refusal, only when the run declared list-finiteness
    /// premises; absent otherwise, so rule 44's rows are unchanged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_bound: Option<ListBoundFact>,
}

/// Every loop of one function, in source order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionMeasures {
    pub function: String,
    pub loops: Vec<LoopFact>,
}

impl FunctionMeasures {
    /// The loops carrying a checked measure.
    pub fn measured(&self) -> impl Iterator<Item = (&LoopFact, &Measure)> {
        self.loops
            .iter()
            .filter_map(|fact| fact.measure.as_ref().map(|measure| (fact, measure)))
    }
}

/// The unit's facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoopMeasureReport {
    pub model_version: &'static str,
    pub property: &'static str,
    pub target: String,
    pub functions: Vec<FunctionMeasures>,
}

impl LoopMeasureReport {
    pub fn function(&self, name: &str) -> Option<&FunctionMeasures> {
        self.functions.iter().find(|f| f.function == name)
    }
}

/// Record the facts of every selected function of a unit (every function
/// defined in it when `functions` is empty), from its preprocessed text.
pub fn analyze_loop_measures_preprocessed(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
) -> Result<LoopMeasureReport, String> {
    analyze_loop_measures_preprocessed_with(preprocessed, unit, functions, None)
}

/// [`analyze_loop_measures_preprocessed`], with the list-length bound of
/// rule 48 when `list` is given.
pub fn analyze_loop_measures_preprocessed_with(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    list: Option<&ListSelection>,
) -> Result<LoopMeasureReport, String> {
    let model = crate::c_numeric_obligations::target_model(
        unit,
        crate::c_numeric_obligations::NumericTarget::Default,
    )?;
    let wanted = functions.iter().cloned().collect::<BTreeSet<_>>();
    // A deep `else if` chain nests the AST deeper than serde's default limit
    // and a worker's default stack: read it on a stack sized for it, one
    // top-level declaration at a time.
    let collected = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, || {
                let mut reader = UnitReader::new(&model, &wanted);
                reader.list = list;
                crate::typed_c_ast::translation_unit_ast_with(preprocessed, unit, |bytes| {
                    use serde::de::DeserializeSeed;
                    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
                    deserializer.disable_recursion_limit();
                    TopSeed {
                        reader: &mut reader,
                    }
                    .deserialize(&mut deserializer)
                })?;
                Ok::<_, String>(reader.finish())
            })
            .map_err(|error| format!("cannot start the loop-measure reader: {error}"))?
            .join()
            .map_err(|_| "the loop-measure reader panicked".to_owned())?
    })?;
    Ok(LoopMeasureReport {
        model_version: MODEL_VERSION,
        property: LOOP_MEASURE_PROPERTY,
        target: model.triple.clone(),
        functions: collected,
    })
}

/// Record the facts of a unit given as source: preprocess it as the gateway
/// does, then read the preprocessed text.
pub fn analyze_loop_measures_c11(
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
) -> Result<LoopMeasureReport, String> {
    analyze_loop_measures_c11_with(unit, functions, None)
}

/// [`analyze_loop_measures_c11`], with the list-length bound of rule 48
/// when `list` is given.
pub fn analyze_loop_measures_c11_with(
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    list: Option<&ListSelection>,
) -> Result<LoopMeasureReport, String> {
    let preprocessed =
        crate::legacy_c::validate_unrestricted_c11(unit).map_err(|e| format!("{e:?}"))?;
    analyze_loop_measures_preprocessed_with(&preprocessed, unit, functions, list)
}

/// Record the facts of every function of an AST already read (a whole
/// translation unit), for `model`'s target.
pub fn measures_of_ast(model: &TargetIntegerModel, ast: &Value) -> Vec<FunctionMeasures> {
    let wanted = BTreeSet::new();
    let mut reader = UnitReader::new(model, &wanted);
    for declaration in children(ast) {
        reader.declaration(declaration);
    }
    reader.finish()
}

const STACK: usize = 256 << 20;

struct UnitReader<'a> {
    model: &'a TargetIntegerModel,
    /// Empty: every function.
    wanted: &'a BTreeSet<String>,
    lines: LineTracker,
    enumerators: BTreeMap<String, i128>,
    functions: Vec<FunctionMeasures>,
    /// Rule 48: the run's list selection, every defined body's frame summary
    /// and each loop's pending list decision, resolved once the whole unit
    /// is read (a callee may be defined after its caller).
    list: Option<&'a ListSelection>,
    summaries: BTreeMap<String, FrameSummary>,
    pending: Vec<Vec<Option<PendingList>>>,
}

impl<'a> UnitReader<'a> {
    fn new(model: &'a TargetIntegerModel, wanted: &'a BTreeSet<String>) -> Self {
        Self {
            model,
            wanted,
            lines: LineTracker::default(),
            enumerators: BTreeMap::new(),
            functions: Vec::new(),
            list: None,
            summaries: BTreeMap::new(),
            pending: Vec::new(),
        }
    }

    /// Resolve every pending list decision against the unit's summaries.
    fn finish(self) -> Vec<FunctionMeasures> {
        let Self {
            list,
            summaries,
            pending,
            mut functions,
            ..
        } = self;
        let Some(list) = list else {
            return functions;
        };
        let empty = ListFinitenessPremises::default();
        let (premises, rejected) = match list {
            ListSelection::Declared(premises) => (premises, None),
            ListSelection::Rejected(error) => (&empty, Some(error.as_str())),
        };
        for (function, pending) in functions.iter_mut().zip(pending) {
            for (fact, pending) in function.loops.iter_mut().zip(pending) {
                fact.list_bound = pending.map(|pending| {
                    list_length_bound::resolve(pending, &summaries, premises, rejected)
                });
            }
        }
        functions
    }

    fn declaration(&mut self, declaration: &Value) {
        // Locations first: Clang elides a line equal to the previous one, so
        // every declaration is walked in document order, kept or not.
        let mut loop_lines = BTreeMap::new();
        self.lines.walk(declaration, &mut loop_lines);
        collect_enumerators(declaration, &mut self.enumerators);
        if kind(declaration) != "FunctionDecl" {
            return;
        }
        let name = declaration["name"].as_str().unwrap_or_default();
        let body = children(declaration)
            .iter()
            .find(|node| kind(node) == "CompoundStmt");
        if self.list.is_some()
            && let Some(body) = body
        {
            self.summaries
                .insert(name.to_owned(), FrameSummary::of(body));
        }
        if !self.wanted.is_empty() && !self.wanted.contains(name) {
            return;
        }
        let Some(body) = body else {
            return;
        };
        let scope = FunctionScope::read(declaration, body, self.model);
        let mut loops = Vec::new();
        let context = Context {
            model: self.model,
            enumerators: &self.enumerators,
            scope: &scope,
            lines: &loop_lines,
            list: self.list.is_some(),
        };
        context.loops_in(body, &mut loops);
        let (loops, pending): (Vec<_>, Vec<_>) = loops.into_iter().unzip();
        self.functions.push(FunctionMeasures {
            function: name.to_owned(),
            loops,
        });
        self.pending.push(pending);
    }
}

struct TopSeed<'r, 'a> {
    reader: &'r mut UnitReader<'a>,
}

impl<'de> serde::de::DeserializeSeed<'de> for TopSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> serde::de::Visitor<'de> for TopSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a Clang translation-unit AST")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "inner" {
                map.next_value_seed(InnerSeed {
                    reader: &mut *self.reader,
                })?;
            } else {
                // The translation unit's own location is invalid (`{}`), so
                // skipping its attributes skips no location.
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

struct InnerSeed<'r, 'a> {
    reader: &'r mut UnitReader<'a>,
}

impl<'de> serde::de::DeserializeSeed<'de> for InnerSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> serde::de::Visitor<'de> for InnerSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the translation unit's declarations")
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(declaration) = seq.next_element::<Value>()? {
            self.reader.declaration(&declaration);
        }
        Ok(())
    }
}

/// Clang's `-ast-dump=json` location state, read in document order: a
/// location prints `file` only when it changes and `line` only when the file
/// or line changes, and a macro location prints its spelling location and
/// then its expansion location, each against the same state.
#[derive(Default)]
struct LineTracker {
    file: Option<String>,
    line: Option<u64>,
}

impl LineTracker {
    fn bare(&mut self, location: &Value) {
        if let Some(file) = location["file"].as_str() {
            self.file = Some(file.to_owned());
        }
        if let Some(line) = location["line"].as_u64() {
            self.line = Some(line);
        }
    }

    /// Read one location; the line it denotes (its expansion's, for a macro).
    fn location(&mut self, location: &Value) -> Option<u64> {
        if !location.is_object() {
            return None;
        }
        if location["spellingLoc"].is_object() || location["expansionLoc"].is_object() {
            self.bare(&location["spellingLoc"]);
            self.bare(&location["expansionLoc"]);
        } else {
            if location.as_object().is_some_and(serde_json::Map::is_empty) {
                return None;
            }
            self.bare(location);
        }
        self.line
    }

    /// Walk a node and its children, recording each loop's keyword line by
    /// node address.
    fn walk(&mut self, node: &Value, loops: &mut BTreeMap<usize, Option<u64>>) {
        self.location(&node["loc"]);
        let begin = self.location(&node["range"]["begin"]);
        self.location(&node["range"]["end"]);
        if matches!(kind(node), "ForStmt" | "WhileStmt" | "DoStmt") {
            loops.insert(std::ptr::from_ref(node) as usize, begin);
        }
        for child in children(node) {
            self.walk(child, loops);
        }
    }
}

fn collect_enumerators(node: &Value, into: &mut BTreeMap<String, i128>) {
    if kind(node) == "EnumConstantDecl" {
        // Only an enumerator Clang evaluated (a `ConstantExpr` with a value).
        let value = children(node)
            .first()
            .filter(|first| kind(first) == "ConstantExpr")
            .and_then(|first| first["value"].as_str())
            .and_then(|value| value.parse::<i128>().ok());
        if let (Some(id), Some(value)) = (node["id"].as_str(), value) {
            into.insert(id.to_owned(), value);
        }
    }
    for child in children(node) {
        collect_enumerators(child, into);
    }
}

fn kind(node: &Value) -> &str {
    node["kind"].as_str().unwrap_or("")
}

fn children(node: &Value) -> &[Value] {
    node["inner"].as_array().map_or(&[], Vec::as_slice)
}

/// Clang's canonical spelling of a node's type.
fn spelling(node: &Value) -> &str {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
        .unwrap_or_default()
}

fn written_spelling(node: &Value) -> &str {
    node["type"]["qualType"]
        .as_str()
        .unwrap_or_else(|| spelling(node))
}

fn is_volatile(node: &Value) -> bool {
    [
        &node["type"]["qualType"],
        &node["type"]["desugaredQualType"],
    ]
    .iter()
    .filter_map(|value| value.as_str())
    .any(|text| text.split_whitespace().any(|word| word == "volatile"))
}

/// A standard integer type other than `_Bool`, at the target.
fn integer_type(model: &TargetIntegerModel, spelling: &str) -> Option<CType> {
    match IntKind::parse(spelling)? {
        IntKind::Bool => None,
        other => Some(model.ctype(other)),
    }
}

fn contains(outer: CType, inner: CType) -> bool {
    outer.min() <= inner.min() && inner.max() <= outer.max()
}

fn narrow(ty: CType) -> bool {
    !ty.signed && ty.width <= 16
}

/// Strip parentheses.
fn bare(node: &Value) -> &Value {
    let mut node = node;
    while kind(node) == "ParenExpr" {
        match children(node).first() {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// The declaration a bare `DeclRefExpr` names.
fn named(node: &Value) -> Option<&str> {
    let node = bare(node);
    (kind(node) == "DeclRefExpr")
        .then(|| node["referencedDecl"]["id"].as_str())
        .flatten()
}

/// A local or formal of the function.
struct Local {
    name: String,
    written_spelling: String,
    ty: Option<CType>,
    automatic: bool,
    volatile: bool,
}

/// What the function as a whole says about its locals.
struct FunctionScope {
    locals: BTreeMap<String, Local>,
    address_taken: BTreeSet<String>,
    opaque_control: bool,
}

const RETURNS_TWICE: &[&str] = &[
    "setjmp",
    "_setjmp",
    "__setjmp",
    "sigsetjmp",
    "__sigsetjmp",
    "savectx",
    "vfork",
    "getcontext",
    "__builtin_setjmp",
];

impl FunctionScope {
    fn read(declaration: &Value, body: &Value, model: &TargetIntegerModel) -> Self {
        let mut scope = Self {
            locals: BTreeMap::new(),
            address_taken: BTreeSet::new(),
            opaque_control: false,
        };
        for parameter in children(declaration)
            .iter()
            .filter(|node| kind(node) == "ParmVarDecl")
        {
            scope.declare(parameter, model);
        }
        scope.scan(body, model);
        scope
    }

    fn declare(&mut self, node: &Value, model: &TargetIntegerModel) {
        let Some(id) = node["id"].as_str() else {
            return;
        };
        let storage = node["storageClass"].as_str();
        let automatic = matches!(storage, None | Some("register")) && node["tls"].is_null();
        self.locals.insert(
            id.to_owned(),
            Local {
                name: node["name"].as_str().unwrap_or_default().to_owned(),
                written_spelling: written_spelling(node).to_owned(),
                ty: integer_type(model, spelling(node)),
                automatic,
                volatile: is_volatile(node),
            },
        );
    }

    fn scan(&mut self, node: &Value, model: &TargetIntegerModel) {
        match kind(node) {
            "VarDecl" => self.declare(node, model),
            "UnaryOperator" if node["opcode"] == "&" => {
                if let Some(id) = children(node).first().and_then(named) {
                    self.address_taken.insert(id.to_owned());
                }
            }
            "GCCAsmStmt" | "MSAsmStmt" | "BlockExpr" => self.opaque_control = true,
            "CallExpr" => {
                let callee = children(node).first().map(callee_name);
                if callee
                    .flatten()
                    .is_some_and(|name| RETURNS_TWICE.contains(&name))
                {
                    self.opaque_control = true;
                }
            }
            _ => {}
        }
        for child in children(node) {
            self.scan(child, model);
        }
    }
}

/// The name of a direct callee (through its decay cast).
fn callee_name(node: &Value) -> Option<&str> {
    let mut node = bare(node);
    while kind(node) == "ImplicitCastExpr" {
        node = bare(children(node).first()?);
    }
    (kind(node) == "DeclRefExpr")
        .then(|| node["referencedDecl"]["name"].as_str())
        .flatten()
}

/// Whether `node` (or a node below it, other than `skip`) writes `id`.
fn writes(node: &Value, id: &str, skip: Option<&Value>) -> bool {
    if skip.is_some_and(|skip| std::ptr::eq(skip, node)) {
        return false;
    }
    let target = |index: usize| children(node).get(index).and_then(named) == Some(id);
    let direct = match kind(node) {
        "BinaryOperator" => node["opcode"] == "=" && target(0),
        "CompoundAssignOperator" => target(0),
        "UnaryOperator" => matches!(node["opcode"].as_str(), Some("++" | "--" | "&")) && target(0),
        _ => false,
    };
    direct || children(node).iter().any(|child| writes(child, id, skip))
}

/// Whether the loop contains a label, or a `case`/`default` of a `switch`
/// outside it (a jump into the loop).
fn entered_by_label(node: &Value, in_switch: bool) -> bool {
    match kind(node) {
        "LabelStmt" => true,
        "CaseStmt" | "DefaultStmt" if !in_switch => true,
        "SwitchStmt" => children(node)
            .iter()
            .any(|child| entered_by_label(child, true)),
        _ => children(node)
            .iter()
            .any(|child| entered_by_label(child, in_switch)),
    }
}

struct Context<'a> {
    model: &'a TargetIntegerModel,
    enumerators: &'a BTreeMap<String, i128>,
    scope: &'a FunctionScope,
    lines: &'a BTreeMap<usize, Option<u64>>,
    /// Rule 48 selected: decide each loop's list shape too.
    list: bool,
}

/// A conjunct's reading: its abstract shape and, when admitted, its bound.
struct Reading {
    shape: Conjunct,
    bound: Option<(BoundKind, String, i128)>,
}

impl Context<'_> {
    fn loops_in(&self, node: &Value, into: &mut Vec<(LoopFact, Option<PendingList>)>) {
        let loop_kind = match kind(node) {
            "ForStmt" => Some("for"),
            "WhileStmt" => Some("while"),
            "DoStmt" => Some("do"),
            _ => None,
        };
        if let Some(loop_kind) = loop_kind {
            let pending = self.list.then(|| self.pending_list(node, loop_kind));
            into.push((self.fact(node, loop_kind), pending));
        }
        for child in children(node) {
            self.loops_in(child, into);
        }
    }

    fn fact(&self, node: &Value, loop_kind: &'static str) -> LoopFact {
        let line = self
            .lines
            .get(&(std::ptr::from_ref(node) as usize))
            .copied()
            .flatten();
        match self.measure(node, loop_kind) {
            Ok(measure) => LoopFact {
                line,
                kind: loop_kind,
                measure: Some(measure),
                refusal: None,
                detail: None,
                list_bound: None,
            },
            Err((refusal, detail)) => LoopFact {
                line,
                kind: loop_kind,
                measure: None,
                refusal: Some(refusal.code()),
                detail: Some(detail),
                list_bound: None,
            },
        }
    }

    fn measure(&self, node: &Value, loop_kind: &str) -> Result<Measure, (Refusal, String)> {
        let refuse = |shape: &Shape, detail: String| -> (Refusal, String) {
            (decide(shape).unwrap_or(Refusal::NoCounter), detail)
        };
        let mut shape = Shape::default();
        let parts = children(node);
        if loop_kind != "for" || parts.len() != 5 {
            return Err(refuse(
                &shape,
                format!("a `{loop_kind}` loop has no counter"),
            ));
        }
        shape.is_for = true;
        let (init, condition, step, body) = (&parts[0], &parts[2], &parts[3], &parts[4]);
        // The step: exactly `c++` or `++c`.
        let counter_id = (kind(step) == "UnaryOperator" && step["opcode"] == "++")
            .then(|| children(step).first().and_then(named))
            .flatten();
        let Some(counter_id) = counter_id else {
            return Err(refuse(
                &shape,
                "the step is not exactly `c++` or `++c`".into(),
            ));
        };
        shape.step_increment = true;
        let counter = self.scope.locals.get(counter_id);
        let counter_type = counter
            .filter(|local| local.automatic && !local.volatile)
            .and_then(|local| local.ty);
        let (Some(counter), Some(counter_type)) = (counter, counter_type) else {
            return Err(refuse(
                &shape,
                "the counter is not an automatic, non-volatile local integer".into(),
            ));
        };
        shape.counter_local_integer = true;
        let name = counter.name.clone();
        shape.counter_address_taken = self.scope.address_taken.contains(counter_id);
        shape.opaque_control = self.scope.opaque_control;
        shape.label_in_loop = [condition, step, body]
            .iter()
            .any(|part| entered_by_label(part, false));
        shape.counter_written =
            writes(condition, counter_id, None) || writes(body, counter_id, None);
        let init_value = self.init_value(init, counter_id, counter_type);
        shape.init_constant = init_value.is_some();
        let early = |shape: &Shape| -> Option<String> {
            Some(
                match decide(&Shape {
                    conjuncts: vec![Conjunct::default()],
                    ..shape.clone()
                })? {
                    Refusal::CounterAddressTaken => format!("the address of `{name}` is taken"),
                    Refusal::OpaqueControl => "the function calls a returns-twice function or \
                                           contains inline assembly or a block literal"
                        .into(),
                    Refusal::LabelInLoop => {
                        "the loop contains a label or a `case` of a `switch` outside it".into()
                    }
                    Refusal::CounterWrittenInLoop => {
                        format!("`{name}` is written in the loop other than by its step")
                    }
                    Refusal::InitNotConstant => format!(
                        "the init clause does not set `{name}` to a representable integer constant"
                    ),
                    _ => return None,
                },
            )
        };
        if let Some(detail) = early(&shape) {
            return Err(refuse(&shape, detail));
        }
        let c0 = init_value.unwrap_or_default();
        let mut readings = Vec::new();
        for conjunct in conjuncts(condition) {
            let conjunct = bare(conjunct);
            if kind(conjunct) != "BinaryOperator" || conjunct["opcode"] != "<" {
                continue;
            }
            let operands = children(conjunct);
            if operands.len() != 2
                || self.counter_operand(&operands[0]).map(|(id, _)| id) != Some(counter_id)
            {
                continue;
            }
            readings.push(self.reading(
                &operands[0],
                &operands[1],
                counter_id,
                counter_type,
                body,
                condition,
            ));
        }
        shape.conjuncts = readings.iter().map(|reading| reading.shape).collect();
        if let Some(refusal) = decide(&shape) {
            let detail = match refusal {
                Refusal::TestNotLessThan => format!("no top-level conjunct of the condition is `{name} < B`"),
                Refusal::ComparisonConverts => "a conversion to the comparison's type may change a value".into(),
                Refusal::CounterMayWrap => format!("the bound may exceed the maximum of `{name}`'s type, so `{name}++` may wrap"),
                Refusal::BoundWrittenInLoop => "the bound is a local written in the loop".into(),
                Refusal::BoundAddressTaken => "the bound is a local whose address is taken in the function, and its type does not bound it".into(),
                _ => "the bound is neither a constant, a stable local nor a value of an unsigned type of at most 16 bits".into(),
            };
            return Err((refusal, detail));
        }
        let admitted = readings
            .iter()
            .filter_map(|reading| reading.bound.clone())
            .collect::<Vec<_>>();
        let (bound_kind, bound, bmax) = admitted
            .iter()
            .min_by_key(|(kind, _, bmax)| (*bmax, *kind))
            .cloned()
            .ok_or_else(|| (Refusal::TestNotLessThan, "no admitted conjunct".to_owned()))?;
        let max_iterations = u128::try_from((bmax - c0).max(0)).unwrap_or_default();
        Ok(Measure {
            counter: name.clone(),
            counter_type: counter.written_spelling.clone(),
            init: c0,
            bound_kind,
            measure: format!("{bound} - {name}"),
            bound,
            max_iterations,
            admitted_conjuncts: admitted.len(),
        })
    }

    /// Rule 48: everything the list decision reads from the function itself;
    /// the frame's callees are composed once the unit is read.
    fn pending_list(&self, node: &Value, loop_kind: &str) -> PendingList {
        let mut shape = ListShape::default();
        let parts = children(node);
        let not_a_list = |detail: String| PendingList {
            shape: ListShape::default(),
            header: None,
            local: FrameSummary::default(),
            detail: Some(detail),
        };
        if loop_kind != "for" || parts.len() != 5 {
            return not_a_list(format!("a `{loop_kind}` loop is not a list traversal"));
        }
        let (condition, step, body) = (&parts[2], &parts[3], &parts[4]);
        let Some(header) = list_length_bound::header(&parts[0], condition, step) else {
            return not_a_list("the header is not rule 5l's list traversal".into());
        };
        shape.list_traversal = true;
        let stable = |id: &str| {
            self.scope
                .locals
                .get(id)
                .is_some_and(|local| local.automatic && !local.volatile)
                && !self.scope.address_taken.contains(id)
                && !writes(condition, id, None)
                && !writes(body, id, None)
        };
        let mut detail = None;
        shape.cursor_stable = header.cursor_ids.iter().all(|id| stable(id));
        if !shape.cursor_stable {
            detail = Some(format!(
                "a cursor of `{}` is written in the loop, address-taken or not an automatic local",
                header.cursor
            ));
        }
        shape.head_member = header.head.is_some();
        if detail.is_none() && !shape.head_member {
            detail = Some(format!(
                "the head `{}` is not `&R->M` with `M` a list member of `struct TAG`",
                header.head_text
            ));
        }
        shape.head_root_stable = header
            .head
            .as_ref()
            .is_some_and(|head| stable(&head.root_id));
        if detail.is_none() && !shape.head_root_stable {
            detail = Some(format!(
                "the head's root in `{}` is not an automatic local or formal that the loop \
                 leaves unwritten and whose address is never taken",
                header.head_text
            ));
        }
        shape.opaque_control = self.scope.opaque_control;
        shape.label_in_loop = [condition, step, body]
            .iter()
            .any(|part| entered_by_label(part, false));
        PendingList {
            shape,
            header: Some(header),
            local: FrameSummary::of(body),
            detail,
        }
    }

    /// `c = c0` or `T c = c0`, `c0` a constant representable in `c`'s type.
    fn init_value(&self, init: &Value, counter_id: &str, counter_type: CType) -> Option<i128> {
        let value = match kind(init) {
            "BinaryOperator" if init["opcode"] == "=" => {
                let parts = children(init);
                (parts.len() == 2 && named(&parts[0]) == Some(counter_id))
                    .then(|| self.constant(&parts[1]))
                    .flatten()?
            }
            "DeclStmt" => {
                let [declaration] = children(init) else {
                    return None;
                };
                (kind(declaration) == "VarDecl"
                    && declaration["id"].as_str() == Some(counter_id)
                    && declaration["init"] == "c")
                    .then(|| children(declaration).last().and_then(|e| self.constant(e)))
                    .flatten()?
            }
            _ => return None,
        };
        counter_type.contains(value).then_some(value)
    }

    /// A constant Clang evaluated, through value-preserving conversions only.
    fn constant(&self, node: &Value) -> Option<i128> {
        let ty = |node: &Value| integer_type(self.model, spelling(node));
        match kind(node) {
            "ParenExpr" => self.constant(children(node).first()?),
            "IntegerLiteral" => {
                let value = node["value"].as_str()?.parse::<i128>().ok()?;
                ty(node)?.contains(value).then_some(value)
            }
            "ConstantExpr" => {
                let value = node["value"].as_str()?.parse::<i128>().ok()?;
                ty(node)?.contains(value).then_some(value)
            }
            "DeclRefExpr" if node["referencedDecl"]["kind"] == "EnumConstantDecl" => self
                .enumerators
                .get(node["referencedDecl"]["id"].as_str()?)
                .copied(),
            "ImplicitCastExpr" | "CStyleCastExpr"
                if matches!(node["castKind"].as_str(), Some("IntegralCast" | "NoOp")) =>
            {
                let value = self.constant(children(node).first()?)?;
                ty(node)?.contains(value).then_some(value)
            }
            "UnaryOperator" if node["opcode"] == "-" => {
                let value = -self.constant(children(node).first()?)?;
                ty(node)?.contains(value).then_some(value)
            }
            "BinaryOperator" if node["opcode"] == "/" => self.array_count(node),
            _ => None,
        }
    }

    /// `sizeof(a) / sizeof(a[k])` or `sizeof(a) / sizeof(*a)`, `a` a
    /// one-dimensional array whose extent Clang's type of `a` states.
    fn array_count(&self, node: &Value) -> Option<i128> {
        let [whole, element] = children(node) else {
            return None;
        };
        fn operand(node: &Value) -> Option<&Value> {
            let node = bare(node);
            (kind(node) == "UnaryExprOrTypeTraitExpr" && node["name"] == "sizeof")
                .then(|| children(node).first())
                .flatten()
                .map(bare)
        }
        let array = operand(whole)?;
        let array_id = named(array)?;
        let element = operand(element)?;
        let base = match (kind(element), element["opcode"].as_str()) {
            ("ArraySubscriptExpr", _) => children(element).first()?,
            ("UnaryOperator", Some("*")) => children(element).first()?,
            _ => return None,
        };
        let mut base = bare(base);
        while kind(base) == "ImplicitCastExpr" && base["castKind"] == "ArrayToPointerDecay" {
            base = bare(children(base).first()?);
        }
        if named(base) != Some(array_id) {
            return None;
        }
        array_extent(spelling(array))
    }

    /// The counter side of `c < B`: the counter's id and the comparison's type,
    /// when every conversion on the way preserves values.
    fn counter_operand<'v>(&self, operand: &'v Value) -> Option<(&'v str, bool)> {
        let (inner, preserving) = self.through_conversions(operand);
        named(inner).map(|id| (id, preserving))
    }

    /// Walk down parentheses and recorded conversions; whether every integral
    /// conversion passed preserves every value of its operand's type.
    fn through_conversions<'v>(&self, node: &'v Value) -> (&'v Value, bool) {
        let mut node = bare(node);
        let mut preserving = true;
        while kind(node) == "ImplicitCastExpr" || kind(node) == "CStyleCastExpr" {
            let Some(inner) = children(node).first() else {
                break;
            };
            match node["castKind"].as_str() {
                Some("LValueToRValue" | "NoOp") => {}
                Some("IntegralCast") => {
                    let to = integer_type(self.model, spelling(node));
                    let from = integer_type(self.model, spelling(inner));
                    preserving &=
                        matches!((to, from), (Some(to), Some(from)) if contains(to, from));
                }
                _ => {
                    preserving = false;
                }
            }
            node = bare(inner);
        }
        (node, preserving)
    }

    fn reading(
        &self,
        counter_side: &Value,
        bound_side: &Value,
        counter_id: &str,
        counter_type: CType,
        body: &Value,
        condition: &Value,
    ) -> Reading {
        let comparison = integer_type(self.model, spelling(counter_side));
        let counter_preserving = self
            .counter_operand(counter_side)
            .is_some_and(|(_, preserving)| preserving)
            && comparison.is_some();
        let mut shape = Conjunct::default();
        // A constant.
        if let Some(value) = self.constant(bound_side) {
            shape.constant = true;
            shape.preserving = counter_preserving
                && comparison.is_some_and(|comparison| comparison.contains(value))
                && integer_type(self.model, spelling(bound_side)) == comparison;
            shape.fits_counter = value <= counter_type.max();
            let bound = conjunct_kind(&shape)
                .ok()
                .map(|kind| (kind, value.to_string(), value));
            return Reading { shape, bound };
        }
        let (inner, preserving) = self.through_conversions(bound_side);
        shape.preserving = counter_preserving && preserving;
        let bound_type = integer_type(self.model, spelling(inner));
        let Some(bound_type) = bound_type else {
            shape.preserving = false;
            return Reading { shape, bound: None };
        };
        shape.narrow = narrow(bound_type);
        shape.fits_counter = contains(counter_type, bound_type);
        let mut label = format!("max({})", written_spelling(inner));
        if let Some(id) = named(inner)
            && let Some(local) = self.scope.locals.get(id)
            && local.automatic
            && !local.volatile
            && local.ty.is_some()
            && id != counter_id
        {
            shape.local = true;
            shape.written = writes(body, id, None) || writes(condition, id, None);
            shape.address_taken = self.scope.address_taken.contains(id);
            if !shape.written && !shape.address_taken {
                label.clone_from(&local.name);
            }
        }
        let bound = conjunct_kind(&shape)
            .ok()
            .map(|kind| (kind, label, bound_type.max()));
        Reading { shape, bound }
    }
}

/// The extent of a one-dimensional constant array type spelled `T[N]`.
fn array_extent(spelling: &str) -> Option<i128> {
    let open = spelling.find('[')?;
    let rest = &spelling[open + 1..];
    let close = rest.find(']')?;
    if close + 1 != rest.len() {
        return None;
    }
    let digits = rest[..close].trim();
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// The top-level conjuncts of a condition (`a && b && c`).
fn conjuncts(condition: &Value) -> Vec<&Value> {
    let node = bare(condition);
    if kind(node) == "BinaryOperator" && node["opcode"] == "&&" {
        children(node).iter().flat_map(conjuncts).collect()
    } else if node.is_object() && !node.as_object().is_some_and(serde_json::Map::is_empty) {
        vec![node]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
#[path = "../tests/unit/loop_measure.rs"]
mod tests;
