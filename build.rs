extern crate bindgen;

use bindgen::callbacks::{ParseCallbacks, IntKind};

extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {

    println!("cargo:rerun-if-changed=src/CDeps");
}
