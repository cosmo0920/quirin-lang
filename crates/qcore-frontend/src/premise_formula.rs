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

//! Card **SP1-SYMBOLIC-OPENSSL-PREMISES** (contract specification C04.18.9,
//! grammar G05; USER DECISION 2026-11-20, verbatim: "we must write down as
//! condition symbols like |-> or other types of logical symbols to represent
//! the prerequisites which compose for premise conditions"): the symbolic
//! statement of a named premise, `requires resource_premise(NAME, FORMULA);`.
//!
//! A formula is read in two steps, both total-or-reject:
//!
//! 1. **Syntax** ([`parse_tokens`]): a bounded precedence reader over the
//!    language's existing tokens into a [`Raw`] tree. Loosest to tightest:
//!    `->` (implication, right-associative), `&&` (conjunction), `|->`
//!    (maps-to, non-associative), `in` and `==` (non-associative), `|` and
//!    `\` (set union and difference: a union chain or one difference, never
//!    mixed without parentheses; `-` is refused, so difference has one
//!    spelling), then a primary: `( F )`,
//!    `quant all #x in SET: F` (its body extends as far as it can), a term
//!    `#name`, `#name(A)` or `#name(A, B)`, an upper-case constant, `{}`.
//! 2. **Sorts** ([`check`]): every term is in the closed [`VOCABULARY`] with
//!    its signature, every constant in [`CLASSES`], every
//!    variable bound by an enclosing `quant` (never shadowing a term or
//!    another variable), and the whole formula is a proposition.
//!
//! The canonical printing ([`canonical`]) is the premise's statement: its
//! identity is NAME plus that text, and every rendering of the premise prints
//! it. A premise stays an undischarged assumption: no consumer that decides a
//! verdict reads it. The Lean model is `QCore.PremiseFormula` (card
//! SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES adds the Fluent Bit block and
//! `QCore.PremiseFormula.FluentBit`, whose three formulas are bound the same
//! way); its decision
//! vectors, vocabulary and the four OpenSSL formulas are bound to this module
//! by `crates/qcore-frontend/tests/premise_formula.rs`.
//!
//! Card **SP4-SYMBOLIC-LIBEVENT-PREMISES** appends, in its own block, the
//! terms `qcore-provider-libevent`'s scheduler premises need (C04.18.9's SP4
//! table): no token, operator, sort or production is added. Its formulas are
//! `QCore.PremiseFormula.Libevent`'s, bound by the same test file.
//!
//! Card **SP5-GLOBAL-PROSE-PREMISE-BAN** retires the string statement: a
//! premise clause states a formula or is refused by name ([`PROSE_PREMISE`],
//! `QF001`), on every route, since every route reads a premise through
//! [`premise_clause`]. Its SP5 block appends the two terms of C04.21's
//! reserved entry-snapshot premise; the clause decision vectors
//! ([`CLAUSE_DECISION_VECTORS`]) are `QCore.PremiseFormula`'s `clauseVectors`.
//!
//! Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS** (contract specification
//! C04.20, C04.18.9's SP8 block; USER DECISIONS 2026-11-23, D1 A, D2 B, D3
//! B): the statement of an outer contract definition, `quant #outer
//! [[contract]] NAME, FORMULA;`, is a formula of this language over the SP8
//! terms (subject `#outer_code`), read by [`outer_header`]; a string
//! statement is refused by name ([`PROSE_OUTER`], `QF004`). A candidate
//! inventory is cited by a registered constant of sort `obj`
//! ([`CANDIDATE_INVENTORIES`]), read back by [`candidate_digests`]. The
//! header vectors ([`OUTER_DECISION_VECTORS`]) and the two statements are
//! `QCore.PremiseFormula.Outer`'s.
//!
//! **`|->` in this context.** The lexer's [`Symbol::MapsTo`] is one token
//! everywhere (three bytes, matched before every shorter spelling, so `|->`
//! never lexes as a union followed by an implication). Its meaning is decided
//! by the delimited context that reads it: a contract state clause, a
//! premise-flow expression (C04.18.3), or this formula, which is exactly the
//! tokens between the comma after `NAME` and the clause's closing parenthesis.
//! Here `F |-> E` says every run of the callback `F` (or of each member of the
//! callback set `F`) during the call has the effect `E`.

use crate::lexer::{Symbol, TokenKind, lex};

/// At most this many tokens in one formula.
pub const MAX_TOKENS: usize = 1024;
/// At most this many nested primaries and operators.
pub const MAX_DEPTH: usize = 48;
/// At most this many tree nodes.
pub const MAX_NODES: usize = 512;

/// A sort of the formula language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    /// An object the call touches.
    Obj,
    /// An object class.
    Cls,
    /// A callback: a function at a position the library calls it from.
    Fn,
    /// A BIO method table.
    Meth,
    /// A resource the caller holds.
    Res,
    SetObj,
    SetFn,
    SetMeth,
    SetRes,
    /// A proposition.
    Prop,
    /// A property of one run of a callback (the right of `|->`).
    Eff,
}

impl Sort {
    /// The Lean constructor (`QCore.PremiseFormula.Srt`).
    #[must_use]
    pub fn lean(self) -> &'static str {
        match self {
            Self::Obj => ".obj",
            Self::Cls => ".cls",
            Self::Fn => ".fn",
            Self::Meth => ".meth",
            Self::Res => ".res",
            Self::SetObj => ".sobj",
            Self::SetFn => ".sfn",
            Self::SetMeth => ".smeth",
            Self::SetRes => ".sres",
            Self::Prop => ".prop",
            Self::Eff => ".eff",
        }
    }

    /// The set of this element sort.
    fn set(self) -> Option<Self> {
        match self {
            Self::Obj => Some(Self::SetObj),
            Self::Fn => Some(Self::SetFn),
            Self::Meth => Some(Self::SetMeth),
            _ => None,
        }
    }
}

/// A binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `->`
    Imp,
    /// `&&`
    Conj,
    /// `|->`
    MapsTo,
    /// `in`
    Mem,
    /// `==`
    Eqv,
    /// `|`
    Union,
    /// `\`
    Diff,
}

impl Op {
    fn spelling(self) -> &'static str {
        match self {
            Self::Imp => "->",
            Self::Conj => "&&",
            Self::MapsTo => "|->",
            Self::Mem => "in",
            Self::Eqv => "==",
            Self::Union => "|",
            Self::Diff => "\\",
        }
    }

    fn lean(self) -> &'static str {
        match self {
            Self::Imp => ".imp",
            Self::Conj => ".conj",
            Self::MapsTo => ".mapsTo",
            Self::Mem => ".mem",
            Self::Eqv => ".eqv",
            Self::Union => ".union",
            Self::Diff => ".diff",
        }
    }

    /// Binding strength, loosest first.
    fn level(self) -> u8 {
        match self {
            Self::Imp => 1,
            Self::Conj => 2,
            Self::MapsTo => 3,
            Self::Mem | Self::Eqv => 4,
            Self::Union | Self::Diff => 5,
        }
    }
}

/// A formula as read, before its sorts are checked. Term and variable names
/// are stored without their `#`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raw {
    /// `quant all #VAR in DOMAIN: BODY`
    Quant {
        var: String,
        domain: Box<Raw>,
        body: Box<Raw>,
    },
    /// `LEFT OP RIGHT`
    Bin(Op, Box<Raw>, Box<Raw>),
    /// `#name`: a nullary term or a bound variable.
    App0(String),
    /// `#name(A)`
    App1(String, Box<Raw>),
    /// `#name(A, B)`
    App2(String, Box<Raw>, Box<Raw>),
    /// An upper-case constant (a class: `SSL`, `BIO`).
    Const(String),
    /// `{}`
    Empty,
}

/// One registered term: `#NAME(ARGS…)` of sort `RESULT`, with its meaning
/// (contract specification C04.18.9, where each is defined).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term {
    pub name: &'static str,
    pub arguments: &'static [Sort],
    pub result: Sort,
    pub meaning: &'static str,
}

