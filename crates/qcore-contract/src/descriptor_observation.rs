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

//! Replay native descriptor observations under the descriptor lifecycle policy
//! (card W43-DESCRIPTOR-NATIVE-CHECK).
//!
//! Input is one JSONL record per interposed call, produced by the native
//! `descriptor_observer.c` harness. This module recomputes every identity's
//! generation, custody owner and liveness independently of the observer's
//! stamps, requires the two to agree, and evaluates the policy clauses in
//! `library/descriptor_native_observation.qv` through the shared
//! [`check_contract_facts`] interpreter. A false policy clause is a recorded
//! **violation** of the named property; malformed, reordered, foreign-process
//! or incomplete records are a hard error (the run is not evidence).
//!
//! Records are supplied, not authenticated. The report is runtime evidence
//! about the recorded executions only: never a static certificate, never an
//! upstream defect by itself. Identities never observed being acquired
//! (created by un-interposed code) are counted as untracked, not judged.

use crate::contract_observation::check_contract_facts;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u16 = 1;
const MAX_EXAMPLES: usize = 20;
const EAI_SYSTEM: i64 = -11;

/// The resource kinds the observer distinguishes. Every pointer kind shares
/// one address space; descriptor numbers are their own space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Fd,
    File,
    Dir,
    DlHandle,
    Addrinfo,
    Mapping,
    ProcessStream,
    Ifaddrs,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Fd => "fd",
            Kind::File => "file",
            Kind::Dir => "dir",
            Kind::DlHandle => "dl_handle",
            Kind::Addrinfo => "addrinfo",
            Kind::Mapping => "mapping",
            Kind::ProcessStream => "process_stream",
            Kind::Ifaddrs => "ifaddrs",
        }
    }

    /// A dl handle is reference counted by the loader.
    fn counted(self) -> bool {
        self == Kind::DlHandle
    }

    fn is_stream(self) -> bool {
        matches!(self, Kind::File | Kind::ProcessStream)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Inherited,
    Acquired,
    Derived,
    Adopted,
    Used,
    Released,
    ReleasedChild,
    Replaced,
    Related,
    Live,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub module: String,
    pub offset: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operand {
    pub role: Role,
    pub kind: Kind,
    pub id: i64,
    pub generation: u32,
    #[serde(default)]
    pub extent: Option<i64>,
    #[serde(default)]
    pub child: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InjectionSpec {
    pub call: String,
    pub index: u64,
    pub error: i64,
    /// Module-offset range `[site_lo, site_hi)` the injection counts in;
    /// `0/0` counts every site.
    pub site_lo: u64,
    pub site_hi: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema_version: u16,
    pub sequence: u64,
    pub pid: i64,
    pub thread: i64,
    pub call: String,
    pub site: Site,
    pub args: Vec<i64>,
    pub result: i64,
    pub errno: i64,
    pub injected: bool,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub injections: Option<Vec<InjectionSpec>>,
    /// Return addresses of the recording call chain (innermost first,
    /// including observer frames), for constructors, failures and releases.
    #[serde(default)]
    pub stack: Option<Vec<Site>>,
    pub operands: Vec<Operand>,
}

/// One recorded false policy clause.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub sequence: u64,
    pub call: String,
    pub site: Site,
    pub stack: Vec<Site>,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PropertyResult {
    pub checks: usize,
    pub violations: usize,
    pub examples: Vec<Violation>,
}

/// A generation live when the process exited, with its recorded origin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveGeneration {
    pub kind: Kind,
    pub id: i64,
    pub generation: u32,
    pub inherited: bool,
    pub owner: Option<(Kind, i64)>,
    pub origin_sequence: u64,
    pub origin_call: String,
    pub origin_site: Site,
    pub origin_stack: Vec<Site>,
    pub origin_path: Option<String>,
}

/// Accepted replay of one run. `static_certificate` is always false.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DescriptorRunReport {
    pub records: usize,
    pub calls: BTreeMap<String, usize>,
    pub acquisitions: BTreeMap<String, usize>,
    pub releases: BTreeMap<String, usize>,
    pub reused_identities: usize,
    pub uses: usize,
    pub constructor_failures: usize,
    pub injected_failures: usize,
    pub injections_configured: usize,
    pub untracked_uses: usize,
    pub untracked_releases: usize,
    pub out_of_model: BTreeMap<String, usize>,
    pub properties: BTreeMap<String, PropertyResult>,
    pub live_at_exit: Vec<LiveGeneration>,
    pub static_certificate: bool,
}

