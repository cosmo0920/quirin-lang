//! Card **FLUENT-BIT-LINK-PREPARE**: `quirin-audit prepare --link`.
//!
//! The `current` route's compile database is configured and partly built by
//! `prepare`, but nothing was linked. This step builds and links the
//! `fluent-bit` executable in that same build tree. It then reads which
//! definition every linked reference resolved to from the linker's own
//! output, never from the corpus text:
//!
//! 1. `cmake --build BUILD --target fluent-bit-bin` (the Makefile
//!    generator's link step, `link.txt`).
//! 2. The same link command is run again with `-Wl,-Map=… -Wl,--cref
//!    -Wl,--no-demangle` into a side path. Its output must equal the built
//!    executable byte for byte, so the map describes that executable.
//! 3. The map gives the linked inputs (`LOAD` lines and the archive members
//!    included), each input section's address range and the cross-reference
//!    table: for every global symbol, the file holding the definition the
//!    linker chose, then the files referencing it. Each linked object's ELF
//!    symbol table, and the executable's symbol tables, dynamic section and
//!    dynamic relocations, are read independently and cross-checked against
//!    the map. A disagreement refuses the symbol.
//! 4. Each linked object that a compile-database entry names is compiled
//!    again from that entry's own command (its `-o` operand redirected) and
//!    must reproduce the object byte for byte. A mismatch is a recorded
//!    refusal of that unit.
//!
//! The result is the link-resolution artifact
//! ([`ARTIFACT_SCHEMA`]), bound to the digests of the executable, every
//! linked input, the compile database, the link command and the toolchain
//! (compiler driver and linker binaries). From it, a candidate rule 4c link
//! map in the route's existing format (`cross_unit_links`) names exactly the
//! symbols with one strong global definition, in the executable, from a
//! reproduced object of a unit the `current` route selects. Every other
//! referenced function symbol is listed as refused with its reason
//! ([`REFUSALS_SCHEMA`]).
//!
//! Nothing reads either file: no route or profile sets the candidate map.
//! Adopting it as a derived resolution is card
//! BODY-SUMMARY-REUSE-LINK-STANDING's. This step is tooling. It changes no
//! verdict and proves no type or calling-convention equality.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};

/// Schema of the link-resolution artifact.
pub const ARTIFACT_SCHEMA: &str = "quirin-link-resolution/1";
/// Schema of the candidate map's refusal list.
pub const REFUSALS_SCHEMA: &str = "quirin-link-map-refusals/1";
/// The CMake target whose output is the `fluent-bit` executable.
pub const TARGET: &str = "fluent-bit-bin";
/// The target's link command, written by CMake's Makefile generator,
/// relative to the build directory. It runs in [`LINK_DIRECTORY`].
const LINK_TXT: &str = "src/CMakeFiles/fluent-bit-bin.dir/link.txt";
const LINK_DIRECTORY: &str = "src";

/// Link flags whose effect on symbol resolution the map and symbol tables
/// cannot settle fail-closed: the step stops and names the flag.
const UNSETTLED_LINK_FLAGS: [&str; 11] = [
    "-flto",
    "--version-script",
    "--wrap",
    "--defsym",
    "muldefs",
    "--allow-multiple-definition",
    "-shared",
    "-Bsymbolic",
    "--dynamic-list",
    "-fuse-ld",
    "--just-symbols",
];

// ---- ELF and archive readers ----------------------------------------------

/// A minimal ELF64 little-endian reader: section headers, symbol tables,
/// dynamic section and relocation sections. Nothing else is interpreted.
mod elf {
    pub const ET_REL: u16 = 1;
    pub const ET_EXEC: u16 = 2;
    pub const ET_DYN: u16 = 3;
    pub const SHN_UNDEF: u16 = 0;
    pub const SHN_COMMON: u16 = 0xfff2;
    pub const STB_LOCAL: u8 = 0;
    pub const STB_GLOBAL: u8 = 1;
    pub const STB_WEAK: u8 = 2;
    pub const STB_GNU_UNIQUE: u8 = 10;
    pub const STT_FUNC: u8 = 2;
    pub const STT_GNU_IFUNC: u8 = 10;
    pub const DF_1_PIE: u64 = 0x0800_0000;
    const SHT_SYMTAB: u32 = 2;
    const SHT_RELA: u32 = 4;
    const SHT_DYNAMIC: u32 = 6;
    const SHT_REL: u32 = 9;
    const SHT_DYNSYM: u32 = 11;
    const DT_NEEDED: i64 = 1;
    const DT_FLAGS_1: i64 = 0x6fff_fffb;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Symbol {
        pub name: String,
        pub bind: u8,
        pub kind: u8,
        pub visibility: u8,
        pub shndx: u16,
        pub value: u64,
    }

    impl Symbol {
        pub fn defined(&self) -> bool {
            self.shndx != SHN_UNDEF
        }
    }

    #[derive(Debug, Default)]
    pub struct Image {
        pub e_type: u16,
        pub symtab: Vec<Symbol>,
        pub dynsym: Vec<Symbol>,
        /// Names of `.dynsym` entries some dynamic relocation refers to.
        pub dynamic_relocations: Vec<String>,
        pub needed: Vec<String>,
        pub flags_1: u64,
    }

    struct Section {
        kind: u32,
        offset: usize,
        size: usize,
        link: usize,
        entsize: usize,
    }

    fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| "truncated ELF".into())
    }

    fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| "truncated ELF".into())
    }

    fn u64_at(bytes: &[u8], at: usize) -> Result<u64, String> {
        bytes
            .get(at..at + 8)
            .map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
            .ok_or_else(|| "truncated ELF".into())
    }

    fn usize_at(bytes: &[u8], at: usize) -> Result<usize, String> {
        usize::try_from(u64_at(bytes, at)?).map_err(|error| error.to_string())
    }

    fn string_at(bytes: &[u8], table: &Section, offset: usize) -> Result<String, String> {
        let start = table.offset + offset;
        let end = table.offset + table.size;
        let tail = bytes
            .get(start..end)
            .ok_or("ELF string outside its table")?;
        let length = tail
            .iter()
            .position(|&byte| byte == 0)
            .ok_or("unterminated ELF string")?;
        String::from_utf8(tail[..length].to_vec()).map_err(|error| error.to_string())
    }

    pub fn is_elf(bytes: &[u8]) -> bool {
        bytes.starts_with(b"\x7fELF")
    }

    pub fn parse(bytes: &[u8]) -> Result<Image, String> {
        if !is_elf(bytes) {
            return Err("not an ELF file".into());
        }
        if bytes.get(4) != Some(&2) || bytes.get(5) != Some(&1) {
            return Err("not a 64-bit little-endian ELF file".into());
        }
        let e_type = u16_at(bytes, 16)?;
        let shoff = usize_at(bytes, 0x28)?;
        let shentsize = usize::from(u16_at(bytes, 0x3a)?);
        let mut shnum = usize::from(u16_at(bytes, 0x3c)?);
        if shoff == 0 {
            return Ok(Image {
                e_type,
                ..Image::default()
            });
        }
        if shentsize != 64 {
            return Err("unexpected ELF section header size".into());
        }
        if shnum == 0 {
            shnum = usize_at(bytes, shoff + 32)?;
        }
        let mut sections = Vec::with_capacity(shnum);
        for index in 0..shnum {
            let header = shoff + index * shentsize;
            sections.push(Section {
                kind: u32_at(bytes, header + 4)?,
                offset: usize_at(bytes, header + 24)?,
                size: usize_at(bytes, header + 32)?,
                link: usize::try_from(u32_at(bytes, header + 40)?)
                    .map_err(|error| error.to_string())?,
                entsize: usize_at(bytes, header + 56)?,
            });
        }
        let symbols = |section: &Section| -> Result<Vec<Symbol>, String> {
            if section.entsize != 24 {
                return Err("unexpected ELF symbol size".into());
            }
            let strings = sections
                .get(section.link)
                .ok_or("ELF symbol table without strings")?;
            let mut out = Vec::with_capacity(section.size / 24);
            for index in 0..section.size / 24 {
                let at = section.offset + index * 24;
                let info = *bytes.get(at + 4).ok_or("truncated ELF")?;
                let other = *bytes.get(at + 5).ok_or("truncated ELF")?;
                let name = usize::try_from(u32_at(bytes, at)?).map_err(|e| e.to_string())?;
                out.push(Symbol {
                    name: string_at(bytes, strings, name)?,
                    bind: info >> 4,
                    kind: info & 0xf,
                    visibility: other & 3,
                    shndx: u16_at(bytes, at + 6)?,
                    value: u64_at(bytes, at + 8)?,
                });
            }
            Ok(out)
        };
        let mut image = Image {
            e_type,
            ..Image::default()
        };
        let mut dynsym_index = None;
        for (index, section) in sections.iter().enumerate() {
            match section.kind {
                SHT_SYMTAB => image.symtab = symbols(section)?,
                SHT_DYNSYM => {
                    image.dynsym = symbols(section)?;
                    dynsym_index = Some(index);
                }
                _ => {}
            }
        }
        for section in &sections {
            match section.kind {
                SHT_RELA | SHT_REL if Some(section.link) == dynsym_index => {
                    let entry = if section.kind == SHT_RELA { 24 } else { 16 };
                    for index in 0..section.size / entry {
                        let info = u64_at(bytes, section.offset + index * entry + 8)?;
                        let symbol = usize::try_from(info >> 32).map_err(|e| e.to_string())?;
                        if symbol != 0 {
                            let name = image
                                .dynsym
                                .get(symbol)
                                .ok_or("dynamic relocation names no symbol")?
                                .name
                                .clone();
                            image.dynamic_relocations.push(name);
                        }
                    }
                }
                SHT_DYNAMIC => {
                    let strings = sections
                        .get(section.link)
                        .ok_or("ELF dynamic section without strings")?;
                    for index in 0..section.size / 16 {
                        let at = section.offset + index * 16;
                        let tag = u64_at(bytes, at)?.cast_signed();
                        let value = u64_at(bytes, at + 8)?;
                        if tag == DT_NEEDED {
                            let offset = usize::try_from(value).map_err(|e| e.to_string())?;
                            image.needed.push(string_at(bytes, strings, offset)?);
                        } else if tag == DT_FLAGS_1 {
                            image.flags_1 = value;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(image)
    }
}

/// The members of a regular (not thin) `ar` archive, by name, with their
/// byte ranges. GNU long names (`//` table, `/N`) are resolved.
fn archive_members(bytes: &[u8]) -> Result<Vec<(String, std::ops::Range<usize>)>, String> {
    if bytes.starts_with(b"!<thin>\n") {
        return Err("thin archive: its members are not inside it".into());
    }
    if !bytes.starts_with(b"!<arch>\n") {
        return Err("not an ar archive".into());
    }
    let mut position = 8;
    let mut long_names: &[u8] = &[];
    let mut members = Vec::new();
    while position < bytes.len() {
        let header = bytes
            .get(position..position + 60)
            .ok_or("truncated archive header")?;
        if &header[58..60] != b"`\n" {
            return Err("malformed archive header".into());
        }
        let field = |range: std::ops::Range<usize>| {
            String::from_utf8_lossy(&header[range])
                .trim_end()
                .to_owned()
        };
        let name = field(0..16);
        let size: usize = field(48..58)
            .parse()
            .map_err(|_| "malformed archive member size".to_owned())?;
        let data = position + 60..position + 60 + size;
        if data.end > bytes.len() {
            return Err("truncated archive member".into());
        }
        if name == "/" || name == "/SYM64/" {
            // The symbol index: not a member.
        } else if name == "//" {
            long_names = &bytes[data.clone()];
        } else if let Some(offset) = name.strip_prefix('/') {
            let offset: usize = offset
                .parse()
                .map_err(|_| format!("unsupported archive member name `{name}`"))?;
            let tail = long_names
                .get(offset..)
                .ok_or("archive long name outside its table")?;
            let end = tail
                .windows(2)
                .position(|pair| pair == b"/\n")
                .ok_or("unterminated archive long name")?;
            members.push((
                String::from_utf8_lossy(&tail[..end]).into_owned(),
                data.clone(),
            ));
        } else if let Some(short) = name.strip_suffix('/') {
            members.push((short.to_owned(), data.clone()));
        } else {
            return Err(format!("unsupported archive member name `{name}`"));
        }
        position = data.end + (size & 1);
    }
    Ok(members)
}

// ---- GNU ld map --------------------------------------------------------------

/// What a GNU ld `-Map` file with `--cref` states, read under `LC_ALL=C`.
#[derive(Debug, Default, PartialEq, Eq)]
struct MapFile {
    /// `archive(member)` labels of every archive member the link included.
    members: BTreeSet<String>,
    /// `LOAD` operands in order.
    loads: Vec<String>,
    /// Input sections named `.text*`: (file label, address, size).
    text: Vec<(String, u64, u64)>,
    /// Cross-reference table: symbol, then the files the table lists (the
    /// definition's file first, then common definitions, then references).
    cref: Vec<(String, Vec<String>)>,
}

const MAP_ARCHIVE: &str = "Archive member included to satisfy reference by file (symbol)";
const MAP_MEMORY: &str = "Linker script and memory map";
const MAP_CREF: &str = "Cross Reference Table";

fn hex(token: &str) -> Option<u64> {
    u64::from_str_radix(token.strip_prefix("0x")?, 16).ok()
}

fn parse_map(text: &str) -> Result<MapFile, String> {
    #[derive(PartialEq)]
    enum State {
        Other,
        Archive,
        Memory,
        Cref,
    }
    let mut state = State::Other;
    let mut map = MapFile::default();
    let mut pending_section: Option<String> = None;
    let mut saw_cref = false;
    for line in text.lines() {
        let top = !line.is_empty() && !line.starts_with(char::is_whitespace);
        match line {
            MAP_ARCHIVE => {
                state = State::Archive;
                continue;
            }
            MAP_MEMORY => {
                state = State::Memory;
                continue;
            }
            MAP_CREF => {
                state = State::Cref;
                saw_cref = true;
                continue;
            }
            _ => {}
        }
        match state {
            State::Archive if top => {
                let label = line.split("  ").next().unwrap_or(line).trim();
                if label.ends_with(')') && label.contains(".a(") {
                    map.members.insert(label.to_owned());
                } else {
                    state = State::Other;
                }
            }
            State::Memory => {
                let pending = pending_section.take();
                if let Some(path) = line.strip_prefix("LOAD ") {
                    map.loads.push(path.trim().to_owned());
                    continue;
                }
                let tokens: Vec<&str> = line.split_whitespace().collect();
                let section_line = line.starts_with(' ') && !line.starts_with("  ");
                if section_line && tokens.len() == 1 {
                    pending_section = Some(tokens[0].to_owned());
                    continue;
                }
                let (name, rest) = if section_line && tokens.len() >= 4 {
                    (Some(tokens[0].to_owned()), &tokens[1..])
                } else if let Some(name) = pending
                    && tokens.len() >= 3
                {
                    (Some(name), &tokens[..])
                } else {
                    (None, &tokens[..])
                };
                if let Some(name) = name
                    && let (Some(address), Some(size)) = (
                        rest.first().and_then(|t| hex(t)),
                        rest.get(1).and_then(|t| hex(t)),
                    )
                    && rest.len() >= 3
                    && name.starts_with(".text")
                {
                    map.text.push((rest[2..].join(" "), address, size));
                }
            }
            State::Cref => {
                if line.trim().is_empty() || (map.cref.is_empty() && line.starts_with("Symbol ")) {
                    continue;
                }
                if top {
                    let mut parts = line.split_whitespace();
                    let name = parts.next().unwrap_or_default().to_owned();
                    let files = parts.map(str::to_owned).collect::<Vec<_>>();
                    if files.len() > 1 {
                        return Err(format!("cross-reference line with spaces: `{line}`"));
                    }
                    map.cref.push((name, files));
                } else {
                    let (_, files) = map
                        .cref
                        .last_mut()
                        .ok_or("cross-reference file before any symbol")?;
                    files.push(line.trim().to_owned());
                }
            }
            _ => {}
        }
    }
    if !saw_cref {
        return Err("the link map has no cross-reference table (--cref)".into());
    }
    Ok(map)
}

/// Card **LINK-STANDING** (rule 40): the bytes of one linked input as the
/// artifact labels it (relative to the link directory): an object file, or
/// the one archive member of that name.
pub(crate) fn input_bytes(link_dir: &Path, label: &str) -> Result<Vec<u8>, String> {
    let read = |label: &str| {
        let path = link_dir.join(label);
        fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let Some((archive, member)) = split_member(label) else {
        return read(label);
    };
    let bytes = read(archive)?;
    let members = archive_members(&bytes).map_err(|error| format!("{archive}: {error}"))?;
    match members
        .iter()
        .filter(|(name, _)| name == member)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [(_, range)] => Ok(bytes[range.clone()].to_vec()),
        found => Err(format!(
            "{label}: {} archive members of that name",
            found.len()
        )),
    }
}

/// `archive(member)` into its two parts.
fn split_member(label: &str) -> Option<(&str, &str)> {
    let open = label.find(".a(")? + 2;
    let member = label.get(open + 1..label.len().checked_sub(1)?)?;
    label.ends_with(')').then(|| (&label[..open], member))
}

// ---- Configurations and reproduction --------------------------------------

/// One compile-database record (one compilation configuration).
#[derive(Clone, Debug)]
pub struct Configuration {
    pub file: PathBuf,
    pub directory: PathBuf,
    pub command: String,
    /// The object the record names (`output`, relative to the build root).
    pub object: Option<PathBuf>,
    /// The `current` route's unit key (`label/relative file`), when the
    /// route selects this configuration.
    pub route_unit: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Reproduction {
    Equal,
    Differs(String),
    Failed(String),
}

/// Compile each configuration again from its own command, with only its
/// `-o` operand redirected into `scratch`, and compare with its object.
/// `PWD` is the record's directory, as `make`'s `cd DIR && …` sets it, so
/// the debug compilation directory is spelled as the build spelled it.
fn reproduce(
    configurations: &[Configuration],
    selected: &[usize],
    jobs: usize,
    scratch: &Path,
) -> Result<BTreeMap<usize, Reproduction>, String> {
    fs::create_dir_all(scratch).map_err(|error| format!("{}: {error}", scratch.display()))?;
    let next = AtomicUsize::new(0);
    let results = Mutex::new(BTreeMap::new());
    let run = |id: usize| -> Reproduction {
        let configuration = &configurations[id];
        let Some(object) = &configuration.object else {
            return Reproduction::Failed("the record names no object".into());
        };
        let mut words = match crate::compile_db::shell_words(&configuration.command) {
            Ok(words) => words,
            Err(error) => return Reproduction::Failed(error),
        };
        let outputs: Vec<usize> = (0..words.len()).filter(|&i| words[i] == "-o").collect();
        let [at] = outputs.as_slice() else {
            return Reproduction::Failed("the command has no single -o operand".into());
        };
        if *at + 1 >= words.len() {
            return Reproduction::Failed("the command's -o has no operand".into());
        }
        let target = scratch.join(format!("{id}.o"));
        words[*at + 1] = target.display().to_string();
        let output = Command::new(&words[0])
            .args(&words[1..])
            .current_dir(&configuration.directory)
            .env("PWD", &configuration.directory)
            .output();
        let result = match output {
            Err(error) => Reproduction::Failed(error.to_string()),
            Ok(output) if !output.status.success() => {
                Reproduction::Failed(format!("compiler exited {}", output.status))
            }
            Ok(_) => match (fs::read(object), fs::read(&target)) {
                (Ok(built), Ok(again)) if built == again => Reproduction::Equal,
                (Ok(_), Ok(again)) => Reproduction::Differs(sha256_hex(&again)),
                (Err(error), _) | (_, Err(error)) => Reproduction::Failed(error.to_string()),
            },
        };
        let _ = fs::remove_file(&target);
        result
    };
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&id) = selected.get(index) else {
                        break;
                    };
                    let result = run(id);
                    results
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .insert(id, result);
                }
            });
        }
    });
    let _ = fs::remove_dir(scratch);
    Ok(results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

// ---- Resolution ------------------------------------------------------------

/// One linked input: an object file or an included archive member.
#[derive(Debug)]
struct Input {
    label: String,
    sha256: String,
    symbols: Vec<elf::Symbol>,
    /// The compile-database configurations whose object has these bytes.
    configurations: Vec<usize>,
}

/// Decision for one function symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub name: String,
    /// `None` when derived; otherwise the refusal reason (a fixed phrase).
    pub refusal: Option<&'static str>,
    pub detail: String,
    pub home_unit: Option<String>,
    pub record: Value,
}