/// The closed vocabulary. Every term a formula names is here; an unknown
/// term rejects the module.
pub const VOCABULARY: &[Term] = &[
    Term {
        name: "args",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call's pointer arguments designate",
    },
    Term {
        name: "created",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call allocates, directly or as a temporary object inside the call",
    },
    Term {
        name: "copied",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call copies (the source of a dup)",
    },
    Term {
        name: "freed",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects whose last reference the call drops: directly, in turn (an object a freed object owns), or as a replaced or temporary object it frees inside the call",
    },
    Term {
        name: "class",
        arguments: &[Sort::Obj],
        result: Sort::Cls,
        meaning: "the object's class",
    },
    Term {
        name: "of",
        arguments: &[Sort::Cls, Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the members of the set whose class is the class",
    },
    Term {
        name: "ctx",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the SSL_CTX of each SSL object of the set",
    },
    Term {
        name: "bios",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the BIOs each SSL object of the set holds (its read and write BIO)",
    },
    Term {
        name: "chain",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the BIOs of the set and every BIO chained after one of them, transitively",
    },
    Term {
        name: "ssl_of",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the SSL object a BIO_f_ssl BIO holds (no object for any other object)",
    },
    Term {
        name: "slots",
        arguments: &[Sort::SetObj],
        result: Sort::SetFn,
        meaning: "every callback slot of an object of the set: each function pointer member the library calls, including the arms SSL_CTX_callback_ctrl writes (the servername, status and ticket key callbacks among them)",
    },
    Term {
        name: "stated",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks a clause of the declaration states the call runs",
    },
    Term {
        name: "ex_data",
        arguments: &[Sort::Cls],
        result: Sort::SetFn,
        meaning: "the new, dup and free callbacks CRYPTO_get_ex_new_index registers for the class",
    },
    Term {
        name: "ex_data_free",
        arguments: &[Sort::Cls],
        result: Sort::SetFn,
        meaning: "the free callbacks CRYPTO_get_ex_new_index registers for the class",
    },
    Term {
        name: "method",
        arguments: &[Sort::Obj],
        result: Sort::Meth,
        meaning: "the method table of a BIO (its BIO_METHOD) or of an X509_LOOKUP (its X509_LOOKUP_METHOD)",
    },
    Term {
        name: "builtin",
        arguments: &[],
        result: Sort::SetMeth,
        meaning: "the method tables the library defines itself, which no setter is handed: the BIO_METHOD BIO_s_socket, BIO_s_mem, BIO_f_buffer or another built-in BIO_s_ or BIO_f_ function returns, and the X509_LOOKUP_METHOD X509_LOOKUP_file, X509_LOOKUP_hash_dir or X509_LOOKUP_store returns",
    },
    Term {
        name: "fns",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the functions of the object's method table (a BIO's BIO_METHOD, an X509_LOOKUP's X509_LOOKUP_METHOD), each as run on that object",
    },
    Term {
        name: "store",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the certificate store (X509_STORE) each SSL_CTX of the set holds",
    },
    Term {
        name: "lookups",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the lookups (X509_LOOKUP) each X509_STORE of the set holds",
    },
    Term {
        name: "teardown",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the callbacks the free of the object runs from its method tables, its provider and its engine, and the callbacks the object holds that its free runs",
    },
    Term {
        name: "settable",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks a setter some declaration of the declaration's crate states can write",
    },
    // Card OPENSSL-FREE-REMAINING-INDIRECT-CALLBACKS (C04.18.9 amended).
    Term {
        name: "library",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions the library defines itself and installs without a setter call of the run: those of the method tables it defines (its default RSA, DSA, DH and EC_KEY methods, its EVP_PKEY_ASN1_METHOD, EVP_PKEY_METHOD, EVP_MD and EVP_CIPHER tables, the EC_METHOD of a group), of the engines it builds in, and the dispatch functions of the default, base, legacy and null providers",
    },
    Term {
        name: "loaded",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions of a module the library loads from outside the run: a provider other than the default, base, legacy and null providers (the FIPS provider and any provider a configuration or OSSL_PROVIDER_load loads among them), and an engine the dynamic engine loads from a shared object",
    },
    Term {
        name: "in_turn",
        arguments: &[Sort::Fn, Sort::SetObj],
        result: Sort::SetFn,
        meaning: "the callbacks a run of the function runs in turn, directly or through the calls it makes, on an object of the set",
    },
    Term {
        name: "caller_held",
        arguments: &[],
        result: Sort::SetRes,
        meaning: "the resources the caller holds when the run starts",
    },
    Term {
        name: "takes",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run takes over",
    },
    Term {
        name: "releases",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run releases",
    },
    Term {
        name: "retains",
        arguments: &[Sort::SetRes],
        result: Sort::SetRes,
        meaning: "the members of the set the run keeps beyond its return",
    },
    Term {
        name: "inert",
        arguments: &[],
        result: Sort::Eff,
        meaning: INERT_DEFINITION,
    },
    // ---- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the qcore-provider-librdkafka
    // block (C04.18.9), appended after the existing entries. ----
    Term {
        name: "false",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the proposition that never holds (falsum): `P -> #false` is the negation of `P`, and `quant all #x in S: #false` says `S` has no member",
    },
    Term {
        name: "rk_conf",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "every rk_conf: an rd_kafka_conf_t (struct rd_kafka_conf_s), a client handle's embedded configuration rk->rk_conf included",
    },
    Term {
        name: "ran",
        arguments: &[Sort::SetFn],
        result: Sort::SetFn,
        meaning: "the members of the set the call runs, directly or in turn through the calls it makes (rd_kafka_log_buf's run of the log callback included)",
    },
    Term {
        name: "registered",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the run's registrations (C04.19.3): each value a call of a declared setter (resource_callback_member_registered) in a unit of the run stores into its member",
    },
    Term {
        name: "defaults",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the stated defaults (C04.19.4): each value a constructor's `quant live #result: TAG.FIELD |-> f` stores into its member",
    },
    Term {
        name: "candidates",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks with a declared candidate: each function a declaration in force states as a candidate of the member it is stored in (C04.19.7), with its own contract",
    },
    Term {
        name: "loaded_registered",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks code loaded at runtime registers (a library plugin.library.paths names, a Fluent Bit plugin flb_plugin_load or flb_plugin_proxy_create loads, and code it invokes); none in the static scope",
    },
    Term {
        name: "enveloped",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the registrations the applicable pinned rk_plugin_library and flb_external_plugin outer contracts admit (C04.20): through a public setter, each meeting its member's candidate",
    },
    Term {
        name: "punned_stores",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks rd_kafka_anyconf_set_prop0's _RK_C_PTR case (rdkafka_conf.c:2087-2088), a type-punned store through the property table, writes into a callback member (the setter's own store among them); Clang's member facts see none of these stores",
    },
    Term {
        name: "conf_copies",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks rd_kafka_anyconf_copy stores into a callback member: the value of the same property of another configuration",
    },
    Term {
        name: "hidden_string",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks the string API (rd_kafka_conf_set by property name) stores into an _RK_HIDDEN callback property",
    },
    Term {
        name: "ut_handle_produce_response",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "ut_handle_ProduceResponse, as the value of a callback member",
    },
    Term {
        name: "call",
        arguments: &[],
        result: Sort::Fn,
        meaning: "the declared function itself, whose one run is the call (so `#call |-> E` says the call has E)",
    },
    Term {
        name: "rd_calloc",
        arguments: &[],
        result: Sort::Fn,
        meaning: "rd_calloc (rd.h:139-143), as the call runs it",
    },
    Term {
        name: "rd_malloc",
        arguments: &[],
        result: Sort::Fn,
        meaning: "rd_malloc (rd.h:145-149), as the call runs it",
    },
    Term {
        name: "rd_kafka_toppar_new0",
        arguments: &[],
        result: Sort::Fn,
        meaning: "rd_kafka_toppar_new0, as the call runs it",
    },
    Term {
        name: "returns_result_of",
        arguments: &[Sort::Fn],
        result: Sort::Eff,
        meaning: "the run returns, as its result's last source, the value a run of the function returned during it",
    },
    Term {
        name: "asserts_result",
        arguments: &[],
        result: Sort::Eff,
        meaning: "the run asserts (rd_assert) that its result is not NULL before it returns",
    },
    Term {
        name: "returns_nonnull",
        arguments: &[],
        result: Sort::Eff,
        meaning: "the run's result is not NULL",
    },
    Term {
        name: "audited_assertions",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the audited compile commands define no NDEBUG: every assertion of the analysed sources is compiled into the audited build",
    },
    Term {
        name: "deployed_assertion",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the deployed build preserves the call's analysed assertion",
    },
    Term {
        name: "deployed_failure_call",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the deployed build preserves the assertion's failure call, which a declaration in force states terminating (it never returns)",
    },
    Term {
        name: "failure_no_return",
        arguments: &[],
        result: Sort::Eff,
        meaning: "a run whose allocation fails does not return normally",
    },
    Term {
        name: "op_callbacks",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the op callbacks the call may run through rd_kafka_op_handle, rd_kafka_op_call or rd_kafka_op_destroy",
    },
    Term {
        name: "served",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the queue the call serves (its rkq argument)",
    },
    Term {
        name: "held_on",
        arguments: &[Sort::SetObj],
        result: Sort::SetRes,
        meaning: "the references the caller holds on an object of the set when the run starts",
    },
    Term {
        name: "refs",
        arguments: &[Sort::SetObj],
        result: Sort::SetRes,
        meaning: "every reference to an object of the set, whoever holds it",
    },
    Term {
        name: "broker_monitor_cb",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callback the op's rko_u.broker_monitor.cb holds",
    },
    Term {
        name: "monitor_add_registered",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks rd_kafka_broker_monitor_add registers, over every call of it",
    },
    Term {
        name: "coord_rkb_monitor_cb",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "rd_kafka_coord_rkb_monitor_cb (rdkafka_broker.c:5497)",
    },
    Term {
        name: "txn_coord_monitor_cb",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "rd_kafka_txn_coord_monitor_cb (rdkafka_txnmgr.c:3248)",
    },
    Term {
        name: "broker",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the broker the op's rko_u.broker_monitor.rkb designates",
    },
    Term {
        name: "handed_only",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "every object the run is handed as an argument is a member of the set",
    },
    Term {
        name: "assign_cb",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the assign callback the group assignor runs (rd_kafka_cgrp_assignor_run)",
    },
    Term {
        name: "assignor_add_registered",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the assign callbacks rd_kafka_assignor_add registers, over every call of it",
    },
    Term {
        name: "range_assign",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the range assignor's assign callback",
    },
    Term {
        name: "roundrobin_assign",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the roundrobin assignor's assign callback",
    },
    Term {
        name: "cooperative_sticky_assign",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the cooperative-sticky assignor's assign callback",
    },
    Term {
        name: "metadata",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the metadata the assignor runs over (the op's rko_u.metadata.mdi)",
    },
    Term {
        name: "members",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the group members the assignor is handed",
    },
    Term {
        name: "reads_only",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "the run writes no object of the set: it only reads them",
    },
    Term {
        name: "op",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the op the call is handed (its rko argument)",
    },
    Term {
        name: "ack_batches",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the list the op's rko_u.share_fetch_fanout.ack_batches designates (none when NULL)",
    },
    Term {
        name: "ack_details",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the lists rd_kafka_share_build_ack_details (rdkafka_share_acknowledgement.c:686-842) returns",
    },
    Term {
        name: "first_add_lists",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the lists created only at their first rd_list_add, so each holds an element from its creation",
    },
    Term {
        name: "nonempty",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the lists that hold at least one element",
    },
    Term {
        name: "logq",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the log queue rk->rk_logq of the client handle the call logs through",
    },
    Term {
        name: "forwards",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the queues of the set and every queue one of them is forwarded to (rkq_fwdq), transitively, while the call enqueues on it",
    },
    Term {
        name: "event_cb",
        arguments: &[Sort::SetObj],
        result: Sort::SetFn,
        meaning: "the queue event callbacks registered on a queue of the set while the call enqueues on it (only rd_kafka_q_cb_event_enable registers one, reached only from the public rd_kafka_queue_cb_event_enable)",
    },
    Term {
        name: "fwd_queue",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the queue rd_kafka_q_enq1 takes a reference to with rd_kafka_q_fwd_get and releases with rd_kafka_q_destroy, the only queue reference the enqueue releases",
    },
    Term {
        name: "unforwarded",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the queues another thread unforwards during the call",
    },
    Term {
        name: "last_released",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the queues whose last reference other than the call's own another thread releases during the call",
    },
    Term {
        name: "cjson_allocate",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "hooks->allocate: the allocation function of the hooks the call is handed",
    },
    Term {
        name: "cjson_reallocate",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "hooks->reallocate: the reallocation function of the hooks the call is handed",
    },
    Term {
        name: "cjson_deallocate",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "hooks->deallocate: the deallocation function of the hooks the call is handed",
    },
    Term {
        name: "libc_hooks",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "malloc, realloc and free, which global_hooks holds unless cJSON_InitHooks installed others (cJSON.c:178-186)",
    },
    Term {
        name: "installed_hooks",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions cJSON_InitHooks installed in global_hooks (a user pair or set)",
    },
    Term {
        name: "fresh_or_null",
        arguments: &[],
        result: Sort::Eff,
        meaning: "the run returns a fresh allocation (one no other reference designates) or NULL",
    },
    Term {
        name: "hook_allocated",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the allocations a run of hooks->allocate or hooks->reallocate returns during the call",
    },
    Term {
        name: "deallocate_releases",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the allocations the matching hooks->deallocate releases when it is handed one (cJSON_Delete releases an item through it)",
    },
    Term {
        name: "cjson_arrays",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cJSON arrays (items of type cJSON_Array) live during the call",
    },
    Term {
        name: "children",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the elements of the cJSON item, in order: its finite child sequence (for cJSON_DetachItemViaPointer, the exact owned-child sequence of the entry environment)",
    },
    Term {
        name: "first",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the first element of the item's child sequence (none when it is empty)",
    },
    Term {
        name: "last",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the last element of the item's child sequence (none when it is empty)",
    },
    Term {
        name: "pred",
        arguments: &[Sort::Obj, Sort::Obj],
        result: Sort::SetObj,
        meaning: "every element immediately before an occurrence of the second item in the first item's child sequence",
    },
    Term {
        name: "child",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the item the item's child member designates (none when NULL)",
    },
    Term {
        name: "next",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the item the item's next member designates (none when NULL)",
    },
    Term {
        name: "prev",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the item the item's prev member designates (none when NULL)",
    },
    Term {
        name: "next_reach",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the items reached from the item by one or more next links",
    },
    Term {
        name: "owned",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the children the item owns: its exact custody domain",
    },
    Term {
        name: "nodup",
        arguments: &[Sort::Obj],
        result: Sort::Prop,
        meaning: "the item's child sequence names no identity twice",
    },
    Term {
        name: "live",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects live at the call's entry",
    },
    Term {
        name: "parent",
        arguments: &[],
        result: Sort::Obj,
        meaning: "the object the call's parent argument designates (the declaration requires it non-null)",
    },
    Term {
        name: "concurrent",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the steps another thread runs while the call runs, each as the function it runs",
    },
    Term {
        name: "keeps_links",
        arguments: &[Sort::Obj],
        result: Sort::Eff,
        meaning: "the run writes neither the item's child member nor the next or prev member of an element of its child sequence",
    },
    // ---- End of the SP2 block. ----
    // ---- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: the Fluent Bit terms
    // (`quirin-provider-fluent-bit`'s three premises). Appended as one block;
    // the entries above are unchanged.
    Term {
        name: "result",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the object the call's result designates (no object for a null or scalar result)",
    },
    Term {
        name: "interior",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the objects the objects of the set own, directly or in turn: what an owned pointer member designates, and what that object owns",
    },
    Term {
        name: "chunk_name",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the name (ch->name, an sds ChunkIO allocates and frees) of the ChunkIO chunk (ic->chunk) each input chunk of the set holds",
    },
    Term {
        name: "tracked",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the resource projection holds a custody state for",
    },
    Term {
        name: "claimed",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects whose custody or lifetime a clause of the declaration states: a parameter mode's object, or a result handed over with custody (a resource_return_none result is not one)",
    },
    Term {
        name: "bound",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the caller binds to a name at the call's site: the result assigned to a variable, a member or an element",
    },
    Term {
        name: "stored",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the caller stores at the call's site where they outlive the statement: through a pointer or into a container",
    },
    Term {
        name: "faithful",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the declaration's result clause states exactly the custody the caller acquires from the call at its site",
    },
    Term {
        name: "admitted",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the gateway admits the call at its site under its rule for a call whose result mode is None and whose parameters are all Borrowed (card O04-TRACE premises N6 and N8)",
    },
    Term {
        name: "config_of",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the Fluent Bit context (struct flb_config, its config member) each input instance of the set belongs to",
    },
    Term {
        name: "ingesting",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the contexts whose is_ingestion_active member is true when the call reads it",
    },
    Term {
        name: "collector",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the collector the call finds by the id it is given in its input instance's collector list (get_collector); no object when none has that id",
    },
    Term {
        name: "event_registered",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the collectors whose own event the call registers with the collector's event loop (mk_event_add, or mk_event_timeout_create for a timer collector)",
    },
    Term {
        name: "set_running",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the collectors whose running member the call sets true",
    },
    Term {
        name: "returns_zero",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the call returns 0",
    },
    Term {
        name: "in_tree",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the output plugins Fluent Bit's own source tree defines (the struct flb_output_plugin of each plugins/out_* plugin)",
    },
    Term {
        name: "synchronous",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the output plugins whose flags carry FLB_OUTPUT_SYNCHRONOUS",
    },
    Term {
        name: "proxy",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the output plugins a proxy registers: plugins defined outside Fluent Bit's tree and loaded at run time through flb_plugin_proxy",
    },
    Term {
        name: "production",
        arguments: &[],
        result: Sort::Prop,
        meaning: "the call is made by a running Fluent Bit engine from its output dispatch, not directly by a test or another embedding",
    },
    Term {
        name: "out_plugin",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the output plugin (its p member) of each output instance of the set",
    },
    // ---- End of SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES.
    // ---- Card SP4-SYMBOLIC-LIBEVENT-PREMISES (C04.18.9, SP4 block): the
    // terms `qcore-provider-libevent`'s scheduler premises need, appended
    // after the SP1, SP2 and SP3 blocks; `#tracked` is SP3's. ----
    Term {
        name: "singleton",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the set whose one member is the object",
    },
    Term {
        name: "live_at_use",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects whose storage is allocated, and not yet released, at every point of the call that reads, writes or releases one of them",
    },
    Term {
        name: "inside",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the objects whose storage lies within the allocation of the object, so that releasing it ends their storage",
    },
    Term {
        name: "released_only_by_call",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects nothing but the call releases: no other call, thread or code, before or after it, releases one",
    },
    Term {
        name: "released",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the call releases, directly or through a function it runs",
    },
    Term {
        name: "ended",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects whose lifetime the call ends, by releasing them or an object whose storage holds theirs",
    },
    Term {
        name: "overlaps",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the objects whose storage overlaps the storage of an object of the set",
    },
    Term {
        name: "stores",
        arguments: &[Sort::SetObj],
        result: Sort::SetObj,
        meaning: "the member cells of the objects of the set that the call writes (a member is a cell)",
    },
    Term {
        name: "interleaved",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions that run concurrently with or reentrantly into the call (another thread's, a signal handler, a reentrant call); `|->` reads each of their runs that overlaps the call",
    },
    Term {
        name: "no_writes",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "the run writes no object of the set",
    },
    Term {
        name: "no_releases",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "the run releases (frees, deletes) no object of the set",
    },
    Term {
        name: "no_resumes",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "the run resumes (switches to) no coroutine of the set",
    },
    Term {
        name: "unread_after",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cells nothing reads after the call returns",
    },
    Term {
        name: "unresumed_after",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cells through which nothing resumes a thread or a coroutine after the call returns",
    },
    Term {
        name: "mk_parent",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the mk_thread the member parent of the mk_http_thread designates",
    },
    Term {
        name: "thread_data",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the object at MK_THREAD_DATA of the mk_thread, (char *) t + sizeof(struct mk_thread) for the mk_thread t: the mk_http_thread its allocation carries",
    },
    Term {
        name: "mk_callee",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the coroutine the member callee of the mk_thread designates",
    },
    Term {
        name: "co_deletable",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the coroutines co_delete accepts: made by co_create and not yet deleted",
    },
    Term {
        name: "created_for",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the coroutines created for the mk_thread",
    },
    Term {
        name: "suspended",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the coroutines suspended (switched away from, not running) while the call runs",
    },
    Term {
        name: "co_active",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the coroutine co_active() returns while the call runs (the running one)",
    },
    Term {
        name: "mk_session",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the mk_http_session the member session of the mk_http_thread designates",
    },
    Term {
        name: "mk_channel",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the mk_channel the member channel of the mk_http_session designates",
    },
    Term {
        name: "mk_event",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the mk_event the member event of the mk_channel designates",
    },
    Term {
        name: "mk_type",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the member type of the mk_event, one scalar cell",
    },
    Term {
        name: "mk_link",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the link member _head of the mk_http_thread, a struct mk_list cell",
    },
    Term {
        name: "lists",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the heads of the mk_lists whose cycle the node is linked in",
    },
    Term {
        name: "caller_list",
        arguments: &[Sort::Obj],
        result: Sort::Obj,
        meaning: "the head of the list through which the call's caller reaches the node",
    },
    Term {
        name: "circular_lists",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the heads of valid circular mk_lists: following next from the head, through live nodes, returns to the head",
    },
    Term {
        name: "reciprocal",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the mk_list nodes n with n->next->prev = n and n->prev->next = n",
    },
    Term {
        name: "nodes",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the nodes of the mk_list the head heads, the head included",
    },
    Term {
        name: "libco_params_key",
        arguments: &[],
        result: Sort::Obj,
        meaning: "the slot of the thread-specific key mk_http_thread_libco_params of the current thread, a pointer cell",
    },
    Term {
        name: "holds",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the object the pointer cell designates when the call reads it; none when it holds NULL",
    },
    Term {
        name: "held_at_return",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the object the pointer cell designates when the call returns; none when it holds NULL",
    },
    Term {
        name: "sole_owned",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the allocations the cell alone owns: it is their one owning reference",
    },
    Term {
        name: "getspecific_ok",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the thread-specific key slots every pthread_getspecific of which during the call succeeds (returns the value the slot holds)",
    },
    Term {
        name: "setspecific_ok",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the thread-specific key slots every pthread_setspecific of which during the call succeeds (returns 0, and the slot then holds the value stored)",
    },
    Term {
        name: "aliases",
        arguments: &[Sort::Obj],
        result: Sort::SetObj,
        meaning: "the cells the source leaves designating the mk_thread, or an object its storage holds, when the call returns: request->thread, channel.thread and the current thread's mk_thread key among them",
    },
    Term {
        name: "stack_deregister",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the Valgrind client request VALGRIND_STACK_DEREGISTER(t->valgrind_stack_id) for the mk_thread t, which reads t->valgrind_stack_id, as mk_thread_destroy(t) runs it where MK_HAVE_VALGRIND compiles it in; none where it does not",
    },
    // ---- Card SP5-GLOBAL-PROSE-PREMISE-BAN (C04.18.9's SP5 block): the
    // reserved entry-snapshot premise of C04.21, whose string statement the
    // card retired. ----
    Term {
        name: "condition_reads",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the member cells the declaration's condition bindings read at the call's entry: for each `quant #NAME = (...)` binding (C04.21), the cell its member path names on the bound formal",
    },
    Term {
        name: "entry_stable",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cells that hold, at each guarded use during the call (each use of a prerequisite of a `[[local]]` block, C04.21), the value they held at the call's entry",
    },
    // ---- Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS (C04.18.9's SP8 block):
    // the terms of an outer contract statement (C04.20). An outer statement's
    // subject is the code the contract's loaders load, not a call: each term
    // is read relative to the contract, as `#call` is relative to its
    // declaration. An effect's "own instructions" exclude what a call of a
    // function of the audited program does. ----
    Term {
        name: "outer_code",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions of the code the contract's loaders load (what each loader hosting the contract opens: rd_kafka_plugin_new a library plugin.library.paths names, flb_plugin_load a C shared object, flb_plugin_proxy_create a Go plugin), and every function outside the audited program a run of one of them runs, directly or in turn (its dependencies and the C library included); no function of the audited program is a member",
    },
    Term {
        name: "conf_registrations",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks code of #outer_code stores, directly or through a call it makes, into an rk_conf callback member (a function pointer member of rd_kafka_conf_s, rk->rk_conf's included)",
    },
    Term {
        name: "public_setter_values",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "each value a call of a public librdkafka setter (a function librdkafka's public headers declare that stores a callback member; the rd_kafka_conf_set_*_cb functions among them) stores into its member",
    },
    Term {
        name: "meets_candidate",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the callbacks that meet the candidate contract (C04.19.3, C04.19.7) declared for the member each is stored in",
    },
    Term {
        name: "interceptors",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the interceptor methods code of #outer_code registers, each as the element of the rd_kafka_conf_s.interceptors list (or its rk->rk_conf alias) it is appended to",
    },
    Term {
        name: "zero_appends",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "for each candidate of the inventory (list member, aliases, element record, union arm, public setter): the elements a call of that candidate's setter appends to exactly that list under exactly that union arm, in a call that returns 0",
    },
    Term {
        name: "conf_lifecycle",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the elements appended under a candidate of the inventory whose lifecycle is configuration",
    },
    Term {
        name: "client_lifecycle",
        arguments: &[Sort::Obj],
        result: Sort::SetFn,
        meaning: "the elements appended under a candidate of the inventory whose lifecycle is pre-initialization, non-share",
    },
    Term {
        name: "on_live_conf",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the interceptors installed by a call handed a live configuration (allocated and not yet destroyed)",
    },
    Term {
        name: "from_on_new",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the interceptors installed by a call made during a run of an on_new interceptor",
    },
    Term {
        name: "before_init",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the interceptors installed before the initialization of the client the installing call is handed completes",
    },
    Term {
        name: "on_share_consumer",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the interceptors installed on a share consumer client",
    },
    Term {
        name: "copies",
        arguments: &[Sort::SetFn],
        result: Sort::SetFn,
        meaning: "the members of the set and every registration the library copies from one of them into another configuration or client (rd_kafka_conf_dup, the copied on_new registrations)",
    },
    Term {
        name: "valid_through_last",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the registrations whose code stays loaded and whose opaque context stays valid from registration until the last point the library can invoke them: the end of the destruction of every configuration or client holding them, destruction callbacks included (USER DECISION 2026-11-23, D3 B)",
    },
    Term {
        name: "borrows_args",
        arguments: &[],
        result: Sort::Eff,
        meaning: "every pointer argument of the run, and each object reachable from it, is borrowed: the run, with every call it makes, takes over, releases, retains and transfers none of them (documented borrowed output-buffer writes allowed)",
    },
    Term {
        name: "borrows_handles",
        arguments: &[],
        result: Sort::Eff,
        meaning: "every scalar resource handle argument of the run (a descriptor such as sockfd among them) is borrowed: the run, with every call it makes, closes, releases, duplicates and transfers none of them",
    },
    Term {
        name: "no_global_custody",
        arguments: &[],
        result: Sort::Eff,
        meaning: "the run, with every call it makes, changes the custody of no object through global state",
    },
    Term {
        name: "balanced",
        arguments: &[],
        result: Sort::Eff,
        meaning: "every resource the run, with every call it makes, acquires is released before the run returns",
    },
    Term {
        name: "stores_none",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "the run stores, by its own instructions, into no cell of the set",
    },
    Term {
        name: "calls_none",
        arguments: &[Sort::SetFn],
        result: Sort::Eff,
        meaning: "the run calls, by its own instructions, no member of the set",
    },
    Term {
        name: "rk_internal",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the member cells of librdkafka's internal records (those its public headers rdkafka.h and rdkafka_mock.h do not define): callback members, rd_kafka_op_s members, broker links, the rule-14t members, queues (rd_kafka_q_s) and list links (rd_list_t, TAILQ links) among them",
    },
    Term {
        name: "rk_public",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions librdkafka's public headers (rdkafka.h, rdkafka_mock.h) declare",
    },
    Term {
        name: "rk_nonpublic",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "librdkafka's functions its public headers do not declare",
    },
    Term {
        name: "meets_published",
        arguments: &[Sort::SetFn],
        result: Sort::Eff,
        meaning: "every call the run makes to a member of the set meets the conditions that function's published documentation states (USER DECISION 2026-11-23, D1 A)",
    },
    Term {
        name: "cjson_init_hooks",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "cJSON_InitHooks",
    },
    Term {
        name: "cjson_global_hooks",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cells of cJSON's global_hooks (its allocate, deallocate and reallocate members)",
    },
    Term {
        name: "cjson_link_cells",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the child, prev and next member cells of every cJSON item",
    },
    Term {
        name: "cjson_handed",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the cJSON items code of #outer_code passes to a cJSON API call, and every item reachable from one through child and next",
    },
    Term {
        name: "allocator_hooks_premise",
        arguments: &[],
        result: Sort::Prop,
        meaning: "every statement of the named premise cjson_allocator_hooks (C04.18.9; on cJSON_New_Item and on print) holds of the allocator hooks installed before the loaded code loads (malloc, realloc and free when none were)",
    },
    Term {
        name: "flb_core_owned",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the member cells Fluent Bit's core owns of every struct flb_*_instance (input, output, filter, processor, custom among them) and struct flb_config",
    },
    Term {
        name: "handed",
        arguments: &[],
        result: Sort::SetObj,
        meaning: "the objects the contract's loaders hand to loaded code and every object reachable from one (C04.20's reach), the objects an exported API returns to it or a callback it registered is handed, and the objects it allocates or defines itself",
    },
    Term {
        name: "exported",
        arguments: &[],
        result: Sort::SetFn,
        meaning: "the functions the Fluent Bit binary exports (it is linked with ENABLE_EXPORTS ON)",
    },
    Term {
        name: "accesses_only",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "every object of the audited program the run reads or writes by its own instructions is a member of the set",
    },
    Term {
        name: "calls_only",
        arguments: &[Sort::SetFn],
        result: Sort::Eff,
        meaning: "every function of the audited program the run calls by its own instructions is a member of the set",
    },
    Term {
        name: "provenance",
        arguments: &[Sort::SetObj],
        result: Sort::Eff,
        meaning: "every pointer the run dereferences, stores or passes is derived (copied, offset within its object, converted to void * and back) from a pointer to an object of the set (USER DECISION 2026-11-23, D2 B with #typed_access)",
    },
    Term {
        name: "typed_access",
        arguments: &[],
        result: Sort::Eff,
        meaning: "the run reads and writes each object only at its own type or a type C permits for it (a character type, the record embedding it): no conversion between unrelated record pointers (USER DECISION 2026-11-23, D2 B)",
    },
];

/// Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS** (C04.18.9's SP8 block):
/// the registered candidate inventory constants, of sort `obj`. Each is
/// `CANDIDATES_` followed by the upper-case hexadecimal SHA-256 of a
/// provider's `outer-candidates.json` (C04.20.1), so an outer statement
/// cites the inventory by a checked identity: `qcore_audit::outer_candidates`
/// refuses selection when the digest names no inventory, and a test binds
/// each constant to the file's digest. A constant of this shape that is not
/// registered is refused like any unknown constant.
pub const CANDIDATE_INVENTORIES: &[&str] =
    &["CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147"];

/// The prefix of a candidate inventory constant.
pub const CANDIDATE_PREFIX: &str = "CANDIDATES_";

/// Whether `word` has the shape of a candidate inventory constant:
/// [`CANDIDATE_PREFIX`] and 64 upper-case hexadecimal digits.
#[must_use]
pub fn candidate_shaped(word: &str) -> bool {
    word.strip_prefix(CANDIDATE_PREFIX).is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    })
}

