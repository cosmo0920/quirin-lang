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

//! Card `W48-QR-TASK-RECORD-CAPTURES` (card 3 of the `.qr` language track):
//! task workers receive and return records.
//!
//! * **Record parameters, results and captures.** A record crosses a spawn as
//!   its leaves: a record of scalars by `copy` (duplicated), a record that owns
//!   a Buffer by `move` (custody of every owned leaf transferred, its scalars
//!   duplicated), decided by the transferability judgment's aggregate kind
//!   from the leaves, and a record comes back through `join` whole.
//! * **Field captures**: `copy r.f`, `move r.buf` (a hole refilled or left
//!   unused), `read r.buf share N` and `lend r.buf LOW..HIGH`, each the
//!   existing capture of one leaf, with the loan and lease rules per leaf.
//! * **Record operations inside workers**, and **§G02j rendering** of fields.
//!
//! Negative controls first, then admissions, then the published examples'
//! native runs on real threads.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::Command;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};

use qcore_compiler::{CompileOptions, CompiledModule, FileDiagnostic, SourceFile, compile_files};
use qcore_core::task_execution::{
    ScalarOperand, ScalarType, TaskExecutionScript, TaskInstructionKind, TaskTerminatorKind,
    execute_with_source_lines,
};

#[cfg(target_os = "linux")]
static NEXT: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn compile(source: &str) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("task_record_captures.qr", source)],
        &CompileOptions::default(),
    )
}

fn admitted(source: &str) -> CompiledModule {
    compile(source).unwrap_or_else(|errors| panic!("expected admission, got {errors:#?}"))
}

fn codes(result: Result<CompiledModule, Vec<FileDiagnostic>>) -> Vec<(String, String)> {
    match result {
        Ok(module) => panic!(
            "expected a refusal, admitted with profile {:?}",
            module.execution_profile
        ),
        Err(errors) => errors
            .into_iter()
            .map(|error| (error.code, error.message))
            .collect(),
    }
}

fn refused_with(source: &str, code: &str, needles: &[&str]) {
    let found = codes(compile(source));
    assert!(
        found.iter().any(|(found, message)| found == code
            && needles.iter().all(|needle| message.contains(needle))),
        "expected {code} mentioning {needles:?}, got {found:#?}"
    );
}

const HEAD: &str = "#import std.concurrency;\nconcurrency checked;\n";
const IO_HEAD: &str =
    "#import std.concurrency;\n#import std.fmt;\n#import std.io;\nconcurrency checked;\n";
const PAIR: &str = "type Pair { a: i32, buf: own<Buffer<u8>> }\n";
const OWNER_ROW: &str = "effects { alloc, throws<AllocError>, spawn, join, may_block }";

/// A parent that builds `let p = Pair { a: 1, buf: b };` and runs `spawn`
/// under a `match` whose arms are `ok` and `err`.
fn pair_program(workers: &str, spawn: &str, ok: &str, err: &str) -> String {
    format!(
        "{HEAD}{PAIR}{workers}\nfn main() -> i32 {OWNER_ROW} {{\n    let seed: u8 = 7;\n    let b = \
         qcore_buffer_new(seed)?;\n    let p = Pair {{ a: 1, buf: b }};\n    thread scope s {{\n        \
         match spawn s {spawn} {{\n            Result::Ok {{ value: h }} => {{\n{ok}\n            }}\n            \
         Result::Err {{ error: failure }} => {{\n{err}\n            }}\n        }}\n    }}\n}}\n"
    )
}

const KEEP: &str = "fn keep(p: Pair use one) -> i32 {\n    drop p.buf;\n    return 0;\n}\n";
const TAKE: &str = "fn take(b: own<Buffer<u8>> use one) -> i32 {\n    drop b;\n    return 0;\n}\n";

// ---------------------------------------------------------------------------
// Negative controls (the brief's list, in its order).
// ---------------------------------------------------------------------------

/// A record with an owned field captured by `copy`: the aggregate judgment
/// asks `copy` of every leaf, and the owned leaf refuses — named, with the
/// contract it lacks.
#[test]
fn a_record_that_owns_captured_by_copy_names_the_owned_leaf() {
    let source = pair_program(
        KEEP,
        "keep(copy p)",
        "                let r = join move h?;\n                drop p.buf;\n                return r;",
        "                drop p.buf;\n                return 0;",
    );
    refused_with(
        &source,
        "Q600",
        &[
            "transferability judgment",
            "the leaf `p.buf`",
            "`copy`",
            "`move` (custody of every owned field",
        ],
    );
}

