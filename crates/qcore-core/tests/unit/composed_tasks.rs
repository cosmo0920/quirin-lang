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

//! `source.std-io-tasks.v1` — the join, against the model directly
//! (card `COMPOSE-CONCURRENCY`).
//!
//! Every rule this composition owns is exercised here rather than only through
//! the frontend, because two of them are **unreachable from source**: the
//! lowering refuses a worker line with its own message before a site is ever
//! built, and it can only place a site at a position of the block it is
//! building. Keeping both arms as fail-closed backstops and pinning them at the
//! model is the `STD-IO-COMPOSE` §6.1 precedent — if the reachability ever
//! changes, a test says so rather than an arm quietly passing.

use super::*;
use crate::std_io::{Level, Rendering, StdIoEvent, StdIoOperand};
use crate::task_execution::{
    ScalarType, TaskBlock, TaskEffects, TaskFunction, TaskTerminator, TaskTerminatorKind,
};
use std::collections::BTreeSet;

const SPAN: TaskSpan = TaskSpan { start: 0, end: 1 };

/// `fn main() -> i32 { let n = 42; return n; }`, as the task IR.
///
/// One instruction, so a site may stand at index 0 (before the binding), index 1
/// (after it, before the terminator) and nowhere else.
fn tasks() -> TaskModule {
    let block = TaskBlock {
        instructions: vec![TaskInstruction {
            span: SPAN,
            kind: TaskInstructionKind::BindScalar {
                slot: 1,
                value: ScalarOperand::I32(42),
            },
        }],
        terminator: TaskTerminator {
            span: SPAN,
            kind: TaskTerminatorKind::Return {
                value: ScalarOperand::Slot(1),
            },
        },
    };
    TaskModule {
        entry: 0,
        functions: BTreeMap::from([(
            0,
            TaskFunction {
                name: "main".to_owned(),
                parameters: Vec::new(),
                result: ScalarType::I32,
                errors: BTreeSet::new(),
                error_payloads: BTreeSet::new(),
                declared_effects: TaskEffects::default(),
                entry: 0,
                blocks: BTreeMap::from([(0, block)]),
                span: SPAN,
            },
        )]),
        source_len: 64,
        target: None,
        records: Default::default(),
    }
}

fn program(sites: Vec<TaskIoSite>, declares_io: bool) -> ComposedTaskProgram {
    ComposedTaskProgram {
        tasks: tasks(),
        sites,
        declares_io,
        worker_io: BTreeSet::new(),
    }
}

fn print_n(block: BlockId, index: usize) -> TaskIoSite {
    print_n_in(0, block, index)
}

fn print_n_in(function: FunctionId, block: BlockId, index: usize) -> TaskIoSite {
    TaskIoSite {
        function,
        block,
        index,
        event: StdIoEvent::Print {
            rendering: Rendering::Dec,
            operand: StdIoOperand::Scalar("n".to_owned()),
        },
        operand: Some(1),
        span: SPAN,
    }
}

fn rules(program: &ComposedTaskProgram) -> Vec<ComposedTaskRule> {
    match check_composed_task_program(program) {
        Ok(_) => Vec::new(),
        Err(ComposedTaskRefusal::Join(errors)) => {
            errors.into_iter().map(|error| error.rule).collect()
        }
        Err(ComposedTaskRefusal::Tasks(errors)) => {
            panic!("the task half should check: {errors:?}")
        }
    }
}

#[test]
fn the_task_half_is_checked_first_and_unedited() {
    // A program with no sites at all is the task route's program with an empty
    // list beside it, and it checks.
    let checked = check_composed_task_program(&program(Vec::new(), false)).unwrap();
    assert_eq!(checked.tasks().module(), &tasks());
    assert!(checked.certificate().effects.iter().next().is_none());
    assert!(checked.certificate().placements.is_empty());
}