/// The lower-case SHA-256 a candidate inventory constant names.
#[must_use]
pub fn candidate_digest(word: &str) -> Option<String> {
    candidate_shaped(word).then(|| word[CANDIDATE_PREFIX.len()..].to_ascii_lowercase())
}

/// Every candidate inventory constant `raw` names, as the lower-case digest.
#[must_use]
pub fn candidate_digests(raw: &Raw) -> std::collections::BTreeSet<String> {
    fn walk(raw: &Raw, out: &mut std::collections::BTreeSet<String>) {
        match raw {
            Raw::Quant { domain, body, .. } => {
                walk(domain, out);
                walk(body, out);
            }
            Raw::Bin(_, left, right) | Raw::App2(_, left, right) => {
                walk(left, out);
                walk(right, out);
            }
            Raw::App1(_, argument) => walk(argument, out),
            Raw::Const(word) => {
                if let Some(digest) = candidate_digest(word) {
                    out.insert(digest);
                }
            }
            Raw::App0(_) | Raw::Empty => {}
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(raw, &mut out);
    out
}

/// The one defined term: `#inert` abbreviates this effect (C04.18.9).
pub const INERT_DEFINITION: &str = "#takes(#caller_held) == {} && #releases(#caller_held) == {} \
&& #retains(#caller_held) == {}";

/// The class constants: `CRYPTO_get_ex_new_index`'s (3.5) ex_data classes.
pub const CLASSES: &[&str] = &[
    "BIO",
    "DH",
    "DSA",
    "EC_KEY",
    "ENGINE",
    "EVP_PKEY",
    "RSA",
    "SSL",
    "SSL_CTX",
    "SSL_SESSION",
    "UI",
    "UI_METHOD",
    "X509",
    "X509_STORE",
    "X509_STORE_CTX",
];

fn error(message: &str) -> String {
    format!("premise-formula: {message}")
}

fn lowercase_identifier(word: &str) -> bool {
    word.len() <= 64
        && word.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && word
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn constant_word(word: &str) -> bool {
    // Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS: a candidate inventory
    // constant is 75 bytes; every other constant keeps the 64-byte bound.
    (word.len() <= 64 || candidate_shaped(word))
        && word.bytes().next().is_some_and(|b| b.is_ascii_uppercase())
        && word
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

struct Reader<'a> {
    tokens: &'a [TokenKind],
    at: usize,
    nodes: usize,
    depth: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at)
    }

    fn symbol(&self, symbol: Symbol) -> bool {
        self.peek() == Some(&TokenKind::Symbol(symbol))
    }

    fn keyword(&self, word: &str) -> bool {
        matches!(self.peek(), Some(TokenKind::Word(w)) if w == word)
    }

    fn take(&mut self, symbol: Symbol) -> bool {
        let found = self.symbol(symbol);
        if found {
            self.at += 1;
        }
        found
    }

    fn need(&mut self, symbol: Symbol, what: &str) -> Result<(), String> {
        if self.take(symbol) {
            Ok(())
        } else {
            Err(error(&format!("expected {what}")))
        }
    }

    fn need_keyword(&mut self, word: &str) -> Result<(), String> {
        if self.keyword(word) {
            self.at += 1;
            Ok(())
        } else {
            Err(error(&format!("expected `{word}`")))
        }
    }

    fn name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some(TokenKind::Word(word)) if lowercase_identifier(word) => {
                let word = word.clone();
                self.at += 1;
                Ok(word)
            }
            _ => Err(error("`#` is followed by a lower-case name")),
        }
    }

    fn node(&mut self, raw: Raw) -> Result<Raw, String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(error("too many nodes"));
        }
        Ok(raw)
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(error("nested too deeply"));
        }
        Ok(())
    }

    fn bin(&mut self, op: Op, left: Raw, right: Raw) -> Result<Raw, String> {
        self.node(Raw::Bin(op, Box::new(left), Box::new(right)))
    }

    fn implication(&mut self) -> Result<Raw, String> {
        self.enter()?;
        let left = self.conjunction()?;
        let raw = if self.take(Symbol::Arrow) {
            let right = self.implication()?;
            self.bin(Op::Imp, left, right)?
        } else {
            left
        };
        self.depth -= 1;
        Ok(raw)
    }

    fn conjunction(&mut self) -> Result<Raw, String> {
        let mut left = self.maps_to()?;
        while self.take(Symbol::AndAnd) {
            let right = self.maps_to()?;
            left = self.bin(Op::Conj, left, right)?;
        }
        Ok(left)
    }

    fn maps_to(&mut self) -> Result<Raw, String> {
        let left = self.relation()?;
        if !self.take(Symbol::MapsTo) {
            return Ok(left);
        }
        let right = self.relation()?;
        if self.symbol(Symbol::MapsTo) {
            return Err(error("`|->` does not chain: parenthesize"));
        }
        self.bin(Op::MapsTo, left, right)
    }

    fn relation(&mut self) -> Result<Raw, String> {
        let left = self.set()?;
        let op = if self.keyword("in") {
            Op::Mem
        } else if self.symbol(Symbol::EqualEqual) {
            Op::Eqv
        } else {
            return Ok(left);
        };
        self.at += 1;
        let right = self.set()?;
        if self.keyword("in") || self.symbol(Symbol::EqualEqual) {
            return Err(error("`in` and `==` do not chain: parenthesize"));
        }
        self.bin(op, left, right)
    }

    fn set(&mut self) -> Result<Raw, String> {
        let mut left = self.primary()?;
        if self.symbol(Symbol::Pipe) {
            while self.take(Symbol::Pipe) {
                let right = self.primary()?;
                left = self.bin(Op::Union, left, right)?;
            }
            if self.symbol(Symbol::Backslash) {
                return Err(error("`|` and `\\` do not mix: parenthesize"));
            }
        } else if self.take(Symbol::Backslash) {
            let right = self.primary()?;
            left = self.bin(Op::Diff, left, right)?;
            if self.symbol(Symbol::Pipe) || self.symbol(Symbol::Backslash) {
                return Err(error("`\\` does not chain or mix with `|`: parenthesize"));
            }
        }
        if self.symbol(Symbol::Minus) {
            return Err(error("set difference is spelled `\\`, not `-`"));
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Raw, String> {
        self.enter()?;
        let raw = match self.peek().cloned() {
            Some(TokenKind::Symbol(Symbol::LeftParen)) => {
                self.at += 1;
                let inner = self.implication()?;
                self.need(Symbol::RightParen, "`)`")?;
                inner
            }
            Some(TokenKind::Symbol(Symbol::LeftBrace)) => {
                self.at += 1;
                self.need(Symbol::RightBrace, "`}` (the only set literal is `{}`)")?;
                self.node(Raw::Empty)?
            }
            Some(TokenKind::Word(word)) if word == "quant" => {
                self.at += 1;
                self.need_keyword("all")?;
                self.need(Symbol::Hash, "`#` before the bound variable")?;
                let var = self.name()?;
                self.need_keyword("in")?;
                let domain = self.set()?;
                self.need(Symbol::Colon, "`:` after the domain")?;
                let body = self.implication()?;
                self.node(Raw::Quant {
                    var,
                    domain: Box::new(domain),
                    body: Box::new(body),
                })?
            }
            Some(TokenKind::Word(word)) if constant_word(&word) => {
                self.at += 1;
                self.node(Raw::Const(word))?
            }
            Some(TokenKind::Symbol(Symbol::Hash)) => {
                self.at += 1;
                let name = self.name()?;
                if self.take(Symbol::LeftParen) {
                    let first = self.implication()?;
                    let raw = if self.take(Symbol::Comma) {
                        let second = self.implication()?;
                        if self.symbol(Symbol::Comma) {
                            return Err(error("no term takes more than two arguments"));
                        }
                        Raw::App2(name, Box::new(first), Box::new(second))
                    } else {
                        Raw::App1(name, Box::new(first))
                    };
                    self.need(Symbol::RightParen, "`)` after the arguments")?;
                    self.node(raw)?
                } else {
                    self.node(Raw::App0(name))?
                }
            }
            _ => return Err(error("expected a term, `quant`, `(` or `{}`")),
        };
        self.depth -= 1;
        Ok(raw)
    }
}

/// The syntax step: the whole of `tokens` (no end-of-file token) as one
/// formula, or a rejection.
pub fn parse_tokens(tokens: &[TokenKind]) -> Result<Raw, String> {
    if tokens.is_empty() {
        return Err(error("empty formula"));
    }
    if tokens.len() > MAX_TOKENS {
        return Err(error("too many tokens"));
    }
    let mut reader = Reader {
        tokens,
        at: 0,
        nodes: 0,
        depth: 0,
    };
    let raw = reader.implication()?;
    if reader.at != tokens.len() {
        return Err(error("unexpected token after the formula"));
    }
    Ok(raw)
}

/// The registered term `name`.
#[must_use]
pub fn term(name: &str) -> Option<&'static Term> {
    VOCABULARY.iter().find(|term| term.name == name)
}

