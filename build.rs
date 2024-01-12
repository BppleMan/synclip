use std::collections::VecDeque;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let protobuf_dir = manifest_dir.join("proto");
    let protos = list_files_recursively(&protobuf_dir);
    if !protos.is_empty() {
        tonic_build::configure()
            .build_client(true)
            .build_server(true)
            // .out_dir(src_dir)
            .compile(&protos, &[protobuf_dir])
            .unwrap();
    }
}

pub fn list_files_recursively(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back(path.as_ref().to_path_buf());
    while let Some(path) = queue.pop_front() {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    queue.push_back(path);
                } else {
                    result.push(path);
                }
            }
        } else {
            result.push(path);
        }
    }
    result
}

pub fn list_mods(path: impl AsRef<Path>) -> Vec<DirEntry> {
    fs::read_dir(path)
        .unwrap()
        .flatten()
        .filter(|entry| !entry.file_name().eq_ignore_ascii_case("lib.rs"))
        .collect::<Vec<_>>()
}
