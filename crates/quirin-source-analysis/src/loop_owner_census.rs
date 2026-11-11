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

//! The loop-owner obligation over one program (card **W47-LOOP-OWNER-CONTRACTS**,
//! gateway rule 10p, contract specification §C04.9).
//!
//! [`qcore_c_gateway::thread_entry_census::read_program`] reads the files
//! lexically; this adapter turns that reading into a
//! [`qcore_core::loop_owner::ThreadProgram`], derives every function's
//! contexts, derives each loop's owner from its dispatcher and decides the
//! owner obligation at every call to a function whose contract declares one.
//! The adapter lives here, not in the gateway, because the gateway depends on
//! no kernel crate (workspace layering).

use std::collections::BTreeMap;

use qcore_c_gateway::thread_entry_census::{
    BuildConfiguration, CensusFile, CensusReading, HopTables, PremiseInputs, ReadEffect, ReadHop,
    ReadMode, SitePremise, read_program_premised,
};
use qcore_c_gateway::typed_dispatch_census::TypedFacts;
use qcore_contract::contract_loader::{DeclaredLoopOwner, LoopOwnerOperation};
use qcore_core::loop_owner::{
    Context, Derived, Edge, HopKind, Mode, ModeEffect, OwnerVerdict, Owners, ThreadProgram,
    ThreadRoot, derive, owner_verdict,
};

pub use qcore_c_gateway::thread_entry_census::FLUENT_BIT_HOPS;

/// The obligation's answer at one call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerSite {
    pub file: String,
    pub line: usize,
    /// The calling function's census name.
    pub function: String,
    pub callee: String,
    pub operation: LoopOwnerOperation,
    pub verdict: OwnerVerdict,
}

/// The census of one program.
#[derive(Clone, Debug)]
pub struct Census {
    pub reading: CensusReading,
    pub program: ThreadProgram,
    pub derived: Vec<Derived>,
    pub owners: Owners,
    pub sites: Vec<OwnerSite>,
}

impl Census {
    /// The derivation of a function by census name.
    #[must_use]
    pub fn derived(&self, name: &str) -> Option<&Derived> {
        self.reading.index(name).map(|id| &self.derived[id])
    }

    /// A witness path's names.
    #[must_use]
    pub fn names(&self, path: &[usize]) -> Vec<String> {
        path.iter()
            .map(|id| self.program.functions[*id].clone())
            .collect()
    }
}

fn mode(mode: ReadMode) -> Mode {
    match mode {
        ReadMode::NonThreaded => Mode::NonThreaded,
        ReadMode::Threaded => Mode::Threaded,
    }
}

fn effect(effect: ReadEffect) -> ModeEffect {
    match effect {
        ReadEffect::Keep => ModeEffect::Keep,
        ReadEffect::Require(m) => ModeEffect::Require(mode(m)),
        ReadEffect::Dispatch(m) => ModeEffect::Dispatch(mode(m)),
    }
}

fn hop(hop: ReadHop) -> HopKind {
    match hop {
        ReadHop::Direct => HopKind::Direct,
        ReadHop::Vtable => HopKind::Vtable,
        ReadHop::CollectorRegistry => HopKind::CollectorRegistry,
        ReadHop::Coroutine => HopKind::Coroutine,
    }
}

/// The kernel program of a reading: every root starts in every mode.
#[must_use]
pub fn thread_program(reading: &CensusReading) -> ThreadProgram {
    ThreadProgram {
        functions: reading.functions.clone(),
        roots: reading
            .roots
            .iter()
            .map(|(function, thread)| ThreadRoot {
                function: *function,
                context: Context {
                    thread: thread.clone(),
                    mode: Mode::Either,
                },
            })
            .collect(),
        edges: reading
            .edges
            .iter()
            .map(|read| Edge {
                caller: read.caller,
                callee: read.callee,
                hop: hop(read.hop),
                effect: effect(read.effect),
            })
            .collect(),
        spawns: reading.spawns.clone(),
        unresolved: reading.unresolved.clone(),
        coroutine_bodies: reading.coroutine_bodies.clone(),
        yields: reading.yields.clone(),
    }
}

/// Read, derive and decide every declared obligation of one program, every
/// conditional arm included.
#[must_use]
pub fn census(
    files: &[CensusFile],
    obligations: &BTreeMap<String, DeclaredLoopOwner>,
    hops: &HopTables,
) -> Census {
    census_configured(files, obligations, hops, None)
}

/// [`census`] under a build configuration (card W47-LOOP-OWNER-COROUTINE-HOP):
/// an arm the configuration decides is not taken is not read.
#[must_use]
pub fn census_configured(
    files: &[CensusFile],
    obligations: &BTreeMap<String, DeclaredLoopOwner>,
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
) -> Census {
    census_typed(files, obligations, hops, configuration, None)
}

/// [`census_configured`] with typed dispatch (card
/// W47-LOOP-OWNER-TYPED-DISPATCH): a field dispatch reaches only the
/// registrants the typed reading stored in the same record type; with no
/// typed facts it is exactly [`census_configured`].
#[must_use]
pub fn census_typed(
    files: &[CensusFile],
    obligations: &BTreeMap<String, DeclaredLoopOwner>,
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
    typed: Option<&TypedFacts>,
) -> Census {
    census_premised(files, obligations, hops, configuration, typed, None)
}

/// [`census_typed`] as the premised census (card W47-PREMISES-INSTANCE):
/// Step B's list-head premises are derived from the program or its flag is
/// underived, and a site whose P-INSTANCE derivation is refuted is not
/// discharged (unresolved, with the refutation as its reason). A site whose
/// premise stays stated keeps its verdict; the reading reports which.
#[must_use]
pub fn census_premised(
    files: &[CensusFile],
    obligations: &BTreeMap<String, DeclaredLoopOwner>,
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
    typed: Option<&TypedFacts>,
    premises: Option<&PremiseInputs>,
) -> Census {
    let reading = read_program_premised(files, hops, configuration, typed, premises);
    let program = thread_program(&reading);
    let derived = derive(&program);
    let owners = Owners::from_dispatchers(
        reading.engine_dispatcher.map(|id| &derived[id]),
        reading.worker_dispatcher.map(|id| &derived[id]),
    );
    let sites = reading
        .calls
        .iter()
        .filter_map(|call| {
            let obligation = obligations.get(&call.callee)?;
            let mut verdict = owner_verdict(&derived[call.caller], &owners);
            // Card W47-PREMISES-INSTANCE: a refuted P-INSTANCE withholds the
            // discharge.
            if let Some(report) = &reading.premises
                && let Some(SitePremise::Refuted(why)) = report
                    .instance_sites
                    .get(&format!("{}:{} {}", call.file, call.line, call.callee))
                && matches!(verdict, OwnerVerdict::Discharged { .. })
            {
                verdict = OwnerVerdict::Unresolved {
                    reasons: std::iter::once(why.clone()).collect(),
                };
            }
            Some(OwnerSite {
                file: call.file.clone(),
                line: call.line,
                function: program.functions[call.caller].clone(),
                callee: call.callee.clone(),
                operation: obligation.operation,
                verdict,
            })
        })
        .collect();
    Census {
        reading,
        program,
        derived,
        owners,
        sites,
    }
}