#[test]
fn a_line_after_its_binding_is_admitted_and_one_before_it_is_not() {
    // The whole liveness question, in one pair. Neither answer is computed
    // here: `readable_probe` puts an ordinary copy of the scalar at exactly the
    // rendering's position and asks `check_task_module`.
    assert_eq!(rules(&program(vec![print_n(0, 1)], true)), Vec::new());
    assert_eq!(
        rules(&program(vec![print_n(0, 0)], true)),
        vec![ComposedTaskRule::OperandNotReadable]
    );
}

#[test]
fn the_probe_is_the_task_models_own_copy() {
    // Stated directly, so the argument in the module header is checkable: the
    // module the probe builds is the program with one `BindScalar` inserted,
    // and it is that module the task checker accepts or refuses.
    let probe = readable_probe(&tasks(), 0, 0, 1, 1, 9).unwrap();
    assert_eq!(probe.functions[&0].blocks[&0].instructions.len(), 2);
    assert_eq!(
        probe.functions[&0].blocks[&0].instructions[1].kind,
        TaskInstructionKind::BindScalar {
            slot: 9,
            value: ScalarOperand::Slot(1),
        }
    );
    assert!(check_task_module(&probe).is_ok());
    assert!(readable_probe(&tasks(), 0, 0, 9, 1, 9).is_none());
    assert!(readable_probe(&tasks(), 0, 7, 0, 1, 9).is_none());
}

#[test]
fn a_line_in_a_function_this_program_never_runs_is_refused() {
    // **The residue of `COMPOSE-CONCURRENCY`'s worker refusal after card
    // `OBS-REORDERING` lifted the covered shapes.** A worker body is admitted
    // (see the `worker_lines` tests below); a function neither the entry nor
    // spawned has no thread to attribute a line to, and is refused.
    assert_eq!(
        rules(&program(vec![print_n_in(0, 7, 0)], true)),
        vec![ComposedTaskRule::SiteOutsideProgram]
    );
    assert_eq!(
        rules(&program(vec![print_n_in(7, 0, 0)], true)),
        vec![ComposedTaskRule::SiteOutsideProgram]
    );
}

#[test]
fn a_line_at_a_position_the_block_does_not_have_is_refused() {
    assert_eq!(
        rules(&program(vec![print_n(0, 5)], true)),
        vec![ComposedTaskRule::SiteOutOfRange]
    );
}

#[test]
fn two_lines_ordered_against_the_program_are_refused() {
    // Source order and performed order must agree, or a projection would have
    // been checked against a program the emitted C does not perform.
    assert_eq!(
        rules(&program(vec![print_n(0, 1), print_n(0, 0)], true)),
        vec![
            ComposedTaskRule::SiteOrder,
            ComposedTaskRule::OperandNotReadable
        ]
    );
    // Two lines at the *same* position are the ordinary case of two consecutive
    // statements and are admitted, in list order.
    assert_eq!(
        rules(&program(vec![print_n(0, 1), print_n(0, 1)], true)),
        Vec::new()
    );
}

#[test]
fn a_borrowed_render_is_refused_on_this_route() {
    let site = TaskIoSite {
        function: 0,
        block: 0,
        index: 1,
        event: StdIoEvent::Log {
            level: Level::Warn,
            rendering: Rendering::Hex,
            operand: StdIoOperand::Borrowed("b".to_owned()),
        },
        operand: None,
        span: SPAN,
    };
    assert_eq!(
        rules(&program(vec![site], true)),
        vec![ComposedTaskRule::UnrenderableOperand]
    );
}

#[test]
fn a_scalar_operand_with_no_slot_is_refused() {
    let mut site = print_n(0, 1);
    site.operand = None;
    assert_eq!(
        rules(&program(vec![site], true)),
        vec![ComposedTaskRule::UnrenderableOperand]
    );
}