fn sort_of(context: &[(String, Sort)], raw: &Raw) -> Result<Sort, String> {
    let ill = |what: &str| Err(error(&format!("ill-sorted {what}")));
    match raw {
        Raw::Quant { var, domain, body } => {
            if term(var).is_some() || context.iter().any(|(bound, _)| bound == var) {
                return Err(error(&format!(
                    "`#{var}` shadows a term or a bound variable"
                )));
            }
            let element = match sort_of(context, domain)? {
                Sort::SetObj => Sort::Obj,
                Sort::SetFn => Sort::Fn,
                Sort::SetMeth => Sort::Meth,
                _ => return ill("quantifier domain"),
            };
            let mut inner = context.to_vec();
            inner.push((var.clone(), element));
            match sort_of(&inner, body)? {
                Sort::Prop => Ok(Sort::Prop),
                _ => ill("quantifier body"),
            }
        }
        Raw::Bin(op, left, right) => {
            let (l, r) = (sort_of(context, left)?, sort_of(context, right)?);
            match (op, l, r) {
                (Op::Imp, Sort::Prop, Sort::Prop) => Ok(Sort::Prop),
                (Op::Conj, Sort::Prop, Sort::Prop) => Ok(Sort::Prop),
                (Op::Conj, Sort::Eff, Sort::Eff) => Ok(Sort::Eff),
                (Op::MapsTo, Sort::Fn | Sort::SetFn, Sort::Eff) => Ok(Sort::Prop),
                (Op::Mem, element, set) if element.set() == Some(set) => Ok(Sort::Prop),
                (Op::Eqv, Sort::SetRes, Sort::SetRes) => Ok(Sort::Eff),
                (Op::Union | Op::Diff, Sort::SetObj, Sort::SetObj) => Ok(Sort::SetObj),
                (Op::Union | Op::Diff, Sort::SetFn, Sort::SetFn) => Ok(Sort::SetFn),
                (Op::Union | Op::Diff, Sort::SetMeth, Sort::SetMeth) => Ok(Sort::SetMeth),
                _ => ill(&format!("`{}`", op.spelling())),
            }
        }
        Raw::App0(name) => {
            if let Some((_, sort)) = context.iter().rev().find(|(bound, _)| bound == name) {
                return Ok(*sort);
            }
            match term(name) {
                Some(term) if term.arguments.is_empty() => Ok(term.result),
                Some(term) => Err(error(&format!(
                    "`#{name}` takes {} argument(s)",
                    term.arguments.len()
                ))),
                None => Err(error(&format!(
                    "unknown term or unbound variable `#{name}`"
                ))),
            }
        }
        Raw::App1(name, argument) => {
            let Some(term) = term(name) else {
                return Err(error(&format!("unknown term `#{name}`")));
            };
            let argument = sort_of(context, argument)?;
            if term.arguments == [argument] {
                Ok(term.result)
            } else {
                ill(&format!("application of `#{name}`"))
            }
        }
        Raw::App2(name, first, second) => {
            let Some(term) = term(name) else {
                return Err(error(&format!("unknown term `#{name}`")));
            };
            let (first, second) = (sort_of(context, first)?, sort_of(context, second)?);
            if term.arguments == [first, second] {
                Ok(term.result)
            } else {
                ill(&format!("application of `#{name}`"))
            }
        }
        Raw::Const(word) => {
            if CLASSES.contains(&word.as_str()) {
                Ok(Sort::Cls)
            } else if CANDIDATE_INVENTORIES.contains(&word.as_str()) {
                // Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS.
                Ok(Sort::Obj)
            } else {
                Err(error(&format!("unknown constant `{word}`")))
            }
        }
        Raw::Empty => Ok(Sort::SetRes),
    }
}

/// The sort of a closed formula, or the rejection (`QCore.PremiseFormula.check`).
pub fn sort(raw: &Raw) -> Result<Sort, String> {
    sort_of(&[], raw)
}

/// The well-formedness judgment: a closed, well-sorted proposition
/// (`QCore.PremiseFormula.wf`).
pub fn check(raw: &Raw) -> Result<(), String> {
    match sort(raw)? {
        Sort::Prop => Ok(()),
        _ => Err(error("a premise states a proposition")),
    }
}

/// A well-formed premise formula and its canonical printing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PremiseFormula {
    pub raw: Raw,
    pub canonical: String,
}

/// Syntax, then sorts: the formula `tokens` spell, or the rejection.
pub fn read(tokens: &[TokenKind]) -> Result<PremiseFormula, String> {
    let raw = parse_tokens(tokens)?;
    check(&raw)?;
    let canonical = canonical(&raw);
    Ok(PremiseFormula { raw, canonical })
}

/// [`read`] over source text.
pub fn read_source(source: &str) -> Result<PremiseFormula, String> {
    let tokens = lex(source)
        .map_err(|e| error(&e.to_string()))?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| *kind != TokenKind::Eof)
        .collect::<Vec<_>>();
    read(&tokens)
}

fn level(raw: &Raw) -> u8 {
    match raw {
        Raw::Quant { .. } => 0,
        Raw::Bin(op, ..) => op.level(),
        _ => 6,
    }
}

/// `open`: nothing follows this node inside its parentheses, so a
/// quantifier may stand bare there (its body extends to the end).
fn print(raw: &Raw, open: bool, out: &mut String) {
    match raw {
        Raw::Quant { var, domain, body } => {
            if !open {
                out.push('(');
            }
            out.push_str("quant all #");
            out.push_str(var);
            out.push_str(" in ");
            operand(domain, level(domain) < 5, false, out);
            out.push_str(": ");
            print(body, true, out);
            if !open {
                out.push(')');
            }
        }
        Raw::Bin(op, left, right) => {
            let (l, r) = (level(left), level(right));
            let (left_parens, right_parens) = match op {
                // Right-associative.
                Op::Imp => (l <= 1, r < 1 && !open),
                // Left-associative.
                Op::Conj => (l < 2, r <= 2 && !(r == 0 && open)),
                Op::MapsTo => (l <= 3, r <= 3),
                Op::Mem | Op::Eqv => (l <= 4, r <= 4),
                Op::Union => (l < 5 || matches!(**left, Raw::Bin(Op::Diff, ..)), r <= 5),
                Op::Diff => (l <= 5, r <= 5),
            };
            operand(left, left_parens, false, out);
            out.push(' ');
            out.push_str(op.spelling());
            out.push(' ');
            operand(right, right_parens, open, out);
        }
        Raw::App0(name) => {
            out.push('#');
            out.push_str(name);
        }
        Raw::App1(name, argument) => {
            out.push('#');
            out.push_str(name);
            out.push('(');
            print(argument, true, out);
            out.push(')');
        }
        Raw::App2(name, first, second) => {
            out.push('#');
            out.push_str(name);
            out.push('(');
            print(first, true, out);
            out.push_str(", ");
            print(second, true, out);
            out.push(')');
        }
        Raw::Const(word) => out.push_str(word),
        Raw::Empty => out.push_str("{}"),
    }
}

fn operand(raw: &Raw, parens: bool, open: bool, out: &mut String) {
    if parens {
        out.push('(');
        print(raw, true, out);
        out.push(')');
    } else {
        print(raw, open, out);
    }
}

