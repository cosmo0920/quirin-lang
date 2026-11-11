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

//! Condition-variable publication and join-before-release census: the lexical
//! reading (card **W47-CONDVAR-JOIN-PUBLICATION**, C4–C5, gateway rule 10q).
//!
//! # What it reads
//!
//! The same closed set of files as the thread-entry census
//! ([`crate::thread_entry_census`]), with the gateway's tokenizer. Every
//! function body is read into a statement tree (blocks, `if`/`else`, `while`,
//! `for`, `do`, macro loops `NAME(…) { … }`, `return`, `break`, `continue`
//! and expression statements; a statement inside a conditional-compilation
//! group is *maybe executed*). Two interpreters run over the trees and return
//! plain readings; `quirin_source_analysis::condvar_join_census` turns them
//! into `qcore_core::condvar_join` inputs and decides each site.
//!
//! * **C4, per function** ([`read_program`]'s [`FunctionSync`]): a dataflow
//!   over the tree tracks which mutexes are held on every path (`must`) and on
//!   some path (`may`), and which predicate facts were observed under each
//!   mutex since it was last acquired. It records every wait (its enclosing
//!   loop's predicate and body shape), signal, broadcast, `init`/`destroy`,
//!   guarded field access and kept-refusal call with the state at that point.
//!   A wait releases and reacquires its mutex, so it clears the observation.
//! * **C5, per test root** ([`ReleaseReading`]): a path interpreter runs
//!   each test-table root on the test thread, inlining every function whose
//!   direct calls reach a spawn, a join or a release, and tracks per path the
//!   threads started (spawn wrappers, the thread identifiers they write and
//!   the fields those are stored in) and how each was joined: a join whose
//!   zero result the path checked, an unchecked one, a failed one, or none.
//!   It records the join state of every live thread at every release:
//!   [`SyncTables::releases`] calls, the `destroy` of a published local's
//!   condvar or mutex, and the scope end of a local whose address the root
//!   stored into a field.
//! * **Stop paths** (card **W47-CONDVAR-JOIN-STOP-PATHS**): the path reader
//!   tells the first join of a thread the path started (and the wrapper that
//!   started it) from a repeated join, drops the null arm of a field test the
//!   adapter's decider excludes ([`NullTestReading`], the kernel's
//!   `null_branch_excluded`), reads every join/detach identity, every
//!   `pthread_self` and every spawn wrapper's joinability program-wide, and
//!   runs every spawned entry to its exit ([`EntryExit`]) so the adapter can
//!   decide a live thread's own children.
//!
//! Objects are **field-keyed**: `&test.mutex` and `&test->mutex` are the key
//! `tail_pause_test.mutex` when `test` is declared `struct tail_pause_test`.
//! A base that is not a declared plain identifier gives no key.
//!
//! # Fail-closed
//!
//! A `goto`, a `switch`, a statement a conditional group splits, a lock or
//! unlock with no key, a loop the wait shape does not match, an unresolved
//! join identity, a call that may start or join a thread through a hop the
//! path reader does not follow, and a path budget overrun each leave the
//! affected readings unresolved; nothing here admits by default.
//!
//! Premises stated, not proved (trust boundary 28): the tokenizer's reading;
//! the statement reader's structure; that a field-keyed object names one
//! object per instance and a waiter and its signaller name the same instance
//! (P-SYNC-INSTANCE); that a truth macro ([`SyncTables::truth_macros`],
//! acutest's `TEST_CHECK`) is its argument's truth; that a thread identifier
//! stored in a field is the one the same context spawned (P-JOIN-IDENTITY);
//! and that a test's root thread is the only thread live before it starts
//! one (acutest's per-test fork).

// Token-range readers index several parallel slices by position.
#![allow(clippy::needless_range_loop)]

use std::collections::{BTreeMap, BTreeSet};

use crate::legacy_c::{
    ConditionalDirective, Token, is_identifier, matching, tokenize_with_conditionals,
};
use crate::thread_entry_census::{CensusFile, CensusReading, HopTables};

mod lifecycle;
mod stop_paths;

pub use lifecycle::{
    BodyPathReading, LifeHistory, LifecycleBodyReading, LifecycleCallReading, LifecycleRoles,
    LifecycleScenario, ReadLifeEvent, ReadStartResult, ReadTraceJoin,
};
pub use stop_paths::{
    DEFAULT_ATTRIBUTE_CREATE, NullTestReading, SelfSite, ThreadIdSite, WrapperReading,
};

/// The closed binding tables of one program family.
#[derive(Clone, Copy, Debug)]
pub struct SyncTables {
    /// `(wrapper, entry argument, thread-identifier out argument)`.
    pub spawns: &'static [(&'static str, usize, Option<usize>)],
    pub join: &'static str,
    pub lock: &'static str,
    pub unlock: &'static str,
    pub wait: &'static str,
    pub timed_wait: &'static str,
    pub signal: &'static str,
    pub broadcast: &'static str,
    pub cond_init: &'static str,
    pub cond_destroy: &'static str,
    pub mutex_destroy: &'static str,
    /// Macros whose value is their argument's truth.
    pub truth_macros: &'static [&'static str],
    /// Allocation calls whose result is a fresh object.
    pub allocators: &'static [&'static str],
    /// Calls that release storage another thread may retain.
    pub releases: &'static [&'static str],
    /// Calls that end the process (no release follows on the path).
    pub exits: &'static [&'static str],
    /// Calls constraint 27 keeps refused, with their kind.
    pub refused: &'static [(&'static str, ReadRefusal)],
    /// Tokens constraint 27 keeps refused wherever they stand.
    pub refused_tokens: &'static [(&'static str, ReadRefusal)],
}

/// Fluent Bit's tables at `90978ac6` (W47-TAIL-PAUSE-BINDINGS B01–B02, B41–B58).
pub const FLUENT_BIT_SYNC: SyncTables = SyncTables {
    spawns: &[
        // lib/monkey/mk_core/mk_utils.c:297: the thread identifier is `*tid`.
        ("mk_utils_worker_spawn", 0, Some(2)),
        ("pthread_create", 2, Some(0)),
        // src/flb_worker.c:77: the identifier is `*tid`.
        ("flb_worker_create", 0, Some(2)),
        // src/flb_thread_pool.c: the identifier is kept in the pool thread.
        ("flb_tp_thread_create", 1, None),
    ],
    join: "pthread_join",
    lock: "pthread_mutex_lock",
    unlock: "pthread_mutex_unlock",
    wait: "pthread_cond_wait",
    timed_wait: "pthread_cond_timedwait",
    signal: "pthread_cond_signal",
    broadcast: "pthread_cond_broadcast",
    cond_init: "pthread_cond_init",
    cond_destroy: "pthread_cond_destroy",
    mutex_destroy: "pthread_mutex_destroy",
    truth_macros: &["TEST_CHECK"],
    allocators: &["malloc", "calloc", "flb_malloc", "flb_calloc"],
    releases: &["unlink", "flb_destroy"],
    exits: &["exit", "abort", "_exit"],
    refused: &[
        ("pthread_cond_broadcast", ReadRefusal::Broadcast),
        ("cnd_wait", ReadRefusal::C11Condvar),
        ("cnd_timedwait", ReadRefusal::C11Condvar),
        ("cnd_signal", ReadRefusal::C11Condvar),
        ("cnd_broadcast", ReadRefusal::C11Condvar),
        ("pthread_rwlock_rdlock", ReadRefusal::Rwlock),
        ("pthread_rwlock_wrlock", ReadRefusal::Rwlock),
        ("pthread_rwlock_tryrdlock", ReadRefusal::Rwlock),
        ("pthread_rwlock_trywrlock", ReadRefusal::Rwlock),
        ("pthread_mutex_trylock", ReadRefusal::TryOrTimedLock),
        ("pthread_mutex_timedlock", ReadRefusal::TryOrTimedLock),
        (
            "pthread_mutexattr_settype",
            ReadRefusal::RecursiveOrDynamicMutex,
        ),
    ],
    refused_tokens: &[
        ("PTHREAD_COND_INITIALIZER", ReadRefusal::StaticInitializer),
        (
            "PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP",
            ReadRefusal::RecursiveOrDynamicMutex,
        ),
        (
            "PTHREAD_MUTEX_RECURSIVE",
            ReadRefusal::RecursiveOrDynamicMutex,
        ),
    ],
};

/// A kept refusal the reader saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadRefusal {
    Broadcast,
    C11Condvar,
    Rwlock,
    TryOrTimedLock,
    StaticInitializer,
    RecursiveOrDynamicMutex,
}

/// Whether a mutex is held at a site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadHeld {
    Yes,
    No,
    Unknown,
}

/// The body shape of a loop around a wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadLoopBody {
    WaitOnly,
    WaitThenBreakOnNonzero,
    Other,
}

/// A fact a condition establishes: the key is truthy (`true`) or zero.
pub type Fact = (String, bool);

/// The loop around a wait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadPredicateLoop {
    /// The keys the loop condition reads.
    pub predicate: BTreeSet<String>,
    /// What the loop's exit establishes (the negated condition), when the
    /// condition is one atom.
    pub exit_fact: Option<Fact>,
    pub under_mutex: bool,
    pub body: ReadLoopBody,
}

/// One wait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadWait {
    pub condvar: Option<String>,
    pub mutex: Option<String>,
    pub timed: bool,
    pub held: ReadHeld,
    pub predicate_loop: Option<ReadPredicateLoop>,
}

/// One event of a signalling critical section, in program order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadSectionEvent {
    Write(String),
    Signal,
}

/// One signal or broadcast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadSignal {
    pub condvar: Option<String>,
    pub broadcast: bool,
    pub held: BTreeSet<String>,
    pub held_unknown: bool,
    /// The enclosing critical section of each held mutex (lock to unlock in
    /// the same block), when it is one.
    pub sections: BTreeMap<String, Vec<ReadSectionEvent>>,
}

/// One `init` or `destroy` of a condvar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadLifecycle {
    pub condvar: Option<String>,
    pub init: bool,
    /// The object is fresh: a local struct, or a local assigned from an
    /// allocator, that no call has received and no spawn has followed.
    pub fresh: bool,
    /// Facts observed under some mutex before the site on every path, not
    /// cleared by a later unlock.
    pub observed_ever: BTreeSet<Fact>,
}

/// One access of a field-keyed object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadAccess {
    pub key: String,
    pub write: bool,
    pub held: BTreeSet<String>,
    pub held_unknown: bool,
    /// Facts observed under each held mutex since it was acquired.
    pub observed: BTreeMap<String, BTreeSet<Fact>>,
    pub observed_ever: BTreeSet<Fact>,
    /// The access is in a wait loop's own condition.
    pub in_wait_condition: bool,
    /// The access is in the fresh phase of its base (see [`ReadLifecycle`]).
    pub fresh: bool,
}

/// What one site is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadSite {
    Wait(ReadWait),
    Signal(ReadSignal),
    Lifecycle(ReadLifecycle),
    Access(ReadAccess),
    Refused(ReadRefusal),
}

/// One site the C4 reader recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncSite {
    pub file: String,
    pub line: usize,
    /// The function's census name (`NAME` or `PATH::NAME`).
    pub function: String,
    pub callee: String,
    pub in_loop: bool,
    pub site: ReadSite,
}

/// The C4 reading of one function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSync {
    pub function: String,
    pub file: String,
    /// Why the body could not be read, if it could not.
    pub unresolved: Option<String>,
}

/// How a live thread was joined at a release, on one path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadJoin {
    CheckedSuccess,
    Unchecked,
    /// The first join of a thread this path started through `wrapper`, its
    /// result ignored (card W47-CONDVAR-JOIN-STOP-PATHS).
    UncheckedFirst {
        wrapper: String,
    },
    /// A join of a thread this path had already joined.
    Repeated,
    Failed,
    /// No join; `true` when a timed wait timed out on the path, and the
    /// second flag when the path awaited a publication.
    Absent {
        timed_out: bool,
        awaited: bool,
    },
}

/// One release the C5 reader recorded, over every path of one root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseReading {
    pub root: String,
    pub file: String,
    pub line: usize,
    pub function: String,
    /// The release call, or `scope end of NAME`.
    pub what: String,
    /// Each live thread's join state, per distinct path state, with the site
    /// of the join as its witness.
    pub live: BTreeMap<String, BTreeSet<(ReadJoin, String)>>,
    pub unresolved: BTreeSet<String>,
    /// Card W47-FLB-LIB-LIFECYCLE: per distinct path state, every context's
    /// lifecycle history (key, events) at the release, the release's own
    /// lifecycle event included. Empty unless lifecycle roles were given.
    pub lifecycle: BTreeSet<Vec<(String, LifeHistory)>>,
    /// Card W47-FLB-LIB-STRICT-PAIRING: per distinct path state, its
    /// histories together with that path's own join evidence and unresolved
    /// reasons, so the release can be decided on the paths that keep the
    /// lifecycle apart from those that do not. Empty unless lifecycle roles
    /// were given.
    pub paths: BTreeSet<ReleasePath>,
}

/// One path state reaching a release (card W47-FLB-LIB-STRICT-PAIRING).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleasePath {
    /// Every keyed context's lifecycle history on the path.
    pub histories: Vec<(String, LifeHistory)>,
    /// Each thread's join state on the path, with its witness.
    pub live: BTreeMap<String, (ReadJoin, String)>,
    pub unresolved: BTreeSet<String>,
}

