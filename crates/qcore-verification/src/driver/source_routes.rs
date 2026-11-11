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

//! Card `ROUTE-SELECTION-ORDER-RESTATED`: the driver's source routes, in
//! selection order, written **once**.
//!
//! Before this card `compile_elaborated_source` wrote the order three times —
//! the closure refusal's if-chain over the first seven selectors, the contract
//! guard's 21-entry list (`route_selection::source_only_route`, card
//! `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`) and the route if-chain itself —
//! and the guard's list had already drifted from the chain by one arm (the
//! socket-and-event composition). [`SOURCE_ROUTES`] is now the one order:
//! [`select`] reads it with first-match semantics, exactly as the if-chain
//! did, and the driver reads the one selected entry for the closure refusal,
//! the contract guard, the selection refusal and the lowering. A route added
//! to the table is guarded by construction: the guard is a field of the entry
//! the driver lowers, not a second list.
//!
//! The Lean model is `QCore.RouteSelection`'s selector table
//! (`AuditRouteSelection`): first match over the table is the if-chain in
//! table order, the guard refuses a contract module exactly when the selected
//! entry is source-only, and with every entry but the last source-only a
//! contract module is lowered only on the last, the sequential route.
use qcore_frontend::frontend::{
    DeclarationKind, SemanticPass, SourceDiagnostic, SourceModuleMode, SourceProgram,
    TypedSourceProgram,
};
use qcore_frontend::source_scalar_calls::PrintCallSelection;
use qcore_frontend::{
    source_bitfield_concurrency, source_composed, source_composed_tasks, source_epoch,
    source_epoch_tasks, source_event, source_extern, source_ffi, source_http, source_mutex,
    source_record, source_resource_calls, source_scalar_calls, source_socket, source_std_buffer,
    source_std_io, source_std_string, source_tasks,
};

use super::{
    CompileOptions, CompiledModule, EntryOnlyRoute, FileDiagnostic, FileRange, RecordedTarget,
    check_sequential_parameters, compile_bitfield_task_source, compile_buffer_source,
    compile_composed_epoch_source, compile_composed_source, compile_composed_task_source,
    compile_epoch_source, compile_event_source, compile_extern_source, compile_ffi_source,
    compile_http_source, compile_mutex_source, compile_resource_call_source, compile_scalar_calls,
    compile_sequential_source, compile_socket_source, compile_std_io_source, compile_string_source,
    has_unmodeled_function, refuse_unmodeled_functions, std_io_lowering_refuses, task_route,
};

/// What one route's selector reads in a module.
pub(super) enum Selection {
    /// The selector does not hold: selection moves to the next entry.
    Absent,
    /// The selector holds: the module is compiled on this route.
    Selected,
    /// The selector holds and refuses at selection — the print selector's
    /// ambiguous `print (` pair (card `QR-PRINT-ROUTE-MATCHES-DECLARATION`).
    /// It still selects its entry, so the contract guard reads that entry
    /// first, as it did before this card.
    Refused(SourceDiagnostic),
}

impl Selection {
    fn of(selected: bool) -> Self {
        if selected {
            Self::Selected
        } else {
            Self::Absent
        }
    }
}

/// What a module spelling a closure meets on a route (card
/// `W48-QR-CLOSURES`): a closure is lifted into a hidden worker that the two
/// scoped-task routes lower, and every other route would meet a worker it
/// does not model, so a module spelling a closure and selecting any other
/// route is refused (`Q600`), naming the route's residual, before that route
/// reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClosureRule {
    /// One of the two scoped-task routes, which lower the hidden worker.
    Hosts,
    /// Any other route: the closure is refused with this message.
    Refuses(&'static str),
}

const CLOSURE_ON_FFI: &str = "a closure on the FFI route: `source.ffi.v1` hosts no task, so a \
     closure — and an imported struct it would capture — stays refused \
     (W48-QR-TASK-FFI-STRUCT-WORKER)";
