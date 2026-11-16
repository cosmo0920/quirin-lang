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

//! The plugin's own checker: per binding (one libcurl function), lexical
//! obligations over the pinned release's source that, together, are why the
//! function's resource-v1 contract (`contracts/libcurl/libcurl.qv`) holds.
//!
//! An obligation is one of six decidable shapes over the tokens of a file
//! or of one function's body ([`Obligation`]). The checker answers `Safe`
//! for a binding exactly when the tree is the pinned release (every file
//! [`crate::pin::FILES`] names has its pinned digest) and every obligation
//! of the binding holds; otherwise `Unknown`, with the obligation that
//! failed as the reason. A function whose contract the resource-v1
//! vocabulary cannot state carries no contract: its binding is `Unknown`
//! with the reason, and the witnesses for that reason are checked and
//! recorded as evidence. The checker never answers `Unsafe`: it looks for
//! no counterexample.
//!
//! What the answer is worth is the firewall's to decide (contract
//! specification C11.1, C11.2): no trusted-core checker accepts this
//! plugin's evidence, so a `Safe` is published as `Pending`
//! (`plugin-claim-uncertified`) and never as `Safe`. The obligations'
//! soundness — that a shape implies its clause — and the named assumptions
//! are this checker's premises (`QCore.LibcurlContractPlugin`).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::pin;
use crate::source::{self, Token};

/// The checker's own answer for one binding. There is no `Unsafe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Safe,
    Unknown,
}

impl Answer {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Unknown => "Unknown",
        }
    }
}

/// One lexical obligation. Patterns are token texts separated by spaces.
#[derive(Clone, Copy, Debug)]
pub enum Obligation {
    /// `function` is defined exactly once in `file`, with no directive in
    /// its body, and its header spells exactly `header`.
    Defined {
        file: &'static str,
        function: &'static str,
        header: &'static str,
    },
    /// `function` is declared exactly once in `file`, spelled `declaration`.
    Declared {
        file: &'static str,
        function: &'static str,
        declaration: &'static str,
    },
    /// The body of `function` contains `tokens` contiguously.
    Contains {
        file: &'static str,
        function: &'static str,
        tokens: &'static str,
    },
    /// Every occurrence of `identifier` in the body of `function` lies
    /// inside an occurrence of one of `patterns`.
    OnlyIn {
        file: &'static str,
        function: &'static str,
        identifier: &'static str,
        patterns: &'static [&'static str],
    },
    /// The body of `function` has at least one `return`, and the
    /// expression of every `return` is one of `expressions` (`""` for a
    /// bare `return;`).
    Returns {
        file: &'static str,
        function: &'static str,
        expressions: &'static [&'static str],
    },
    /// `file` contains `tokens` contiguously (a definition outside a
    /// function: a macro, a file-scope initializer, a case of a switch).
    FileHas {
        file: &'static str,
        tokens: &'static str,
    },
}

impl Obligation {
    #[must_use]
    pub fn file(&self) -> &'static str {
        match self {
            Self::Defined { file, .. }
            | Self::Declared { file, .. }
            | Self::Contains { file, .. }
            | Self::OnlyIn { file, .. }
            | Self::Returns { file, .. }
            | Self::FileHas { file, .. } => file,
        }
    }

    /// The obligation as one sentence.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Defined {
                file,
                function,
                header,
            } => format!("{file}: `{function}` is defined once as `{header}`"),
            Self::Declared {
                file,
                function,
                declaration,
            } => format!("{file}: `{function}` is declared once as `{declaration}`"),
            Self::Contains {
                file,
                function,
                tokens,
            } => format!("{file}: the body of `{function}` contains `{tokens}`"),
            Self::OnlyIn {
                file,
                function,
                identifier,
                patterns,
            } => format!(
                "{file}: in the body of `{function}`, `{identifier}` occurs only in {}",
                patterns
                    .iter()
                    .map(|pattern| format!("`{pattern}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Returns {
                file,
                function,
                expressions,
            } => format!(
                "{file}: every `return` of `{function}` returns one of {}",
                expressions
                    .iter()
                    .map(|expression| format!("`{expression}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::FileHas { file, tokens } => format!("{file} contains `{tokens}`"),
        }
    }
}

/// What a binding is: a contract and the obligations it rests on, or no
/// contract and why.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Contracted {
        /// The `ensures` clauses of the function's contract, as written.
        clauses: &'static [&'static str],
        obligations: &'static [Obligation],
    },
    Uncontracted {
        /// Why the resource-v1 vocabulary cannot state the function's
        /// contract.
        reason: &'static str,
        /// Source facts that witness the reason.
        witnesses: &'static [Obligation],
    },
}