/// A signaller that signals at most once per thread (derived, see
/// [`at_most_once`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalOnce {
    pub condvar: String,
    /// `Ok(entry)` when derived, `Err(reason)` otherwise.
    pub derived: Result<String, String>,
}

/// A spawned entry run on its own thread to its exit (card
/// W47-CONDVAR-JOIN-STOP-PATHS): the threads its paths started and how each
/// was joined at the exit, per distinct path state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryExit {
    /// The entry's census name.
    pub entry: String,
    pub live: BTreeMap<String, BTreeSet<(ReadJoin, String)>>,
    pub unresolved: BTreeSet<String>,
}

/// The whole reading of one program.
#[derive(Clone, Debug, Default)]
pub struct SyncReading {
    pub census: CensusReading,
    pub functions: Vec<FunctionSync>,
    pub sites: Vec<SyncSite>,
    pub releases: Vec<ReleaseReading>,
    pub signal_once: Vec<SignalOnce>,
    /// The fields a spawned thread identifier is stored in.
    pub thread_fields: BTreeMap<String, BTreeSet<String>>,
    /// The file that defines each census name.
    pub defined_in: BTreeMap<String, String>,
    /// Card W47-CONDVAR-JOIN-STOP-PATHS: every null test of a field in a read
    /// function, with its decider's answer.
    pub null_tests: Vec<NullTestReading>,
    /// Every `pthread_join`/`pthread_detach` and its identity.
    pub thread_sites: Vec<ThreadIdSite>,
    /// Every `pthread_self()`.
    pub self_sites: Vec<SelfSite>,
    /// Every spawn wrapper's joinability.
    pub wrappers: Vec<WrapperReading>,
    /// Every spawned entry run to its exit, by plain entry name.
    pub entry_exits: BTreeMap<String, EntryExit>,
    /// Card W47-FLB-LIB-LIFECYCLE: every call of a declared lifecycle
    /// function on a test root's paths (empty unless roles were given).
    pub lifecycle_calls: Vec<LifecycleCallReading>,
    /// Every declared stop and destroy run from each clause's state.
    pub lifecycle_bodies: Vec<LifecycleBodyReading>,
}

// ---------------------------------------------------------------- scanning

struct Definition {
    file: usize,
    census: String,
    body: (usize, usize),
    /// Parameter names with their `struct` type, when declared one.
    parameters: Vec<(String, Option<String>)>,
}

struct FileScan {
    path: String,
    tokens: Vec<Token>,
    /// Token ranges `[start, end)` inside conditional groups.
    groups: Vec<(usize, usize)>,
}

fn scan(files: &[CensusFile]) -> (Vec<FileScan>, Vec<Definition>) {
    let mut scans = Vec::new();
    let mut definitions = Vec::new();
    for (file, source) in files.iter().enumerate() {
        let (raw, marks) = tokenize_with_conditionals(&source.text);
        // The tokenizer emits one-character punctuators; merge the operators
        // this reader matches (`->`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, `++`,
        // `--` and the compound assignments). `map[i]` is raw token `i`'s index
        // in the merged list.
        let mut tokens: Vec<Token> = Vec::with_capacity(raw.len());
        let mut map = Vec::with_capacity(raw.len() + 1);
        let mut index = 0;
        while index < raw.len() {
            map.push(tokens.len());
            let first = raw[index].text.as_str();
            let second = raw.get(index + 1).map_or("", |t| t.text.as_str());
            let third = raw.get(index + 2).map_or("", |t| t.text.as_str());
            let merged = match (first, second, third) {
                ("<", "<", "=") | (">", ">", "=") => Some(3),
                ("-", ">", _)
                | ("=" | "!" | "<" | ">" | "+" | "-" | "*" | "/" | "|" | "&" | "^", "=", _) => {
                    Some(2)
                }
                ("&", "&", _) | ("|", "|", _) | ("+", "+", _) | ("-", "-", _) => Some(2),
                ("<", "<", _) | (">", ">", _) => Some(2),
                _ => None,
            };
            match merged {
                Some(width) => {
                    let text = raw[index..index + width]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<String>();
                    tokens.push(Token {
                        text,
                        line: raw[index].line,
                    });
                    for _ in 1..width {
                        map.push(tokens.len() - 1);
                    }
                    index += width;
                }
                None => {
                    tokens.push(raw[index].clone());
                    index += 1;
                }
            }
        }
        map.push(tokens.len());
        let at = |raw_index: usize| map.get(raw_index).copied().unwrap_or(tokens.len());
        let mut groups = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        for mark in &marks {
            match mark.directive() {
                ConditionalDirective::Open => open.push(at(mark.token_index())),
                ConditionalDirective::Close => {
                    if let Some(start) = open.pop() {
                        groups.push((start, at(mark.token_index())));
                    }
                }
                _ => {}
            }
        }
        let mut depth = 0usize;
        let mut statement_start = 0usize;
        let mut index = 0;
        while index < tokens.len() {
            let text = tokens[index].text.as_str();
            match text {
                "{" => depth += 1,
                "}" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        statement_start = index + 1;
                    }
                }
                ";" if depth == 0 => statement_start = index + 1,
                _ if depth == 0
                    && is_identifier(text)
                    && !KEYWORDS.contains(&text)
                    && tokens.get(index + 1).is_some_and(|t| t.text == "(") =>
                {
                    if let Some(close) = matching(&tokens, index + 1, "(", ")")
                        && tokens.get(close + 1).is_some_and(|t| t.text == "{")
                        && let Some(end) = matching(&tokens, close + 1, "{", "}")
                    {
                        let is_static = tokens[statement_start..index]
                            .iter()
                            .any(|token| token.text == "static");
                        let census = if is_static {
                            format!("{}::{text}", source.path)
                        } else {
                            text.to_owned()
                        };
                        definitions.push(Definition {
                            file,
                            census,
                            body: (close + 1, end),
                            parameters: parameters(&tokens, index + 1, close),
                        });
                        index = end + 1;
                        statement_start = index;
                        continue;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        scans.push(FileScan {
            path: source.path.clone(),
            tokens,
            groups,
        });
    }
    (scans, definitions)
}

const KEYWORDS: &[&str] = &[
    "if",
    "while",
    "for",
    "switch",
    "return",
    "sizeof",
    "do",
    "else",
    "case",
    "goto",
    "typeof",
    "__typeof__",
    "_Alignof",
    "__attribute__",
    "__builtin_expect",
    "defined",
];

fn parameters(tokens: &[Token], open: usize, close: usize) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut start = open + 1;
    let mut depth = 0usize;
    for index in open + 1..=close {
        let text = tokens[index].text.as_str();
        match text {
            "(" | "[" => depth += 1,
            ")" | "]" if index < close => depth = depth.saturating_sub(1),
            _ => {}
        }
        if (text == "," && depth == 0) || index == close {
            let part = &tokens[start..index];
            let name = part
                .iter()
                .rev()
                .find(|token| is_identifier(&token.text))
                .map(|token| token.text.clone());
            let ty = part
                .windows(2)
                .find(|pair| pair[0].text == "struct" && is_identifier(&pair[1].text))
                .map(|pair| pair[1].text.clone());
            if let Some(name) = name {
                out.push((name, ty));
            }
            start = index + 1;
        }
    }
    out
}

/// `struct T [*...] NAME` declarations in a body (and the parameters).
fn struct_types(tokens: &[Token], definition: &Definition) -> BTreeMap<String, String> {
    let mut types: BTreeMap<String, String> = definition
        .parameters
        .iter()
        .filter_map(|(name, ty)| ty.clone().map(|ty| (name.clone(), ty)))
        .collect();
    let (open, close) = definition.body;
    let mut index = open;
    while index + 2 < close {
        if tokens[index].text == "struct" && is_identifier(&tokens[index + 1].text) {
            let ty = tokens[index + 1].text.clone();
            let mut cursor = index + 2;
            loop {
                while tokens.get(cursor).is_some_and(|t| t.text == "*") {
                    cursor += 1;
                }
                let Some(name) = tokens.get(cursor) else {
                    break;
                };
                if !is_identifier(&name.text) || KEYWORDS.contains(&name.text.as_str()) {
                    break;
                }
                let next = tokens.get(cursor + 1).map_or("", |t| t.text.as_str());
                if !matches!(next, ";" | "=" | "," | "[" | ")") {
                    break;
                }
                types.insert(name.text.clone(), ty.clone());
                // A further declarator of the same declaration.
                let mut skip = cursor + 1;
                let mut depth = 0usize;
                while let Some(token) = tokens.get(skip) {
                    match token.text.as_str() {
                        "(" | "[" | "{" => depth += 1,
                        ")" | "]" | "}" => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        "," if depth == 0 => break,
                        ";" if depth == 0 => break,
                        _ => {}
                    }
                    skip += 1;
                }
                if tokens.get(skip).is_some_and(|t| t.text == ",") && next != ")" {
                    cursor = skip + 1;
                    continue;
                }
                break;
            }
        }
        index += 1;
    }
    types
}

// -------------------------------------------------------------- statements

#[derive(Clone, Debug)]
enum Stmt {
    Block(Vec<Stmt>),
    If {
        cond: (usize, usize),
        then: Box<Stmt>,
        els: Option<Box<Stmt>>,
    },
    /// `while`/`for`: the condition is checked before each iteration.
    Loop {
        cond: Option<(usize, usize)>,
        init: Option<(usize, usize)>,
        step: Option<(usize, usize)>,
        body: Box<Stmt>,
        /// A `do … while` or a macro loop: not a predicate loop.
        plain: bool,
    },
    Return(usize, usize),
    Break,
    Continue,
    Expr(usize, usize),
    /// Maybe executed (inside a conditional-compilation group).
    Maybe(Box<Stmt>),
    Empty,
}

struct Parser<'a> {
    tokens: &'a [Token],
    /// The conditional groups inside the body being read.
    groups: Vec<(usize, usize)>,
    error: Option<String>,
}