const CLOSURE_ON_FIELD_FOOTPRINT: &str = "a closure on the field-footprint route: \
     `source.bitfield-concurrency.v1` decides named workers' field captures only \
     (W48-QR-CLOSURE-ROUTES)";
const CLOSURE_ON_EPOCH: &str = "a closure on an epoch route: an epoch task's worker is a named \
     function the epoch ledger reads by name, and a closure's hidden worker is not lowered there \
     yet (W48-QR-CLOSURE-EPOCH)";
const CLOSURE_ON_MUTEX: &str = "a closure on the mutex route: `source.mutex.guard.v1` lowers \
     named workers only, and `share … under M` is a scope clause rather than a capture \
     (W48-QR-CLOSURE-MUTEX)";
const CLOSURE_ELSEWHERE: &str = "a closure is a spawn target of the scoped-task routes, inside \
     a `thread scope` of a module marked `concurrency checked;`";

/// The inputs every lowering entry reads.
pub(super) struct RouteInput<'a> {
    pub(super) parsed: &'a SourceProgram,
    pub(super) typed: &'a TypedSourceProgram,
    pub(super) mode: SourceModuleMode,
    pub(super) options: &'a CompileOptions,
    pub(super) ranges: &'a [FileRange],
    pub(super) target: Option<&'a RecordedTarget>,
}

type Lowering = fn(&RouteInput<'_>) -> Result<CompiledModule, Vec<FileDiagnostic>>;

/// One route of `compile_elaborated_source`.
pub(super) struct SourceRoute {
    /// The route's name, as the contract guard's refusal states it.
    pub(super) name: &'static str,
    /// The route's selector.
    pub(super) select: fn(&SourceProgram) -> Selection,
    /// The route's lowering entry.
    pub(super) lower: Lowering,
    /// Whether the route lowers bodies a `module contract;` module would run.
    /// Every route but the sequential one is: a contract module declares
    /// contracts, and only the sequential route reads contract modules.
    pub(super) source_only: bool,
    /// What a closure meets on this route.
    pub(super) closure: ClosureRule,
}

/// An entry-only route's lowering followed by its widened entry-only
/// refusal (card `W48-QR-SCALAR-TYPES-R2`, item 0).
fn entry_only(
    lowered: Result<CompiledModule, Vec<FileDiagnostic>>,
    route: EntryOnlyRoute,
    input: &RouteInput<'_>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    lowered.and_then(|module| refuse_unmodeled_functions(module, route, input.parsed, input.ranges))
}

const SOCKET: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.socket.v1",
    code: "Q080",
    pass: SemanticPass::StandardSocket,
};
const EVENT: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.event.v1",
    code: "Q058",
    pass: SemanticPass::Event,
};
const STRING: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.string.v1",
    code: "Q068",
    pass: SemanticPass::StandardString,
};
const BUFFER: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.buffer.v1",
    code: "Q051",
    pass: SemanticPass::StandardBuffer,
};
const HTTP: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.http.v1",
    code: "Q090",
    pass: SemanticPass::StandardHttp,
};
const STD_IO: EntryOnlyRoute = EntryOnlyRoute {
    name: "source.std-io.v1",
    code: "Q030",
    pass: SemanticPass::StandardIo,
};

fn lower_socket(input: &RouteInput<'_>) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let RouteInput {
        parsed,
        typed,
        mode,
        options,
        ranges,
        ..
    } = *input;
    entry_only(
        compile_socket_source(parsed, typed, mode, options, ranges),
        SOCKET,
        input,
    )
}

fn lower_string(input: &RouteInput<'_>) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let RouteInput {
        parsed,
        typed,
        mode,
        options,
        ranges,
        target,
    } = *input;
    entry_only(
        compile_string_source(parsed, typed, mode, options, ranges, target),
        STRING,
        input,
    )
}

fn lower_scalar_calls(input: &RouteInput<'_>) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    let RouteInput {
        parsed,
        typed,
        mode,
        options,
        ranges,
        target,
    } = *input;
    compile_scalar_calls(parsed, typed, mode, options, ranges, target)
}