/// The canonical printing: single spaces around every binary operator, `: `
/// after a quantifier's domain, `, ` between arguments, and parentheses only
/// where the precedence needs them. [`parse_tokens`] of the printing is the
/// same tree.
#[must_use]
pub fn canonical(raw: &Raw) -> String {
    let mut out = String::new();
    print(raw, true, &mut out);
    out
}

/// The tree in the Lean model's syntax (`QCore.PremiseFormula.Raw`).
#[must_use]
pub fn lean(raw: &Raw) -> String {
    match raw {
        Raw::Quant { var, domain, body } => {
            format!("(.quant \"{var}\" {} {})", lean(domain), lean(body))
        }
        Raw::Bin(op, left, right) => format!("(.bin {} {} {})", op.lean(), lean(left), lean(right)),
        Raw::App0(name) => format!("(.app0 \"{name}\")"),
        Raw::App1(name, argument) => format!("(.app1 \"{name}\" {})", lean(argument)),
        Raw::App2(name, first, second) => {
            format!("(.app2 \"{name}\" {} {})", lean(first), lean(second))
        }
        Raw::Const(word) => format!("(.const \"{word}\")"),
        Raw::Empty => ".empty".to_owned(),
    }
}

/// The introducer of a named premise.
pub const PREMISE_HEAD: &str = "resource_premise";

/// Card **SP5-GLOBAL-PROSE-PREMISE-BAN** (contract specification C04.18,
/// diagnostics namespace `QF***`): the code of the refusal of a premise whose
/// statement is a string. The string form `resource_premise(NAME,
/// "STATEMENT")` is retired: a premise states a C04.18.9 formula, and a
/// module stating a string is rejected whole, on every route that reads the
/// contract format (all of them read it through [`premise_clause`]).
pub const PROSE_PREMISE: &str = "QF001";

/// A named premise clause, read from its expression's tokens (no `requires`,
/// no `;`, no end-of-file token). `Ok(None)` when the clause is not headed
/// `resource_premise(`; a clause so headed is `resource_premise(NAME,
/// FORMULA)`, `NAME` a lower-case identifier and `FORMULA` a well-formed
/// formula ([`read`]), and anything else rejects. A string statement is
/// refused by name ([`PROSE_PREMISE`]), never read and never dropped: there
/// is no string statement in this reader's result.
pub fn premise_clause(tokens: &[TokenKind]) -> Result<Option<(String, PremiseFormula)>, String> {
    if !matches!(tokens.first(), Some(TokenKind::Word(head)) if head == PREMISE_HEAD) {
        return Ok(None);
    }
    let malformed = || {
        "malformed premise: write `requires resource_premise(NAME, FORMULA);` with a \
         lowercase identifier and a C04.18.9 formula"
            .to_owned()
    };
    let [
        _,
        TokenKind::Symbol(Symbol::LeftParen),
        TokenKind::Word(name),
        TokenKind::Symbol(Symbol::Comma),
        statement @ ..,
        TokenKind::Symbol(Symbol::RightParen),
    ] = tokens
    else {
        return Err(malformed());
    };
    let identifier = name.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if !identifier {
        return Err(malformed());
    }
    if statement
        .iter()
        .any(|token| matches!(token, TokenKind::StringLiteral(_)))
    {
        return Err(prose_premise(name));
    }
    let formula = read(statement).map_err(|e| format!("premise `{name}`: {e}"))?;
    Ok(Some((name.clone(), formula)))
}

/// The refusal of a premise named `name` whose statement is (or contains) a
/// string literal.
#[must_use]
pub fn prose_premise(name: &str) -> String {
    format!(
        "{PROSE_PREMISE}: premise `{name}` states a string; the string form \
         `resource_premise(NAME, \"STATEMENT\")` is retired (contract specification \
         C04.18): write `requires resource_premise({name}, FORMULA);` with a C04.18.9 formula"
    )
}

/// The retired introducer of an opaque hypothesis atom.
pub const HYPOTHESIS_HEAD: &str = "resource_hypothesis";

/// Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS** (contract specification C04.18.7,
/// diagnostics namespace `QF***`): the code of the refusal of a hypothesis
/// atom stated as a string. The form `resource_hypothesis(NAME,
/// "STATEMENT")` (C04.18.1's only atom form) is retired: an atom is a typed
/// proposition, `quant proposition #NAME(PARAMETERS) = FORMULA;` (G01p). A
/// module stating one is rejected whole by the structural parser every
/// route reads the contract format through ([`hypothesis_clause`]), whatever
/// the clause's introducer; the atom is never read and never dropped.
pub const PROSE_HYPOTHESIS: &str = "QF002";

/// Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS**: the refusal of a clause headed
/// `resource_hypothesis`, read from its expression's tokens (no introducer,
/// no `;`). `Ok(())` when the clause is not so headed. Every such clause is
/// refused ([`PROSE_HYPOTHESIS`]): the string statement was the form's only
/// statement, so there is nothing left to admit.
pub fn hypothesis_clause(tokens: &[TokenKind]) -> Result<(), String> {
    if !matches!(tokens.first(), Some(TokenKind::Word(head)) if head == HYPOTHESIS_HEAD) {
        return Ok(());
    }
    let name = match tokens {
        [
            _,
            TokenKind::Symbol(Symbol::LeftParen),
            TokenKind::Word(name),
            ..,
        ] => name.as_str(),
        _ => "NAME",
    };
    Err(prose_hypothesis(name))
}

/// The refusal of a `resource_hypothesis` atom named `name`.
#[must_use]
pub fn prose_hypothesis(name: &str) -> String {
    format!(
        "{PROSE_HYPOTHESIS}: hypothesis atom `{name}` uses the retired string form \
         `resource_hypothesis(NAME, \"STATEMENT\")` (contract specification C04.18.7): write \
         `quant proposition #{name}(PARAMETERS) = FORMULA;` with a typed G01p formula"
    )
}

/// Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS** (contract specification
/// C04.20, grammar G05g, diagnostics namespace `QF***`): the code of the
/// refusal of an outer contract definition whose statement is a string. The
/// form `quant #outer [[contract]] NAME, "STATEMENT";` is retired: a
/// definition states a C04.18.9 formula, `quant #outer [[contract]] NAME,
/// FORMULA;`. A module stating a string is rejected whole by the structural
/// parser every route reads the contract format through ([`outer_header`]),
/// and by the outer contract loader; the statement is never read and never
/// dropped.
pub const PROSE_OUTER: &str = "QF004";

/// An outer contract header, read from its clause's tokens starting at
/// `#outer` (no `quant`, no `;`). `Ok(None)` when the clause is not a header
/// (`#outer [[contract]] …`) or its shape is none of the two forms (the outer
/// loader names the shape error). `Ok(Some((NAME, None)))` is a citation,
/// `Ok(Some((NAME, Some(formula))))` a definition whose statement is a
/// well-formed formula ([`read`]). A statement holding a string literal is
/// refused by name ([`PROSE_OUTER`]), whatever the name's spelling; a
/// formula the judgment refuses rejects too.
pub fn outer_header(
    tokens: &[TokenKind],
) -> Result<Option<(String, Option<PremiseFormula>)>, String> {
    let [
        TokenKind::Symbol(Symbol::Hash),
        TokenKind::Word(outer),
        TokenKind::Symbol(Symbol::LeftBracket),
        TokenKind::Symbol(Symbol::LeftBracket),
        TokenKind::Word(contract),
        TokenKind::Symbol(Symbol::RightBracket),
        TokenKind::Symbol(Symbol::RightBracket),
        rest @ ..,
    ] = tokens
    else {
        return Ok(None);
    };
    if outer != "outer" || contract != "contract" {
        return Ok(None);
    }
    match rest {
        [TokenKind::Word(name)] => Ok(Some((name.clone(), None))),
        [
            TokenKind::Word(name),
            TokenKind::Symbol(Symbol::Comma),
            statement @ ..,
        ] if !statement.is_empty() => {
            if statement
                .iter()
                .any(|token| matches!(token, TokenKind::StringLiteral(_)))
            {
                return Err(prose_outer(name));
            }
            let formula = read(statement).map_err(|e| format!("outer contract `{name}`: {e}"))?;
            Ok(Some((name.clone(), Some(formula))))
        }
        [first, ..] if tokens_hold_string(rest) => {
            let name = match first {
                TokenKind::Word(name) => name.as_str(),
                _ => "NAME",
            };
            Err(prose_outer(name))
        }
        _ => Ok(None),
    }
}

fn tokens_hold_string(tokens: &[TokenKind]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token, TokenKind::StringLiteral(_)))
}

/// The refusal of an outer contract `name` whose statement is (or contains)
/// a string literal.
#[must_use]
pub fn prose_outer(name: &str) -> String {
    format!(
        "{PROSE_OUTER}: outer contract `{name}` states a string; the string form \
         `quant #outer [[contract]] NAME, \"STATEMENT\";` is retired (contract specification \
         C04.20): write `quant #outer [[contract]] {name}, FORMULA;` with a C04.18.9 formula"
    )
}

/// Card **SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS**: the outer header decision
/// vectors shared with `proof/QCore/PremiseFormula/Outer.lean`
/// (`outerVectors`): a definition header's tokens from `#outer`, and whether
/// [`outer_header`] accepts it as a definition. A statement holding a string
/// literal renders as the Lean `.str` (refused with [`PROSE_OUTER`]); any
/// other renders as `.formula` of its tree, accepted exactly when
/// well-formed.
pub const OUTER_DECISION_VECTORS: &[(&str, bool)] = &[
    (
        "#outer [[contract]] plugin, #outer_code |-> #stores_none(#flb_core_owned)",
        true,
    ),
    (
        "#outer [[contract]] plugin, quant all #f in #interceptors: #f in #zero_appends(CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147)",
        true,
    ),
    ("#outer [[contract]] plugin, #allocator_hooks_premise", true),
    // The retired string form, whatever the string says.
    (
        "#outer [[contract]] plugin, \"a loaded plugin stores into no member the core owns\"",
        false,
    ),
    // A quoted well-formed formula is a string.
    (
        "#outer [[contract]] plugin, \"#outer_code |-> #balanced\"",
        false,
    ),
    // A string beside a formula.
    (
        "#outer [[contract]] plugin, #outer_code |-> #balanced && \"and more\"",
        false,
    ),
    // A formula the judgment refuses: code is not a set of cells.
    (
        "#outer [[contract]] plugin, #outer_code |-> #stores_none(#outer_code)",
        false,
    ),
    // An inventory constant no provider registers.
    (
        "#outer [[contract]] plugin, quant all #f in #interceptors: #f in #zero_appends(CANDIDATES_0000000000000000000000000000000000000000000000000000000000000000)",
        false,
    ),
];

/// Card **SP5-GLOBAL-PROSE-PREMISE-BAN**: the clause decision vectors shared
/// with `proof/QCore/PremiseFormula.lean` (`clauseVectors`): a premise
/// clause's expression and whether [`premise_clause`] accepts it as a
/// premise. Each statement either holds a string literal (the Lean `.str`,
/// refused with [`PROSE_PREMISE`]) or parses as a formula (the Lean
/// `.formula`, accepted exactly when well-formed); the test binds each row to
/// the Lean table and its decision to this reader.
pub const CLAUSE_DECISION_VECTORS: &[(&str, bool)] = &[
    (
        "resource_premise(slots, quant all #f in #stated: #f |-> #inert)",
        true,
    ),
    (
        "resource_premise(served_queue, #op_callbacks |-> #takes(#held_on(#served)) == {})",
        true,
    ),
    ("resource_premise(never, #false)", true),
    // The retired string form, whatever the string says.
    (
        "resource_premise(served_queue, \"a callback takes nothing\")",
        false,
    ),
    // A quoted well-formed formula is a string.
    ("resource_premise(slots, \"#stated |-> #inert\")", false),
    ("resource_premise(blank, \"\")", false),
    // A string beside a formula.
    (
        "resource_premise(mixed, #stated |-> #inert && \"and more\")",
        false,
    ),
    // A formula the judgment refuses.
    ("resource_premise(unknown, #unknown |-> #inert)", false),
    ("resource_premise(unsorted, #freed |-> #inert)", false),
];

