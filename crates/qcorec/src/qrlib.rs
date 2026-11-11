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

//! `quirin qrlib generate|check` and `quirin build` (card `W48-QRLIB-LINKAGE`).
//!
//! A `.qrlib` is generated on every build, before linking, from the artifact's
//! bytes and the build's declared inputs; `check` regenerates and refuses a
//! stored manifest that differs as stale; `build` regenerates every selected
//! manifest from its own declared inputs, checks the program against them,
//! emits the C, hashes every pinned file again and hands the recorded Clang
//! exactly the pinned artifacts.
//!
//! References a manifest records — the artifact and the header — resolve
//! against the manifest's own directory (the `-o` directory for `generate`),
//! with no search, exactly as a manifest's `header` always has. An absolute
//! reference, such as a system library's, is used as written. A contract module
//! is keyed by the path spelled on the command line, which is also the `from
//! MODULE` key the manifest records.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_compiler::linkage::generate::{GenerationInputs, check_stale, generate};
use qcore_compiler::linkage::link;
use qcore_compiler::native_library::{
    LinkageKind, NativeLibraryManifest, RuntimePremise, load_native_library_manifest,
};
use qcore_compiler::scalar_types::RecordedTarget;
use qcore_compiler::{CompileOptions, SourceFile, compile_files_with_filesystem_for_target};

/// Command-line flags: every `--name value` pair, repeatable, and the
/// positional arguments.
struct Flags {
    values: BTreeMap<String, Vec<String>>,
    positional: Vec<String>,
}

impl Flags {
    fn parse(arguments: &[String], known: &[&str]) -> Result<Self, String> {
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut positional = Vec::new();
        let mut index = 0;
        while index < arguments.len() {
            let argument = &arguments[index];
            if let Some(name) = argument
                .strip_prefix("--")
                .or_else(|| (argument == "-o").then_some("output"))
            {
                if !known.contains(&name) {
                    return Err(format!("unknown option `{argument}`"));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| format!("`{argument}` requires a value"))?;
                values
                    .entry(name.to_owned())
                    .or_default()
                    .push(value.clone());
            } else if argument.starts_with('-') {
                return Err(format!("unknown option `{argument}`"));
            } else {
                positional.push(argument.clone());
            }
            index += 1;
        }
        Ok(Self { values, positional })
    }

    fn one(&self, name: &str) -> Result<Option<&str>, String> {
        match self.values.get(name).map(Vec::as_slice) {
            None | Some([]) => Ok(None),
            Some([value]) => Ok(Some(value)),
            Some(_) => Err(format!("--{name} specified more than once")),
        }
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.one(name)?
            .ok_or_else(|| format!("--{name} is required; a declared input is never defaulted"))
    }

    fn all(&self, name: &str) -> &[String] {
        self.values.get(name).map_or(&[], Vec::as_slice)
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn recorded_target(path: &str) -> Result<RecordedTarget, String> {
    let text = read_text(Path::new(path))?;
    qcore_compiler::scalar_types::load_recorded_target(&text)
        .map_err(|error| format!("{path}: {error}"))
}

/// The contract selection, keyed by the path as spelled.
fn contracts(paths: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut selected = BTreeMap::new();
    for path in paths {
        if selected
            .insert(path.clone(), read_text(Path::new(path))?)
            .is_some()
        {
            return Err(format!(
                "the contract module `{path}` is selected more than once"
            ));
        }
    }
    Ok(selected)
}

/// Resolve a recorded reference against the manifest's directory.
fn resolve(directory: &Path, reference: &str) -> PathBuf {
    let path = Path::new(reference);
    if path.is_absolute() {
        path.to_owned()
    } else {
        directory.join(path)
    }
}

fn directory_of(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_owned)
}

fn linkage(text: &str) -> Result<LinkageKind, String> {
    match text {
        "static" => Ok(LinkageKind::Static),
        "dynamic" => Ok(LinkageKind::Dynamic),
        other => Err(format!(
            "--linkage `{other}` is neither `static` nor `dynamic`"
        )),
    }
}

/// Everything one generation reads, owned.
struct Owned {
    library: String,
    version: String,
    linkage: LinkageKind,
    artifact: String,
    header: Option<(String, Vec<u8>)>,
    target: RecordedTarget,
    unwinding: String,
    allocator_custody: String,
    thread_posture: String,
    contracts: BTreeMap<String, String>,
    os_build: Option<String>,
    directory: PathBuf,
    runtime_dir: Option<PathBuf>,
}

impl Owned {
    fn inputs(&self) -> GenerationInputs<'_> {
        GenerationInputs {
            library: &self.library,
            version: &self.version,
            linkage: self.linkage,
            artifact: &self.artifact,
            header: self
                .header
                .as_ref()
                .map(|(reference, bytes)| (reference.as_str(), bytes.as_slice())),
            target: &self.target,
            unwinding: &self.unwinding,
            allocator_custody: &self.allocator_custody,
            thread_posture: &self.thread_posture,
            contracts: &self.contracts,
            os_build: self.os_build.as_deref(),
        }
    }