/// A field moved to a worker leaves a hole: using it before a refill is the
/// core's use of a slot that holds no Buffer, on the path on which the move
/// happened.
#[test]
fn a_moved_field_used_before_refill_is_refused() {
    let source = pair_program(
        TAKE,
        "take(move p.buf)",
        "                let r = join move h?;\n                let v = qcore_buffer_read(p.buf);\n                return r;",
        "                drop p.buf;\n                return 0;",
    );
    refused_with(&source, "Q600", &["is not an initialized owned buffer"]);
    // The same program refilling the hole first is admitted: the hole was the
    // refusal, not the field.
    let refilled = format!(
        "{HEAD}{PAIR}{TAKE}\nfn main() -> i32 effects {{ alloc, throws<AllocError>, spawn, join, \
         may_block, mut }} {{\n    let seed: u8 = 7;\n    let b = qcore_buffer_new(seed)?;\n    let p = \
         Pair {{ a: 1, buf: b }};\n    thread scope s {{\n        match spawn s take(move p.buf) {{\n            \
         Result::Ok {{ value: h }} => {{\n                let r = join move h?;\n                let c = \
         qcore_buffer_new(seed)?;\n                p.buf = c;\n                let v = \
         qcore_buffer_read(p.buf);\n                drop p.buf;\n                let n = v as \
         i32;\n                return n;\n            }}\n            Result::Err {{ error: failure }} => {{\n                drop p.buf;\n                return 0;\n            \
         }}\n        }}\n    }}\n}}\n"
    );
    admitted(&refilled);
}

const WIDE: &str = "type Wide { tag: i32, buf: own<Buffer> }\n";
const FILL: &str = "fn fill(l: lend<Buffer>) -> unit {\n    let u = qcore_buffer_write(l, 0, 9);\n    return;\n}\n";
const READER: &str =
    "fn reader(r: borrow<Buffer>) -> i32 {\n    let v = qcore_buffer_read(r);\n    return v;\n}\n";

/// A parent holding `let w = Wide { tag: 1, buf: b };` that opens two
/// creations, the second inside the first's `Ok` arm.
fn wide_program(workers: &str, first: &str, second: &str) -> String {
    format!(
        "{HEAD}{WIDE}{workers}\nfn main() -> i32 {OWNER_ROW} {{\n    let b = qcore_buffer_new(1)?;\n    \
         let w = Wide {{ tag: 1, buf: b }};\n    thread scope s {{\n        match spawn s {first} {{\n            \
         Result::Ok {{ value: h1 }} => {{\n                match spawn s {second} {{\n                    \
         Result::Ok {{ value: h2 }} => {{\n                        join move h1;\n                        \
         join move h2;\n                        drop w.buf;\n                        return 0;\n                    \
         }}\n                    Result::Err {{ error: e }} => {{\n                        join move h1;\n                        \
         drop w.buf;\n                        return 0;\n                    }}\n                }}\n            }}\n            \
         Result::Err {{ error: failure }} => {{\n                drop w.buf;\n                return 0;\n            \
         }}\n        }}\n    }}\n}}\n"
    )
}

/// Two lends of one field whose ranges share a byte: the ledger's overlap
/// decision over that leaf, unchanged.
#[test]
fn two_overlapping_lends_of_one_field_are_refused() {
    let source = wide_program(FILL, "fill(lend w.buf 0..2)", "fill(lend w.buf 1..3)");
    refused_with(&source, "Q600", &["OverlappingLeases"]);
    // Disjoint ranges of the same field are the protocol, and admitted.
    admitted(&wide_program(
        FILL,
        "fill(lend w.buf 0..2)",
        "fill(lend w.buf 2..4)",
    ));
}

/// A read share and a write lease of one leaf together: refused — the lease
/// finds the leaf lent for reading.
#[test]
fn a_read_share_and_a_write_lease_of_one_leaf_are_refused() {
    let workers = format!("{READER}{FILL}");
    let source = wide_program(
        &workers,
        "reader(read w.buf share 512)",
        "fill(lend w.buf 0..2)",
    );
    let found = codes(compile(
        &source.replace("join move h1;", "let r1 = join move h1?;"),
    ));
    assert!(
        found.iter().any(|(code, message)| code == "Q600"
            && (message.contains("does not mix outstanding read loans and write leases")
                || message.contains("LoanOutstanding"))),
        "{found:#?}"
    );
}

/// A record captured into a parameter of another record type: the judgment's
/// delivered-kind refusal — record types are nominal, even with equal leaves.
#[test]
fn a_record_into_another_record_types_parameter_is_refused() {
    let source = format!(
        "{HEAD}type P {{ x: i32 }}\ntype Q {{ x: i32 }}\nfn w(q: Q) -> i32 {{\n    let v = q.x;\n    \
         return v;\n}}\nfn main() -> i32 effects {{ throws<SpawnError>, spawn, join, may_block }} {{\n    \
         let p = P {{ x: 1 }};\n    thread scope s {{\n        let h = spawn s w(copy p)?;\n        let r = \
         join move h?;\n        return r;\n    }}\n}}\n"
    );
    refused_with(
        &source,
        "Q600",
        &[
            "delivers a `P`",
            "receives a `Q`",
            "record types are nominal",
        ],
    );
}