fn functions(program: &SourceProgram) -> usize {
    program
        .declarations
        .iter()
        .filter(|declaration| declaration.value.kind == DeclarationKind::Function)
        .count()
}

/// Every route of `compile_elaborated_source`, in selection order; the first
/// entry whose selector does not read [`Selection::Absent`] is the module's
/// route. The last entry, the sequential route, always selects. Each entry's
/// comment says why it stands where it does; the order is the one the
/// if-chain had, entry for entry.
pub(super) static SOURCE_ROUTES: [SourceRoute; 22] = [
    // Card `W48-QR-C-STRUCT-IMPORT`: `source.ffi.v1` is selected before every
    // other route, by `#import std.ffi.unsafe;` or a `struct!` declaration. A
    // file that imports C is unsafe as a whole (user decision 2), so it must
    // reach no route that would publish its results without the unsafe-FFI
    // assumption, and a statement this route does not model is refused here
    // rather than handed to a route that ignores the import. No file written
    // before the card carries either marker (the census), so no existing
    // module moves.
    SourceRoute {
        name: "source.ffi.v1",
        select: |parsed| Selection::of(source_ffi::has_source_ffi(parsed)),
        lower: |input| {
            compile_ffi_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ON_FFI),
    },
    // `source.bitfield-concurrency.v1`: selected before the generic task route
    // because it composes the task parser's real `write OBJECT.FIELD` captures
    // with an explicit source-derived layout. Falling through would discard
    // the record storage-unit identity and refuse for the generic route's
    // shallower reason rather than deciding the concurrent writes.
    SourceRoute {
        name: "source.bitfield-concurrency.v1",
        select: |parsed| {
            Selection::of(source_bitfield_concurrency::has_source_bitfield_tasks(
                parsed,
            ))
        },
        lower: |input| {
            compile_bitfield_task_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ON_FIELD_FOOTPRINT),
    },
    // Selected before the pure epoch route: a module carrying both an epoch
    // scope and the scoped-task profile's own marker is accounting for a
    // real task region, and must not be handed to a route whose workers are
    // declarations with no bodies.
    SourceRoute {
        name: "source.epoch.tasks.v1",
        select: |parsed| Selection::of(source_epoch_tasks::has_composed_epoch(parsed)),
        lower: |input| {
            compile_composed_epoch_source(
                input.parsed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ON_EPOCH),
    },
    // Selected before every other concurrent route: an epoch scope is also
    // a `thread scope`, and a module that declares a quantitative epoch
    // must not be handed to a route that models no accounting at all.
    SourceRoute {
        name: "source.epoch.bound.v1",
        select: |parsed| Selection::of(source_epoch::has_source_epoch(parsed)),
        lower: |input| {
            compile_epoch_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ON_EPOCH),
    },
    // Selected before the task route: a `thread scope` alone also selects
    // scoped tasks, and a module declaring a mutex generation must not be
    // handed to a route that models no mutex at all.
    SourceRoute {
        name: "source.mutex.guard.v1",
        select: |parsed| Selection::of(source_mutex::has_source_mutex(parsed)),
        lower: |input| {
            compile_mutex_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ON_MUTEX),
    },
    // Selected after every *more specific* concurrent route and before the task
    // route it composes (card `COMPOSE-CONCURRENCY`). The order is what keeps
    // every other exclusion intact: a module carrying an epoch scope or a mutex
    // generation never reaches this entry, and a module that also extern-calls
    // fails this conjunction and falls through to the task route with its
    // existing `Q600`.
    SourceRoute {
        name: "source.std-io-tasks.v1",
        select: |parsed| Selection::of(source_composed_tasks::has_source_composed_tasks(parsed)),
        lower: |input| {
            compile_composed_task_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Hosts,
    },
    SourceRoute {
        name: "the scoped-task route",
        select: |parsed| Selection::of(source_tasks::has_source_tasks(parsed)),
        lower: |input| {
            task_route(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                false,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Hosts,
    },
    // Selected after every concurrent route and before both routes it composes
    // (card `STD-IO-COMPOSE`). The order is what makes the composition safe to
    // add: a module that spawns *and* prints, or spawns *and* extern-calls,
    // never reaches here, so both concurrency exclusions keep their existing
    // refusals; and a module using one of the two composed feature sets fails
    // this conjunction and falls through to that route byte for byte.
    SourceRoute {
        name: "source.std-io-extern.v1",
        select: |parsed| Selection::of(source_composed::has_source_composed(parsed)),
        lower: |input| {
            compile_composed_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Selected after the concurrent routes on purpose: a module that both
    // spawns and calls an extern function stays with the task route, which
    // keeps its existing refusal. Lifting that is a separate card, because a
    // task worker is a separate checked model (design §3.4).
    SourceRoute {
        name: "source.extern-call.v1",
        select: |parsed| Selection::of(source_extern::has_source_extern(parsed)),
        lower: |input| {
            compile_extern_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
                input.target,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `SOCKET-EVENT-COMPOSE`, selected immediately **above** the event
    // route and reaching `compile_socket_source`, which now models both
    // vocabularies. This retires the socket card's named follow-on: a module
    // that dispatches and sockets is no longer refused by the event route.
    //
    // The conjunction is deliberately narrow — *exactly* these two special
    // vocabularies and no third. A module that also buffers, strings or does
    // HTTP declines this entry and reaches the route it reached before, keeping
    // that route's existing refusal byte for byte (`EVENT-BUFFER-PAYLOAD`,
    // `HTTP-V1-EVENT-COMPOSE`). Every concurrent route and the extern route are
    // selected above all of it, so a module that dispatches, sockets and spawns
    // still keeps its `Q600`.
    //
    // The contract guard's list before card `ROUTE-SELECTION-ORDER-RESTATED`
    // had no entry for this arm and named the event route for such a module;
    // the guard now names the route that would lower it.
    SourceRoute {
        name: "source.socket.v1",
        select: |parsed| {
            Selection::of(
                source_event::has_source_event(parsed)
                    && source_socket::has_source_socket(parsed)
                    && !source_std_buffer::has_source_buffer(parsed)
                    && !source_std_string::has_source_string(parsed)
                    && !source_http::has_source_http(parsed),
            )
        },
        lower: lower_socket,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `STD-EVENT`. Selected after every concurrent route and after the
    // extern route, and **before** the standard-IO and buffer routes, on the
    // same principle those state: a module that both dispatches and prints
    // stays with the more specific model.
    //
    // Since card `COMPOSE-EVENT-LINES` that model admits the lines rather than
    // refusing them: `print`, `log` and `flush` are checked here through
    // `source.std-io.v1`'s own reader and rendered through its own emitter, and
    // one value form (`let NAME = dispatch ready E;`) names the byte a turn
    // drained. Selection itself is unmoved, and the refusal a module that
    // *buffers* and dispatches receives is unmoved with it — that composition
    // would put two custody models in one entry and is a card of its own
    // (`EVENT-BUFFER-PAYLOAD`). A `print` inside a dispatch turn is still not
    // spellable, because v1 has no source-level callback body.
    SourceRoute {
        name: "source.event.v1",
        select: |parsed| Selection::of(source_event::has_source_event(parsed)),
        lower: |input| {
            entry_only(
                compile_event_source(
                    input.parsed,
                    input.typed,
                    input.mode,
                    input.options,
                    input.ranges,
                ),
                EVENT,
                input,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `STRING-CONCAT`, and **the whole of the route-selection change it
    // makes**: a module that spells `borrow_utf8(…)` reaches the string route,
    // which since that card models both vocabularies, rather than the buffer
    // route below. The condition is the bridge construct and nothing else, so
    // a module that holds a buffer without spelling the bridge keeps the buffer
    // route and that route's existing refusal — which is to say every program
    // written before this card travels an identical code path, and the one new
    // door is opened only by the one new spelling.
    SourceRoute {
        name: "source.string.v1",
        select: |parsed| Selection::of(source_std_string::has_source_string_bridge(parsed)),
        lower: lower_string,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    SourceRoute {
        name: "source.resource-calls.v1",
        select: |parsed| Selection::of(source_resource_calls::has_resource_calls(parsed)),
        lower: |input| {
            compile_resource_call_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `W48-QR-USABLE-RECORDS`, and **the whole of the route-selection
    // change it makes**: a module that declares a record, holds no buffer and
    // declares a function besides its entry reaches `source.scalar-calls.v1`,
    // which since that card carries records of scalars across calls by
    // value. Every such module was refused before — the buffer route models
    // its entry only (`Q051`) — so the door opens only for modules that were
    // refused, and a module with one function keeps the buffer route byte for
    // byte.
    SourceRoute {
        name: "source.scalar-calls.v1",
        select: |parsed| {
            Selection::of(
                source_record::has_source_record(parsed)
                    && !source_std_buffer::has_source_buffer(parsed)
                    && functions(parsed) > 1,
            )
        },
        lower: lower_scalar_calls,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `STD-BUFFER`, selected after the extern route above and before the
    // standard-IO route below. Both neighbours are deliberate:
    //
    // * **After the extern route**, so a module that both allocates a buffer and
    //   calls a declared extern function stays with the extern route and keeps
    //   that route's existing refusal for the statements it does not model. That
    //   is the answer to "does a buffer cross the extern boundary" — no, and the
    //   refusal is an *existing* route's rather than a new one this card wrote.
    //   A C04 pointer is an extentless `u8` projection, so admitting the
    //   crossing would discard the extent this route carries and replace it with
    //   a premise (`STD-BUFFER-EXTERN`).
    // * **Before the standard-IO route**, so a module that both buffers and
    //   prints reaches this route, which models both — its line-producing events
    //   are `source.std-io.v1`'s own, read by that route's own reader and
    //   rendered by that route's own emitter. A module that only prints fails
    //   this conjunction and falls through byte for byte.
    //
    // Every concurrent route is selected above both, so a module that buffers
    // and spawns never reaches here and keeps its `Q600`.
    //
    // Card `AGGREGATES-V1`, and **the whole of the route-selection change it
    // makes**: a module that declares a record type reaches the buffer route,
    // which since that card models the aggregate as well. The condition is the
    // **declaration and nothing else**, on the `STRING-CONCAT` precedent, so a
    // module that holds a buffer without declaring one keeps the route it had
    // and a module that declares none travels an identical code path. No
    // program written before this card can satisfy the guard, because
    // `type NAME { … }` did not parse at all: the one new door is opened only by
    // the one new spelling.
    //
    // It is placed **here**, beside the buffer guard, for that route's own
    // reasons: every concurrent route, the extern route and the event route are
    // selected above it, so a module that declares a record and spawns, calls
    // out or dispatches keeps that route's existing refusal — measured, not
    // assumed.
    SourceRoute {
        name: "source.buffer.v1",
        select: |parsed| {
            Selection::of(
                source_std_buffer::has_source_buffer(parsed)
                    || source_record::has_source_record(parsed),
            )
        },
        lower: |input| {
            entry_only(
                compile_buffer_source(
                    input.parsed,
                    input.typed,
                    input.mode,
                    input.options,
                    input.ranges,
                    input.target,
                ),
                BUFFER,
                input,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `HTTP-QUIRIN-V1`, selected after the buffer route above and **before**
    // both routes it composes. All three neighbours are deliberate, on exactly
    // the principle every sibling states about the others:
    //
    // * **After the buffer route**, so a module that holds an owned buffer and
    //   declares an HTTP message stays with the buffer route and keeps that
    //   route's existing refusal. A message body that is a buffer's extent
    //   rather than a literal is the named follow-on `HTTP-V1-BUFFER-CONTENT`.
    // * **Before the string route**, so a module that declares a message and
    //   binds a payload with `let s = "…";` reaches this route, which models
    //   both — its bound views are `source.string.v1`'s own literal
    //   constructions, decoded by that route's own decoder. A module that only
    //   slices falls through byte for byte, and a module that *slices* while
    //   doing HTTP is refused here by name (`HTTP-V1-STRING-COMPOSE`): this
    //   route models no `len`, no slice and no rendered text.
    // * **Before the socket route**, so a module that declares a message and
    //   sends it reaches this route, which models both — its transport
    //   statements are `source.socket.v1`'s own, read by **that route's own
    //   reader** (`source_socket::socket_statement`) and checked by that route's
    //   own unedited checker. A module that only sockets fails this conjunction
    //   and falls through byte for byte.
    //
    // The event route is selected above this one, so a module that dispatches and
    // does HTTP stays with `source.event.v1` (`HTTP-V1-EVENT-COMPOSE`); the
    // extern route is above all of them, and every concurrent route above that,
    // so a module that does HTTP and spawns keeps its `Q600`.
    SourceRoute {
        name: "source.http.v1",
        select: |parsed| Selection::of(source_http::has_source_http(parsed)),
        lower: |input| {
            entry_only(
                compile_http_source(
                    input.parsed,
                    input.typed,
                    input.mode,
                    input.options,
                    input.ranges,
                ),
                HTTP,
                input,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `STRING-SLICE`, selected after the buffer route above and before the
    // standard-IO route below. Both neighbours are deliberate, on exactly the
    // principle its two siblings state about each other:
    //
    // * **After the buffer route**, so a module that both holds an owned buffer
    //   and constructs a string view stays with the buffer route and keeps that
    //   route's existing refusal for the statements it does not model. That is
    //   the v1 answer to amendment §5 — `std.buffer` is not implicitly
    //   convertible to `string` — and the refusal is an *existing* route's
    //   rather than a new one this card wrote. Admitting the composition needs a
    //   checked UTF-8 borrow or freeze with mutation exclusion, which is the
    //   named follow-on `STRING-SLICE-BUFFER-BORROW`.
    // * **Before the standard-IO route**, so a module that both slices and
    //   prints reaches this route, which models both — its line-producing events
    //   are `source.std-io.v1`'s own, read by that route's own reader and
    //   rendered by that route's own emitter, over **scalars only**. A module
    //   that only prints fails this conjunction and falls through byte for byte.
    //
    // The extern route is selected above both, so a module that constructs a
    // string and calls a declared extern function never reaches here and keeps
    // that route's existing refusal. That is amendment §6's crossing boundary,
    // measured rather than asserted: no C04 pointer contract transports this
    // aggregate or its extent, and supplying one is `STRING-SLICE-ABI`. Every
    // concurrent route is selected above all three, so a module that strings and
    // spawns keeps its `Q600`.
    SourceRoute {
        name: "source.string.v1",
        select: |parsed| Selection::of(source_std_string::has_source_string(parsed)),
        lower: lower_string,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `QUIRIN-SOCKET-V1`, selected after the buffer and string routes above
    // and before the standard-IO route below. All three neighbours are
    // deliberate, on exactly the principle the two siblings state about each
    // other:
    //
    // * **After the buffer route**, so a module that both holds an owned buffer
    //   and opens a socket stays with the buffer route and keeps that route's
    //   existing refusal for the statements it does not model. That is the v1
    //   answer to "can a send carry a buffer" — no, and the refusal is an
    //   *existing* route's rather than a new one this card wrote. Admitting it
    //   needs a payload that is a borrowed extent rather than a list of
    //   check-time constants, which is the named follow-on
    //   `SOCKET-BUFFER-PAYLOAD`.
    // * **After the string route**, for the same reason and with the same
    //   follow-on name for text (`SOCKET-STRING-PAYLOAD`).
    // * **Before the standard-IO route**, so a module that both receives a byte
    //   and prints it reaches this route, which models both — its line-producing
    //   statements are `source.std-io.v1`'s own, read by that route's own reader
    //   and rendered by that route's own emitter, over **scalars only**. A module
    //   that only prints fails this conjunction and falls through byte for byte.
    //
    // The event route is selected above this one, so a module that dispatches and
    // sockets stays with `source.event.v1` and keeps its refusal; composing the
    // two is the named follow-on `SOCKET-EVENT-COMPOSE`. The extern route is
    // selected above all four, and every concurrent route above that, so a module
    // that sockets and spawns keeps its `Q600`.
    SourceRoute {
        name: "source.socket.v1",
        select: |parsed| Selection::of(source_socket::has_source_socket(parsed)),
        lower: lower_socket,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `QR-PRINT-ROUTE-MATCHES-DECLARATION`: selected by a `print (` call
    // at a statement head inside a source function body, never by the token
    // pair alone (a declaration name, an `extern fn print(...)` or a contract
    // clause); an ambiguous pair refuses at selection rather than being routed.
    SourceRoute {
        name: "source.scalar-calls.v1",
        select: |parsed| match source_scalar_calls::print_call_selection(parsed) {
            PrintCallSelection::Absent => Selection::Absent,
            PrintCallSelection::Selected => Selection::Selected,
            PrintCallSelection::Ambiguous(refusal) => Selection::Refused(refusal),
        },
        lower: lower_scalar_calls,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // Card `W48-QR-SCALAR-TYPES`, and the whole of the route-selection change
    // it makes. A module with `std.io` lines and scalar-call syntax reaches the
    // scalar route, which since that card renders those lines through
    // `source.std-io.v1`'s own reader, **only when the std.io lowering refuses
    // it**. Every module the std.io lowering admits keeps that route byte for
    // byte — including an std.io entry beside an ignored helper function,
    // which that route admits today — so the door opens only for modules that
    // were refused. Both lowerings are source-only, so the guard names this
    // entry for either.
    SourceRoute {
        name: "source.std-io.v1",
        select: |parsed| Selection::of(source_std_io::has_source_std_io(parsed)),
        lower: |input| {
            if source_scalar_calls::has_scalar_calls(input.parsed)
                && (std_io_lowering_refuses(input.parsed, input.typed, input.mode, input.options)
                    || has_unmodeled_function(input.parsed, input.typed, input.mode, input.options))
            {
                return lower_scalar_calls(input);
            }
            entry_only(
                compile_std_io_source(
                    input.parsed,
                    input.typed,
                    input.mode,
                    input.options,
                    input.ranges,
                ),
                STD_IO,
                input,
            )
        },
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    SourceRoute {
        name: "source.scalar-calls.v1",
        select: |parsed| Selection::of(source_scalar_calls::has_scalar_calls(parsed)),
        lower: lower_scalar_calls,
        source_only: true,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
    // The sequential cleanup route (§G02), the one route that reads contract
    // modules, selected when no other route is. Card `W48-QR-SCALAR-TYPES-R2`,
    // item 0: it checks every parameter annotation it admits, after its own
    // passes so that every refusal it gave before keeps its bytes. Its
    // lowering builds `CheckedSourceModule::Sequential` on every admission.
    SourceRoute {
        name: "the sequential cleanup route",
        select: |_| Selection::Selected,
        lower: |input| {
            compile_sequential_source(
                input.parsed,
                input.typed,
                input.mode,
                input.options,
                input.ranges,
            )
            .and_then(|module| {
                check_sequential_parameters(input.parsed, input.target, input.ranges)
                    .map(|()| module)
            })
        },
        source_only: false,
        closure: ClosureRule::Refuses(CLOSURE_ELSEWHERE),
    },
];

/// The module's route: the first entry of [`SOURCE_ROUTES`] whose selector
/// does not read [`Selection::Absent`], with what it read. Selectors are pure
/// and are evaluated in table order up to the first match, as the if-chain
/// evaluated its conditions.
pub(super) fn select(parsed: &SourceProgram) -> (&'static SourceRoute, Selection) {
    let [others @ .., last] = &SOURCE_ROUTES;
    others
        .iter()
        .find_map(|route| match (route.select)(parsed) {
            Selection::Absent => None,
            selection => Some((route, selection)),
        })
        .unwrap_or((last, Selection::Selected))
}

#[cfg(test)]
mod tests {
    use super::{CLOSURE_ELSEWHERE, ClosureRule, SOURCE_ROUTES, Selection};

    /// The route order the driver's if-chain had at base `1a399e6a1`, by the
    /// name each arm's guard refusal states. Pinned here and in Lean
    /// (`QCore.RouteSelection.driverTable`); an entry added to the table must
    /// be added to both.
    const ORDER: [(&str, bool); 22] = [
        ("source.ffi.v1", true),
        ("source.bitfield-concurrency.v1", true),
        ("source.epoch.tasks.v1", true),
        ("source.epoch.bound.v1", true),
        ("source.mutex.guard.v1", true),
        ("source.std-io-tasks.v1", true),
        ("the scoped-task route", true),
        ("source.std-io-extern.v1", true),
        ("source.extern-call.v1", true),
        ("source.socket.v1", true),
        ("source.event.v1", true),
        ("source.string.v1", true),
        ("source.resource-calls.v1", true),
        ("source.scalar-calls.v1", true),
        ("source.buffer.v1", true),
        ("source.http.v1", true),
        ("source.string.v1", true),
        ("source.socket.v1", true),
        ("source.scalar-calls.v1", true),
        ("source.std-io.v1", true),
        ("source.scalar-calls.v1", true),
        ("the sequential cleanup route", false),
    ];

    #[test]
    fn the_table_is_the_pinned_order() {
        let table = SOURCE_ROUTES
            .iter()
            .map(|route| (route.name, route.source_only))
            .collect::<Vec<_>>();
        assert_eq!(table, ORDER);
    }

    /// The completeness check after routing and the guard in
    /// `compile_scalar_calls` were deleted because a contract module is
    /// lowered only by an entry that is not source-only: this pins that the
    /// sequential route is the only such entry and stands last, so an entry
    /// added anywhere with `source_only: false` fails here.
    #[test]
    fn only_the_last_entry_reads_contract_modules() {
        let [others @ .., last] = &SOURCE_ROUTES;
        assert!(!last.source_only);
        assert_eq!(last.name, "the sequential cleanup route");
        assert!(others.iter().all(|route| route.source_only));
    }

    /// The closure refusal's order: the first seven entries carry the
    /// refusals the closure if-chain named, the two scoped-task routes host
    /// closures, and every later entry refuses with the general message.
    #[test]
    fn closure_rules_follow_the_table() {
        let hosts = SOURCE_ROUTES
            .iter()
            .map(|route| route.closure == ClosureRule::Hosts)
            .collect::<Vec<_>>();
        let expected = (0..SOURCE_ROUTES.len())
            .map(|index| index == 5 || index == 6)
            .collect::<Vec<_>>();
        assert_eq!(hosts, expected);
        assert!(
            SOURCE_ROUTES[7..]
                .iter()
                .all(|route| route.closure == ClosureRule::Refuses(CLOSURE_ELSEWHERE))
        );
        assert_eq!(SOURCE_ROUTES[2].closure, SOURCE_ROUTES[3].closure);
    }

    /// The sequential route's selector holds for every module, so first match
    /// always finds an entry.
    #[test]
    fn the_last_selector_always_holds() {
        let parsed =
            qcore_frontend::frontend::parse_source_program("fn main() -> i32 { return 0; }")
                .expect("parses");
        let [.., last] = &SOURCE_ROUTES;
        assert!(matches!((last.select)(&parsed), Selection::Selected));
        let (route, _) = super::select(&parsed);
        assert_eq!(route.name, "the sequential cleanup route");
    }
}
