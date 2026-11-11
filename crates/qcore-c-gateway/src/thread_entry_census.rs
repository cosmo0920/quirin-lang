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

//! Thread-entry census: derived thread identity for the loop-owner obligation
//! (card **W47-LOOP-OWNER-CONTRACTS**, gateway rule 10p).
//!
//! # What it reads
//!
//! A closed set of C source files — one program, for example the in_tail
//! runtime test binary's library, plugin and test sources — read lexically
//! with the gateway's own tokenizer. Every definition is found at file scope;
//! a `static` one is named `FILE::NAME`, an external one `NAME`.
//! [`read_program`] returns the plain reading ([`CensusReading`]): roots,
//! edges with their hop and mode effect, spawns, unresolved functions and every
//! call. The gateway depends on no kernel crate (workspace layering), so
//! `quirin_source_analysis::loop_owner_census` turns the reading into a
//! `qcore_core::loop_owner::ThreadProgram`, derives it and decides the owner
//! obligation at every call to a function whose contract declares one.
//!
//! # The hop kinds it follows (W47-TAIL-PAUSE-BINDINGS §2.1)
//!
//! * **Direct** calls by name.
//! * **A**: each function listed in the test table (`TEST_LIST = { {"n", f},
//!   ... }`) is a root on the test thread `T`.
//! * **S**: a function named at a spawn wrapper's entry argument
//!   ([`HopTables::spawns`]) is a new thread named after it, in the spawner's
//!   mode.
//! * **V / V\***: a function stored in a named field (`.F = f` in an
//!   initializer, `x->F = f` or `x.F = f` at run time, the revised test's
//!   copied vtable included) is dispatched at every `->F (` / `.F (` call, in
//!   the dispatcher's context. Field-keyed: a registrant is attributed every
//!   dispatcher of that field name, whatever object it was stored in.
//! * **R**: a function named at a collector registration's callback argument
//!   ([`HopTables::collector_registrations`]) is registered on
//!   [`HopTables::collector_field`].
//! * **L**: a dispatch of the collector field inside a loop dispatcher
//!   ([`HopTables::loop_dispatchers`]) runs the callback on the dispatcher's
//!   thread in that loop's mode.
//! * **F**: a call inside the then/else block of `if (P(x))` or
//!   `if (!P(x))`, `P` the threaded-flag predicate, requires that mode.
//! * **C** (card W47-LOOP-OWNER-COROUTINE-HOP): a coroutine runs on the thread
//!   that resumes it. A listed constructor ([`CoroutineHop`]) names its entry
//!   at the coroutine primitive's entry argument (`co_create(_, E, _)`); each
//!   function that calls the constructor as `v = K(…)` and resumes `v` through
//!   the listed wrapper is a derived resumer, and runs the entry after its
//!   hand-off (inside a loop dispatcher, in that loop's mode, as hop L). The
//!   entry's calls up to its single switch back to the resumer
//!   (`co_switch(x->caller)`) run in the constructor. A constructor call that
//!   is not so resumed, or a wrapper call on a handle that is not the
//!   constructor's, leaves the entry unresolved. A function the entry
//!   dispatches after its hand-off is a coroutine body: one that may reach a
//!   non-terminal yield has an underived resumer after the yield (the kernel's
//!   rule), and a collector registrant whose input plugin's descriptor flags
//!   say it runs no coroutine is not dispatched from the entry at all.
//!
//! # Configured reading
//!
//! [`read_program_configured`] reads conditional groups under a
//! [`BuildConfiguration`]: an arm is dropped only when its condition is
//! decided false (or an earlier arm is decided true) for that file's
//! translation unit; a group whose first kept arm is decided true is read as
//! that arm alone. A condition it cannot decide keeps its arm, and every kept
//! arm is scanned as the raw reading does.
//!
//! # Fail-closed
//!
//! Every other occurrence of a defined function's name — an address passed
//! to anything but a recognised hop, stored in a variable, returned, listed
//! in a positional initializer, or written in a macro replacement list — marks
//! it unresolved, and so is every field whose value is read other than by a
//! call or a test (its registrants may be called from anywhere). A file whose
//! conditional groups this reader cannot structure marks every function it
//! defines unresolved. Unresolved reasons flow to every function reached.
//!
//! Premises stated, not proved (trust boundary 28): the tokenizer's reading
//! of the files; that no code outside the read files calls or takes the
//! address of a read function by name; that a function stored in a field is
//! called only through that field's name; and that along a derived path the
//! instance a collector operation names is the instance whose mode the path
//! fixed (P-INSTANCE: `ctx->ins` is the dispatching instance).

use std::collections::{BTreeMap, BTreeSet};

/// The threaded flag a hop fixes or requires (hop F, hop L).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadMode {
    NonThreaded,
    Threaded,
}

/// What an edge does to the mode of the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadEffect {
    Keep,
    Require(ReadMode),
    Dispatch(ReadMode),
}

/// The hop an edge was read through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadHop {
    Direct,
    Vtable,
    CollectorRegistry,
    /// Hop C: a derived resumer runs a coroutine's entry.
    Coroutine,
}

/// One edge the census read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadEdge {
    pub caller: usize,
    pub callee: usize,
    pub hop: ReadHop,
    pub effect: ReadEffect,
}

/// One call by name, for the obligation check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadCall {
    pub file: String,
    pub line: usize,
    pub caller: usize,
    pub callee: String,
}

/// The census's reading of one program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CensusReading {
    /// Census names (`NAME`, or `PATH::NAME` for a `static` definition).
    pub functions: Vec<String>,
    /// Test-table roots and their thread.
    pub roots: Vec<(usize, String)>,
    pub edges: Vec<ReadEdge>,
    /// `(spawner, entry)`: the entry is a new thread named after it.
    pub spawns: Vec<(usize, usize)>,
    /// Functions the census could not close, with why.
    pub unresolved: BTreeMap<usize, BTreeSet<String>>,
    pub calls: Vec<ReadCall>,
    /// The single definition of each loop dispatcher, when there is one.
    pub engine_dispatcher: Option<usize>,
    pub worker_dispatcher: Option<usize>,
    /// `(entry, body)`: `body` is dispatched from coroutine `entry` after its
    /// hand-off, so it runs on the entry's resumer (hop C).
    pub coroutine_bodies: Vec<(usize, usize)>,
    /// Functions that yield their coroutine without ending it, with where.
    pub yields: BTreeMap<usize, String>,
    /// Card W47-LOOP-OWNER-TYPED-DISPATCH: field dispatch sites by how the
    /// typed reading keyed them (`untyped`, `typed`, `unmatched`), and
    /// Step B's derivation for each collector registrant registered outside
    /// a plugin directory.
    pub dispatch_typing: BTreeMap<String, Vec<String>>,
    pub instance_flags: BTreeMap<usize, InstanceFlag>,
    /// Card W47-SERVICE-PROPERTY-KEYS: each keyed setter call Step B's
    /// list-head region read (`PATH:LINE SETTER`), with its derived table and
    /// per-key reach, or why its keys are not read.
    pub property_keys: BTreeMap<String, Vec<String>>,
    /// Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: per read file, each
    /// reserved macro the recorded compiler decided in one of its conditions
    /// (`NAME=defined` / `NAME=undefined`), where the configured reading
    /// alone left it undecided.
    pub predefined_decisions: BTreeMap<String, BTreeSet<String>>,
    /// Card W47-PREMISES-CORO-PLUGIN: P-CORO-HANDLE, P-PLUGIN-DIR and the
    /// immutable threaded flag as derived from the typed reading, with the
    /// named residuals and their sites (empty without a typed reading).
    pub coro_plugin_premises: BTreeMap<String, Vec<String>>,
    /// Card W47-PREMISES-INSTANCE: the premised census's report (`None`
    /// outside it).
    pub premises: Option<PremiseReport>,
}

impl CensusReading {
    /// A function's index by census name.
    #[must_use]
    pub fn index(&self, name: &str) -> Option<usize> {
        self.functions.iter().position(|function| function == name)
    }
}

use crate::legacy_c::{
    ConditionalDirective, ConditionalMark, Token, is_identifier, tokenize_with_conditionals,
};

// Card W47-LOOP-OWNER-INSTANCE-FLOW: the instance a runtime plugin copy is
// installed on, read by Step B.
#[path = "loop_owner_instance.rs"]
mod loop_owner_instance;
pub use loop_owner_instance::{FLUENT_BIT_INSTANCE_FLOW, InstanceFlowTables};

// Card W47-SERVICE-PROPERTY-KEYS: the keys a variadic property setter
// carries, read by Step B's list-head region.
#[path = "loop_owner_property_keys.rs"]
mod loop_owner_property_keys;
pub use loop_owner_property_keys::{FLUENT_BIT_PROPERTY_KEYS, PropertyKeyTables};

// Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: reserved macros decided from the
// recorded compiler, for the configured reading.
#[path = "loop_owner_predefined_macros.rs"]
pub mod predefined_macros;

// Card W47-PREMISES-CORO-PLUGIN: P-CORO-HANDLE, P-PLUGIN-DIR and the
// immutable threaded flag, derived from the program.
#[path = "loop_owner_coro_plugin.rs"]
mod loop_owner_coro_plugin;
pub use loop_owner_coro_plugin::{CoroPluginTables, FLUENT_BIT_CORO_PLUGIN};
// Card W47-PREMISES-INSTANCE: the list-head instance's premises and
// P-INSTANCE, derived from the pinned source.
#[path = "loop_owner_premises_instance.rs"]
mod loop_owner_premises_instance;
pub use loop_owner_premises_instance::{
    FLUENT_BIT_INSTANCE_PREMISES, FieldsByMember, GeneratedFile, InstancePremiseTables,
    PremiseInputs, PremiseReport, SitePremise,
};
// Card W47-PREMISES-HOLDER-REGION: the list-head region's macro invocations
// read from the units' own expansions, and the holder's opacity derived.
#[path = "loop_owner_holder_opaque.rs"]
mod holder_opaque;
#[path = "loop_owner_region_macros.rs"]
pub mod region_macros;

/// The closed hop vocabulary of one program family.
#[derive(Clone, Copy, Debug)]
pub struct HopTables {
    /// `(wrapper, index of the entry argument)`.
    pub spawns: &'static [(&'static str, usize)],
    /// `(registration, index of the callback argument)`.
    pub collector_registrations: &'static [(&'static str, usize)],
    /// The field a collector registration stores its callback in.
    pub collector_field: &'static str,
    /// `(dispatcher function, mode)`: a dispatch of the collector field inside
    /// that function runs on its thread in that mode.
    pub loop_dispatchers: &'static [(&'static str, ReadMode)],
    /// The test-table variable (hop A).
    pub test_table: &'static str,
    /// The thread name of a test-table root.
    pub test_thread: &'static str,
    /// The threaded-flag predicate (hop F).
    pub threaded_predicate: &'static str,
    /// `(primitive, index of the entry argument)` that creates a coroutine
    /// (hop C).
    pub coroutine_primitive: (&'static str, usize),
    /// The coroutine constructors whose resumers are derived.
    pub coroutines: &'static [CoroutineHop],
    /// The context switch; `switch(x->RESUMER_FIELD)` switches back to the
    /// coroutine's resumer (a yield, or an entry's hand-off).
    pub switch_primitive: &'static str,
    pub resumer_field: &'static str,
    /// Functions whose switch back ends the coroutine: it is not resumed again.
    pub terminal_returns: &'static [&'static str],
    /// The input-plugin descriptor type (`struct T NAME = { … }`), its flags
    /// field and the flag that makes an instance run its collectors in a
    /// coroutine, and the directory plugins live under.
    pub plugin_descriptor: &'static str,
    pub plugin_flags_field: &'static str,
    pub coroutine_flag: &'static str,
    pub plugin_root: &'static str,
    /// The record a collector registration stores its callback in (card
    /// W47-LOOP-OWNER-TYPED-DISPATCH): used only when the typed reading shows
    /// a parameter stored in [`HopTables::collector_field`] of that record.
    pub collector_record: &'static str,
    /// The instance's plugin field, the instance's coroutine flag (written
    /// only by the constructor, from the creating plugin's flags), the
    /// instance constructor with its plugin-name argument, and the
    /// descriptor's name field (Step B).
    pub instance_plugin_field: &'static str,
    pub instance_flag_field: &'static str,
    pub instance_constructor: (&'static str, usize),
    pub plugin_name_field: &'static str,
}

/// One coroutine constructor whose resumers the census derives (hop C).
#[derive(Clone, Copy, Debug)]
pub struct CoroutineHop {
    /// Names the entry at the coroutine primitive and returns the handle.
    pub constructor: &'static str,
    /// The wrapper a caller resumes the returned handle with.
    pub resume: &'static str,
}