    fn regenerate(&self, stored: Option<&str>) -> Result<String, String> {
        let directory = self.directory.clone();
        let loader = move |reference: &str| {
            fs::read(resolve(&directory, reference)).map_err(|error| error.to_string())
        };
        let runtime_dir = self.runtime_dir.clone();
        let runtime = move |dll: &str| -> Result<Vec<u8>, String> {
            let Some(directory) = &runtime_dir else {
                return Err("no --runtime-dir names where the OS keeps it".into());
            };
            fs::read(directory.join(dll)).map_err(|error| error.to_string())
        };
        let result = match stored {
            None => generate(&self.inputs(), &loader, Some(&runtime)),
            Some(stored) => check_stale(stored, &self.inputs(), &loader, Some(&runtime)),
        };
        result
            .map(|generated| generated.text)
            .map_err(|refusal| refusal.to_string())
    }
}

fn from_flags(flags: &Flags, directory: PathBuf) -> Result<Owned, String> {
    let header = match flags.one("header")? {
        None => None,
        Some(reference) => Some((reference.to_owned(), read(&resolve(&directory, reference))?)),
    };
    Ok(Owned {
        library: flags.required("library")?.to_owned(),
        version: flags.required("version")?.to_owned(),
        linkage: linkage(flags.required("linkage")?)?,
        artifact: flags.required("artifact")?.to_owned(),
        header,
        target: recorded_target(flags.required("recorded-target")?)?,
        unwinding: flags.required("unwinding")?.to_owned(),
        allocator_custody: flags.required("allocator-custody")?.to_owned(),
        thread_posture: flags.required("thread-posture")?.to_owned(),
        contracts: contracts(flags.all("contracts"))?,
        os_build: flags.one("os-build")?.map(ToOwned::to_owned),
        directory,
        runtime_dir: flags.one("runtime-dir")?.map(PathBuf::from),
    })
}

const GENERATION_FLAGS: [&str; 13] = [
    "library",
    "version",
    "linkage",
    "artifact",
    "header",
    "recorded-target",
    "unwinding",
    "allocator-custody",
    "thread-posture",
    "contracts",
    "os-build",
    "runtime-dir",
    "output",
];

/// `quirin qrlib generate …` and `quirin qrlib check STORED …`.
pub fn qrlib_command(arguments: &[String]) -> Result<(), String> {
    let (command, rest) = arguments
        .split_first()
        .ok_or("quirin qrlib: expected `generate` or `check`")?;
    let flags = Flags::parse(rest, &GENERATION_FLAGS)?;
    match command.as_str() {
        "generate" => {
            if !flags.positional.is_empty() {
                return Err("quirin qrlib generate takes no positional argument".into());
            }
            let output = PathBuf::from(flags.required("output")?);
            let owned = from_flags(&flags, directory_of(&output))?;
            let text = owned.regenerate(None)?;
            fs::write(&output, &text).map_err(|error| format!("{}: {error}", output.display()))?;
            let manifest = load_native_library_manifest(&text).map_err(|e| e.to_string())?;
            println!(
                "generated {}: library `{}` ({} linkage), {} entr{}, artifact `{}` at {}",
                output.display(),
                manifest.identity.library,
                owned.linkage.spelling(),
                manifest.contracted.len(),
                if manifest.contracted.len() == 1 {
                    "y"
                } else {
                    "ies"
                },
                manifest.identity.artifact,
                manifest.identity.hash
            );
            Ok(())
        }
        "check" => {
            let [stored] = flags.positional.as_slice() else {
                return Err("quirin qrlib check takes exactly one stored manifest".into());
            };
            if flags.one("output")?.is_some() {
                return Err("quirin qrlib check writes nothing; -o is not accepted".into());
            }
            let stored_path = PathBuf::from(stored);
            let owned = from_flags(&flags, directory_of(&stored_path))?;
            owned.regenerate(Some(&read_text(&stored_path)?))?;
            println!(
                "{stored}: identical to the manifest regenerated from the artifact and the declared inputs"
            );
            Ok(())
        }
        other => Err(format!(
            "quirin qrlib: unknown command `{other}`; expected `generate` or `check`"
        )),
    }
}

/// The declared inputs a stored generated manifest records, with the build's
/// contract selection and recorded target: what `build` regenerates it from.
fn declared_inputs(
    path: &Path,
    manifest: &NativeLibraryManifest,
    selection: &BTreeMap<String, String>,
    target: &RecordedTarget,
    runtime_dir: Option<&Path>,
) -> Result<Owned, String> {
    let linkage = manifest.linkage.as_ref().ok_or_else(|| {
        format!(
            "{}: a hand-written v1 manifest; `quirin build` links only through a generated one",
            path.display()
        )
    })?;
    let directory = directory_of(path);
    let mut contracts = BTreeMap::new();
    for module in manifest.slice() {
        let text = selection.get(module).ok_or_else(|| {
            format!(
                "{}: binds the contract module `{module}`, which the build does not select with --contracts",
                path.display()
            )
        })?;
        contracts.insert(module.to_owned(), text.clone());
    }
    let header = match &manifest.header {
        None => None,
        Some(header) => Some((
            header.reference.clone(),
            read(&resolve(&directory, &header.reference))?,
        )),
    };
    let os_build = linkage.runtime.iter().find_map(|premise| match premise {
        RuntimePremise::Dll { os_build, .. } => Some(os_build.clone()),
        _ => None,
    });
    Ok(Owned {
        library: manifest.identity.library.clone(),
        version: manifest.identity.version.clone(),
        linkage: linkage.kind,
        artifact: manifest.identity.artifact.clone(),
        header,
        target: target.clone(),
        unwinding: "abort".into(),
        allocator_custody: "closed".into(),
        thread_posture: "single_caller".into(),
        contracts,
        os_build,
        directory,
        runtime_dir: runtime_dir.map(Path::to_owned),
    })
}

