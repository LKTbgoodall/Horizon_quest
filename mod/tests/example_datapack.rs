//! The example data pack in `examples/quest-datapack`: its files, read the way
//! Horizon hands a reload listener its folder (`<namespace>:<path>` and the
//! parsed JSON, sorted by id), must make a pack without a single problem.

use horizon_mod_horizon_quests::hq_validate::{DATA_FOLDER, load_data};
use std::path::{Path, PathBuf};

/// Every `.json` file under `dir`, as its path below it with `/`.
fn files(dir: &Path, below: &str, out: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let rel = if below.is_empty() { name } else { format!("{below}/{name}") };
        if path.is_dir() {
            files(&path, &rel, out);
        } else if let Some(stem) = rel.strip_suffix(".json") {
            out.push((stem.to_owned(), path));
        }
    }
}

#[test]
fn the_example_data_pack_loads_cleanly() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/quest-datapack/data");
    let mut read = Vec::new();
    for entry in std::fs::read_dir(&data).unwrap() {
        let namespace_dir = entry.unwrap().path();
        let namespace = namespace_dir.file_name().unwrap().to_string_lossy().into_owned();
        let mut found = Vec::new();
        files(&namespace_dir.join(DATA_FOLDER), "", &mut found);
        for (path, file) in found {
            let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
            read.push((format!("{namespace}:{path}"), json));
        }
    }
    read.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(read.len(), 3, "{read:?}");

    let loaded = load_data(&read).expect("the data pack holds a chapter");
    let problems: Vec<String> = loaded.problems.iter().map(|p| p.to_string()).collect();
    assert!(problems.is_empty(), "{problems:#?}");
    assert_eq!(loaded.file.chapters.len(), 1);
    assert_eq!(loaded.file.chapters[0].quests.len(), 2);
    assert_eq!(loaded.file.reward_tables.len(), 1);
}