/// Property names, in report order, and the policy function each one runs.
pub const PROPERTIES: &[(&str, &str)] = &[
    ("release_at_most_once", "descriptor_release_at_most_once"),
    ("no_use_after_release", "descriptor_no_use_after_release"),
    ("release_matches_kind", "descriptor_release_matches_kind"),
    ("leak_set_well_defined", "descriptor_leak_set_well_defined"),
    ("generation_fresh", "descriptor_generation_fresh"),
    ("counted_reference", "descriptor_counted_reference"),
    ("sentinel_validity", "descriptor_sentinel_validity"),
    ("relation_holds", "descriptor_relation_holds"),
    (
        "operand_identity_valid",
        "descriptor_operand_identity_valid",
    ),
    ("mapping_extent", "descriptor_mapping_extent"),
];

const FD_CONSTRUCTORS: &[&str] = &[
    "open",
    "openat",
    "socket",
    "accept",
    "accept4",
    "dup",
    "fcntl_dupfd",
    "epoll_create",
    "epoll_create1",
    "eventfd",
    "timerfd_create",
    "inotify_init1",
    "memfd_create",
    "mkstemp",
];
const PAIR_CONSTRUCTORS: &[&str] = &["pipe", "pipe2", "socketpair"];
const USE_CALLS: &[&str] = &[
    "accept_enter",
    "fcntl",
    "ioctl",
    "read",
    "write",
    "pread",
    "pwrite",
    "pread64",
    "pwrite64",
    "readv",
    "writev",
    "recv",
    "recvfrom",
    "recvmsg",
    "send",
    "sendto",
    "sendmsg",
    "connect",
    "bind",
    "listen",
    "shutdown",
    "getsockopt",
    "setsockopt",
    "getsockname",
    "getpeername",
    "fstat",
    "fstat64",
    "ftruncate",
    "ftruncate64",
    "lseek",
    "lseek64",
    "fsync",
    "fdatasync",
    "fchmod",
    "flock",
    "fallocate",
    "posix_fallocate",
    "isatty",
    "epoll_wait",
    "epoll_pwait",
    "timerfd_settime",
    "epoll_ctl",
    "fread",
    "fwrite",
    "fgets",
    "fputs",
    "fputc",
    "fgetc",
    "fflush",
    "fseek",
    "ftell",
    "feof",
    "ferror",
    "getline",
    "setvbuf",
    "vfprintf",
    "fprintf",
    "readdir",
    "rewinddir",
    "dlsym",
    "msync",
    "madvise",
];

#[derive(Clone, Debug)]
struct Slot {
    kind: Kind,
    generation: u32,
    live: bool,
    refs: u32,
    /// The stream or directory owning this descriptor, if any.
    owner: Option<(Kind, i64)>,
    /// The descriptor a live stream or directory owns.
    child: Option<i64>,
    extent: Option<i64>,
    /// The current generation was ended by an observed release.
    released: bool,
    inherited: bool,
    origin: u64,
}

struct Origin {
    call: String,
    site: Site,
    stack: Vec<Site>,
    path: Option<String>,
}

struct Replay<'a> {
    policy: &'a str,
    fds: BTreeMap<i64, Slot>,
    ptrs: BTreeMap<i64, Slot>,
    origins: BTreeMap<u64, Origin>,
    report: DescriptorRunReport,
    pid: Option<i64>,
    finished: bool,
}