/// Fluent Bit's hops as W47-TAIL-PAUSE-BINDINGS bound them at `90978ac6`.
pub const FLUENT_BIT_HOPS: HopTables = HopTables {
    spawns: &[
        // lib/monkey/mk_core/mk_utils.c:297 (mk_worker_start.callback).
        ("mk_utils_worker_spawn", 0),
        // src/flb_worker.c:77 (flb_worker.func).
        ("flb_worker_create", 0),
        // src/flb_thread_pool.c:63 (flb_tp_thread.params.func).
        ("flb_tp_thread_create", 1),
        ("pthread_create", 2),
    ],
    collector_registrations: &[
        ("flb_input_set_collector_time", 1),
        ("flb_input_set_collector_event", 1),
        ("flb_input_set_collector_socket", 1),
        ("flb_input_set_collector_server", 1),
    ],
    collector_field: "cb_collect",
    loop_dispatchers: &[
        // src/flb_input.c: the engine loop, config->evl.
        ("flb_input_collector_fd", ReadMode::NonThreaded),
        // src/flb_input_thread.c: the worker loop, thi->evl.
        ("input_collector_fd", ReadMode::Threaded),
    ],
    test_table: "TEST_LIST",
    test_thread: "T",
    threaded_predicate: "flb_input_is_threaded",
    // include/fluent-bit/flb_input.h:720 (`coro->callee = co_create(…,
    // input_pre_cb_collect, …)` in flb_input_coro_collect), resumed by
    // flb_input_coro_resume at src/flb_input.c:3318 and
    // src/flb_input_thread.c:167.
    coroutine_primitive: ("co_create", 1),
    coroutines: &[CoroutineHop {
        constructor: "flb_input_coro_collect",
        resume: "flb_input_coro_resume",
    }],
    switch_primitive: "co_switch",
    resumer_field: "caller",
    // include/fluent-bit/flb_input.h:772: FLB_INPUT_RETURN's yield after
    // flb_input_coro_prepare_destroy.
    terminal_returns: &["flb_input_return_do"],
    // src/flb_input.c:625: `if (plugin->flags & FLB_INPUT_CORO)
    // instance->runs_in_coroutine = FLB_TRUE;`.
    plugin_descriptor: "flb_input_plugin",
    plugin_flags_field: "flags",
    coroutine_flag: "FLB_INPUT_CORO",
    plugin_root: "plugins/",
    // src/flb_input.c:2245 (`coll->cb_collect = cb` in collector_create).
    collector_record: "struct flb_input_collector",
    // src/flb_input.c:490, :501, :625-626 (flb_input_new: `instance->p =
    // plugin`, runs_in_coroutine from the creating plugin's flags), read by
    // the route switch at src/flb_input.c:3313 and src/flb_input_thread.c:162.
    instance_plugin_field: "p",
    instance_flag_field: "runs_in_coroutine",
    instance_constructor: ("flb_input_new", 1),
    plugin_name_field: "name",
};

impl HopTables {
    /// The typed facts this census reads besides dispatches and stores.
    #[must_use]
    pub fn typed_options(&self) -> crate::typed_dispatch_census::TypedOptions {
        crate::typed_dispatch_census::TypedOptions {
            literal_calls: vec![(
                self.instance_constructor.0.to_owned(),
                self.instance_constructor.1,
            )],
            named_objects: vec![(
                format!("struct {}", self.plugin_descriptor),
                self.plugin_name_field.to_owned(),
            )],
            aliases: BTreeMap::new(),
            member_fields: Vec::new(),
            argument_calls: Vec::new(),
            key_tables: Vec::new(),
            record_views: Vec::new(),
        }
    }

    /// [`HopTables::typed_options`] with the instance-flow facts (card
    /// W47-LOOP-OWNER-INSTANCE-FLOW): the creator wrappers' literals and the
    /// member accesses Step B's list-head and holder derivations read.
    #[must_use]
    pub fn instance_flow_options(
        &self,
        flow: &InstanceFlowTables,
    ) -> crate::typed_dispatch_census::TypedOptions {
        let mut options = self.typed_options();
        options.literal_calls.extend(
            flow.creator_wrappers
                .iter()
                .map(|(wrapper, argument)| ((*wrapper).to_owned(), *argument)),
        );
        options.member_fields = flow.member_fields(self);
        // Card W47-PREMISES-CORO-PLUGIN: the members and the setter key this
        // card's derivations read.
        options
            .member_fields
            .extend(FLUENT_BIT_CORO_PLUGIN.member_fields(self));
        options
            .literal_calls
            .extend(FLUENT_BIT_CORO_PLUGIN.literal_calls());
        options
    }

    /// [`HopTables::instance_flow_options`] with the property-key facts (card
    /// W47-SERVICE-PROPERTY-KEYS): the setters' argument lists, the keyed
    /// branches' comparisons and the key tables.
    #[must_use]
    pub fn property_key_options(
        &self,
        flow: &InstanceFlowTables,
        keys: &PropertyKeyTables,
    ) -> crate::typed_dispatch_census::TypedOptions {
        let mut options = self.instance_flow_options(flow);
        options.argument_calls = keys.argument_calls();
        options.key_tables = keys.key_tables.iter().map(|t| (*t).to_owned()).collect();
        // Card W47-PREMISES-CORO-PLUGIN: the property setter's arm keys.
        options
            .argument_calls
            .extend(FLUENT_BIT_CORO_PLUGIN.argument_calls());
        options
    }

    /// [`HopTables::property_key_options`] with the premise facts (card
    /// W47-PREMISES-INSTANCE): an input instance's list link, and the listed
    /// records' non-member uses.
    #[must_use]
    pub fn premise_options(
        &self,
        flow: &InstanceFlowTables,
        keys: &PropertyKeyTables,
        premises: &InstancePremiseTables,
    ) -> crate::typed_dispatch_census::TypedOptions {
        let mut options = self.property_key_options(flow, keys);
        options.member_fields.push((
            flow.instance_record.to_owned(),
            premises.instance_link.to_owned(),
        ));
        options.record_views = premises.records.iter().map(|r| (*r).to_owned()).collect();
        options
    }
}

/// Step B's derivation of one collector registrant's coroutine flag when its
/// plugin directory does not decide it (card W47-LOOP-OWNER-TYPED-DISPATCH).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceFlag {
    /// `Some(false)` only when both the instance's creating plugin and the
    /// plugin copy it runs are derived to run no coroutine.
    pub derived: Option<bool>,
    /// What the derivation rests on, or where it stopped.
    pub reason: String,
}

/// The build a configured reading is for: the macros each translation unit's
/// compile command defines, and the macro prefixes the build configuration
/// owns.
#[derive(Clone, Debug, Default)]
pub struct BuildConfiguration {
    /// A read `.c` file's path → the macros its compile command defines.
    pub units: BTreeMap<String, BTreeSet<String>>,
    /// A macro with one of these prefixes that no compile command defines
    /// and no read file `#define`s is undefined; any other macro not defined
    /// by the unit's command is undecided.
    pub owned_prefixes: Vec<String>,
    /// Macros `#define`d in files the census does not read (the build's
    /// generated headers): never decided undefined.
    pub defined_elsewhere: BTreeSet<String>,
    /// Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: reserved macros the
    /// recorded compiler decides per command; `None` decides none.
    pub predefined: Option<predefined_macros::PredefinedMacros>,
    /// Card W47-PREMISES-HOLDER-REGION: the units' own expansions of the
    /// list-head region's macro invocations; `None` reads none.
    pub region_macros: Option<region_macros::RegionMacros>,
}

/// A condition's value under a configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Truth {
    True,
    False,
    Unknown,
}

impl Truth {
    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    fn or(self, other: Self) -> Self {
        self.not().and(other.not()).not()
    }
}

/// What a configured reading of one file knows about each macro.
struct MacroView<'a> {
    defined: &'a BTreeSet<String>,
    defined_by_some_unit: &'a BTreeSet<String>,
    written: &'a BTreeSet<String>,
    undefined_somewhere: &'a BTreeSet<String>,
    owned_prefixes: &'a [String],
    /// The read file's path and the recorded compiler's decisions (card
    /// W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS).
    path: &'a str,
    predefined: Option<&'a predefined_macros::PredefinedMacros>,
    /// The decisions the recorded compiler made here, for the report.
    decisions: std::cell::RefCell<BTreeSet<String>>,
}

impl MacroView<'_> {
    fn defined(&self, name: &str) -> Truth {
        if self.defined.contains(name) && !self.undefined_somewhere.contains(name) {
            Truth::True
        } else if self
            .owned_prefixes
            .iter()
            .any(|p| name.starts_with(p.as_str()))
            && !self.defined_by_some_unit.contains(name)
            && !self.written.contains(name)
        {
            Truth::False
        } else {
            // A reserved macro the recorded compiler decides: a read file's
            // own `#define`/`#undef` (or a generated header's) still wins.
            let truth = match self.predefined.and_then(|p| p.decide(self.path, name)) {
                Some(true) if !self.undefined_somewhere.contains(name) => Truth::True,
                Some(false) if !self.written.contains(name) => Truth::False,
                _ => Truth::Unknown,
            };
            if truth != Truth::Unknown {
                self.decisions.borrow_mut().insert(format!(
                    "{name}={}",
                    if truth == Truth::True {
                        "defined"
                    } else {
                        "undefined"
                    }
                ));
            }
            truth
        }
    }
}

/// The value of one conditional directive's condition, `#else` being true.
/// Only `defined`, `!`, `&&`, `||` and parentheses are read; anything else
/// is undecided.
fn directive_truth(text: &str, view: &MacroView<'_>) -> Truth {
    let mut line = text.replace("\\\r\n", " ").replace("\\\n", " ");
    while let Some(start) = line.find("/*") {
        let end = line[start + 2..]
            .find("*/")
            .map_or(line.len(), |e| start + 2 + e + 2);
        line.replace_range(start..end, " ");
    }
    if let Some(start) = line.find("//") {
        line.truncate(start);
    }
    let body = line.trim_start().trim_start_matches('#').trim_start();
    let word_end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(body.len());
    let (word, rest) = body.split_at(word_end);
    let rest = rest.trim();
    let single = |rest: &str| {
        if is_identifier(rest) {
            view.defined(rest)
        } else {
            Truth::Unknown
        }
    };
    match word {
        "else" => Truth::True,
        "ifdef" | "elifdef" => single(rest),
        "ifndef" | "elifndef" => single(rest).not(),
        "if" | "elif" => expression_truth(rest, view),
        _ => Truth::Unknown,
    }
}

fn expression_truth(text: &str, view: &MacroView<'_>) -> Truth {
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
        } else if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            tokens.push(&text[start..index]);
        } else if text[index..].starts_with("&&") || text[index..].starts_with("||") {
            tokens.push(&text[index..index + 2]);
            index += 2;
        } else if (byte == b'!' && bytes.get(index + 1) != Some(&b'='))
            || byte == b'('
            || byte == b')'
        {
            tokens.push(&text[index..=index]);
            index += 1;
        } else {
            return Truth::Unknown;
        }
    }
    let mut position = 0;
    let value = disjunction(&tokens, &mut position, view);
    if position == tokens.len() {
        value.unwrap_or(Truth::Unknown)
    } else {
        Truth::Unknown
    }
}

fn disjunction(tokens: &[&str], position: &mut usize, view: &MacroView<'_>) -> Option<Truth> {
    let mut value = conjunction(tokens, position, view)?;
    while tokens.get(*position) == Some(&"||") {
        *position += 1;
        value = value.or(conjunction(tokens, position, view)?);
    }
    Some(value)
}

fn conjunction(tokens: &[&str], position: &mut usize, view: &MacroView<'_>) -> Option<Truth> {
    let mut value = unary(tokens, position, view)?;
    while tokens.get(*position) == Some(&"&&") {
        *position += 1;
        value = value.and(unary(tokens, position, view)?);
    }
    Some(value)
}

fn unary(tokens: &[&str], position: &mut usize, view: &MacroView<'_>) -> Option<Truth> {
    match *tokens.get(*position)? {
        "!" => {
            *position += 1;
            Some(unary(tokens, position, view)?.not())
        }
        "(" => {
            *position += 1;
            let value = disjunction(tokens, position, view)?;
            (tokens.get(*position) == Some(&")")).then(|| {
                *position += 1;
                value
            })
        }
        "defined" => {
            *position += 1;
            let parenthesised = tokens.get(*position) == Some(&"(");
            if parenthesised {
                *position += 1;
            }
            let name = *tokens.get(*position)?;
            if !is_identifier(name) {
                return None;
            }
            *position += 1;
            if parenthesised {
                if tokens.get(*position) != Some(&")") {
                    return None;
                }
                *position += 1;
            }
            Some(view.defined(name))
        }
        // A macro's value, a number or any other operand is not read.
        _ => None,
    }
}

/// One conditional group being narrowed by [`configure`].
struct Narrowing {
    /// The group stands in a kept arm of every enclosing group.
    live: bool,
    /// Some earlier arm is decided true: no later arm is taken.
    decided: bool,
    /// Whether the current arm is kept.
    kept: bool,
    /// Whether a kept arm has been seen, and whether the first one was
    /// decided true (then the group is read as that arm alone).
    first_kept: Option<bool>,
}