/// A record moved to a worker while one of its owned leaves is lent to
/// another: the move of that leaf is refused by the ledger, naming the loan.
#[test]
fn a_record_moved_while_a_leaf_is_lent_is_refused() {
    let source = format!(
        "{HEAD}{WIDE}{READER}fn keep(p: Wide use one) -> i32 {{\n    drop p.buf;\n    return 0;\n}}\n\
         fn main() -> i32 {OWNER_ROW} {{\n    let b = qcore_buffer_new(1)?;\n    let w = Wide {{ tag: 1, \
         buf: b }};\n    thread scope s {{\n        match spawn s reader(read w.buf share 1024) {{\n            \
         Result::Ok {{ value: h1 }} => {{\n                match spawn s keep(move w) {{\n                    \
         Result::Ok {{ value: h2 }} => {{\n                        let r1 = join move h1?;\n                        \
         let r2 = join move h2?;\n                        return r1;\n                    }}\n                    \
         Result::Err {{ error: e }} => {{\n                        let r1 = join move h1?;\n                        \
         drop w.buf;\n                        return r1;\n                    }}\n                }}\n            \
         }}\n            Result::Err {{ error: failure }} => {{\n                drop w.buf;\n                \
         return 0;\n            }}\n        }}\n    }}\n}}\n"
    );
    refused_with(&source, "Q600", &["LoanOutstanding"]);
}

/// Spawn failure with a moved record restores it: on the failure arm the
/// record is live again — its scalar field readable, its Buffer owned and
/// owed a release — and a failure arm that forgets the Buffer leaks it.
#[test]
fn spawn_failure_restores_a_moved_record() {
    let restored = pair_program(
        KEEP,
        "keep(move p)",
        "                let r = join move h?;\n                return r;",
        "                let a = p.a;\n                drop p.buf;\n                return a;",
    );
    admitted(&restored);
    let forgotten = pair_program(
        KEEP,
        "keep(move p)",
        "                let r = join move h?;\n                return r;",
        "                let a = p.a;\n                return a;",
    );
    refused_with(&forgotten, "Q600", &["owned buffer"]);
    // And on the arm on which the creation succeeded, the record is moved.
    let used = pair_program(
        KEEP,
        "keep(move p)",
        "                let r = join move h?;\n                let a = p.a;\n                return a;",
        "                drop p.buf;\n                return 0;",
    );
    refused_with(&used, "Q600", &["record `p` was moved whole"]);
}

/// A `write r` capture is still refused by name, before any leaf is asked.
#[test]
fn a_write_capture_of_a_record_is_refused_by_name() {
    let source = pair_program(
        KEEP,
        "keep(write p)",
        "                let r = join move h?;\n                return r;",
        "                drop p.buf;\n                return 0;",
    );
    refused_with(
        &source,
        "Q600",
        &["a `write` capture names no transfer or share contract"],
    );
}

// ---------------------------------------------------------------------------
// The other boundaries this card draws.
// ---------------------------------------------------------------------------

/// `move` of a record of scalars is refused for its mode — it crosses by copy
/// — and a sharing mode has no whole-record protocol.
#[test]
fn a_copy_record_does_not_move_and_no_record_is_shared_whole() {
    let copy_record = |capture: &str| {
        format!(
            "{HEAD}type P {{ x: i32, y: u8 }}\nfn w(p: P) -> i32 {{\n    let v = p.x;\n    return v;\n}}\n\
             fn main() -> i32 effects {{ throws<SpawnError>, spawn, join, may_block }} {{\n    let \
             p = P {{ x: 1, y: 2 }};\n    thread scope s {{\n        let h = spawn s w({capture})?;\n        \
             let r = join move h?;\n        return r;\n    }}\n}}\n"
        )
    };
    admitted(&copy_record("copy p"));
    refused_with(
        &copy_record("move p"),
        "Q600",
        &["owns no Buffer", "crosses by duplication"],
    );
    refused_with(
        &copy_record("read p share 512"),
        "Q600",
        &["a record shares none whole"],
    );
}