/// Everything read from one link.
pub struct Resolution {
    pub decisions: Vec<Decision>,
    pub inputs: Value,
    pub configurations: Value,
    pub executable: Value,
    pub counts: Value,
}

const NO_DEFINITION: &str = "no definition in the link";
const SHARED_OBJECT: &str = "defined in a shared object";
const NOT_DATABASE: &str = "defining object has no unique compile database entry";
const DISAGREE: &str = "link map and symbol tables disagree";
const IFUNC: &str = "indirect function (resolved at run time)";
const WEAK: &str = "weak definition";
const DUPLICATE: &str = "more than one definition among the linked objects";
const INTERPOSABLE: &str = "the executable resolves it at run time (dynamic relocation)";
const NOT_EXECUTABLE: &str = "the linked image is not an executable";
const NOT_REPRODUCED: &str = "home object is not reproduced by its compile database entry";
const NOT_ROUTE_UNIT: &str = "home configuration is not a unit of the current route";
const AMBIGUOUS_UNIT: &str = "home unit key names more than one route configuration";

/// The refusal phrases, in decision order.
pub const REFUSAL_REASONS: [&str; 12] = [
    NOT_EXECUTABLE,
    NO_DEFINITION,
    SHARED_OBJECT,
    DISAGREE,
    NOT_DATABASE,
    IFUNC,
    WEAK,
    DUPLICATE,
    INTERPOSABLE,
    NOT_REPRODUCED,
    NOT_ROUTE_UNIT,
    AMBIGUOUS_UNIT,
];

fn binding(symbol: &elf::Symbol) -> &'static str {
    match (symbol.shndx, symbol.bind) {
        (elf::SHN_COMMON, _) => "common",
        (_, elf::STB_LOCAL) => "local",
        (_, elf::STB_GLOBAL) => "global",
        (_, elf::STB_WEAK) => "weak",
        (_, elf::STB_GNU_UNIQUE) => "unique",
        _ => "other",
    }
}

fn visibility(symbol: &elf::Symbol) -> &'static str {
    ["default", "internal", "hidden", "protected"][usize::from(symbol.visibility)]
}

fn kind(symbol: &elf::Symbol) -> &'static str {
    match symbol.kind {
        elf::STT_FUNC => "function",
        elf::STT_GNU_IFUNC => "ifunc",
        1 => "object",
        6 => "tls",
        0 => "notype",
        _ => "other",
    }
}