#[test]
fn writing_without_declaring_io_is_refused() {
    assert_eq!(
        rules(&program(vec![print_n(0, 1)], false)),
        vec![ComposedTaskRule::EffectNotDeclared]
    );
    // And the rule is `source.std-io.v1`'s own, applied to the projection: a
    // program of only barriers still performs `Io`, and a program of nothing
    // does not.
    let flush = TaskIoSite {
        function: 0,
        block: 0,
        index: 1,
        event: StdIoEvent::Flush,
        operand: None,
        span: SPAN,
    };
    assert_eq!(
        rules(&program(vec![flush.clone()], false)),
        vec![ComposedTaskRule::EffectNotDeclared]
    );
    assert_eq!(rules(&program(vec![flush], true)), Vec::new());
    assert_eq!(rules(&program(Vec::new(), false)), Vec::new());
}

#[test]
fn a_literal_operand_needs_no_slot_and_no_binding() {
    let site = TaskIoSite {
        function: 0,
        block: 0,
        index: 0,
        event: StdIoEvent::Print {
            rendering: Rendering::Hex,
            operand: StdIoOperand::Literal(-1),
        },
        operand: None,
        span: SPAN,
    };
    assert_eq!(rules(&program(vec![site], true)), Vec::new());
}

#[test]
fn the_projection_carries_no_static_scalar_environment() {
    // The one structural difference from `source.std-io-extern.v1`, stated as a
    // fact about the data: a composed task operand's number is computed by the
    // task model at the rendering's own step, so it is not a static binding and
    // does not travel through `StdIoProgram::scalars`.
    let projection = std_io_projection(&program(vec![print_n(0, 1)], true));
    assert!(projection.scalars.is_empty());
    assert!(projection.identities.is_empty());
    assert_eq!(projection.events.len(), 2);
    assert_eq!(projection.events[1], StdIoEvent::Return);
}

#[test]
fn the_certificate_names_the_surface_this_program_uses() {
    let sites = vec![
        print_n(0, 1),
        TaskIoSite {
            function: 0,
            block: 0,
            index: 1,
            event: StdIoEvent::Log {
                level: Level::Fatal,
                rendering: Rendering::Hex,
                operand: StdIoOperand::Literal(3),
            },
            operand: None,
            span: SPAN,
        },
        TaskIoSite {
            function: 0,
            block: 0,
            index: 1,
            event: StdIoEvent::Flush,
            operand: None,
            span: SPAN,
        },
    ];
    let program = program(sites, true);
    let checked = check_composed_task_program(&program).unwrap();
    let certificate = checked.certificate();
    assert_eq!(
        certificate.renderings,
        BTreeSet::from([Rendering::Dec, Rendering::Hex])
    );
    assert_eq!(certificate.levels, BTreeSet::from([Level::Fatal]));
    assert_eq!(
        certificate.streams,
        BTreeSet::from([crate::std_io::Stream::Out, crate::std_io::Stream::Err])
    );
    assert_eq!(certificate.flushes, 1);
    assert_eq!(certificate.placements.len(), 3);
    assert!(recheck_composed_task_certificate(&program, certificate).is_ok());
}

#[test]
fn a_tampered_certificate_does_not_survive_rederivation() {
    let program = program(vec![print_n(0, 1)], true);
    let checked = check_composed_task_program(&program).unwrap();
    let mut tampered = checked.certificate().clone();
    tampered.placements.clear();
    assert_eq!(
        recheck_composed_task_certificate(&program, &tampered),
        Err(ComposedTaskRecheckFailure::Placements)
    );
    let mut tampered = checked.certificate().clone();
    tampered.flushes += 1;
    assert_eq!(
        recheck_composed_task_certificate(&program, &tampered),
        Err(ComposedTaskRecheckFailure::Flushes)
    );
    let mut tampered = checked.certificate().clone();
    tampered.renderings.insert(Rendering::Hex);
    assert_eq!(
        recheck_composed_task_certificate(&program, &tampered),
        Err(ComposedTaskRecheckFailure::Renderings)
    );
    // A program the recheck itself refuses never reaches a field comparison.
    let refused = self::program(vec![print_n(0, 0)], true);
    assert_eq!(
        recheck_composed_task_certificate(&refused, checked.certificate()),
        Err(ComposedTaskRecheckFailure::Refused)
    );
}

