//! Test-only discovery of the single canonical source for each logical module.
use std::{collections::BTreeMap, fs, path::PathBuf};

pub fn sources() -> BTreeMap<String, PathBuf> {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let index = repository.join("contract/corpus.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    let mut sources = BTreeMap::new();
    for root in manifest["roots"].as_array().unwrap() {
        let root = index.parent().unwrap().join(root.as_str().unwrap());
        assert!(
            root.is_dir() && !root.is_symlink(),
            "invalid corpus root: {}",
            root.display()
        );
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                assert!(
                    !path.is_symlink(),
                    "symbolic link in corpus: {}",
                    path.display()
                );
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "qcore") {
                    let logical = path
                        .strip_prefix(&root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    assert!(
                        sources.insert(logical.clone(), path).is_none(),
                        "duplicate logical contract: {logical}"
                    );
                }
            }
        }
    }
    sources
}