/// The decision vectors shared with `proof/QCore/PremiseFormula.lean`
/// (`decisionVectors`): a formula [`parse_tokens`] reads, and whether the
/// judgment [`check`] accepts it. The test binds each row's tree to the Lean
/// table and its decision to this reader.
pub const DECISION_VECTORS: &[(&str, bool)] = &[
    (
        "quant all #o in #created | #copied | #freed: #ex_data(#class(#o)) |-> #inert",
        true,
    ),
    (
        "#stated |-> (#takes(#caller_held) == {} && #releases(#caller_held) == {} && #retains(#caller_held) == {})",
        true,
    ),
    ("quant all #f in #stated: #f |-> #inert", true),
    (
        "quant all #b in #of(BIO, #args): #b in #freed -> #ex_data_free(#class(#b)) |-> #inert",
        true,
    ),
    (
        "quant all #b in #args: #method(#b) in #builtin -> #fns(#b) |-> #inert",
        true,
    ),
    (
        "#slots(#args) \\ #stated |-> #inert && #slots(#ctx(#args)) |-> #inert",
        true,
    ),
    // An unknown term.
    ("#unknown |-> #inert", false),
    // An ill-sorted application: `#class` takes an object, not a set.
    ("#ex_data(#class(#args)) |-> #inert", false),
    // An unbound variable.
    (
        "quant all #o in #created: #ex_data(#class(#x)) |-> #inert",
        false,
    ),
    // A variable shadowing a term.
    ("quant all #args in #created: #stated |-> #inert", false),
    // A variable shadowing a variable.
    (
        "quant all #o in #created: quant all #o in #freed: #stated |-> #inert",
        false,
    ),
    // Not a proposition: a callback set, an effect.
    ("#stated", false),
    ("#inert", false),
    // A union of two sorts.
    ("#created | #stated |-> #inert", false),
    // Membership of a class in a set of objects.
    ("SSL in #args", false),
    // A set of objects is not a callback set.
    ("#freed |-> #inert", false),
    // An unknown constant.
    (
        "quant all #b in #of(NOT_A_CLASS, #args): #fns(#b) |-> #inert",
        false,
    ),
    // Arguments in the wrong order.
    (
        "quant all #b in #of(#args, BIO): #fns(#b) |-> #inert",
        false,
    ),
    // A term used without its arguments.
    ("#slots |-> #inert", false),
    // A quantifier body that is not a proposition.
    ("quant all #o in #created: #o", false),
    // The empty set is a resource set only.
    ("{} |-> #inert", false),
    // An effect where a proposition stands.
    ("#stated |-> #inert && #inert", false),
    // ---- Card SP2-SYMBOLIC-LIBRDKAFKA-PREMISES: the qcore-provider-librdkafka
    // block, one or more rows per new signature shape. ----
    // The falsum is a proposition, so a premise may state it.
    ("#false", true),
    ("#concurrent |-> #keeps_links(#parent)", true),
    (
        "quant all #c in #children(#parent): #c in #next_reach(#c) -> #false",
        true,
    ),
    (
        "quant all #b in #children(#parent): quant all #a in #pred(#parent, #b): #a in #prev(#b)",
        true,
    ),
    ("#op_callbacks |-> #takes(#held_on(#served)) == {}", true),
    (
        "#call |-> #returns_result_of(#rd_calloc) && #audited_assertions",
        true,
    ),
    (
        "quant all #f in #ran(#event_cb(#forwards(#logq))): #false",
        true,
    ),
    ("#nodup(#parent)", true),
    (
        "#coord_rkb_monitor_cb |-> (#handed_only(#broker) && #reads_only(#op))",
        true,
    ),
    // `|->` binds tighter than `&&`, so an unparenthesized second effect
    // stands where a proposition must.
    (
        "#coord_rkb_monitor_cb |-> #handed_only(#broker) && #reads_only(#op)",
        false,
    ),
    ("#call in #registered", true),
    // A proposition is not a callback.
    ("#false |-> #inert", false),
    // A set of objects where an object stands.
    ("#nodup(#children(#parent))", false),
    // A two-argument term with one argument.
    ("#pred(#parent) in #live", false),
    // An object is not a callback.
    ("#parent |-> #inert", false),
    ("#parent in #registered", false),
    ("#call |-> #returns_result_of(#parent)", false),
    // An effect, alone or as an equation, is not a premise.
    ("#held_on(#served) == #refs(#op)", false),
    ("#keeps_links(#parent)", false),
    // An object is not a quantifier domain.
    ("quant all #x in #parent: #false", false),
    // ---- End of the SP2 block. ----
    // ---- Card SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES: every Fluent Bit term,
    // in an accepted and a refused position.
    (
        "quant all #r in #result: #r in #interior(#args) && #r in #chunk_name(#args)",
        true,
    ),
    (
        "quant all #r in #result: #r in #result \\ (#tracked | #claimed | #bound | #stored)",
        true,
    ),
    ("#admitted -> #faithful", true),
    (
        "quant all #g in #config_of(#args): #g in #ingesting -> #returns_zero",
        true,
    ),
    (
        "quant all #c in #collector: #c in #event_registered && #c in #set_running",
        true,
    ),
    (
        "#production -> (quant all #p in #out_plugin(#args): #p in #synchronous \\ #in_tree && #p in #proxy)",
        true,
    ),
    // A proposition is not a callback.
    ("#faithful |-> #inert", false),
    // A set of objects is not a proposition.
    ("#result", false),
    // A proposition is not a set of objects.
    ("#interior(#admitted) == {}", false),
    // A term without its argument.
    ("quant all #r in #result: #r in #chunk_name", false),
    // A proposition is not an element.
    ("#returns_zero in #collector", false),
    // An object set where a callback set stands.
    ("#out_plugin(#args) |-> #inert", false),
    // ---- Card SP4-SYMBOLIC-LIBEVENT-PREMISES (SP4 block). ----
    (
        "quant all #m in #args: #mk_parent(#m) in #live_at_use \\ (#released | #ended)",
        true,
    ),
    (
        "#interleaved |-> (#no_writes(#singleton(#libco_params_key)) && #no_resumes(#tracked))",
        true,
    ),
    (
        "quant all #m in #args: quant all #c in #holds(#mk_parent(#m)): #c in #singleton(#m)",
        true,
    ),
    (
        "quant all #f in #stack_deregister(#libco_params_key): #f |-> #no_releases(#tracked)",
        true,
    ),
    // `|->` binds tighter than `&&`: an effect conjoined to a proposition.
    (
        "#interleaved |-> #no_writes(#tracked) && #no_resumes(#tracked)",
        false,
    ),
    // An accessor takes an object, not a set.
    ("#mk_parent(#args) in #live_at_use", false),
    // An effect takes a set, not an object.
    ("#interleaved |-> #no_writes(#libco_params_key)", false),
    // Objects are not callbacks.
    ("#live_at_use |-> #inert", false),
    // Membership of an object in an object.
    ("#libco_params_key in #libco_params_key", false),
    // An effect is not a proposition.
    ("#no_writes(#tracked)", false),
    // A singleton set of objects is not a resource set.
    (
        "#interleaved |-> #singleton(#libco_params_key) == {}",
        false,
    ),
    // ---- Card SP5-GLOBAL-PROSE-PREMISE-BAN (SP5 block). ----
    (
        "quant all #m in #condition_reads: #m in #entry_stable",
        true,
    ),
    // Cells are not callbacks.
    ("#condition_reads |-> #inert", false),
    // A set of cells is not a proposition.
    ("#entry_stable", false),
    // A set of cells is not an element.
    ("#condition_reads in #entry_stable", false),
    // ---- Card SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS (SP8 block). ----
    (
        "#outer_code |-> (#calls_none(#cjson_init_hooks) && #stores_none(#rk_internal) && #meets_published(#rk_public))",
        true,
    ),
    (
        "quant all #f in #copies(#conf_registrations | #interceptors): #f in #valid_through_last",
        true,
    ),
    (
        "quant all #f in #interceptors: #f |-> (#inert && #borrows_args && #borrows_handles && #no_global_custody && #balanced)",
        true,
    ),
    (
        "quant all #f in #interceptors: #f in #client_lifecycle(CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147) \\ #on_share_consumer",
        true,
    ),
    (
        "#outer_code |-> (#accesses_only(#handed) && #calls_only(#exported) && #provenance(#handed) && #typed_access)",
        true,
    ),
    // An inventory is an object, not a class.
    (
        "quant all #o in #of(CANDIDATES_5C3F8C4723832C0E9E6EBB784BE6A57EBC0264D519421D5B95A18A8F68C18147, #args): #false",
        false,
    ),
    // Cells are not callbacks.
    ("#outer_code |-> #calls_none(#cjson_link_cells)", false),
    // Code is not a set of cells.
    ("#outer_code |-> #stores_none(#outer_code)", false),
    // An effect is not a proposition.
    ("#typed_access", false),
    // A class is not an inventory.
    ("quant all #f in #zero_appends(SSL): #false", false),
];

// ---- Card SP6-SYMBOLIC-HYPOTHESIS-ATOMS: the typed rendering (C04.18.9's
// SP6 block). A typed hypothesis atom (C04.18.7, G01p) whose parameters are
// registered terms states, in first-order form, what a premise formula over
// those terms states. ----

/// The G01p value sorts of the typed rendering: C04.18.9's element sorts,
/// spelled as they are there, and `run` (the SP6 block).
pub const RENDERED_VALUE_SORTS: &[(&str, &str)] = &[
    ("obj", "an object (C04.18.9 `obj`)"),
    ("cls", "an object class (C04.18.9 `cls`)"),
    (
        "fn",
        "a callback: a function at a position the library calls it from (C04.18.9 `fn`)",
    ),
    ("meth", "a BIO method table (C04.18.9 `meth`)"),
    ("res", "a resource the caller holds (C04.18.9 `res`)"),
    (
        "run",
        "one run of a callback: what an `eff` term is a property of (card SP6)",
    ), // Card SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS (C04.18.9's SP9 block):
    // the plugin model's sorts, prefixed so no generic name is reserved.
    (
        "plg_event",
        "an event of the selected plugin trace (`PluginPremiseDecomposition.Event`): a registration, interception, invocation, acquisition, release, close, copy or other action of loaded code",
    ),
    (
        "plg_plugin",
        "a loaded artifact of the selected environment: a librdkafka plugin library or a Fluent Bit C or Go plugin",
    ),
    (
        "plg_family",
        "the family of a loaded artifact: librdkafka (`#plg_rk`), Fluent Bit C (`#plg_fluent_c`) or Fluent Bit Go (`#plg_fluent_go`)",
    ),
    (
        "plg_action",
        "the kind of a plugin trace event (`PluginPremiseDecomposition.Action`)",
    ),
    (
        "plg_callback",
        "a callback function of the plugin trace (an event's function)",
    ),
    (
        "plg_member",
        "an rk_conf callback member a registration or invocation is at",
    ),
    ("plg_context", "an opaque context an invocation receives"),
    ("plg_handle", "a loader handle of a loaded artifact"),
    ("plg_activation", "one activation of an interceptor"),
    (
        "plg_resource",
        "a resource an interceptor activation acquires or releases",
    ),
    ("plg_symbol", "a symbol a loaded artifact requires"),
    ("plg_digest", "an artifact, build or inventory identity"),
    (
        "plg_abi",
        "an ABI identity of a resolved or expected address",
    ),
    // Card SP10-FORMAL-BINDING-DESCRIPTIONS (C04.18.9's SP10 block): the
    // sorts of a binding's own terms, prefixed `bnd_`.
    (
        "bnd_verdict_context",
        "a nine-coordinate verdict Context (`HypothesisGraph.Context`)",
    ),
    (
        "bnd_property_view",
        "the property view a bound graph is replayed from (resource, bounds/null or declared model)",
    ),
];

/// The G01p sort of a rendered term: a value of a rendered value sort, a
/// predicate over rendered value sorts, or (card
/// SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS) a function from rendered value
/// sorts to a rendered value sort or `integer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderedSort {
    Value(&'static str),
    Predicate(&'static [&'static str]),
    Function(&'static [&'static str], &'static str),
}

impl RenderedSort {
    /// The G01p parameter sort.
    #[must_use]
    pub fn sort(self) -> qcore_schemas::quant_proposition::Sort {
        use qcore_schemas::quant_proposition::Sort as G;
        match self {
            Self::Value(name) => G::Value { name: name.into() },
            Self::Predicate(arguments) => G::Predicate {
                arguments: arguments.iter().map(|s| (*s).to_owned()).collect(),
            },
            Self::Function(arguments, result) => G::Function {
                arguments: arguments.iter().map(|s| (*s).to_owned()).collect(),
                result: result.to_owned(),
            },
        }
    }
}

/// A term of the typed rendering that no premise formula names (C04.18.9's
/// SP6, SP7, SP9 and SP10 blocks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderedTerm {
    pub name: &'static str,
    pub sort: RenderedSort,
    pub meaning: &'static str,
    /// The card whose block introduced the term (`SP6`, `SP7`, `SP9`, `SP10`).
    pub block: &'static str,
}

const OBJ: RenderedSort = RenderedSort::Value("obj");
const NULLARY: RenderedSort = RenderedSort::Predicate(&[]);
const OBJ1: RenderedSort = RenderedSort::Predicate(&["obj"]);
const OBJ2: RenderedSort = RenderedSort::Predicate(&["obj", "obj"]);
const OBJ3: RenderedSort = RenderedSort::Predicate(&["obj", "obj", "obj"]);
/// An integer-valued event component of the plugin model (SP10, SP9's scheme).
const PLG_EVENT_FLAG: RenderedSort = RenderedSort::Function(&["plg_event"], "integer");

