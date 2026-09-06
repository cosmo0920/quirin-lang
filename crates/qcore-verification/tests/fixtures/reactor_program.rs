use qcore_core::event_reactor::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestProfile(pub bool);
impl ReactorProfile for TestProfile {
    fn threading_enabled(self) -> bool {
        self.0
    }
}

pub fn key(id: u32) -> EventKey {
    EventKey::new(id, 1)
}

fn summary(argument: u32, reason: ActivationReason) -> CallbackSummary {
    CallbackSummary {
        argument,
        argument_mode: ArgumentMode::Owned,
        trigger_reasons: BTreeSet::from([reason]),
        payload_authority: BTreeMap::from([(7, PayloadAccessKind::Write)]),
        may_defer: false,
        may_spawn_tasks: false,
    }
}

pub fn one_shot(base: u32, event: EventKey, argument: u32) -> EventProgram<TestProfile> {
    EventProgram {
        live_arguments: BTreeSet::from([argument]),
        operations: vec![
            EventOperation::CreateBase {
                base,
                profile: TestProfile(true),
            },
            EventOperation::Assign {
                event,
                base,
                descriptor: Some(3),
                flags: EventFlags::new(EventFlags::READ),
                priority: 0,
                summary: summary(argument, ActivationReason::Read),
            },
            EventOperation::Add { event },
            EventOperation::BeginDispatch {
                base,
                dispatcher: 10,
            },
            EventOperation::Activate {
                event,
                reason: ActivationReason::Read,
                cross_thread: false,
            },
            EventOperation::CallbackEnter {
                event,
                reason: ActivationReason::Read,
            },
            EventOperation::Access {
                context: CallbackContext::Event(event),
                payload: 7,
                kind: PayloadAccessKind::Write,
                synchronized: false,
            },
            EventOperation::CallbackExit { event },
            EventOperation::EndDispatch { base },
            EventOperation::FreeEvent { event },
            EventOperation::FreeBase { base },
        ],
    }
}