/// A field write needs `mut`; a refill over a live field is refused; a record
/// that owns returns by `move`; a `bytes` field stays the buffer route's.
#[test]
fn the_field_and_result_boundaries_refuse() {
    let write = |row: &str| {
        format!(
            "{HEAD}type P {{ x: i32 }}\nfn main() -> i32 effects {{ {row} }} {{\n    let p = P {{ x: 1 \
             }};\n    p.x = 2;\n    let v = p.x;\n    thread scope s {{\n        return v;\n    }}\n}}\n"
        )
    };
    refused_with(&write("spawn"), "Q300", &["derives `mut`"]);
    admitted(&write("mut"));
    let refill_live = format!(
        "{HEAD}{PAIR}fn main() -> i32 effects {{ alloc, throws<AllocError>, mut }} {{\n    let seed: u8 \
         = 7;\n    let b = qcore_buffer_new(seed)?;\n    let c = qcore_buffer_new(seed)?;\n    let p = Pair \
         {{ a: 1, buf: b }};\n    p.buf = c;\n    drop p.buf;\n    thread scope s {{\n        return 0;\n    \
         }}\n}}\n"
    );
    refused_with(&refill_live, "Q600", &["OverwriteOfLiveField"]);
    let copied_return = format!(
        "{HEAD}{PAIR}fn w(p: Pair use one) -> Pair {{\n    return p;\n}}\nfn main() -> i32 effects {{ \
         spawn }} {{\n    thread scope s {{\n        return 0;\n    }}\n}}\n"
    );
    refused_with(&copied_return, "Q600", &["return move p;"]);
    let bytes = format!(
        "{HEAD}type F {{ tag: i32, body: bytes }}\nfn main() -> i32 effects {{ spawn }} {{\n    thread \
         scope s {{\n        return 0;\n    }}\n}}\n"
    );
    refused_with(&bytes, "Q500", &["WRITE-LEND-TASKS-EXTENT"]);
}

/// A record held by another record moved in whole, and a record left in
/// different states by two continuing arms, are used no more.
#[test]
fn a_moved_or_unsettled_record_is_used_no_more() {
    let moved_in = format!(
        "{HEAD}{PAIR}type Outer {{ n: i32, inner: Pair }}\nfn main() -> i32 {OWNER_ROW} {{\n    let \
         seed: u8 = 7;\n    let b = qcore_buffer_new(seed)?;\n    let p = Pair {{ a: 1, buf: b }};\n    let \
         o = Outer {{ n: 2, inner: p }};\n    let a = p.a;\n    drop o.inner.buf;\n    thread scope s {{\n        \
         return a;\n    }}\n}}\n"
    );
    refused_with(&moved_in, "Q600", &["record `p` was moved whole"]);
    let unsettled = format!(
        "{HEAD}{PAIR}{KEEP}\nfn main() -> i32 {OWNER_ROW} {{\n    let seed: u8 = 7;\n    let b = \
         qcore_buffer_new(seed)?;\n    let p = Pair {{ a: 1, buf: b }};\n    thread scope s {{\n        match \
         spawn s keep(move p) {{\n            Result::Ok {{ value: h }} => {{\n                let r = join \
         move h?;\n            }}\n            Result::Err {{ error: failure }} => {{\n                let z = \
         0;\n            }}\n        }}\n        let a = p.a;\n        return a;\n    }}\n}}\n"
    );
    refused_with(&unsettled, "Q600", &["W48-QR-TASK-RECORD-ARM-MERGE"]);
}

/// A sub-record of scalars crosses by `copy` into a parameter of its own type;
/// a sub-record that owns crosses field by field and is refused whole.
#[test]
fn a_sub_record_copies_and_an_owning_one_crosses_field_by_field() {
    let program = |worker: &str, capture: &str| {
        format!(
            "{HEAD}type Tag {{ id: u16 }}\n{PAIR}type Job {{ tag: Tag, part: Pair }}\n{worker}\n\
             fn main() -> i32 {OWNER_ROW} {{\n    let seed: u8 = 7;\n    let b = \
             qcore_buffer_new(seed)?;\n    let id: u16 = 3;\n    let t = Tag {{ id: id }};\n    let p = \
             Pair {{ a: 1, buf: b }};\n    let j = Job {{ tag: t, part: p }};\n    thread scope s {{\n        \
             match spawn s w({capture}) {{\n            Result::Ok {{ value: h }} => {{\n                let \
             r = join move h?;\n                drop j;\n                return r;\n            }}\n            \
             Result::Err {{ error: failure }} => {{\n                drop j;\n                return 0;\n            \
             }}\n        }}\n    }}\n}}\n"
        )
    };
    admitted(&program(
        "fn w(t: Tag) -> i32 {\n    let v = t.id;\n    let n = v as i32;\n    return n;\n}",
        "copy j.tag",
    ));
    refused_with(
        &program(
            "fn w(p: Pair use one) -> i32 {\n    drop p.buf;\n    return 0;\n}",
            "move j.part",
        ),
        "Q600",
        &["W48-QR-NESTED-OWNED-SUBRECORD-MOVE"],
    );
}

// ---------------------------------------------------------------------------
// Admissions, and what the core is handed.
// ---------------------------------------------------------------------------

