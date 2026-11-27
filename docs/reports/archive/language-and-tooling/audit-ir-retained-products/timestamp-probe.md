# Exact timestamp identity control

The raw production comparison exits 1. Both differing lifetime-summary keys
are reproduced from one Monkey unit by changing only its eight `__TIME__` bytes:
12:08:53 (baseline) and 17:54:05 (candidate), with `__DATE__` fixed by the real
same-day runs to Oct 22 2026. Both entries are empty objects. The native probe
uses the existing public compile-option parser and gateway preprocessor, falling
back to the recorded command in exactly the pre-pass manner. No audit source,
compile database, baseline artifact or produced report was altered.

Native probe (built with `rustc --edition=2024`, linking this gate’s qcore-audit
and qcore-c-gateway rlibs through `-L dependency=private/audit-ir-retained-products/target/debug/deps`; compile and run exit 0):

```rust
use std::{fs, path::Path, process::Command};
use qcore_audit::compile_db::{compile_entries, compiler_options, shell_words};
use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect, preprocess_unrestricted_c11};
fn main() {
 let args: Vec<String> = std::env::args().collect();
 let entries = compile_entries(&fs::read_to_string(&args[1]).unwrap()).unwrap();
 let entry = entries.iter().find(|e| e.file.ends_with("mk_bin/monkey.c")).unwrap();
 let raw = fs::read_to_string(&entry.file).unwrap();
 let mut result = None;
 if let Ok(o) = compiler_options(entry) {
  let unit = C11TranslationUnit {source:&raw, input:o.input, dialect:o.dialect, include_paths:o.include_paths, definitions:o.definitions, preprocessor_options:o.preprocessor_options, ..Default::default()};
  if let Ok(source) = preprocess_unrestricted_c11(&unit) {
   result = Some((source, format!("{:?}\0{:?}\0{:?}\0{:?}\0{:?}\0",unit.input,unit.dialect,unit.include_paths,unit.definitions,unit.preprocessor_options)));
   println!("gateway preprocessing");
  }
 }
 let (source, descriptor) = result.unwrap_or_else(|| {
  let words = shell_words(&entry.command).unwrap(); let mut argv = Vec::new(); let mut skip = false;
  for word in &words[1..] {
   if skip {skip=false;continue}
   match word.as_str() {"-c"|"-MD"|"-MMD"|"-MP"|"-M"|"-MM"|"-MG"=>{}, "-o"|"-MF"|"-MT"|"-MQ"=>skip=true,
    _ if word.starts_with("-o") && word.len()>2 =>{},
    _ if ["-MF","-MT","-MQ"].iter().any(|flag|word.starts_with(flag)&&word.len()>3)=>{}, _=>argv.push(word.clone()) }
  }
  let out=Command::new(&words[0]).args(argv).args(["-E","-P"]).current_dir(&entry.directory).output().unwrap(); assert!(out.status.success());
  let unit=C11TranslationUnit {dialect:CCompilerDialect::Gnu17,..Default::default()};
  println!("recorded command fallback preprocessing");
  (String::from_utf8(out.stdout).unwrap(), format!("{:?}\0{:?}\0{:?}\0{:?}\0{:?}\0",unit.input,unit.dialect,unit.include_paths,unit.definitions,unit.preprocessor_options))
 });
 fs::write(Path::new(&args[2]).join("monkey-preprocessed.txt"),source).unwrap();
 fs::write(Path::new(&args[2]).join("monkey-descriptor.bin"),descriptor).unwrap();
}
```

The database and output directory arguments are the frozen production database
and `private/audit-ir-retained-products`. Then run
`python3 docs/reports/audit-ir-retained-products/scripts/trace_timestamp.py`
(exit 0); [receipt](timestamp-identity.json). The first Python probe exited 1
because its pattern omitted the literal `" "` between the date and time; the
corrected probe matches the actual declaration. This diagnoses real input
volatility; it is not an instruction to erase identity fields for comparison.
