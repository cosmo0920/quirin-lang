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

//! Explicit selection of bundled semantic modules. No module is implicit.
//!
//! Imports expose vocabulary; existing checkers still discharge every operation.
//! Resolution is exact and total-or-reject, with no filesystem or provider fallback.

pub const CONCURRENCY_MODULE: &str = "std.concurrency";

/// Card `source.extern-call.v1`: calling a declared extern function is a
/// file-local opt-in, exactly like concurrency. The module grants the *call*
/// vocabulary only; the callee's contract still has to be selected explicitly
/// and admitted by the resource-contract v1 loader, and every custody
/// conclusion drawn through it remains a declared assumption.
pub const EXTERN_MODULE: &str = "std.extern";

/// Card `UNSAFE-LINKAGE`: declaring an extern function that has **no** contract
/// at all, so that an external library can be linked without one. It grants the
/// *declaration* only; the call still needs [`EXTERN_MODULE`], so an
/// unchecked-linkage file names both modules. Holding this module changes
/// nothing about a contracted callee, and holding [`EXTERN_MODULE`] alone keeps
/// an uncontracted callee refused.
pub const UNCHECKED_EXTERN_MODULE: &str = "std.extern.unsafe";

/// Card `STD-MODULES`: the rendering vocabulary shared by `std.io` and
/// `std.logger`. It names how a scalar becomes the bytes of a line and nothing
/// else — it opens no stream, so importing it alone produces no output.
pub const FMT_MODULE: &str = "std.fmt";

/// Card `STD-MODULES`: writing a rendered line to the standard output stream.
pub const IO_MODULE: &str = "std.io";

/// Card `STD-EVENT`: the checked event vocabulary — one reactor context, a
/// registration on a program-created endpoint, a declared dispatch turn and
/// teardown — over the existing `event_reactor`/`os_endpoint` models
/// (`source.event.v1`, `docs/design/STD_EVENT_V1.md`).
///
/// A file-local opt-in exactly like `std.concurrency`: the context is
/// unavailable by default and importing another compilation input never grants
/// this file the vocabulary. The module grants no threading — v1's reactor
/// profile is single-threaded and a cross-thread activation is refused by the
/// reactor model itself.
pub const EVENT_MODULE: &str = "std.event";

/// Card `STD-MODULES`: writing a level-tagged rendered line to the standard
/// error stream, plus the `flush` sink-handoff barrier. Separate from
/// `std.io` because a diagnostic stream and a data stream are different
/// capabilities; neither grants the other.
pub const LOGGER_MODULE: &str = "std.logger";

/// Card `STD-BUFFER`: a checked, owned, extent-carrying byte buffer
/// (`source.buffer.v1`). It grants the acquisition, the indexed byte access,
/// the extent read and the release, and nothing else — no rendering, so a
/// module that both buffers and prints names this module *and* the standard-IO
/// ones. A byte buffer is not a string: this module grants no text, no
/// encoding and no literal, because none exists.
///
/// The **module** keeps the name `std.buffer` even though the constructor is
/// spelled `bytes(N)`: a module name is a dotted path in an `import`
/// declaration and never a reserved word, so the measured `buffer` collision
/// (555 source-corpus occurrences, in binding position in shipped fixtures)
/// does not reach it.
pub const BUFFER_MODULE: &str = "std.buffer";

/// Card `STRING-SLICE`: immutable UTF-8 **string views**
/// (`source.string.v1`, `docs/design/STRING_SLICE_V1.md`). It grants the string
/// literal, the byte-length query and the checked slice, and nothing else.
///
/// A ninth independent file-local opt-in. It is granted by no other module and
/// grants none in return — in particular it is **not** `std.buffer`: a buffer is
/// an owned, mutable, explicitly released `{identity, extent, custody}` and a
/// string is an immutable, custody-free `{ptr, len}` view of storage it does not
/// own. Neither is implicitly convertible to the other, and the conversion
/// itself is the named follow-on `STRING-SLICE-BUFFER-BORROW`.
///
/// It grants no **output**: `std.io` and `std.logger` still accept a rendered
/// *scalar* only, and making them accept text is the named follow-on
/// `STRING-SLICE-IO` (amendment §7). A module that slices and prints a length
/// therefore names this module *and* the output modules.
pub const STRING_MODULE: &str = "std.string";