fn functions(module: &CompiledModule) -> &qcore_core::task_execution::TaskModule {
    match module.checked.as_composed_tasks() {
        Some(composed) => composed.tasks().module(),
        None => match &module.checked {
            qcore_compiler::CheckedSourceModule::ScopedTasks(checked) => checked.module(),
            other => panic!("not a task module: {other:?}"),
        },
    }
}

/// A record parameter is its leaves, and a record capture is its leaves'
/// captures: a copy of each scalar and a move of each owned Buffer.
#[test]
fn a_record_capture_is_its_leaves_captures() {
    let source = pair_program(
        "fn keep(p: Pair use one) -> Pair {\n    return move p;\n}\n",
        "keep(move p)",
        "                let q = join move h?;\n                let a = q.a;\n                drop q.buf;\n                return a;",
        "                drop p.buf;\n                return 0;",
    );
    let module = admitted(&source);
    let tasks = functions(&module);
    let keep = tasks
        .functions
        .values()
        .find(|function| function.name == "keep")
        .unwrap();
    assert_eq!(
        keep.parameters
            .iter()
            .map(|(_, kind)| *kind)
            .collect::<Vec<_>>(),
        vec![
            ScalarType::I32,
            ScalarType::OwnedBuffer(qcore_core::scalar_types::ScalarType::U8)
        ]
    );
    assert!(matches!(keep.result, ScalarType::Record(_)));
    let main = &tasks.functions[&tasks.entry];
    let spawn = main
        .blocks
        .values()
        .find_map(|block| match &block.terminator.kind {
            TaskTerminatorKind::TrySpawn { arguments, .. } => Some(arguments.clone()),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        spawn.as_slice(),
        [ScalarOperand::Slot(_), ScalarOperand::Move(_)]
    ));
    // The collected record is unpacked into two leaf slots.
    assert!(
        main.blocks.values().any(
            |block| block.instructions.iter().any(|instruction| matches!(
                &instruction.kind,
                TaskInstructionKind::UnpackRecord { leaves, .. } if leaves.len() == 2
            ))
        )
    );
}

/// Field captures are the existing captures of one leaf: a copy of a scalar
/// field, a read share and a write lease of an owned one; and two fields
/// leased to two workers at once — two leaves, two resources — are admitted.
#[test]
fn field_captures_are_the_existing_captures_of_one_leaf() {
    let two = format!(
        "{HEAD}type Two {{ tag: i32, left: own<Buffer>, right: own<Buffer> }}\n{FILL}fn show(t: i32) -> \
         i32 {{\n    return t;\n}}\nfn main() -> i32 {OWNER_ROW} {{\n    let l = qcore_buffer_new(1)?;\n    \
         match qcore_buffer_new(2) {{\n    Result::Err {{ error: shortfall }} => {{\n        drop l;\n        \
         return 1;\n    }}\n    Result::Ok {{ value: r }} => {{\n    let t = Two {{ tag: 5, left: l, right: r \
         }};\n    thread scope s {{\n        match spawn s fill(lend t.left 0..4) {{\n            Result::Ok {{ value: h1 }} => {{\n                \
         match spawn s fill(lend t.right 0..4) {{\n                    Result::Ok {{ value: h2 }} => {{\n                        \
         join move h1;\n                        join move h2;\n                        let a = \
         qcore_buffer_read(t.left);\n                        let b = qcore_buffer_read(t.right);\n                        \
         drop t;\n                        return a;\n                    }}\n                    Result::Err {{ error: e \
         }} => {{\n                        join move h1;\n                        drop t;\n                        return \
         0;\n                    }}\n                }}\n            }}\n            Result::Err {{ error: failure }} => \
         {{\n                drop t;\n                return 0;\n            }}\n        }}\n    }}\n    }}\n    \
         }}\n}}\n"
    );
    admitted(&two);
    let reader = format!(
        "{HEAD}{WIDE}{READER}fn show(t: i32) -> i32 {{\n    return t;\n}}\nfn main() -> i32 {OWNER_ROW} \
         {{\n    let b = qcore_buffer_new(3)?;\n    let w = Wide {{ tag: 4, buf: b }};\n    thread scope s {{\n        \
         match spawn s reader(read w.buf share 512) {{\n            Result::Ok {{ value: h1 }} => {{\n                \
         match spawn s show(copy w.tag) {{\n                    Result::Ok {{ value: h2 }} => {{\n                        \
         let x = join move h1?;\n                        let y = join move h2?;\n                        drop \
         w.buf;\n                        return x;\n                    }}\n                    Result::Err {{ error: e \
         }} => {{\n                        let x = join move h1?;\n                        drop w.buf;\n                        \
         return x;\n                    }}\n                }}\n            }}\n            Result::Err {{ error: failure }} \
         => {{\n                drop w.buf;\n                return 0;\n            }}\n        }}\n    }}\n}}\n"
    );
    admitted(&reader);
}