/// The SP6 block, then the SP7 block, then the SP9 block, then the SP10 block.
///
/// SP6: `#runs` is the maps-to operator's own semantics: `F |-> E` holds
/// when every run of every member of `F` has `E` (`QCore.PremiseFormula`'s
/// `Model.runs`), so its rendering is `forall (#f: fn) { (F(#f) => forall
/// (#r: run) { (#runs(#f,#r) => E(#r)) }) }`.
///
/// SP7 (card SP6-GUARDED-SYMBOL-PROSE): the terms the guarded symbols of
/// `cJSON_DetachItemViaPointer` need beyond the registered ones: the call's
/// item and NULL arguments, the exit environment (each `#exit_X` is the
/// registered `#X` read at the call's exit; `#concurrent` and `#keeps_links`
/// do not depend on the environment), the erasure relating the two child
/// sequences, the admission, and two assumptions no model derives:
/// `#source_corresponds` (user decision 2026-11-23, environment E2) and
/// `#caller_refutes_membership` (user decision 2026-11-23,
/// invalid_membership I2: a warrant term inside a proposition). The Lean
/// model is `QCore.CjsonGuardedSymbols.render`.
///
/// SP9 (card SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS; user decisions of
/// 2026-11-23 on the card's stop: D1 A′, D2 b, D3 I2): the terms the 14
/// guarded symbols of the frozen plugin sidecar need, each read in one
/// selected `QCore.PluginPremiseDecomposition.Environment`, which stays
/// implicit (as SP7's entry terms read the entry environment). Every name is
/// prefixed `plg_`, so no generic name (`#time`, `#action`, `#member`) is
/// reserved; `#plg_X` is the typed atoms' parameter `#X` with its
/// environment argument dropped. `#plg_ev` is the reached invocation the
/// cases are read at, `#plg_admits_callback_closure` the admission, and two
/// assumptions no model derives: `#plg_trace_corresponds` (environment, as
/// E2) and `#plg_refutes_registration` (unregistered, a warrant term as I2).
/// The Lean model is `QCore.PluginGuardedSymbols.render`.
///
/// SP10 (card SP10-FORMAL-BINDING-DESCRIPTIONS; user decision 2026-11-24 on
/// the A′ prefix scheme: "Prefix, then land"): the terms a binding
/// manifest's or flow model's typed environment reference (C04.18.2,
/// C04.18.3) needs beyond a bound atom's own parameters. The plugin-model
/// components no bound atom quantifies (the cJSON hook and list components,
/// the frame, API and pointer-forgery components) follow SP9's scheme:
/// prefixed `plg_`, environment implicit, over SP9's sorts. The terms of the
/// binding itself (the never-derived source correspondence, E2's form; the
/// held Context; the three property views) are prefixed `bnd_` over two
/// `bnd_` sorts. The Lean list is `QCore.FormalBindingEnvironments.sp10Terms`.
pub const RENDERED_TERMS: &[RenderedTerm] = &[
    RenderedTerm {
        name: "runs",
        sort: RenderedSort::Predicate(&["fn", "run"]),
        meaning: "the runs of a callback: `#runs(#f,#r)` says `#r` is a run of `#f` (the semantics of `|->`)",
        block: "SP6",
    },
    RenderedTerm {
        name: "item",
        sort: OBJ,
        meaning: "the object the call's item argument designates (meaningful when `#item_null()` does not hold)",
        block: "SP7",
    },
    RenderedTerm {
        name: "item_null",
        sort: NULLARY,
        meaning: "the call's item argument is NULL",
        block: "SP7",
    },
    RenderedTerm {
        name: "parent_null",
        sort: NULLARY,
        meaning: "the call's parent argument is NULL",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_parent",
        sort: OBJ,
        meaning: "`#parent` at the call's exit: the parent object of the exit environment",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_live",
        sort: OBJ1,
        meaning: "`#live` at the call's exit: the objects live when the call returns",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_owned",
        sort: OBJ2,
        meaning: "`#owned` at the call's exit: `#exit_owned(p,c)` says the exit parent `p` owns `c`",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_children",
        sort: OBJ2,
        meaning: "`#children` at the call's exit: `#exit_children(p,c)` says `c` is in the exit parent `p`'s child sequence",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_child",
        sort: OBJ2,
        meaning: "`#child` at the call's exit: the item the exit parent's child member designates (none when NULL)",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_first",
        sort: OBJ2,
        meaning: "`#first` at the call's exit: the first element of the exit child sequence",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_last",
        sort: OBJ2,
        meaning: "`#last` at the call's exit: the last element of the exit child sequence",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_pred",
        sort: OBJ3,
        meaning: "`#pred` at the call's exit: `#exit_pred(p,b,a)` says `a` is immediately before `b` in the exit child sequence",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_next",
        sort: OBJ2,
        meaning: "`#next` at the call's exit: the item an item's next member designates when the call returns (none when NULL)",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_prev",
        sort: OBJ2,
        meaning: "`#prev` at the call's exit: the item an item's prev member designates when the call returns (none when NULL)",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_next_reach",
        sort: OBJ2,
        meaning: "`#next_reach` at the call's exit: the items reached by one or more exit next links",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_nodup",
        sort: OBJ1,
        meaning: "`#nodup` at the call's exit: the exit child sequence names no identity twice",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_runs",
        sort: RenderedSort::Predicate(&["fn", "run"]),
        meaning: "`#runs` over the exit environment's interference trace",
        block: "SP7",
    },
    RenderedTerm {
        name: "exit_erases",
        sort: OBJ1,
        meaning: "`#exit_erases(i)`: the exit child sequence is the entry child sequence with its first occurrence of `i` erased",
        block: "SP7",
    },
    RenderedTerm {
        name: "admits_removed",
        sort: NULLARY,
        meaning: "the gateway admits the call at its site under the declaration's Removed contract (its resource_owned, resource_removed and resource_return_removed clauses)",
        block: "SP7",
    },
    RenderedTerm {
        name: "source_corresponds",
        sort: NULLARY,
        meaning: "the call's actual entry heap, complete interference trace, caller identity and nine-coordinate context are what the terms denote: an independent assumption no model or source check establishes, whose disposition stays Open (user decision 2026-11-23)",
        block: "SP7",
    },
    RenderedTerm {
        name: "caller_refutes_membership",
        sort: OBJ2,
        meaning: "`#caller_refutes_membership(p,i)`: the caller holds positive evidence that `i` is not an owned child of `p`; it entails `!#owned(p,i)` and is never derived from the absence of a membership proof (user decision 2026-11-23)",
        block: "SP7",
    },
    RenderedTerm {
        name: "plg_in_trace",
        sort: RenderedSort::Predicate(&["plg_event"]),
        meaning: "`#plg_in_trace(e)`: the event `e` is in the selected trace (every loaded artifact's callback events)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_action",
        sort: RenderedSort::Function(&["plg_event"], "plg_action"),
        meaning: "the event's action",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_plugin",
        sort: RenderedSort::Function(&["plg_event"], "plg_plugin"),
        meaning: "the loaded artifact the event belongs to",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_function",
        sort: RenderedSort::Function(&["plg_event"], "plg_callback"),
        meaning: "the callback the event registers, invokes or destroys",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_member",
        sort: RenderedSort::Function(&["plg_event"], "plg_member"),
        meaning: "the rk_conf callback member the event is at",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_opaque",
        sort: RenderedSort::Function(&["plg_event"], "plg_context"),
        meaning: "the opaque context the event passes",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_handle",
        sort: RenderedSort::Function(&["plg_event"], "plg_handle"),
        meaning: "the loader handle the event closes",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_resource",
        sort: RenderedSort::Function(&["plg_event"], "plg_resource"),
        meaning: "the resource the event acquires or releases",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_activation",
        sort: RenderedSort::Function(&["plg_event"], "plg_activation"),
        meaning: "the interceptor activation the event happens in",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_time",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "the event's position in the trace",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_result",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "the event's integer result (0 for a successful registration or interception)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_public_setter",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the event goes through a public setter, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_exact_candidate",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when an interception names the exact pinned list or union-arm candidate, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_pointer_borrowed",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when every pointer the event passes into an interceptor is borrowed, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_scalar_borrowed",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when every scalar resource the event passes into an interceptor is borrowed, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_local_resource",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the event's resource was acquired locally in its activation, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_conf_live",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the configuration an interception registers on is live, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_from_on_new",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when an interception is made during a run of an on_new interceptor, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_initialized",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the client an interception registers on is already initialized, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_share_consumer",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the client an interception registers on is a share consumer, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_config_registration",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when an interception is registered on a configuration (rather than a client), else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_in_interceptor",
        sort: RenderedSort::Function(&["plg_event"], "integer"),
        meaning: "1 when the event happens inside an interceptor activation, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_invoke",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a callback invocation",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_register",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a callback registration",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_intercept",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of an interceptor registration",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_acquire",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a local acquisition",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_release",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a release",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_retain",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a retention",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_transfer",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a custody transfer",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_return_local",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action dropping a local acquisition before its activation returns",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_global_custody",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action taking global custody",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_close",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a loader close",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_destroy_callback",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a destruction callback invocation",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_copy_reference",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action copying a registration with its loader reference",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_rk",
        sort: RenderedSort::Value("plg_family"),
        meaning: "the librdkafka plugin family",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_fluent_c",
        sort: RenderedSort::Value("plg_family"),
        meaning: "the Fluent Bit C plugin family",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_fluent_go",
        sort: RenderedSort::Value("plg_family"),
        meaning: "the Fluent Bit Go plugin family",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_loaded",
        sort: RenderedSort::Predicate(&["plg_plugin", "plg_family"]),
        meaning: "`#plg_loaded(p,f)`: the artifact `p` is loaded as a member of family `f`",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_loader_enabled",
        sort: RenderedSort::Function(&["plg_family"], "integer"),
        meaning: "1 when the selected build enables the family's loader, else 0",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_actual_build",
        sort: RenderedSort::Function(&["plg_plugin"], "plg_digest"),
        meaning: "the artifact's actual build identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_expected_build",
        sort: RenderedSort::Function(&["plg_plugin"], "plg_digest"),
        meaning: "the artifact's selected build identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_actual_inventory",
        sort: RenderedSort::Value("plg_digest"),
        meaning: "the actual loaded-artifact inventory identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_expected_inventory",
        sort: RenderedSort::Value("plg_digest"),
        meaning: "the selected loaded-artifact inventory identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_actual_artifact",
        sort: RenderedSort::Function(&["plg_plugin"], "plg_digest"),
        meaning: "the artifact's actual identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_expected_artifact",
        sort: RenderedSort::Function(&["plg_plugin"], "plg_digest"),
        meaning: "the artifact's selected identity",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_required_symbol",
        sort: RenderedSort::Predicate(&["plg_plugin", "plg_symbol"]),
        meaning: "`#plg_required_symbol(p,s)`: the artifact `p` requires the symbol `s`",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_resolved_address",
        sort: RenderedSort::Function(&["plg_plugin", "plg_symbol"], "integer"),
        meaning: "the address the symbol resolves to in the artifact (0 when unresolved)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_expected_address",
        sort: RenderedSort::Function(&["plg_plugin", "plg_symbol"], "integer"),
        meaning: "the selected address of the symbol in the artifact",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_actual_abi",
        sort: RenderedSort::Function(&["integer"], "plg_abi"),
        meaning: "the actual ABI at an address",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_expected_abi",
        sort: RenderedSort::Function(&["integer"], "plg_abi"),
        meaning: "the selected ABI at an address",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_member_candidate",
        sort: RenderedSort::Predicate(&["plg_member", "plg_callback"]),
        meaning: "`#plg_member_candidate(m,f)`: `f` is a declared candidate of the member `m`",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_static_default",
        sort: RenderedSort::Predicate(&["plg_member", "plg_callback"]),
        meaning: "`#plg_static_default(m,f)`: `f` is the member `m`'s stated static default in the selected build",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_code_live",
        sort: RenderedSort::Predicate(&["plg_callback", "integer"]),
        meaning: "`#plg_code_live(f,t)`: the callback's code is live at trace position `t`",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_context_live",
        sort: RenderedSort::Predicate(&["plg_context", "integer"]),
        meaning: "`#plg_context_live(x,t)`: the opaque context is live at trace position `t`",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_handle_for",
        sort: RenderedSort::Function(&["plg_plugin"], "plg_handle"),
        meaning: "the loader handle of the artifact",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_ref_count",
        sort: RenderedSort::Function(&["plg_handle", "integer"], "integer"),
        meaning: "the handle's loader reference count at a trace position",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_return_boundary",
        sort: RenderedSort::Function(&["plg_activation"], "integer"),
        meaning: "the trace position at which the activation returns",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_rk_proposed",
        sort: RenderedSort::Predicate(&["plg_plugin"]),
        meaning: "`#plg_rk_proposed(p)`: the artifact meets the proposed rk_plugin_library outer envelope (`PluginPremiseDecomposition.RkProposed`)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_fluent_proposed",
        sort: RenderedSort::Predicate(&["plg_plugin"]),
        meaning: "`#plg_fluent_proposed(p)`: the artifact meets the proposed flb_external_plugin outer envelope (`PluginPremiseDecomposition.FluentProposed`)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_ev",
        sort: RenderedSort::Value("plg_event"),
        meaning: "the reached rk_conf callback invocation of rd_kafka_dump0 the decomposition's cases and postconditions are read at; the decomposition is read at every reached invocation (user decision 2026-11-23)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_admits_callback_closure",
        sort: RenderedSort::Predicate(&[]),
        meaning: "the gateway admits every reached rk_conf callback invocation of rd_kafka_dump0 under its rk_plugin_callback_closure premise; never inferred from a sampled callback",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_trace_corresponds",
        sort: RenderedSort::Predicate(&[]),
        meaning: "the selected environment is the run's: every loaded librdkafka and Fluent Bit C/Go artifact, every callback event, the build, ABI and inventory and the full context are what the terms denote. It is an independent assumption: no model or source check establishes it, and its disposition stays Open (user decision 2026-11-23)",
        block: "SP9",
    },
    RenderedTerm {
        name: "plg_refutes_registration",
        sort: RenderedSort::Predicate(&["plg_event"]),
        meaning: "`#plg_refutes_registration(e)`: positive evidence that the invocation `e` has neither its required successful public registration nor a stated static default. It entails both absences and is never derived from a failed registration proof (user decision 2026-11-23, D3)",
        block: "SP9",
    },
    RenderedTerm {
        name: "bnd_source_corresponds",
        sort: NULLARY,
        meaning: "the terms of a binding's environment denote what the selected source, C code and native execution actually do, and the bound Lean propositions correspond to that source: an independent assumption no model, check or content hash establishes, whose disposition stays Open (the form of environment E2, user decision 2026-11-23; Lean `FormalBindingEnvironments.Interpretation.sourceCorresponds`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "bnd_held_context",
        sort: RenderedSort::Value("bnd_verdict_context"),
        meaning: "the nine independently held Context coordinates (artifact, property, semantic profile, scope, bounds, assumptions, source identity, build identity, evidence version) the binding is read under, compared by exact equality and never transported to another property or Context (Lean `HypothesisGraph.Context`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "bnd_resource_view",
        sort: RenderedSort::Value("bnd_property_view"),
        meaning: "the exact independently held resource-v1 source view the bound graph is replayed from (Lean `FormalBindingEnvironments.PropertyView.resource`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "bnd_null_view",
        sort: RenderedSort::Value("bnd_property_view"),
        meaning: "the exact independently held bounds-v1 null source view the bound graph is replayed from (Lean `FormalBindingEnvironments.PropertyView.null`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "bnd_model_view",
        sort: RenderedSort::Value("bnd_property_view"),
        meaning: "the exact independently held declared resource-type model view the bound graph is replayed from (Lean `FormalBindingEnvironments.PropertyView.model`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_hook_install",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a cJSON global allocator hook installation (`cJSON_InitHooks`; Lean `QCore.PluginPremiseDecomposition.Action.hookInstall`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_hook_replace",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action replacing the cJSON global allocator hooks (Lean `QCore.PluginPremiseDecomposition.Action.hookReplace`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_cjson_link_write",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action writing a cJSON item link (child, next, prev) by loaded code (Lean `QCore.PluginPremiseDecomposition.Action.cjsonLinkWrite`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_cjson_call",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a cJSON API call by loaded code (Lean `QCore.PluginPremiseDecomposition.Action.cjsonCall`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_proper_list",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the list a cJSON API call is handed is well formed, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.properList`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_preexisting_hooks_valid",
        sort: RenderedSort::Predicate(&["plg_plugin"]),
        meaning: "`#plg_preexisting_hooks_valid(p)`: the allocator hooks installed before the artifact `p` is loaded are valid (Lean `QCore.PluginPremiseDecomposition.Environment.preexistingHooksValid`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_core_write",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a store by loaded code into Fluent Bit core state (Lean `QCore.PluginPremiseDecomposition.Action.coreWrite`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_internal_write",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a store by loaded code into librdkafka internal state (Lean `QCore.PluginPremiseDecomposition.Action.internalWrite`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_api_call",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a library API call by loaded code (Lean `QCore.PluginPremiseDecomposition.Action.apiCall`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_is_rk_call",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when an API call is a librdkafka call, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.isRkCall`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_public_rk_symbol",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the called librdkafka symbol is public, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.publicRkSymbol`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_api_precondition",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the called API's public precondition holds at the event, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.apiPrecondition`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_access",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action of a pointer access by loaded code (Lean `QCore.PluginPremiseDecomposition.Action.access`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_handed_object",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the accessed object was handed to the loaded code, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.handedObject`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_exported_api",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the access goes through an exported API, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.exportedApi`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_forge",
        sort: RenderedSort::Value("plg_action"),
        meaning: "the action making a pointer from a non-pointer value (Lean `QCore.PluginPremiseDecomposition.Action.forge`)",
        block: "SP10",
    },
    RenderedTerm {
        name: "plg_forged",
        sort: PLG_EVENT_FLAG,
        meaning: "1 when the pointer used at the event is forged, else 0 (Lean `QCore.PluginPremiseDecomposition.Event.forged`)",
        block: "SP10",
    },
];

/// The G01p parameter sort of a name of the typed rendering: a registered
/// C04.18.9 term in its rendered sort, or an SP6/SP7/SP9/SP10 term; `None`
/// otherwise.
#[must_use]
pub fn rendered_parameter_sort(name: &str) -> Option<qcore_schemas::quant_proposition::Sort> {
    if let Some(t) = term(name) {
        rendered_sort(t)
    } else {
        RENDERED_TERMS
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.sort.sort())
    }
}

impl Sort {
    /// The G01p value sort of an element sort.
    fn rendered_element(self) -> Option<&'static str> {
        match self {
            Self::Obj => Some("obj"),
            Self::Cls => Some("cls"),
            Self::Fn => Some("fn"),
            Self::Meth => Some("meth"),
            Self::Res => Some("res"),
            _ => None,
        }
    }
}

