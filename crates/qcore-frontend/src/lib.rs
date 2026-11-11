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

//! QCore language front end (layer L1 of `docs/GENERALIZATION_PLAN.md`
//! §3.1): lexer, axis/dimension parser and static environment, quant-index
//! parsing/resolution, source elaboration and whole-program semantic integration.
//!
//! Moved from `qcore-compiler` at the W1 relocation; the compiler crate
//! re-exports every module here at its previous path.

pub mod check;
pub mod contract_inclusion;
pub mod contract_lifecycle;
pub mod frontend;
pub mod lexer;
pub mod module_imports;
pub mod parser;
pub mod pipe;
pub mod quant_index;
// Card QR-LEXICAL-SELECTORS-DECLARATION-NAMES: the token-stream selectors' shared reading.
pub mod route_selection;
pub mod source_annotation;
pub mod source_bitfield_concurrency;
pub mod source_composed;
pub mod source_composed_tasks;
pub mod source_epoch;
pub mod source_epoch_tasks;
pub mod source_event;
pub mod source_extension;
pub mod source_extern;
pub mod source_ffi;
pub mod source_http;
pub mod source_loop;
pub mod source_mutex;
pub mod source_mutex_footprint;
pub mod source_record;
pub mod source_region;
pub mod source_resource_calls;
pub mod source_socket;
pub mod source_std_buffer;
pub mod source_std_io;
pub mod source_std_string;
pub mod source_tasks;
// Card W48-QR-SCALAR-TYPES-R4: the value statement's reader, shared by the
// extern and task routes.
pub mod source_value_statement;

pub use parser::parse_program;

// Internal aliases preserve the relocated semantic integration's imports without
// introducing a dependency on compiler orchestration or external verification.
#[cfg(test)]
use qcore_core::Effect;
pub(crate) use qcore_core::{
    ResourceId, cfg, cleanup_lowering, error_region, ownership, resource_flow, typed_errors, usage,
};

pub mod whole_program;
pub use whole_program::{
    CallSite, CheckedFunction, CheckedProgram, FunctionSemantics,
    FunctionSummary as WholeFunctionSummary, ParameterOwnership, check_whole_program,
    check_whole_program_with_externs,
};

pub mod source_scalar_calls;

pub mod premise_flow;

pub mod guarded_decomposition;

pub mod quant_proposition;

pub mod quant_effect;

pub mod quant_theorem;

pub mod quant_condition;