/// One binding: a libcurl function the ranking names.
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub function: &'static str,
    /// The ranking position: `curl_global_init` first, then the librdkafka
    /// census's twelve real functions by pairs.
    pub rank: u32,
    pub kind: Kind,
}

/// The named assumptions every answer rests on, beside the obligations.
pub const ASSUMPTIONS: &[(&str, &str)] = &[
    (
        "upstream-release",
        "the libcurl the audited program links is upstream 8.18.0 (the pinned tarball); \
         Ubuntu 8.18.0-1ubuntu2.8's patches touch no function a Safe answer rests on",
    ),
    (
        "release-build",
        "libcurl is built with BUILDING_LIBCURL and without CURLDEBUG, so the curlx_* \
         allocator macros are the Curl_c* function pointers (lib/curl_setup.h)",
    ),
    (
        "default-allocators",
        "the program never calls curl_global_init_mem, so Curl_cmalloc, Curl_ccalloc, \
         Curl_crealloc, Curl_cstrdup and Curl_cfree stay the C library's malloc, calloc, \
         realloc, strdup and free (lib/easy.c)",
    ),
    (
        "libc-c-standard-abi",
        "the C library's allocation functions behave as the C standard specifies",
    ),
    (
        "heap-kind",
        "heap custody does not name the releaser (contract specification C04: a position \
         with no kind clause is heap), so a libcurl object released by another heap \
         releaser is not told apart, as for every heap contract",
    ),
];

const EASY: &str = "lib/easy.c";
const SLIST: &str = "lib/slist.c";
const ESCAPE: &str = "lib/escape.c";
const URLAPI: &str = "lib/urlapi.c";
const SETUP: &str = "lib/curl_setup.h";
const DYNBUF: &str = "lib/curlx/dynbuf.c";
const CURL_H: &str = "include/curl/curl.h";
const URLAPI_H: &str = "include/curl/urlapi.h";
const EASY_H: &str = "include/curl/easy.h";

/// The allocator facts every heap obligation shares.
const ALLOCATOR_MACROS: [Obligation; 6] = [
    Obligation::FileHas {
        file: SETUP,
        tokens: "# define curlx_strdup ( ptr ) Curl_cstrdup ( ptr )",
    },
    Obligation::FileHas {
        file: SETUP,
        tokens: "# define curlx_malloc ( size ) Curl_cmalloc ( size )",
    },
    Obligation::FileHas {
        file: SETUP,
        tokens: "# define curlx_calloc ( nbelem , size ) Curl_ccalloc ( nbelem , size )",
    },
    Obligation::FileHas {
        file: SETUP,
        tokens: "# define curlx_realloc ( ptr , size ) Curl_crealloc ( ptr , size )",
    },
    Obligation::FileHas {
        file: SETUP,
        tokens: "# define curlx_free ( ptr ) Curl_cfree ( ptr )",
    },
    Obligation::FileHas {
        file: EASY,
        tokens: "Curl_cfree = ( curl_free_callback ) free ;",
    },
];