/// Card `QUIRIN-SOCKET-V1`: the checked TCP and UDP socket vocabulary
/// (`source.socket.v1`). It grants socket creation, bind, listen, connect,
/// accept, send, receive and close over the **loopback interface**, and nothing
/// else.
///
/// A tenth independent file-local opt-in, on exactly the terms
/// `std.concurrency` and `std.event` state: the vocabulary is unavailable by
/// default and importing another compilation input never grants this file the
/// capability.
///
/// The name is `std.socket` and deliberately **not** `std.net`. Both words
/// census clean (card report §4), so the choice is argued: a module name in this
/// language claims exactly what it grants, and what this one grants is the BSD
/// socket vocabulary. `std.net` would name a capability space — addressing, name
/// resolution, interfaces, TLS — of which v1 implements none, and it stays
/// available for whichever card supplies one.
///
/// It grants no **output**: `std.io` and `std.logger` still own every line, so a
/// module that receives a byte and prints it names this module *and* the output
/// ones. It grants no **buffer**: a payload is a bracketed list of check-time
/// byte constants, and sending the contents of a `std.buffer` acquisition is the
/// named follow-on `SOCKET-BUFFER-PAYLOAD`. It grants no **reactor**:
/// registering a socket with `source.event.v1` is `SOCKET-EVENT-COMPOSE`.
pub const SOCKET_MODULE: &str = "std.socket";

/// Card `HTTP-QUIRIN-V1`: the checked HTTP/1.1 message vocabulary
/// (`source.http.v1`) — a request line or a status line, header fields, CRLF
/// framing and a fixed literal content, all constructed and checked at compile
/// time against RFC 9112.
///
/// An eleventh independent file-local opt-in, and the composition rule it states
/// is the point rather than a detail:
///
/// * **It grants no transport.** A message is a value; putting one on a wire is
///   `std.socket`'s `send`/`recv`, so a program that actually speaks HTTP names
///   **both** modules. That is `STRING-SLICE-IO`'s rule ("what changed is that a
///   file naming both can render text") applied to a protocol.
/// * **It grants no text.** A quoted token at a method, a target, a field name,
///   a field value or a content position is *this module's own syntax* and never
///   a `string` value: nothing can bind it, slice it, measure it or render it.
///   `STRING_SLICE_V1.md` §2 says so in as many words — "recognizing quoted
///   tokens for structural syntax … does not construct a string value and MUST
///   NOT acquire this new import requirement" — so a pure HTTP program needs no
///   `std.string`. A program that *binds* a payload with `let s = "…";` does
///   construct one, and then it names `std.string` too.
/// * **It grants no output.** A transcript line is `std.io`'s or
///   `std.logger`'s.
///
/// The name is `std.http` and claims exactly what it grants: an HTTP/1.1 message
/// subset. It is not `std.net` and not `std.web`.
pub const HTTP_MODULE: &str = "std.http";

