//! Card **ALIAS-AGGREGATE**: read-only probe over one pinned Cycle 19 unit.
//!
//! It runs the production gateway analysis over a single compilation-database
//! entry and prints, for one named function, every CFG node in line order with
//! the instruction the importer lowered it to, the preprocessed source line and
//! the diagnostics reported at it.
//!
//! It publishes no verdict, writes no audit artifact and reads no cached
//! evidence. Its only purpose is to make the *binding conjunct* of a refusal
//! visible on the production corpus rather than on a reduced fixture, which is
//! what card NEIGHBOURS recorded as `NEIGHBOURS-FIXTURE-PROXY`.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_audit::compile_db::{
    CompileEntry, compile_entries, compiler_options_compatibility as compiler_options,
};
use qcore_c_gateway::legacy_c::{
    LegacyDiagnostic, LegacyDiagnosticKind, LegacyInstruction, LegacyProgramSummary,
};
use qcore_compiler::contract_loader::load_resource_contract_manifest;
use qcore_compiler::source_profiles::{FLUENT_BIT_SOURCE_SET, load_profile};
use qcore_compiler::{
    C11TranslationUnit, LegacyFunctionScope, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example alias_aggregate_probe");
    let mut arguments = std::env::args_os().skip(1);
    let (Some(database), Some(file), Some(function)) = (
        arguments.next().map(PathBuf::from),
        arguments.next().map(PathBuf::from),
        arguments
            .next()
            .map(|value| value.to_string_lossy().into_owned()),
    ) else {
        eprintln!("usage: alias_aggregate_probe <compile_commands.json> <source.c> <function>");
        return ExitCode::FAILURE;
    };
    match run(&database, &file, &function) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("alias_aggregate_probe: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(database: &Path, file: &Path, function: &str) -> Result<(), String> {
    let profile = load_profile(FLUENT_BIT_SOURCE_SET).map_err(|error| error.to_string())?;
    let manifest = profile
        .resource_contract_manifest_path()
        .ok_or("source-set profile supplies no resource contracts")?;
    let contracts = load_resource_contract_manifest(&manifest)?;
    let entries = compile_entries(
        &fs::read_to_string(database)
            .map_err(|error| format!("{}: {error}", database.display()))?,
    )?;
    let file = file.canonicalize().map_err(|error| error.to_string())?;
    let entry = entries
        .iter()
        .find(|entry| entry.file == file)
        .ok_or_else(|| format!("no compile entry for {}", file.display()))?;
    let (preprocessed, summary) = analyze(entry, &contracts, function)?;
    if let Some(path) = std::env::var_os("QCORE_PROBE_PREPROCESSED") {
        fs::write(&path, &preprocessed).map_err(|error| error.to_string())?;
    }
    let lines = preprocessed.lines().collect::<Vec<_>>();
    let target = summary
        .functions
        .get(function)
        .ok_or_else(|| format!("`{function}` was not analysed"))?;
    let mut by_line: BTreeMap<usize, Vec<&LegacyDiagnostic>> = BTreeMap::new();
    for diagnostic in &target.diagnostics {
        by_line.entry(diagnostic.line).or_default().push(diagnostic);
    }
    println!("function {function}: status {:?}", target.status);
    println!("diagnostics: {}", target.diagnostics.len());
    let mut nodes = target.cfg.nodes.iter().enumerate().collect::<Vec<_>>();
    nodes.sort_by_key(|(index, node)| (node.line, *index));
    for (index, node) in nodes {
        let source = lines
            .get(node.line.saturating_sub(1))
            .map(|text| text.trim())
            .unwrap_or("");
        println!(
            "L{:<6} n{index:<4} {:<40} | {source}",
            node.line,
            instruction(&node.instruction)
        );
        for diagnostic in by_line.get(&node.line).into_iter().flatten() {
            println!("        !! {}", describe(diagnostic));
        }
    }
    println!("exits:");
    for exit in &target.exits {
        println!(
            "  L{:<6} node {:?} custody {:?} sites {:?}",
            exit.line,
            exit.node,
            exit.custody,
            exit.sites
                .iter()
                .map(|site| format!(
                    "site{}@L{} {:?}",
                    site.site, site.acquisition_line, site.disposition
                ))
                .collect::<Vec<_>>()
        );
    }
    let orphans = by_line
        .keys()
        .filter(|line| !target.cfg.nodes.iter().any(|node| node.line == **line))
        .copied()
        .collect::<Vec<_>>();
    for line in orphans {
        println!("L{line:<6} (no CFG node)");
        for diagnostic in by_line.get(&line).into_iter().flatten() {
            println!("        !! {}", describe(diagnostic));
        }
    }
    Ok(())
}

fn analyze(
    entry: &CompileEntry,
    contracts: &ResourceContracts,
    function: &str,
) -> Result<(String, LegacyProgramSummary), String> {
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    let preprocessed =
        qcore_compiler::validate_unrestricted_c11(&unit).map_err(|error| error.to_string())?;
    // `QCORE_PROBE_WITHDRAW=OLD=>NEW[;OLD=>NEW…]` re-analyses the *preprocessed*
    // text with the named statements replaced, so the card can measure whether a
    // residual is a consequence of a neighbouring refusal by withdrawing that
    // refusal and nothing else. Read-only with respect to every file.
    if let Some(spec) = std::env::var_os("QCORE_PROBE_WITHDRAW") {
        let mut text = preprocessed.clone();
        for pair in spec.to_string_lossy().split(';') {
            let Some((from, to)) = pair.split_once("=>") else {
                return Err(format!("malformed withdrawal `{pair}`"));
            };
            if !text.contains(from) {
                return Err(format!("withdrawal source `{from}` is absent"));
            }
            text = text.replace(from, to);
        }
        let summary = qcore_compiler::analyze_legacy_c_translation_unit(&text, function, contracts)
            .map_err(|error| format!("{error:?}"))?;
        return Ok((text, summary));
    }
    match analyze_unrestricted_c11(&unit, function, contracts).map_err(|error| error.to_string())? {
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => Ok((preprocessed, summary)),
        UnrestrictedLegacyResult::Unknown { reason, .. } => {
            Err(format!("no partial summary: {reason}"))
        }
        UnrestrictedLegacyResult::Unsafe(summary) => Ok((preprocessed, summary)),
        UnrestrictedLegacyResult::Safe(certificate) => Ok((preprocessed, certificate.summary)),
    }
}

fn instruction(instruction: &LegacyInstruction) -> String {
    match instruction {
        LegacyInstruction::Acquire { destination, site } => {
            format!("Acquire {destination} @{site}")
        }
        LegacyInstruction::Alias {
            destination,
            source,
        } => format!("Alias {destination} = {source}"),
        LegacyInstruction::Clear { destination } => format!("Clear {destination}"),
        LegacyInstruction::Release { source } => format!("Release {source}"),
        LegacyInstruction::Escape { source } => format!("Escape {source}"),
        LegacyInstruction::EscapeIfOwned { source } => format!("EscapeIfOwned {source}"),
        LegacyInstruction::ExactFormalReturn { source, parameter } => {
            format!("ExactFormalReturn {source} parameter {parameter}")
        }
        LegacyInstruction::ContractCall {
            function,
            destination,
            arguments,
            aggregate_reads,
            aggregate_consumed,
            aggregate_addresses,
            alias_reads,
            nested,
            ..
        } => {
            let mut text = format!(
                "Call {function}({}) -> {}",
                arguments
                    .iter()
                    .map(|argument| argument.as_deref().unwrap_or("_").to_owned())
                    .collect::<Vec<_>>()
                    .join(", "),
                destination.as_deref().unwrap_or("_")
            );
            if !aggregate_reads.is_empty() {
                text.push_str(&format!(" reads{aggregate_reads:?}"));
            }
            if !aggregate_consumed.is_empty() {
                text.push_str(&format!(" consumed{aggregate_consumed:?}"));
            }
            if !aggregate_addresses.is_empty() {
                text.push_str(&format!(" addresses{aggregate_addresses:?}"));
            }
            if !alias_reads.is_empty() {
                text.push_str(&format!(" alias{}", alias_reads.len()));
            }
            for nested in nested {
                text.push_str(&format!(" nested={}", nested.function));
            }
            text
        }
        LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } => format!("BranchOnNull {source} true_is_null={true_is_null}"),
        LegacyInstruction::BranchOnValidity {
            source,
            when_true,
            when_false,
        } => format!("BranchOnValidity {source} true={when_true:?} false={when_false:?}"),
        LegacyInstruction::BranchOnDistinct { .. } => "BranchOnDistinct".to_owned(),
        LegacyInstruction::CellProtocolStep { .. } => "CellProtocolStep".to_owned(),
        LegacyInstruction::OpaqueValue {
            destination,
            alias_read,
        } => format!(
            "OpaqueValue {destination} alias_read={}",
            alias_read.is_some()
        ),
        LegacyInstruction::VaArgPointer { destination } => format!("VaArgPointer {destination}"),
        LegacyInstruction::ScalarMemberStore {
            base, alias_read, ..
        } => format!(
            "ScalarMemberStore {base} alias_read={}",
            alias_read.is_some()
        ),
        LegacyInstruction::IndirectScalarAccess {
            typed_alternative: _,
            bases,
            alias_reads,
            destination,
            frame,
            borrowed_destination,
            // Card W44-INDIRECT-CHAIN-ACCESS: printed only when present, so
            // every single-selection line reads exactly as before.
            chains,
            // Card CG12-VOID-VIEW-READBACK: not printed, so every line reads
            // exactly as before.
            readback: _,
        } => format!(
            "IndirectScalarAccess bases={} alias_reads={} destination={} frame={}{}{}",
            bases.join(","),
            alias_reads.len(),
            destination.as_deref().unwrap_or("-"),
            frame.is_some(),
            if chains.is_empty() {
                String::new()
            } else {
                format!(" chains={}", chains.len())
            },
            if *borrowed_destination {
                " borrowed=true"
            } else {
                ""
            },
        ),
        LegacyInstruction::PointerStep {
            destination,
            source,
            offset,
        } => format!("PointerStep {destination} = {source} {offset:?}"),
        LegacyInstruction::ArgumentReads {
            names,
            pointer_bases,
            pointer_places,
        } => format!("ArgumentReads {names:?} pointer {pointer_bases:?} {pointer_places:?}"),
        LegacyInstruction::AggregateStore { place } => format!("AggregateStore {place}"),
        // Card W45-SUB-OBJECT-STORES.
        LegacyInstruction::SubObjectStore {
            destination, value, ..
        } => format!("SubObjectStore {} = {value:?}", destination.place),
        LegacyInstruction::ElementStore {
            base, index, call, ..
        } => format!("ElementStore {base}[{index:?}] call={}", call.is_some()),
        LegacyInstruction::ElementRelease {
            base,
            index,
            callee,
            ..
        } => format!("ElementRelease {callee}({base}[{index:?}])"),
        LegacyInstruction::BranchOnElementNull {
            base,
            index,
            true_is_null,
            ..
        } => format!("BranchOnElementNull {base}[{index:?}] true_is_null={true_is_null}"),
        LegacyInstruction::CountedLoopHeader { index, bound } => {
            format!("CountedLoopHeader {index} < {bound:?}")
        }
        LegacyInstruction::CountedLoopStep { index } => format!("CountedLoopStep {index}"),
        LegacyInstruction::ForgetScalar { destination } => format!("ForgetScalar {destination}"),
        LegacyInstruction::Unknown { description, .. } => format!("Unknown: {description}"),
        LegacyInstruction::NoOp => "NoOp".to_owned(),
        // Card CG11-POINTER-VALUED-STORES (gateway rule 10v).
        LegacyInstruction::TrackedSelfInit {
            function,
            base,
            path,
            writes,
            ..
        } => {
            format!("TrackedSelfInit {function} {base} {path:?} {writes:?}")
        }
        LegacyInstruction::PointerStore { store, .. } => {
            format!("PointerStore {} = {}", store.place, store.value)
        }
        LegacyInstruction::SelfInitCall { function, base, .. } => {
            format!("SelfInitCall {function}({base}) [checked empty frame]")
        }
    }
}