/// The bindings, in ranking order.
pub const BINDINGS: &[Binding] = &[
    Binding {
        function: "curl_global_init",
        rank: 1,
        kind: Kind::Contracted {
            clauses: &["resource_borrowed(flags)", "resource_return_none()"],
            obligations: &[
                Obligation::Defined {
                    file: EASY,
                    function: "curl_global_init",
                    header: "CURLcode curl_global_init ( long flags )",
                },
                Obligation::Declared {
                    file: CURL_H,
                    function: "curl_global_init",
                    declaration: "CURL_EXTERN CURLcode curl_global_init ( long flags ) ;",
                },
                // CURLcode is an enumeration: the result is a scalar.
                Obligation::FileHas {
                    file: CURL_H,
                    tokens: "typedef enum { CURLE_OK = 0 ,",
                },
                Obligation::Returns {
                    file: EASY,
                    function: "curl_global_init",
                    expressions: &["result"],
                },
                Obligation::OnlyIn {
                    file: EASY,
                    function: "curl_global_init",
                    identifier: "flags",
                    patterns: &["global_init ( flags , TRUE )"],
                },
            ],
        },
    },
    Binding {
        function: "curl_slist_append",
        rank: 2,
        kind: Kind::Contracted {
            clauses: &[
                "resource_owned(list, nullable)",
                "resource_borrowed(data)",
                "resource_return_reallocated(list, nullable)",
            ],
            obligations: &[
                Obligation::Defined {
                    file: SLIST,
                    function: "curl_slist_append",
                    header: "struct curl_slist * curl_slist_append ( struct curl_slist * list , const char * data )",
                },
                Obligation::Declared {
                    file: CURL_H,
                    function: "curl_slist_append",
                    declaration: "CURL_EXTERN struct curl_slist * curl_slist_append ( struct curl_slist * list , const char * data ) ;",
                },
                // `data` is copied and never kept.
                Obligation::OnlyIn {
                    file: SLIST,
                    function: "curl_slist_append",
                    identifier: "data",
                    patterns: &["curlx_strdup ( data )"],
                },
                // A failed copy returns NULL before `list` is touched.
                Obligation::Contains {
                    file: SLIST,
                    function: "curl_slist_append",
                    tokens: "char * dupdata = curlx_strdup ( data ) ; if ( ! dupdata ) return NULL ; \
                             list = Curl_slist_append_nodup ( list , dupdata ) ; \
                             if ( ! list ) curlx_free ( dupdata ) ; return list ;",
                },
                Obligation::Returns {
                    file: SLIST,
                    function: "curl_slist_append",
                    expressions: &["NULL", "list"],
                },
                Obligation::Defined {
                    file: SLIST,
                    function: "Curl_slist_append_nodup",
                    header: "struct curl_slist * Curl_slist_append_nodup ( struct curl_slist * list , const char * data )",
                },
                // The node is allocated first; on failure NULL returns
                // before `list` is read; a NULL `list` makes the node the
                // list; otherwise the node is linked last and `list`, the
                // same head, is returned.
                Obligation::Contains {
                    file: SLIST,
                    function: "Curl_slist_append_nodup",
                    tokens: "new_item = curlx_malloc ( sizeof ( struct curl_slist ) ) ; \
                             if ( ! new_item ) return NULL ; new_item -> next = NULL ; \
                             new_item -> data = CURL_UNCONST ( data ) ; \
                             if ( ! list ) return new_item ; last = slist_get_last ( list ) ; \
                             last -> next = new_item ; return list ;",
                },
                Obligation::OnlyIn {
                    file: SLIST,
                    function: "Curl_slist_append_nodup",
                    identifier: "list",
                    patterns: &[
                        "if ( ! list ) return new_item ;",
                        "last = slist_get_last ( list ) ;",
                        "return list ;",
                    ],
                },
                Obligation::Returns {
                    file: SLIST,
                    function: "Curl_slist_append_nodup",
                    expressions: &["NULL", "new_item", "list"],
                },
                ALLOCATOR_MACROS[0],
                ALLOCATOR_MACROS[1],
                ALLOCATOR_MACROS[4],
                ALLOCATOR_MACROS[5],
            ],
        },
    },
    Binding {
        function: "curl_slist_free_all",
        rank: 3,
        kind: Kind::Contracted {
            clauses: &[
                "resource_released(list, nullable)",
                "resource_return_none()",
            ],
            obligations: &[
                Obligation::Defined {
                    file: SLIST,
                    function: "curl_slist_free_all",
                    header: "void curl_slist_free_all ( struct curl_slist * list )",
                },
                Obligation::Declared {
                    file: CURL_H,
                    function: "curl_slist_free_all",
                    declaration: "CURL_EXTERN void curl_slist_free_all ( struct curl_slist * list ) ;",
                },
                // NULL releases nothing; otherwise every node and its string
                // are freed, the last node ending the walk.
                Obligation::Contains {
                    file: SLIST,
                    function: "curl_slist_free_all",
                    tokens: "if ( ! list ) return ; item = list ; do { next = item -> next ; \
                             Curl_safefree ( item -> data ) ; curlx_free ( item ) ; \
                             item = next ; } while ( next ) ;",
                },
                Obligation::OnlyIn {
                    file: SLIST,
                    function: "curl_slist_free_all",
                    identifier: "list",
                    patterns: &["if ( ! list ) return ;", "item = list ;"],
                },
                Obligation::FileHas {
                    file: SETUP,
                    tokens: "# define Curl_safefree ( ptr ) do { curlx_free ( ptr ) ; ( ptr ) = NULL ; } while ( 0 )",
                },
                ALLOCATOR_MACROS[4],
                ALLOCATOR_MACROS[5],
            ],
        },
    },
    Binding {
        function: "curl_easy_setopt",
        rank: 4,
        kind: Kind::Uncontracted {
            reason: "what curl_easy_setopt does with its third argument depends on the option: \
                     CURLOPT_HTTPHEADER and CURLOPT_ERRORBUFFER (both passed by librdkafka) keep \
                     the caller's pointer in the handle, string options copy it, and callback \
                     options register a function; resource-v1 has no clause indexed by an \
                     option value over a variadic tail, so no contract is written",
            witnesses: &[
                Obligation::FileHas {
                    file: "lib/setopt.c",
                    tokens: "case CURLOPT_HTTPHEADER : s -> headers = slist ; break ;",
                },
                Obligation::FileHas {
                    file: "lib/setopt.c",
                    tokens: "case CURLOPT_ERRORBUFFER : s -> errorbuffer = ptr ; break ;",
                },
            ],
        },
    },
    Binding {
        function: "curl_easy_cleanup",
        rank: 5,
        kind: Kind::Uncontracted {
            reason: "curl_easy_cleanup releases the handle through Curl_close, which may run \
                     callbacks the handle registered (closing sockets, debug output) with \
                     caller data; the checker does not establish that it touches nothing of \
                     the caller's beyond the handle",
            witnesses: &[Obligation::Contains {
                file: EASY,
                function: "curl_easy_cleanup",
                tokens: "Curl_close ( & data ) ;",
            }],
        },
    },
    Binding {
        function: "curl_easy_escape",
        rank: 6,
        kind: Kind::Contracted {
            clauses: &[
                "resource_borrowed(handle)",
                "resource_borrowed(text)",
                "resource_borrowed(length)",
                "resource_return_owned(nullable)",
            ],
            obligations: &[
                Obligation::Defined {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    header: "char * curl_easy_escape ( CURL * data , const char * string , int inlength )",
                },
                Obligation::Declared {
                    file: CURL_H,
                    function: "curl_easy_escape",
                    declaration: "CURL_EXTERN char * curl_easy_escape ( CURL * handle , const char * string , int length ) ;",
                },
                // The handle is not used at all.
                Obligation::OnlyIn {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    identifier: "data",
                    patterns: &["( void ) data ;"],
                },
                // The string is only read.
                Obligation::OnlyIn {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    identifier: "string",
                    patterns: &["! string ||", "strlen ( string )", "* string ++"],
                },
                // Every result is NULL, a fresh copy, or the buffer the
                // function's own dynbuf allocated.
                Obligation::Returns {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    expressions: &["NULL", "curlx_strdup ( \"\" )", "curlx_dyn_ptr ( & d )"],
                },
                Obligation::Contains {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    tokens: "curlx_dyn_init ( & d , length * 3 + 1 ) ;",
                },
                Obligation::Contains {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    tokens: "if ( curlx_dyn_addn ( & d , & in , 1 ) ) return NULL ;",
                },
                Obligation::Contains {
                    file: ESCAPE,
                    function: "curl_easy_escape",
                    tokens: "if ( curlx_dyn_addn ( & d , out , 3 ) ) return NULL ;",
                },
                // An append that fails frees the buffer first, so a NULL
                // result leaves nothing allocated.
                Obligation::Returns {
                    file: DYNBUF,
                    function: "curlx_dyn_addn",
                    expressions: &["dyn_nappend ( s , mem , len )"],
                },
                Obligation::Contains {
                    file: DYNBUF,
                    function: "dyn_nappend",
                    tokens: "if ( fit > s -> toobig ) { curlx_dyn_free ( s ) ; return CURLE_TOO_LARGE ; }",
                },
                Obligation::Contains {
                    file: DYNBUF,
                    function: "dyn_nappend",
                    tokens: "void * p = curlx_realloc ( s -> bufr , a ) ; if ( ! p ) { curlx_dyn_free ( s ) ; return CURLE_OUT_OF_MEMORY ; }",
                },
                Obligation::Returns {
                    file: DYNBUF,
                    function: "dyn_nappend",
                    expressions: &["CURLE_TOO_LARGE", "CURLE_OUT_OF_MEMORY", "CURLE_OK"],
                },
                Obligation::Returns {
                    file: DYNBUF,
                    function: "curlx_dyn_ptr",
                    expressions: &["s -> bufr"],
                },
                ALLOCATOR_MACROS[0],
                ALLOCATOR_MACROS[3],
                ALLOCATOR_MACROS[5],
                Obligation::FileHas {
                    file: EASY,
                    tokens: "Curl_crealloc = ( curl_realloc_callback ) realloc ;",
                },
            ],
        },
    },
    Binding {
        function: "curl_easy_getinfo",
        rank: 7,
        kind: Kind::Uncontracted {
            reason: "what curl_easy_getinfo writes through its third argument depends on the \
                     information asked for: a number, or (CURLINFO_CONTENT_TYPE, which \
                     librdkafka asks for) a pointer into memory the handle owns; resource-v1 \
                     has no clause indexed by an option value over a variadic tail",
            witnesses: &[
                Obligation::Contains {
                    file: EASY,
                    function: "curl_easy_getinfo",
                    tokens: "paramp = va_arg ( arg , void * ) ; result = Curl_getinfo ( data , info , paramp ) ;",
                },
                Obligation::FileHas {
                    file: "lib/getinfo.c",
                    tokens: "case CURLINFO_CONTENT_TYPE : * param_charp = data -> info . contenttype ; break ;",
                },
            ],
        },
    },
    Binding {
        function: "curl_easy_perform",
        rank: 8,
        kind: Kind::Uncontracted {
            reason: "curl_easy_perform runs the transfer, which calls the callbacks the handle \
                     registered (librdkafka's write callback with its request as data) and \
                     writes into buffers the handle keeps; resource-v1 cannot name a callback \
                     registered by an earlier curl_easy_setopt",
            witnesses: &[
                Obligation::Declared {
                    file: EASY_H,
                    function: "curl_easy_perform",
                    declaration: "CURL_EXTERN CURLcode curl_easy_perform ( CURL * curl ) ;",
                },
                Obligation::Returns {
                    file: EASY,
                    function: "curl_easy_perform",
                    expressions: &["easy_perform ( data , FALSE )"],
                },
            ],
        },
    },
    Binding {
        function: "curl_url",
        rank: 9,
        kind: Kind::Contracted {
            clauses: &["resource_return_owned(nullable)"],
            obligations: &[
                Obligation::Defined {
                    file: URLAPI,
                    function: "curl_url",
                    header: "CURLU * curl_url ( void )",
                },
                Obligation::Declared {
                    file: URLAPI_H,
                    function: "curl_url",
                    declaration: "CURL_EXTERN CURLU * curl_url ( void ) ;",
                },
                Obligation::Returns {
                    file: URLAPI,
                    function: "curl_url",
                    expressions: &["curlx_calloc ( 1 , sizeof ( struct Curl_URL ) )"],
                },
                ALLOCATOR_MACROS[2],
                Obligation::FileHas {
                    file: EASY,
                    tokens: "Curl_ccalloc = ( curl_calloc_callback ) calloc ;",
                },
            ],
        },
    },
    Binding {
        function: "curl_url_cleanup",
        rank: 10,
        kind: Kind::Contracted {
            clauses: &[
                "resource_released(handle, nullable)",
                "resource_return_none()",
            ],
            obligations: &[
                Obligation::Defined {
                    file: URLAPI,
                    function: "curl_url_cleanup",
                    header: "void curl_url_cleanup ( CURLU * u )",
                },
                Obligation::Declared {
                    file: URLAPI_H,
                    function: "curl_url_cleanup",
                    declaration: "CURL_EXTERN void curl_url_cleanup ( CURLU * handle ) ;",
                },
                Obligation::Contains {
                    file: URLAPI,
                    function: "curl_url_cleanup",
                    tokens: "if ( u ) { free_urlhandle ( u ) ; curlx_free ( u ) ; }",
                },
                Obligation::OnlyIn {
                    file: URLAPI,
                    function: "curl_url_cleanup",
                    identifier: "u",
                    patterns: &["if ( u ) { free_urlhandle ( u ) ; curlx_free ( u ) ; }"],
                },
                // The handle's ten strings are freed with it.
                Obligation::Contains {
                    file: URLAPI,
                    function: "free_urlhandle",
                    tokens: "curlx_free ( u -> scheme ) ; curlx_free ( u -> user ) ; \
                             curlx_free ( u -> password ) ; curlx_free ( u -> options ) ; \
                             curlx_free ( u -> host ) ; curlx_free ( u -> zoneid ) ; \
                             curlx_free ( u -> port ) ; curlx_free ( u -> path ) ; \
                             curlx_free ( u -> query ) ; curlx_free ( u -> fragment ) ;",
                },
                ALLOCATOR_MACROS[4],
                ALLOCATOR_MACROS[5],
            ],
        },
    },
    Binding {
        function: "curl_url_get",
        rank: 11,
        kind: Kind::Uncontracted {
            reason: "curl_url_get hands back a string it allocates through an out-parameter, \
                     and resource-v1 reads an allocation through an out slot only with a \
                     resource kind (contract specification C04.6), none of which is a libcurl \
                     string; the checker does not establish the out-slot write",
            witnesses: &[Obligation::Declared {
                file: URLAPI_H,
                function: "curl_url_get",
                declaration: "CURL_EXTERN CURLUcode curl_url_get ( const CURLU * handle , CURLUPart what , char * * part , unsigned int flags ) ;",
            }],
        },
    },
    Binding {
        function: "curl_url_set",
        rank: 12,
        kind: Kind::Uncontracted {
            reason: "curl_url_set hands its string argument to URL-parsing helpers \
                     (parseurl_and_replace and others) that copy parts of it into the \
                     handle; the checker does not establish that none of them keeps the \
                     caller's pointer",
            witnesses: &[Obligation::Declared {
                file: URLAPI_H,
                function: "curl_url_set",
                declaration: "CURL_EXTERN CURLUcode curl_url_set ( CURLU * handle , CURLUPart what , const char * part , unsigned int flags ) ;",
            }],
        },
    },
];