/// Card `W48-QR-C-STRUCT-IMPORT` (user decision 1): the file-level opt-in to
/// the unsafe FFI — importing a C struct with `struct! { … }` and calling C
/// through `[| quant #c_ffi |]`-marked contracts. The precedent is
/// [`UNCHECKED_EXTERN_MODULE`]: a trust act, granted by no other module and
/// granting nothing else. A call still needs [`EXTERN_MODULE`], exactly as an
/// unchecked-linkage call does, and rendering still needs the output modules.
/// Importing it marks the **whole file** unsafe: every result from the file
/// carries the unsafe-FFI assumption (`qcore_core::ffi`).
pub const FFI_MODULE: &str = "std.ffi.unsafe";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModuleImports {
    concurrency: bool,
    extern_calls: bool,
    unchecked_extern_calls: bool,
    fmt: bool,
    io: bool,
    logger: bool,
    buffer: bool,
    event: bool,
    string: bool,
    socket: bool,
    http: bool,
    ffi: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownModule(pub String);

impl std::fmt::Display for UnknownModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown module `{}`", self.0)
    }
}

impl std::error::Error for UnknownModule {}

impl ModuleImports {
    pub fn resolve<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self, UnknownModule> {
        let mut imports = Self::default();
        for name in names {
            match name {
                CONCURRENCY_MODULE => imports.concurrency = true,
                EXTERN_MODULE => imports.extern_calls = true,
                UNCHECKED_EXTERN_MODULE => imports.unchecked_extern_calls = true,
                FMT_MODULE => imports.fmt = true,
                IO_MODULE => imports.io = true,
                LOGGER_MODULE => imports.logger = true,
                BUFFER_MODULE => imports.buffer = true,
                EVENT_MODULE => imports.event = true,
                STRING_MODULE => imports.string = true,
                SOCKET_MODULE => imports.socket = true,
                HTTP_MODULE => imports.http = true,
                FFI_MODULE => imports.ffi = true,
                _ => return Err(UnknownModule(name.to_owned())),
            }
        }
        Ok(imports)
    }

    pub fn concurrency(self) -> bool {
        self.concurrency
    }

    /// Whether this file may call a declared extern function at all.
    pub fn extern_calls(self) -> bool {
        self.extern_calls
    }

    /// Whether this file may declare an extern function with no contract at
    /// all (card `UNSAFE-LINKAGE`). Never granted by [`EXTERN_MODULE`].
    pub fn unchecked_extern_calls(self) -> bool {
        self.unchecked_extern_calls
    }

    /// Whether this file may name a rendering (`dec`, `hex`).
    pub fn fmt(self) -> bool {
        self.fmt
    }

    /// Whether this file may write a rendered line to standard output.
    pub fn io(self) -> bool {
        self.io
    }

    /// Whether this file may write a level-tagged line to standard error, or
    /// hand the logger's stream to its sink with `flush`.
    pub fn logger(self) -> bool {
        self.logger
    }

    /// Whether this file may acquire, index, measure or release an owned byte
    /// buffer. Granted by no other module: `std.io` never grants it and it
    /// never grants `std.io`.
    pub fn buffer(self) -> bool {
        self.buffer
    }

    /// Whether this file may open an event context and register interest on a
    /// descriptor (card `STD-EVENT`).
    pub fn event(self) -> bool {
        self.event
    }

    /// Whether this file may construct a string view, query its byte length or
    /// slice it (card `STRING-SLICE`). Granted by no other module: `std.buffer`
    /// never grants it and it never grants `std.buffer`.
    pub fn string(self) -> bool {
        self.string
    }

    /// Whether this file may create, bind, listen, connect, accept, send on,
    /// receive from or close a socket (card `QUIRIN-SOCKET-V1`). Granted by no
    /// other module, and granting none in return.
    pub fn socket(self) -> bool {
        self.socket
    }

    /// Whether this file may declare an HTTP/1.1 request or response, add a
    /// header field to one or set its content (card `HTTP-QUIRIN-V1`). Granted
    /// by no other module, and granting none in return — in particular it opens
    /// no socket and constructs no `string`.
    pub fn http(self) -> bool {
        self.http
    }

    /// Whether this file may import a C struct or call C through a
    /// `#c_ffi`-marked contract (card `W48-QR-C-STRUCT-IMPORT`). Granted by no
    /// other module.
    pub fn ffi(self) -> bool {
        self.ffi
    }
}