fn expect(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

impl<'a> Replay<'a> {
    fn new(policy: &'a str) -> Self {
        let properties = PROPERTIES
            .iter()
            .map(|(name, _)| ((*name).to_owned(), PropertyResult::default()))
            .collect();
        Replay {
            policy,
            fds: BTreeMap::new(),
            ptrs: BTreeMap::new(),
            origins: BTreeMap::new(),
            report: DescriptorRunReport {
                records: 0,
                calls: BTreeMap::new(),
                acquisitions: BTreeMap::new(),
                releases: BTreeMap::new(),
                reused_identities: 0,
                uses: 0,
                constructor_failures: 0,
                injected_failures: 0,
                injections_configured: 0,
                untracked_uses: 0,
                untracked_releases: 0,
                out_of_model: BTreeMap::new(),
                properties,
                live_at_exit: Vec::new(),
                static_certificate: false,
            },
            pid: None,
            finished: false,
        }
    }

    /// Evaluate one property's policy clause. A false clause is recorded as
    /// a violation; an interpreter error (missing fact, bad policy) is fatal.
    fn property(
        &mut self,
        property: &str,
        row: &Record,
        facts: &[(&str, i128)],
        detail: impl FnOnce() -> String,
    ) -> Result<bool, String> {
        let function = PROPERTIES
            .iter()
            .find(|(name, _)| *name == property)
            .map(|(_, function)| *function)
            .ok_or_else(|| format!("unknown property {property}"))?;
        let outcome = check_contract_facts(self.policy, function, facts);
        let entry = self
            .report
            .properties
            .get_mut(property)
            .expect("registered property");
        entry.checks += 1;
        match outcome {
            Ok(()) => Ok(true),
            Err(error) if error.contains("observed contract violation") => {
                entry.violations += 1;
                if entry.examples.len() < MAX_EXAMPLES {
                    entry.examples.push(Violation {
                        sequence: row.sequence,
                        call: row.call.clone(),
                        site: row.site.clone(),
                        stack: row.stack.clone().unwrap_or_default(),
                        detail: format!("{}; {error}", detail()),
                    });
                }
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    fn table(&mut self, kind: Kind) -> &mut BTreeMap<i64, Slot> {
        if kind == Kind::Fd {
            &mut self.fds
        } else {
            &mut self.ptrs
        }
    }

    fn slot(&self, kind: Kind, id: i64) -> Option<&Slot> {
        if kind == Kind::Fd {
            self.fds.get(&id)
        } else {
            self.ptrs.get(&id)
        }
    }

    fn current_generation(&self, kind: Kind, id: i64) -> u32 {
        self.slot(kind, id).map_or(0, |slot| slot.generation)
    }

    fn stamp(&self, operand: &Operand) -> Result<(), String> {
        expect(
            operand.generation == self.current_generation(operand.kind, operand.id),
            &format!(
                "observer generation {} disagrees with replay for {} {}",
                operand.generation,
                operand.kind.name(),
                operand.id
            ),
        )
    }

    fn identity_valid(kind: Kind, id: i64) -> bool {
        if kind == Kind::Fd { id >= 0 } else { id > 0 }
    }

    /// Acquire a generation (or a counted reference) for `operand`.
    fn acquire(
        &mut self,
        row: &Record,
        operand: &Operand,
        owner: Option<(Kind, i64)>,
        inherited: bool,
    ) -> Result<(), String> {
        expect(
            Self::identity_valid(operand.kind, operand.id),
            "acquired identity is not valid",
        )?;
        let previous = self.slot(operand.kind, operand.id).cloned();
        let already_live = previous.as_ref().is_some_and(|slot| slot.live);
        let previous_generation = previous.as_ref().map_or(0, |slot| slot.generation);
        let counted = operand.kind.counted()
            && already_live
            && previous
                .as_ref()
                .is_some_and(|slot| slot.kind == operand.kind);
        if counted {
            expect(
                operand.generation == previous_generation,
                "observer generation disagrees for counted reference",
            )?;
            self.property(
                "counted_reference",
                row,
                &[
                    ("counted_kind", 1),
                    ("already_live", i128::from(already_live)),
                    ("previous_generation", previous_generation.into()),
                    ("generation", operand.generation.into()),
                ],
                || format!("{} {}", operand.kind.name(), operand.id),
            )?;
            let slot = self
                .table(operand.kind)
                .get_mut(&operand.id)
                .expect("live slot");
            slot.refs += 1;
            return Ok(());
        }
        expect(
            previous_generation < u32::MAX && operand.generation == previous_generation + 1,
            &format!(
                "observer generation {} disagrees with replay {} for {} {}",
                operand.generation,
                previous_generation + 1,
                operand.kind.name(),
                operand.id
            ),
        )?;
        self.property(
            "generation_fresh",
            row,
            &[
                ("already_live", i128::from(already_live)),
                ("previous_generation", previous_generation.into()),
                ("generation", operand.generation.into()),
            ],
            || {
                format!(
                    "{} {} acquired while generation {previous_generation} is live (unobserved release)",
                    operand.kind.name(),
                    operand.id
                )
            },
        )?;
        if already_live {
            // Recovery after the recorded violation: the previous generation
            // ended unobserved. A child it owned is left as recorded.
            if let Some(slot) = self.table(operand.kind).get_mut(&operand.id) {
                slot.live = false;
            }
        }
        if previous_generation > 0 {
            self.report.reused_identities += 1;
        }
        *self
            .report
            .acquisitions
            .entry(operand.kind.name().into())
            .or_default() += 1;
        self.origins.entry(row.sequence).or_insert_with(|| Origin {
            call: row.call.clone(),
            site: row.site.clone(),
            stack: row.stack.clone().unwrap_or_default(),
            path: row.path.clone(),
        });
        let slot = Slot {
            kind: operand.kind,
            generation: operand.generation,
            live: true,
            refs: 1,
            owner,
            child: operand.child,
            extent: operand.extent,
            released: false,
            inherited,
            origin: row.sequence,
        };
        self.table(operand.kind).insert(operand.id, slot);
        Ok(())
    }

    /// Check a use of `operand`'s identity; `stream_use` accepts either
    /// FILE kind for FILE-API calls.
    fn use_operand(&mut self, row: &Record, operand: &Operand) -> Result<(), String> {
        self.stamp(operand)?;
        self.report.uses += 1;
        // fflush(NULL) flushes every stream and is not a use of an identity.
        if row.call == "fflush" && operand.id == 0 {
            return Ok(());
        }
        let valid = Self::identity_valid(operand.kind, operand.id);
        self.property(
            "operand_identity_valid",
            row,
            &[("identity_valid", i128::from(valid))],
            || {
                format!(
                    "{} used with invalid identity {}",
                    operand.kind.name(),
                    operand.id
                )
            },
        )?;
        if !valid {
            return Ok(());
        }
        let slot = if operand.kind == Kind::Mapping {
            self.containing_mapping(operand.id)
        } else {
            self.slot(operand.kind, operand.id).cloned()
        };
        let Some(slot) = slot else {
            self.report.untracked_uses += 1;
            return Ok(());
        };
        let use_after_release = !slot.live && slot.released;
        if !slot.live && !slot.released {
            self.report.untracked_uses += 1;
        }
        self.property(
            "no_use_after_release",
            row,
            &[("use_after_release", i128::from(use_after_release))],
            || {
                format!(
                    "{} {} generation {} used after its release",
                    operand.kind.name(),
                    operand.id,
                    slot.generation
                )
            },
        )?;
        Ok(())
    }

    fn containing_mapping(&self, address: i64) -> Option<Slot> {
        if let Some(slot) = self.ptrs.get(&address) {
            return Some(slot.clone());
        }
        self.ptrs
            .range(..address)
            .rev()
            .find(|(base, slot)| {
                slot.kind == Kind::Mapping
                    && slot.live
                    && slot
                        .extent
                        .is_some_and(|extent| address < base.saturating_add(extent))
            })
            .map(|(_, slot)| slot.clone())
    }

    /// Release `operand`. `via` is the stream/directory whose release also
    /// releases this child descriptor, if any. Returns the released child.
    fn release(
        &mut self,
        row: &Record,
        operand: &Operand,
        via: Option<(Kind, i64)>,
    ) -> Result<Option<i64>, String> {
        self.stamp(operand)?;
        let valid = Self::identity_valid(operand.kind, operand.id);
        self.property(
            "operand_identity_valid",
            row,
            &[("identity_valid", i128::from(valid))],
            || {
                format!(
                    "{} released with invalid identity {} (result {}, errno {})",
                    operand.kind.name(),
                    operand.id,
                    row.result,
                    row.errno
                )
            },
        )?;
        if !valid {
            return Ok(None);
        }
        let Some(slot) = self.slot(operand.kind, operand.id).cloned() else {
            self.report.untracked_releases += 1;
            return Ok(None);
        };
        if !slot.live {
            if slot.released {
                self.property(
                    "release_at_most_once",
                    row,
                    &[("released_before", 1)],
                    || {
                        format!(
                            "{} {} generation {} released again (result {}, errno {})",
                            operand.kind.name(),
                            operand.id,
                            slot.generation,
                            row.result,
                            row.errno
                        )
                    },
                )?;
            } else {
                self.report.untracked_releases += 1;
            }
            return Ok(None);
        }
        self.property(
            "release_at_most_once",
            row,
            &[("released_before", 0)],
            String::new,
        )?;
        let kind_matches = if operand.kind == Kind::Fd {
            true
        } else {
            slot.kind == operand.kind
        };
        let custody_matches = slot.owner == via;
        self.property(
            "release_matches_kind",
            row,
            &[
                ("kind_matches", i128::from(kind_matches)),
                ("custody_matches", i128::from(custody_matches)),
            ],
            || {
                format!(
                    "{} {} generation {} of kind {} released as {} with owner {:?} via {:?}",
                    operand.kind.name(),
                    operand.id,
                    slot.generation,
                    slot.kind.name(),
                    operand.kind.name(),
                    slot.owner,
                    via
                )
            },
        )?;
        if operand.kind == Kind::Fd && row.call == "close" && row.result == -1 && row.errno == 9 {
            // The kernel reports the live generation as not open: an
            // unobserved release preceded this one.
            self.property(
                "generation_fresh",
                row,
                &[
                    ("already_live", 1),
                    ("previous_generation", slot.generation.into()),
                    ("generation", slot.generation.into()),
                ],
                || {
                    format!(
                        "close of observed-live fd {} failed with EBADF (unobserved release)",
                        operand.id
                    )
                },
            )?;
        }
        let slot_ref = self
            .table(operand.kind)
            .get_mut(&operand.id)
            .expect("live slot");
        if operand.kind.counted() && slot_ref.refs > 1 {
            slot_ref.refs -= 1;
            *self
                .report
                .releases
                .entry(format!("{}_reference", operand.kind.name()))
                .or_default() += 1;
            return Ok(None);
        }
        slot_ref.refs = 0;
        slot_ref.live = false;
        slot_ref.released = true;
        let child = slot_ref.child.take();
        *self
            .report
            .releases
            .entry(operand.kind.name().into())
            .or_default() += 1;
        Ok(child)
    }

    fn sentinel(&mut self, row: &Record, facts: SentinelFacts) -> Result<(), String> {
        if !facts.succeeded {
            self.report.constructor_failures += 1;
            if row.injected {
                self.report.injected_failures += 1;
            }
        }
        expect(
            !row.injected || !facts.succeeded,
            "injected record reports success",
        )?;
        self.property(
            "sentinel_validity",
            row,
            &[
                ("succeeded", i128::from(facts.succeeded)),
                ("result_is_sentinel", i128::from(facts.sentinel)),
                ("result_valid", i128::from(facts.valid)),
                ("acquired_count", facts.acquired as i128),
                ("expected_acquired", facts.expected as i128),
                ("errno_required", i128::from(facts.errno_required)),
                ("errno_value", row.errno.into()),
            ],
            || {
                format!(
                    "result {} errno {} acquired {} expected {}",
                    row.result, row.errno, facts.acquired, facts.expected
                )
            },
        )?;
        Ok(())
    }

    fn relation(&mut self, row: &Record, holds: bool, detail: String) -> Result<(), String> {
        self.property(
            "relation_holds",
            row,
            &[("relation_matches", i128::from(holds))],
            || detail,
        )?;
        Ok(())
    }

    fn step(&mut self, row: &Record) -> Result<(), String> {
        expect(
            row.schema_version == SCHEMA_VERSION,
            "unsupported schema version",
        )?;
        expect(!self.finished, "record after the exit record")?;
        match self.pid {
            None => {
                expect(row.call == "start", "first record is not the start record")?;
                self.pid = Some(row.pid);
            }
            Some(pid) => expect(row.pid == pid, "record from a different process")?,
        }
        *self.report.calls.entry(row.call.clone()).or_default() += 1;
        let ops = &row.operands;
        let roles: Vec<Role> = ops.iter().map(|operand| operand.role).collect();
        let call = row.call.as_str();
        match call {
            "start" => {
                expect(row.sequence == 1, "start record is not first")?;
                let injections = row.injections.as_ref().ok_or("start lacks injections")?;
                self.report.injections_configured = injections.len();
                for operand in ops {
                    expect(
                        operand.role == Role::Inherited,
                        "start operand is not inherited",
                    )?;
                    self.acquire(row, operand, None, true)?;
                }
                // Inherited streams own their recorded descriptors.
                for operand in ops.iter().filter(|operand| operand.kind.is_stream()) {
                    let child = operand
                        .child
                        .ok_or("inherited stream lacks its descriptor")?;
                    if let Some(slot) = self.fds.get_mut(&child).filter(|slot| slot.live) {
                        slot.owner = Some((operand.kind, operand.id));
                    }
                }
            }
            "exit" => self.exit(row)?,
            _ if FD_CONSTRUCTORS.contains(&call) => {
                let (sources, rest) = split_used(ops);
                expect(sources.len() <= 1, "constructor has more than one source")?;
                for source in sources {
                    self.use_operand(row, source)?;
                }
                let succeeded = row.result >= 0;
                expect(
                    rest.iter()
                        .all(|operand| operand.role == Role::Acquired && operand.kind == Kind::Fd),
                    "constructor operand is not an acquired descriptor",
                )?;
                let acquired = rest.len();
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == -1,
                        valid: succeeded,
                        acquired,
                        expected: 1,
                        errno_required: true,
                    },
                )?;
                for operand in rest {
                    expect(
                        operand.id == row.result,
                        "acquired descriptor is not the result",
                    )?;
                    self.acquire(row, operand, None, false)?;
                }
                if (call == "dup" || call == "fcntl_dupfd")
                    && let (Some(source), true) = (sources.first(), succeeded)
                {
                    self.relation(
                        row,
                        row.result != source.id,
                        format!("dup result {} equals its source", row.result),
                    )?;
                }
            }
            _ if PAIR_CONSTRUCTORS.contains(&call) => {
                expect(
                    ops.iter()
                        .all(|operand| operand.role == Role::Acquired && operand.kind == Kind::Fd),
                    "pair operand is not an acquired descriptor",
                )?;
                let succeeded = row.result == 0;
                let valid =
                    ops.len() == 2 && ops[0].id >= 0 && ops[1].id >= 0 && ops[0].id != ops[1].id;
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == -1,
                        valid,
                        acquired: ops.len(),
                        expected: 2,
                        errno_required: true,
                    },
                )?;
                for operand in ops {
                    self.acquire(row, operand, None, false)?;
                }
            }
            "dup2" | "dup3" => {
                expect(
                    matches!(
                        roles.as_slice(),
                        [Role::Used]
                            | [Role::Used, Role::Acquired]
                            | [Role::Used, Role::Replaced, Role::Acquired]
                    ),
                    "dup2 operand shape",
                )?;
                expect(row.args.len() == 3, "dup2 arguments")?;
                self.use_operand(row, &ops[0])?;
                let succeeded = row.result >= 0;
                let expected = usize::from(succeeded && row.result != row.args[0]);
                let acquired = roles.iter().filter(|role| **role == Role::Acquired).count();
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == -1,
                        valid: succeeded,
                        acquired,
                        expected,
                        errno_required: true,
                    },
                )?;
                if succeeded {
                    self.relation(
                        row,
                        row.result == row.args[1],
                        format!("dup2 result {} is not target {}", row.result, row.args[1]),
                    )?;
                }
                // dup2 onto a stream's descriptor (redirecting stdio) replaces
                // the generation under the same owner: the stream now refers
                // to the new generation, so custody transfers with it.
                let mut owner = None;
                for operand in &ops[1..] {
                    match operand.role {
                        Role::Replaced => {
                            owner = self.fds.get(&operand.id).and_then(|slot| slot.owner);
                            self.release(row, operand, owner)?;
                        }
                        _ => self.acquire(row, operand, owner, false)?,
                    }
                }
            }
            "close" => {
                expect(
                    matches!(roles.as_slice(), [Role::Released]) && ops[0].kind == Kind::Fd,
                    "close operand shape",
                )?;
                expect(
                    row.args.first() == Some(&ops[0].id),
                    "close operand is not its argument",
                )?;
                self.release(row, &ops[0], None)?;
            }
            "fopen" | "tmpfile" | "fdopen" | "popen" | "opendir" | "fdopendir" => {
                self.open_stream(row)?
            }
            "fclose" | "pclose" | "closedir" => {
                let kind = match call {
                    "fclose" => Kind::File,
                    "pclose" => Kind::ProcessStream,
                    _ => Kind::Dir,
                };
                expect(
                    matches!(
                        roles.as_slice(),
                        [Role::Released] | [Role::Released, Role::ReleasedChild]
                    ) && ops[0].kind == kind,
                    "stream release operand shape",
                )?;
                let child = self.release(row, &ops[0], None)?;
                match (ops.get(1), child) {
                    (Some(recorded), Some(expected_child)) => {
                        expect(
                            recorded.kind == Kind::Fd && recorded.id == expected_child,
                            "released child is not the stream's descriptor",
                        )?;
                        self.release(row, recorded, Some((ops[0].kind, ops[0].id)))?;
                    }
                    (Some(recorded), None) => {
                        // The observer released a child the replay does not
                        // attribute to this stream (untracked or already
                        // released stream): replay it without an owner claim.
                        self.release(row, recorded, Some((ops[0].kind, ops[0].id)))?;
                    }
                    (None, Some(_)) => {
                        return Err("stream release omitted its descriptor".into());
                    }
                    (None, None) => {}
                }
            }
            "fileno" | "dirfd" => {
                expect(
                    matches!(roles.as_slice(), [Role::Used] | [Role::Used, Role::Related]),
                    "relation operand shape",
                )?;
                self.use_operand(row, &ops[0])?;
                if let Some(related) = ops.get(1) {
                    self.stamp(related)?;
                    let owner = self.slot(ops[0].kind, ops[0].id).cloned();
                    if let Some(owner) = owner.filter(|slot| slot.live) {
                        let child = owner.child;
                        let child_live = child
                            .and_then(|fd| self.fds.get(&fd))
                            .is_some_and(|slot| slot.live);
                        self.relation(
                            row,
                            child == Some(related.id) && child_live,
                            format!(
                                "{call} returned {} but the stream owns {:?} (live {child_live})",
                                related.id, child
                            ),
                        )?;
                    }
                }
            }
            "dlopen" => {
                expect(
                    ops.iter().all(|operand| {
                        operand.role == Role::Acquired && operand.kind == Kind::DlHandle
                    }),
                    "dlopen operand shape",
                )?;
                let succeeded = row.result != 0;
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == 0,
                        valid: succeeded,
                        acquired: ops.len(),
                        expected: 1,
                        errno_required: false,
                    },
                )?;
                for operand in ops {
                    expect(operand.id == row.result, "dl handle is not the result")?;
                    self.acquire(row, operand, None, false)?;
                }
            }
            "getaddrinfo" | "getifaddrs" => {
                let kind = if call == "getaddrinfo" {
                    Kind::Addrinfo
                } else {
                    Kind::Ifaddrs
                };
                expect(
                    ops.iter()
                        .all(|operand| operand.role == Role::Acquired && operand.kind == kind),
                    "list constructor operand shape",
                )?;
                let succeeded = row.result == 0;
                let (sentinel, errno_required) = if call == "getaddrinfo" {
                    (row.result != 0, row.result == EAI_SYSTEM)
                } else {
                    (row.result == -1, true)
                };
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel,
                        valid: ops.iter().all(|operand| operand.id > 0),
                        acquired: ops.len(),
                        expected: 1,
                        errno_required,
                    },
                )?;
                for operand in ops {
                    self.acquire(row, operand, None, false)?;
                }
            }
            "freeaddrinfo" | "freeifaddrs" | "dlclose" => {
                let kind = match call {
                    "freeaddrinfo" => Kind::Addrinfo,
                    "freeifaddrs" => Kind::Ifaddrs,
                    _ => Kind::DlHandle,
                };
                expect(
                    matches!(roles.as_slice(), [Role::Released]) && ops[0].kind == kind,
                    "list release operand shape",
                )?;
                self.release(row, &ops[0], None)?;
            }
            "mmap" => {
                let (sources, rest) = split_used(ops);
                expect(sources.len() <= 1, "mmap has more than one source")?;
                for source in sources {
                    self.use_operand(row, source)?;
                }
                expect(
                    rest.iter().all(|operand| {
                        operand.role == Role::Acquired && operand.kind == Kind::Mapping
                    }),
                    "mmap operand shape",
                )?;
                let succeeded = row.result != -1;
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == -1,
                        valid: row.result > 0,
                        acquired: rest.len(),
                        expected: 1,
                        errno_required: true,
                    },
                )?;
                for operand in rest {
                    expect(
                        operand.id == row.result && operand.extent == row.args.first().copied(),
                        "mapping is not the result with its length",
                    )?;
                    self.acquire(row, operand, None, false)?;
                }
            }
            "munmap" => {
                expect(
                    matches!(roles.as_slice(), [Role::Released]) && ops[0].kind == Kind::Mapping,
                    "munmap operand shape",
                )?;
                self.release_mapping(row, &ops[0])?;
            }
            "mremap" => {
                let succeeded = row.result != -1;
                if succeeded {
                    expect(
                        matches!(roles.as_slice(), [Role::Released, Role::Acquired]),
                        "mremap operand shape",
                    )?;
                    self.release_mapping(row, &ops[0])?;
                    self.acquire(row, &ops[1], None, false)?;
                } else {
                    expect(
                        matches!(roles.as_slice(), [Role::Used]),
                        "mremap failure shape",
                    )?;
                    self.use_operand(row, &ops[0])?;
                }
                self.sentinel(
                    row,
                    SentinelFacts {
                        succeeded,
                        sentinel: row.result == -1,
                        valid: row.result > 0,
                        acquired: usize::from(succeeded),
                        expected: 1,
                        errno_required: true,
                    },
                )?;
            }
            "syscall_descriptor" | "fork" => {
                expect(ops.is_empty(), "out-of-model record carries operands")?;
                *self.report.out_of_model.entry(call.into()).or_default() += 1;
            }
            _ if USE_CALLS.contains(&call) => {
                expect(
                    !ops.is_empty()
                        && ops.len() <= 2
                        && ops.iter().all(|operand| operand.role == Role::Used),
                    "use operand shape",
                )?;
                for operand in ops {
                    self.use_operand(row, operand)?;
                }
            }
            _ => return Err(format!("unknown observed call {call}")),
        }
        Ok(())
    }

    fn release_mapping(&mut self, row: &Record, operand: &Operand) -> Result<(), String> {
        let recorded = self.ptrs.get(&operand.id).filter(|slot| slot.live).cloned();
        match recorded {
            Some(slot) => {
                self.property(
                    "mapping_extent",
                    row,
                    &[(
                        "extent_matches",
                        i128::from(slot.kind != Kind::Mapping || slot.extent == operand.extent),
                    )],
                    || {
                        format!(
                            "mapping {} of extent {:?} released with extent {:?}",
                            operand.id, slot.extent, operand.extent
                        )
                    },
                )?;
            }
            None => {
                if self.containing_mapping(operand.id).is_some() {
                    self.property("mapping_extent", row, &[("extent_matches", 0)], || {
                        format!("partial release inside a live mapping at {}", operand.id)
                    })?;
                    self.stamp(operand)?;
                    return Ok(());
                }
            }
        }
        self.release(row, operand, None)?;
        Ok(())
    }

    fn open_stream(&mut self, row: &Record) -> Result<(), String> {
        let call = row.call.as_str();
        let (kind, adopts) = match call {
            "fopen" | "tmpfile" => (Kind::File, false),
            "fdopen" => (Kind::File, true),
            "popen" => (Kind::ProcessStream, false),
            "opendir" => (Kind::Dir, false),
            _ => (Kind::Dir, true),
        };
        let ops = &row.operands;
        let roles: Vec<Role> = ops.iter().map(|operand| operand.role).collect();
        let succeeded = row.result != 0;
        let mut rest = ops.as_slice();
        if adopts {
            expect(
                matches!(roles.first(), Some(Role::Used)),
                "adoption lacks its source",
            )?;
            self.use_operand(row, &ops[0])?;
            rest = &ops[1..];
        }
        let expected_roles: &[Role] = match (succeeded, adopts) {
            (false, _) => &[],
            (true, true) => &[Role::Acquired, Role::Adopted],
            (true, false) => &[Role::Acquired, Role::Derived],
        };
        let shape = rest.iter().map(|operand| operand.role).collect::<Vec<_>>();
        let derived_missing = succeeded && !adopts && shape.as_slice() == [Role::Acquired];
        expect(
            shape.as_slice() == expected_roles || derived_missing,
            "stream constructor operand shape",
        )?;
        let acquired = shape
            .iter()
            .filter(|role| matches!(role, Role::Acquired | Role::Derived))
            .count();
        self.sentinel(
            row,
            SentinelFacts {
                succeeded,
                sentinel: row.result == 0,
                valid: succeeded,
                acquired,
                expected: if adopts || derived_missing { 1 } else { 2 },
                errno_required: true,
            },
        )?;
        if !succeeded {
            return Ok(());
        }
        let stream = &rest[0];
        expect(
            stream.kind == kind && stream.id == row.result,
            "stream operand is not the result",
        )?;
        let mut stream_operand = stream.clone();
        if let Some(child) = rest.get(1) {
            expect(child.kind == Kind::Fd, "stream child is not a descriptor")?;
            stream_operand.child = Some(child.id);
        }
        self.acquire(row, &stream_operand, None, false)?;
        if let Some(child) = rest.get(1) {
            if adopts {
                self.stamp(child)?;
                let source_live = self.fds.get(&child.id).cloned();
                let holds = source_live
                    .as_ref()
                    .is_some_and(|slot| slot.live && slot.owner.is_none())
                    && child.id == ops[0].id;
                self.relation(
                    row,
                    holds || source_live.is_none(),
                    format!(
                        "{call} adopted descriptor {} that is not live and caller-owned",
                        child.id
                    ),
                )?;
                if let Some(slot) = self.fds.get_mut(&child.id).filter(|slot| slot.live) {
                    slot.owner = Some((kind, stream.id));
                }
            } else {
                self.acquire(row, child, Some((kind, stream.id)), false)?;
            }
        }
        Ok(())
    }

    fn exit(&mut self, row: &Record) -> Result<(), String> {
        let mut observed = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for operand in &row.operands {
            expect(operand.role == Role::Live, "exit operand is not live")?;
            observed.insert((operand.kind, operand.id, operand.generation));
            let space = if operand.kind == Kind::Fd { 0 } else { 1 };
            identities.insert((space, operand.id));
        }
        let unique = identities.len() == row.operands.len();
        let mut replayed = BTreeSet::new();
        let mut live = Vec::new();
        let mut every_origin = true;
        for (id, slot) in self.fds.iter().chain(self.ptrs.iter()) {
            if !slot.live {
                continue;
            }
            replayed.insert((slot.kind, *id, slot.generation));
            let origin = self.origins.get(&slot.origin);
            every_origin &= origin.is_some();
            if let Some(origin) = origin {
                live.push(LiveGeneration {
                    kind: slot.kind,
                    id: *id,
                    generation: slot.generation,
                    inherited: slot.inherited,
                    owner: slot.owner,
                    origin_sequence: slot.origin,
                    origin_call: origin.call.clone(),
                    origin_site: origin.site.clone(),
                    origin_stack: origin.stack.clone(),
                    origin_path: origin.path.clone(),
                });
            }
        }
        let matches = observed == replayed;
        self.property(
            "leak_set_well_defined",
            row,
            &[
                ("observer_live_set_matches", i128::from(matches)),
                ("unique_live_generation_per_identity", i128::from(unique)),
                ("every_live_generation_has_origin", i128::from(every_origin)),
            ],
            || {
                format!(
                    "observer live set differs: only observed {:?}, only replayed {:?}",
                    observed.difference(&replayed).collect::<Vec<_>>(),
                    replayed.difference(&observed).collect::<Vec<_>>()
                )
            },
        )?;
        self.report.live_at_exit = live;
        self.finished = true;
        Ok(())
    }
}