/// The functions the ranking names but this plugin never binds: libcurl's
/// `typecheck-gcc.h` warning stubs, a separate question
/// (`W48-LIBCURL-TYPECHECK-STUBS`).
#[must_use]
pub fn is_typecheck_stub(function: &str) -> bool {
    function.starts_with("Wcurl_easy_setopt_err_")
        || function.starts_with("Wcurl_easy_getinfo_err_")
        || function.starts_with("Wcurl_multi_setopt_err_")
        || function.starts_with("Wcurl_share_setopt_err_")
}

/// The binding of `function`, if the plugin has one.
#[must_use]
pub fn binding(function: &str) -> Option<&'static Binding> {
    BINDINGS.iter().find(|binding| binding.function == function)
}

/// The outcome of one obligation.
#[derive(Clone, Debug)]
pub struct Checked {
    pub obligation: String,
    pub holds: bool,
    pub detail: String,
    pub file: String,
    pub lines: Option<(usize, usize)>,
}

/// The checker's answer for one binding.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub function: &'static str,
    pub rank: u32,
    pub answer: Answer,
    pub contracted: bool,
    pub reasons: Vec<String>,
    pub checked: Vec<Checked>,
}

/// The pinned files of a prepared tree, verified and tokenized.
pub struct Tree {
    files: BTreeMap<&'static str, Vec<Token>>,
    /// Whether every file was checked against the pin ([`Tree::open`]). A
    /// tree built from texts ([`Tree::from_texts`]) decides obligations but
    /// never establishes a binding.
    verified: bool,
}