/// Drop the arms a configuration decides are not taken, and the marks of a
/// group reduced to one decided arm. The token stream keeps its lines.
fn configure(
    tokens: Vec<Token>,
    marks: &[ConditionalMark],
    text: &str,
    view: &MacroView<'_>,
) -> (Vec<Token>, Vec<(usize, ConditionalDirective)>) {
    let mut kept_tokens = Vec::with_capacity(tokens.len());
    let mut kept_marks = Vec::new();
    let mut groups: Vec<Narrowing> = Vec::new();
    let mut next_mark = 0;
    let live = |groups: &[Narrowing]| groups.iter().all(|g| g.live && g.kept);
    for index in 0..=tokens.len() {
        while next_mark < marks.len() && marks[next_mark].token_index() <= index {
            let mark = marks[next_mark];
            next_mark += 1;
            let (start, end) = mark.span();
            let truth = || directive_truth(text.get(start..end).unwrap_or(""), view);
            match mark.directive() {
                ConditionalDirective::Open => {
                    let parent = live(&groups);
                    let condition = if parent { truth() } else { Truth::False };
                    let kept = parent && condition != Truth::False;
                    let first_kept = kept.then_some(condition == Truth::True);
                    if first_kept == Some(false) {
                        kept_marks.push((kept_tokens.len(), ConditionalDirective::Open));
                    }
                    groups.push(Narrowing {
                        live: parent,
                        decided: condition == Truth::True,
                        kept,
                        first_kept,
                    });
                }
                directive @ (ConditionalDirective::Alternative
                | ConditionalDirective::Otherwise) => {
                    let Some(group) = groups.last_mut() else {
                        // A stray arm: leave it to the structural scan.
                        kept_marks.push((kept_tokens.len(), directive));
                        continue;
                    };
                    if !group.live {
                        continue;
                    }
                    let condition = if group.decided { Truth::False } else { truth() };
                    group.kept = condition != Truth::False;
                    if group.kept {
                        match group.first_kept {
                            None => {
                                group.first_kept = Some(condition == Truth::True);
                                if condition != Truth::True {
                                    kept_marks
                                        .push((kept_tokens.len(), ConditionalDirective::Open));
                                }
                            }
                            Some(_) => kept_marks.push((kept_tokens.len(), directive)),
                        }
                    }
                    group.decided |= condition == Truth::True;
                }
                ConditionalDirective::Close => match groups.pop() {
                    Some(group) => {
                        if group.first_kept == Some(false) {
                            kept_marks.push((kept_tokens.len(), ConditionalDirective::Close));
                        }
                    }
                    None => kept_marks.push((kept_tokens.len(), ConditionalDirective::Close)),
                },
            }
        }
        if index < tokens.len() && live(&groups) {
            kept_tokens.push(tokens[index].clone());
        }
    }
    (kept_tokens, kept_marks)
}

/// One file of the program.
#[derive(Clone, Debug)]
pub struct CensusFile {
    pub path: String,
    pub text: String,
}

struct Definition {
    file: usize,
    name: String,
    census_name: String,
    /// The name token, the body's `{` and its `}`.
    header: usize,
    body: (usize, usize),
    /// Parameter names, which shadow a function of the same name inside the
    /// definition.
    parameters: BTreeSet<String>,
    /// The first parameter's name, when the list has one (Step B).
    first_parameter: Option<String>,
}

struct Scan {
    tokens: Vec<Token>,
    definitions: Vec<usize>,
    /// Token indices of definition names, which are not uses.
    headers: BTreeSet<usize>,
    /// Token indices of file-scope declarations' names.
    declarations: BTreeSet<usize>,
    unstructured: bool,
    /// Card W47-SERVICE-PROPERTY-KEYS: the token positions a conditional
    /// directive the reading kept stands at (an undecided group's arms).
    directives: Vec<usize>,
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

fn is_callable_name(text: &str) -> bool {
    is_identifier(text) && !KEYWORDS.contains(&text)
}

/// Find file-scope definitions and declarations, reading conditional groups
/// as the raw enumeration does: every arm is scanned from the group's entry
/// depth, and a group whose arms end at different depths is unstructured.
fn scan_file(
    file: usize,
    path: &str,
    text: &str,
    definitions: &mut Vec<Definition>,
    view: Option<&MacroView<'_>>,
) -> Scan {
    let (tokens, raw_marks) = tokenize_with_conditionals(text);
    let (tokens, marks) = match view {
        Some(view) => configure(tokens, &raw_marks, text, view),
        None => {
            let marks = raw_marks
                .iter()
                .map(|mark| (mark.token_index(), mark.directive()))
                .collect::<Vec<_>>();
            (tokens, marks)
        }
    };
    let mut scan = Scan {
        tokens: Vec::new(),
        definitions: Vec::new(),
        headers: BTreeSet::new(),
        declarations: BTreeSet::new(),
        unstructured: false,
        directives: marks.iter().map(|(index, _)| *index).collect(),
    };
    let mut depth = 0usize;
    let mut frames: Vec<(usize, Vec<usize>, bool)> = Vec::new();
    let mut next_mark = 0;
    let mut open: Option<(usize, usize)> = None; // (name token, body open)
    let mut linkage: Vec<usize> = Vec::new();
    let mut statement_start = 0usize;
    let mut index = 0;
    while index < tokens.len() {
        while next_mark < marks.len() && marks[next_mark].0 <= index {
            apply_mark(
                marks[next_mark].1,
                &mut depth,
                &mut frames,
                &mut scan.unstructured,
            );
            next_mark += 1;
        }
        let text = tokens[index].text.as_str();
        match text {
            "{" if depth == 0
                && index >= 2
                && tokens[index - 2].text == "extern"
                && tokens[index - 1].text.starts_with("__qcore_literal") =>
            {
                // An `extern "C" {` linkage block is not a scope.
                linkage.push(depth);
            }
            "}" if depth == 0 && linkage.last() == Some(&depth) => {
                linkage.pop();
                statement_start = index + 1;
            }
            "{" => {
                if depth == 0
                    && let Some((name, _)) = open.take().filter(|(_, body)| *body == index)
                {
                    open = Some((name, index));
                } else if depth == 0 {
                    open = None;
                }
                depth += 1;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some((name, body)) = open.take()
                        && tokens[body].text == "{"
                    {
                        let is_static = tokens[statement_start..name]
                            .iter()
                            .any(|token| token.text == "static");
                        let name_text = tokens[name].text.clone();
                        let census_name = if is_static {
                            format!("{path}::{name_text}")
                        } else {
                            name_text.clone()
                        };
                        scan.definitions.push(definitions.len());
                        scan.headers.insert(name);
                        definitions.push(Definition {
                            file,
                            name: name_text,
                            census_name,
                            header: name,
                            body: (body, index),
                            parameters: parameter_names(&tokens, name + 1, body),
                            first_parameter: first_parameter_name(&tokens, name + 1),
                        });
                    }
                    statement_start = index + 1;
                }
            }
            ";" if depth == 0 => {
                open = None;
                statement_start = index + 1;
            }
            _ if depth == 0
                && open.is_none()
                && is_callable_name(text)
                && tokens.get(index + 1).is_some_and(|next| next.text == "(") =>
            {
                if let Some(close) = crate::legacy_c::matching(&tokens, index + 1, "(", ")") {
                    // Skip attribute lists and K&R-free trailers up to `{` or `;`.
                    let after = close + 1;
                    match tokens.get(after).map(|token| token.text.as_str()) {
                        Some("{") => open = Some((index, after)),
                        _ => {
                            scan.declarations.insert(index);
                        }
                    }
                }
            }
            _ => {}
        }
        index += 1;
    }
    // A directive after the last token (an include guard's closing `#endif`)
    // stands at `tokens.len()`, which the token loop never reaches: it closes
    // its group here (card W47-LOOP-OWNER-COROUTINE-HOP; before, every such
    // file was read as unstructured).
    for (_, directive) in &marks[next_mark..] {
        apply_mark(*directive, &mut depth, &mut frames, &mut scan.unstructured);
    }
    if !frames.is_empty() || depth != 0 {
        scan.unstructured = true;
    }
    scan.tokens = tokens;
    scan
}

/// One conditional directive's effect on the raw scan's brace depth: every arm
/// is scanned from the group's entry depth, and a group whose arms end at
/// different depths is unstructured.
fn apply_mark(
    directive: ConditionalDirective,
    depth: &mut usize,
    frames: &mut Vec<(usize, Vec<usize>, bool)>,
    unstructured: &mut bool,
) {
    match directive {
        ConditionalDirective::Open => frames.push((*depth, Vec::new(), false)),
        ConditionalDirective::Alternative | ConditionalDirective::Otherwise => {
            if let Some(frame) = frames.last_mut() {
                frame.1.push(*depth);
                frame.2 |= directive == ConditionalDirective::Otherwise;
                *depth = frame.0;
            } else {
                *unstructured = true;
            }
        }
        ConditionalDirective::Close => {
            if let Some((entry, mut arms, saw_else)) = frames.pop() {
                arms.push(*depth);
                if !saw_else {
                    arms.push(entry);
                }
                if arms.iter().any(|arm| *arm != arms[0]) {
                    *unstructured = true;
                }
                *depth = arms.into_iter().max().unwrap_or(entry);
            } else {
                *unstructured = true;
            }
        }
    }
}

/// The names a parameter list declares: an identifier directly before `,`,
/// `)` or `[` at the list's own depth, or a function pointer's `(*NAME)`.
fn parameter_names(tokens: &[Token], open: usize, end: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    for index in open..end {
        match tokens[index].text.as_str() {
            "(" => depth += 1,
            ")" => depth = depth.saturating_sub(1),
            text if is_identifier(text) => {
                let next = tokens.get(index + 1).map_or("", |t| t.text.as_str());
                let previous = tokens[index - 1].text.as_str();
                if (depth == 1 && matches!(next, "," | ")" | "["))
                    || (previous == "*" && next == ")")
                {
                    names.insert(text.to_owned());
                }
            }
            _ => {}
        }
    }
    names
}

/// The first parameter's name: the identifier directly before the list's
/// first `,` or its `)` (a plain declarator only; a function-pointer or array
/// parameter gives none).
fn first_parameter_name(tokens: &[Token], open: usize) -> Option<String> {
    let close = crate::legacy_c::matching(tokens, open, "(", ")")?;
    let mut depth = 0usize;
    for index in open + 1..=close {
        match tokens[index].text.as_str() {
            "(" | "[" => depth += 1,
            ")" | "]" if depth > 0 => depth -= 1,
            "," | ")" if depth == 0 => {
                let name = tokens.get(index.checked_sub(1)?)?.text.as_str();
                return (is_identifier(name) && index - 1 > open && name != "void")
                    .then(|| name.to_owned());
            }
            _ => {}
        }
    }
    None
}

/// The names written in `#define` replacement lists (the tokenizer skips
/// directives, so they are read here from the raw text).
fn macro_names(text: &str) -> BTreeMap<String, usize> {
    let mut names = BTreeMap::new();
    let mut logical = String::new();
    let mut start_line = 0;
    for (number, line) in text.lines().enumerate() {
        if logical.is_empty() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') || !trimmed[1..].trim_start().starts_with("define") {
                continue;
            }
            start_line = number + 1;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            logical.push_str(stripped);
            logical.push(' ');
            continue;
        }
        logical.push_str(line);
        let body = logical.trim_start().trim_start_matches('#').trim_start();
        let body = body.strip_prefix("define").unwrap_or(body);
        let mut word = String::new();
        let mut first = true;
        for character in body.chars().chain(std::iter::once(' ')) {
            if character.is_ascii_alphanumeric() || character == '_' {
                word.push(character);
            } else {
                if !word.is_empty() {
                    // The first word is the macro's own name.
                    if !first {
                        names.entry(word.clone()).or_insert(start_line);
                    }
                    first = false;
                    word.clear();
                }
                if first && character == '(' {
                    first = false;
                }
            }
        }
        logical.clear();
    }
    names
}

