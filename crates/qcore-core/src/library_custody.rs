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

//! Pure reference-count, container and synchronized-custody model.
//! Evidence kinds are declared model premises, not proof of native synchronization.
//! Resource identifiers denote unique lifetimes. No provider or source importer is linked.

use std::collections::{BTreeMap, BTreeSet};

pub type ExternalResourceId = u32;
pub type ExternalContainerId = u32;
pub type ExternalContextId = u32;
pub type ExternalEvidenceId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalHolder {
    Local(ExternalContextId),
    Container(ExternalContainerId),
    Thread(ExternalContextId),
    Publication(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalContainerKind {
    IntrusiveList,
    TypedQueue,
    Arena,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationEvidenceKind {
    AtomicPublication,
    ThreadHandoff,
    Reclamation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryCustodyOperation {
    Acquire {
        resource: ExternalResourceId,
        holder: ExternalHolder,
    },
    Retain {
        resource: ExternalResourceId,
        from: ExternalHolder,
        retained: ExternalHolder,
    },
    Release {
        resource: ExternalResourceId,
        holder: ExternalHolder,
    },
    CreateContainer {
        container: ExternalContainerId,
        kind: ExternalContainerKind,
    },
    Store {
        container: ExternalContainerId,
        resource: ExternalResourceId,
        from: ExternalHolder,
    },
    Remove {
        container: ExternalContainerId,
        resource: ExternalResourceId,
        to: ExternalHolder,
    },
    DestroyContainer {
        container: ExternalContainerId,
        destroy_members: bool,
    },
    Publish {
        resource: ExternalResourceId,
        from: ExternalHolder,
        publication: u32,
        evidence: ExternalEvidenceId,
    },
    Handoff {
        resource: ExternalResourceId,
        from: ExternalContextId,
        to: ExternalContextId,
        evidence: ExternalEvidenceId,
    },
    Reclaim {
        resource: ExternalResourceId,
        publication: u32,
        to: ExternalContextId,
        evidence: ExternalEvidenceId,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryCustodyProgram {
    pub evidence: BTreeMap<ExternalEvidenceId, SynchronizationEvidenceKind>,
    pub operations: Vec<LibraryCustodyOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryCustodyDiagnostic {
    DuplicateResource(ExternalResourceId),
    DuplicateContainer(ExternalContainerId),
    MissingResource(ExternalResourceId),
    MissingContainer(ExternalContainerId),
    InvalidHolder(ExternalResourceId),
    RefcountOverflow(ExternalResourceId),
    ArenaRemoval(ExternalContainerId),
    ContainerNotEmpty(ExternalContainerId),
    MissingSynchronization(ExternalEvidenceId),
    LiveResource(ExternalResourceId),
    LiveContainer(ExternalContainerId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCustodyCertificate {
    pub discharged_resources: BTreeSet<ExternalResourceId>,
    pub destroyed_containers: BTreeSet<ExternalContainerId>,
    pub checked_evidence: BTreeSet<ExternalEvidenceId>,
}

pub fn check_library_custody(
    program: &LibraryCustodyProgram,
) -> Result<LibraryCustodyCertificate, Vec<LibraryCustodyDiagnostic>> {
    let mut resources = BTreeMap::<ExternalResourceId, BTreeMap<ExternalHolder, u32>>::new();
    let mut containers = BTreeMap::<ExternalContainerId, ExternalContainerKind>::new();
    let mut discharged = BTreeSet::new();
    let mut destroyed = BTreeSet::new();
    let mut checked_evidence = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();

    for operation in &program.operations {
        match *operation {
            LibraryCustodyOperation::Acquire { resource, holder } => {
                if resources.contains_key(&resource) || discharged.contains(&resource) {
                    diagnostics.insert(LibraryCustodyDiagnostic::DuplicateResource(resource));
                } else if let ExternalHolder::Container(container) = holder
                    && !containers.contains_key(&container)
                {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                } else {
                    resources.insert(resource, BTreeMap::from([(holder, 1)]));
                }
            }
            LibraryCustodyOperation::Retain {
                resource,
                from,
                retained,
            } => match resources.get_mut(&resource) {
                Some(_)
                    if matches!(retained, ExternalHolder::Container(container)
                    if !containers.contains_key(&container)) =>
                {
                    let ExternalHolder::Container(container) = retained else {
                        unreachable!("guard establishes a container holder")
                    };
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                }
                Some(holders) if holders.contains_key(&from) => {
                    let count = holders.entry(retained).or_default();
                    match count.checked_add(1) {
                        Some(incremented) => *count = incremented,
                        None => {
                            diagnostics
                                .insert(LibraryCustodyDiagnostic::RefcountOverflow(resource));
                        }
                    }
                }
                Some(_) => {
                    diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
                }
                None => {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
                }
            },
            LibraryCustodyOperation::Release { resource, holder } => {
                remove_holder(
                    resource,
                    holder,
                    &mut resources,
                    &mut discharged,
                    &mut diagnostics,
                );
            }
            LibraryCustodyOperation::CreateContainer { container, kind } => {
                if containers.insert(container, kind).is_some() || destroyed.contains(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::DuplicateContainer(container));
                }
            }
            LibraryCustodyOperation::Store {
                container,
                resource,
                from,
            } => {
                if !containers.contains_key(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                } else {
                    move_holder(
                        resource,
                        from,
                        ExternalHolder::Container(container),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Remove {
                container,
                resource,
                to,
            } => match containers.get(&container) {
                Some(ExternalContainerKind::Arena) => {
                    diagnostics.insert(LibraryCustodyDiagnostic::ArenaRemoval(container));
                }
                Some(_) => {
                    if let ExternalHolder::Container(target) = to
                        && !containers.contains_key(&target)
                    {
                        diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(target));
                    } else {
                        move_holder(
                            resource,
                            ExternalHolder::Container(container),
                            to,
                            &mut resources,
                            &mut diagnostics,
                        );
                    }
                }
                None => {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                }
            },
            LibraryCustodyOperation::DestroyContainer {
                container,
                destroy_members,
            } => {
                if !containers.contains_key(&container) {
                    diagnostics.insert(LibraryCustodyDiagnostic::MissingContainer(container));
                    continue;
                }
                let members = resources
                    .iter()
                    .filter_map(|(resource, holders)| {
                        holders
                            .contains_key(&ExternalHolder::Container(container))
                            .then_some(*resource)
                    })
                    .collect::<Vec<_>>();
                if !destroy_members && !members.is_empty() {
                    diagnostics.insert(LibraryCustodyDiagnostic::ContainerNotEmpty(container));
                    continue;
                }
                for resource in members {
                    remove_all_holder(
                        resource,
                        ExternalHolder::Container(container),
                        &mut resources,
                        &mut discharged,
                        &mut diagnostics,
                    );
                }
                containers.remove(&container);
                destroyed.insert(container);
            }
            LibraryCustodyOperation::Publish {
                resource,
                from,
                publication,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::AtomicPublication,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        from,
                        ExternalHolder::Publication(publication),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Handoff {
                resource,
                from,
                to,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::ThreadHandoff,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        ExternalHolder::Thread(from),
                        ExternalHolder::Thread(to),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
            LibraryCustodyOperation::Reclaim {
                resource,
                publication,
                to,
                evidence,
            } => {
                if require_evidence(
                    evidence,
                    SynchronizationEvidenceKind::Reclamation,
                    &program.evidence,
                    &mut checked_evidence,
                    &mut diagnostics,
                ) {
                    move_holder(
                        resource,
                        ExternalHolder::Publication(publication),
                        ExternalHolder::Local(to),
                        &mut resources,
                        &mut diagnostics,
                    );
                }
            }
        }
    }
    diagnostics.extend(
        resources
            .keys()
            .copied()
            .map(LibraryCustodyDiagnostic::LiveResource),
    );
    diagnostics.extend(
        containers
            .keys()
            .copied()
            .map(LibraryCustodyDiagnostic::LiveContainer),
    );
    if diagnostics.is_empty() {
        Ok(LibraryCustodyCertificate {
            discharged_resources: discharged,
            destroyed_containers: destroyed,
            checked_evidence,
        })
    } else {
        Err(diagnostics.into_iter().collect())
    }
}

fn move_holder(
    resource: ExternalResourceId,
    from: ExternalHolder,
    to: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) if holders.contains_key(&from) => {
            if from == to {
                return;
            }
            let Some(destination) = holders.get(&to).copied().unwrap_or_default().checked_add(1)
            else {
                diagnostics.insert(LibraryCustodyDiagnostic::RefcountOverflow(resource));
                return;
            };
            decrement_holder(holders, from);
            holders.insert(to, destination);
        }
        Some(_) => {
            diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn remove_holder(
    resource: ExternalResourceId,
    holder: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    discharged: &mut BTreeSet<ExternalResourceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) => {
            if !holders.contains_key(&holder) {
                diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
            } else {
                decrement_holder(holders, holder);
                if holders.is_empty() {
                    resources.remove(&resource);
                    discharged.insert(resource);
                }
            }
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn remove_all_holder(
    resource: ExternalResourceId,
    holder: ExternalHolder,
    resources: &mut BTreeMap<ExternalResourceId, BTreeMap<ExternalHolder, u32>>,
    discharged: &mut BTreeSet<ExternalResourceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) {
    match resources.get_mut(&resource) {
        Some(holders) => {
            if holders.remove(&holder).is_none() {
                diagnostics.insert(LibraryCustodyDiagnostic::InvalidHolder(resource));
            } else if holders.is_empty() {
                resources.remove(&resource);
                discharged.insert(resource);
            }
        }
        None => {
            diagnostics.insert(LibraryCustodyDiagnostic::MissingResource(resource));
        }
    }
}

fn decrement_holder(holders: &mut BTreeMap<ExternalHolder, u32>, holder: ExternalHolder) {
    let count = holders
        .get_mut(&holder)
        .expect("caller checked that the holder exists");
    *count -= 1;
    if *count == 0 {
        holders.remove(&holder);
    }
}

fn require_evidence(
    evidence: ExternalEvidenceId,
    expected: SynchronizationEvidenceKind,
    available: &BTreeMap<ExternalEvidenceId, SynchronizationEvidenceKind>,
    checked: &mut BTreeSet<ExternalEvidenceId>,
    diagnostics: &mut BTreeSet<LibraryCustodyDiagnostic>,
) -> bool {
    if available.get(&evidence) == Some(&expected) {
        checked.insert(evidence);
        true
    } else {
        diagnostics.insert(LibraryCustodyDiagnostic::MissingSynchronization(evidence));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_retains_by_one_holder_are_counted() {
        let program = LibraryCustodyProgram {
            operations: vec![
                LibraryCustodyOperation::Acquire {
                    resource: 1,
                    holder: ExternalHolder::Local(1),
                },
                LibraryCustodyOperation::Retain {
                    resource: 1,
                    from: ExternalHolder::Local(1),
                    retained: ExternalHolder::Local(1),
                },
                LibraryCustodyOperation::Release {
                    resource: 1,
                    holder: ExternalHolder::Local(1),
                },
                LibraryCustodyOperation::Release {
                    resource: 1,
                    holder: ExternalHolder::Local(1),
                },
            ],
            ..LibraryCustodyProgram::default()
        };
        assert!(check_library_custody(&program).is_ok());

        let mut leaked = program;
        leaked.operations.pop();
        assert_eq!(
            check_library_custody(&leaked).unwrap_err(),
            vec![LibraryCustodyDiagnostic::LiveResource(1)]
        );
    }

    #[test]
    fn arena_destructor_cascades_but_queue_and_publication_fail_closed() {
        let arena = LibraryCustodyProgram {
            operations: vec![
                LibraryCustodyOperation::CreateContainer {
                    container: 1,
                    kind: ExternalContainerKind::Arena,
                },
                LibraryCustodyOperation::Acquire {
                    resource: 2,
                    holder: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Retain {
                    resource: 2,
                    from: ExternalHolder::Local(0),
                    retained: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Store {
                    container: 1,
                    resource: 2,
                    from: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::Store {
                    container: 1,
                    resource: 2,
                    from: ExternalHolder::Local(0),
                },
                LibraryCustodyOperation::DestroyContainer {
                    container: 1,
                    destroy_members: true,
                },
            ],
            ..LibraryCustodyProgram::default()
        };
        assert!(check_library_custody(&arena).is_ok());

        let mut missing_evidence = arena;
        missing_evidence.operations.insert(
            2,
            LibraryCustodyOperation::Publish {
                resource: 2,
                from: ExternalHolder::Local(0),
                publication: 1,
                evidence: 99,
            },
        );
        assert!(
            check_library_custody(&missing_evidence)
                .unwrap_err()
                .contains(&LibraryCustodyDiagnostic::MissingSynchronization(99))
        );
    }
}