#[test]
fn a_rendering_leaves_the_task_module_untouched() {
    // "Composing added no custody path", as a statement about the data: the
    // checked task half of a program with lines is byte-identical to the
    // checked task half of the same program without them.
    let with = check_composed_task_program(&program(vec![print_n(0, 1)], true)).unwrap();
    let without = check_composed_task_program(&program(Vec::new(), false)).unwrap();
    assert_eq!(with.tasks(), without.tasks());
}

// ---------------------------------------------------------------------------
// Card `OBS-REORDERING`: a line in a spawned worker's body.
// ---------------------------------------------------------------------------

/// `fn main() { let n = -42; spawn worker(n); join; }` with
/// `fn worker(v: i32) -> i32 { return v; }`, as the task IR.
fn spawning() -> TaskModule {
    use crate::task_execution::ErrorOperand;
    use TaskInstructionKind as I;
    use TaskTerminatorKind as T;
    let instruction = |kind| TaskInstruction { span: SPAN, kind };
    let block = |instructions: Vec<TaskInstructionKind>, kind| TaskBlock {
        instructions: instructions.into_iter().map(instruction).collect(),
        terminator: TaskTerminator { span: SPAN, kind },
    };
    let main = TaskFunction {
        name: "main".into(),
        parameters: vec![],
        result: ScalarType::I32,
        errors: BTreeSet::from(["SpawnError".to_owned()]),
        error_payloads: BTreeSet::new(),
        declared_effects: TaskEffects {
            alloc: false,
            spawn: true,
            join: true,
            may_block: true,
            comparisons: Default::default(),
        },
        entry: 0,
        span: SPAN,
        blocks: BTreeMap::from([
            (
                0,
                block(
                    vec![
                        I::BindScalar {
                            slot: 0,
                            value: ScalarOperand::I32(-42),
                        },
                        I::EnterScope { scope: 1 },
                    ],
                    T::TrySpawn {
                        scope: 1,
                        worker: 1,
                        arguments: vec![ScalarOperand::Slot(0)],
                        handle_slot: 1,
                        success: 1,
                        spawn_error: 2,
                    },
                ),
            ),
            (
                1,
                block(
                    vec![I::Join {
                        handle: 1,
                        outcome_slot: Some(2),
                    }],
                    T::TryOutcome {
                        result_slot: 2,
                        value_slot: 3,
                        error_slot: 4,
                        success: 3,
                        error: 4,
                    },
                ),
            ),
            (
                2,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Raise {
                        error: ErrorOperand::Tag("SpawnError".into()),
                        payload: None,
                    },
                ),
            ),
            (
                3,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Return {
                        value: ScalarOperand::Slot(3),
                    },
                ),
            ),
            (
                4,
                block(
                    vec![I::ExitScope { scope: 1 }],
                    T::Raise {
                        error: ErrorOperand::Slot(4),
                        payload: None,
                    },
                ),
            ),
        ]),
    };
    let worker = TaskFunction {
        name: "worker".into(),
        parameters: vec![(0, ScalarType::I32)],
        result: ScalarType::I32,
        errors: BTreeSet::new(),
        error_payloads: BTreeSet::new(),
        declared_effects: TaskEffects::default(),
        entry: 0,
        span: SPAN,
        blocks: BTreeMap::from([(
            0,
            block(
                vec![],
                T::Return {
                    value: ScalarOperand::Slot(0),
                },
            ),
        )]),
    };
    TaskModule {
        entry: 0,
        functions: BTreeMap::from([(0, main), (1, worker)]),
        source_len: 1,
        target: None,
        records: Default::default(),
    }
}