fn arguments(tokens: &[Token], open: usize) -> Option<(Vec<(usize, usize)>, usize)> {
    let close = crate::legacy_c::matching(tokens, open, "(", ")")?;
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = open + 1;
    for (index, token) in tokens.iter().enumerate().take(close).skip(open + 1) {
        match token.text.as_str() {
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
    Some((ranges, close))
}

/// The end (inclusive) of the statement or block starting at `start`.
fn statement_end(tokens: &[Token], start: usize, limit: usize) -> usize {
    if tokens.get(start).is_some_and(|token| token.text == "{") {
        return crate::legacy_c::matching(tokens, start, "{", "}").unwrap_or(limit);
    }
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

/// Token ranges of a body in which the threaded flag is fixed (hop F).
fn mode_ranges(
    tokens: &[Token],
    body: (usize, usize),
    predicate: &str,
) -> Vec<(usize, usize, ReadMode)> {
    let mut ranges = Vec::new();
    for index in body.0..body.1 {
        if tokens[index].text != "if" || tokens.get(index + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        let Some(close) = crate::legacy_c::matching(tokens, index + 1, "(", ")") else {
            continue;
        };
        let condition = tokens[index + 2..close]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let (negated, rest) = match condition.as_slice() {
            ["!", rest @ ..] => (true, rest),
            rest => (false, rest),
        };
        let fixed = match rest {
            [name, "(", argument, ")"] if *name == predicate && is_identifier(argument) => {
                if negated {
                    ReadMode::NonThreaded
                } else {
                    ReadMode::Threaded
                }
            }
            _ => continue,
        };
        let then_end = statement_end(tokens, close + 1, body.1);
        ranges.push((close + 1, then_end, fixed));
        if tokens.get(then_end + 1).is_some_and(|t| t.text == "else") {
            let else_end = statement_end(tokens, then_end + 2, body.1);
            let other = if fixed == ReadMode::Threaded {
                ReadMode::NonThreaded
            } else {
                ReadMode::Threaded
            };
            ranges.push((then_end + 2, else_end, other));
        }
    }
    ranges
}

/// A file's `#define`s (name, names the replacement list writes) and
/// `#undef`s.
type MacroDefinitions = (Vec<(String, BTreeSet<String>)>, BTreeSet<String>);

/// Every `#define` in a file (its name and the names its replacement list
/// writes) and every `#undef`'s name.
fn macro_definitions(text: &str) -> MacroDefinitions {
    let mut defines = Vec::new();
    let mut undefs = BTreeSet::new();
    let mut logical = String::new();
    for line in text.lines() {
        if logical.is_empty() {
            let trimmed = line.trim_start();
            let Some(directive) = trimmed.strip_prefix('#') else {
                continue;
            };
            let directive = directive.trim_start();
            if let Some(name) = directive.strip_prefix("undef") {
                let name = name.trim();
                if is_identifier(name) {
                    undefs.insert(name.to_owned());
                }
                continue;
            }
            if !directive.starts_with("define") {
                continue;
            }
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            logical.push_str(stripped);
            logical.push(' ');
            continue;
        }
        logical.push_str(line);
        // Comments are not replacement text.
        while let Some(start) = logical.find("/*") {
            let end = logical[start + 2..]
                .find("*/")
                .map_or(logical.len(), |e| start + 2 + e + 2);
            logical.replace_range(start..end, " ");
        }
        if let Some(start) = logical.find("//") {
            logical.truncate(start);
        }
        let body = logical.trim_start().trim_start_matches('#').trim_start();
        let body = body.strip_prefix("define").unwrap_or(body);
        let words = body
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|word| is_identifier(word))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if let Some((name, rest)) = words.split_first() {
            defines.push((name.clone(), rest.iter().cloned().collect()));
        }
        logical.clear();
    }
    (defines, undefs)
}

/// Read one program's files into a [`CensusReading`], every conditional arm
/// included.
#[must_use]
pub fn read_program(files: &[CensusFile], hops: &HopTables) -> CensusReading {
    read_program_configured(files, hops, None)
}

/// Read one program's files under a build configuration, when one is given:
/// an arm the configuration decides is not taken is not read.
#[must_use]
pub fn read_program_configured(
    files: &[CensusFile],
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
) -> CensusReading {
    read_program_typed(files, hops, configuration, None)
}

/// What one lexical registration is keyed by (card
/// W47-LOOP-OWNER-TYPED-DISPATCH): the record the typed reading stored it in,
/// or [`Keyed::Any`] when no typed fact names the site (the field-keyed
/// reading: every dispatch of the field reaches it).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Keyed {
    Any,
    Record(String),
}

/// How one lexical field dispatch is typed.
#[derive(Clone, Debug)]
enum Typing {
    /// No typed fact: every registrant of the field.
    Untyped,
    /// Only registrants keyed by one of these records (or by none).
    Records(BTreeSet<String>),
    /// A cast, a union, an anonymous record: no registrant is attributed, and
    /// every registrant of the field is unresolved.
    Unmatched(BTreeSet<String>),
}

fn dispatch_typing(
    typed: Option<&crate::typed_dispatch_census::TypedFacts>,
    path: &str,
    line: usize,
    field: &str,
) -> Typing {
    use crate::typed_dispatch_census::{TypedRecord, dispatch_key};
    let Some(records) = typed.and_then(|t| t.dispatches.get(&dispatch_key(path, line, field)))
    else {
        return Typing::Untyped;
    };
    let unmatched = records
        .iter()
        .filter_map(|record| match record {
            TypedRecord::Unmatched(why) => Some(why.clone()),
            TypedRecord::Record(_) => None,
        })
        .collect::<BTreeSet<_>>();
    if !unmatched.is_empty() {
        return Typing::Unmatched(unmatched);
    }
    Typing::Records(
        records
            .iter()
            .filter_map(|record| match record {
                TypedRecord::Record(key) => Some(key.clone()),
                TypedRecord::Unmatched(_) => None,
            })
            .collect(),
    )
}

/// The keys of one lexical store `x->F = f` / `.F = f`, and why it is
/// unmatched when it is.
fn store_keys(
    typed: Option<&crate::typed_dispatch_census::TypedFacts>,
    path: &str,
    line: usize,
    field: &str,
    function: &str,
) -> (BTreeSet<Keyed>, BTreeSet<String>) {
    use crate::typed_dispatch_census::{TypedRecord, store_key};
    let Some(typed) = typed else {
        return (BTreeSet::from([Keyed::Any]), BTreeSet::new());
    };
    let records = [field, ""]
        .iter()
        .filter_map(|f| typed.stores.get(&store_key(path, line, f, function)))
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut keys = BTreeSet::new();
    let mut unmatched = BTreeSet::new();
    for record in records {
        match record {
            TypedRecord::Record(key) => {
                keys.insert(Keyed::Record(key.clone()));
            }
            TypedRecord::Unmatched(why) => {
                unmatched.insert(why.clone());
            }
        }
    }
    if keys.is_empty() || !unmatched.is_empty() {
        keys.insert(Keyed::Any);
    }
    (keys, unmatched)
}

/// [`read_program_configured`] with typed dispatch (card
/// W47-LOOP-OWNER-TYPED-DISPATCH): a field dispatch reaches only the
/// registrants stored in the same record type, when the typed reading names
/// both. Without typed facts the reading is exactly the field-keyed one.
#[must_use]
pub fn read_program_typed(
    files: &[CensusFile],
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
    typed: Option<&crate::typed_dispatch_census::TypedFacts>,
) -> CensusReading {
    read_program_premised(files, hops, configuration, typed, None)
}

/// [`read_program_typed`] as the premised census (card
/// W47-PREMISES-INSTANCE) when `premises` is given: Step B's list-head
/// premises are derived or its flag is underived, and P-INSTANCE is reported
/// per listed collector operation ([`CensusReading::premises`]).
#[must_use]
pub fn read_program_premised(
    files: &[CensusFile],
    hops: &HopTables,
    configuration: Option<&BuildConfiguration>,
    typed: Option<&crate::typed_dispatch_census::TypedFacts>,
    premises: Option<&PremiseInputs>,
) -> CensusReading {
    let macros = files
        .iter()
        .map(|source| macro_definitions(&source.text))
        .collect::<Vec<_>>();
    let written = macros
        .iter()
        .flat_map(|(defines, _)| defines.iter().map(|(name, _)| name.clone()))
        .chain(
            configuration
                .into_iter()
                .flat_map(|configuration| configuration.defined_elsewhere.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let undefined_somewhere = macros
        .iter()
        .flat_map(|(_, undefs)| undefs.iter().cloned())
        .collect::<BTreeSet<_>>();
    let (common, any_unit) = configuration.map_or_else(
        || (BTreeSet::new(), BTreeSet::new()),
        |configuration| {
            let mut units = configuration.units.values();
            let common = units.next().map_or_else(BTreeSet::new, |first| {
                units.fold(first.clone(), |acc, unit| {
                    acc.intersection(unit).cloned().collect()
                })
            });
            let any = configuration
                .units
                .values()
                .flat_map(|unit| unit.iter().cloned())
                .collect::<BTreeSet<_>>();
            (common, any)
        },
    );
    let mut definitions = Vec::new();
    let scans = files
        .iter()
        .enumerate()
        .map(|(file, source)| {
            let view = configuration.map(|configuration| {
                let unit = configuration.units.get(&source.path);
                MacroView {
                    defined: unit.unwrap_or(&common),
                    defined_by_some_unit: unit.unwrap_or(&any_unit),
                    written: &written,
                    undefined_somewhere: &undefined_somewhere,
                    owned_prefixes: &configuration.owned_prefixes,
                    path: &source.path,
                    predefined: configuration.predefined.as_ref(),
                    decisions: std::cell::RefCell::default(),
                }
            });
            let scan = scan_file(
                file,
                &source.path,
                &source.text,
                &mut definitions,
                view.as_ref(),
            );
            let decisions = view
                .map(|view| view.decisions.into_inner())
                .unwrap_or_default();
            (scan, decisions)
        })
        .collect::<Vec<_>>();
    let mut program = CensusReading::default();
    let scans = scans
        .into_iter()
        .zip(files)
        .map(|((scan, decisions), source)| {
            if !decisions.is_empty() {
                program
                    .predefined_decisions
                    .insert(source.path.clone(), decisions);
            }
            scan
        })
        .collect::<Vec<_>>();
    let mut index = BTreeMap::new();
    for definition in &definitions {
        if !index.contains_key(&definition.census_name) {
            index.insert(definition.census_name.clone(), program.functions.len());
            program.functions.push(definition.census_name.clone());
        }
    }
    let plain_name = |id: usize| {
        let label = &program.functions[id];
        label
            .rsplit_once("::")
            .map_or(label.as_str(), |(_, name)| name)
            .to_owned()
    };
    // Name resolution: a static of the same file, else every static header
    // definition of the name, else the external definition.
    let mut by_name: BTreeMap<&str, Vec<&Definition>> = BTreeMap::new();
    for definition in &definitions {
        by_name
            .entry(&definition.name)
            .or_default()
            .push(definition);
    }
    let resolve = |file: usize, name: &str| -> Vec<usize> {
        let Some(candidates) = by_name.get(name) else {
            return Vec::new();
        };
        let own = candidates
            .iter()
            .filter(|d| d.file == file && d.census_name.contains("::"))
            .map(|d| index[&d.census_name])
            .collect::<Vec<_>>();
        if !own.is_empty() {
            return own;
        }
        let mut found = candidates
            .iter()
            .filter(|d| !d.census_name.contains("::") || files[d.file].path.ends_with(".h"))
            .map(|d| index[&d.census_name])
            .collect::<Vec<_>>();
        found.sort_unstable();
        found.dedup();
        found
    };
    let mut unresolved: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut escape = |ids: &[usize], reason: String| {
        for id in ids {
            unresolved.entry(*id).or_default().insert(reason.clone());
        }
    };
    // field -> registrant -> what its registrations are keyed by.
    let mut registrants: BTreeMap<String, BTreeMap<usize, BTreeSet<Keyed>>> = BTreeMap::new();
    let collector_keys = typed.map_or_else(
        || BTreeSet::from([Keyed::Any]),
        |typed| {
            if typed.parameter_stores.iter().any(|store| {
                store.record == hops.collector_record && store.field == hops.collector_field
            }) {
                BTreeSet::from([Keyed::Record(hops.collector_record.to_owned())])
            } else {
                BTreeSet::from([Keyed::Any])
            }
        },
    );
    // (caller, field, effect, file, position of the field name, typing)
    let mut dispatches: Vec<(usize, String, ReadEffect, usize, usize, Typing)> = Vec::new();
    // Step B: (registrant, registering function, instance argument, file).
    let mut collector_registrations: Vec<(usize, usize, Option<String>, usize)> = Vec::new();
    // Step B: (stored function, field, file, position of the field name).
    let mut field_stores: Vec<(usize, String, usize, usize)> = Vec::new();
    // Step B: the functions storing the instance's coroutine flag.
    let mut flag_writers: BTreeSet<Option<usize>> = BTreeSet::new();
    // Step B: descriptor object -> its coroutine flag.
    let mut descriptor_objects: BTreeMap<String, Option<bool>> = BTreeMap::new();
    // Step B: functions whose address is taken or named in a macro.
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    let mut poisoned_fields: BTreeMap<String, String> = BTreeMap::new();
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: every site a field is read as a value.
    let mut poison_sites: Vec<(String, String, usize)> = Vec::new();
    let mut edges = BTreeSet::new();
    // Direct edges with where they were read, so hop C can place an entry's
    // calls before or after its hand-off.
    let mut positioned: Vec<(usize, usize, ReadEdge)> = Vec::new();
    let mut spawns = BTreeSet::new();
    let mut roots = BTreeSet::new();
    let mut calls: Vec<ReadCall> = Vec::new();
    // Hop C.
    let numeric_macros = numeric_macros(&macros);
    let plugin_dir = |path: &str| {
        path.strip_prefix(hops.plugin_root)
            .and_then(|rest| rest.split_once('/'))
            .map(|(dir, _)| dir.to_owned())
    };
    let mut descriptor_flags: BTreeMap<String, Vec<Option<bool>>> = BTreeMap::new();
    let mut collector_sites: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    // entry -> the constructors that create it.
    let mut entries: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    // (resumer, coroutine hop, handle variable, path, line, effect)
    let mut constructor_calls: Vec<(usize, usize, Option<String>, String, usize, ReadEffect)> =
        Vec::new();
    // (caller, coroutine hop, handle variable, path, line)
    let mut resume_calls: Vec<(usize, usize, Option<String>, String, usize)> = Vec::new();
    // (caller, file, position): `switch(x->RESUMER_FIELD)`.
    let mut switch_backs: Vec<(usize, usize, usize)> = Vec::new();

    for (file, scan) in scans.iter().enumerate() {
        let path = &files[file].path;
        let tokens = &scan.tokens;
        if scan.unstructured {
            for definition in &scan.definitions {
                let id = index[&definitions[*definition].census_name];
                escape(
                    &[id],
                    format!("{path}: a conditional group this reader cannot structure"),
                );
            }
        }
        for (name, line) in macro_names(&files[file].text) {
            let ids = resolve(file, &name);
            if !ids.is_empty() {
                taken.extend(ids.iter().copied());
                escape(&ids, format!("named in a macro at {path}:{line}"));
            }
        }
        // Which definition each token belongs to.
        let mut owner_of = vec![None; tokens.len()];
        let mut shadow_of: Vec<Option<&BTreeSet<String>>> = vec![None; tokens.len()];
        let mut modes: Vec<(usize, usize, ReadMode)> = Vec::new();
        for definition in &scan.definitions {
            let definition = &definitions[*definition];
            let id = index[&definition.census_name];
            for slot in &mut owner_of[definition.body.0..=definition.body.1] {
                *slot = Some(id);
            }
            for slot in &mut shadow_of[definition.header + 1..=definition.body.1] {
                *slot = Some(&definition.parameters);
            }
            modes.extend(mode_ranges(
                tokens,
                definition.body,
                hops.threaded_predicate,
            ));
        }
        let mode_at = |position: usize| {
            modes
                .iter()
                .filter(|(start, end, _)| *start <= position && position <= *end)
                .map(|(_, _, mode)| *mode)
                .collect::<BTreeSet<_>>()
        };
        let mut consumed = BTreeSet::new();
        // The test table (hop A).
        for position in 0..tokens.len() {
            if tokens[position].text != hops.test_table
                || tokens.get(position + 1).is_none_or(|t| t.text != "=")
                || tokens.get(position + 2).is_none_or(|t| t.text != "{")
            {
                continue;
            }
            let Some(end) = crate::legacy_c::matching(tokens, position + 2, "{", "}") else {
                continue;
            };
            for entry in position + 3..end {
                // `{ "name", f }`
                if tokens[entry].text == "{"
                    && tokens.get(entry + 2).is_some_and(|t| t.text == ",")
                    && tokens.get(entry + 4).is_some_and(|t| t.text == "}")
                {
                    for id in resolve(file, &tokens[entry + 3].text) {
                        roots.insert((id, hops.test_thread.to_owned()));
                        consumed.insert(entry + 3);
                    }
                }
            }
        }
        // The tokenizer emits every punctuator byte on its own: `->` is `-`
        // `>`, `==` is `=` `=`, `&&` is `&` `&`.
        let at = |position: usize| tokens.get(position).map_or("", |t| t.text.as_str());
        // Input-plugin descriptors and their coroutine flag (hop C).
        if let Some(dir) = plugin_dir(path) {
            for position in 0..tokens.len() {
                if at(position) != "struct"
                    || at(position + 1) != hops.plugin_descriptor
                    || !is_identifier(at(position + 2))
                    || at(position + 3) != "="
                    || at(position + 4) != "{"
                {
                    continue;
                }
                let Some(end) = crate::legacy_c::matching(tokens, position + 4, "{", "}") else {
                    continue;
                };
                let flag = descriptor_flag(tokens, position + 4, end, hops, &numeric_macros);
                descriptor_flags.entry(dir.clone()).or_default().push(flag);
                // Step B: the same object name twice (two builds' variants)
                // keeps only a flag both agree on.
                descriptor_objects
                    .entry(at(position + 2).to_owned())
                    .and_modify(|seen| {
                        if *seen != flag {
                            *seen = None;
                        }
                    })
                    .or_insert(flag);
            }
        }
        for position in 0..tokens.len() {
            let text = tokens[position].text.as_str();
            let previous = position.checked_sub(1).map_or("", at);
            let before_previous = position.checked_sub(2).map_or("", at);
            let member = previous == "." || (previous == ">" && before_previous == "-");
            // A field read, store or dispatch.
            if member && is_identifier(text) {
                let next = tokens.get(position + 1).map(|t| t.text.as_str());
                let after_next = at(position + 2);
                let next = match (next, after_next) {
                    (Some("=" | "!"), "=") => Some("=="),
                    (Some("&"), "&") | (Some("|"), "|") => Some("&&"),
                    (Some("-"), ">") => Some("->"),
                    (Some("+"), "+") | (Some("-"), "-") => Some("++"),
                    (other, _) => other,
                };
                match next {
                    Some("(") => {
                        if let Some(caller) = owner_of[position] {
                            let caller_name = plain_name(caller);
                            let effect = hops
                                .loop_dispatchers
                                .iter()
                                .find(|(dispatcher, _)| {
                                    *dispatcher == caller_name && text == hops.collector_field
                                })
                                .map_or(ReadEffect::Keep, |(_, mode)| ReadEffect::Dispatch(*mode));
                            let typing = dispatch_typing(typed, path, tokens[position].line, text);
                            dispatches.push((
                                caller,
                                text.to_owned(),
                                effect,
                                file,
                                position,
                                typing,
                            ));
                        }
                    }
                    Some("=") => {
                        let value = tokens.get(position + 2).map(|t| t.text.as_str());
                        let end = tokens.get(position + 3).map(|t| t.text.as_str());
                        if let Some(value) = value
                            && matches!(end, Some(";" | "," | "}"))
                        {
                            let ids = resolve(file, value);
                            if !ids.is_empty() {
                                if text == hops.collector_field {
                                    for id in &ids {
                                        collector_sites
                                            .entry(*id)
                                            .or_default()
                                            .insert(path.clone());
                                    }
                                }
                                let line = tokens[position + 2].line;
                                let (keys, unmatched) = store_keys(typed, path, line, text, value);
                                for why in unmatched {
                                    escape(
                                        &ids,
                                        format!(
                                            "stored in `{text}` at {path}:{line} through an \
                                             unmatched record type ({why})"
                                        ),
                                    );
                                }
                                let slot = registrants.entry(text.to_owned()).or_default();
                                for id in &ids {
                                    slot.entry(*id).or_default().extend(keys.iter().cloned());
                                    field_stores.push((*id, text.to_owned(), file, position));
                                }
                                consumed.insert(position + 2);
                            }
                        }
                        if text == hops.instance_flag_field {
                            flag_writers.insert(owner_of[position]);
                        }
                    }
                    Some(_) if text == hops.instance_flag_field => {
                        // Any other write form (`|=`, `++`, …) or a read.
                        if !matches!(next, Some("==" | "&&" | ")" | ";" | "?")) {
                            flag_writers.insert(None);
                        }
                    }
                    Some("==" | "&&" | "?" | "[" | "." | "->" | "++") | None => {}
                    Some(")") if closes_a_test(tokens, position + 1) => {}
                    Some(_) => {
                        // The field's value is read into something other than
                        // a call or a test: its registrants may be called from
                        // anywhere that value reaches.
                        let line = tokens[position].line;
                        poisoned_fields
                            .entry(text.to_owned())
                            .or_insert_with(|| format!("{path}:{line}"));
                        poison_sites.push((text.to_owned(), path.clone(), line));
                    }
                }
                continue;
            }
            if !is_callable_name(text) || consumed.contains(&position) {
                continue;
            }
            // A parameter of the same name shadows the function.
            if shadow_of[position].is_some_and(|names| names.contains(text)) {
                continue;
            }
            if scan.headers.contains(&position) || scan.declarations.contains(&position) {
                continue;
            }
            let next = tokens.get(position + 1).map(|t| t.text.as_str());
            let address_of = previous == "&" && before_previous != "&";
            if next == Some("(") && !address_of {
                let Some(caller) = owner_of[position] else {
                    continue;
                };
                let Some((ranges, _)) = arguments(tokens, position + 1) else {
                    continue;
                };
                let line = tokens[position].line;
                // Spawns (hop S) and collector registrations (hop R).
                let spawn = hops.spawns.iter().find(|(name, _)| *name == text);
                let registration = hops
                    .collector_registrations
                    .iter()
                    .find(|(name, _)| *name == text);
                for (table_hit, is_spawn) in [(spawn, true), (registration, false)] {
                    let Some((_, argument)) = table_hit else {
                        continue;
                    };
                    if let Some((start, end)) = ranges.get(*argument)
                        && end - start == 1
                    {
                        let ids = resolve(file, &tokens[*start].text);
                        if !ids.is_empty() {
                            consumed.insert(*start);
                            for id in ids {
                                if is_spawn {
                                    spawns.insert((caller, id));
                                } else {
                                    collector_sites.entry(id).or_default().insert(path.clone());
                                    registrants
                                        .entry(hops.collector_field.to_owned())
                                        .or_default()
                                        .entry(id)
                                        .or_default()
                                        .extend(collector_keys.iter().cloned());
                                    let instance = ranges
                                        .first()
                                        .filter(|(start, end)| end - start == 1)
                                        .map(|(start, _)| tokens[*start].text.clone());
                                    collector_registrations.push((id, caller, instance, file));
                                }
                            }
                        }
                    }
                }
                // Hop C: a listed constructor names its entry at the
                // coroutine primitive; the entry's address is then consumed.
                let caller_name = plain_name(caller);
                if text == hops.coroutine_primitive.0
                    && hops
                        .coroutines
                        .iter()
                        .any(|hop| hop.constructor == caller_name)
                    && let Some((start, end)) = ranges.get(hops.coroutine_primitive.1)
                    && end - start == 1
                {
                    let ids = resolve(file, &tokens[*start].text);
                    if !ids.is_empty() {
                        consumed.insert(*start);
                        for id in ids {
                            entries.entry(id).or_default().insert(caller);
                        }
                    }
                }
                if text == hops.switch_primitive
                    && let Some((start, end)) = ranges.first()
                {
                    let argument = tokens[*start..*end]
                        .iter()
                        .map(|t| t.text.as_str())
                        .collect::<Vec<_>>();
                    if matches!(argument.as_slice(),
                        [base, "-", ">", field] | [base, ".", field]
                            if is_identifier(base) && *field == hops.resumer_field)
                    {
                        switch_backs.push((caller, file, position));
                    }
                }
                let handle = |range: Option<&(usize, usize)>| {
                    range
                        .filter(|(start, end)| end - start == 1)
                        .map(|(start, _)| tokens[*start].text.clone())
                        .filter(|name| is_identifier(name))
                };
                let fixed = mode_at(position);
                let effect = match fixed.iter().collect::<Vec<_>>().as_slice() {
                    [] => ReadEffect::Keep,
                    [mode] => ReadEffect::Require(**mode),
                    // Nested contradictory branches: this call is unreachable.
                    _ => continue,
                };
                if let Some(hop) = hops
                    .coroutines
                    .iter()
                    .position(|hop| hop.constructor == text)
                {
                    let variable = (at(position.wrapping_sub(1)) == "="
                        && at(position.wrapping_sub(2)) != "="
                        && is_identifier(at(position.wrapping_sub(2))))
                    .then(|| at(position - 2).to_owned());
                    let dispatch = hops
                        .loop_dispatchers
                        .iter()
                        .find(|(dispatcher, _)| *dispatcher == caller_name)
                        .map_or(effect, |(_, mode)| ReadEffect::Dispatch(*mode));
                    constructor_calls.push((caller, hop, variable, path.clone(), line, dispatch));
                }
                if let Some(hop) = hops.coroutines.iter().position(|hop| hop.resume == text) {
                    resume_calls.push((caller, hop, handle(ranges.first()), path.clone(), line));
                }
                for callee in resolve(file, text) {
                    positioned.push((
                        file,
                        position,
                        ReadEdge {
                            caller,
                            callee,
                            hop: ReadHop::Direct,
                            effect,
                        },
                    ));
                }
                calls.push(ReadCall {
                    file: path.clone(),
                    line,
                    caller,
                    callee: text.to_owned(),
                });
                // Arguments are scanned in their own positions.
                continue;
            }
            // Any other occurrence of a defined function's name escapes.
            let ids = resolve(file, text);
            if !ids.is_empty() {
                taken.extend(ids.iter().copied());
                let line = tokens[position].line;
                escape(&ids, format!("address of `{text}` taken at {path}:{line}"));
            }
        }
    }
    // Hop C: each entry's hand-off, its single switch back to the resumer
    // outside any loop. Its calls up to the switch run in the constructor.
    let mut handoffs: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for entry in entries.keys() {
        let switches = switch_backs
            .iter()
            .filter(|(caller, ..)| caller == entry)
            .collect::<Vec<_>>();
        let looped = definitions
            .iter()
            .filter(|d| index[&d.census_name] == *entry)
            .any(|d| {
                scans[d.file].tokens[d.body.0..=d.body.1]
                    .iter()
                    .any(|t| matches!(t.text.as_str(), "while" | "for" | "do" | "goto"))
            });
        match switches.as_slice() {
            [(_, file, position)] if !looped => {
                handoffs.insert(*entry, (*file, *position));
            }
            _ => escape(
                &[*entry],
                format!(
                    "the hand-off of coroutine entry `{}` cannot be read (one switch back to \
                     its resumer, outside any loop)",
                    program.functions[*entry]
                ),
            ),
        }
    }
    let placed = |caller: usize, file: usize, position: usize| -> (Vec<usize>, bool) {
        match handoffs.get(&caller) {
            Some((handoff_file, handoff)) if *handoff_file == file && position <= *handoff => {
                (entries[&caller].iter().copied().collect(), false)
            }
            Some(_) => (vec![caller], true),
            None => (vec![caller], false),
        }
    };
    let mut bodies: BTreeSet<(usize, usize)> = BTreeSet::new();
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: the unplaced direct calls.
    let raw_direct = positioned
        .iter()
        .map(|(_, _, edge)| (edge.caller, edge.callee))
        .collect::<Vec<_>>();
    for (file, position, edge) in positioned {
        let (callers, body) = placed(edge.caller, file, position);
        for caller in callers {
            edges.insert(ReadEdge { caller, ..edge });
        }
        if body {
            bodies.insert((edge.caller, edge.callee));
        }
    }
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: the census graph as reachability for
    // Step B's list-head derivation, whatever the coroutine route.
    let reach = instance_reach(&InstanceReachInputs {
        definitions: &definitions,
        index: &index,
        scans: &scans,
        raw_direct: &raw_direct,
        dispatches: &dispatches,
        registrants: &registrants,
        spawns: &spawns,
        entries: &entries,
        constructor_calls: &constructor_calls,
        coroutines: hops.coroutines,
        functions: &program.functions,
        taken: &taken,
        poison_sites: &poison_sites,
        typed,
    });
    let dispatch_sites = dispatches
        .iter()
        .map(
            |(caller, field, _, file, position, typing)| loop_owner_instance::DispatchSite {
                caller: *caller,
                field: field.clone(),
                file: *file,
                position: *position,
                records: match typing {
                    Typing::Records(records) => Some(records.clone()),
                    Typing::Untyped | Typing::Unmatched(_) => None,
                },
            },
        )
        .collect::<Vec<_>>();
    // Step B (card W47-LOOP-OWNER-TYPED-DISPATCH): a collector registrant
    // registered outside a plugin directory, on an instance whose plugin is a
    // runtime copy.
    // Card W47-SERVICE-PROPERTY-KEYS: every macro's replacement-list names,
    // over every read file.
    let mut macro_bodies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (defines, _) in &macros {
        for (name, words) in defines {
            macro_bodies
                .entry(name.clone())
                .or_default()
                .extend(words.iter().cloned());
        }
    }
    let step_b = StepB {
        files,
        hops,
        typed,
        definitions: &definitions,
        scans: &scans,
        index: &index,
        registrants: &registrants,
        collector_registrations: &collector_registrations,
        field_stores: &field_stores,
        direct_edges: &edges,
        spawns: &spawns,
        taken: &taken,
        flag_writers: &flag_writers,
        descriptor_objects: &descriptor_objects,
        numeric_macros: &numeric_macros,
        functions: &program.functions,
        flow: &FLUENT_BIT_INSTANCE_FLOW,
        reach: &reach,
        dispatch_sites: &dispatch_sites,
        notes: std::cell::RefCell::new(BTreeSet::new()),
        keys: &FLUENT_BIT_PROPERTY_KEYS,
        macro_bodies: &macro_bodies,
        property_readings: std::cell::RefCell::new(BTreeMap::new()),
        premises,
        region_macros: configuration.and_then(|c| c.region_macros.as_ref()),
        copy_literal: std::cell::RefCell::new(None),
    };
    let instance_flags = derive_instance_flags(&step_b);
    let property_keys = step_b.property_readings.take();
    // Card W47-PREMISES-CORO-PLUGIN: the premises the route drop, the
    // directory flags and the fixed modes rest on. An underived P-CORO-HANDLE
    // leaves every coroutine entry unresolved and keeps every route.
    let coro_premises = step_b.coro_plugin_premises(&FLUENT_BIT_CORO_PLUGIN);
    let _ = step_b.notes.take();
    let route_may_be_dropped = coro_premises.route_may_be_dropped();
    let flags_derived = typed.is_none() || matches!(coro_premises.descriptor_flags, Some(Ok(_)));
    if let Some(Err(why)) = &coro_premises.coro_handle {
        let ids = entries.keys().copied().collect::<Vec<_>>();
        escape(&ids, format!("P-CORO-HANDLE is not derived: {why}"));
    }
    // Which inputs run their collectors in a coroutine, from their plugin
    // directory's descriptor flags, else from Step B; undecided when a
    // registration is outside a plugin directory and Step B does not derive
    // it, or the directory's descriptors disagree.
    let runs_in_coroutine = |id: usize| -> Option<bool> {
        let from_directory = || {
            // Card W47-PREMISES-CORO-PLUGIN: with a typed reading, a
            // directory's flag decides only a registrant registered on its
            // own plugins' instances, under derived descriptor flags.
            if typed.is_some()
                && (!flags_derived || !matches!(coro_premises.registrations.get(&id), Some(Ok(_))))
            {
                return None;
            }
            let mut value = None;
            for path in collector_sites.get(&id)? {
                for flag in descriptor_flags.get(&plugin_dir(path)?)? {
                    let flag = (*flag)?;
                    if value.is_some_and(|v| v != flag) {
                        return None;
                    }
                    value = Some(flag);
                }
            }
            value
        };
        from_directory().or_else(|| {
            instance_flags
                .get(&id)
                .and_then(|flag| flag.derived)
                .filter(|_| flags_derived)
        })
    };
    // Field dispatch edges (hops V, V*, R, L), keyed by record type when the
    // typed reading names both the dispatch and the registration.
    let mut dispatch_typing_tally: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (caller, field, effect, file, position, typing) in &dispatches {
        let hop = if field == hops.collector_field {
            ReadHop::CollectorRegistry
        } else {
            ReadHop::Vtable
        };
        let kind = match typing {
            Typing::Untyped => "untyped",
            Typing::Records(_) => "typed",
            Typing::Unmatched(_) => "unmatched",
        };
        dispatch_typing_tally
            .entry(kind.to_owned())
            .or_default()
            .push(format!(
                "{}:{}:{field}",
                files[*file].path, scans[*file].tokens[*position].line
            ));
        if let Typing::Unmatched(why) = typing {
            let ids = registrants
                .get(field)
                .into_iter()
                .flat_map(BTreeMap::keys)
                .copied()
                .collect::<Vec<_>>();
            let line = scans[*file].tokens[*position].line;
            escape(
                &ids,
                format!(
                    "dispatched through `{field}` at {}:{line}, whose record type is not \
                     matched ({})",
                    files[*file].path,
                    why.iter().cloned().collect::<Vec<_>>().join("; ")
                ),
            );
            continue;
        }
        let (callers, body) = placed(*caller, *file, *position);
        for (callee, keys) in registrants.get(field).into_iter().flatten() {
            let reached = match typing {
                Typing::Untyped | Typing::Unmatched(_) => true,
                Typing::Records(records) => keys.iter().any(|key| match key {
                    Keyed::Any => true,
                    Keyed::Record(record) => records.contains(record),
                }),
            };
            if !reached {
                continue;
            }
            // An input whose descriptors run no coroutine is never
            // dispatched from a coroutine entry.
            if body
                && field == hops.collector_field
                && route_may_be_dropped
                && runs_in_coroutine(*callee) == Some(false)
            {
                continue;
            }
            for from in &callers {
                edges.insert(ReadEdge {
                    caller: *from,
                    callee: *callee,
                    hop,
                    effect: *effect,
                });
            }
            if body {
                bodies.insert((*caller, *callee));
            }
        }
    }
    // Hop C's resumers: a constructor call resumed in the same function
    // through the wrapper on the handle it returned.
    let constructed = |hop: usize| {
        entries
            .iter()
            .filter(|(_, constructors)| {
                constructors
                    .iter()
                    .any(|k| plain_name(*k) == hops.coroutines[hop].constructor)
            })
            .map(|(entry, _)| *entry)
            .collect::<Vec<_>>()
    };
    for (caller, hop, variable, path, line, effect) in &constructor_calls {
        let coroutine = &hops.coroutines[*hop];
        let resumed = variable.as_ref().is_some_and(|variable| {
            resume_calls
                .iter()
                .any(|(c, h, v, ..)| c == caller && h == hop && v.as_ref() == Some(variable))
        });
        for entry in constructed(*hop) {
            if resumed {
                edges.insert(ReadEdge {
                    caller: *caller,
                    callee: entry,
                    hop: ReadHop::Coroutine,
                    effect: *effect,
                });
            } else {
                escape(
                    &[entry],
                    format!(
                        "`{}` constructs coroutine `{}` at {path}:{line} without resuming the \
                         handle it returns through `{}` there: its resumer is not derived",
                        coroutine.constructor, program.functions[entry], coroutine.resume
                    ),
                );
            }
        }
    }
    for (caller, hop, variable, path, line) in &resume_calls {
        let coroutine = &hops.coroutines[*hop];
        let derived = variable.as_ref().is_some_and(|variable| {
            constructor_calls
                .iter()
                .any(|(c, h, v, ..)| c == caller && h == hop && v.as_ref() == Some(variable))
        });
        if !derived {
            escape(
                &constructed(*hop),
                format!(
                    "`{}` resumes a handle not derived from `{}` at {path}:{line}: the \
                     coroutine's resumer is not derived",
                    coroutine.resume, coroutine.constructor
                ),
            );
        }
    }
    // Yields: a function (not an entry) that switches back to its resumer is
    // a yield primitive; a macro that does, or names a primitive, is a yield
    // macro; a call of either yields, except in a terminal return.
    let primitives = switch_backs
        .iter()
        .map(|(caller, ..)| *caller)
        .filter(|caller| !entries.contains_key(caller))
        .collect::<BTreeSet<_>>();
    let primitive_names = primitives
        .iter()
        .map(|id| plain_name(*id))
        .collect::<BTreeSet<_>>();
    let yield_macros = macros
        .iter()
        .flat_map(|(defines, _)| defines.iter())
        .filter(|(_, words)| {
            (words.contains(hops.switch_primitive) && words.contains(hops.resumer_field))
                || words.iter().any(|word| primitive_names.contains(word))
        })
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let mut yields: BTreeMap<usize, String> = BTreeMap::new();
    for call in &calls {
        if entries.contains_key(&call.caller)
            || hops
                .terminal_returns
                .contains(&plain_name(call.caller).as_str())
        {
            continue;
        }
        if primitive_names.contains(&call.callee) || yield_macros.contains(&call.callee) {
            yields
                .entry(call.caller)
                .or_insert_with(|| format!("`{}` at {}:{}", call.callee, call.file, call.line));
        }
    }
    // A primitive run other than by a direct call (a field registrant, or a
    // coroutine body itself) yields where it runs.
    for primitive in &primitives {
        if registrants.values().any(|set| set.contains_key(primitive))
            || bodies.iter().any(|(_, body)| body == primitive)
        {
            yields.entry(*primitive).or_insert_with(|| {
                format!(
                    "`{}` switches back to its resumer",
                    program.functions[*primitive]
                )
            });
        }
    }
    for (field, at) in &poisoned_fields {
        let ids = registrants
            .get(field)
            .into_iter()
            .flat_map(BTreeMap::keys)
            .copied()
            .collect::<Vec<_>>();
        escape(&ids, format!("field `{field}` read as a value at {at}"));
    }
    // Card W47-PREMISES-CORO-PLUGIN: an underived immutable threaded flag
    // fixes no mode: every edge that fixes one leaves its callee unresolved.
    if let Some(Err(why)) = &coro_premises.threaded_flag {
        let ids = edges
            .iter()
            .filter(|edge| edge.effect != ReadEffect::Keep)
            .map(|edge| edge.callee)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        escape(
            &ids,
            format!("the threaded flag is not derived immutable after start: {why}"),
        );
    }
    if typed.is_some() {
        program.coro_plugin_premises = coro_premises.report(&program.functions);
    }
    program.edges = edges.into_iter().collect();
    program.spawns = spawns.into_iter().collect();
    program.roots = roots.into_iter().collect();
    program.unresolved = unresolved;
    program.calls = calls;
    program.coroutine_bodies = bodies.into_iter().collect();
    program.yields = yields;
    program.dispatch_typing = dispatch_typing_tally;
    program.instance_flags = instance_flags;
    program.property_keys = property_keys;
    // Card W47-PREMISES-INSTANCE: P-INSTANCE over the final edges, read by a
    // Step B view that needs neither the direct edges nor the spawns.
    if premises.is_some() {
        let (no_edges, no_spawns) = (BTreeSet::new(), BTreeSet::new());
        let view = StepB {
            files,
            hops,
            typed,
            definitions: &definitions,
            scans: &scans,
            index: &index,
            registrants: &registrants,
            collector_registrations: &collector_registrations,
            field_stores: &field_stores,
            direct_edges: &no_edges,
            spawns: &no_spawns,
            taken: &taken,
            flag_writers: &flag_writers,
            descriptor_objects: &descriptor_objects,
            numeric_macros: &numeric_macros,
            functions: &program.functions,
            flow: &FLUENT_BIT_INSTANCE_FLOW,
            reach: &reach,
            dispatch_sites: &dispatch_sites,
            notes: std::cell::RefCell::new(BTreeSet::new()),
            keys: &FLUENT_BIT_PROPERTY_KEYS,
            macro_bodies: &macro_bodies,
            property_readings: std::cell::RefCell::new(BTreeMap::new()),
            premises,
            region_macros: configuration.and_then(|c| c.region_macros.as_ref()),
            copy_literal: std::cell::RefCell::new(None),
        };
        let report = view.instance_premises(&program);
        program.premises = Some(report);
    }
    let dispatcher = |mode: ReadMode| {
        let (name, _) = hops.loop_dispatchers.iter().find(|(_, m)| *m == mode)?;
        let ids = by_name
            .get(name)
            .map(|found| {
                let mut ids = found
                    .iter()
                    .map(|d| index[&d.census_name])
                    .collect::<Vec<_>>();
                ids.dedup();
                ids
            })
            .unwrap_or_default();
        (ids.len() == 1).then(|| ids[0])
    };
    program.engine_dispatcher = dispatcher(ReadMode::NonThreaded);
    program.worker_dispatcher = dispatcher(ReadMode::Threaded);
    program
}

/// `(resumer, coroutine hop, handle variable, path, line, effect)` (hop C).
type ConstructorCall = (usize, usize, Option<String>, String, usize, ReadEffect);

/// What the instance-flow reachability reads (card
/// W47-LOOP-OWNER-INSTANCE-FLOW).
struct InstanceReachInputs<'a> {
    definitions: &'a [Definition],
    index: &'a BTreeMap<String, usize>,
    scans: &'a [Scan],
    raw_direct: &'a [(usize, usize)],
    dispatches: &'a [(usize, String, ReadEffect, usize, usize, Typing)],
    registrants: &'a BTreeMap<String, BTreeMap<usize, BTreeSet<Keyed>>>,
    spawns: &'a BTreeSet<(usize, usize)>,
    entries: &'a BTreeMap<usize, BTreeSet<usize>>,
    constructor_calls: &'a [ConstructorCall],
    coroutines: &'a [CoroutineHop],
    functions: &'a [String],
    taken: &'a BTreeSet<usize>,
    poison_sites: &'a [(String, String, usize)],
    typed: Option<&'a crate::typed_dispatch_census::TypedFacts>,
}

/// The census graph over-approximated as reachability (card
/// W47-LOOP-OWNER-INSTANCE-FLOW): every direct call at its own caller, every
/// registrant a dispatch may reach (the coroutine route not pruned), spawns
/// and coroutine entries; and the functions an unmodelled call may reach.
fn instance_reach(inputs: &InstanceReachInputs<'_>) -> loop_owner_instance::Reach {
    let mut reach = loop_owner_instance::Reach::default();
    let mut next: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut edge = |from: usize, to: usize| {
        next.entry(from).or_default().insert(to);
    };
    for (caller, callee) in inputs.raw_direct {
        edge(*caller, *callee);
    }
    for (caller, field, _, _, _, typing) in inputs.dispatches {
        for (callee, keys) in inputs.registrants.get(field).into_iter().flatten() {
            let reached = match typing {
                Typing::Untyped | Typing::Unmatched(_) => true,
                Typing::Records(records) => keys.iter().any(|key| match key {
                    Keyed::Any => true,
                    Keyed::Record(record) => records.contains(record),
                }),
            };
            if reached {
                edge(*caller, *callee);
            }
        }
    }
    for (spawner, entry) in inputs.spawns {
        edge(*spawner, *entry);
    }
    let plain = |id: usize| {
        let name = &inputs.functions[id];
        name.rsplit_once("::")
            .map_or(name.as_str(), |(_, plain)| plain)
            .to_owned()
    };
    for (entry, constructors) in inputs.entries {
        for constructor in constructors {
            edge(*constructor, *entry);
        }
    }
    for (caller, hop, ..) in inputs.constructor_calls {
        for (entry, constructors) in inputs.entries {
            if constructors
                .iter()
                .any(|k| plain(*k) == inputs.coroutines[*hop].constructor)
            {
                edge(*caller, *entry);
            }
        }
    }
    reach.next = next;
    for id in inputs.taken {
        reach.wild.insert(
            *id,
            "its address is taken or it is named in a macro".to_owned(),
        );
    }
    for (field, path, line) in inputs.poison_sites {
        let records = inputs.typed.and_then(|typed| {
            typed
                .field_values
                .get(&format!("{path}|{line}|{field}"))
                .filter(|records| {
                    records
                        .iter()
                        .all(|r| matches!(r, crate::typed_dispatch_census::TypedRecord::Record(_)))
                })
                .map(|records| {
                    records
                        .iter()
                        .filter_map(|r| match r {
                            crate::typed_dispatch_census::TypedRecord::Record(r) => Some(r.clone()),
                            crate::typed_dispatch_census::TypedRecord::Unmatched(_) => None,
                        })
                        .collect::<BTreeSet<_>>()
                })
        });
        for (id, keys) in inputs.registrants.get(field).into_iter().flatten() {
            let escapes = records.as_ref().is_none_or(|records| {
                keys.iter().any(|key| match key {
                    Keyed::Any => true,
                    Keyed::Record(record) => records.contains(record),
                })
            });
            if escapes {
                reach
                    .wild
                    .entry(*id)
                    .or_insert_with(|| format!("field `{field}` read as a value at {path}:{line}"));
            }
        }
    }
    for definition in inputs.definitions {
        if inputs.scans[definition.file].unstructured {
            reach
                .unstructured
                .insert(inputs.index[&definition.census_name]);
        }
    }
    reach
}

/// What Step B reads (card W47-LOOP-OWNER-TYPED-DISPATCH).
struct StepB<'a> {
    files: &'a [CensusFile],
    hops: &'a HopTables,
    typed: Option<&'a crate::typed_dispatch_census::TypedFacts>,
    definitions: &'a [Definition],
    scans: &'a [Scan],
    index: &'a BTreeMap<String, usize>,
    registrants: &'a BTreeMap<String, BTreeMap<usize, BTreeSet<Keyed>>>,
    collector_registrations: &'a [(usize, usize, Option<String>, usize)],
    field_stores: &'a [(usize, String, usize, usize)],
    direct_edges: &'a BTreeSet<ReadEdge>,
    spawns: &'a BTreeSet<(usize, usize)>,
    taken: &'a BTreeSet<usize>,
    flag_writers: &'a BTreeSet<Option<usize>>,
    descriptor_objects: &'a BTreeMap<String, Option<bool>>,
    numeric_macros: &'a BTreeSet<String>,
    functions: &'a [String],
    /// Card W47-LOOP-OWNER-INSTANCE-FLOW.
    flow: &'a InstanceFlowTables,
    reach: &'a loop_owner_instance::Reach,
    dispatch_sites: &'a [loop_owner_instance::DispatchSite],
    notes: std::cell::RefCell<BTreeSet<String>>,
    /// Card W47-SERVICE-PROPERTY-KEYS.
    keys: &'a PropertyKeyTables,
    macro_bodies: &'a BTreeMap<String, BTreeSet<String>>,
    property_readings: std::cell::RefCell<BTreeMap<String, Vec<String>>>,
    /// Card W47-PREMISES-INSTANCE: the premised census's inputs.
    premises: Option<&'a PremiseInputs>,
    /// Card W47-PREMISES-HOLDER-REGION.
    region_macros: Option<&'a region_macros::RegionMacros>,
    copy_literal: std::cell::RefCell<Option<String>>,
}

/// A flag derivation's value: `Some(false)` no coroutine, `Some(true)` the
/// coroutine flag, `None` underived; with the reason.
type Derivation = Result<bool, String>;

/// Join two derivations of one flag: the coroutine flag anywhere wins, an
/// underived one withholds a "no coroutine".
fn join_flags(a: Derivation, b: Derivation) -> Derivation {
    match (a, b) {
        (Ok(true), _) | (_, Ok(true)) => Ok(true),
        (Err(why), _) | (_, Err(why)) => Err(why),
        (Ok(false), Ok(false)) => Ok(false),
    }
}

/// The coroutine flag an `A | B | …` operand list sets: `Ok(true)` when an
/// operand is the flag, `Ok(false)` when every operand is a number or a
/// numeric macro, an error otherwise.
fn flags_operands(texts: &[&str], hops: &HopTables, numeric: &BTreeSet<String>) -> Derivation {
    let mut value = false;
    for (index, text) in texts.iter().enumerate() {
        if index % 2 == 1 {
            if *text != "|" {
                return Err(format!(
                    "a flags value this reading cannot evaluate (`{text}`)"
                ));
            }
        } else if *text == hops.coroutine_flag {
            value = true;
        } else if !(text.chars().all(|c| c.is_ascii_digit()) || numeric.contains(*text)) {
            return Err(format!(
                "a flags operand this reading cannot evaluate (`{text}`)"
            ));
        }
    }
    if texts.len() % 2 == 1 {
        Ok(value)
    } else {
        Err("an empty or unfinished flags value".to_owned())
    }
}

impl StepB<'_> {
    fn definition_at(&self, file: usize, position: usize) -> Option<&Definition> {
        self.scans[file]
            .definitions
            .iter()
            .map(|d| &self.definitions[*d])
            .find(|d| d.body.0 <= position && position <= d.body.1)
    }

    fn texts(&self, file: usize) -> Vec<&str> {
        self.scans[file]
            .tokens
            .iter()
            .map(|t| t.text.as_str())
            .collect()
    }

    /// The instance flag of local `instance` in `g`: only an instance assigned
    /// once, from the listed constructor with a plugin name the typed reading
    /// reads, whose one descriptor of that name decides its flag; and only when
    /// the flag field is written by the constructor alone.
    fn instance_flag(&self, g: &Definition, instance: &str) -> Derivation {
        let constructor = self.hops.instance_constructor.0;
        let writers = self
            .flag_writers
            .iter()
            .map(|writer| writer.map(|id| self.functions[id].as_str()))
            .collect::<BTreeSet<_>>();
        if writers != BTreeSet::from([Some(constructor)]) {
            return Err(format!(
                "`{}` is written other than by `{constructor}` alone",
                self.hops.instance_flag_field
            ));
        }
        if g.parameters.contains(instance) {
            return Err(format!("the instance `{instance}` is a parameter"));
        }
        let texts = self.texts(g.file);
        let path = &self.files[g.file].path;
        let mut creation = None;
        for k in g.body.0..=g.body.1 {
            if texts[k] != instance || matches!(texts[k - 1], "." | ">") {
                continue;
            }
            if texts[k - 1] == "&" && texts[k - 2] != "&" {
                return Err(format!("the address of the instance `{instance}` is taken"));
            }
            if texts[k + 1] == "=" && texts[k + 2] != "=" {
                if creation.is_some() {
                    return Err(format!("the instance `{instance}` is assigned twice"));
                }
                creation = Some(k + 2);
            }
        }
        let Some(start) = creation else {
            return Err(format!("the instance `{instance}` is never assigned here"));
        };
        // Card W47-LOOP-OWNER-INSTANCE-FLOW: an instance read as a context's
        // input-list head.
        if texts[start] == self.flow.list_entry_macro && texts[start + 1] == "(" {
            return self.list_head_instance(g, instance, start);
        }
        if texts[start] != constructor || texts[start + 1] != "(" {
            return Err(format!(
                "the instance `{instance}` is not tied to its creation: it is assigned from \
                 `{}` at {path}:{}, not from `{constructor}`",
                texts[start], self.scans[g.file].tokens[start].line
            ));
        }
        let line = self.scans[g.file].tokens[start].line;
        let literals = self
            .typed
            .and_then(|typed| {
                typed
                    .literal_calls
                    .get(&crate::typed_dispatch_census::literal_call_key(
                        path,
                        line,
                        constructor,
                    ))
            })
            .cloned()
            .unwrap_or_default();
        let [literal] = literals.iter().collect::<Vec<_>>()[..] else {
            return Err(format!(
                "the plugin name `{constructor}` creates `{instance}` from at {path}:{line} is \
                 not one literal the typed reading reads"
            ));
        };
        // Card W47-LOOP-OWNER-INSTANCE-FLOW: every descriptor the name matches,
        // ignoring ASCII case as `check_protocol` compares.
        self.copy_literal.replace(Some(literal.clone()));
        self.descriptors_flag(literal)
    }

    /// The copy of a plugin at local object path `base` in `g`, installed on
    /// one instance: its flag joined with the instance's. Any use of the copy
    /// or its object this reading does not model leaves it underived.
    fn copy_flag(&self, g: &Definition, base: &[&str]) -> Derivation {
        let texts = self.texts(g.file);
        let root = base[0];
        let hops = self.hops;
        if g.parameters.contains(root) {
            return Err(format!("the copy's object `{root}` is a parameter"));
        }
        let mut sources: Vec<Derivation> = Vec::new();
        let mut installed: BTreeSet<String> = BTreeSet::new();
        // Card W47-LOOP-OWNER-INSTANCE-FLOW: `X->data = &root;`.
        let mut escapes: BTreeSet<String> = BTreeSet::new();
        let mut declared = false;
        let mut blockers: Vec<String> = Vec::new();
        let statement_end = |from: usize| {
            (from..=g.body.1)
                .find(|i| texts[*i] == ";")
                .unwrap_or(g.body.1)
        };
        for k in g.body.0..=g.body.1 {
            if texts[k] != root || matches!(texts[k - 1], "." | ">") {
                continue;
            }
            let path_matches = base
                .iter()
                .enumerate()
                .all(|(i, part)| texts.get(k + 2 * i) == Some(part))
                && (1..base.len()).all(|i| texts.get(k + 2 * i - 1) == Some(&"."));
            if !path_matches {
                // The object holding the copy: a declaration, another member
                // or a by-value use leave the copy alone; its address does not.
                if !declared && is_identifier(texts[k - 1]) && texts[k - 1] != "return" {
                    declared = true;
                    if texts[k + 1] == "=" {
                        let end = statement_end(k + 2);
                        if !matches!(texts[k + 2..end], ["{", "0", "}"] | ["{", "}"]) {
                            blockers.push(format!("`{root}` is initialized from a value"));
                            continue;
                        }
                        sources.push(Ok(false));
                    }
                    continue;
                }
                if texts[k - 1] == "&"
                    && texts[k - 2] == "="
                    && k >= 7
                    && texts[k - 3] == self.flow.instance_data_field
                    && texts[k - 4] == ">"
                    && texts[k - 5] == "-"
                    && is_identifier(texts[k - 6])
                    && !matches!(texts[k - 7], "." | ">")
                    && texts[k + 1] == ";"
                {
                    escapes.insert(texts[k - 6].to_owned());
                    continue;
                }
                if texts[k - 1] == "&" && texts[k - 2] != "&" && texts[k + 1] != "." {
                    blockers.push(format!(
                        "the object `{root}` holding the copy escapes (its address is taken)"
                    ));
                    continue;
                }
                if texts[k + 1] == "=" && texts[k + 2] != "=" {
                    blockers.push(format!("`{root}` is assigned whole"));
                    continue;
                }
                continue;
            }
            if !declared {
                blockers.push(format!("`{root}` is not a local object of this function"));
                continue;
            }
            let after = k + 2 * base.len() - 1;
            if texts[k - 1] == "&" && texts[k - 2] != "&" {
                // `X->p = &copy;` installs it; `memcpy(&copy, &D, …)` copies D.
                if k >= 6
                    && texts[k - 2] == "="
                    && texts[k - 3] == hops.instance_plugin_field
                    && texts[k - 4] == ">"
                    && texts[k - 5] == "-"
                    && is_identifier(texts[k - 6])
                    && texts[after] == ";"
                {
                    installed.insert(texts[k - 6].to_owned());
                } else if k >= 3
                    && texts[k - 3] == "memcpy"
                    && texts[k - 2] == "("
                    && texts[after] == ","
                    && texts[after + 1] == "&"
                    && is_identifier(texts[after + 2])
                    && texts[after + 3] == ","
                {
                    sources.push(self.descriptor_source(texts[after + 2]));
                } else {
                    blockers.push("the copy's address is passed on".to_owned());
                    continue;
                }
                continue;
            }
            if (texts[k - 1] == "+" && texts[k - 2] == "+")
                || (texts[k - 1] == "-" && texts[k - 2] == "-")
            {
                blockers.push("the copy is updated in place".to_owned());
                continue;
            }
            match (texts[after], texts[after + 1]) {
                ("=", next) if next != "=" => {
                    let end = statement_end(after + 1);
                    match &texts[after + 1..end] {
                        ["*", instance, "-", ">", field]
                            if *field == hops.instance_plugin_field && is_identifier(instance) =>
                        {
                            sources.push(self.instance_flag(g, instance));
                        }
                        [descriptor] if is_identifier(descriptor) => {
                            sources.push(self.descriptor_source(descriptor));
                        }
                        other => {
                            blockers
                                .push(format!("the copy is assigned from `{}`", other.join(" ")));
                            continue;
                        }
                    }
                }
                (".", field) if field == hops.plugin_flags_field => {
                    let op = texts[after + 2];
                    let next = texts[after + 3];
                    if op == "=" && next != "=" {
                        let end = statement_end(after + 3);
                        sources.push(flags_operands(
                            &texts[after + 3..end],
                            hops,
                            self.numeric_macros,
                        ));
                    } else if op == "|" && next == "=" {
                        let end = statement_end(after + 4);
                        match flags_operands(&texts[after + 4..end], hops, self.numeric_macros) {
                            Ok(false) => {}
                            other => sources.push(other),
                        }
                    } else if matches!(
                        (op, next),
                        ("&" | "^" | "+" | "-" | "*" | "/" | "%", "=")
                            | ("+", "+")
                            | ("-", "-")
                            | ("<", "<")
                            | (">", ">")
                    ) {
                        blockers.push("the copy's flags are updated in place".to_owned());
                        continue;
                    }
                }
                _ => {}
            }
        }
        if !escapes.is_empty() {
            if escapes != installed || base.len() < 2 {
                blockers.push(format!(
                    "the object `{root}` holding the copy escapes into an instance other than \
                     the one the copy is installed on"
                ));
            } else if let Err(why) = self.holder_escape(g, root, base[1]) {
                blockers.push(why);
            }
        }
        let installed = installed.iter().collect::<Vec<_>>();
        self.copy_literal.replace(None);
        let instance = match installed[..] {
            [instance] => self.instance_flag(g, instance),
            _ => Err("the copy is not installed on exactly one instance".to_owned()),
        };
        // Card W47-PREMISES-HOLDER-REGION: P-HOLDER-OPAQUE derived.
        if !escapes.is_empty()
            && blockers.is_empty()
            && instance.is_ok()
            && let Err(why) = self.holder_opaque(g, root, base[1])
        {
            blockers.push(why);
        }
        if sources.is_empty() {
            blockers.push("the copy is never initialized".to_owned());
        }
        for derivation in sources.iter().chain(std::iter::once(&instance)) {
            if let Err(why) = derivation
                && !blockers.contains(why)
            {
                blockers.push(why.clone());
            }
        }
        if !blockers.is_empty() {
            return Err(blockers.join("; "));
        }
        let copied = sources.into_iter().reduce(join_flags).unwrap_or(Ok(true));
        join_flags(copied, instance)
    }

    fn descriptor_source(&self, name: &str) -> Derivation {
        match self.descriptor_objects.get(name) {
            Some(Some(flag)) => Ok(*flag),
            _ => Err(format!(
                "the copy's source `{name}` is not a derived descriptor"
            )),
        }
    }

    /// Step B for one registration: `registrant` registered in `function` on
    /// that function's first parameter, `function` stored only in fields of
    /// plugin copies.
    fn registration(&self, function: usize, instance: Option<&str>) -> Derivation {
        let name = &self.functions[function];
        let definitions = self
            .definitions
            .iter()
            .filter(|d| self.index[&d.census_name] == function)
            .collect::<Vec<_>>();
        let [definition] = definitions[..] else {
            return Err(format!("`{name}` is not defined once"));
        };
        let Some(parameter) = definition.first_parameter.as_deref() else {
            return Err(format!("`{name}` has no named first parameter"));
        };
        if instance != Some(parameter) {
            return Err(format!(
                "the registration in `{name}` is not on its first parameter"
            ));
        }
        let texts = self.texts(definition.file);
        if (definition.body.0..=definition.body.1).any(|k| {
            texts[k] == parameter
                && !matches!(texts[k - 1], "." | ">")
                && texts[k + 1] == "="
                && texts[k + 2] != "="
        }) {
            return Err(format!("`{name}` reassigns its instance parameter"));
        }
        if self.taken.contains(&function)
            || self.spawns.iter().any(|(_, entry)| *entry == function)
            || self
                .direct_edges
                .iter()
                .any(|edge| edge.callee == function && edge.hop == ReadHop::Direct)
        {
            return Err(format!(
                "`{name}` is called or named other than through a plugin field"
            ));
        }
        let stores = self
            .field_stores
            .iter()
            .filter(|(id, ..)| *id == function)
            .collect::<Vec<_>>();
        let registered = self
            .registrants
            .values()
            .filter(|set| set.contains_key(&function))
            .count();
        if stores.is_empty()
            || registered
                != stores
                    .iter()
                    .map(|(_, f, ..)| f)
                    .collect::<BTreeSet<_>>()
                    .len()
        {
            return Err(format!(
                "`{name}` is not registered only through plugin fields"
            ));
        }
        // Card W47-LOOP-OWNER-INSTANCE-FLOW: P-CALLBACK-INSTANCE, derived.
        let fields = stores
            .iter()
            .map(|(_, field, ..)| field.clone())
            .collect::<BTreeSet<_>>();
        let mut keys = BTreeSet::new();
        let mut any_key = false;
        for set in self.registrants.values() {
            for key in set.get(&function).into_iter().flatten() {
                match key {
                    Keyed::Any => any_key = true,
                    Keyed::Record(record) => {
                        keys.insert(record.clone());
                    }
                }
            }
        }
        self.callback_instance(function, &fields, &keys, any_key)?;
        let mut flag: Derivation = Ok(false);
        for (_, _, file, position) in stores {
            let texts = self.texts(*file);
            if texts[position - 1] != "." {
                return Err(format!(
                    "`{name}` is stored through a pointer, not in a local plugin copy"
                ));
            }
            let mut start = position - 1;
            while start >= 2 && texts[start] == "." && is_identifier(texts[start - 1]) {
                start -= 2;
            }
            let base = texts[start + 1..position - 1]
                .iter()
                .copied()
                .filter(|t| *t != ".")
                .collect::<Vec<_>>();
            if base.is_empty() || matches!(texts[start], "-" | ">" | ")" | "]") {
                return Err(format!(
                    "`{name}` is stored through a pointer, not in a local plugin copy"
                ));
            }
            let Some(g) = self.definition_at(*file, *position) else {
                return Err(format!("`{name}` is stored outside a function"));
            };
            flag = join_flags(flag, self.copy_flag(g, &base));
        }
        flag
    }
}

/// Step B (card W47-LOOP-OWNER-TYPED-DISPATCH): the coroutine flag of each
/// collector registrant registered outside a plugin directory. The route
/// switch reads the instance's flag, set by its constructor from the plugin it
/// was created from, so a registrant registered in a callback of a runtime
/// plugin copy is derived to run no coroutine only when the copy's own flags
/// and the flag of the one instance it is installed on both derive none.
fn derive_instance_flags(step: &StepB<'_>) -> BTreeMap<usize, InstanceFlag> {
    // registrant -> (registering function, instance argument, file).
    type Registrations<'r> = Vec<(usize, Option<&'r str>, usize)>;
    let mut by_registrant: BTreeMap<usize, Registrations<'_>> = BTreeMap::new();
    for (registrant, function, instance, file) in step.collector_registrations {
        by_registrant
            .entry(*registrant)
            .or_default()
            .push((*function, instance.as_deref(), *file));
    }
    let mut flags = BTreeMap::new();
    for (registrant, registrations) in by_registrant {
        if registrations
            .iter()
            .all(|(_, _, file)| step.files[*file].path.starts_with(step.hops.plugin_root))
        {
            continue;
        }
        let stored = step.field_stores.iter().any(|(id, ..)| *id == registrant);
        let derivation = if stored {
            Err(format!(
                "`{}` is also stored in a field",
                step.functions[registrant]
            ))
        } else {
            registrations
                .iter()
                .map(|(function, instance, _)| step.registration(*function, *instance))
                .reduce(join_flags)
                .unwrap_or_else(|| Err("no registration".to_owned()))
        };
        let notes = step.notes.take();
        let (derived, reason) = match derivation {
            Ok(false) => (
                Some(false),
                std::iter::once(
                    "the copy's flags and its instance's creating plugin derive no coroutine"
                        .to_owned(),
                )
                .chain(notes)
                .collect::<Vec<_>>()
                .join("; "),
            ),
            Ok(true) => (Some(true), "the coroutine flag is derived".to_owned()),
            // Card W47-LOOP-OWNER-INSTANCE-FLOW: what did derive, beside why
            // the flag did not.
            Err(why) if !notes.is_empty() => (
                None,
                format!(
                    "{why}; derived nonetheless: {}",
                    notes.into_iter().collect::<Vec<_>>().join("; ")
                ),
            ),
            Err(why) => (None, why),
        };
        flags.insert(registrant, InstanceFlag { derived, reason });
    }
    flags
}

