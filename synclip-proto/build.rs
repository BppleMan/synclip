use std::collections::VecDeque;
use std::fs::DirEntry;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let protobuf_dir = manifest_dir.join("proto");
    // build script的构建缓存的范围限制
    // println!("cargo:rerun-if-changed={}", protobuf_dir.display());
    let src_dir = manifest_dir.join("src");
    // clean_src(&src_dir);
    let protos = list_files_recursively(&protobuf_dir);
    if !protos.is_empty() {
        tonic_build::configure()
            .build_client(true)
            .build_server(true)
            .out_dir(src_dir)
            .compile(&protos, &[protobuf_dir])
            .unwrap();
    }
    // update_lib(src_dir);
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

pub fn clean_src(path: impl AsRef<Path>) {
    list_mods(path)
        .iter()
        .for_each(|entry| fs::remove_file(entry.path()).unwrap());
}

pub fn update_lib(path: impl AsRef<Path>) {
    let mods = list_mods(&path)
        .iter()
        .map(|entry| {
            let mod_name = entry.file_name().to_str().unwrap().replace(".rs", "");
            format!("pub mod {};", mod_name)
        })
        .collect::<Vec<_>>()
        .join("\n")
        .add("\n");
    let lib_file = path.as_ref().join("lib.rs");
    fs::write(lib_file, mods).unwrap();
}
