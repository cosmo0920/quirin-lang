//! EXT-01: compare old and proposed source names on every mapped source.
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use serde_json::json;
use std::{fs, process::Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mapping = fs::read_to_string("docs/reports/extension-compatibility/mapping.tsv")?;
    let mut rows = Vec::new();
    let mut admitted = 0;
    let mut refused = 0;
    let mut changed = 0;
    for row in mapping.lines().skip(1) {
        let columns: Vec<_> = row.split('\t').collect();
        let old = columns[0];
        let new = columns[3];
        if !old.ends_with(".qcore") {
            continue;
        }
        let source = fs::read_to_string(old)?;
        let old_result =
            compile_files(&[SourceFile::new(old, &source)], &CompileOptions::default());
        let new_result =
            compile_files(&[SourceFile::new(new, &source)], &CompileOptions::default());
        let (same, outcome) = match (&old_result, &new_result) {
            (Ok(left), Ok(right)) => {
                admitted += 1;
                let same = left.mode == right.mode
                    && left.entry == right.entry
                    && left.execution_profile == right.execution_profile
                    && left.result_profile == right.result_profile
                    && left.declaration_count == right.declaration_count
                    && left.c_source == right.c_source.as_ref().map(|s| s.replace(new, old))
                    && format!("{:?}", left.checked)
                        == format!("{:?}", right.checked).replace(new, old)
                    && format!("{:?}", left.reference_observation)
                        == format!("{:?}", right.reference_observation).replace(new, old)
                    && format!("{:?}", left.extern_observation)
                        == format!("{:?}", right.extern_observation).replace(new, old)
                    && right.extension_warnings.is_empty();
                (same, "accepted")
            }
            (Err(left), Err(right)) => {
                refused += 1;
                (
                    format!("{left:?}") == format!("{right:?}").replace(new, old),
                    "refused",
                )
            }
            _ => (false, "outcome-changed"),
        };
        if !same {
            changed += 1;
        }
        rows.push(json!({"old":old,"new":new,"same_except_source_path_and_warning":same,"outcome":outcome}));
    }
    let revision = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    if !revision.status.success() {
        return Err("git rev-parse failed".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"source_tip":String::from_utf8(revision.stdout)?.trim(),"source_files":rows.len(),"accepted":admitted,"refused":refused,"changed":changed,"comparison":"Current reader at legacy and proposed names; explicit filename substitution only; warning fields excluded. Default compile options, no native manifest selection. Not a C corpus audit or a baseline binary comparison.","rows":rows})
        )?
    );
    if changed != 0 {
        return Err(format!("{changed} changed source outcomes").into());
    }
    Ok(())
}