/// Macros every definition of which writes no name (a number): a flags
/// operand spelled with one is not the coroutine flag.
fn numeric_macros(macros: &[MacroDefinitions]) -> BTreeSet<String> {
    let mut numeric: BTreeMap<&str, bool> = BTreeMap::new();
    for (defines, _) in macros {
        for (name, words) in defines {
            let entry = numeric.entry(name.as_str()).or_insert(true);
            *entry &= words.is_empty();
        }
    }
    numeric
        .into_iter()
        .filter(|(_, numeric)| *numeric)
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// A plugin descriptor's coroutine flag from its `.flags = A | B | …`
/// initializer: `Some(true)` when an operand is the flag, `Some(false)` when
/// every operand is a number or a numeric macro (or there is no flags
/// initializer: the field is zero), `None` otherwise.
fn descriptor_flag(
    tokens: &[Token],
    open: usize,
    close: usize,
    hops: &HopTables,
    numeric_macros: &BTreeSet<String>,
) -> Option<bool> {
    let mut depth = 0usize;
    let mut index = open + 1;
    while index < close {
        match tokens[index].text.as_str() {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth = depth.saturating_sub(1),
            "." if depth == 0
                && tokens
                    .get(index + 1)
                    .is_some_and(|t| t.text == hops.plugin_flags_field)
                && tokens.get(index + 2).is_some_and(|t| t.text == "=") =>
            {
                let mut value = false;
                let mut operand = true;
                let mut cursor = index + 3;
                while cursor < close && tokens[cursor].text != "," {
                    let text = tokens[cursor].text.as_str();
                    if operand {
                        if text == hops.coroutine_flag {
                            value = true;
                        } else if !(text.chars().all(|c| c.is_ascii_digit())
                            || numeric_macros.contains(text))
                        {
                            return None;
                        }
                    } else if text != "|" {
                        return None;
                    }
                    operand = !operand;
                    cursor += 1;
                }
                return (!operand).then_some(value);
            }
            _ => {}
        }
        index += 1;
    }
    Some(false)
}

/// Whether the `)` at `close` ends a condition (`if (…)`, `while (…)`, a
/// parenthesised test) rather than a call's argument list.
fn closes_a_test(tokens: &[Token], close: usize) -> bool {
    let mut depth = 0usize;
    for index in (0..=close).rev() {
        match tokens[index].text.as_str() {
            ")" => depth += 1,
            "(" => {
                depth -= 1;
                if depth == 0 {
                    let before = index
                        .checked_sub(1)
                        .map(|p| tokens[p].text.as_str())
                        .unwrap_or("");
                    return matches!(before, "if" | "while" | "(" | "!" | "&&" | "||" | "return")
                        || !is_identifier(before);
                }
            }
            _ => {}
        }
    }
    false
}
