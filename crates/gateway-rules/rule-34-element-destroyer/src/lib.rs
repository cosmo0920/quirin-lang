//! Gateway rule 34 (`docs/agents/rules/34.md`; card
//! W48-SCHED-CLEANUP-PRODUCTION, moved into its own rule package by card
//! RULE34-PILOT, the fifth pilot of GATEWAY-RULE-API-PILOT-MIGRATIONS).
//!
//! One arm, registered in `crates/gateway-rules/registry.toml` under rule id
//! `34`: [`destroyer_order::ElementDestroyer`], the transfer arm
//! `destroyer-order` (route map §10). A declaration may state
//! `resource_unpublished(p)` beside `resource_released(p, …)`, the element
//! destroyer; the arm orders that unlink **before** the call's own reading, so
//! the declared release ends exactly the custody the unlink took. Every other
//! container effect keeps the host's order (after the call).
//!
//! The package reads only the rule API's transfer query
//! ([`qcore_gateway_rule_api::TransferQuery`]) and decides nothing else: which
//! entries an unlink admits (rules 10d and 10f) stays the host's. Its switch
//! is the host's: with `34` in `QCORE_GATEWAY_RULES_OFF` the arm is not asked,
//! and the host refuses a call whose unlink and release no transfer rule
//! orders (its guard, which is not switchable: the host's own order, release
//! first, is what rule 34 forbids). The Lean models are
//! `QCore.ElementDestroyer` (the caller's reading) and
//! `QCore.ElementDestroyerSwitch` (this arm's phases, which are this crate's
//! `DECISION_VECTORS`, and the switch).

pub mod destroyer_order;

pub use destroyer_order::ElementDestroyer;

/// The package's registry id.
pub const ID: &str = "34";