impl Parser<'_> {
    fn fail(&mut self, index: usize, why: &str) {
        if self.error.is_none() {
            let line = self.tokens.get(index).map_or(0, |t| t.line);
            self.error = Some(format!("{why} at line {line}"));
        }
    }

    fn block(&mut self, open: usize, close: usize) -> Vec<Stmt> {
        let mut out = Vec::new();
        let mut index = open + 1;
        while index < close && self.error.is_none() {
            let (stmt, next) = self.statement(index, close);
            out.push(stmt);
            if next <= index {
                self.fail(index, "no progress");
                break;
            }
            index = next;
        }
        out
    }

    /// Wrap a statement that a conditional group encloses; refuse one a group
    /// boundary splits.
    fn grouped(&mut self, stmt: Stmt, start: usize, end: usize) -> Stmt {
        let mut maybe = false;
        for (gs, ge) in self.groups.clone() {
            if gs <= start && end < ge {
                maybe = true;
            } else if gs < end + 1 && ge > start && !(start <= gs && ge <= end + 1) {
                self.fail(start, "a conditional group splits a statement");
            } else if start < gs
                && ge <= end + 1
                && matches!(stmt, Stmt::Expr(..) | Stmt::Return(..))
            {
                self.fail(start, "a conditional group inside an expression");
            }
        }
        if maybe {
            Stmt::Maybe(Box::new(stmt))
        } else {
            stmt
        }
    }

    fn statement(&mut self, index: usize, limit: usize) -> (Stmt, usize) {
        let (stmt, end) = self.statement_inner(index, limit);
        let stmt = self.grouped(stmt, index, end.saturating_sub(1));
        (stmt, end)
    }

    fn paren(&mut self, open: usize) -> Option<usize> {
        if self.tokens.get(open).is_none_or(|t| t.text != "(") {
            self.fail(open, "expected `(`");
            return None;
        }
        let close = matching(self.tokens, open, "(", ")");
        if close.is_none() {
            self.fail(open, "unbalanced `(`");
        }
        close
    }

    fn statement_inner(&mut self, index: usize, limit: usize) -> (Stmt, usize) {
        let tokens = self.tokens;
        let text = tokens[index].text.as_str();
        match text {
            "{" => {
                let Some(close) = matching(tokens, index, "{", "}") else {
                    self.fail(index, "unbalanced `{`");
                    return (Stmt::Empty, limit);
                };
                (Stmt::Block(self.block(index, close)), close + 1)
            }
            ";" => (Stmt::Empty, index + 1),
            "if" => {
                let Some(close) = self.paren(index + 1) else {
                    return (Stmt::Empty, limit);
                };
                let (then, next) = self.statement(close + 1, limit);
                if tokens.get(next).is_some_and(|t| t.text == "else") {
                    let (els, after) = self.statement(next + 1, limit);
                    (
                        Stmt::If {
                            cond: (index + 2, close),
                            then: Box::new(then),
                            els: Some(Box::new(els)),
                        },
                        after,
                    )
                } else {
                    (
                        Stmt::If {
                            cond: (index + 2, close),
                            then: Box::new(then),
                            els: None,
                        },
                        next,
                    )
                }
            }
            "while" => {
                let Some(close) = self.paren(index + 1) else {
                    return (Stmt::Empty, limit);
                };
                let (body, next) = self.statement(close + 1, limit);
                (
                    Stmt::Loop {
                        cond: Some((index + 2, close)),
                        init: None,
                        step: None,
                        body: Box::new(body),
                        plain: false,
                    },
                    next,
                )
            }
            "for" => {
                let Some(close) = self.paren(index + 1) else {
                    return (Stmt::Empty, limit);
                };
                let mut parts = Vec::new();
                let mut start = index + 2;
                let mut depth = 0usize;
                for cursor in index + 2..close {
                    match tokens[cursor].text.as_str() {
                        "(" | "[" | "{" => depth += 1,
                        ")" | "]" | "}" => depth = depth.saturating_sub(1),
                        ";" if depth == 0 => {
                            parts.push((start, cursor));
                            start = cursor + 1;
                        }
                        _ => {}
                    }
                }
                parts.push((start, close));
                if parts.len() != 3 {
                    self.fail(index, "a `for` header without two `;`");
                    return (Stmt::Empty, limit);
                }
                let (body, next) = self.statement(close + 1, limit);
                let nonempty = |(s, e): (usize, usize)| (s < e).then_some((s, e));
                (
                    Stmt::Loop {
                        cond: nonempty(parts[1]),
                        init: nonempty(parts[0]),
                        step: nonempty(parts[2]),
                        body: Box::new(body),
                        plain: false,
                    },
                    next,
                )
            }
            "do" => {
                let (body, next) = self.statement(index + 1, limit);
                if tokens.get(next).is_none_or(|t| t.text != "while") {
                    self.fail(index, "a `do` without `while`");
                    return (Stmt::Empty, limit);
                }
                let Some(close) = self.paren(next + 1) else {
                    return (Stmt::Empty, limit);
                };
                (
                    Stmt::Loop {
                        cond: Some((next + 2, close)),
                        init: None,
                        step: None,
                        body: Box::new(body),
                        plain: true,
                    },
                    close + 2,
                )
            }
            "return" => {
                let end = expression_end(tokens, index + 1, limit);
                (Stmt::Return(index + 1, end), end + 1)
            }
            "break" => (Stmt::Break, index + 2),
            "continue" => (Stmt::Continue, index + 2),
            "switch" | "goto" | "case" | "default" | "asm" | "__asm__" => {
                self.fail(index, &format!("`{text}` is outside the statement reader"));
                (Stmt::Empty, limit)
            }
            _ if is_identifier(text)
                && tokens.get(index + 1).is_some_and(|t| t.text == ":")
                && tokens.get(index + 2).is_none_or(|t| t.text != ":") =>
            {
                self.fail(index, "a label is outside the statement reader");
                (Stmt::Empty, limit)
            }
            _ if is_identifier(text)
                && tokens.get(index + 1).is_some_and(|t| t.text == "(")
                && let Some(close) = matching(tokens, index + 1, "(", ")")
                && tokens.get(close + 1).is_some_and(|t| t.text == "{") =>
            {
                // A macro loop: `mk_list_foreach(head, list) { … }`.
                let (body, next) = self.statement(close + 1, limit);
                (
                    Stmt::Loop {
                        cond: None,
                        init: Some((index, close + 1)),
                        step: None,
                        body: Box::new(body),
                        plain: true,
                    },
                    next,
                )
            }
            _ => {
                let end = expression_end(tokens, index, limit);
                (Stmt::Expr(index, end), end + 1)
            }
        }
    }
}

fn expression_end(tokens: &[Token], start: usize, limit: usize) -> usize {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().take(limit).skip(start) {
        match token.text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            ";" if depth == 0 => return index,
            _ => {}
        }
    }
    limit
}

/// Calls in a token range, outermost first by position: `(name index, open,
/// close)`.
fn calls_in(tokens: &[Token], start: usize, end: usize) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for index in start..end {
        let text = tokens[index].text.as_str();
        if is_identifier(text)
            && !KEYWORDS.contains(&text)
            && tokens.get(index + 1).is_some_and(|t| t.text == "(")
            && (index == 0 || !matches!(tokens[index - 1].text.as_str(), "." | "->"))
            && let Some(close) = matching(tokens, index + 1, "(", ")")
            && close < end
        {
            out.push((index, index + 1, close));
        }
    }
    out
}