/// `quirin build --recorded-target FILE --contracts FILE.qv… --native-library
/// M.qrlib… [--entry NAME] [--runtime-dir DIR] -o EXE FILE.qr…`.
pub fn build_command(arguments: &[String]) -> Result<(), String> {
    let flags = Flags::parse(
        arguments,
        &[
            "recorded-target",
            "contracts",
            "native-library",
            "entry",
            "runtime-dir",
            "output",
        ],
    )?;
    if flags.positional.is_empty() {
        return Err("quirin build: no input files were provided".into());
    }
    let output = PathBuf::from(flags.required("output")?);
    let target = recorded_target(flags.required("recorded-target")?)?;
    let selection = contracts(flags.all("contracts"))?;
    let runtime_dir = flags.one("runtime-dir")?.map(PathBuf::from);
    // 1. Regenerate every selected manifest from its own declared inputs, and
    //    refuse a stale one: a manifest is generated on every build.
    let mut manifests = BTreeMap::new();
    let mut plans = Vec::new();
    for path in flags.all("native-library") {
        let path = PathBuf::from(path);
        let text = read_text(&path)?;
        let manifest = load_native_library_manifest(&text)
            .map_err(|refusal| format!("{}: {refusal}", path.display()))?;
        let owned = declared_inputs(
            &path,
            &manifest,
            &selection,
            &target,
            runtime_dir.as_deref(),
        )?;
        owned
            .regenerate(Some(&text))
            .map_err(|refusal| format!("{}: {refusal}", path.display()))?;
        manifests.insert(path.display().to_string(), text);
        plans.push((path, manifest));
    }
    // 2. Check and emit.
    let files = flags
        .positional
        .iter()
        .map(|path| read_text(Path::new(path)).map(|source| SourceFile::new(path.clone(), source)))
        .collect::<Result<Vec<_>, _>>()?;
    let options = CompileOptions {
        entry: flags.one("entry")?.map(ToOwned::to_owned),
        extern_contracts: (!selection.is_empty()).then(|| selection.clone()),
        native_libraries: (!manifests.is_empty()).then_some(manifests),
        native_library_headers: None,
    };
    let module = compile_files_with_filesystem_for_target(&files, &options, Some(&target))
        .map_err(|diagnostics| {
            diagnostics
                .iter()
                .map(qcore_compiler::format_diagnostic)
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    let source = module
        .c_source
        .as_ref()
        .ok_or("quirin build: the module emits no C")?;
    let c_path = output.with_extension("c");
    fs::write(&c_path, source).map_err(|error| format!("{}: {error}", c_path.display()))?;
    // 3. Hash every pinned file again, immediately before the link, and hand
    //    the linker exactly the pinned artifacts.
    let mut inputs = Vec::new();
    for (path, manifest) in &plans {
        let directory = directory_of(path);
        let plan = link::plan(manifest, &|reference: &str| resolve(&directory, reference))
            .map_err(|refusal| format!("{}: {refusal}", path.display()))?;
        inputs.push(plan);
    }
    let mut includes = Vec::new();
    for assumption in module.ffi_assumptions() {
        if let qcore_compiler::ffi::FfiAssumption::BuilderIncludes { directories } = assumption {
            includes.extend(directories.iter().cloned());
        }
    }
    let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into());
    let mut command = Command::new(&compiler);
    command.args([
        "-std=c11",
        "-O2",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-ffp-contract=off",
    ]);
    for include in &includes {
        command.arg("-I").arg(include);
    }
    command.arg(&c_path);
    for plan in &inputs {
        command.arg(&plan.input);
    }
    command.arg("-o").arg(&output);
    let status = command
        .status()
        .map_err(|error| format!("{compiler}: {error}"))?;
    if !status.success() {
        return Err(format!(
            "{compiler} failed ({status}) linking {}",
            output.display()
        ));
    }
    println!(
        "built {}: {} regenerated manifest(s) identical to the stored ones; linked {}",
        output.display(),
        inputs.len(),
        inputs
            .iter()
            .map(|plan| format!(
                "`{}` ({})",
                plan.input.display(),
                plan.verified
                    .iter()
                    .map(|(reference, digest)| format!("{reference} at {digest}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}