/// Record operations inside a worker: construct, read and write a record of
/// its own, and return it whole; a worker's `ArithmeticError` still reaches
/// its creator through `join`.
#[test]
fn a_worker_constructs_reads_and_writes_its_own_records() {
    let source = format!(
        "{HEAD}type Acc {{ total: i64, count: u8 }}\nfn tally(x: i64) -> Acc effects {{ mut, \
         throws<ArithmeticError> }} {{\n    let one: u8 = 1;\n    let a = Acc {{ total: x, count: one }};\n    \
         let t = a.total;\n    let doubled = t + t;\n    a.total = doubled;\n    return a;\n}}\nfn main() -> i32 \
         effects {{ throws<SpawnError>, throws<ArithmeticError>, spawn, join, may_block }} {{\n    let x: i64 = \
         20;\n    thread scope s {{\n        let h = spawn s tally(copy x)?;\n        let a = join move h?;\n        \
         let c = a.count;\n        let n = c as i32;\n        return n;\n    }}\n}}\n"
    );
    let module = admitted(&source);
    let composed = qcore_core::task_execution::execute_task_module(
        match &module.checked {
            qcore_compiler::CheckedSourceModule::ScopedTasks(checked) => checked,
            other => panic!("not the task route: {other:?}"),
        },
        &TaskExecutionScript::default(),
    )
    .unwrap();
    let rendered = qcore_core::task_execution::render_task_observation(&composed);
    assert!(
        rendered.contains(
            "QCORE_TASK_RESULT 1 RECORD 2\nQCORE_TASK_RESULT 1 I64 40\nQCORE_TASK_RESULT 1 U8 1\n"
        ),
        "{rendered}"
    );
}

/// §G02j: a scalar field renders in the parent and in a worker, at its type.
#[test]
fn a_record_field_renders_in_the_parent_and_in_a_worker() {
    let source = format!(
        "{IO_HEAD}type P {{ x: i32, flag: bool }}\nfn w(p: P) -> i32 effects {{ io }} {{\n    print dec \
         p.flag;\n    let v = p.x;\n    return v;\n}}\nfn main() -> i32 effects {{ throws<SpawnError>, spawn, \
         join, may_block, io }} {{\n    let p = P {{ x: 7, flag: true }};\n    print dec p.x;\n    thread scope \
         s {{\n        let h = spawn s w(copy p)?;\n        let r = join move h?;\n        print dec r;\n        \
         return 0;\n    }}\n}}\n"
    );
    let module = admitted(&source);
    let composed = module.checked.as_composed_tasks().unwrap();
    let observation = execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap();
    let (out, _) = observation.projections();
    let lines: String = out
        .lines()
        .filter(|line| !line.starts_with("QCORE_"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_eq!(lines, "7\ntrue\n7\n", "{out}");
    let hex_bool = source.replace("print dec p.flag", "print hex p.flag");
    refused_with(&hex_bool, "Q050", &["no `hex` rendering"]);
    let whole = source.replace("print dec p.x;", "print dec p;");
    refused_with(&whole, "Q050", &["is not an `i32`"]);
}

// ---------------------------------------------------------------------------
// The published examples, natively on real threads.
// ---------------------------------------------------------------------------

const EXAMPLES: [(&str, &str); 3] = [
    (
        "record_workers",
        include_str!("../../examples/threads/record_workers.qr"),
    ),
    (
        "record_field_leases",
        include_str!("../../examples/threads/record_field_leases.qr"),
    ),
    (
        "record_spawn_failure",
        include_str!("../../examples/threads/record_spawn_failure.qr"),
    ),
];

fn example(name: &str, source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new(
            format!("examples/threads/{name}.qr"),
            source,
        )],
        &CompileOptions::default(),
    )
    .unwrap_or_else(|errors| panic!("{name} does not check: {errors:#?}"))
}

/// The reference projections under a creation script.
fn reference(module: &CompiledModule, spawn_outcomes: Vec<bool>) -> (String, String) {
    let composed = module.checked.as_composed_tasks().unwrap();
    execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript {
            spawn_outcomes,
            allocation_outcomes: Vec::new(),
        },
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap()
    .projections()
}

/// The three builds every native row of the track uses.
#[cfg(target_os = "linux")]
const BUILDS: [(&str, &[&str]); 3] = [
    (
        "O0-ubsan",
        &[
            "-O0",
            "-g",
            "-fsanitize=undefined,integer",
            "-fno-sanitize-recover=all",
        ],
    ),
    (
        "O2-ubsan",
        &[
            "-O2",
            "-g",
            "-fsanitize=undefined,integer",
            "-fno-sanitize-recover=all",
        ],
    ),
    ("O2", &["-O2"]),
];

#[cfg(target_os = "linux")]
struct Run {
    stdout: String,
    stderr: String,
    status: Option<i32>,
}

