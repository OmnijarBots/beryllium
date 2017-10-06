extern crate protoc_rust;

use protoc_rust::Args;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=messages.proto");

    let root = env::var("CARGO_MANIFEST_DIR").unwrap();

    let args = Args {
        out_dir: &out_dir,
        input: &[&format!("{}/messages.proto", root)],
        includes: &[&root],
    };

    protoc_rust::run(args).expect("codegen");

    // Workaround for https://github.com/rust-lang/rust/issues/18849
    let source_path = PathBuf::from(&out_dir).join("messages.rs");
    let mut fd = File::open(&source_path).unwrap();
    let mut bytes = vec![];
    fd.read_to_end(&mut bytes).unwrap();

    let mut fd = File::create(&source_path).unwrap();
    fd.write_all(b"mod messages_proto {\n").unwrap();
    fd.write_all(&bytes).unwrap();
    fd.write_all(b"}").unwrap();
}