fn spawning_program(sites: Vec<TaskIoSite>, worker_io: bool) -> ComposedTaskProgram {
    ComposedTaskProgram {
        tasks: spawning(),
        sites,
        declares_io: true,
        worker_io: if worker_io {
            BTreeSet::from([1])
        } else {
            BTreeSet::new()
        },
    }
}

fn rules_of(program: &ComposedTaskProgram) -> Vec<ComposedTaskRule> {
    match check_composed_task_program(program) {
        Ok(_) => Vec::new(),
        Err(ComposedTaskRefusal::Join(errors)) => {
            errors.into_iter().map(|error| error.rule).collect()
        }
        Err(ComposedTaskRefusal::Tasks(errors)) => {
            panic!("the task half should check: {errors:?}")
        }
    }
}

/// A line rendering the worker's own parameter, in the worker's own body. This
/// is what card `COMPOSE-CONCURRENCY` refused and card `OBS-REORDERING` admits.
fn worker_line() -> TaskIoSite {
    TaskIoSite {
        function: 1,
        block: 0,
        index: 0,
        event: StdIoEvent::Log {
            level: Level::Info,
            rendering: Rendering::Dec,
            operand: StdIoOperand::Scalar("v".to_owned()),
        },
        operand: Some(0),
        span: SPAN,
    }
}

#[test]
fn a_line_in_a_spawned_worker_is_admitted() {
    assert_eq!(
        rules_of(&spawning_program(vec![worker_line()], true)),
        Vec::new()
    );
}

/// **The declaration.** Placing a line in a worker body is what declares the
/// admitted reordering relation, and the certificate says so. A program with no
/// worker line declares nothing and keeps the strict comparison.
#[test]
fn a_worker_line_declares_the_reordering_relation() {
    let with = check_composed_task_program(&spawning_program(vec![worker_line()], true)).unwrap();
    assert!(with.certificate().admits_reordering);
    let without = check_composed_task_program(&spawning_program(Vec::new(), true)).unwrap();
    assert!(!without.certificate().admits_reordering);
    // A **parent** line in the same spawning program declares nothing: `n` is
    // bound at index 0 of block 0, so a line at index 1 reads it.
    let mut parent_line = worker_line();
    parent_line.function = 0;
    parent_line.index = 1;
    let parent_only =
        check_composed_task_program(&spawning_program(vec![parent_line], true)).unwrap();
    assert!(!parent_only.certificate().admits_reordering);
}

#[test]
fn a_tampered_reordering_declaration_does_not_survive_rederivation() {
    let program = spawning_program(vec![worker_line()], true);
    let checked = check_composed_task_program(&program).unwrap();
    let mut tampered = checked.certificate().clone();
    tampered.admits_reordering = false;
    assert_eq!(
        recheck_composed_task_certificate(&program, &tampered),
        Err(ComposedTaskRecheckFailure::Reordering)
    );
}

/// A worker that writes must declare `io` in its **own** row: the effect
/// discipline of this model is per function, and a worker body is a function
/// body.
#[test]
fn a_worker_that_writes_without_declaring_io_is_refused() {
    assert_eq!(
        rules_of(&spawning_program(vec![worker_line()], false)),
        vec![ComposedTaskRule::WorkerEffectNotDeclared]
    );
}

/// The liveness question is still the task checker's, and it is asked of the
/// **worker's** function: a line before the worker's parameter would be
/// readable is refused there exactly as in the parent.
#[test]
fn a_worker_lines_operand_is_asked_of_the_task_checker() {
    let mut site = worker_line();
    site.operand = Some(9); // a slot the worker never binds
    assert_eq!(
        rules_of(&spawning_program(vec![site], true)),
        vec![ComposedTaskRule::OperandNotReadable]
    );
}

#[test]
fn the_spawned_worker_set_is_read_off_the_modules_own_terminators() {
    assert_eq!(spawned_workers(&spawning()), BTreeSet::from([1]));
    assert_eq!(spawned_workers(&tasks()), BTreeSet::new());
}
