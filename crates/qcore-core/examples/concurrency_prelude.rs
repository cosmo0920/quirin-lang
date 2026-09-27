//! Custody model counterpart of examples/threads/core_context.qr.
//! Run with `cargo run -p qcore-core --example concurrency_prelude`.
//! This models both creation outcomes; it does not launch operating-system threads.
use qcore_core::prelude::concurrency::{
    ScopedTaskLedger,
    scoped_tasks::{ROOT_TASK, ResourceId, ResourceState, TaskLedgerError, TaskOutcome, TaskState},
};

fn run(created: bool) -> Result<(), TaskLedgerError> {
    let mut context = ScopedTaskLedger::default();
    let buffer: ResourceId = 1;
    context.acquire(ROOT_TASK, buffer)?;
    context.enter_scope(ROOT_TASK, 1)?;
    context.reserve(ROOT_TASK, 1, 1, &[buffer])?;
    assert!(context.use_resource(ROOT_TASK, buffer).is_err());

    if created {
        context.commit(ROOT_TASK, 1, 1, 1)?;
        context.use_resource(1, buffer)?;
        context.complete(1, TaskOutcome::OwnedBuffer(buffer), &[buffer])?;
        assert_eq!(context.task_state(1), Some(TaskState::Completed));
        assert!(context.use_resource(ROOT_TASK, buffer).is_err());
        assert_eq!(
            context.exit_scope(ROOT_TASK, 1),
            Err(TaskLedgerError::ScopeBusy(1))
        );
        let completion = context.join(ROOT_TASK, 1)?;
        assert_eq!(completion.outcome, TaskOutcome::OwnedBuffer(buffer));
        assert_eq!(completion.resources, vec![buffer]);
        assert!(context.join(ROOT_TASK, 1).is_err());
    } else {
        context.rollback(ROOT_TASK, 1)?;
        assert_eq!(context.task_state(1), None);
    }

    context.use_resource(ROOT_TASK, buffer)?;
    context.drop_resource(ROOT_TASK, buffer)?;
    context.exit_scope(ROOT_TASK, 1)?;
    assert_eq!(context.resource_state(buffer), Some(ResourceState::Dropped));
    println!("creation {created}: original buffer returned and released; scope closed");
    Ok(())
}

fn main() -> Result<(), TaskLedgerError> {
    run(true)?;
    run(false)
}
