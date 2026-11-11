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

//! Card W48-QRLIB-LINKAGE: build a C static library with the host toolchain
//! and generate its `.qrlib`, for tests whose `.qr` programs call C through
//! `#c_ffi` contracts. Every such call now resolves through a generated
//! manifest the file binds with `extern library NAME;`, so a test that checks
//! one builds the artifact first — the same order a real build takes.
//!
//! The library is compiled by the `cc` crate: Clang (`QCORE_CLANG`, else
//! `clang`) and `ar` on Unix, Microsoft's `cl.exe` and `lib.exe` under MSVC.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_c_gateway::linkage::generate::{GenerationInputs, generate};
use qcore_c_gateway::linkage::link::{self, LinkPlan};
use qcore_core::native_library::{LinkageKind, load_native_library_manifest};
use qcore_core::scalar_types::RecordedTarget;

fn host() -> String {
    let version = Command::new("rustc").arg("-vV").output().unwrap();
    assert!(version.status.success());
    String::from_utf8(version.stdout)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap()
        .to_owned()
}

/// Compile `sources` into the static library `name` inside `directory`, with
/// `includes` on the include path, and return the archive's path.
pub fn static_library(
    directory: &Path,
    name: &str,
    sources: &[PathBuf],
    includes: &[&Path],
) -> PathBuf {
    let host = host();
    let mut build = cc::Build::new();
    build
        .host(&host)
        .target(&host)
        .out_dir(directory)
        .opt_level(1)
        .debug(false)
        .cargo_metadata(false)
        .warnings(true)
        .warnings_into_errors(true);
    if cfg!(target_env = "msvc") {
        // Keep warning-as-error builds reproducible when headers contain
        // UTF-8 text but the active Windows code page cannot represent it.
        build.flag("/utf-8");
    } else {
        build.compiler(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
        build.flag("-std=c11");
    }
    for include in includes {
        build.include(include);
    }
    for source in sources {
        build.file(source);
    }
    build.compile(name);
    let archive = if cfg!(target_env = "msvc") {
        directory.join(format!("{name}.lib"))
    } else {
        directory.join(format!("lib{name}.a"))
    };
    assert!(archive.is_file(), "{} was not written", archive.display());
    archive
}

/// A generated manifest: its selection key (a `.qrlib` path) and its text.
pub struct Generated {
    pub key: String,
    pub text: String,
}

impl Generated {
    /// The `CompileOptions::native_libraries` selection holding it.
    pub fn selection(&self) -> BTreeMap<String, String> {
        BTreeMap::from([(self.key.clone(), self.text.clone())])
    }

    /// The verified link: every pinned file hashed again, and the one path to
    /// hand the linker.
    pub fn link_plan(&self) -> LinkPlan {
        let manifest = load_native_library_manifest(&self.text).unwrap();
        link::plan(&manifest, &|reference: &str| PathBuf::from(reference)).unwrap()
    }
}

/// Generate the `.qrlib` of `artifact` (recorded by its absolute path) for
/// `library`, binding every declaration of `contracts`.
pub fn manifest(
    library: &str,
    linkage: LinkageKind,
    artifact: &Path,
    header: Option<&Path>,
    target: &RecordedTarget,
    contracts: &BTreeMap<String, String>,
) -> Generated {
    let artifact_ref = artifact.display().to_string();
    let header_bytes = header.map(|path| std::fs::read(path).unwrap());
    let header_ref = header.map(|path| path.display().to_string());
    let inputs = GenerationInputs {
        library,
        version: "0.1.0",
        linkage,
        artifact: &artifact_ref,
        header: header_ref.as_deref().zip(header_bytes.as_deref()),
        target,
        unwinding: "abort",
        allocator_custody: "closed",
        thread_posture: "single_caller",
        contracts,
        os_build: None,
    };
    let loader = |reference: &str| std::fs::read(reference).map_err(|error| error.to_string());
    let generated = generate(&inputs, &loader, None).unwrap_or_else(|refusal| {
        panic!("the manifest of `{library}` is not generated: {refusal}")
    });
    Generated {
        key: artifact
            .with_file_name(format!("{library}.qrlib"))
            .display()
            .to_string(),
        text: generated.text,
    }
}