fn arguments(tokens: &[Token], open: usize, close: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for index in open + 1..close {
        match tokens[index].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            "," if depth == 0 => {
                ranges.push((start, index));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < close || !ranges.is_empty() {
        ranges.push((start, close));
    }
    ranges
}

fn texts(tokens: &[Token], range: (usize, usize)) -> Vec<&str> {
    tokens[range.0..range.1]
        .iter()
        .map(|token| token.text.as_str())
        .collect()
}

/// The field key of `[&] BASE . FIELD`, `[&] BASE -> FIELD` (optionally
/// indexed), with parentheses stripped; `None` for anything else.
fn object_key(
    tokens: &[Token],
    range: (usize, usize),
    types: &BTreeMap<String, String>,
) -> Option<String> {
    let mut words = texts(tokens, range);
    while words.first() == Some(&"(") && words.last() == Some(&")") {
        words = words[1..words.len() - 1].to_vec();
    }
    if words.first() == Some(&"&") {
        words.remove(0);
    }
    match words.as_slice() {
        [base, "." | "->", field, rest @ ..]
            if is_identifier(base)
                && is_identifier(field)
                && (rest.is_empty()
                    || (rest.first() == Some(&"[")
                        && rest.last() == Some(&"]")
                        && rest.iter().filter(|w| **w == "[").count() == 1)) =>
        {
            // One trailing index is the array member itself; a member of an
            // element (`a->v[i].m`) is not this key.
            types.get(*base).map(|ty| format!("{ty}.{field}"))
        }
        _ => None,
    }
}

/// The plain identifier at the root of `[&] NAME …`.
fn root_name(tokens: &[Token], range: (usize, usize)) -> Option<String> {
    let words = texts(tokens, range);
    let first = words
        .iter()
        .find(|word| !matches!(**word, "&" | "(" | "*"))?;
    is_identifier(first).then(|| (*first).to_owned())
}

/// Strip truth macros and one negation: `(negated, inner range)`.
fn condition_atom(
    tokens: &[Token],
    range: (usize, usize),
    tables: &SyncTables,
) -> (bool, (usize, usize)) {
    let (mut start, mut end) = range;
    let mut negated = false;
    loop {
        while start < end
            && tokens[start].text == "("
            && matching(tokens, start, "(", ")") == Some(end - 1)
        {
            start += 1;
            end -= 1;
        }
        if start < end && tokens[start].text == "!" {
            negated = !negated;
            start += 1;
            continue;
        }
        if end > start + 2
            && tables.truth_macros.contains(&tokens[start].text.as_str())
            && tokens[start + 1].text == "("
            && matching(tokens, start + 1, "(", ")") == Some(end - 1)
        {
            start += 2;
            end -= 1;
            continue;
        }
        break;
    }
    (negated, (start, end))
}

/// The fact a condition establishes when it holds: `KEY` is truthy/zero.
fn condition_fact(
    tokens: &[Token],
    range: (usize, usize),
    types: &BTreeMap<String, String>,
    tables: &SyncTables,
) -> Option<Fact> {
    let (negated, inner) = condition_atom(tokens, range, tables);
    let words = texts(tokens, inner);
    let (key_range, truthy) = match words.as_slice() {
        [.., "==", "0"] => ((inner.0, inner.1 - 2), false),
        [.., "!=", "0"] => ((inner.0, inner.1 - 2), true),
        _ => (inner, true),
    };
    let key = object_key(tokens, key_range, types)?;
    Some((key, truthy != negated))
}

// ------------------------------------------------------------ C4 dataflow

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LockState {
    must: BTreeSet<String>,
    may: BTreeSet<String>,
    observed: BTreeMap<String, BTreeSet<Fact>>,
    observed_ever: BTreeSet<Fact>,
    unknown_lock: bool,
    /// Fresh local bases (no call has received them, no spawn since).
    fresh: BTreeSet<String>,
}

fn join_state(left: Option<LockState>, right: Option<LockState>) -> Option<LockState> {
    match (left, right) {
        (None, x) | (x, None) => x,
        (Some(a), Some(b)) => {
            let mut observed = BTreeMap::new();
            for (mutex, facts) in &a.observed {
                if let Some(other) = b.observed.get(mutex) {
                    observed.insert(mutex.clone(), facts.intersection(other).cloned().collect());
                }
            }
            Some(LockState {
                must: a.must.intersection(&b.must).cloned().collect(),
                may: a.may.union(&b.may).cloned().collect(),
                observed,
                observed_ever: a
                    .observed_ever
                    .intersection(&b.observed_ever)
                    .cloned()
                    .collect(),
                unknown_lock: a.unknown_lock || b.unknown_lock,
                fresh: a.fresh.intersection(&b.fresh).cloned().collect(),
            })
        }
    }
}

struct Flow {
    normal: Option<LockState>,
    breaks: Option<LockState>,
    continues: Option<LockState>,
}

struct C4<'a> {
    tokens: &'a [Token],
    types: BTreeMap<String, String>,
    tables: &'a SyncTables,
    path: &'a str,
    function: &'a str,
    sites: Vec<SyncSite>,
    loop_depth: usize,
    /// Token ranges of the current wait loop condition.
    wait_condition: Option<(usize, usize)>,
}

impl C4<'_> {
    fn record(&mut self, index: usize, callee: &str, site: ReadSite) {
        self.sites.push(SyncSite {
            file: self.path.to_owned(),
            line: self.tokens[index].line,
            function: self.function.to_owned(),
            callee: callee.to_owned(),
            in_loop: self.loop_depth > 0,
            site,
        });
    }

    fn held(&self, state: &LockState, key: &Option<String>) -> ReadHeld {
        match key {
            Some(key) if state.must.contains(key) => ReadHeld::Yes,
            Some(key) if !state.may.contains(key) && !state.unknown_lock => ReadHeld::No,
            _ => ReadHeld::Unknown,
        }
    }

    /// Accesses and calls of one expression range, in token order.
    fn expression(&mut self, range: (usize, usize), state: &mut LockState) {
        let tokens = self.tokens;
        let calls = calls_in(tokens, range.0, range.1);
        // Field accesses.
        for index in range.0..range.1 {
            let text = tokens[index].text.as_str();
            if !is_identifier(text)
                || !self.types.contains_key(text)
                || tokens
                    .get(index + 1)
                    .is_none_or(|t| !matches!(t.text.as_str(), "." | "->"))
                || (index > 0 && matches!(tokens[index - 1].text.as_str(), "." | "->" | "&"))
            {
                continue;
            }
            let Some(field) = tokens.get(index + 2).filter(|t| is_identifier(&t.text)) else {
                continue;
            };
            let key = format!("{}.{}", self.types[text], field.text);
            let mut after = index + 3;
            while tokens.get(after).is_some_and(|t| t.text == "[") {
                after = matching(tokens, after, "[", "]").map_or(after + 1, |close| close + 1);
            }
            let write = tokens.get(after).is_some_and(|t| {
                matches!(
                    t.text.as_str(),
                    "=" | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "|="
                        | "&="
                        | "^="
                        | "<<="
                        | ">>="
                        | "++"
                        | "--"
                )
            });
            let in_wait_condition = self
                .wait_condition
                .is_some_and(|(s, e)| s <= index && index < e);
            let access = ReadAccess {
                key,
                write,
                held: state.must.clone(),
                held_unknown: state.unknown_lock || state.may != state.must,
                observed: state.observed.clone(),
                observed_ever: state.observed_ever.clone(),
                in_wait_condition,
                fresh: state.fresh.contains(text),
            };
            self.record(index, text, ReadSite::Access(access));
        }
        for (name_index, open, close) in calls {
            let name = tokens[name_index].text.clone();
            let args = arguments(tokens, open, close);
            let key = |n: usize| {
                args.get(n)
                    .and_then(|r| object_key(tokens, *r, &self.types))
            };
            let t = self.tables;
            if name == t.lock {
                let key = key(0);
                match &key {
                    Some(key) => {
                        state.must.insert(key.clone());
                        state.may.insert(key.clone());
                        state.observed.insert(key.clone(), BTreeSet::new());
                    }
                    None => state.unknown_lock = true,
                }
            } else if name == t.unlock {
                match key(0) {
                    Some(key) => {
                        state.must.remove(&key);
                        state.may.remove(&key);
                        state.observed.remove(&key);
                    }
                    None => state.unknown_lock = true,
                }
            } else if name == t.wait || name == t.timed_wait {
                let condvar = key(0);
                let mutex = key(1);
                let held = self.held(state, &mutex);
                self.record(
                    name_index,
                    &name,
                    ReadSite::Wait(ReadWait {
                        condvar,
                        mutex: mutex.clone(),
                        timed: name == t.timed_wait,
                        held,
                        predicate_loop: None,
                    }),
                );
                // The wait released the mutex: the predicate must be rechecked.
                if let Some(mutex) = mutex {
                    state.observed.insert(mutex, BTreeSet::new());
                }
            } else if name == t.signal || name == t.broadcast {
                let condvar = key(0);
                self.record(
                    name_index,
                    &name,
                    ReadSite::Signal(ReadSignal {
                        condvar,
                        broadcast: name == t.broadcast,
                        held: state.must.clone(),
                        held_unknown: state.unknown_lock || state.may != state.must,
                        sections: BTreeMap::new(),
                    }),
                );
            } else if name == t.cond_init || name == t.cond_destroy {
                let condvar = key(0);
                let fresh = args
                    .first()
                    .and_then(|r| root_name(tokens, *r))
                    .is_some_and(|root| state.fresh.contains(&root));
                self.record(
                    name_index,
                    &name,
                    ReadSite::Lifecycle(ReadLifecycle {
                        condvar,
                        init: name == t.cond_init,
                        fresh,
                        observed_ever: state.observed_ever.clone(),
                    }),
                );
            } else if let Some((_, refusal)) = t.refused.iter().find(|(n, _)| *n == name) {
                self.record(name_index, &name, ReadSite::Refused(*refusal));
            } else if name != t.mutex_destroy && name != "pthread_mutex_init" {
                // Any other call may publish what it receives. A thread start
                // alone does not: no other thread can reach an object whose
                // address has not escaped.
                for range in &args {
                    if let Some(root) = root_name(tokens, *range) {
                        state.fresh.remove(&root);
                    }
                }
            }
        }
        // A fresh base: `NAME = ALLOCATOR(…)`.
        let words = texts(tokens, range);
        if let [name, "=", call, "(", ..] = words.as_slice()
            && self.types.contains_key(*name)
            && self.tables.allocators.contains(call)
        {
            state.fresh.insert((*name).to_owned());
        }
        // A base stored anywhere is published.
        if let Some(position) = words.iter().position(|w| *w == "=")
            && position > 0
        {
            for word in &words[position + 1..] {
                if self.types.contains_key(*word) {
                    state.fresh.remove(*word);
                }
            }
        }
        for (token, refusal) in self.tables.refused_tokens {
            if let Some(index) = (range.0..range.1).find(|i| tokens[*i].text == *token) {
                self.record(index, token, ReadSite::Refused(*refusal));
            }
        }
    }

    fn statements(&mut self, list: &[Stmt], state: Option<LockState>) -> Flow {
        let mut flow = Flow {
            normal: state,
            breaks: None,
            continues: None,
        };
        let mut index = 0;
        while index < list.len() {
            let inner = self.statement(&list[index], flow.normal.take());
            flow.normal = inner.normal;
            flow.breaks = join_state(flow.breaks, inner.breaks);
            flow.continues = join_state(flow.continues, inner.continues);
            index += 1;
        }
        flow
    }

    fn statement(&mut self, stmt: &Stmt, state: Option<LockState>) -> Flow {
        let none = |normal| Flow {
            normal,
            breaks: None,
            continues: None,
        };
        let Some(mut current) = state else {
            // Unreachable: still read the sites with an empty state so that
            // nothing is silently dropped.
            let probe = self.statement(
                stmt,
                Some(LockState {
                    unknown_lock: true,
                    ..LockState::default()
                }),
            );
            let _ = probe;
            return none(None);
        };
        match stmt {
            Stmt::Empty => none(Some(current)),
            Stmt::Expr(start, end) => {
                self.expression((*start, *end), &mut current);
                none(Some(current))
            }
            Stmt::Return(start, end) => {
                self.expression((*start, *end), &mut current);
                none(None)
            }
            Stmt::Break => Flow {
                normal: None,
                breaks: Some(current),
                continues: None,
            },
            Stmt::Continue => Flow {
                normal: None,
                breaks: None,
                continues: Some(current),
            },
            Stmt::Block(list) => self.statements(list, Some(current)),
            Stmt::Maybe(inner) => {
                let flow = self.statement(inner, Some(current.clone()));
                Flow {
                    normal: join_state(flow.normal, Some(current)),
                    breaks: flow.breaks,
                    continues: flow.continues,
                }
            }
            Stmt::If { cond, then, els } => {
                self.expression(*cond, &mut current);
                let fact = condition_fact(self.tokens, *cond, &self.types, self.tables);
                let mut then_state = current.clone();
                let mut else_state = current.clone();
                if let Some((key, truthy)) = fact {
                    for mutex in current.must.iter() {
                        then_state
                            .observed
                            .entry(mutex.clone())
                            .or_default()
                            .insert((key.clone(), truthy));
                        else_state
                            .observed
                            .entry(mutex.clone())
                            .or_default()
                            .insert((key.clone(), !truthy));
                    }
                    if !current.must.is_empty() {
                        then_state.observed_ever.insert((key.clone(), truthy));
                        else_state.observed_ever.insert((key, !truthy));
                    }
                }
                let a = self.statement(then, Some(then_state));
                let b = match els {
                    Some(els) => self.statement(els, Some(else_state)),
                    None => none(Some(else_state)),
                };
                Flow {
                    normal: join_state(a.normal, b.normal),
                    breaks: join_state(a.breaks, b.breaks),
                    continues: join_state(a.continues, b.continues),
                }
            }
            Stmt::Loop {
                cond,
                init,
                step,
                body,
                plain,
            } => {
                if let Some(init) = init {
                    self.expression(*init, &mut current);
                }
                if let Some(shape) = self.wait_loop(stmt) {
                    return self.read_wait_loop(stmt, shape, current);
                }
                let _ = plain;
                // Iterate to a fixed point over the must/may sets.
                let mut entry = current.clone();
                let mut exit = None;
                let sites_before = self.sites.len();
                for _ in 0..6 {
                    self.sites.truncate(sites_before);
                    let mut head = entry.clone();
                    if let Some(cond) = cond {
                        self.expression(*cond, &mut head);
                    }
                    self.loop_depth += 1;
                    let flow = self.statement(body, Some(head.clone()));
                    self.loop_depth -= 1;
                    let mut back = join_state(flow.normal, flow.continues);
                    if let (Some(step), Some(state)) = (step, back.as_mut()) {
                        self.expression(*step, state);
                    }
                    let next = join_state(Some(current.clone()), back).unwrap_or_default();
                    exit = join_state(Some(head), flow.breaks);
                    if next == entry {
                        break;
                    }
                    entry = next;
                }
                none(exit)
            }
        }
    }

    /// The wait loop shape: a `while`/`for` whose body is a wait (optionally
    /// assigned) and at most one `if (RET) break;`.
    fn wait_loop(&self, stmt: &Stmt) -> Option<(usize, ReadLoopBody)> {
        let Stmt::Loop {
            body,
            plain: false,
            cond: Some(_),
            ..
        } = stmt
        else {
            return None;
        };
        let list: Vec<&Stmt> = match body.as_ref() {
            Stmt::Block(list) => list.iter().filter(|s| !matches!(s, Stmt::Empty)).collect(),
            other => vec![other],
        };
        let tokens = self.tokens;
        let wait_in = |range: (usize, usize)| {
            calls_in(tokens, range.0, range.1)
                .into_iter()
                .find(|(n, _, _)| {
                    let name = tokens[*n].text.as_str();
                    name == self.tables.wait || name == self.tables.timed_wait
                })
        };
        // Any wait inside the body at all?
        let mut any = false;
        visit_exprs(body, &mut |(s, e)| {
            if wait_in((s, e)).is_some() {
                any = true;
            }
        });
        if !any {
            return None;
        }
        match list.as_slice() {
            [Stmt::Expr(s, e)] if wait_in((*s, *e)).is_some() => {
                Some((wait_in((*s, *e))?.0, ReadLoopBody::WaitOnly))
            }
            [
                Stmt::Expr(s, e),
                Stmt::If {
                    cond,
                    then,
                    els: None,
                },
            ] => {
                let wait = wait_in((*s, *e))?;
                let words = texts(tokens, (*s, *e));
                let result = match words.as_slice() {
                    [name, "=", ..] if is_identifier(name) => (*name).to_owned(),
                    _ => return Some((wait.0, ReadLoopBody::Other)),
                };
                let cond_words = texts(tokens, *cond);
                let breaks_on_nonzero = matches!(
                    cond_words.as_slice(),
                    [name, "!=", "0"] | [name] if *name == result
                );
                let then_breaks = match then.as_ref() {
                    Stmt::Break => true,
                    Stmt::Block(inner) => matches!(inner.as_slice(), [Stmt::Break]),
                    _ => false,
                };
                Some((
                    wait.0,
                    if breaks_on_nonzero && then_breaks {
                        ReadLoopBody::WaitThenBreakOnNonzero
                    } else {
                        ReadLoopBody::Other
                    },
                ))
            }
            _ => {
                let mut first = None;
                visit_exprs(body, &mut |(s, e)| {
                    if first.is_none() {
                        first = wait_in((s, e)).map(|w| w.0);
                    }
                });
                first.map(|w| (w, ReadLoopBody::Other))
            }
        }
    }

    fn read_wait_loop(
        &mut self,
        stmt: &Stmt,
        (wait_index, body_shape): (usize, ReadLoopBody),
        current: LockState,
    ) -> Flow {
        let Stmt::Loop {
            cond: Some(cond),
            body,
            ..
        } = stmt
        else {
            unreachable!("wait_loop matched a conditioned loop");
        };
        let mut head = current.clone();
        self.wait_condition = Some(*cond);
        self.expression(*cond, &mut head);
        self.wait_condition = None;
        let predicate: BTreeSet<String> = {
            let mut keys = BTreeSet::new();
            for index in cond.0..cond.1 {
                if let Some(ty) = self.types.get(&self.tokens[index].text)
                    && self
                        .tokens
                        .get(index + 1)
                        .is_some_and(|t| matches!(t.text.as_str(), "." | "->"))
                    && let Some(field) = self.tokens.get(index + 2)
                {
                    keys.insert(format!("{ty}.{}", field.text));
                }
            }
            keys
        };
        let continue_fact = condition_fact(self.tokens, *cond, &self.types, self.tables);
        let exit_fact = continue_fact.map(|(key, truthy)| (key, !truthy));
        let sites_before = self.sites.len();
        self.loop_depth += 1;
        let flow = self.statement(body, Some(head.clone()));
        self.loop_depth -= 1;
        // The wait's mutex, from the recorded wait site.
        let mutex = self.sites[sites_before..]
            .iter()
            .find_map(|site| match &site.site {
                ReadSite::Wait(wait) if self.tokens[wait_index].line == site.line => {
                    wait.mutex.clone()
                }
                _ => None,
            });
        let under_mutex = mutex.as_ref().is_some_and(|m| {
            current.must.contains(m) && flow.normal.as_ref().is_some_and(|s| s.must.contains(m))
        });
        for site in &mut self.sites[sites_before..] {
            if let ReadSite::Wait(wait) = &mut site.site
                && site.line == self.tokens[wait_index].line
            {
                wait.predicate_loop = Some(ReadPredicateLoop {
                    predicate: predicate.clone(),
                    exit_fact: exit_fact.clone(),
                    under_mutex,
                    body: body_shape,
                });
            }
        }
        // The normal exit establishes the exit fact under the mutex; a break
        // establishes nothing.
        let mut exit = join_state(Some(head), flow.normal);
        if let (Some(state), Some(fact), Some(mutex)) = (exit.as_mut(), &exit_fact, &mutex)
            && state.must.contains(mutex)
        {
            state
                .observed
                .entry(mutex.clone())
                .or_default()
                .insert(fact.clone());
            state.observed_ever.insert(fact.clone());
        }
        Flow {
            normal: join_state(exit, flow.breaks),
            breaks: None,
            continues: None,
        }
    }
}

fn visit_exprs(stmt: &Stmt, visit: &mut impl FnMut((usize, usize))) {
    match stmt {
        Stmt::Expr(s, e) | Stmt::Return(s, e) => visit((*s, *e)),
        Stmt::Block(list) => list.iter().for_each(|s| visit_exprs(s, visit)),
        Stmt::If { cond, then, els } => {
            visit(*cond);
            visit_exprs(then, visit);
            if let Some(els) = els {
                visit_exprs(els, visit);
            }
        }
        Stmt::Loop {
            cond,
            init,
            step,
            body,
            ..
        } => {
            for range in [init, cond, step].into_iter().flatten() {
                visit(*range);
            }
            visit_exprs(body, visit);
        }
        Stmt::Maybe(inner) => visit_exprs(inner, visit),
        Stmt::Break | Stmt::Continue | Stmt::Empty => {}
    }
}