/// Build the emitted C with `-std=c11 -Wall -Wextra -Werror -pedantic` and the
/// build's flags (and `defines`, which the spawn-failure row uses to refuse a
/// creation through the runtime's own `QCORE_TASK_CREATE` seam), and run it.
#[cfg(target_os = "linux")]
fn build_and_run(c: &str, flags: &[&str], prelude: Option<&str>) -> Result<Run, String> {
    let dir = std::env::temp_dir().join(format!(
        "qcore-task-records-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let source = match prelude {
        Some(prelude) => format!("{prelude}{c}"),
        None => c.to_owned(),
    };
    std::fs::write(dir.join("main.c"), source).unwrap();
    let out = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()))
        .args([
            "-std=c11",
            "-pthread",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-pedantic",
        ])
        .args(flags)
        .arg(dir.join("main.c"))
        .arg("-o")
        .arg(dir.join("main"))
        .output()
        .expect("Clang is required for this native check");
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let run = Command::new(dir.join("main"))
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Ok(Run {
        stdout: String::from_utf8(run.stdout).unwrap(),
        stderr: String::from_utf8(run.stderr).unwrap(),
        status: run.status.code(),
    })
}

/// A prelude that refuses every task creation through the runtime's own
/// `QCORE_TASK_CREATE` seam, so the program takes its spawn-failure arm.
#[cfg(target_os = "linux")]
const REFUSE_CREATION: &str = "#include <threads.h>\nstatic int qcore_refuse_create(thrd_t *thread, \
     int (*function)(void *), void *argument) {\n    (void)thread; (void)function; (void)argument;\n    \
     return thrd_error;\n}\n#define QCORE_TASK_CREATE qcore_refuse_create\n";

/// Each example, three builds: stdout and stderr equal the reference, exit 0,
/// no sanitizer report; the spawn-failure example also with every creation
/// refused, against the reference run whose creation fails. The summary goes
/// to the report's `native-task-records.json` when
/// `QCORE_W48_TASK_RECORDS_NATIVE_SUMMARY` names a file.
#[cfg(target_os = "linux")]
#[test]
fn the_published_examples_run_clean_under_three_builds() {
    let mut summary = Vec::new();
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let c = module.c_source.as_deref().unwrap();
        let mut cases = vec![("created", reference(&module, Vec::new()), None)];
        if name == "record_spawn_failure" {
            cases.push((
                "creation-refused",
                reference(&module, vec![false]),
                Some(REFUSE_CREATION),
            ));
        }
        for (arm, (stdout, stderr), prelude) in cases {
            for (label, flags) in BUILDS {
                let run = build_and_run(c, flags, prelude).unwrap_or_else(|error| {
                    panic!("{name} {label}: the emitted C does not build: {error}")
                });
                assert_eq!(run.status, Some(0), "{name} {arm} {label}: {}", run.stderr);
                assert_eq!(run.stdout, stdout, "{name} {arm} {label}");
                assert_eq!(
                    run.stderr, stderr,
                    "{name} {arm} {label}: a sanitizer report or a changed line"
                );
                summary.push(format!(
                    "{{\"program\": \"examples/threads/{name}.qr\", \"arm\": \"{arm}\", \"build\": \
                     \"{label}\", \"exit\": 0, \"stdout_lines\": {}, \"stderr_lines\": {}, \
                     \"sanitizer_reports\": 0}}",
                    stdout.lines().count(),
                    stderr.lines().count()
                ));
            }
        }
    }
    for row in tsan_rows() {
        summary.push(row);
    }
    if let Ok(path) = std::env::var("QCORE_W48_TASK_RECORDS_NATIVE_SUMMARY") {
        let (compiler, triple) = qcore_c_gateway::ffi_layout::running_compiler().unwrap();
        std::fs::write(
            path,
            format!(
                "{{\n \"compiler\": \"{compiler}\",\n \"triple\": \"{triple}\",\n \"flags\": \"-std=c11 \
                 -pthread -Wall -Wextra -Werror -pedantic\",\n \"runs\": [\n  {}\n ]\n}}\n",
                summary.join(",\n  ")
            ),
        )
        .unwrap();
    }
}

#[cfg(target_os = "linux")]
#[path = "../../tests/support/thread_sanitizer.rs"]
mod thread_sanitizer;