impl Tree {
    /// Read every pinned file under `root`, refusing the tree unless each has
    /// its pinned digest.
    ///
    /// # Errors
    ///
    /// A mismatch (named file by file) or a file the tokenizer refuses.
    pub fn open(root: &Path) -> Result<Self, String> {
        let mismatches = pin::verify_tree(root);
        if !mismatches.is_empty() {
            return Err(format!(
                "the prepared tree {} is not the pinned libcurl {}: {}",
                root.display(),
                pin::VERSION,
                pin::describe(&mismatches)
            ));
        }
        let mut files = BTreeMap::new();
        for (file, _) in pin::FILES {
            let bytes = std::fs::read(root.join(file))
                .map_err(|error| format!("{}: {error}", root.join(file).display()))?;
            let text = String::from_utf8_lossy(&bytes);
            files.insert(
                *file,
                source::tokenize(&text).map_err(|error| format!("{file}: {error}"))?,
            );
        }
        Ok(Self {
            files,
            verified: true,
        })
    }

    /// A tree of the given texts, not checked against the pin: obligations
    /// are decided over it, and [`answer`] establishes nothing from it.
    ///
    /// # Errors
    ///
    /// A text the tokenizer refuses.
    pub fn from_texts(texts: &[(&'static str, &str)]) -> Result<Self, String> {
        let mut files = BTreeMap::new();
        for (file, text) in texts {
            files.insert(
                *file,
                source::tokenize(text).map_err(|error| format!("{file}: {error}"))?,
            );
        }
        Ok(Self {
            files,
            verified: false,
        })
    }

    /// Whether the tree was checked against the pin.
    #[must_use]
    pub fn verified(&self) -> bool {
        self.verified
    }

    fn tokens(&self, file: &str) -> Result<&[Token], String> {
        self.files
            .get(file)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("{file} is not a pinned file the checker reads"))
    }
}

fn lines(tokens: &[Token]) -> Option<(usize, usize)> {
    Some((tokens.first()?.line, tokens.last()?.line))
}

/// Decide one obligation over the tree.
#[must_use]
pub fn check(tree: &Tree, obligation: &Obligation) -> Checked {
    let result = decide(tree, obligation);
    let (holds, detail, lines) = match result {
        Ok((lines, detail)) => (true, detail, lines),
        Err(detail) => (false, detail, None),
    };
    Checked {
        obligation: obligation.describe(),
        holds,
        detail,
        file: obligation.file().to_owned(),
        lines,
    }
}

type Decided = Result<(Option<(usize, usize)>, String), String>;

fn decide(tree: &Tree, obligation: &Obligation) -> Decided {
    let tokens = tree.tokens(obligation.file())?;
    match *obligation {
        Obligation::Defined {
            function, header, ..
        } => {
            let found = source::definition(tokens, function)?;
            let spelled = source::spelled(&found.header);
            if spelled != header {
                return Err(format!("the header reads `{spelled}`"));
            }
            Ok((Some((found.first_line, found.last_line)), "defined".into()))
        }
        Obligation::Declared {
            function,
            declaration,
            ..
        } => {
            let found = source::declaration(tokens, function)?;
            let spelled = source::spelled(&found.tokens);
            if spelled != declaration {
                return Err(format!("the declaration reads `{spelled}`"));
            }
            Ok((lines(&found.tokens), "declared".into()))
        }
        Obligation::Contains {
            function,
            tokens: pattern,
            ..
        } => {
            let body = source::definition(tokens, function)?.body;
            let words = source::words(pattern);
            let starts = source::occurrences(&body, &words);
            let start = *starts
                .first()
                .ok_or_else(|| format!("the body of `{function}` does not contain it"))?;
            Ok((
                lines(&body[start..start + words.len()]),
                format!("{} occurrence(s)", starts.len()),
            ))
        }
        Obligation::OnlyIn {
            function,
            identifier,
            patterns,
            ..
        } => {
            let body = source::definition(tokens, function)?.body;
            let mut covered = vec![false; body.len()];
            for pattern in patterns {
                let words = source::words(pattern);
                for start in source::occurrences(&body, &words) {
                    for (offset, word) in words.iter().enumerate() {
                        if *word == identifier {
                            covered[start + offset] = true;
                        }
                    }
                }
            }
            let uses: Vec<usize> = body
                .iter()
                .enumerate()
                .filter(|(_, token)| token.text == identifier)
                .map(|(index, _)| index)
                .collect();
            if let Some(stray) = uses.iter().find(|index| !covered[**index]) {
                return Err(format!(
                    "`{identifier}` also occurs at line {} outside the allowed shapes",
                    body[*stray].line
                ));
            }
            Ok((
                lines(&body),
                format!("{} occurrence(s), all in the allowed shapes", uses.len()),
            ))
        }
        Obligation::Returns {
            function,
            expressions,
            ..
        } => {
            let body = source::definition(tokens, function)?.body;
            let mut count = 0;
            for (index, token) in body.iter().enumerate() {
                if token.text != "return" {
                    continue;
                }
                count += 1;
                let end = body[index..]
                    .iter()
                    .position(|token| token.text == ";")
                    .map(|offset| index + offset)
                    .ok_or_else(|| format!("a `return` at line {} has no `;`", token.line))?;
                let expression = source::spelled(&body[index + 1..end]);
                if !expressions
                    .iter()
                    .any(|allowed| source::words(allowed).join(" ") == expression)
                {
                    return Err(format!(
                        "the `return` at line {} returns `{expression}`",
                        token.line
                    ));
                }
            }
            if count == 0 {
                return Err(format!("`{function}` has no `return`"));
            }
            Ok((lines(&body), format!("{count} return(s)")))
        }
        Obligation::FileHas {
            tokens: pattern, ..
        } => {
            let words = source::words(pattern);
            let starts = source::occurrences(tokens, &words);
            let start = *starts
                .first()
                .ok_or_else(|| "the file does not contain it".to_owned())?;
            Ok((
                lines(&tokens[start..start + words.len()]),
                format!("{} occurrence(s)", starts.len()),
            ))
        }
    }
}

/// The checker's answer for `binding` over `tree`.
#[must_use]
pub fn answer(tree: &Tree, binding: &Binding) -> Outcome {
    match binding.kind {
        Kind::Contracted { obligations, .. } => {
            let checked: Vec<Checked> = obligations
                .iter()
                .map(|obligation| check(tree, obligation))
                .collect();
            let mut failed: Vec<String> = checked
                .iter()
                .filter(|checked| !checked.holds)
                .map(|checked| format!("{}: {}", checked.obligation, checked.detail))
                .collect();
            if !tree.verified {
                failed.insert(
                    0,
                    "the tree was not checked against the pin, so nothing is established".into(),
                );
            }
            let (answer, reasons) = if failed.is_empty() {
                (
                    Answer::Safe,
                    vec![format!(
                        "every one of the {} obligation(s) of `{}`'s contract holds of the \
                         pinned libcurl {}",
                        checked.len(),
                        binding.function,
                        pin::VERSION
                    )],
                )
            } else {
                (Answer::Unknown, failed)
            };
            Outcome {
                function: binding.function,
                rank: binding.rank,
                answer,
                contracted: true,
                reasons,
                checked,
            }
        }
        Kind::Uncontracted { reason, witnesses } => Outcome {
            function: binding.function,
            rank: binding.rank,
            answer: Answer::Unknown,
            contracted: false,
            reasons: vec![reason.to_owned()],
            checked: witnesses
                .iter()
                .map(|obligation| check(tree, obligation))
                .collect(),
        },
    }
}

/// Every binding's answer, in ranking order.
#[must_use]
pub fn answer_all(tree: &Tree) -> Vec<Outcome> {
    BINDINGS
        .iter()
        .map(|binding| answer(tree, binding))
        .collect()
}

/// The evidence for the typecheck stubs: their definition in
/// `typecheck-gcc.h` and the names defined there.
#[must_use]
pub fn stub_evidence(tree: &Tree) -> Value {
    let definition = check(
        tree,
        &Obligation::FileHas {
            file: "include/curl/typecheck-gcc.h",
            tokens: "# define CURLWARNING ( id , message ) static void __attribute__ ( ( \
                     __warning__ ( message ) ) ) __attribute__ ( ( __unused__ ) ) \
                     __attribute__ ( ( __noinline__ ) ) id ( void ) { __asm__ ( \"\" ) ; }",
        },
    );
    let guard = check(
        tree,
        &Obligation::FileHas {
            file: "include/curl/typecheck-gcc.h",
            tokens: "# define curl_easy_setopt ( handle , option , value ) __extension__ ( { \
                     if ( __builtin_constant_p ( option ) ) {",
        },
    );
    let names: Vec<String> = tree
        .tokens("include/curl/typecheck-gcc.h")
        .map(|tokens| {
            tokens
                .windows(3)
                .filter(|window| window[0].text == "CURLWARNING" && window[1].text == "(")
                .map(|window| window[2].text.clone())
                .filter(|name| is_typecheck_stub(name))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();
    json!({
        "question": "W48-LIBCURL-TYPECHECK-STUBS",
        "treatment": "not bound: no contract and no claim; a caller's call to a stub keeps whatever the host decides for it",
        "definition": checked_json(&definition),
        "guard": checked_json(&guard),
        "stubs_defined": names,
    })
}

/// One obligation's record.
#[must_use]
pub fn checked_json(checked: &Checked) -> Value {
    json!({
        "obligation": checked.obligation,
        "holds": checked.holds,
        "detail": checked.detail,
        "file": checked.file,
        "sha256": pin::pinned_digest(&checked.file),
        "lines": checked.lines.map(|(first, last)| json!([first, last])),
    })
}

/// One binding's record.
#[must_use]
pub fn outcome_json(outcome: &Outcome) -> Value {
    let binding = binding(outcome.function).expect("an outcome of a binding");
    let clauses = match binding.kind {
        Kind::Contracted { clauses, .. } => json!(clauses),
        Kind::Uncontracted { .. } => Value::Null,
    };
    json!({
        "function": outcome.function,
        "rank": outcome.rank,
        "contracted": outcome.contracted,
        "clauses": clauses,
        "answer": outcome.answer.as_str(),
        "reasons": outcome.reasons,
        "obligations": outcome.checked.iter().map(checked_json).collect::<Vec<_>>(),
    })
}