/// Read the link: `link_dir` is where the link command ran (labels in the
/// map are relative to it), `map_text` the map, `executable` the image the
/// map describes.
fn resolve(
    link_dir: &Path,
    map_text: &str,
    executable: &[u8],
    configurations: &[Configuration],
    jobs: usize,
    scratch: &Path,
) -> Result<Resolution, String> {
    let map = parse_map(map_text)?;
    let image = elf::parse(executable)?;
    let is_executable = image.e_type == elf::ET_EXEC
        || (image.e_type == elf::ET_DYN && image.flags_1 & elf::DF_1_PIE != 0);
    let read = |label: &str| {
        let path = link_dir.join(label);
        fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
    };
    // Every configuration's object, by digest.
    let mut by_digest: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (id, configuration) in configurations.iter().enumerate() {
        if let Some(object) = &configuration.object
            && let Ok(bytes) = fs::read(object)
        {
            by_digest.entry(sha256_hex(&bytes)).or_default().push(id);
        }
    }
    // Linked inputs: LOAD operands that are relocatable objects, and the
    // archive members the map says were included. Shared objects are kept
    // by label. LLVM bitcode (LTO) stops the step.
    let mut inputs: Vec<Input> = Vec::new();
    let mut shared: BTreeSet<String> = BTreeSet::new();
    let mut scripts: BTreeSet<String> = BTreeSet::new();
    let mut archives: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let take = |label: String, bytes: &[u8]| -> Result<Input, String> {
        if bytes.starts_with(b"BC\xc0\xde") {
            return Err(format!(
                "{label} is LLVM bitcode (LTO): the link map does not describe compiled objects"
            ));
        }
        let parsed = elf::parse(bytes).map_err(|error| format!("{label}: {error}"))?;
        if parsed.e_type != elf::ET_REL {
            return Err(format!("{label}: linked input is not a relocatable object"));
        }
        let sha256 = sha256_hex(bytes);
        Ok(Input {
            label,
            configurations: by_digest.get(&sha256).cloned().unwrap_or_default(),
            sha256,
            symbols: parsed.symtab,
        })
    };
    let mut seen = BTreeSet::new();
    for label in &map.loads {
        if !seen.insert(label.clone()) {
            continue;
        }
        let bytes = read(label)?;
        if bytes.starts_with(b"!<arch>\n") || bytes.starts_with(b"!<thin>\n") {
            archives.insert(label.clone(), bytes);
        } else if elf::is_elf(&bytes) {
            match elf::parse(&bytes)?.e_type {
                elf::ET_DYN => {
                    shared.insert(label.clone());
                }
                _ => inputs.push(take(label.clone(), &bytes)?),
            }
        } else if bytes.starts_with(b"BC\xc0\xde") {
            return Err(format!("{label} is LLVM bitcode (LTO)"));
        } else {
            scripts.insert(label.clone());
        }
    }
    let mut ambiguous_members = BTreeSet::new();
    for label in &map.members {
        let (archive, member) =
            split_member(label).ok_or_else(|| format!("malformed member label `{label}`"))?;
        let bytes = archives
            .get(archive)
            .ok_or_else(|| format!("{label}: its archive is not a LOAD operand"))?;
        let members = archive_members(bytes).map_err(|error| format!("{archive}: {error}"))?;
        let matches: Vec<_> = members.iter().filter(|(name, _)| name == member).collect();
        match matches.as_slice() {
            [(_, range)] => inputs.push(take(label.clone(), &bytes[range.clone()])?),
            [] => return Err(format!("{label}: no such archive member")),
            _ => {
                ambiguous_members.insert(label.clone());
            }
        }
    }
    let index: BTreeMap<&str, usize> = inputs
        .iter()
        .enumerate()
        .map(|(id, input)| (input.label.as_str(), id))
        .collect();
    // Every archive member the memory map or the cross-reference table names
    // must be an included member: otherwise the map is not read completely.
    for label in map
        .text
        .iter()
        .map(|(label, _, _)| label)
        .chain(map.cref.iter().flat_map(|(_, files)| files))
    {
        if split_member(label).is_some()
            && !index.contains_key(label.as_str())
            && !ambiguous_members.contains(label)
        {
            return Err(format!(
                "{label} appears in the map but not among the included archive members"
            ));
        }
    }
    // Definitions and references by name across every linked input.
    let mut definitions: BTreeMap<&str, Vec<(usize, &elf::Symbol)>> = BTreeMap::new();
    let mut references: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    for (id, input) in inputs.iter().enumerate() {
        for symbol in &input.symbols {
            if symbol.bind == elf::STB_LOCAL || symbol.name.is_empty() {
                continue;
            }
            if symbol.defined() {
                definitions
                    .entry(&symbol.name)
                    .or_default()
                    .push((id, symbol));
            } else {
                references.entry(&symbol.name).or_default().insert(id);
            }
        }
    }
    let mut text: BTreeMap<&str, Vec<(u64, u64)>> = BTreeMap::new();
    for (label, address, size) in &map.text {
        text.entry(label).or_default().push((*address, *size));
    }
    let mut executable_entries: BTreeMap<&str, Vec<&elf::Symbol>> = BTreeMap::new();
    for symbol in image.symtab.iter().filter(|symbol| symbol.defined()) {
        executable_entries
            .entry(&symbol.name)
            .or_default()
            .push(symbol);
    }
    let relocated: BTreeMap<&str, usize> =
        image
            .dynamic_relocations
            .iter()
            .fold(BTreeMap::new(), |mut counts, name| {
                *counts.entry(name.as_str()).or_default() += 1;
                counts
            });
    let exported: BTreeSet<&str> = image
        .dynsym
        .iter()
        .filter(|symbol| symbol.defined())
        .map(|symbol| symbol.name.as_str())
        .collect();
    let shared_kinds: BTreeMap<&str, u8> = image
        .dynsym
        .iter()
        .filter(|symbol| !symbol.defined())
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    // Homes to reproduce: every configuration a linked input matches.
    let linked_configurations: BTreeSet<usize> = inputs
        .iter()
        .filter(|input| input.configurations.len() == 1)
        .flat_map(|input| input.configurations.iter().copied())
        .collect();
    let started = Instant::now();
    eprintln!(
        "prepare: reproducing {} linked object(s) from their compile database entries",
        linked_configurations.len()
    );
    let reproduced = reproduce(
        configurations,
        &linked_configurations.iter().copied().collect::<Vec<_>>(),
        jobs,
        scratch,
    )?;
    eprintln!(
        "prepare: reproduction finished in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    let mut route_keys: BTreeMap<&str, usize> = BTreeMap::new();
    for configuration in configurations {
        if let Some(key) = &configuration.route_unit {
            *route_keys.entry(key).or_default() += 1;
        }
    }
    let mut decisions = Vec::new();
    let mut skipped_versioned = 0usize;
    for (name, files) in &map.cref {
        if name.contains('@') {
            skipped_versioned += 1;
            continue;
        }
        let defined = definitions.get(name.as_str()).cloned().unwrap_or_default();
        let referencing = references.get(name.as_str()).cloned().unwrap_or_default();
        let first = files.first().map(String::as_str).unwrap_or_default();
        let definer = index.get(first).copied();
        let definer_symbol = definer.and_then(|id| {
            defined
                .iter()
                .find(|(input, _)| *input == id)
                .map(|(_, symbol)| *symbol)
        });
        let symbol_kind = definer_symbol.map_or_else(
            || shared_kinds.get(name.as_str()).copied().unwrap_or(0),
            |symbol| symbol.kind,
        );
        if symbol_kind != elf::STT_FUNC && symbol_kind != elf::STT_GNU_IFUNC {
            continue;
        }
        let referencing_others: BTreeSet<usize> = referencing
            .iter()
            .copied()
            .filter(|id| Some(*id) != definer)
            .collect();
        if referencing_others.is_empty() {
            continue;
        }
        // The cross-reference table's references must be the inputs whose
        // symbol tables reference the name.
        let cref_references: BTreeSet<usize> = files
            .iter()
            .skip(1)
            .filter_map(|file| index.get(file.as_str()).copied())
            .filter(|id| !defined.iter().any(|(input, _)| input == id))
            .collect();
        let agree_references = cref_references == referencing;
        let home_configuration = definer
            .map(|id| &inputs[id].configurations)
            .and_then(|found| (found.len() == 1).then(|| found[0]));
        let entries = executable_entries
            .get(name.as_str())
            .cloned()
            .unwrap_or_default();
        let in_text = definer_symbol.is_some_and(|_| {
            entries.iter().any(|entry| {
                text.get(first).is_some_and(|ranges| {
                    ranges
                        .iter()
                        .any(|(start, size)| entry.value >= *start && entry.value < start + size)
                })
            })
        });
        let reproduction = home_configuration.and_then(|id| reproduced.get(&id));
        let route_unit = home_configuration.and_then(|id| configurations[id].route_unit.clone());
        let (refusal, detail): (Option<&'static str>, String) = if !is_executable {
            (Some(NOT_EXECUTABLE), format!("ELF type {}", image.e_type))
        } else if shared.contains(first) {
            (Some(SHARED_OBJECT), first.to_owned())
        } else if definer.is_none() || definer_symbol.is_none() {
            if defined.is_empty() {
                (Some(NO_DEFINITION), first.to_owned())
            } else {
                (
                    Some(DISAGREE),
                    format!("the table names {first}, which does not define it"),
                )
            }
        } else if !agree_references {
            (
                Some(DISAGREE),
                "the table's references differ from the symbol tables' references".into(),
            )
        } else if !in_text {
            (
                Some(DISAGREE),
                "no executable symbol of this name lies in the definer's text".into(),
            )
        } else if home_configuration.is_none() {
            (
                Some(NOT_DATABASE),
                format!(
                    "{first} matches {} configuration(s)",
                    definer.map_or(0, |id| inputs[id].configurations.len())
                ),
            )
        } else if symbol_kind == elf::STT_GNU_IFUNC {
            (Some(IFUNC), String::new())
        } else if definer_symbol.is_some_and(|symbol| symbol.bind != elf::STB_GLOBAL) {
            (
                Some(WEAK),
                definer_symbol.map(binding).unwrap_or_default().to_owned(),
            )
        } else if defined.len() != 1 {
            (
                Some(DUPLICATE),
                defined
                    .iter()
                    .map(|(id, symbol)| format!("{} ({})", inputs[*id].label, binding(symbol)))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else if let Some(count) = relocated.get(name.as_str()) {
            (Some(INTERPOSABLE), format!("{count} relocation(s)"))
        } else if reproduction != Some(&Reproduction::Equal) {
            (Some(NOT_REPRODUCED), format!("{reproduction:?}"))
        } else if route_unit.is_none() {
            (
                Some(NOT_ROUTE_UNIT),
                configurations[home_configuration.unwrap_or_default()]
                    .file
                    .display()
                    .to_string(),
            )
        } else if route_unit
            .as_deref()
            .and_then(|key| route_keys.get(key))
            .is_some_and(|count| *count != 1)
        {
            (Some(AMBIGUOUS_UNIT), route_unit.clone().unwrap_or_default())
        } else {
            (None, String::new())
        };
        let record = json!({
            "name": name,
            "kind": if symbol_kind == elf::STT_GNU_IFUNC { "ifunc" } else { "function" },
            "definition": {
                "file": first,
                "in": if shared.contains(first) { "shared_object" } else if definer.is_some() { "executable" } else { "none" },
                "input": definer,
                "configuration": home_configuration,
                "binding": definer_symbol.map(binding),
                "visibility": definer_symbol.map(visibility),
                "type": definer_symbol.map(kind),
            },
            "definitions": defined.iter().map(|(id, symbol)| json!({"input": id, "binding": binding(symbol)})).collect::<Vec<_>>(),
            "referenced_by": referencing_others,
            "executable": {
                "entries": entries.iter().map(|entry| json!({"binding": binding(entry), "visibility": visibility(entry), "value": entry.value})).collect::<Vec<_>>(),
                "in_definer_text": in_text,
                "exported": exported.contains(name.as_str()),
                "dynamic_relocations": relocated.get(name.as_str()).copied().unwrap_or(0),
            },
            "home_unit": route_unit,
            "decision": refusal.map_or("derived", |_| "refused"),
            "reason": refusal,
            "detail": detail,
        });
        decisions.push(Decision {
            name: name.clone(),
            refusal,
            detail,
            home_unit: refusal.is_none().then_some(route_unit).flatten(),
            record,
        });
    }
    let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
    for decision in &decisions {
        *by_reason
            .entry(decision.refusal.unwrap_or("derived"))
            .or_default() += 1;
    }
    let configurations_json = configurations
        .iter()
        .enumerate()
        .map(|(id, configuration)| {
            let linked = linked_configurations.contains(&id);
            json!({
                "id": id,
                "file": configuration.file,
                "directory": configuration.directory,
                "command_sha256": sha256_hex(configuration.command.as_bytes()),
                "object": configuration.object,
                "linked": linked,
                "reproduction": match reproduced.get(&id) {
                    None => Value::Null,
                    Some(Reproduction::Equal) => json!("equal"),
                    Some(Reproduction::Differs(digest)) => json!({"differs": digest}),
                    Some(Reproduction::Failed(error)) => json!({"failed": error}),
                },
                "route_unit": configuration.route_unit,
            })
        })
        .collect::<Vec<_>>();
    let inputs_json = inputs
        .iter()
        .enumerate()
        .map(|(id, input)| {
            json!({
                "id": id,
                "label": input.label,
                "sha256": input.sha256,
                "configurations": input.configurations,
            })
        })
        .collect::<Vec<_>>();
    let counts = json!({
        "linked_inputs": inputs.len(),
        "linked_inputs_with_one_configuration": inputs.iter().filter(|input| input.configurations.len() == 1).count(),
        "linked_inputs_without_configuration": inputs.iter().filter(|input| input.configurations.is_empty()).count(),
        "ambiguous_archive_members": ambiguous_members,
        "shared_objects": shared,
        "linker_scripts": scripts,
        "configurations": configurations.len(),
        "configurations_linked": linked_configurations.len(),
        "configurations_reproduced": reproduced.values().filter(|result| **result == Reproduction::Equal).count(),
        "configurations_not_reproduced": reproduced.values().filter(|result| **result != Reproduction::Equal).count(),
        "cross_reference_symbols": map.cref.len(),
        "versioned_names_skipped": skipped_versioned,
        "function_symbols_referenced_across_inputs": decisions.len(),
        "by_decision": by_reason,
    });
    let executable_json = json!({
        "elf_type": image.e_type,
        "pie": image.flags_1 & elf::DF_1_PIE != 0,
        "is_executable": is_executable,
        "needed": image.needed,
        "dynamic_symbols_defined": exported.len(),
        "dynamic_relocations_against_symbols": image.dynamic_relocations.len(),
    });
    Ok(Resolution {
        decisions,
        inputs: Value::Array(inputs_json),
        configurations: Value::Array(configurations_json),
        executable: executable_json,
        counts,
    })
}

/// The candidate map, in the route's `cross_unit_links` format, and its
/// refusal list.
fn candidate_map(resolution: &Resolution, premise: &str, artifact_sha256: &str) -> (Value, Value) {
    let symbols = resolution
        .decisions
        .iter()
        .filter_map(|decision| {
            decision
                .home_unit
                .as_ref()
                .map(|home| json!({"function": decision.name, "home_unit": home}))
        })
        .collect::<Vec<_>>();
    let refused = resolution
        .decisions
        .iter()
        .filter_map(|decision| {
            decision.refusal.map(|reason| {
                json!({"function": decision.name, "reason": reason, "detail": decision.detail})
            })
        })
        .collect::<Vec<_>>();
    (
        json!({"schema_version": 1, "premise": premise, "symbols": symbols}),
        json!({
            "schema": REFUSALS_SCHEMA,
            "artifact_sha256": artifact_sha256,
            "reasons": REFUSAL_REASONS,
            "counts": resolution.counts["by_decision"],
            "refused": refused,
        }),
    )
}

// ---- The prepare step ------------------------------------------------------

/// What `prepare --link` needs from the clang-build step.
pub struct LinkRequest<'a> {
    pub source: &'a Path,
    pub build: &'a Path,
    pub database: &'a Path,
    pub cmake: &'a str,
    pub jobs: usize,
    pub output: &'a Path,
    pub profile_directory: &'a Path,
}

/// The files `prepare --link` wrote, with their digests.
pub struct LinkPrepared {
    pub record: Value,
    pub artifact: PathBuf,
    pub candidate: PathBuf,
}

fn sha_file(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| format!("non-UTF-8 path {}", path.display()))
}

fn checked(command: &mut Command, what: &str) -> Result<std::process::Output, String> {
    let output = command
        .output()
        .map_err(|error| format!("{what}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{what} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output)
}

fn tracked_changes(source: &Path) -> Result<String, String> {
    let output = checked(
        Command::new("git").args([
            "-C",
            text(source)?,
            "status",
            "--porcelain",
            "--untracked-files=no",
        ]),
        "git status",
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The route's unit key of every configuration the `current` route selects
/// (`audit.json`'s library selections over the profile's source roots).
fn route_units(
    database_text: &str,
    source: &Path,
    profile_directory: &Path,
) -> Result<Vec<Option<String>>, String> {
    let entries = crate::compile_db::compile_entries(database_text)?;
    let profile = qcore_contract::profile_loader::load_profile_directory(profile_directory)
        .map_err(|error| error.to_string())?;
    let mut keys = vec![None; entries.len()];
    for selection in crate::load_library_selections(&profile_directory.join("audit.json"))? {
        let library = profile
            .library(&selection.library)
            .ok_or("selected library is absent from profile")?;
        let label = selection
            .label
            .clone()
            .unwrap_or_else(|| selection.library.clone());
        for chosen in crate::current::select_configurations(&entries, source, library)? {
            let file = crate::input_paths::relative_within(&chosen.file, source)
                .ok_or("route unit outside the checkout")?;
            for (id, entry) in entries.iter().enumerate() {
                if entry.file == chosen.file
                    && entry.directory == chosen.directory
                    && entry.command == chosen.command
                {
                    keys[id] = Some(format!("{label}/{}", file.display()));
                }
            }
        }
    }
    Ok(keys)
}

/// Build and link `fluent-bit`, read the link, and write the artifact, the
/// candidate map and its refusals under `request.output`.
pub fn prepare_link(request: &LinkRequest) -> Result<LinkPrepared, String> {
    fs::create_dir_all(request.output)
        .map_err(|error| format!("{}: {error}", request.output.display()))?;
    let before = tracked_changes(request.source)?;
    let build_log = request.output.join("link-build.log");
    let build_command: Vec<String> = vec![
        request.cmake.into(),
        "--build".into(),
        text(request.build)?.into(),
        "--target".into(),
        TARGET.into(),
        format!("-j{}", request.jobs),
    ];
    eprintln!("prepare: building {TARGET}; log: {}", build_log.display());
    let started = Instant::now();
    let log = fs::File::create(&build_log).map_err(|error| error.to_string())?;
    let status = Command::new(&build_command[0])
        .args(&build_command[1..])
        .stdout(log.try_clone().map_err(|error| error.to_string())?)
        .stderr(log)
        .status()
        .map_err(|error| format!("{}: {error}", build_command[0]))?;
    let build_seconds = started.elapsed().as_secs_f64();
    if !status.success() {
        return Err(format!(
            "building {TARGET} failed ({status}); see {}",
            build_log.display()
        ));
    }
    if tracked_changes(request.source)? != before {
        return Err("building fluent-bit changed tracked source files".into());
    }
    // The link command CMake ran, and its stop conditions.
    let link_txt = request.build.join(LINK_TXT);
    let link_text = fs::read_to_string(&link_txt).map_err(|error| {
        format!(
            "{}: {error} (the link step reads the Makefile generator's link command)",
            link_txt.display()
        )
    })?;
    let words = crate::compile_db::shell_words(link_text.trim())?;
    for word in &words {
        if let Some(flag) = UNSETTLED_LINK_FLAGS
            .iter()
            .find(|flag| word.contains(*flag))
        {
            return Err(format!(
                "the link command carries `{word}` ({flag}): its effect on symbol resolution is a \
                 choice this step does not settle"
            ));
        }
    }
    let outputs: Vec<usize> = (0..words.len()).filter(|&i| words[i] == "-o").collect();
    let [at] = outputs.as_slice() else {
        return Err("the link command has no single -o operand".into());
    };
    let link_dir = request.build.join(LINK_DIRECTORY);
    let executable = link_dir.join(words.get(at + 1).ok_or("-o without operand")?);
    // The driver's own linker invocation names the linker.
    let mut probe = words.clone();
    probe.insert(1, "-###".into());
    let probe = checked(
        Command::new(&probe[0])
            .args(&probe[1..])
            .current_dir(&link_dir)
            .env("PWD", &link_dir)
            .env("LC_ALL", "C"),
        "link driver probe",
    )?;
    let jobs_text = String::from_utf8_lossy(&probe.stderr);
    let linker = jobs_text
        .lines()
        .rev()
        .find_map(|line| {
            let line = line.trim();
            line.starts_with('"')
                .then(|| line[1..].split('"').next().unwrap_or_default().to_owned())
        })
        .ok_or("the driver probe names no linker")?;
    let linker_version = checked(
        Command::new(&linker).arg("--version").env("LC_ALL", "C"),
        "linker version",
    )?;
    let linker_version = String::from_utf8_lossy(&linker_version.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    if !linker_version.starts_with("GNU ld") {
        return Err(format!(
            "the linker is `{linker_version}`; the map reader reads GNU ld's map"
        ));
    }
    // The same link, into a side path, with the map and its cross-reference
    // table. The map describes the executable only if the outputs are equal.
    let map_path = request.output.join("fluent-bit.map");
    let side = request.output.join("fluent-bit.relinked");
    let mut relink = words.clone();
    relink[at + 1] = text(&side)?.into();
    for word in &mut relink {
        if word.starts_with("--dependency-file=") {
            *word = format!(
                "--dependency-file={}",
                text(&request.output.join("relink.d"))?
            );
        }
    }
    relink.extend([
        format!("-Wl,-Map={}", text(&map_path)?),
        "-Wl,--cref".into(),
        "-Wl,--no-demangle".into(),
    ]);
    checked(
        Command::new(&relink[0])
            .args(&relink[1..])
            .current_dir(&link_dir)
            .env("PWD", &link_dir)
            .env("LC_ALL", "C"),
        "relink with map",
    )?;
    let executable_bytes =
        fs::read(&executable).map_err(|error| format!("{}: {error}", executable.display()))?;
    let side_bytes = fs::read(&side).map_err(|error| format!("{}: {error}", side.display()))?;
    let executable_sha256 = sha256_hex(&executable_bytes);
    if side_bytes != executable_bytes {
        return Err(format!(
            "the relinked image ({}) differs from {} ({executable_sha256}); the map would not \
             describe the executable",
            sha256_hex(&side_bytes),
            executable.display()
        ));
    }
    drop(side_bytes);
    fs::remove_file(&side).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(request.output.join("relink.d"));
    // Configurations: the database records, in order, with their objects
    // and the route's unit keys.
    let database_text = fs::read_to_string(request.database)
        .map_err(|error| format!("{}: {error}", request.database.display()))?;
    let records: Vec<Value> =
        serde_json::from_str(&database_text).map_err(|error| error.to_string())?;
    let entries = crate::compile_db::compile_entries(&database_text)?;
    let keys = route_units(&database_text, request.source, request.profile_directory)?;
    let configurations = entries
        .into_iter()
        .zip(records.iter())
        .zip(keys)
        .map(|((entry, record), route_unit)| Configuration {
            object: record["output"]
                .as_str()
                .map(|output| request.build.join(output)),
            file: entry.file,
            directory: entry.directory,
            command: entry.command,
            route_unit,
        })
        .collect::<Vec<_>>();
    let map_text = fs::read_to_string(&map_path)
        .map_err(|error| format!("{}: {error}", map_path.display()))?;
    let resolution = resolve(
        &link_dir,
        &map_text,
        &executable_bytes,
        &configurations,
        request.jobs,
        &request.output.join("reproduce"),
    )?;
    let tool = |path: &str| -> Result<Value, String> {
        let resolved = fs::canonicalize(path).map_err(|error| format!("{path}: {error}"))?;
        Ok(json!({"path": path, "resolved": resolved, "sha256": sha_file(&resolved)?}))
    };
    let clang_version = checked(Command::new(&words[0]).arg("--version"), "compiler version")?;
    let artifact = json!({
        "schema": ARTIFACT_SCHEMA,
        "scope": "Link resolution of the fluent-bit executable, read from GNU ld's map and cross-reference table and cross-checked against every linked object's and the executable's ELF symbol tables. Not an input to any verdict; no route reads it. It establishes, per referenced function symbol, which linked definition the static link chose; it does not establish the call-site type or calling convention, nor resolution for a caller outside the linked objects.",
        "executable": {
            "path": executable,
            "sha256": executable_sha256,
            "image": resolution.executable,
        },
        "link": {
            "target": TARGET,
            "link_txt": link_txt,
            "link_txt_sha256": sha_file(&link_txt)?,
            "command": words,
            "directory": link_dir,
            "map_command": relink,
            "map": map_path,
            "map_sha256": sha256_hex(map_text.as_bytes()),
            "relinked_equal": true,
            "build_command": build_command,
            "build_seconds": build_seconds,
        },
        "toolchain": {
            "driver": tool(&words[0])?,
            "driver_version": String::from_utf8_lossy(&clang_version.stdout).trim(),
            "linker": tool(&linker)?,
            "linker_version": linker_version,
        },
        "compile_database": {
            "path": request.database,
            "sha256": sha256_hex(database_text.as_bytes()),
        },
        "profile_directory": request.profile_directory,
        "counts": resolution.counts,
        "inputs": resolution.inputs,
        "configurations": resolution.configurations,
        "symbols": resolution.decisions.iter().map(|decision| decision.record.clone()).collect::<Vec<_>>(),
    });
    let artifact_path = request.output.join("link-resolution.json");
    let artifact_text = serde_json::to_string_pretty(&artifact).map_err(|e| e.to_string())? + "\n";
    fs::write(&artifact_path, &artifact_text).map_err(|error| error.to_string())?;
    let artifact_sha256 = sha256_hex(artifact_text.as_bytes());
    let premise = format!(
        "DERIVED CANDIDATE (card FLUENT-BIT-LINK-PREPARE, not adopted): in the linked fluent-bit executable \
         (sha256 {executable_sha256}), linked by GNU ld from the compile database (sha256 {}) as recorded \
         in the link-resolution artifact (sha256 {artifact_sha256}), each listed symbol has exactly one \
         definition among the linked objects: a strong global definition from the listed home unit's \
         object, which its compile database entry reproduces byte for byte, and the executable carries \
         no dynamic relocation against it. Not established here: resolution for a caller outside the \
         executable's linked objects, and equality of the call-site type and calling convention with \
         the home definition.",
        sha256_hex(database_text.as_bytes())
    );
    let (map, refusals) = candidate_map(&resolution, &premise, &artifact_sha256);
    let candidate_path = request.output.join("link-map.candidate.json");
    let refusals_path = request.output.join("link-map.candidate.refusals.json");
    let candidate_text = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())? + "\n";
    let refusals_text = serde_json::to_string_pretty(&refusals).map_err(|e| e.to_string())? + "\n";
    fs::write(&candidate_path, &candidate_text).map_err(|error| error.to_string())?;
    fs::write(&refusals_path, &refusals_text).map_err(|error| error.to_string())?;
    let record = json!({
        "artifact": artifact_path,
        "artifact_sha256": artifact_sha256,
        "candidate_map": candidate_path,
        "candidate_map_sha256": sha256_hex(candidate_text.as_bytes()),
        "refusals": refusals_path,
        "refusals_sha256": sha256_hex(refusals_text.as_bytes()),
        "executable": executable,
        "executable_sha256": executable_sha256,
        "build_log": build_log,
        "counts": artifact["counts"],
        "scope": "candidate only: no route or profile reads the candidate map; adopting it is BODY-SUMMARY-REUSE-LINK-STANDING's",
    });
    Ok(LinkPrepared {
        record,
        artifact: artifact_path,
        candidate: candidate_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut long = Vec::new();
        let mut names = Vec::new();
        for (name, _) in members {
            if name.len() > 15 {
                names.push(format!("/{}", long.len()));
                long.extend_from_slice(name.as_bytes());
                long.extend_from_slice(b"/\n");
            } else {
                names.push(format!("{name}/"));
            }
        }
        let header = |name: &str, size: usize| {
            format!("{name:<16}{:<12}{:<6}{:<6}{:<8}{size:<10}`\n", 0, 0, 0, 644).into_bytes()
        };
        let mut out = b"!<arch>\n".to_vec();
        let push = |out: &mut Vec<u8>, name: &str, data: &[u8]| {
            out.extend(header(name, data.len()));
            out.extend_from_slice(data);
            if data.len() % 2 == 1 {
                out.push(b'\n');
            }
        };
        push(&mut out, "/", b"\0\0\0\0");
        if !long.is_empty() {
            push(&mut out, "//", &long);
        }
        for ((_, data), name) in members.iter().zip(&names) {
            push(&mut out, name, data);
        }
        out
    }

    #[test]
    fn archive_members_resolve_short_and_long_names() {
        let bytes = archive(&[
            ("a.c.o", b"one"),
            ("a_rather_long_member_name.c.o", b"second"),
            ("a.c.o", b"three"),
        ]);
        let members = archive_members(&bytes).unwrap();
        let read: Vec<(&str, &[u8])> = members
            .iter()
            .map(|(name, range)| (name.as_str(), &bytes[range.clone()]))
            .collect();
        assert_eq!(
            read,
            vec![
                ("a.c.o", &b"one"[..]),
                ("a_rather_long_member_name.c.o", &b"second"[..]),
                ("a.c.o", &b"three"[..]),
            ]
        );
        assert!(archive_members(b"!<thin>\n").is_err());
        assert!(archive_members(b"not an archive").is_err());
    }

    #[test]
    fn map_reader_takes_members_loads_text_and_cross_references() {
        let text = "\
Archive member included to satisfy reference by file (symbol)

libb.a(b.o)                   a.o (home_strong)
../library/libvery-long-archive-name.a(a_member_with_a_long_name.c.o)
                              a.o (other)

Merging program properties

Linker script and memory map

LOAD /lib/x86_64-linux-gnu/Scrt1.o
LOAD a.o
LOAD libb.a
 .text          0x0000000000001140       0x6b a.o
                0x0000000000001140                main
 *fill*         0x00000000000011ab        0x5
 .text          0x00000000000011b0       0x28 libb.a(b.o)
                0x00000000000011b0                home_strong
 .text.a_section_name_long_enough_to_wrap
                0x0000000000001200       0x10 ../library/libvery-long-archive-name.a(a_member_with_a_long_name.c.o)
OUTPUT(prog elf64-x86-64)

Cross Reference Table

Symbol                                            File
a_very_long_function_name_that_exceeds_the_cref_symbol_column_width libb.a(b.o)
                                                  a.o
home_strong                                       libb.a(b.o)
                                                  a.o
puts                                              /usr/lib/x86_64-linux-gnu/libc.so.6
                                                  a.o
";
        let map = parse_map(text).unwrap();
        assert_eq!(
            map.members,
            BTreeSet::from([
                "libb.a(b.o)".to_owned(),
                "../library/libvery-long-archive-name.a(a_member_with_a_long_name.c.o)".to_owned()
            ])
        );
        assert_eq!(
            map.loads,
            vec!["/lib/x86_64-linux-gnu/Scrt1.o", "a.o", "libb.a"]
        );
        assert_eq!(
            map.text,
            vec![
                ("a.o".to_owned(), 0x1140, 0x6b),
                ("libb.a(b.o)".to_owned(), 0x11b0, 0x28),
                (
                    "../library/libvery-long-archive-name.a(a_member_with_a_long_name.c.o)"
                        .to_owned(),
                    0x1200,
                    0x10
                ),
            ]
        );
        assert_eq!(map.cref.len(), 3);
        assert_eq!(
            map.cref[0].1,
            vec!["libb.a(b.o)".to_owned(), "a.o".to_owned()]
        );
        assert_eq!(map.cref[2].1[0], "/usr/lib/x86_64-linux-gnu/libc.so.6");
        assert!(parse_map("Linker script and memory map\n").is_err());
        assert_eq!(
            split_member("../x/libz.a(m.c.o)"),
            Some(("../x/libz.a", "m.c.o"))
        );
        assert_eq!(split_member("a.o"), None);
    }

    /// A real link with Clang and GNU ld: strong, weak, duplicated,
    /// shared-object and unreproduced definitions, and a home outside the
    /// route.
    #[test]
    fn a_small_link_derives_only_unique_strong_reproduced_route_homes() {
        let clang = "/usr/lib/llvm-21/bin/clang";
        if !Path::new(clang).is_file() || Command::new("ld").arg("--version").output().is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "quirin-link-resolution-{}-{}",
            std::process::id(),
            sha256_hex(format!("{:?}", Instant::now()).as_bytes())
        ));
        fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, body: &str| fs::write(dir.join(name), body).unwrap();
        write(
            "main.c",
            "#include <stdio.h>\nint strong_home(int);int weak_home(int);int other_home(void);\
             int drifted_home(void);int unrouted_home(void);int pulled(void);\
             static int local_only(void){return 3;}\
             int main(void){puts(\"x\");return strong_home(1)+weak_home(2)+other_home()+\
             drifted_home()+unrouted_home()+pulled()+local_only();}\n",
        );
        write(
            "homes.c",
            "int strong_home(int x){return x+1;}\n__attribute__((weak)) int weak_home(int x){return x;}\n\
             int other_home(void){return 0;}\n",
        );
        write(
            "strong_twin.c",
            "int weak_home(int x){return x*2;}\nint pulled(void){return 9;}\n",
        );
        write("drifted.c", "int drifted_home(void){int a=1;return a+1;}\n");
        write("unrouted.c", "int unrouted_home(void){return 5;}\n");
        let compile = |source: &str, extra: &[&str]| -> String {
            let object = format!("{source}.o");
            let path = dir.join(source);
            let status = Command::new(clang)
                .args(["-g", "-c"])
                .args(extra)
                .args(["-o", &object, path.to_str().unwrap()])
                .current_dir(&dir)
                .env("PWD", &dir)
                .status()
                .unwrap();
            assert!(status.success());
            format!("{clang} -g -c -o {object} {}", dir.join(source).display())
        };
        let main = compile("main.c", &[]);
        let homes = compile("homes.c", &[]);
        let twin = compile("strong_twin.c", &[]);
        // Built at -O2, recorded at -O0: the record does not reproduce it.
        compile("drifted.c", &["-O2"]);
        let drifted = format!(
            "{clang} -g -c -o drifted.c.o {}",
            dir.join("drifted.c").display()
        );
        let unrouted = compile("unrouted.c", &[]);
        let archive = Command::new("ar")
            .args([
                "qc",
                "libhomes.a",
                "homes.c.o",
                "strong_twin.c.o",
                "drifted.c.o",
                "unrouted.c.o",
            ])
            .current_dir(&dir)
            .status()
            .unwrap();
        assert!(archive.success());
        let link = |output: &str, map: bool| {
            let mut command = Command::new(clang);
            command
                .args(["main.c.o", "libhomes.a", "-o", output])
                .current_dir(&dir)
                .env("LC_ALL", "C");
            if map {
                command.args(["-Wl,-Map=prog.map", "-Wl,--cref", "-Wl,--no-demangle"]);
            }
            assert!(command.status().unwrap().success());
        };
        link("prog", false);
        link("prog.relinked", true);
        let executable = fs::read(dir.join("prog")).unwrap();
        assert_eq!(executable, fs::read(dir.join("prog.relinked")).unwrap());
        let configuration = |source: &str, command: String, routed: bool| Configuration {
            file: dir.join(source),
            directory: dir.clone(),
            command,
            object: Some(dir.join(format!("{source}.o"))),
            route_unit: routed.then(|| format!("lib/{source}")),
        };
        let configurations = vec![
            configuration("main.c", main, true),
            configuration("homes.c", homes, true),
            configuration("strong_twin.c", twin, true),
            configuration("drifted.c", drifted, true),
            configuration("unrouted.c", unrouted, false),
        ];
        let map_text = fs::read_to_string(dir.join("prog.map")).unwrap();
        let resolution = resolve(
            &dir,
            &map_text,
            &executable,
            &configurations,
            2,
            &dir.join("reproduce"),
        )
        .unwrap();
        let decided: BTreeMap<&str, (Option<&str>, Option<&str>)> = resolution
            .decisions
            .iter()
            .map(|d| (d.name.as_str(), (d.refusal, d.home_unit.as_deref())))
            .collect();
        assert_eq!(decided["strong_home"], (None, Some("lib/homes.c")));
        assert_eq!(decided["other_home"], (None, Some("lib/homes.c")));
        assert_eq!(decided["pulled"], (None, Some("lib/strong_twin.c")));
        // homes.c's weak definition and strong_twin.c's strong one are both
        // linked; the linker chose the strong one, and the name is refused.
        assert_eq!(decided["weak_home"].0, Some(DUPLICATE));
        assert_eq!(decided["puts"].0, Some(SHARED_OBJECT));
        assert_eq!(decided["drifted_home"].0, Some(NOT_REPRODUCED));
        assert_eq!(decided["unrouted_home"].0, Some(NOT_ROUTE_UNIT));
        assert!(!decided.contains_key("local_only"));
        // The C runtime's start file references `main` from another input.
        assert_eq!(decided["main"], (None, Some("lib/main.c")));
        let (map, refusals) = candidate_map(&resolution, "test premise", "0");
        assert_eq!(map["symbols"].as_array().unwrap().len(), 4);
        // crtbeginS.o's reference to libc's `__cxa_finalize` is the fifth.
        assert_eq!(decided["__cxa_finalize"].0, Some(SHARED_OBJECT));
        assert_eq!(refusals["refused"].as_array().unwrap().len(), 5);
        let _ = fs::remove_dir_all(&dir);
    }
}