/// ThreadSanitizer over the concurrent examples, through the existing pthread
/// binding (`TASK-ROUTE-TSAN-PTHREAD-BINDING`): unforced, and forced with both
/// workers of the lease example live together. Each row is clean — no report
/// of any kind — and prints the reference stdout. Returns the summary rows.
#[cfg(target_os = "linux")]
fn tsan_rows() -> Vec<String> {
    use thread_sanitizer::{Row, Schedule, ScheduleEvent};
    let mut rows = Vec::new();
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let c = module.c_source.as_deref().unwrap();
        let (stdout, _) = reference(&module, Vec::new());
        let schedules: &[(Schedule, usize)] = if name == "record_field_leases" {
            &[
                (Schedule::Unforced, 0),
                (Schedule::ForcedTogether, 2),
                (Schedule::ForcedReverse, 2),
            ]
        } else {
            &[(Schedule::Unforced, 0)]
        };
        for (schedule, cohort) in schedules {
            let run = Row {
                tag: name,
                source: c,
                schedule: *schedule,
                cohort: *cohort,
                ..Row::default()
            }
            .execute()
            .unwrap_or_else(|error| panic!("{name} tsan: {error}"));
            assert!(
                run.clean,
                "{name} {}: ThreadSanitizer reported:\n{}",
                schedule.tag(),
                run.stderr
            );
            assert_eq!(run.races, 0);
            // A forced row means the schedule was forced: both workers of the
            // cohort were released and ran their bodies.
            if schedule.is_forced() {
                let started = run
                    .schedule
                    .iter()
                    .filter(|event| matches!(event, ScheduleEvent::Start(_)))
                    .count();
                assert_eq!(
                    started,
                    *cohort,
                    "{name} {}: {:?}",
                    schedule.tag(),
                    run.schedule
                );
            }
            assert_eq!(run.stdout, stdout, "{name} {}", schedule.tag());
            rows.push(format!(
                "{{\"program\": \"examples/threads/{name}.qr\", \"arm\": \"created\", \"build\": \
                 \"tsan-{}\", \"exit\": 0, \"stdout_lines\": {}, \"sanitizer_reports\": 0}}",
                schedule.tag(),
                stdout.lines().count()
            ));
        }
    }
    rows
}

/// §G02j natively: a worker logs three fields of the record it received (one
/// emitter on stderr, so its order is determined) and the parent prints fields
/// before the creation and after the join, beside the trace (all the parent's
/// own writes on stdout). Both streams equal the reference under all three
/// builds, and the declared reordering relation admits the stderr run.
#[cfg(target_os = "linux")]
#[test]
fn record_fields_render_natively_in_a_worker_and_the_parent() {
    let source = "#import std.concurrency;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n\
                  concurrency checked;\ntype Reading { sensor: u8, level: i16, ok: bool }\n\
                  fn report(r: Reading) -> i16 effects { io } {\n    log info dec r.sensor;\n    \
                  log info dec r.level;\n    log info dec r.ok;\n    let l = r.level;\n    return l;\n}\n\
                  fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {\n    \
                  log_level trace;\n    let s8: u8 = 3;\n    let lv: i16 = -12;\n    let r = Reading { \
                  sensor: s8, level: lv, ok: true };\n    print dec r.level;\n    thread scope s {\n        \
                  let h = spawn s report(copy r)?;\n        let back = join move h?;\n        print dec \
                  back;\n        print hex r.sensor;\n        return 0;\n    }\n}\n";
    let module = admitted(source);
    let composed = module.checked.as_composed_tasks().unwrap();
    assert!(composed.certificate().admits_reordering);
    let observation = execute_with_source_lines(
        composed.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed.sites(),
    )
    .unwrap();
    let (stdout, stderr) = observation.projections();
    assert_eq!(stderr, "[INFO] 3\n[INFO] -12\n[INFO] true\n");
    assert!(stdout.starts_with("-12\n"), "{stdout}");
    assert!(stdout.contains("\n-12\n0x03\n"), "{stdout}");
    let c = module.c_source.as_deref().unwrap();
    for (label, flags) in BUILDS {
        let run = build_and_run(c, flags, None)
            .unwrap_or_else(|error| panic!("{label}: the emitted C does not build: {error}"));
        assert_eq!(run.status, Some(0), "{label}: {}", run.stderr);
        assert_eq!(run.stdout, stdout, "{label}");
        observation
            .compare_stream(qcore_core::std_io::Stream::Err, &run.stderr)
            .unwrap_or_else(|refusal| panic!("{label}: stderr outside the relation: {refusal}"));
        assert_eq!(run.stderr, stderr, "{label}");
    }
}

/// The examples page publishes each example's transcript, read out of the
/// page and compared with the reference.
#[test]
fn the_threads_page_publishes_the_record_transcripts() {
    let page = std::fs::read_to_string(root().join("examples/threads/README.md"))
        .unwrap()
        .replace("\r\n", "\n");
    for (name, source) in EXAMPLES {
        let module = example(name, source);
        let (stdout, _) = reference(&module, Vec::new());
        let marker = format!("<!-- transcript:{name} -->\n\n```text\n");
        let block = page
            .split_once(marker.as_str())
            .unwrap_or_else(|| panic!("the `{name}` transcript is not on the page"))
            .1
            .split_once("```")
            .unwrap()
            .0;
        assert_eq!(block, stdout, "{name}");
    }
}