/// The critical sections around each signal: in the block that holds the
/// signal statement (at any depth), the nearest `lock(M)` before it and
/// `unlock(M)` after it, with every write and signal between in token order.
fn sections(
    tokens: &[Token],
    body: (usize, usize),
    signal: usize,
    types: &BTreeMap<String, String>,
    tables: &SyncTables,
) -> BTreeMap<String, Vec<ReadSectionEvent>> {
    let mut out = BTreeMap::new();
    let calls = calls_in(tokens, body.0, body.1);
    let key_of = |open: usize, close: usize| {
        arguments(tokens, open, close)
            .first()
            .and_then(|r| object_key(tokens, *r, types))
    };
    for (lock_index, open, close) in calls.iter().copied() {
        if tokens[lock_index].text != tables.lock || lock_index > signal {
            continue;
        }
        let Some(mutex) = key_of(open, close) else {
            continue;
        };
        let Some((unlock_index, _, _)) = calls.iter().copied().find(|(n, o, c)| {
            *n > signal
                && tokens[*n].text == tables.unlock
                && key_of(*o, *c).as_ref() == Some(&mutex)
        }) else {
            continue;
        };
        // No other lock or unlock of M between.
        if calls.iter().any(|(n, o, c)| {
            *n > lock_index
                && *n < unlock_index
                && (tokens[*n].text == tables.lock || tokens[*n].text == tables.unlock)
                && key_of(*o, *c).as_ref() == Some(&mutex)
        }) {
            continue;
        }
        let mut events = Vec::new();
        for index in lock_index..unlock_index {
            let text = tokens[index].text.as_str();
            if text == tables.signal && tokens.get(index + 1).is_some_and(|t| t.text == "(") {
                events.push(ReadSectionEvent::Signal);
                continue;
            }
            if let Some(ty) = types.get(text)
                && tokens
                    .get(index + 1)
                    .is_some_and(|t| matches!(t.text.as_str(), "." | "->"))
                && (index == 0 || !matches!(tokens[index - 1].text.as_str(), "." | "->" | "&"))
                && let Some(field) = tokens.get(index + 2)
            {
                let mut after = index + 3;
                while tokens.get(after).is_some_and(|t| t.text == "[") {
                    after = matching(tokens, after, "[", "]").map_or(after + 1, |c| c + 1);
                }
                if tokens.get(after).is_some_and(|t| {
                    matches!(
                        t.text.as_str(),
                        "=" | "+=" | "-=" | "|=" | "&=" | "++" | "--"
                    )
                }) {
                    events.push(ReadSectionEvent::Write(format!("{ty}.{}", field.text)));
                }
            }
        }
        out.insert(mutex, events);
    }
    out
}