struct SentinelFacts {
    succeeded: bool,
    sentinel: bool,
    valid: bool,
    acquired: usize,
    expected: usize,
    errno_required: bool,
}

fn split_used(ops: &[Operand]) -> (&[Operand], &[Operand]) {
    let used = ops
        .iter()
        .take_while(|operand| operand.role == Role::Used)
        .count();
    ops.split_at(used)
}

/// Replay one run's JSONL records under `policy` (the text of
/// `library/descriptor_native_observation.qv`). Returns the accepted
/// report, including every recorded violation; errors mean the records are
/// not evidence (malformed, foreign, reordered, incomplete or inconsistent).
pub fn check_descriptor_observations(
    source: &str,
    policy: &str,
) -> Result<DescriptorRunReport, String> {
    let mut replay = Replay::new(policy);
    for (index, line) in source.lines().enumerate() {
        let row: Record =
            serde_json::from_str(line).map_err(|error| format!("record {}: {error}", index + 1))?;
        if row.sequence != index as u64 + 1 {
            return Err(format!("record {}: noncontiguous sequence", index + 1));
        }
        replay
            .step(&row)
            .map_err(|error| format!("record {} ({}): {error}", index + 1, row.call))?;
        replay.report.records += 1;
    }
    if !replay.finished {
        return Err("missing exit record".into());
    }
    Ok(replay.report)
}

#[cfg(test)]
#[path = "../tests/unit/descriptor_observation.rs"]
mod tests;