/// The G01p parameter sort of a registered term, or `None` when the term has
/// no first-order rendering (it takes a set as an argument). An element-sorted
/// term is a value (nullary) or a function; a term of sort `set(S)` is a
/// predicate over its arguments and one `S` (`#t(x…, y)` is `y in #t(x…)`); a
/// `prop` term is a predicate over its arguments; an `eff` term is a predicate
/// over its arguments and one `run`.
#[must_use]
pub fn rendered_sort(term: &Term) -> Option<qcore_schemas::quant_proposition::Sort> {
    use qcore_schemas::quant_proposition::Sort as G;
    let mut arguments = term
        .arguments
        .iter()
        .map(|s| s.rendered_element().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    let element = |s: Sort| match s {
        Sort::SetObj => Some("obj"),
        Sort::SetFn => Some("fn"),
        Sort::SetMeth => Some("meth"),
        Sort::SetRes => Some("res"),
        _ => None,
    };
    Some(match term.result {
        Sort::Prop => G::Predicate { arguments },
        Sort::Eff => {
            arguments.push("run".into());
            G::Predicate { arguments }
        }
        result => {
            if let Some(member) = element(result) {
                arguments.push(member.into());
                G::Predicate { arguments }
            } else {
                let result = result.rendered_element()?.to_owned();
                if arguments.is_empty() {
                    G::Value { name: result }
                } else {
                    G::Function { arguments, result }
                }
            }
        }
    })
}

/// Card SP6-SYMBOLIC-HYPOTHESIS-ATOMS: whether a typed atom is a typed
/// rendering over registered terms: every parameter is a registered term
/// (C04.18.9's vocabulary) with its rendered sort, or an SP6, SP7 or SP9 term with its
/// sort, and every quantifier ranges over a rendered value sort. The atom's
/// meaning is then the terms' meanings, nothing more.
pub fn check_rendering(
    definition: &qcore_schemas::quant_proposition::Definition,
) -> Result<(), String> {
    use qcore_schemas::quant_proposition::Formula as F;
    for parameter in &definition.parameters {
        let expected = rendered_parameter_sort(&parameter.name);
        if expected.as_ref() != Some(&parameter.sort) {
            return Err(format!(
                "typed atom `{}`: parameter `#{}` is not a registered term in its rendered sort",
                definition.id, parameter.name
            ));
        }
    }
    fn walk(f: &F) -> Result<(), String> {
        match f {
            F::Forall { sort, body, .. } | F::Exists { sort, body, .. } => {
                if !RENDERED_VALUE_SORTS.iter().any(|(s, _)| s == sort) {
                    return Err(format!("quantifier over `{sort}`, not a rendered sort"));
                }
                walk(body)
            }
            F::Not { body } => walk(body),
            F::And { left, right } | F::Or { left, right } | F::Implies { left, right } => {
                walk(left)?;
                walk(right)
            }
            F::Predicate { .. } | F::Equal { .. } | F::Compare { .. } => Ok(()),
        }
    }
    walk(&definition.formula)
}

// ---- Card SP6-GUARDED-SYMBOL-PROSE: guarded symbols state formulas
// (contract specification C04.18.6, C04.18.9's SP7 block). ----

/// The reserved prefix of the guarded source heads (C04.18.6).
pub const GUARDED_PREFIX: &str = "resource_guarded_";

/// Card **SP6-GUARDED-SYMBOL-PROSE** (contract specification C04.18.6,
/// diagnostics namespace `QF***`): the code of the refusal of a guarded
/// symbol or guarded decomposition document stated as a string, and of a
/// guarded proposition that is not a canonical C04.18.9 typed-rendering
/// formula. `resource_guarded_symbol_v1(ID, ROLE, "TEXT", ...)` and
/// `resource_guarded_decomposition_v1(ENTRY, "DOCUMENT")` are retired: a
/// symbol states an unquoted formula over registered terms, a document its
/// unquoted clauses. A module stating either string is rejected whole by the
/// structural parser every route reads the contract format through.
pub const PROSE_GUARDED: &str = "QF003";

/// The refusal of the guarded clause headed `head` with ID `id` that states
/// a string.
#[must_use]
pub fn prose_guarded(head: &str, id: &str) -> String {
    if head.starts_with("resource_guarded_decomposition") {
        format!(
            "{PROSE_GUARDED}: guarded document `{id}` is a string; the string form \
             `resource_guarded_decomposition_v1(ENTRY, \"DOCUMENT\")` is retired (contract \
             specification C04.18.6): write its clauses unquoted, separated by `,`"
        )
    } else {
        format!(
            "{PROSE_GUARDED}: guarded symbol `{id}` states a string; the string form \
             `resource_guarded_symbol_v1(ID, ROLE, \"TEXT\", ...)` is retired (contract \
             specification C04.18.6): write its proposition as a C04.18.9 typed-rendering \
             formula over registered terms"
        )
    }
}

/// The definition a guarded formula `formula` of symbol `id` reads as: its
/// free names, each with its rendered sort, as parameters (sorted by name).
fn guarded_definition(
    id: &str,
    formula: qcore_schemas::quant_proposition::Formula,
) -> Result<qcore_schemas::quant_proposition::Definition, String> {
    use qcore_schemas::quant_proposition::{Definition, Formula as F, Parameter, Term};
    use std::collections::BTreeSet;
    fn term_names(t: &Term, bound: &BTreeSet<String>, free: &mut BTreeSet<String>) {
        match t {
            Term::Variable { name } => {
                if !bound.contains(name) {
                    free.insert(name.clone());
                }
            }
            Term::Integer { .. } => {}
            Term::Apply {
                function,
                arguments,
            } => {
                free.insert(function.clone());
                for a in arguments {
                    term_names(a, bound, free);
                }
            }
            Term::Add { left, right } | Term::Sub { left, right } | Term::Mul { left, right } => {
                term_names(left, bound, free);
                term_names(right, bound, free);
            }
            Term::Neg { term } => term_names(term, bound, free),
        }
    }
    fn names(f: &F, bound: &mut BTreeSet<String>, free: &mut BTreeSet<String>) {
        match f {
            F::Predicate { name, arguments } => {
                free.insert(name.clone());
                for a in arguments {
                    term_names(a, bound, free);
                }
            }
            F::Equal { left, right } | F::Compare { left, right, .. } => {
                term_names(left, bound, free);
                term_names(right, bound, free);
            }
            F::Not { body } => names(body, bound, free),
            F::And { left, right } | F::Or { left, right } | F::Implies { left, right } => {
                names(left, bound, free);
                names(right, bound, free);
            }
            F::Forall { variable, body, .. } | F::Exists { variable, body, .. } => {
                let fresh = bound.insert(variable.clone());
                names(body, bound, free);
                if fresh {
                    bound.remove(variable);
                }
            }
        }
    }
    let mut free = BTreeSet::new();
    names(&formula, &mut BTreeSet::new(), &mut free);
    let mut parameters = Vec::new();
    for name in free {
        let Some(sort) = rendered_parameter_sort(&name) else {
            return Err(format!(
                "guarded symbol `{id}`: `#{name}` is not a registered term of the typed rendering"
            ));
        };
        parameters.push(Parameter { name, sort });
    }
    let definition = Definition {
        id: id.to_owned(),
        parameters,
        formula,
    };
    crate::quant_proposition::validate(&definition)
        .map_err(|e| format!("guarded symbol `{id}`: {e}"))?;
    check_rendering(&definition).map_err(|e| format!("guarded symbol `{id}`: {e}"))?;
    Ok(definition)
}

/// Card SP6-GUARDED-SYMBOL-PROSE: the proposition of guarded symbol `id`,
/// read from its formula's tokens: a C04.18.9 typed-rendering formula (G01p)
/// whose free names are registered terms (or SP6/SP7/SP9 terms), typed in their
/// rendered sorts. Returns the canonical printing (the binding's
/// proposition) and the definition it reads as. A string token is refused
/// by name ([`PROSE_GUARDED`]); anything else that is not such a formula
/// rejects.
pub fn guarded_formula(
    id: &str,
    tokens: &[TokenKind],
) -> Result<(String, qcore_schemas::quant_proposition::Definition), String> {
    if tokens
        .iter()
        .any(|t| matches!(t, TokenKind::StringLiteral(_)))
    {
        return Err(prose_guarded("resource_guarded_symbol_v1", id));
    }
    let formula = crate::quant_proposition::parse_formula_tokens(tokens)
        .map_err(|e| format!("guarded symbol `{id}`: {e}"))?;
    let canonical = crate::quant_proposition::print(&formula);
    if canonical.len() > 4096 {
        return Err(format!("guarded symbol `{id}`: proposition byte limit"));
    }
    let definition = guarded_definition(id, formula)?;
    Ok((canonical, definition))
}

/// Card SP6-GUARDED-SYMBOL-PROSE: a guarded binding's proposition text (a
/// standalone binding, C04.18.5, or one an inline declaration produced) must
/// be the canonical printing of a guarded formula ([`guarded_formula`]);
/// English, a string or a noncanonical spelling is refused with
/// [`PROSE_GUARDED`].
pub fn guarded_proposition(
    id: &str,
    text: &str,
) -> Result<qcore_schemas::quant_proposition::Definition, String> {
    let refuse = || {
        format!(
            "{PROSE_GUARDED}: guarded symbol `{id}`'s proposition is not the canonical printing \
             of a C04.18.9 typed-rendering formula (contract specification C04.18.6)"
        )
    };
    if text.len() > 4096 {
        return Err(refuse());
    }
    let tokens = crate::lexer::lex(text).map_err(|_| refuse())?;
    let kinds = tokens.into_iter().map(|t| t.kind).collect::<Vec<_>>();
    let (canonical, definition) = guarded_formula(id, &kinds).map_err(|e| {
        if e.starts_with(PROSE_GUARDED) {
            e
        } else {
            format!("{}: {e}", refuse())
        }
    })?;
    if canonical != text {
        return Err(refuse());
    }
    Ok(definition)
}

/// Card SP6-GUARDED-SYMBOL-PROSE: the guarded clause decision vectors shared
/// with `proof/QCore/PremiseFormula/CjsonGuardedSymbols.lean`
/// (`guardedVectors`): a guarded source clause and whether
/// `guarded_decomposition::parse_source_clause` admits it. A clause holding
/// a string is the Lean `.str` (refused, [`PROSE_GUARDED`]); a formula is the
/// Lean `.formula` (admitted exactly when it is a typed rendering).
pub const GUARDED_DECISION_VECTORS: &[(&str, bool)] = &[
    (
        "resource_guarded_symbol_v1(environment, environment, #source_corresponds())",
        true,
    ),
    (
        "resource_guarded_symbol_v1(membership, prerequisite, ((!#parent_null() && !#item_null()) && (#live(#parent) && #owned(#parent,#item))), detach_chain)",
        true,
    ),
    // The retired string forms, whatever the string says.
    (
        "resource_guarded_symbol_v1(detach, operation, \"Admission of the selected call\", detach_chain)",
        false,
    ),
    // A quoted formula is a string.
    (
        "resource_guarded_symbol_v1(environment, environment, \"#source_corresponds()\")",
        false,
    ),
    (
        "resource_guarded_decomposition_v1(g, \"guarded_decomposition_v1;\")",
        false,
    ),
    // A name that is not a registered term.
    (
        "resource_guarded_symbol_v1(environment, environment, #environment_holds())",
        false,
    ),
    // A registered term in the wrong sort.
    (
        "resource_guarded_symbol_v1(environment, environment, #item_null(#item))",
        false,
    ),
];