// ------------------------------------------------------------- C5 paths

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Joined {
    /// Started and not joined; the note says where a joining function
    /// returned without joining it.
    Live(Option<String>),
    /// A join whose result is held in a local and not yet checked.
    Pending(String),
    Done(ReadJoin, String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Val {
    Const(i64),
    /// The result of joining this thread (at this site).
    JoinResult(String, String),
    /// A thread identifier naming these entries.
    ThreadId(BTreeSet<String>),
    /// The result of a timed wait.
    TimedWait,
    /// Known nonzero (a failed join's result).
    NonZero,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Global {
    threads: BTreeMap<String, Joined>,
    /// The spawn wrapper that started each thread on this path.
    spawned_by: BTreeMap<String, String>,
    unresolved: BTreeSet<String>,
    timed_out: bool,
    awaited: bool,
    /// Card W47-FLB-LIB-LIFECYCLE: each keyed context's lifecycle history on
    /// this path (empty unless lifecycle roles were given).
    histories: BTreeMap<String, LifeHistory>,
    /// Every join executed on this path, as classified (only while a
    /// declared body is run for a clause check).
    trace: Vec<(ReadTraceJoin, String)>,
}

type Env = BTreeMap<String, Val>;
type PState = (Global, Env);

#[derive(Default)]
struct PFlow {
    normal: BTreeSet<PState>,
    breaks: BTreeSet<PState>,
    continues: BTreeSet<PState>,
    returns: BTreeSet<(Global, Option<Val>, String)>,
}

const PATH_BUDGET: usize = 4096;

/// Statement-state steps one spawned entry's run may take (card
/// W47-CONDVAR-JOIN-STOP-PATHS). A run that exhausts it leaves every state
/// from there on unresolved: the budget bounds the reading, never a verdict.
pub const ENTRY_STEP_BUDGET: usize = 200_000;

/// The reason an entry run past its step budget carries.
pub const ENTRY_BUDGET_EXHAUSTED: &str =
    "the path reader's step budget for this entry's run was exhausted";

struct C5<'a> {
    scans: &'a [FileScan],
    definitions: &'a [Definition],
    trees: &'a BTreeMap<usize, Result<Vec<Stmt>, String>>,
    by_census: &'a BTreeMap<String, usize>,
    tables: &'a SyncTables,
    relevant: &'a BTreeSet<usize>,
    joins: &'a BTreeSet<usize>,
    may_thread: &'a BTreeMap<usize, String>,
    thread_fields: &'a BTreeMap<String, BTreeSet<String>>,
    /// Card W47-CONDVAR-JOIN-STOP-PATHS: `(file, condition start)` of every
    /// null test whose null arm the decider excluded, with whether the
    /// condition holds exactly when the field is null.
    excluded: &'a BTreeMap<(usize, usize), bool>,
    root: String,
    root_definition: usize,
    /// The struct types of the running frame's names.
    types: BTreeMap<String, String>,
    stack: Vec<usize>,
    published: BTreeSet<String>,
    releases: BTreeMap<(String, usize, String, String), ReleaseReading>,
    /// Remaining statement-state steps of a spawned entry's run; `None` for a
    /// test root (unbounded, as before).
    budget: Option<usize>,
    /// Card W47-FLB-LIB-LIFECYCLE: the declared lifecycle roles, tracked on a
    /// test root's paths only.
    roles: Option<&'a LifecycleRoles>,
    /// Record every executed join in the path's trace (a clause check).
    trace: bool,
    lifecycle_calls: BTreeMap<(String, usize, String), LifecycleCallReading>,
}

impl C5<'_> {
    fn resolve(&self, file: usize, name: &str) -> Option<usize> {
        let path = &self.scans[file].path;
        self.by_census
            .get(&format!("{path}::{name}"))
            .or_else(|| self.by_census.get(name))
            .copied()
    }

    fn release(
        &mut self,
        definition: usize,
        index: usize,
        what: String,
        states: &BTreeSet<PState>,
    ) {
        let def = &self.definitions[definition];
        let scan = &self.scans[def.file];
        let key = (
            scan.path.clone(),
            scan.tokens[index].line,
            def.census.clone(),
            what.clone(),
        );
        let entry = self.releases.entry(key).or_insert_with(|| ReleaseReading {
            root: self.root.clone(),
            file: scan.path.clone(),
            line: scan.tokens[index].line,
            function: def.census.clone(),
            what,
            live: BTreeMap::new(),
            unresolved: BTreeSet::new(),
            lifecycle: BTreeSet::new(),
            paths: BTreeSet::new(),
        });
        let per_path = self.roles.is_some();
        for (global, _) in states {
            entry.unresolved.extend(global.unresolved.iter().cloned());
            if !global.histories.is_empty() {
                entry.lifecycle.insert(
                    global
                        .histories
                        .iter()
                        .map(|(key, history)| (key.clone(), history.clone()))
                        .collect(),
                );
            }
            let mut path_live = BTreeMap::new();
            for (thread, joined) in &global.threads {
                let evidence = match joined {
                    Joined::Live(note) => (
                        ReadJoin::Absent {
                            timed_out: global.timed_out,
                            awaited: global.awaited,
                        },
                        note.clone()
                            .unwrap_or_else(|| "no join on this path".to_owned()),
                    ),
                    Joined::Pending(site) => (ReadJoin::Unchecked, site.clone()),
                    Joined::Done(join, site) => (join.clone(), site.clone()),
                };
                if per_path {
                    path_live.insert(thread.clone(), evidence.clone());
                }
                entry
                    .live
                    .entry(thread.clone())
                    .or_default()
                    .insert(evidence);
            }
            if per_path {
                entry.paths.insert(ReleasePath {
                    histories: global
                        .histories
                        .iter()
                        .map(|(key, history)| (key.clone(), history.clone()))
                        .collect(),
                    live: path_live,
                    unresolved: global.unresolved.clone(),
                });
            }
        }
    }

    fn site(&self, definition: usize, index: usize) -> String {
        let def = &self.definitions[definition];
        format!(
            "{}:{}",
            self.scans[def.file].path, self.scans[def.file].tokens[index].line
        )
    }

    fn cap(&self, states: BTreeSet<PState>) -> BTreeSet<PState> {
        if states.len() <= PATH_BUDGET {
            return states;
        }
        states
            .into_iter()
            .take(PATH_BUDGET)
            .map(|(mut g, e)| {
                g.unresolved
                    .insert("the path budget was exceeded".to_owned());
                (g, e)
            })
            .collect()
    }

    /// Run a root: a `return` records the scope end where it stands, the
    /// closing brace for every path that falls off the end.
    fn run_root(&mut self, root: usize) {
        let Some(Ok(tree)) = self.trees.get(&root) else {
            let _ = self.run_function(root, Global::default());
            return;
        };
        let tree = tree.clone();
        let def = &self.definitions[root];
        self.types = struct_types(&self.scans[def.file].tokens, def);
        self.stack.push(root);
        let flow = self.block(
            root,
            &tree,
            BTreeSet::from([(Global::default(), Env::new())]),
        );
        self.stack.pop();
        let close = self.definitions[root].body.1;
        for (global, _) in &flow.normal {
            self.scope_end(root, close, global);
        }
    }

    fn run_function(
        &mut self,
        definition: usize,
        global: Global,
    ) -> BTreeSet<(Global, Option<Val>)> {
        let Some(Ok(tree)) = self.trees.get(&definition) else {
            let mut g = global;
            let reason = match self.trees.get(&definition) {
                Some(Err(reason)) => reason.clone(),
                _ => "no body".to_owned(),
            };
            g.unresolved.insert(format!(
                "`{}` is not read: {reason}",
                self.definitions[definition].census
            ));
            return BTreeSet::from([(g, None)]);
        };
        if self.stack.contains(&definition) || self.stack.len() > 8 {
            let mut g = global;
            g.unresolved.insert(format!(
                "`{}` recurses or nests too deeply for the path reader",
                self.definitions[definition].census
            ));
            return BTreeSet::from([(g, None)]);
        }
        self.stack.push(definition);
        let tree = tree.clone();
        let def = &self.definitions[definition];
        let saved = std::mem::replace(
            &mut self.types,
            struct_types(&self.scans[def.file].tokens, def),
        );
        let joins_here = self.joins.contains(&definition);
        let live_on_entry: BTreeSet<String> = global
            .threads
            .iter()
            .filter(|(_, joined)| matches!(joined, Joined::Live(_)))
            .map(|(thread, _)| thread.clone())
            .collect();
        let flow = self.block(definition, &tree, BTreeSet::from([(global, Env::new())]));
        self.types = saved;
        self.stack.pop();
        let mut out = BTreeSet::new();
        let note = |g: Global, site: &str| -> Global {
            if !joins_here {
                return g;
            }
            let mut g = g;
            for (thread, joined) in &mut g.threads {
                if live_on_entry.contains(thread) && matches!(joined, Joined::Live(_)) {
                    *joined = Joined::Live(Some(format!(
                        "returned from `{}` at {site} without joining",
                        self.definitions[definition].census
                    )));
                }
            }
            g
        };
        for (g, value, site) in flow.returns {
            out.insert((note(g, &site), value));
        }
        let close = self.site(definition, self.definitions[definition].body.1);
        for (g, _) in flow.normal {
            out.insert((note(g, &close), None));
        }
        out
    }

    fn block(&mut self, definition: usize, list: &[Stmt], states: BTreeSet<PState>) -> PFlow {
        let mut flow = PFlow {
            normal: states,
            ..PFlow::default()
        };
        for stmt in list {
            if flow.normal.is_empty() {
                break;
            }
            let inner = self.stmt(definition, stmt, std::mem::take(&mut flow.normal));
            flow.normal = self.cap(inner.normal);
            flow.breaks.extend(inner.breaks);
            flow.continues.extend(inner.continues);
            flow.returns.extend(inner.returns);
        }
        flow
    }

    fn stmt(&mut self, definition: usize, stmt: &Stmt, states: BTreeSet<PState>) -> PFlow {
        let normal = |normal| PFlow {
            normal,
            ..PFlow::default()
        };
        if let Some(budget) = &mut self.budget {
            if *budget == 0 {
                return normal(
                    states
                        .into_iter()
                        .map(|(mut g, e)| {
                            g.unresolved.insert(ENTRY_BUDGET_EXHAUSTED.to_owned());
                            (g, e)
                        })
                        .collect(),
                );
            }
            *budget = budget.saturating_sub(states.len().max(1));
        }
        match stmt {
            Stmt::Empty => normal(states),
            Stmt::Break => PFlow {
                breaks: states,
                ..PFlow::default()
            },
            Stmt::Continue => PFlow {
                continues: states,
                ..PFlow::default()
            },
            Stmt::Block(list) => self.block(definition, list, states),
            Stmt::Maybe(inner) => {
                let mut flow = self.stmt(definition, inner, states.clone());
                flow.normal.extend(states);
                flow
            }
            Stmt::Expr(s, e) => {
                let mut out = PFlow::default();
                for state in states {
                    for (next, _) in self.expression(definition, (*s, *e), state) {
                        if let Some(next) = next {
                            out.normal.insert(next);
                        }
                    }
                }
                out
            }
            Stmt::Return(s, e) => {
                let mut out = PFlow::default();
                for state in states {
                    for (next, value) in self.expression(definition, (*s, *e), state) {
                        if let Some((g, env)) = next {
                            let value = value.or_else(|| self.value_of(definition, (*s, *e), &env));
                            if definition == self.root_definition {
                                self.scope_end(definition, *s, &g);
                            }
                            let site = self.site(definition, s.saturating_sub(1));
                            out.returns.insert((g, value, site));
                        }
                    }
                }
                out
            }
            Stmt::If { cond, then, els } => {
                let mut taken = BTreeSet::new();
                let mut not_taken = BTreeSet::new();
                for state in states {
                    for (next, _) in self.expression(definition, *cond, state) {
                        let Some(next) = next else { continue };
                        let (t, f) = self.branch(definition, *cond, next);
                        taken.extend(t);
                        not_taken.extend(f);
                    }
                }
                let mut a = self.stmt(definition, then, taken);
                let b = match els {
                    Some(els) => self.stmt(definition, els, not_taken),
                    None => normal(not_taken),
                };
                a.normal.extend(b.normal);
                a.breaks.extend(b.breaks);
                a.continues.extend(b.continues);
                a.returns.extend(b.returns);
                a
            }
            Stmt::Loop {
                cond,
                init,
                body,
                step,
                ..
            } => {
                let mut current = states;
                if let Some(init) = init {
                    let mut next = BTreeSet::new();
                    for state in current {
                        for (s, _) in self.expression(definition, *init, state) {
                            next.extend(s);
                        }
                    }
                    current = next;
                }
                // Zero or one iteration: the condition may fail at once, or
                // the body runs once and the loop exits.
                let (enter, skip) = match cond {
                    Some(cond) => {
                        let mut enter = BTreeSet::new();
                        let mut skip = BTreeSet::new();
                        for state in current {
                            for (next, _) in self.expression(definition, *cond, state) {
                                let Some(next) = next else { continue };
                                let (t, f) = self.branch(definition, *cond, next);
                                enter.extend(t);
                                skip.extend(f);
                            }
                        }
                        (enter, skip)
                    }
                    None => (current.clone(), current),
                };
                let flow = self.stmt(definition, body, enter);
                let waits_in_body = {
                    let mut found = false;
                    visit_exprs(body, &mut |(s, e)| {
                        let tokens = &self.scans[self.definitions[definition].file].tokens;
                        if calls_in(tokens, s, e).iter().any(|(n, _, _)| {
                            let name = tokens[*n].text.as_str();
                            name == self.tables.wait || name == self.tables.timed_wait
                        }) {
                            found = true;
                        }
                    });
                    found
                };
                // A wait loop's condition exit observed the predicate too.
                let mut after: BTreeSet<PState> = skip
                    .into_iter()
                    .map(|(mut g, e)| {
                        if waits_in_body {
                            g.awaited = true;
                        }
                        (g, e)
                    })
                    .collect();
                let mut back: BTreeSet<PState> = flow.normal;
                back.extend(flow.continues);
                if let Some(step) = step {
                    let mut stepped = BTreeSet::new();
                    for state in back {
                        for (s, _) in self.expression(definition, *step, state) {
                            stepped.extend(s);
                        }
                    }
                    back = stepped;
                }
                // Leaving after one iteration: a wait loop's exit awaited.
                let waits = {
                    let mut found = false;
                    visit_exprs(body, &mut |(s, e)| {
                        let tokens = &self.scans[self.definitions[definition].file].tokens;
                        if calls_in(tokens, s, e).iter().any(|(n, _, _)| {
                            let name = tokens[*n].text.as_str();
                            name == self.tables.wait || name == self.tables.timed_wait
                        }) {
                            found = true;
                        }
                    });
                    found
                };
                for (mut g, e) in back {
                    if waits {
                        g.awaited = true;
                    }
                    after.insert((g, e));
                }
                after.extend(flow.breaks);
                PFlow {
                    normal: after,
                    breaks: BTreeSet::new(),
                    continues: BTreeSet::new(),
                    returns: flow.returns,
                }
            }
        }
    }

    fn scope_end(&mut self, definition: usize, index: usize, global: &Global) {
        let published = self.published.clone();
        for name in published {
            let states = BTreeSet::from([(global.clone(), Env::new())]);
            self.release(definition, index, format!("scope end of `{name}`"), &states);
        }
    }

    /// Split states on a condition: `(taken, not taken)`.
    fn branch(
        &mut self,
        definition: usize,
        cond: (usize, usize),
        (global, env): PState,
    ) -> (BTreeSet<PState>, BTreeSet<PState>) {
        let file = self.definitions[definition].file;
        if let Some(null_when_true) = self.excluded.get(&(file, cond.0)) {
            // The null arm is infeasible on every analysed execution.
            let state = BTreeSet::from([(global, env)]);
            return if *null_when_true {
                (BTreeSet::new(), state)
            } else {
                (state, BTreeSet::new())
            };
        }
        let tokens = &self.scans[file].tokens;
        let (negated, inner) = condition_atom(tokens, cond, self.tables);
        let words = texts(tokens, inner);
        // `NAME`, `NAME == INT`, `NAME != INT`.
        let parsed: Option<(&str, bool, i64)> = match words.as_slice() {
            [name] if is_identifier(name) => Some((name, false, 0)),
            [name, "==", value] => value.parse().ok().map(|v| (*name, true, v)),
            [name, "==", "-", value] => value.parse::<i64>().ok().map(|v| (*name, true, -v)),
            [name, "!=", value] => value.parse().ok().map(|v| (*name, false, v)),
            [name, "!=", "-", value] => value.parse::<i64>().ok().map(|v| (*name, false, -v)),
            _ => None,
        };
        let both = || {
            (
                BTreeSet::from([(global.clone(), env.clone())]),
                BTreeSet::from([(global.clone(), env.clone())]),
            )
        };
        let Some((name, equal, value)) = parsed else {
            return both();
        };
        // `[!] NAME` tests NAME != 0.
        let (equal, value) = if words.len() == 1 {
            (false, 0)
        } else {
            (equal, value)
        };
        let holds_when = |actual: i64| (actual == value) == equal;
        let flip = |taken: bool| taken != negated;
        match env.get(name) {
            Some(Val::Const(actual)) => {
                let taken = flip(holds_when(*actual));
                let state = BTreeSet::from([(global, env)]);
                if taken {
                    (state, BTreeSet::new())
                } else {
                    (BTreeSet::new(), state)
                }
            }
            Some(Val::JoinResult(thread, site)) if value == 0 => {
                // Zero is success.
                let mut success = global.clone();
                success.threads.insert(
                    thread.clone(),
                    Joined::Done(ReadJoin::CheckedSuccess, site.clone()),
                );
                let mut failure = global.clone();
                failure
                    .threads
                    .insert(thread.clone(), Joined::Done(ReadJoin::Failed, site.clone()));
                let mut env_success = env.clone();
                env_success.insert(name.to_owned(), Val::Const(0));
                let mut env = env;
                env.insert(name.to_owned(), Val::NonZero);
                let success_taken = flip(holds_when(0));
                let s = BTreeSet::from([(success, env_success)]);
                let f = BTreeSet::from([(failure, env)]);
                if success_taken { (s, f) } else { (f, s) }
            }
            Some(Val::NonZero) if value == 0 => {
                let taken = flip(!equal);
                let state = BTreeSet::from([(global, env)]);
                if taken {
                    (state, BTreeSet::new())
                } else {
                    (BTreeSet::new(), state)
                }
            }
            Some(Val::TimedWait) if value == 0 => {
                let mut timed_out = global.clone();
                timed_out.timed_out = true;
                let zero_taken = flip(holds_when(0));
                let s = BTreeSet::from([(global, env.clone())]);
                let f = BTreeSet::from([(timed_out, env)]);
                if zero_taken { (s, f) } else { (f, s) }
            }
            _ => both(),
        }
    }

    fn value_of(&self, definition: usize, range: (usize, usize), env: &Env) -> Option<Val> {
        let tokens = &self.scans[self.definitions[definition].file].tokens;
        match texts(tokens, range).as_slice() {
            [value] => value
                .parse()
                .ok()
                .map(Val::Const)
                .or_else(|| env.get(*value).cloned()),
            ["-", value] => value.parse::<i64>().ok().map(|v| Val::Const(-v)),
            _ => None,
        }
    }

    /// One expression: its calls in order, then its assignment. Returns each
    /// resulting state (`None` when the path ended) with the value of a
    /// trailing call, for a `return`.
    fn expression(
        &mut self,
        definition: usize,
        range: (usize, usize),
        state: PState,
    ) -> Vec<(Option<PState>, Option<Val>)> {
        let file = self.definitions[definition].file;
        let tokens = &self.scans[file].tokens;
        let calls = calls_in(tokens, range.0, range.1);
        let words = texts(tokens, range);
        // `NAME = …` at the head.
        let assigned: Option<String> = match words.as_slice() {
            [name, "=", ..] if is_identifier(name) => Some((*name).to_owned()),
            _ => None,
        };
        // Only calls not nested inside another call's arguments matter here,
        // except for the kinds the reader tracks wherever they stand.
        let mut current: Vec<(PState, Option<Val>)> = vec![(state, None)];
        for (name_index, open, close) in calls {
            let name = tokens[name_index].text.clone();
            let args = arguments(tokens, open, close);
            let site = self.site(definition, name_index);
            // Card W47-FLB-LIB-LIFECYCLE: a declared lifecycle call records
            // its context's history before it; a stop or destroy is appended
            // now (a destroy's release sees it), a start once its value is
            // known.
            let lifecycle = self.lifecycle_call(definition, &name, &args, name_index, &current);
            if let Some((key, operation)) = &lifecycle
                && *operation != qcore_contract::contract_loader::LifecycleOperation::Start
            {
                let event =
                    if *operation == qcore_contract::contract_loader::LifecycleOperation::Stop {
                        ReadLifeEvent::Stop
                    } else {
                        ReadLifeEvent::Destroy
                    };
                for ((global, _), _) in &mut current {
                    global
                        .histories
                        .entry(key.clone())
                        .or_default()
                        .push((event.clone(), site.clone()));
                }
            }
            let mut next: Vec<(PState, Option<Val>)> = Vec::new();
            for ((mut global, mut env), _) in current {
                if self.tables.exits.contains(&name.as_str()) {
                    continue; // The path ends.
                }
                if let Some((_, entry_arg, out_arg)) =
                    self.tables.spawns.iter().find(|(n, _, _)| *n == name)
                {
                    let entry = args
                        .get(*entry_arg)
                        .and_then(|r| root_name(tokens, *r))
                        .unwrap_or_default();
                    if entry.is_empty() {
                        global
                            .unresolved
                            .insert(format!("a spawn at {site} names no entry"));
                    } else {
                        global.threads.insert(entry.clone(), Joined::Live(None));
                        // `pthread_create` with a null attribute object
                        // creates a joinable thread by default.
                        let wrapper = if name == "pthread_create"
                            && args.get(1).is_some_and(|r| {
                                matches!(texts(tokens, *r).as_slice(), ["NULL" | "0"])
                            }) {
                            stop_paths::DEFAULT_ATTRIBUTE_CREATE.to_owned()
                        } else {
                            name.clone()
                        };
                        global.spawned_by.insert(entry.clone(), wrapper);
                        if let Some(out) = out_arg.and_then(|o| args.get(o))
                            && let Some(local) = root_name(tokens, *out)
                            && texts(tokens, *out).first() == Some(&"&")
                        {
                            env.insert(local, Val::ThreadId(BTreeSet::from([entry])));
                        }
                    }
                    next.push(((global, env), None));
                    continue;
                }
                if name == self.tables.join {
                    let thread = args.first().and_then(|r| {
                        let words = texts(tokens, *r);
                        match words.as_slice() {
                            [local] => match env.get(*local) {
                                Some(Val::ThreadId(set)) => Some(set.clone()),
                                _ => None,
                            },
                            [base, "->" | ".", field] => self
                                .types
                                .get(*base)
                                .and_then(|ty| self.thread_fields.get(&format!("{ty}.{field}")))
                                .or_else(|| self.thread_fields.get(*field))
                                .cloned(),
                            [.., "->" | ".", field] => self.thread_fields.get(*field).cloned(),
                            _ => None,
                        }
                    });
                    let thread = thread
                        .filter(|set| set.len() == 1)
                        .and_then(|set| set.into_iter().next());
                    // Card W47-FLB-LIB-LIFECYCLE: a clause check records every
                    // join the body executes, classified as below.
                    if self.trace {
                        let kind = match &thread {
                            None => ReadTraceJoin::Unresolved,
                            Some(t) if !global.threads.contains_key(t) => ReadTraceJoin::NotStarted,
                            Some(t) if !matches!(global.threads.get(t), Some(Joined::Live(_))) => {
                                ReadTraceJoin::Repeated
                            }
                            Some(_) => ReadTraceJoin::First,
                        };
                        global.trace.push((kind, site.clone()));
                    }
                    match thread {
                        // A join of a thread this path never started changes
                        // nothing this reader tracks.
                        Some(thread) if !global.threads.contains_key(&thread) => {
                            next.push(((global, env), None));
                        }
                        // A thread this path already joined: the join is
                        // repeated, whatever its result (card
                        // W47-CONDVAR-JOIN-STOP-PATHS).
                        Some(thread)
                            if !matches!(global.threads.get(&thread), Some(Joined::Live(_))) =>
                        {
                            global
                                .threads
                                .insert(thread, Joined::Done(ReadJoin::Repeated, site.clone()));
                            next.push(((global, env), None));
                        }
                        Some(thread) => {
                            let value = Val::JoinResult(thread.clone(), site.clone());
                            if assigned.is_some()
                                || words.first() == Some(&"return")
                                || range_is_return(tokens, range)
                            {
                                global.threads.insert(thread, Joined::Pending(site.clone()));
                                next.push(((global, env), Some(value)));
                            } else {
                                let wrapper =
                                    global.spawned_by.get(&thread).cloned().unwrap_or_default();
                                global.threads.insert(
                                    thread,
                                    Joined::Done(
                                        ReadJoin::UncheckedFirst { wrapper },
                                        site.clone(),
                                    ),
                                );
                                next.push(((global, env), None));
                            }
                        }
                        None => {
                            global
                                .unresolved
                                .insert(format!("the thread joined at {site} is not derived"));
                            next.push(((global, env), None));
                        }
                    }
                    continue;
                }
                if name == self.tables.timed_wait {
                    next.push(((global, env), Some(Val::TimedWait)));
                    continue;
                }
                if self.tables.releases.contains(&name.as_str())
                    || ((name == self.tables.cond_destroy || name == self.tables.mutex_destroy)
                        && args
                            .first()
                            .and_then(|r| root_name(tokens, *r))
                            .is_some_and(|root| self.published.contains(&root)))
                {
                    let states = BTreeSet::from([(global.clone(), env.clone())]);
                    self.release(definition, name_index, format!("`{name}`"), &states);
                    next.push(((global, env), None));
                    continue;
                }
                if name == "memcpy"
                    && let [destination, source, ..] = args.as_slice()
                    && let Some(local) = root_name(tokens, *destination)
                    && let [.., "->" | ".", field] = texts(tokens, *source).as_slice()
                    && let Some(set) = self.thread_fields.get(*field)
                    && set.len() == 1
                {
                    env.insert(local, Val::ThreadId(set.clone()));
                    next.push(((global, env), None));
                    continue;
                }
                match self.resolve(file, &name) {
                    Some(callee) if self.relevant.contains(&callee) => {
                        for (g, value) in self.run_function(callee, global.clone()) {
                            next.push(((g, env.clone()), value));
                        }
                    }
                    Some(callee) => {
                        if let Some(reason) = self.may_thread.get(&callee) {
                            global.unresolved.insert(reason.clone());
                        }
                        next.push(((global, env), None));
                    }
                    None => next.push(((global, env), None)),
                }
            }
            if let Some((key, qcore_contract::contract_loader::LifecycleOperation::Start)) =
                &lifecycle
            {
                for ((global, _), value) in &mut next {
                    let result = lifecycle::start_result(value.as_ref());
                    global
                        .histories
                        .entry(key.clone())
                        .or_default()
                        .push((ReadLifeEvent::Start(result), site.clone()));
                }
            }
            current = next;
            if current.is_empty() {
                return vec![(None, None)];
            }
        }
        // Card W47-FLB-LIB-LIFECYCLE: `KEY = CREATOR(…)` starts a context's
        // history.
        let created = self.created_key(definition, &words, range);
        let mut out = Vec::new();
        for ((mut global, mut env), value) in current {
            if let Some((key, site)) = &created {
                global
                    .histories
                    .insert(key.clone(), vec![(ReadLifeEvent::Create, site.clone())]);
            }
            if let Some(name) = &assigned {
                let rest = &words[2..];
                let assigned_value = if let Some(value) = value.clone() {
                    Some(value)
                } else if let [.., "->" | ".", field] = rest
                    && let Some(set) = self.thread_fields.get(*field)
                {
                    Some(Val::ThreadId(set.clone()))
                } else {
                    match rest {
                        [v] => v.parse().ok().map(Val::Const),
                        ["-", v] => v.parse::<i64>().ok().map(|v| Val::Const(-v)),
                        _ => None,
                    }
                };
                match assigned_value {
                    Some(v) => {
                        env.insert(name.clone(), v);
                    }
                    None => {
                        env.remove(name);
                    }
                }
            }
            // `X->F = &L` or `&L.G`: `L` is published.
            if let Some(position) = words.iter().position(|w| *w == "=")
                && position > 1
                && matches!(words[position - 2], "->" | ".")
                && words.get(position + 1) == Some(&"&")
                && let Some(local) = words.get(position + 2)
                && definition == self.root_definition
            {
                self.published.insert((*local).to_owned());
            }
            out.push((Some((global, env)), value));
        }
        out
    }
}

fn range_is_return(tokens: &[Token], range: (usize, usize)) -> bool {
    range.0 > 0 && tokens[range.0 - 1].text == "return"
}

// ------------------------------------------------------------- the program

/// A definition's statement tree, or why it cannot be read.
fn parse_body(scan: &FileScan, definition: &Definition) -> Result<Vec<Stmt>, String> {
    let (open, close) = definition.body;
    let mut parser = Parser {
        tokens: &scan.tokens,
        groups: Vec::new(),
        error: None,
    };
    for (gs, ge) in &scan.groups {
        if *gs > open && *ge <= close {
            parser.groups.push((*gs, *ge));
        } else if *gs <= open && *ge > close {
            // The whole definition is inside the group.
        } else if *gs < close && *ge > open {
            parser.fail(open, "a conditional group splits the body");
        }
    }
    let tree = parser.block(open, close);
    match parser.error {
        Some(error) => Err(error),
        None => Ok(tree),
    }
}

/// Read one program: the thread-entry census, every function's C4 sites, the
/// C5 releases of every test root and the single-signal derivations. No null
/// arm is excluded: [`read_program_with`] takes the decider.
#[must_use]
pub fn read_program(files: &[CensusFile], hops: &HopTables, tables: &SyncTables) -> SyncReading {
    read_program_with(files, hops, tables, &|_| Err("no decider".to_owned()))
}

/// [`read_program`] with a decider for null tests (card
/// W47-CONDVAR-JOIN-STOP-PATHS): the path reader drops the null arm of every
/// test for which `decide` returns `Ok`.
#[must_use]
pub fn read_program_with(
    files: &[CensusFile],
    hops: &HopTables,
    tables: &SyncTables,
    decide: &dyn Fn(&NullTestReading) -> Result<(), String>,
) -> SyncReading {
    read_program_lifecycle(files, hops, tables, decide, None)
}

/// [`read_program_with`], tracking the declared lifecycle roles (card
/// W47-FLB-LIB-LIFECYCLE): every declared call on a test root's paths with
/// its context's history, the histories at every release, and every declared
/// stop and destroy run from each clause's state. With `None` the reading is
/// exactly [`read_program_with`]'s.
#[must_use]
pub fn read_program_lifecycle(
    files: &[CensusFile],
    hops: &HopTables,
    tables: &SyncTables,
    decide: &dyn Fn(&NullTestReading) -> Result<(), String>,
    roles: Option<&LifecycleRoles>,
) -> SyncReading {
    let census = crate::thread_entry_census::read_program(files, hops);
    let (scans, definitions) = scan(files);
    let mut by_census = BTreeMap::new();
    for (id, definition) in definitions.iter().enumerate() {
        by_census.entry(definition.census.clone()).or_insert(id);
    }
    let names: BTreeSet<&str> = [
        tables.join,
        tables.lock,
        tables.wait,
        tables.timed_wait,
        tables.signal,
        tables.broadcast,
        tables.cond_init,
        tables.cond_destroy,
    ]
    .into_iter()
    .chain(tables.refused.iter().map(|(n, _)| *n))
    .chain(tables.refused_tokens.iter().map(|(n, _)| *n))
    .chain(tables.spawns.iter().map(|(n, _, _)| *n))
    .chain(tables.releases.iter().copied())
    .collect();
    let path_names: BTreeSet<&str> = [tables.join]
        .into_iter()
        .chain(tables.spawns.iter().map(|(n, _, _)| *n))
        .chain(tables.releases.iter().copied())
        .collect();
    // Direct calls by definition, and trees for the bodies that matter.
    let mut direct: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); definitions.len()];
    let mut own: Vec<bool> = vec![false; definitions.len()];
    let mut own_path: Vec<bool> = vec![false; definitions.len()];
    let resolve = |file: usize, name: &str| {
        by_census
            .get(&format!("{}::{name}", scans[file].path))
            .or_else(|| by_census.get(name))
            .copied()
    };
    for (id, definition) in definitions.iter().enumerate() {
        let tokens = &scans[definition.file].tokens;
        for index in definition.body.0..definition.body.1 {
            let text = tokens[index].text.as_str();
            if names.contains(text) {
                own[id] = true;
            }
            if path_names.contains(text) {
                own_path[id] = true;
            }
            if is_identifier(text)
                && tokens.get(index + 1).is_some_and(|t| t.text == "(")
                && let Some(callee) = resolve(definition.file, text)
            {
                direct[id].insert(callee);
            }
        }
    }
    let joins: BTreeSet<usize> = (0..definitions.len())
        .filter(|id| {
            let tokens = &scans[definitions[*id].file].tokens;
            (definitions[*id].body.0..definitions[*id].body.1)
                .any(|i| tokens[i].text == tables.join)
        })
        .collect();
    // Functions whose direct calls reach a spawn, a join or a release.
    let mut relevant: BTreeSet<usize> = (0..definitions.len()).filter(|id| own_path[*id]).collect();
    loop {
        let before = relevant.len();
        for id in 0..definitions.len() {
            if !relevant.contains(&id) && direct[id].iter().any(|c| relevant.contains(c)) {
                relevant.insert(id);
            }
        }
        if relevant.len() == before {
            break;
        }
    }
    // Functions that may start or join a thread through any hop the census
    // follows, but not through direct calls alone.
    let mut may_thread: BTreeMap<usize, String> = BTreeMap::new();
    {
        let mut reach: BTreeSet<usize> =
            census.spawns.iter().map(|(spawner, _)| *spawner).collect();
        loop {
            let before = reach.len();
            for edge in &census.edges {
                if reach.contains(&edge.callee) {
                    reach.insert(edge.caller);
                }
            }
            if reach.len() == before {
                break;
            }
        }
        for census_id in reach {
            if let Some(id) = by_census.get(&census.functions[census_id])
                && !relevant.contains(id)
            {
                may_thread.insert(
                    *id,
                    format!(
                        "`{}` may start a thread through a hop the path reader does not follow",
                        definitions[*id].census
                    ),
                );
            }
        }
    }
    // Fields holding a spawned thread identifier: `X->F = L;` after a spawn
    // wrote `&L`, or a spawn writing `&X->F` directly.
    let mut thread_fields: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for definition in &definitions {
        let tokens = &scans[definition.file].tokens;
        let types = struct_types(tokens, definition);
        let field_keys = |range: &[&str]| -> Vec<String> {
            match range {
                [.., base, "->" | ".", field] => {
                    let mut keys = vec![(*field).to_owned()];
                    if let Some(ty) = types.get(*base)
                        && range.len() == 3
                    {
                        keys.push(format!("{ty}.{field}"));
                    }
                    keys
                }
                _ => Vec::new(),
            }
        };
        let mut locals: BTreeMap<String, String> = BTreeMap::new();
        for (name_index, open, close) in calls_in(tokens, definition.body.0, definition.body.1) {
            let name = tokens[name_index].text.as_str();
            if let Some((_, entry_arg, Some(out_arg))) =
                tables.spawns.iter().find(|(n, _, _)| *n == name)
            {
                let args = arguments(tokens, open, close);
                let entry = args.get(*entry_arg).and_then(|r| root_name(tokens, *r));
                if let (Some(entry), Some(out)) = (entry, args.get(*out_arg)) {
                    match texts(tokens, *out).as_slice() {
                        ["&", local] => {
                            locals.insert((*local).to_owned(), entry);
                        }
                        ["&", rest @ ..] if rest.len() >= 3 => {
                            for key in field_keys(rest) {
                                thread_fields.entry(key).or_default().insert(entry.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for index in definition.body.0..definition.body.1 {
            if tokens[index].text == "="
                && index >= 2
                && matches!(tokens[index - 2].text.as_str(), "->" | ".")
                && let Some(value) = tokens.get(index + 1)
                && tokens.get(index + 2).is_some_and(|t| t.text == ";")
                && let Some(entry) = locals.get(&value.text)
            {
                let start = (definition.body.0..index)
                    .rev()
                    .find(|i| matches!(tokens[*i].text.as_str(), ";" | "{" | "}"))
                    .map_or(index, |i| i + 1);
                let words = texts(tokens, (start, index));
                for key in field_keys(&words) {
                    thread_fields.entry(key).or_default().insert(entry.clone());
                }
            }
        }
    }
    // Statement trees and C4 for every function with a synchronization token.
    let mut trees: BTreeMap<usize, Result<Vec<Stmt>, String>> = BTreeMap::new();
    let mut functions = Vec::new();
    let mut sites = Vec::new();
    for (id, definition) in definitions.iter().enumerate() {
        if !own[id] && !relevant.contains(&id) {
            continue;
        }
        let scan = &scans[definition.file];
        let result = parse_body(scan, definition);
        functions.push(FunctionSync {
            function: definition.census.clone(),
            file: scan.path.clone(),
            unresolved: result.as_ref().err().cloned(),
        });
        if own[id] {
            let types = struct_types(&scan.tokens, definition);
            let mut c4 = C4 {
                tokens: &scan.tokens,
                types: types.clone(),
                tables,
                path: &scan.path,
                function: &definition.census,
                sites: Vec::new(),
                loop_depth: 0,
                wait_condition: None,
            };
            match &result {
                Ok(tree) => {
                    let mut initial = LockState::default();
                    // A local struct object is fresh until something receives it.
                    for name in types.keys() {
                        if !definition.parameters.iter().any(|(p, _)| p == name) {
                            let declared_value = scan.tokens[definition.body.0..definition.body.1]
                                .windows(3)
                                .any(|w| {
                                    w[0].text == types[name]
                                        && w[1].text == *name
                                        && matches!(w[2].text.as_str(), ";" | "=")
                                });
                            if declared_value {
                                initial.fresh.insert(name.clone());
                            }
                        }
                    }
                    c4.statements(tree, Some(initial));
                }
                Err(_) => {
                    // Record the sites with no state so that each is visible,
                    // unresolved.
                    let unknown = Stmt::Expr(definition.body.0 + 1, definition.body.1);
                    c4.statement(
                        &unknown,
                        Some(LockState {
                            unknown_lock: true,
                            ..LockState::default()
                        }),
                    );
                }
            }
            for site in &mut c4.sites {
                if let ReadSite::Signal(signal) = &mut site.site {
                    let index = (definition.body.0..definition.body.1)
                        .find(|i| {
                            scan.tokens[*i].line == site.line
                                && (scan.tokens[*i].text == tables.signal
                                    || scan.tokens[*i].text == tables.broadcast)
                        })
                        .unwrap_or(definition.body.0);
                    signal.sections =
                        sections(&scan.tokens, definition.body, index, &types, tables);
                }
            }
            sites.extend(c4.sites);
        }
        trees.insert(id, result);
    }
    // Card W47-CONDVAR-JOIN-STOP-PATHS: null tests, decided; thread sites.
    let types = stop_paths::types(&scans);
    let mut null_tests = stop_paths::null_tests(&scans, &definitions, &trees, &types, tables);
    let mut excluded: BTreeMap<(usize, usize), bool> = BTreeMap::new();
    for test in &mut null_tests {
        test.excluded = decide(test);
        if test.excluded.is_ok() {
            excluded.insert((test.file_index, test.cond_start), test.null_when_true);
        }
    }
    let (thread_sites, self_sites, wrappers) =
        stop_paths::thread_sites(&scans, &definitions, tables, &thread_fields);
    // C5: every test root, with the relevant bodies read.
    let mut releases = Vec::new();
    let mut lifecycle_calls = Vec::new();
    for (root_id, _thread) in &census.roots {
        let root_name = &census.functions[*root_id];
        let Some(&root) = by_census.get(root_name) else {
            continue;
        };
        if !relevant.contains(&root) {
            continue;
        }
        let mut c5 = C5 {
            scans: &scans,
            definitions: &definitions,
            trees: &trees,
            by_census: &by_census,
            tables,
            relevant: &relevant,
            joins: &joins,
            may_thread: &may_thread,
            thread_fields: &thread_fields,
            excluded: &excluded,
            root: root_name.clone(),
            root_definition: root,
            types: BTreeMap::new(),
            stack: Vec::new(),
            published: BTreeSet::new(),
            releases: BTreeMap::new(),
            budget: None,
            roles,
            trace: false,
            lifecycle_calls: BTreeMap::new(),
        };
        c5.run_root(root);
        releases.extend(c5.releases.into_values());
        lifecycle_calls.extend(c5.lifecycle_calls.into_values());
    }
    // Every spawned entry, run on its own thread to its exit (its releases
    // are not a test root's and are not recorded).
    let mut entry_exits = BTreeMap::new();
    for (_, entry_id) in &census.spawns {
        let label = &census.functions[*entry_id];
        let plain = label.rsplit_once("::").map_or(label.as_str(), |(_, n)| n);
        if entry_exits.contains_key(plain) {
            continue;
        }
        let Some(&entry) = by_census.get(label) else {
            continue;
        };
        let mut c5 = C5 {
            scans: &scans,
            definitions: &definitions,
            trees: &trees,
            by_census: &by_census,
            tables,
            relevant: &relevant,
            joins: &joins,
            may_thread: &may_thread,
            thread_fields: &thread_fields,
            excluded: &excluded,
            root: label.clone(),
            root_definition: usize::MAX,
            types: BTreeMap::new(),
            stack: Vec::new(),
            published: BTreeSet::new(),
            releases: BTreeMap::new(),
            budget: Some(ENTRY_STEP_BUDGET),
            roles: None,
            trace: false,
            lifecycle_calls: BTreeMap::new(),
        };
        let mut exit = EntryExit {
            entry: label.clone(),
            ..EntryExit::default()
        };
        let parsed;
        let exits = if trees.contains_key(&entry) {
            c5.run_function(entry, Global::default())
        } else {
            // An entry the path reader does not otherwise read: read it now.
            parsed = parse_body(&scans[definitions[entry].file], &definitions[entry]);
            let mut local = trees.clone();
            local.insert(entry, parsed);
            let mut c5 = C5 {
                trees: &local,
                ..c5
            };
            c5.run_function(entry, Global::default())
        };
        for (global, _) in exits {
            exit.unresolved.extend(global.unresolved.iter().cloned());
            for (thread, joined) in &global.threads {
                let evidence = match joined {
                    Joined::Live(note) => (
                        ReadJoin::Absent {
                            timed_out: global.timed_out,
                            awaited: global.awaited,
                        },
                        note.clone()
                            .unwrap_or_else(|| "no join on this path".to_owned()),
                    ),
                    Joined::Pending(site) => (ReadJoin::Unchecked, site.clone()),
                    Joined::Done(join, site) => (join.clone(), site.clone()),
                };
                exit.live
                    .entry(thread.clone())
                    .or_default()
                    .insert(evidence);
            }
        }
        entry_exits.insert(plain.to_owned(), exit);
    }
    // Card W47-FLB-LIB-LIFECYCLE: every declared stop and destroy, run from
    // each clause's state with the join trace on.
    let lifecycle_bodies = match roles {
        Some(roles) => {
            let mut c5 = C5 {
                scans: &scans,
                definitions: &definitions,
                trees: &trees,
                by_census: &by_census,
                tables,
                relevant: &relevant,
                joins: &joins,
                may_thread: &may_thread,
                thread_fields: &thread_fields,
                excluded: &excluded,
                root: "lifecycle clause check".to_owned(),
                root_definition: usize::MAX,
                types: BTreeMap::new(),
                stack: Vec::new(),
                published: BTreeSet::new(),
                releases: BTreeMap::new(),
                budget: Some(ENTRY_STEP_BUDGET),
                roles: None,
                trace: true,
                lifecycle_calls: BTreeMap::new(),
            };
            lifecycle::read_bodies(&mut c5, roles)
        }
        None => Vec::new(),
    };
    // Kept-refusal tokens at file scope (a static initialiser).
    for (file, scan) in scans.iter().enumerate() {
        for (index, token) in scan.tokens.iter().enumerate() {
            let Some((_, refusal)) = tables.refused_tokens.iter().find(|(n, _)| *n == token.text)
            else {
                continue;
            };
            let inside = definitions
                .iter()
                .any(|d| d.file == file && d.body.0 < index && index < d.body.1);
            if !inside {
                sites.push(SyncSite {
                    file: scan.path.clone(),
                    line: token.line,
                    function: format!("{}::(file scope)", scan.path),
                    callee: token.text.clone(),
                    in_loop: false,
                    site: ReadSite::Refused(*refusal),
                });
            }
        }
    }
    let signal_once = at_most_once(
        &sites,
        &census,
        &definitions,
        &scans,
        &trees,
        &by_census,
        tables,
    );
    let defined_in = definitions
        .iter()
        .map(|d| (d.census.clone(), scans[d.file].path.clone()))
        .collect();
    SyncReading {
        census,
        functions,
        sites,
        releases,
        signal_once,
        thread_fields,
        defined_in,
        null_tests,
        thread_sites,
        self_sites,
        wrappers,
        entry_exits,
        lifecycle_calls,
        lifecycle_bodies,
    }
}

/// Derive, per condvar, that every signaller signals at most once per thread:
/// every signal site of the condvar is in one function `S`, not in a loop;
/// every call of `S` is in one function `G`, a spawn entry the census never
/// calls directly; no call of `S` in `G` is in a loop, and every one but the
/// textually last is followed at once by `return`.
fn at_most_once(
    sites: &[SyncSite],
    census: &CensusReading,
    definitions: &[Definition],
    scans: &[FileScan],
    trees: &BTreeMap<usize, Result<Vec<Stmt>, String>>,
    by_census: &BTreeMap<String, usize>,
    tables: &SyncTables,
) -> Vec<SignalOnce> {
    let mut by_condvar: BTreeMap<String, Vec<&SyncSite>> = BTreeMap::new();
    for site in sites {
        if let ReadSite::Signal(signal) = &site.site
            && let Some(condvar) = &signal.condvar
        {
            by_condvar.entry(condvar.clone()).or_default().push(site);
        }
    }
    let _ = tables;
    let mut out = Vec::new();
    for (condvar, signals) in by_condvar {
        let derived = (|| {
            let functions: BTreeSet<&String> = signals.iter().map(|s| &s.function).collect();
            let [signaller] = functions.into_iter().collect::<Vec<_>>()[..] else {
                return Err("the condvar is signalled from more than one function".to_owned());
            };
            if signals.len() != 1 || signals[0].in_loop {
                return Err(format!(
                    "`{signaller}` does not signal exactly once, outside a loop"
                ));
            }
            let plain = signaller
                .rsplit_once("::")
                .map_or(signaller.as_str(), |(_, n)| n);
            let calls: Vec<_> = census.calls.iter().filter(|c| c.callee == plain).collect();
            let callers: BTreeSet<usize> = calls.iter().map(|c| c.caller).collect();
            let [caller] = callers.into_iter().collect::<Vec<_>>()[..] else {
                return Err(format!(
                    "`{signaller}` is called from more than one function"
                ));
            };
            if !census.spawns.iter().any(|(_, entry)| *entry == caller)
                || census.edges.iter().any(|e| e.callee == caller)
            {
                return Err(format!(
                    "`{}` is not a thread entry the census never calls directly",
                    census.functions[caller]
                ));
            }
            let caller_name = &census.functions[caller];
            let Some(&definition) = by_census.get(caller_name) else {
                return Err(format!("`{caller_name}` is not read"));
            };
            let parsed;
            let tree = match trees.get(&definition) {
                Some(Ok(tree)) => tree,
                Some(Err(why)) => return Err(format!("`{caller_name}`'s body is not read: {why}")),
                None => {
                    parsed = parse_body(
                        &scans[definitions[definition].file],
                        &definitions[definition],
                    )
                    .map_err(|why| format!("`{caller_name}`'s body is not read: {why}"))?;
                    &parsed
                }
            };
            let tokens = &scans[definitions[definition].file].tokens;
            // Walk the tree: each call statement of the signaller, whether it
            // is in a loop, and whether `return` follows it at once.
            let mut found: Vec<(usize, bool, bool)> = Vec::new();
            fn walk(
                list: &[Stmt],
                in_loop: bool,
                tokens: &[Token],
                plain: &str,
                found: &mut Vec<(usize, bool, bool)>,
            ) {
                for (position, stmt) in list.iter().enumerate() {
                    let followed_by_return =
                        matches!(list.get(position + 1), Some(Stmt::Return(..)));
                    match stmt {
                        Stmt::Expr(s, e) => {
                            for (n, _, _) in calls_in(tokens, *s, *e) {
                                if tokens[n].text == plain {
                                    found.push((n, in_loop, followed_by_return));
                                }
                            }
                        }
                        Stmt::Return(s, e) => {
                            for (n, _, _) in calls_in(tokens, *s, *e) {
                                if tokens[n].text == plain {
                                    found.push((n, in_loop, true));
                                }
                            }
                        }
                        Stmt::Block(inner) => walk(inner, in_loop, tokens, plain, found),
                        Stmt::Maybe(inner) => {
                            walk(std::slice::from_ref(inner), in_loop, tokens, plain, found)
                        }
                        Stmt::If { cond, then, els } => {
                            if calls_in(tokens, cond.0, cond.1)
                                .iter()
                                .any(|(n, _, _)| tokens[*n].text == plain)
                            {
                                found.push((cond.0, true, false));
                            }
                            walk(std::slice::from_ref(then), in_loop, tokens, plain, found);
                            if let Some(els) = els {
                                walk(std::slice::from_ref(els), in_loop, tokens, plain, found);
                            }
                        }
                        Stmt::Loop { body, .. } => {
                            walk(std::slice::from_ref(body), true, tokens, plain, found)
                        }
                        _ => {}
                    }
                }
            }
            walk(tree, false, tokens, plain, &mut found);
            if found.len() != calls.iter().filter(|c| c.caller == caller).count()
                || found.is_empty()
            {
                return Err(format!(
                    "a call of `{signaller}` in `{caller_name}` is not a statement"
                ));
            }
            let last = found.iter().map(|(n, _, _)| *n).max().unwrap_or(0);
            for (n, in_loop, followed) in &found {
                if *in_loop {
                    return Err(format!("`{caller_name}` calls `{signaller}` in a loop"));
                }
                if *n != last && !followed {
                    return Err(format!(
                        "`{caller_name}` can call `{signaller}` twice on a path"
                    ));
                }
            }
            Ok(caller_name.clone())
        })();
        out.push(SignalOnce { condvar, derived });
    }
    out
}