fn describe(diagnostic: &LegacyDiagnostic) -> String {
    match &diagnostic.kind {
        LegacyDiagnosticKind::Leak { acquisition_line } => format!("leak from L{acquisition_line}"),
        LegacyDiagnosticKind::DoubleRelease { acquisition_line } => {
            format!("double release of L{acquisition_line}")
        }
        LegacyDiagnosticKind::ReleaseAfterEscape { acquisition_line } => {
            format!("release after escape of L{acquisition_line}")
        }
        LegacyDiagnosticKind::CustodyConflict { acquisition_line } => {
            format!("custody conflict at L{acquisition_line}")
        }
        LegacyDiagnosticKind::InvalidRelease { acquisition_line } => {
            format!("invalid release of a pointer into L{acquisition_line}")
        }
        LegacyDiagnosticKind::UseAfterRelease { acquisition_line } => {
            format!("use after release of a pointer derived from L{acquisition_line}")
        }
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquisition_line,
            acquired,
            released,
        } => format!(
            "{} release of the {} acquired at L{acquisition_line}",
            released.spelling(),
            acquired.spelling()
        ),
        LegacyDiagnosticKind::UseAfterClose {
            acquisition_line,
            kind,
        } => format!(
            "use after release of the {} acquired at L{acquisition_line}",
            kind.spelling()
        ),
        LegacyDiagnosticKind::CountedUser { description } => {
            format!("counted user: {description}")
        }
        LegacyDiagnosticKind::UnknownAlias { variable } => {
            format!("unresolved resource identity `{variable}`")
        }
        LegacyDiagnosticKind::Unsupported { description } => format!("unsupported: {description}"),
        LegacyDiagnosticKind::SummaryConflict { .. } => "summary conflict".to_owned(),
    }
}
