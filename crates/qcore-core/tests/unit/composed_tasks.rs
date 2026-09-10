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
    }
}

fn program(sites: Vec<TaskIoSite>, declares_io: bool) -> ComposedTaskProgram {
    ComposedTaskProgram {
        tasks: tasks(),
        sites,
        declares_io,
    }
}

fn print_n(block: BlockId, index: usize) -> TaskIoSite {
    TaskIoSite {
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
    let probe = readable_probe(&tasks(), 0, 1, 1, 9).unwrap();
    assert_eq!(probe.functions[&0].blocks[&0].instructions.len(), 2);
    assert_eq!(
        probe.functions[&0].blocks[&0].instructions[1].kind,
        TaskInstructionKind::BindScalar {
            slot: 9,
            value: ScalarOperand::Slot(1),
        }
    );
    assert!(check_task_module(&probe).is_ok());
    assert!(readable_probe(&tasks(), 0, 9, 1, 9).is_none());
    assert!(readable_probe(&tasks(), 7, 0, 1, 9).is_none());
}

#[test]
fn a_line_outside_the_entry_is_refused() {
    // Unreachable from source — the lowering refuses a worker line first, with
    // its own message. Kept, and pinned here, as the model's own fail-closed
    // statement of the interleaving ground.
    assert_eq!(
        rules(&program(vec![print_n(7, 0)], true)),
        vec![ComposedTaskRule::SiteOutsideEntry]
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
